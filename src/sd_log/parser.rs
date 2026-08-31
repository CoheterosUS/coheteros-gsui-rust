use crate::telemetry::packet::{Command, FlightState, RelayState};
use super::record::*;

struct RecordReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> RecordReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn u8(&mut self) -> u8 {
        let v = self.buf[self.pos];
        self.pos += 1;
        v
    }

    fn u16_le(&mut self) -> u16 {
        let v = u16::from_le_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        v
    }

    fn u32_le(&mut self) -> u32 {
        let v = u32::from_le_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]);
        self.pos += 4;
        v
    }

    fn i32_le(&mut self) -> i32 {
        let v = i32::from_le_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]);
        self.pos += 4;
        v
    }

    fn f32_le(&mut self) -> f64 {
        let v = f32::from_le_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]);
        self.pos += 4;
        v as f64
    }

    fn gps_scaled(&mut self) -> f64 {
        self.i32_le() as f64 / GPS_SCALE
    }
}

pub fn parse_sd_record(buf: &[u8; SD_RECORD_SIZE]) -> Option<SdRecord> {
    let mut r = RecordReader::new(buf);

    let sync = r.u16_le();
    if sync != SD_SYNC_WORD {
        return None;
    }
    if buf[SD_RECORD_SIZE - 1] != SD_SYNC_END {
        return None;
    }

    let tick = r.u32_le();
    let accel = [r.f32_le(), r.f32_le(), r.f32_le()];
    let gyro = [r.f32_le(), r.f32_le(), r.f32_le()];
    let mag = [r.f32_le(), r.f32_le(), r.f32_le()];
    let pressure_pa = r.f32_le();
    let temperature_c = r.f32_le();
    let latitude = r.gps_scaled();
    let longitude = r.gps_scaled();
    let gps_altitude = r.f32_le();
    let unix_time = r.u32_le();
    let milliseconds = r.u16_le();
    let satellites = r.u8();
    let flags = r.u32_le();
    let battery_voltage = r.f32_le();
    let state = FlightState::from_u8(r.u8())?;
    let relay = RelayState::from_u8(r.u8());
    let last_command = Command::from_u8(r.u8()).unwrap_or(Command::None);

    Some(SdRecord {
        raw: *buf,
        tick,
        accel,
        gyro,
        mag,
        pressure_pa,
        temperature_c,
        latitude,
        longitude,
        gps_altitude,
        unix_time,
        milliseconds,
        satellites,
        flags,
        battery_voltage,
        state,
        relay,
        last_command,
    })
}

pub fn parse_sd_file(data: &[u8]) -> Vec<SdRecord> {
    let mut records = Vec::new();

    // Try aligned reading first
    let mut offset = 0;
    while offset + SD_RECORD_SIZE <= data.len() {
        let chunk: &[u8; SD_RECORD_SIZE] = data[offset..offset + SD_RECORD_SIZE]
            .try_into()
            .unwrap();

        let sync = u16::from_le_bytes([chunk[0], chunk[1]]);
        if sync == SD_SYNC_WORD && chunk[SD_RECORD_SIZE - 1] == SD_SYNC_END {
            if let Some(record) = parse_sd_record(chunk) {
                records.push(record);
                offset += SD_RECORD_SIZE;
                continue;
            }
        }

        // Aligned read failed — scan forward for next sync word
        offset += 1;
        while offset + 1 < data.len() {
            if data[offset] == 0xFE && data[offset + 1] == 0xCA {
                break;
            }
            offset += 1;
        }
    }

    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_sd_record() -> [u8; SD_RECORD_SIZE] {
        let mut buf = [0u8; SD_RECORD_SIZE];
        buf[0] = 0xFE; // sync LSB
        buf[1] = 0xCA; // sync MSB
        buf[2..6].copy_from_slice(&500u32.to_le_bytes()); // tick
        // accel_x = 9.81 as f32
        buf[6..10].copy_from_slice(&9.81f32.to_le_bytes());
        buf[77] = 0; // state = Idle
        buf[78] = 0; // relay
        buf[79] = 0; // command
        buf[SD_RECORD_SIZE - 1] = SD_SYNC_END;
        buf
    }

    #[test]
    fn parse_valid_sd_record() {
        let buf = make_test_sd_record();
        let r = parse_sd_record(&buf).unwrap();
        assert_eq!(r.tick, 500);
        assert!((r.accel[0] - 9.81).abs() < 0.001);
        assert_eq!(r.state, FlightState::Idle);
    }

    #[test]
    fn reject_bad_sync() {
        let mut buf = make_test_sd_record();
        buf[0] = 0x00;
        assert!(parse_sd_record(&buf).is_none());
    }

    #[test]
    fn reject_bad_footer() {
        let mut buf = make_test_sd_record();
        buf[SD_RECORD_SIZE - 1] = 0x00;
        assert!(parse_sd_record(&buf).is_none());
    }

    #[test]
    fn parse_file_aligned() {
        let rec = make_test_sd_record();
        let mut data = Vec::new();
        data.extend_from_slice(&rec);
        data.extend_from_slice(&rec);
        data.extend_from_slice(&rec);
        let results = parse_sd_file(&data);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn parse_file_with_garbage_prefix() {
        let rec = make_test_sd_record();
        let mut data = vec![0xAA, 0xBB, 0xCC];
        data.extend_from_slice(&rec);
        let results = parse_sd_file(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tick, 500);
    }
}
