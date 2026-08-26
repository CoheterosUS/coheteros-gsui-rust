use std::fmt;

pub const PACKET_SIZE: usize = 95;
pub const SYNC_WORD: u16 = 0xCAFE;
pub const SYNC_END: u8 = 0xBE;
pub const SCALE: f64 = 100.0;
pub const GPS_SCALE: f64 = 10_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FlightState {
    Idle = 0,
    Calibration = 1,
    Prelaunch = 2,
    Burn = 3,
    PassiveBurnout = 4,
    ActiveBurnout = 5,
    Apogee = 6,
    Parachute = 7,
    Landed = 8,
    GroundAbort = 9,
    DescentAbort = 10,
}

impl FlightState {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Idle),
            1 => Some(Self::Calibration),
            2 => Some(Self::Prelaunch),
            3 => Some(Self::Burn),
            4 => Some(Self::PassiveBurnout),
            5 => Some(Self::ActiveBurnout),
            6 => Some(Self::Apogee),
            7 => Some(Self::Parachute),
            8 => Some(Self::Landed),
            9 => Some(Self::GroundAbort),
            10 => Some(Self::DescentAbort),
            _ => None,
        }
    }
}

impl fmt::Display for FlightState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "IDLE"),
            Self::Calibration => write!(f, "CALIBRATION"),
            Self::Prelaunch => write!(f, "PRELAUNCH"),
            Self::Burn => write!(f, "BURN"),
            Self::PassiveBurnout => write!(f, "PASSIVE BURNOUT"),
            Self::ActiveBurnout => write!(f, "ACTIVE BURNOUT"),
            Self::Apogee => write!(f, "APOGEE"),
            Self::Parachute => write!(f, "PARACHUTE"),
            Self::Landed => write!(f, "LANDED"),
            Self::GroundAbort => write!(f, "GROUND ABORT"),
            Self::DescentAbort => write!(f, "DESCENT ABORT"),
        }
    }
}

pub const FAULT_NAMES: &[(u32, &str)] = &[
    (1 << 0, "BMP280 Idle Failed"),
    (1 << 1, "BMP280 Perf Failed"),
    (1 << 2, "BMP581 Idle Failed"),
    (1 << 3, "BMP581 Perf Failed"),
    (1 << 4, "IIM42653 Idle Failed"),
    (1 << 5, "IIM42653 Perf Failed"),
    (1 << 6, "IIS2MDCTR Idle Failed"),
    (1 << 7, "IIS2MDCTR Perf Failed"),
    (1 << 8, "SD Mount Failed"),
    (1 << 9, "SD Open Failed"),
];

pub fn active_faults(flags: u32) -> Vec<&'static str> {
    FAULT_NAMES
        .iter()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|(_, name)| *name)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayState {
    pub drogue_fired: bool,
    pub parachute_fired: bool,
}

impl RelayState {
    pub fn from_u8(v: u8) -> Self {
        Self {
            drogue_fired: v & 0x01 != 0,
            parachute_fired: v & 0x02 != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    None = 0x00,
    Reset = 0x01,
    GroundAbort = 0x02,
    Calibration = 0x03,
    Drogue = 0x04,
    Landed = 0x05,
}

impl Command {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::None),
            0x01 => Some(Self::Reset),
            0x02 => Some(Self::GroundAbort),
            0x03 => Some(Self::Calibration),
            0x04 => Some(Self::Drogue),
            0x05 => Some(Self::Landed),
            _ => None,
        }
    }

    pub fn command_byte(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "NONE"),
            Self::Reset => write!(f, "RESET"),
            Self::GroundAbort => write!(f, "GROUND ABORT"),
            Self::Calibration => write!(f, "CALIBRATION"),
            Self::Drogue => write!(f, "DROGUE"),
            Self::Landed => write!(f, "LANDED"),
        }
    }
}

