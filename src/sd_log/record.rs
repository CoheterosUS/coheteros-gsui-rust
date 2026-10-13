use crate::telemetry::packet::{Command, FlightState, PacketField, RelayState};

pub const SD_RECORD_SIZE: usize = 123;
pub const FLASH_RECORD_SIZE: usize = 32;
pub const SD_SYNC_WORD: u16 = 0xCAFE;
pub const SD_SYNC_END: u8 = 0xBE;
pub const GPS_SCALE: f64 = 10_000_000.0;
pub const TICK_RATE_HZ: f64 = 1000.0;

#[derive(Debug, Clone)]
pub struct SdRecord {
    pub raw: Vec<u8>,
    pub tick: u32,
    pub accel: [f64; 3],
    pub gyro: [f64; 3],
    pub mag: [f64; 3],
    pub pressure_pa: f64,
    pub temperature_c: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub gps_altitude: f64,
    pub unix_time: u32,
    pub milliseconds: u16,
    pub satellites: u8,
    pub vel: [f64; 3],
    pub pos: [f64; 3],
    pub quat: [f64; 4],
    pub kalman_pz: f64,
    pub flags: u32,
    pub battery_voltage: f64,
    pub state: FlightState,
    pub relay: RelayState,
    pub last_command: Command,
}

pub const SD_RECORD_FIELDS: &[PacketField] = &[
    PacketField { name: "SYNC",      offset: 0,  length: 2, color: [255, 255, 255] },
    PacketField { name: "TICK",      offset: 2,  length: 4, color: [100, 200, 255] },
    PacketField { name: "ACCEL X",   offset: 6,  length: 4, color: [255, 100, 100] },
    PacketField { name: "ACCEL Y",   offset: 10, length: 4, color: [255, 130, 100] },
    PacketField { name: "ACCEL Z",   offset: 14, length: 4, color: [255, 160, 100] },
    PacketField { name: "GYRO X",    offset: 18, length: 4, color: [100, 255, 100] },
    PacketField { name: "GYRO Y",    offset: 22, length: 4, color: [130, 255, 100] },
    PacketField { name: "GYRO Z",    offset: 26, length: 4, color: [160, 255, 100] },
    PacketField { name: "MAG X",     offset: 30, length: 4, color: [200, 100, 255] },
    PacketField { name: "MAG Y",     offset: 34, length: 4, color: [220, 130, 255] },
    PacketField { name: "MAG Z",     offset: 38, length: 4, color: [240, 160, 255] },
    PacketField { name: "PRESSURE",  offset: 42, length: 4, color: [255, 200, 50] },
    PacketField { name: "TEMP",      offset: 46, length: 4, color: [255, 150, 50] },
    PacketField { name: "LAT",       offset: 50, length: 4, color: [50, 200, 200] },
    PacketField { name: "LON",       offset: 54, length: 4, color: [80, 220, 200] },
    PacketField { name: "GPS ALT",   offset: 58, length: 4, color: [110, 240, 200] },
    PacketField { name: "UNIX TIME", offset: 62, length: 4, color: [180, 180, 255] },
    PacketField { name: "SATS",      offset: 66, length: 1, color: [140, 255, 200] },
    PacketField { name: "VEL X",    offset: 67, length: 4, color: [255, 180, 180] },
    PacketField { name: "VEL Y",    offset: 71, length: 4, color: [255, 200, 180] },
    PacketField { name: "VEL Z",    offset: 75, length: 4, color: [255, 220, 180] },
    PacketField { name: "POS X",    offset: 79, length: 4, color: [180, 255, 180] },
    PacketField { name: "POS Y",    offset: 83, length: 4, color: [180, 255, 200] },
    PacketField { name: "POS Z",    offset: 87, length: 4, color: [180, 255, 220] },
    PacketField { name: "QUAT W",   offset: 91, length: 4, color: [180, 180, 255] },
    PacketField { name: "QUAT X",   offset: 95, length: 4, color: [200, 180, 255] },
    PacketField { name: "QUAT Y",   offset: 99, length: 4, color: [220, 180, 255] },
    PacketField { name: "QUAT Z",   offset: 103, length: 4, color: [240, 180, 255] },
    PacketField { name: "KALMAN PZ", offset: 107, length: 4, color: [120, 220, 240] },
    PacketField { name: "FLAGS",     offset: 111, length: 4, color: [255, 80, 80] },
    PacketField { name: "BATTERY",   offset: 115, length: 4, color: [255, 255, 0] },
    PacketField { name: "STATE",     offset: 119, length: 1, color: [0, 200, 255] },
    PacketField { name: "RELAY",     offset: 120, length: 1, color: [255, 165, 0] },
    PacketField { name: "CMD",       offset: 121, length: 1, color: [180, 180, 255] },
    PacketField { name: "SYNC END",  offset: 122, length: 1, color: [255, 255, 255] },
];

pub const FLASH_RECORD_FIELDS: &[PacketField] = &[
    PacketField { name: "SYNC",      offset: 0,  length: 2, color: [255, 255, 255] },
    PacketField { name: "TICK",      offset: 2,  length: 4, color: [100, 200, 255] },
    PacketField { name: "ACCEL X",   offset: 6,  length: 4, color: [255, 100, 100] },
    PacketField { name: "ACCEL Y",   offset: 10, length: 4, color: [255, 130, 100] },
    PacketField { name: "ACCEL Z",   offset: 14, length: 4, color: [255, 160, 100] },
    PacketField { name: "GYRO X",    offset: 18, length: 4, color: [100, 255, 100] },
    PacketField { name: "GYRO Y",    offset: 22, length: 4, color: [130, 255, 100] },
    PacketField { name: "GYRO Z",    offset: 26, length: 4, color: [160, 255, 100] },
    PacketField { name: "STATE",     offset: 30, length: 1, color: [0, 200, 255] },
    PacketField { name: "SYNC END",  offset: 31, length: 1, color: [255, 255, 255] },
];
