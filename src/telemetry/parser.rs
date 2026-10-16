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

    #[allow(dead_code)]
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
}

fn parse_shared(r: &mut PacketReader) -> Option<(u32, [f64; 3], [f64; 3], [f64; 4], FlightState)> {
    let tick = r.u32_le();
    let accel = [
        r.i16_le() as f64 / ACCEL_SCALE,
        r.i16_le() as f64 / ACCEL_SCALE,
        r.i16_le() as f64 / ACCEL_SCALE,
    ];
    let gyro = [
        r.i16_le() as f64 / GYRO_SCALE,
        r.i16_le() as f64 / GYRO_SCALE,
        r.i16_le() as f64 / GYRO_SCALE,
    ];
    let quat = [
        r.i16_le() as f64 / QUAT_SCALE,
        r.i16_le() as f64 / QUAT_SCALE,
        r.i16_le() as f64 / QUAT_SCALE,
        r.i16_le() as f64 / QUAT_SCALE,
    ];
    let state = FlightState::from_u8(r.u8())?;
    Some((tick, accel, gyro, quat, state))
}

fn default_telemetry(raw: [u8; PACKET_SIZE]) -> Telemetry {
    Telemetry {
        raw,
        source: TelemetrySource::A,
        tick: 0,
        accel: [0.0; 3],
        gyro: [0.0; 3],
        quat: [1.0, 0.0, 0.0, 0.0],
        state: FlightState::Idle,
        baro_altitude: 0.0,
        pos_z: 0.0,
        vel_z: 0.0,
        kalman_pz: 0.0,
        flags: 0,
        relay: RelayState::from_u8(0),
        satellites: 0,
        last_command: Command::None,
        latitude: 0.0,
        longitude: 0.0,
        gps_altitude: 0.0,
        unix_time: 0,
        pressure_pa: 0.0,
        temperature_c: 0.0,
        battery_voltage: 0.0,
    }
}

pub fn parse_packet(buf: &[u8; PACKET_SIZE]) -> Option<Telemetry> {
    let sync = u16::from_le_bytes([buf[0], buf[1]]);
    if sync != SYNC_WORD {
        return None;
    }
    if buf[PACKET_SIZE - 1] != SYNC_END {
        return None;
    }

    // LEN counts everything between the 4-byte header and the footer: 0x30 (48)
    if buf[3] as usize != PACKET_SIZE - 5 {
        return None;
    }

    let cmd = buf[2];
    match cmd {
        0x80 => parse_telem_a(buf),
        0x81 => parse_telem_b(buf),
        _ => None,
    }
}

fn parse_telem_a(buf: &[u8; PACKET_SIZE]) -> Option<Telemetry> {
    let mut r = PacketReader::new(buf);
    r.pos = 4; // skip sync + cmd + len
    let (tick, accel, gyro, quat, state) = parse_shared(&mut r)?;

    let baro_altitude = r.f32_le();
    let pos_z = r.f32_le();
    let vel_z = r.f32_le();
    let kalman_pz = r.f32_le();
    let flags = r.u32_le();
    let relay = RelayState::from_u8(r.u8());
    let satellites = r.u8();
    let last_command = Command::from_u8(r.u8()).unwrap_or(Command::None);

    let mut t = default_telemetry(*buf);
    t.source = TelemetrySource::A;
    t.tick = tick;
    t.accel = accel;
    t.gyro = gyro;
    t.quat = quat;
    t.state = state;
    t.baro_altitude = baro_altitude;
    t.pos_z = pos_z;
    t.vel_z = vel_z;
    t.kalman_pz = kalman_pz;
    t.flags = flags;
    t.relay = relay;
    t.satellites = satellites;
    t.last_command = last_command;
    Some(t)
}

