use std::collections::VecDeque;

use crate::sd_log::parser::parse_sd_file;
use crate::sd_log::record::{SdRecord, TICK_RATE_HZ};
use crate::sd_viewer::charts;
use crate::telemetry::packet::{Command, FlightState};
use crate::ui::map::MapState;

pub struct StateSegment {
    pub start: f64,
    pub end: f64,
    pub state: FlightState,
    pub color: egui::Color32,
}

impl StateSegment {
    pub fn label(&self) -> &'static str {
        self.state.label()
    }
}

pub struct TimelineMarker {
    pub timestamp: f64,
    pub label: &'static str,
    pub color: egui::Color32,
    pub is_command: bool,
}

pub trait FlightStateExt {
    fn label(&self) -> &'static str;
    fn timeline_color(&self) -> egui::Color32;
}

impl FlightStateExt for FlightState {
    fn label(&self) -> &'static str {
        match self {
            FlightState::Idle => "IDLE",
            FlightState::Calibration => "CALIBRATION",
            FlightState::Prelaunch => "PRELAUNCH",
            FlightState::Burn => "BURN",
            FlightState::PassiveBurnout => "PASSIVE BURNOUT",
            FlightState::ActiveBurnout => "ACTIVE BURNOUT",
            FlightState::Apogee => "APOGEE",
            FlightState::Parachute => "PARACHUTE",
            FlightState::Landed => "LANDED",
            FlightState::GroundAbort => "GROUND ABORT",
            FlightState::DescentAbort => "DESCENT ABORT",
        }
    }

    fn timeline_color(&self) -> egui::Color32 {
        match self {
            FlightState::Idle => egui::Color32::from_rgb(120, 120, 120),
            FlightState::Calibration => egui::Color32::from_rgb(180, 180, 100),
            FlightState::Prelaunch => egui::Color32::from_rgb(100, 180, 220),
            FlightState::Burn => egui::Color32::from_rgb(255, 140, 0),
            FlightState::PassiveBurnout => egui::Color32::from_rgb(255, 200, 80),
            FlightState::ActiveBurnout => egui::Color32::from_rgb(230, 180, 60),
            FlightState::Apogee => egui::Color32::from_rgb(0, 200, 100),
            FlightState::Parachute => egui::Color32::from_rgb(0, 180, 255),
            FlightState::Landed => egui::Color32::from_rgb(80, 200, 80),
            FlightState::GroundAbort => egui::Color32::from_rgb(255, 50, 50),
            FlightState::DescentAbort => egui::Color32::from_rgb(255, 80, 80),
        }
    }
}

pub const REPLAY_SPEEDS: &[f64] = &[1.0, 1.5, 2.0, 4.0];

pub struct SdViewerState {
    pub records: Vec<SdRecord>,
    pub file_path: Option<String>,
    pub selected_index: usize,
    pub error: Option<String>,
    pub zoom_x: Option<(f64, f64)>,
    pub reset_zoom: bool,
    pub zoomed_segment: Option<(f64, f64)>,
    pub link_axes: bool,
    pub lock_gps: bool,
    pub timestamps: Vec<f64>,
    pub accel_x: Vec<f64>,
    pub accel_y: Vec<f64>,
    pub accel_z: Vec<f64>,
    pub gyro_x: Vec<f64>,
    pub gyro_y: Vec<f64>,
    pub gyro_z: Vec<f64>,
    pub gps_altitude: Vec<f64>,
    pub pressure: Vec<f64>,
    pub temperature: Vec<f64>,
    pub battery: Vec<f64>,
    pub state_segments: Vec<StateSegment>,
    pub timeline_markers: Vec<TimelineMarker>,
    pub gps_trail: VecDeque<(f64, f64)>,
    pub map_state: Option<MapState>,
    pub replay_playing: bool,
    pub replay_speed_index: usize,
    pub replay_last_wall: Option<f64>,
}

