use std::collections::VecDeque;
use std::time::Instant;

use crate::telemetry::packet::{self, FlightState, Telemetry};

pub const MAX_DATA_POINTS: usize = 500;
const MAX_CONSOLE_ENTRIES: usize = 1000;

pub struct AppState {
    pub latest: Option<Telemetry>,
    pub baro_altitude: VecDeque<f64>,
    pub gps_altitude: VecDeque<f64>,
    pub baro_velocity: VecDeque<f64>,
    pub accel_x: VecDeque<f64>,
    pub accel_y: VecDeque<f64>,
    pub accel_z: VecDeque<f64>,
    pub gyro_x: VecDeque<f64>,
    pub gyro_y: VecDeque<f64>,
    pub gyro_z: VecDeque<f64>,
    pub gps_trail: VecDeque<(f64, f64)>,
    pub packet_count: u64,
    pub bytes_received: u64,
    pub throughput_kbps: f64,
    pub packets_per_sec: f64,
    throughput_bytes_window: u64,
    throughput_packets_window: u64,
    throughput_last_update: Instant,
    pub connected: bool,
    pub available_ports: Vec<String>,
    pub selected_port: String,
    pub selected_baud: u32,
    pub ground_pos: Option<(f64, f64)>,
    pub expected_packet_rate: u32,
    pub dark_mode: bool,
    pub lock_gps: bool,
    pub max_baro_altitude: f64,
    pub max_gps_altitude: f64,
    pub max_baro_velocity: f64,
    pub max_accel_magnitude: f64,
    pub max_temperature: f64,
    pub min_temperature: f64,
    pub min_battery_voltage: f64,
    pub max_battery_voltage: f64,
    pub extremes_initialized: bool,
    pub console: VecDeque<ConsoleEntry>,
    last_flight_state: Option<FlightState>,
    session_start: Instant,
}

pub enum ConsoleEntry {
    StateChange { state: FlightState, elapsed_secs: f64 },
    CommandSent { command: String, elapsed_secs: f64 },
    Message { text: String, elapsed_secs: f64 },
}

impl AppState {
    pub fn new() -> Self {
        Self {
            latest: None,
            baro_altitude: VecDeque::with_capacity(MAX_DATA_POINTS),
            gps_altitude: VecDeque::with_capacity(MAX_DATA_POINTS),
            baro_velocity: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_x: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_y: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_x: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_y: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            gps_trail: VecDeque::with_capacity(MAX_DATA_POINTS),
            packet_count: 0,
            bytes_received: 0,
            throughput_kbps: 0.0,
            packets_per_sec: 0.0,
            throughput_bytes_window: 0,
            throughput_packets_window: 0,
            throughput_last_update: Instant::now(),
            connected: false,
            available_ports: Vec::new(),
            selected_port: String::new(),
            selected_baud: 115200,
            ground_pos: None,
            expected_packet_rate: 10,
            dark_mode: true,
            lock_gps: true,
            max_baro_altitude: f64::NEG_INFINITY,
            max_gps_altitude: f64::NEG_INFINITY,
            max_baro_velocity: f64::NEG_INFINITY,
            max_accel_magnitude: 0.0,
            max_temperature: f64::NEG_INFINITY,
            min_temperature: f64::INFINITY,
            min_battery_voltage: f64::INFINITY,
            max_battery_voltage: f64::NEG_INFINITY,
            extremes_initialized: false,
            console: VecDeque::with_capacity(MAX_CONSOLE_ENTRIES),
            last_flight_state: None,
            session_start: Instant::now(),
        }
    }

    pub fn push_telemetry(&mut self, t: Telemetry) {
        fn push_bounded(buf: &mut VecDeque<f64>, val: f64) {
            if buf.len() >= MAX_DATA_POINTS {
                buf.pop_front();
            }
            buf.push_back(val);
        }

        push_bounded(&mut self.baro_altitude, t.baro_altitude);
        push_bounded(&mut self.gps_altitude, t.gps_altitude);
        push_bounded(&mut self.baro_velocity, t.baro_velocity);
        push_bounded(&mut self.accel_x, t.accel[0]);
        push_bounded(&mut self.accel_y, t.accel[1]);
        push_bounded(&mut self.accel_z, t.accel[2]);
        push_bounded(&mut self.gyro_x, t.gyro[0]);
        push_bounded(&mut self.gyro_y, t.gyro[1]);
        push_bounded(&mut self.gyro_z, t.gyro[2]);
        if t.latitude != 0.0 || t.longitude != 0.0 {
            if self.gps_trail.len() >= MAX_DATA_POINTS {
                self.gps_trail.pop_front();
            }
            self.gps_trail.push_back((t.latitude, t.longitude));
        }
        self.packet_count += 1;
        self.bytes_received += packet::PACKET_SIZE as u64;
        self.throughput_bytes_window += packet::PACKET_SIZE as u64;
        self.throughput_packets_window += 1;
        let elapsed = self.throughput_last_update.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.throughput_kbps = self.throughput_bytes_window as f64 / elapsed;
            self.packets_per_sec = self.throughput_packets_window as f64 / elapsed;
            self.throughput_bytes_window = 0;
            self.throughput_packets_window = 0;
            self.throughput_last_update = Instant::now();
        }
        let accel_mag = (t.accel[0] * t.accel[0] + t.accel[1] * t.accel[1] + t.accel[2] * t.accel[2]).sqrt();
        if !self.extremes_initialized {
            self.max_baro_altitude = t.baro_altitude;
            self.max_gps_altitude = t.gps_altitude;
            self.max_baro_velocity = t.baro_velocity;
            self.max_accel_magnitude = accel_mag;
            self.max_temperature = t.temperature_c;
            self.min_temperature = t.temperature_c;
            self.min_battery_voltage = t.battery_voltage;
            self.max_battery_voltage = t.battery_voltage;
            self.extremes_initialized = true;
        } else {
            self.max_baro_altitude = self.max_baro_altitude.max(t.baro_altitude);
            self.max_gps_altitude = self.max_gps_altitude.max(t.gps_altitude);
            self.max_baro_velocity = self.max_baro_velocity.max(t.baro_velocity);
            self.max_accel_magnitude = self.max_accel_magnitude.max(accel_mag);
            self.max_temperature = self.max_temperature.max(t.temperature_c);
            self.min_temperature = self.min_temperature.min(t.temperature_c);
            self.min_battery_voltage = self.min_battery_voltage.min(t.battery_voltage);
            self.max_battery_voltage = self.max_battery_voltage.max(t.battery_voltage);
        }

        let state_changed = self.last_flight_state.map_or(true, |s| s != t.state);
        if state_changed {
            self.push_console(ConsoleEntry::StateChange {
                state: t.state,
                elapsed_secs: self.session_start.elapsed().as_secs_f64(),
            });
            self.last_flight_state = Some(t.state);
        }

        self.latest = Some(t);
    }

    fn push_console(&mut self, entry: ConsoleEntry) {
        if self.console.len() >= MAX_CONSOLE_ENTRIES {
            self.console.pop_front();
        }
        self.console.push_back(entry);
    }

    pub fn push_message(&mut self, text: &str) {
        self.push_console(ConsoleEntry::Message {
            text: text.to_uppercase(),
            elapsed_secs: self.session_start.elapsed().as_secs_f64(),
        });
    }

    pub fn push_command(&mut self, cmd: &str) {
        self.push_console(ConsoleEntry::CommandSent {
            command: cmd.to_string(),
            elapsed_secs: self.session_start.elapsed().as_secs_f64(),
        });
    }
}
