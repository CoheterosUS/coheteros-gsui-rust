use super::packet::*;

fn read_u16_le(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}

fn read_i32_le(buf: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}

fn scaled(raw: i32) -> f64 {
    raw as f64 / SCALE
}

pub fn parse_packet(buf: &[u8; PACKET_SIZE]) -> Option<Telemetry> {
    let sync = read_u16_le(buf, 0);
    if sync != SYNC_WORD {
        return None;
    }
    if buf[94] != SYNC_END {
        return None;
    }

    let state_byte = buf[91];
    let state = FlightState::from_u8(state_byte)?;
    let last_cmd = Command::from_u8(buf[93]).unwrap_or(Command::None);

    Some(Telemetry {
        tick: read_u32_le(buf, 2),
        accel: [
            scaled(read_i32_le(buf, 6)),
            scaled(read_i32_le(buf, 10)),
            scaled(read_i32_le(buf, 14)),
        ],
        gyro: [
            scaled(read_i32_le(buf, 18)),
            scaled(read_i32_le(buf, 22)),
            scaled(read_i32_le(buf, 26)),
        ],
        mag: [
            scaled(read_i32_le(buf, 30)),
            scaled(read_i32_le(buf, 34)),
            scaled(read_i32_le(buf, 38)),
        ],
        pressure_pa: scaled(read_i32_le(buf, 42)),
        temperature_c: scaled(read_i32_le(buf, 46)),
        latitude: read_i32_le(buf, 50) as f64 / GPS_SCALE,
        longitude: read_i32_le(buf, 54) as f64 / GPS_SCALE,
        gps_altitude: scaled(read_i32_le(buf, 58)),
        satellites: buf[62],
        baro_altitude: scaled(read_i32_le(buf, 63)),
        baro_velocity: scaled(read_i32_le(buf, 67)),
        velocity: [
            scaled(read_i32_le(buf, 71)),
            scaled(read_i32_le(buf, 75)),
            scaled(read_i32_le(buf, 79)),
        ],
        flags: read_u32_le(buf, 83),
        battery_voltage: scaled(read_i32_le(buf, 87)),
        state,
        relay: RelayState::from_u8(buf[92]),
        last_command: last_cmd,
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
            if frame[94] == SYNC_END {
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
        // tick = 1000
        buf[2..6].copy_from_slice(&1000u32.to_le_bytes());
        // accel_x = 9.81 * 100 = 981
        buf[6..10].copy_from_slice(&981i32.to_le_bytes());
        // state = IDLE
        buf[91] = 0;
        // relay = 0
        buf[92] = 0;
        // last_command = NONE
        buf[93] = 0;
        // footer
        buf[94] = 0xBE;
        buf
    }

    #[test]
    fn parse_valid_packet() {
        let buf = make_test_packet();
        let t = parse_packet(&buf).unwrap();
        assert_eq!(t.tick, 1000);
        assert!((t.accel[0] - 9.81).abs() < 0.01);
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
        buf[94] = 0x00;
        assert!(parse_packet(&buf).is_none());
    }

    #[test]
    fn stream_parser_resync() {
        let mut parser = StreamParser::new();
        let pkt = make_test_packet();
        let mut data = vec![0xAA, 0xBB, 0xCC]; // garbage
        data.extend_from_slice(&pkt);
        let results = parser.feed(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tick, 1000);
        assert_eq!(results[0].tick, 1000);
    }
}
