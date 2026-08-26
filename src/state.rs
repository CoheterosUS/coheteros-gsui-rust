use std::collections::VecDeque;
use std::time::Instant;

use crate::telemetry::packet::{self, Telemetry};

pub const MAX_DATA_POINTS: usize = 500;

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
    pub errors: VecDeque<String>,
    pub ground_pos: Option<(f64, f64)>,
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
            errors: VecDeque::with_capacity(20),
            ground_pos: None,
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
        self.latest = Some(t);
    }

    pub fn push_error(&mut self, msg: String) {
        if self.errors.len() >= 20 {
            self.errors.pop_front();
        }
        self.errors.push_back(msg);
    }
}