fn parse_telem_b(buf: &[u8; PACKET_SIZE]) -> Option<Telemetry> {
    let mut r = PacketReader::new(buf);
    r.pos = 4; // skip sync + cmd + len
    let (tick, accel, gyro, quat, state) = parse_shared(&mut r)?;

    let latitude = r.i32_le() as f64 / GPS_SCALE;
    let longitude = r.i32_le() as f64 / GPS_SCALE;
    let gps_altitude = r.f32_le();
    let unix_time = r.u32_le();
    let pressure_pa = r.f32_le();
    let temperature_c = r.i16_le() as f64 / TEMP_SCALE;
    let battery_voltage = r.u8() as f64 / BATTERY_SCALE;

    let mut t = default_telemetry(*buf);
    t.source = TelemetrySource::B;
    t.tick = tick;
    t.accel = accel;
    t.gyro = gyro;
    t.quat = quat;
    t.state = state;
    t.latitude = latitude;
    t.longitude = longitude;
    t.gps_altitude = gps_altitude;
    t.unix_time = unix_time;
    t.pressure_pa = pressure_pa;
    t.temperature_c = temperature_c;
    t.battery_voltage = battery_voltage;
    Some(t)
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

    fn make_telem_a_packet() -> [u8; PACKET_SIZE] {
        let mut buf = [0u8; PACKET_SIZE];
        buf[0] = 0xFE; // sync LSB
        buf[1] = 0xCA; // sync MSB
        buf[2] = 0x80; // CMD = TELEM_A
        buf[3] = 0x30; // LEN = 48
        // tick at offset 4
        buf[4..8].copy_from_slice(&1000u32.to_le_bytes());
        // accel x at offset 8 (value 9.00 m/s² → 900)
        buf[8..10].copy_from_slice(&900i16.to_le_bytes());
        // state at offset 28
        buf[28] = 0; // IDLE
        // baro_altitude at offset 29 (float32)
        buf[29..33].copy_from_slice(&150.0f32.to_le_bytes());
        // pos_z at offset 33 (float32)
        buf[33..37].copy_from_slice(&(-250.0f32).to_le_bytes());
        // vel_z at offset 37 (float32)
        buf[37..41].copy_from_slice(&(-12.5f32).to_le_bytes());
        // kalman_pz at offset 41 (float32)
        buf[41..45].copy_from_slice(&2.25f32.to_le_bytes());
        // flags at offset 45
        buf[45..49].copy_from_slice(&0u32.to_le_bytes());
        // relay at offset 49
        buf[49] = 0;
        // satellites at offset 50
        buf[50] = 7;
        // last_command at offset 51
        buf[51] = 0;
        buf[PACKET_SIZE - 1] = SYNC_END;
        buf
    }

    fn make_telem_b_packet() -> [u8; PACKET_SIZE] {
        let mut buf = [0u8; PACKET_SIZE];
        buf[0] = 0xFE;
        buf[1] = 0xCA;
        buf[2] = 0x81; // CMD = TELEM_B
        buf[3] = 0x30; // LEN = 48
        buf[4..8].copy_from_slice(&2000u32.to_le_bytes());
        // state at offset 28
        buf[28] = 0;
        // latitude at offset 29 (37.3891 * 10^7)
        buf[29..33].copy_from_slice(&373891000i32.to_le_bytes());
        // longitude at offset 33
        buf[33..37].copy_from_slice(&(-59845000i32).to_le_bytes());
        // gps_altitude at offset 37
        buf[37..41].copy_from_slice(&100.5f32.to_le_bytes());
        // unix_time at offset 41
        buf[41..45].copy_from_slice(&1700000000u32.to_le_bytes());
        // pressure_pa at offset 45
        buf[45..49].copy_from_slice(&101325.0f32.to_le_bytes());
        // temperature_c at offset 49 (x100, 25.50°C → 2550)
        buf[49..51].copy_from_slice(&2550i16.to_le_bytes());
        // battery at offset 51 (x10, 12.3V → 123)
        buf[51] = 123;
        buf[PACKET_SIZE - 1] = SYNC_END;
        buf
    }

    #[test]
    fn parse_telem_a_valid() {
        let buf = make_telem_a_packet();
        let t = parse_packet(&buf).unwrap();
        assert_eq!(t.source, TelemetrySource::A);
        assert_eq!(t.tick, 1000);
        assert!((t.accel[0] - 9.0).abs() < 0.01);
        assert_eq!(t.state, FlightState::Idle);
        assert_eq!(t.satellites, 7);
        assert!((t.baro_altitude - 150.0).abs() < 0.01);
        assert!((t.pos_z - (-250.0)).abs() < 0.01);
        assert!((t.vel_z - (-12.5)).abs() < 0.01);
        assert!((t.kalman_pz - 2.25).abs() < 0.001);
    }

    #[test]
    fn parse_telem_b_valid() {
        let buf = make_telem_b_packet();
        let t = parse_packet(&buf).unwrap();
        assert_eq!(t.source, TelemetrySource::B);
        assert_eq!(t.tick, 2000);
        assert!((t.latitude - 37.3891).abs() < 0.0001);
        assert!((t.longitude - (-5.9845)).abs() < 0.0001);
        assert!((t.gps_altitude - 100.5).abs() < 0.1);
        assert_eq!(t.unix_time, 1700000000);
        assert!((t.pressure_pa - 101325.0).abs() < 1.0);
        assert!((t.temperature_c - 25.5).abs() < 0.01);
        assert!((t.battery_voltage - 12.3).abs() < 0.01);
    }

    #[test]
    fn reject_bad_sync() {
        let mut buf = make_telem_a_packet();
        buf[0] = 0x00;
        assert!(parse_packet(&buf).is_none());
    }

    #[test]
    fn reject_bad_footer() {
        let mut buf = make_telem_a_packet();
        buf[PACKET_SIZE - 1] = 0x00;
        assert!(parse_packet(&buf).is_none());
    }

    #[test]
    fn reject_bad_len() {
        let mut buf = make_telem_a_packet();
        buf[3] = 0x2C;
        assert!(parse_packet(&buf).is_none());
    }

    #[test]
    fn reject_unknown_cmd() {
        let mut buf = make_telem_a_packet();
        buf[2] = 0x99;
        assert!(parse_packet(&buf).is_none());
    }

    #[test]
    fn stream_parser_resync() {
        let mut parser = StreamParser::new();
        let pkt = make_telem_a_packet();
        let mut data = vec![0xAA, 0xBB, 0xCC];
        data.extend_from_slice(&pkt);
        let results = parser.feed(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tick, 1000);
    }

    #[test]
    fn stream_parser_mixed_ab() {
        let mut parser = StreamParser::new();
        let a = make_telem_a_packet();
        let b = make_telem_b_packet();
        let mut data = Vec::new();
        data.extend_from_slice(&a);
        data.extend_from_slice(&b);
        let results = parser.feed(&data);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].source, TelemetrySource::A);
        assert_eq!(results[1].source, TelemetrySource::B);
    }
}
