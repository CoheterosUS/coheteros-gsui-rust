use super::packet::*;

struct PacketReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {
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

    fn i8(&mut self) -> i8 {
        let v = self.buf[self.pos] as i8;
        self.pos += 1;
        v
    }

    fn i16_le(&mut self) -> i16 {
        let v = i16::from_le_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
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

    fn scaled(&mut self) -> f64 {
        self.i32_le() as f64 / SCALE
    }

    fn gps_scaled(&mut self) -> f64 {
        self.i32_le() as f64 / GPS_SCALE
    }
}

pub fn parse_packet(buf: &[u8; PACKET_SIZE]) -> Option<Telemetry> {
    let mut r = PacketReader::new(buf);

    let sync = r.u16_le();
    if sync != SYNC_WORD {
        return None;
    }
    if buf[PACKET_SIZE - 1] != SYNC_END {
        return None;
    }

    let tick = r.u32_le();
    let accel = [r.i16_le() as f64, r.i16_le() as f64, r.i16_le() as f64];
    let gyro = [r.i16_le() as f64, r.i16_le() as f64, r.i16_le() as f64];
    let pressure_pa = r.i16_le() as f64 * PRESSURE_SCALE;
    let temperature_c = r.i8() as f64;
    let latitude = r.gps_scaled();
    let longitude = r.gps_scaled();
    let gps_altitude = r.scaled();
    let satellites = r.u8();
    let baro_altitude = r.scaled();
    let baro_velocity = r.scaled();
    let flags = r.u32_le();
    let battery_voltage = r.i16_le() as f64 / BATTERY_SCALE;
    let state = FlightState::from_u8(r.u8())?;
    let relay = RelayState::from_u8(r.u8());
    let last_command = Command::from_u8(r.u8()).unwrap_or(Command::None);

    Some(Telemetry {
        raw: *buf,
        tick,
        accel,
        gyro,
        pressure_pa,
        temperature_c,
        latitude,
        longitude,
        gps_altitude,
        satellites,
        baro_altitude,
        baro_velocity,
        flags,
        battery_voltage,
        state,
        relay,
        last_command,
    })
}

pub struct StreamParser {
    buf: Vec<u8>,
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(256),
        }
    }

    pub fn feed(&mut self, data: &[u8]) -> Vec<Telemetry> {
        self.buf.extend_from_slice(data);
        let mut packets = Vec::new();

        loop {
            let sync_pos = self.buf.windows(2).position(|w| w[0] == 0xFE && w[1] == 0xCA);
            let Some(pos) = sync_pos else { break };

            if pos > 0 {
                self.buf.drain(..pos);
            }

            if self.buf.len() < PACKET_SIZE {
                break;
            }

            let frame: [u8; PACKET_SIZE] = self.buf[..PACKET_SIZE].try_into().unwrap();
            if frame[PACKET_SIZE - 1] == SYNC_END {
                if let Some(telem) = parse_packet(&frame) {
                    packets.push(telem);
                }
                self.buf.drain(..PACKET_SIZE);
            } else {
                self.buf.drain(..2);
            }
        }

        if self.buf.len() > 512 {
            let keep = self.buf.len().saturating_sub(PACKET_SIZE);
            self.buf.drain(..keep);
        }

        packets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_packet() -> [u8; PACKET_SIZE] {
        let mut buf = [0u8; PACKET_SIZE];
        buf[0] = 0xFE;
        buf[1] = 0xCA;
        buf[2..6].copy_from_slice(&1000u32.to_le_bytes());
        buf[6..8].copy_from_slice(&9i16.to_le_bytes());
        buf[48] = 0;
        buf[49] = 0;
        buf[50] = 0;
        buf[PACKET_SIZE - 1] = SYNC_END;
        buf
    }

    #[test]
    fn parse_valid_packet() {
        let buf = make_test_packet();
        let t = parse_packet(&buf).unwrap();
        assert_eq!(t.tick, 1000);
        assert!((t.accel[0] - 9.0).abs() < 0.01);
        assert_eq!(t.state, FlightState::Idle);
    }

    #[test]
    fn reject_bad_sync() {
        let mut buf = make_test_packet();
        buf[0] = 0x00;
        assert!(parse_packet(&buf).is_none());
    }

    #[test]
    fn reject_bad_footer() {
        let mut buf = make_test_packet();
        buf[PACKET_SIZE - 1] = 0x00;
        assert!(parse_packet(&buf).is_none());
    }

    #[test]
    fn stream_parser_resync() {
        let mut parser = StreamParser::new();
        let pkt = make_test_packet();
        let mut data = vec![0xAA, 0xBB, 0xCC];
        data.extend_from_slice(&pkt);
        let results = parser.feed(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tick, 1000);
    }
}
