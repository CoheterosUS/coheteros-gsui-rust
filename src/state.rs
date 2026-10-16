use std::collections::VecDeque;
use std::time::Instant;

use crate::telemetry::packet::{self, FlightState, Telemetry, TelemetrySource};

pub const MAX_DATA_POINTS: usize = 500;
const MAX_CONSOLE_ENTRIES: usize = 1000;

pub struct AppState {
    pub latest: Option<Telemetry>,
    pub baro_altitude: VecDeque<f64>,
    pub gps_altitude: VecDeque<f64>,
    pub pos_z: VecDeque<f64>,
    pub vel_z: VecDeque<f64>,
    pub kalman_pz: VecDeque<f64>,
    pub accel_x: VecDeque<f64>,
    pub accel_y: VecDeque<f64>,
    pub accel_z: VecDeque<f64>,
    pub gyro_x: VecDeque<f64>,
    pub gyro_y: VecDeque<f64>,
    pub gyro_z: VecDeque<f64>,
    pub battery: VecDeque<f64>,
    pub pressure: VecDeque<f64>,
    pub temperature: VecDeque<f64>,
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
    pub poll_interval_ms: u64,
    pub polling_active: bool,
    pub dark_mode: bool,
    pub lock_gps: bool,
    pub max_baro_altitude: f64,
    pub max_gps_altitude: f64,
    pub max_vert_velocity: f64,
    pub max_accel_magnitude: f64,
    pub min_battery_voltage: f64,
    pub max_battery_voltage: f64,
    pub extremes_initialized: bool,
    pub console: VecDeque<ConsoleEntry>,
    last_flight_state: Option<FlightState>,
    session_start: Instant,
    pub launch_site: Option<(f64, f64)>,
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
            pos_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            vel_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            kalman_pz: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_x: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_y: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_x: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_y: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            battery: VecDeque::with_capacity(MAX_DATA_POINTS),
            pressure: VecDeque::with_capacity(MAX_DATA_POINTS),
            temperature: VecDeque::with_capacity(MAX_DATA_POINTS),
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
            poll_interval_ms: 500,
            polling_active: false,
            dark_mode: true,
            lock_gps: true,
            max_baro_altitude: f64::NEG_INFINITY,
            max_gps_altitude: f64::NEG_INFINITY,
            max_vert_velocity: f64::NEG_INFINITY,
            max_accel_magnitude: 0.0,
            min_battery_voltage: f64::INFINITY,
            max_battery_voltage: f64::NEG_INFINITY,
            extremes_initialized: false,
            console: VecDeque::with_capacity(MAX_CONSOLE_ENTRIES),
            last_flight_state: None,
            session_start: Instant::now(),
            launch_site: None,
        }
    }

    pub fn push_telemetry(&mut self, t: Telemetry) {
        fn push_bounded(buf: &mut VecDeque<f64>, val: f64) {
            if buf.len() >= MAX_DATA_POINTS {
                buf.pop_front();
            }
            buf.push_back(val);
        }

        // Shared fields — always push (2 Hz)
        push_bounded(&mut self.accel_x, t.accel[0]);
        push_bounded(&mut self.accel_y, t.accel[1]);
        push_bounded(&mut self.accel_z, t.accel[2]);
        push_bounded(&mut self.gyro_x, t.gyro[0]);
        push_bounded(&mut self.gyro_y, t.gyro[1]);
        push_bounded(&mut self.gyro_z, t.gyro[2]);

        let is_a = t.source == TelemetrySource::A || t.source == TelemetrySource::Full;
        let is_b = t.source == TelemetrySource::B || t.source == TelemetrySource::Full;

        if is_a {
            push_bounded(&mut self.baro_altitude, t.baro_altitude);
            push_bounded(&mut self.pos_z, t.pos_z);
            push_bounded(&mut self.vel_z, t.vel_z);
            push_bounded(&mut self.kalman_pz, t.kalman_pz.max(0.0).sqrt());
        }

        if is_b {
            push_bounded(&mut self.gps_altitude, t.gps_altitude);
            push_bounded(&mut self.battery, t.battery_voltage);
            push_bounded(&mut self.pressure, t.pressure_pa);
            push_bounded(&mut self.temperature, t.temperature_c);
            if t.latitude != 0.0 || t.longitude != 0.0 {
                if self.gps_trail.len() >= MAX_DATA_POINTS {
                    self.gps_trail.pop_front();
                }
                self.gps_trail.push_back((t.latitude, t.longitude));
            }
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

        // Merge into latest: only overwrite fields that this packet carries
        let merged = if let Some(mut prev) = self.latest.take() {
            // Shared: always update
            prev.raw = t.raw;
            prev.source = t.source;
            prev.tick = t.tick;
            prev.accel = t.accel;
            prev.gyro = t.gyro;
            prev.quat = t.quat;
            prev.state = t.state;
            if is_a {
                prev.baro_altitude = t.baro_altitude;
                prev.pos_z = t.pos_z;
                prev.vel_z = t.vel_z;
                prev.kalman_pz = t.kalman_pz;
                prev.flags = t.flags;
                prev.relay = t.relay;
                prev.satellites = t.satellites;
                prev.last_command = t.last_command;
            }
            if is_b {
                prev.latitude = t.latitude;
                prev.longitude = t.longitude;
                prev.gps_altitude = t.gps_altitude;
                prev.unix_time = t.unix_time;
                prev.pressure_pa = t.pressure_pa;
                prev.temperature_c = t.temperature_c;
                prev.battery_voltage = t.battery_voltage;
            }
            prev
        } else {
            t
        };

        if !self.extremes_initialized {
            self.max_baro_altitude = merged.baro_altitude;
            self.max_gps_altitude = merged.gps_altitude;
            self.max_vert_velocity = -merged.vel_z;
            self.max_accel_magnitude = accel_mag;
            self.min_battery_voltage = merged.battery_voltage;
            self.max_battery_voltage = merged.battery_voltage;
            self.extremes_initialized = true;
        } else {
            if is_a {
                self.max_baro_altitude = self.max_baro_altitude.max(merged.baro_altitude);
                self.max_vert_velocity = self.max_vert_velocity.max(-merged.vel_z);
            }
            if is_b {
                self.max_gps_altitude = self.max_gps_altitude.max(merged.gps_altitude);
                self.min_battery_voltage = self.min_battery_voltage.min(merged.battery_voltage);
                self.max_battery_voltage = self.max_battery_voltage.max(merged.battery_voltage);
            }
            self.max_accel_magnitude = self.max_accel_magnitude.max(accel_mag);
        }

        let state_changed = self.last_flight_state.map_or(true, |s| s != merged.state);
        if state_changed {
            if merged.state == FlightState::Prelaunch && (merged.latitude != 0.0 || merged.longitude != 0.0) {
                self.launch_site = Some((merged.latitude, merged.longitude));
            }
            self.push_console(ConsoleEntry::StateChange {
                state: merged.state,
                elapsed_secs: self.session_start.elapsed().as_secs_f64(),
            });
            self.last_flight_state = Some(merged.state);
        }

        self.latest = Some(merged);
    }

    fn push_console(&mut self, entry: ConsoleEntry) {
        if self.console.len() >= MAX_CONSOLE_ENTRIES {
            self.console.pop_front();
        }
        self.console.push_back(entry);
    }

    pub fn clear_data(&mut self) {
        self.latest = None;
        self.baro_altitude.clear();
        self.gps_altitude.clear();
        self.pos_z.clear();
        self.vel_z.clear();
        self.kalman_pz.clear();
        self.accel_x.clear();
        self.accel_y.clear();
        self.accel_z.clear();
        self.gyro_x.clear();
        self.gyro_y.clear();
        self.gyro_z.clear();
        self.battery.clear();
        self.pressure.clear();
        self.temperature.clear();
        self.gps_trail.clear();
        self.packet_count = 0;
        self.bytes_received = 0;
        self.throughput_kbps = 0.0;
        self.packets_per_sec = 0.0;
        self.throughput_bytes_window = 0;
        self.throughput_packets_window = 0;
        self.throughput_last_update = Instant::now();
        self.max_baro_altitude = f64::NEG_INFINITY;
        self.max_gps_altitude = f64::NEG_INFINITY;
        self.max_vert_velocity = f64::NEG_INFINITY;
        self.max_accel_magnitude = 0.0;
        self.min_battery_voltage = f64::INFINITY;
        self.max_battery_voltage = f64::NEG_INFINITY;
        self.extremes_initialized = false;
        self.console.clear();
        self.last_flight_state = None;
        self.session_start = Instant::now();
        self.launch_site = None;
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

    pub fn downrange(&self) -> Option<f64> {
        let (lat1, lon1) = self.launch_site?;
        let t = self.latest.as_ref()?;
        if t.latitude == 0.0 && t.longitude == 0.0 {
            return None;
        }
        Some(haversine_m(lat1, lon1, t.latitude, t.longitude))
    }

    pub fn distance_from_gs(&self) -> Option<f64> {
        let (lat1, lon1) = self.ground_pos?;
        let t = self.latest.as_ref()?;
        if t.latitude == 0.0 && t.longitude == 0.0 {
            return None;
        }
        Some(haversine_m(lat1, lon1, t.latitude, t.longitude))
    }
}

fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_371_000.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    r * 2.0 * a.sqrt().asin()
}