impl SdViewerState {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            file_path: None,
            selected_index: 0,
            error: None,
            zoom_x: None,
            reset_zoom: false,
            zoomed_segment: None,
            link_axes: true,
            lock_gps: true,
            timestamps: Vec::new(),
            accel_x: Vec::new(),
            accel_y: Vec::new(),
            accel_z: Vec::new(),
            gyro_x: Vec::new(),
            gyro_y: Vec::new(),
            gyro_z: Vec::new(),
            gps_altitude: Vec::new(),
            pressure: Vec::new(),
            temperature: Vec::new(),
            battery: Vec::new(),
            state_segments: Vec::new(),
            timeline_markers: Vec::new(),
            gps_trail: VecDeque::new(),
            map_state: None,
            replay_playing: false,
            replay_speed_index: 0,
            replay_last_wall: None,
        }
    }

    pub fn init_map(&mut self, ctx: &egui::Context) {
        if self.map_state.is_none() {
            self.map_state = Some(MapState::new(ctx));
        }
    }

    pub fn load_file(&mut self, path: &str) {
        self.error = None;
        self.records.clear();
        self.clear_series();

        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(format!("Failed to read file: {}", e));
                return;
            }
        };

        let records = parse_sd_file(&data);
        if records.is_empty() {
            self.error = Some("No valid SD records found in file".to_string());
            return;
        }

        self.extract_series(&records);
        self.build_state_segments(&records);
        self.detect_relay_events(&records);
        self.detect_command_events(&records);

        self.records = records;
        self.selected_index = 0;
        self.file_path = Some(path.to_string());
    }

    fn extract_series(&mut self, records: &[SdRecord]) {
        let count = records.len();
        self.timestamps.reserve(count);
        self.accel_x.reserve(count);
        self.accel_y.reserve(count);
        self.accel_z.reserve(count);
        self.gyro_x.reserve(count);
        self.gyro_y.reserve(count);
        self.gyro_z.reserve(count);
        self.gps_altitude.reserve(count);
        self.pressure.reserve(count);
        self.temperature.reserve(count);
        self.battery.reserve(count);

        for r in records {
            self.timestamps.push(r.tick as f64 / TICK_RATE_HZ);
            self.accel_x.push(r.accel[0]);
            self.accel_y.push(r.accel[1]);
            self.accel_z.push(r.accel[2]);
            self.gyro_x.push(r.gyro[0]);
            self.gyro_y.push(r.gyro[1]);
            self.gyro_z.push(r.gyro[2]);
            self.gps_altitude.push(r.gps_altitude);
            self.pressure.push(r.pressure_pa);
            self.temperature.push(r.temperature_c);
            self.battery.push(r.battery_voltage);
            if r.latitude != 0.0 || r.longitude != 0.0 {
                self.gps_trail.push_back((r.latitude, r.longitude));
            }
        }
    }

    fn build_state_segments(&mut self, records: &[SdRecord]) {
        if records.is_empty() {
            return;
        }
        let mut seg_start = records[0].tick as f64 / TICK_RATE_HZ;
        let mut seg_state = records[0].state;
        for r in &records[1..] {
            let ts = r.tick as f64 / TICK_RATE_HZ;
            if r.state != seg_state {
                self.state_segments.push(StateSegment {
                    start: seg_start,
                    end: ts,
                    state: seg_state,
                    color: seg_state.timeline_color(),
                });
                seg_start = ts;
                seg_state = r.state;
            }
        }
        self.state_segments.push(StateSegment {
            start: seg_start,
            end: records.last().unwrap().tick as f64 / TICK_RATE_HZ,
            state: seg_state,
            color: seg_state.timeline_color(),
        });
    }

    fn detect_relay_events(&mut self, records: &[SdRecord]) {
        let mut prev_drogue = false;
        let mut prev_chute = false;
        for r in records {
            let ts = r.tick as f64 / TICK_RATE_HZ;
            if r.relay.drogue_fired && !prev_drogue {
                self.timeline_markers.push(TimelineMarker {
                    timestamp: ts,
                    label: "DROGUE",
                    color: egui::Color32::from_rgb(255, 165, 0),
                    is_command: false,
                });
            }
            if r.relay.parachute_fired && !prev_chute {
                self.timeline_markers.push(TimelineMarker {
                    timestamp: ts,
                    label: "PARACHUTE",
                    color: egui::Color32::from_rgb(0, 200, 255),
                    is_command: false,
                });
            }
            prev_drogue = r.relay.drogue_fired;
            prev_chute = r.relay.parachute_fired;
        }
    }

    fn detect_command_events(&mut self, records: &[SdRecord]) {
        let mut prev_cmd = Command::None;
        for r in records {
            if r.last_command != prev_cmd && r.last_command != Command::None {
                let ts = r.tick as f64 / TICK_RATE_HZ;
                let (label, color) = match r.last_command {
                    Command::Reset => ("CMD: RESET", egui::Color32::from_rgb(200, 100, 200)),
                    Command::GroundAbort => ("CMD: GROUND ABORT", egui::Color32::from_rgb(255, 50, 50)),
                    Command::Calibration => ("CMD: CALIBRATION", egui::Color32::from_rgb(180, 180, 100)),
                    Command::Drogue => ("CMD: DROGUE", egui::Color32::from_rgb(255, 165, 0)),
                    Command::Landed => ("CMD: LANDED", egui::Color32::from_rgb(80, 200, 80)),
                    Command::None => unreachable!(),
                };
                self.timeline_markers.push(TimelineMarker {
                    timestamp: ts,
                    label,
                    color,
                    is_command: true,
                });
            }
            prev_cmd = r.last_command;
        }
    }

    pub fn selected_record(&self) -> Option<&SdRecord> {
        self.records.get(self.selected_index)
    }

    pub fn duration_secs(&self) -> f64 {
        if self.timestamps.len() < 2 {
            return 0.0;
        }
        self.timestamps.last().unwrap() - self.timestamps.first().unwrap()
    }

    pub fn tick_replay(&mut self, wall_now: f64) {
        if !self.replay_playing || self.timestamps.len() < 2 {
            return;
        }
        let last_wall = match self.replay_last_wall {
            Some(w) => w,
            None => {
                self.replay_last_wall = Some(wall_now);
                return;
            }
        };
        let wall_dt = wall_now - last_wall;
        if wall_dt <= 0.0 {
            return;
        }
        self.replay_last_wall = Some(wall_now);

        let current_t = self.timestamps[self.selected_index];
        let target_t = current_t + wall_dt * self.replay_speed();
        let new_index = charts::timestamp_to_index(&self.timestamps, target_t);

        if new_index >= self.timestamps.len().saturating_sub(1) {
            self.selected_index = self.timestamps.len().saturating_sub(1);
            self.replay_playing = false;
            self.replay_last_wall = None;
        } else {
            self.selected_index = new_index;
        }
    }

    pub fn replay_speed(&self) -> f64 {
        REPLAY_SPEEDS[self.replay_speed_index]
    }

    pub fn close_file(&mut self) {
        self.records.clear();
        self.file_path = None;
        self.error = None;
        self.replay_playing = false;
        self.replay_last_wall = None;
        self.clear_series();
    }

    fn clear_series(&mut self) {
        self.state_segments.clear();
        self.timeline_markers.clear();
        self.timestamps.clear();
        self.accel_x.clear();
        self.accel_y.clear();
        self.accel_z.clear();
        self.gyro_x.clear();
        self.gyro_y.clear();
        self.gyro_z.clear();
        self.gps_altitude.clear();
        self.pressure.clear();
        self.temperature.clear();
        self.battery.clear();
        self.gps_trail.clear();
        self.selected_index = 0;
    }
}