pub fn build_command_frame(cmd: Command) -> [u8; 5] {
    let mut frame = [0u8; 5];
    let sync = SYNC_WORD.to_le_bytes();
    frame[0] = sync[0];
    frame[1] = sync[1];
    frame[2] = cmd.command_byte();
    frame[3] = 0x00;
    frame[4] = SYNC_END;
    frame
}

pub struct PacketField {
    pub name: &'static str,
    pub offset: usize,
    pub length: usize,
    pub color: [u8; 3],
}

pub const PACKET_FIELDS: &[PacketField] = &[
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
    PacketField { name: "SATS",      offset: 62, length: 1, color: [140, 255, 200] },
    PacketField { name: "BARO ALT",  offset: 63, length: 4, color: [255, 100, 200] },
    PacketField { name: "BARO VEL",  offset: 67, length: 4, color: [255, 130, 220] },
    PacketField { name: "VEL X",     offset: 71, length: 4, color: [200, 200, 100] },
    PacketField { name: "VEL Y",     offset: 75, length: 4, color: [220, 220, 100] },
    PacketField { name: "VEL Z",     offset: 79, length: 4, color: [240, 240, 100] },
    PacketField { name: "FLAGS",     offset: 83, length: 4, color: [255, 80, 80] },
    PacketField { name: "BATTERY",   offset: 87, length: 4, color: [255, 255, 0] },
    PacketField { name: "STATE",     offset: 91, length: 1, color: [0, 200, 255] },
    PacketField { name: "RELAY",     offset: 92, length: 1, color: [255, 165, 0] },
    PacketField { name: "CMD",       offset: 93, length: 1, color: [180, 180, 255] },
    PacketField { name: "SYNC END",  offset: 94, length: 1, color: [255, 255, 255] },
];

#[derive(Debug, Clone)]
pub struct Telemetry {
    pub raw: [u8; PACKET_SIZE],
    pub tick: u32,
    pub accel: [f64; 3],
    pub gyro: [f64; 3],
    pub mag: [f64; 3],
    pub pressure_pa: f64,
    pub temperature_c: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub gps_altitude: f64,
    pub satellites: u8,
    pub baro_altitude: f64,
    pub baro_velocity: f64,
    pub velocity: [f64; 3],
    pub flags: u32,
    pub battery_voltage: f64,
    pub state: FlightState,
    pub relay: RelayState,
    pub last_command: Command,
}

impl Telemetry {
    pub fn csv_header() -> &'static [&'static str] {
        &[
            "ground_timestamp",
            "tick",
            "accel_x", "accel_y", "accel_z",
            "gyro_x", "gyro_y", "gyro_z",
            "mag_x", "mag_y", "mag_z",
            "pressure_pa", "temperature_c",
            "latitude", "longitude", "gps_altitude",
            "satellites",
            "baro_altitude", "baro_velocity",
            "velocity_x", "velocity_y", "velocity_z",
            "flags", "battery_voltage",
            "state", "relay", "last_command",
        ]
    }

    pub fn csv_values(&self, ground_timestamp_ms: u128) -> Vec<String> {
        vec![
            ground_timestamp_ms.to_string(),
            self.tick.to_string(),
            self.accel[0].to_string(), self.accel[1].to_string(), self.accel[2].to_string(),
            self.gyro[0].to_string(), self.gyro[1].to_string(), self.gyro[2].to_string(),
            self.mag[0].to_string(), self.mag[1].to_string(), self.mag[2].to_string(),
            self.pressure_pa.to_string(), self.temperature_c.to_string(),
            self.latitude.to_string(), self.longitude.to_string(), self.gps_altitude.to_string(),
            self.satellites.to_string(),
            self.baro_altitude.to_string(), self.baro_velocity.to_string(),
            self.velocity[0].to_string(), self.velocity[1].to_string(), self.velocity[2].to_string(),
            self.flags.to_string(), self.battery_voltage.to_string(),
            (self.state as u8).to_string(),
            (self.relay.drogue_fired as u8 | ((self.relay.parachute_fired as u8) << 1)).to_string(),
            (self.last_command as u8).to_string(),
        ]
    }
}
