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

#[derive(Debug, Clone)]
pub struct Telemetry {
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
