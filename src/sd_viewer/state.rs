use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

use crate::sd_log::csv::{parse_csv_file_with_progress, CsvLayout, CsvLog};
use crate::sd_log::parser::{parse_sd_file_with_progress, parse_flash_file_with_progress};
use crate::sd_log::record::{SdRecord, TICK_RATE_HZ};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    SdLog,
    FlashLog,
    Csv,
}
use crate::sd_viewer::charts;
use crate::telemetry::packet::{Command, FlightState};
use crate::ui::map::MapState;

pub enum BgTaskResult {
    /// Records, flight-start record indices (flash boundary markers), path, source.
    Loaded(Vec<SdRecord>, Vec<usize>, String, DataSource),
    LoadedCsv(CsvLog, String),
    Exported(String),
    Cancelled,
    Error(String),
}

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

pub struct GapInfo {
    pub start: f64,
    pub end: f64,
    pub dropped: u64,
    pub delta_ticks: u32,
    pub expected_ticks: u32,
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
            FlightState::Boost => "BOOST",
            FlightState::Coast => "COAST",
            FlightState::ActiveControl => "ACTIVE CONTROL",
            FlightState::Apogee => "APOGEE",
            FlightState::MainParachute => "MAIN PARACHUTE",
            FlightState::Landed => "LANDED",
            FlightState::GroundAbort => "GROUND ABORT",
            FlightState::DescentAbort => "DESCENT ABORT",
            FlightState::AscentAbort => "ASCENT ABORT",
            FlightState::DeepCalibration => "DEEP CALIBRATION",
        }
    }

    fn timeline_color(&self) -> egui::Color32 {
        match self {
            FlightState::Idle => egui::Color32::from_rgb(120, 120, 120),
            FlightState::Calibration => egui::Color32::from_rgb(180, 180, 100),
            FlightState::Prelaunch => egui::Color32::from_rgb(100, 180, 220),
            FlightState::Boost => egui::Color32::from_rgb(255, 140, 0),
            FlightState::Coast => egui::Color32::from_rgb(255, 200, 80),
            FlightState::ActiveControl => egui::Color32::from_rgb(230, 180, 60),
            FlightState::Apogee => egui::Color32::from_rgb(0, 200, 100),
            FlightState::MainParachute => egui::Color32::from_rgb(0, 180, 255),
            FlightState::Landed => egui::Color32::from_rgb(80, 200, 80),
            FlightState::GroundAbort => egui::Color32::from_rgb(255, 50, 50),
            FlightState::DescentAbort => egui::Color32::from_rgb(255, 80, 80),
            FlightState::AscentAbort => egui::Color32::from_rgb(255, 60, 60),
            FlightState::DeepCalibration => egui::Color32::from_rgb(160, 120, 200),
        }
    }
}

pub const REPLAY_SPEEDS: &[f64] = &[1.0, 1.5, 2.0, 4.0];
pub const DEFAULT_CHART_POINTS: usize = 10_000;

pub struct SdViewerState {
    pub records: Vec<SdRecord>,
    pub data_source: DataSource,
    pub csv_layout: CsvLayout,
    pub file_path: Option<String>,
    pub selected_index: usize,
    pub error: Option<String>,
    pub zoom_x: Option<(f64, f64)>,
    pub reset_zoom: bool,
    pub zoomed_segment: Option<(f64, f64)>,
    pub link_axes: bool,
    pub lock_gps: bool,
    pub map_3d_mode: bool,
    pub timestamps: Vec<f64>,
    pub accel_x: Vec<f64>,
    pub accel_y: Vec<f64>,
    pub accel_z: Vec<f64>,
    pub gyro_x: Vec<f64>,
    pub gyro_y: Vec<f64>,
    pub gyro_z: Vec<f64>,
    pub mag_x: Vec<f64>,
    pub mag_y: Vec<f64>,
    pub mag_z: Vec<f64>,
    pub gps_altitude: Vec<f64>,
    pub pressure: Vec<f64>,
    pub temperature: Vec<f64>,
    pub battery: Vec<f64>,
    pub vel_x: Vec<f64>,
    pub vel_y: Vec<f64>,
    pub vel_z: Vec<f64>,
    pub pos_x: Vec<f64>,
    pub pos_y: Vec<f64>,
    pub pos_z: Vec<f64>,
    pub quat_w: Vec<f64>,
    pub quat_x: Vec<f64>,
    pub quat_y: Vec<f64>,
    pub quat_z: Vec<f64>,
    /// 1-sigma from the Kalman pos-Z variance P[2,2], in meters
    pub kalman_pz_sigma: Vec<f64>,
    pub baro_altitude: Vec<f64>,
    pub baro_altitude_raw: Vec<f64>,
    pub baro_velocity: Vec<f64>,
    pub gaps: Vec<GapInfo>,
    pub state_segments: Vec<StateSegment>,
    pub timeline_markers: Vec<TimelineMarker>,
    pub gps_trail: VecDeque<(f64, f64)>,
    pub map_state: Option<MapState>,
    pub replay_playing: bool,
    pub replay_speed_index: usize,
    pub chart_points: usize,
    pub replay_last_wall: Option<f64>,
    pub bg_receiver: Option<mpsc::Receiver<BgTaskResult>>,
    pub bg_label: Option<String>,
    pub bg_progress: Option<Arc<AtomicUsize>>,
    pub bg_total: usize,
    pub status_message: Option<String>,
    pub last_export_dir: Option<String>,
    pub bg_start_time: Option<Instant>,
    pub bg_cancel: Option<Arc<AtomicBool>>,
    pub detached_charts: HashSet<String>,
    pub trim_start: usize,
    pub trim_end: usize,
    pub trim_active: bool,
    pub max_baro_altitude: f64,
    pub max_gps_altitude: f64,
    pub max_vert_velocity: f64,
    pub max_accel_magnitude: f64,
    pub min_battery_voltage: f64,
    pub max_battery_voltage: f64,
}

impl SdViewerState {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            data_source: DataSource::SdLog,
            csv_layout: CsvLayout::default(),
            file_path: None,
            selected_index: 0,
            error: None,
            zoom_x: None,
            reset_zoom: false,
            zoomed_segment: None,
            link_axes: true,
            lock_gps: true,
            map_3d_mode: false,
            timestamps: Vec::new(),
            accel_x: Vec::new(),
            accel_y: Vec::new(),
            accel_z: Vec::new(),
            gyro_x: Vec::new(),
            gyro_y: Vec::new(),
            gyro_z: Vec::new(),
            mag_x: Vec::new(),
            mag_y: Vec::new(),
            mag_z: Vec::new(),
            gps_altitude: Vec::new(),
            pressure: Vec::new(),
            temperature: Vec::new(),
            battery: Vec::new(),
            vel_x: Vec::new(),
            vel_y: Vec::new(),
            vel_z: Vec::new(),
            pos_x: Vec::new(),
            pos_y: Vec::new(),
            pos_z: Vec::new(),
            quat_w: Vec::new(),
            quat_x: Vec::new(),
            quat_y: Vec::new(),
            quat_z: Vec::new(),
            kalman_pz_sigma: Vec::new(),
            baro_altitude: Vec::new(),
            baro_altitude_raw: Vec::new(),
            baro_velocity: Vec::new(),
            gaps: Vec::new(),
            state_segments: Vec::new(),
            timeline_markers: Vec::new(),
            gps_trail: VecDeque::new(),
            map_state: None,
            replay_playing: false,
            replay_speed_index: 0,
            chart_points: DEFAULT_CHART_POINTS,
            replay_last_wall: None,
            bg_receiver: None,
            bg_label: None,
            bg_progress: None,
            bg_total: 0,
            status_message: None,
            last_export_dir: None,
            bg_start_time: None,
            bg_cancel: None,
            detached_charts: HashSet::new(),
            trim_start: 0,
            trim_end: 0,
            trim_active: false,
            max_baro_altitude: 0.0,
            max_gps_altitude: 0.0,
            max_vert_velocity: 0.0,
            max_accel_magnitude: 0.0,
            min_battery_voltage: 0.0,
            max_battery_voltage: 0.0,
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
        self.bg_label = Some("LOADING".to_string());
        self.bg_start_time = Some(Instant::now());

        self.bg_total = std::fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);
        let progress = Arc::new(AtomicUsize::new(0));
        self.bg_progress = Some(Arc::clone(&progress));

        let (tx, rx) = mpsc::channel();
        let path_owned = path.to_string();
        std::thread::spawn(move || {
            let data = match std::fs::read(&path_owned) {
                Ok(d) => d,
                Err(e) => {
                    let _ = tx.send(BgTaskResult::Error(format!("Failed to read file: {}", e)));
                    return;
                }
            };
            let sd_records = parse_sd_file_with_progress(&data, Some(&progress));
            if !sd_records.is_empty() {
                let _ = tx.send(BgTaskResult::Loaded(sd_records, Vec::new(), path_owned, DataSource::SdLog));
                return;
            }
            progress.store(0, Ordering::Relaxed);
            let (flash_records, flight_starts) = parse_flash_file_with_progress(&data, Some(&progress));
            if !flash_records.is_empty() {
                let _ = tx.send(BgTaskResult::Loaded(flash_records, flight_starts, path_owned, DataSource::FlashLog));
                return;
            }
            let _ = tx.send(BgTaskResult::Error("No valid SD or Flash records found in file".to_string()));
        });
        self.bg_receiver = Some(rx);
    }

    pub fn load_csv_file(&mut self, path: &str) {
        self.error = None;
        self.records.clear();
        self.clear_series();
        self.bg_label = Some("LOADING".to_string());
        self.bg_start_time = Some(Instant::now());

        self.bg_total = std::fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);
        let progress = Arc::new(AtomicUsize::new(0));
        self.bg_progress = Some(Arc::clone(&progress));

        let (tx, rx) = mpsc::channel();
        let path_owned = path.to_string();
        std::thread::spawn(move || {
            let data = match std::fs::read(&path_owned) {
                Ok(d) => d,
                Err(e) => {
                    let _ = tx.send(BgTaskResult::Error(format!("Failed to read file: {}", e)));
                    return;
                }
            };
            match parse_csv_file_with_progress(&data, Some(&progress)) {
                Ok(log) => { let _ = tx.send(BgTaskResult::LoadedCsv(log, path_owned)); }
                Err(e) => { let _ = tx.send(BgTaskResult::Error(e)); }
            }
        });
        self.bg_receiver = Some(rx);
    }

    fn finish_load(&mut self, records: Vec<SdRecord>, flight_starts: Vec<usize>, path: String, source: DataSource) {
        self.data_source = source;
        self.extract_series(&records);
        self.compute_extremes(&records);
        self.build_state_segments(&records);
        self.detect_gaps(&records);
        for idx in flight_starts {
            if let Some(r) = records.get(idx) {
                self.timeline_markers.push(TimelineMarker {
                    timestamp: r.tick as f64 / TICK_RATE_HZ,
                    label: "FLIGHT START",
                    color: egui::Color32::from_rgb(220, 220, 220),
                    is_command: false,
                });
            }
        }
        if self.has_full_data() {
            self.detect_relay_events(&records);
            self.detect_command_events(&records);
        }
        self.records = records;
        self.selected_index = 0;
        self.trim_start = 0;
        self.trim_end = self.records.len().saturating_sub(1);
        self.trim_active = false;
        self.file_path = Some(path);
        self.status_message = Some(format!("LOADED {} {} RECORDS", self.records.len(), self.source_label()));
    }

    fn finish_load_csv(&mut self, log: CsvLog, path: String) {
        self.csv_layout = log.layout;
        self.finish_load(log.records, Vec::new(), path, DataSource::Csv);
        // Prefer the baro values stored in the CSV (firmware or previous export) over recomputing.
        if log.layout.baro {
            self.baro_altitude = log.baro_altitude;
            if log.layout.baro_vel {
                self.baro_velocity = log.baro_velocity;
            }
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self.data_source {
            DataSource::SdLog => "SD",
            DataSource::FlashLog => "FLASH",
            DataSource::Csv => "CSV",
        }
    }

    pub fn start_export(&mut self, path: &str) {
        self.error = None;
        self.bg_label = Some("EXPORTING".to_string());
        self.bg_start_time = Some(Instant::now());
        self.bg_total = self.records.len();
        let progress = Arc::new(AtomicUsize::new(0));
        self.bg_progress = Some(Arc::clone(&progress));
        let cancel = Arc::new(AtomicBool::new(false));
        self.bg_cancel = Some(Arc::clone(&cancel));

        let (tx, rx) = mpsc::channel();
        let records = self.records.clone();
        let baro_altitude = self.baro_altitude.clone();
        let baro_velocity = self.baro_velocity.clone();
        let source = self.data_source;
        let path_owned = path.to_string();
        std::thread::spawn(move || {
            let result = export_csv_to_file(&path_owned, &records, &baro_altitude, &baro_velocity, source, Some(&progress), Some(&cancel));
            match result {
                Ok(true) => { let _ = tx.send(BgTaskResult::Exported(path_owned)); }
                Ok(false) => { let _ = tx.send(BgTaskResult::Cancelled); }
                Err(e) => { let _ = tx.send(BgTaskResult::Error(e)); }
            }
        });
        self.bg_receiver = Some(rx);
    }

    pub fn cancel_export(&mut self) {
        if let Some(ref cancel) = self.bg_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn poll_task(&mut self) -> bool {
        let rx = match self.bg_receiver.as_ref() {
            Some(rx) => rx,
            None => return false,
        };
        match rx.try_recv() {
            Ok(result) => {
                self.bg_receiver = None;
                self.bg_label = None;
                self.bg_progress = None;
                self.bg_total = 0;
                self.bg_start_time = None;
                self.bg_cancel = None;
                match result {
                    BgTaskResult::Loaded(records, flight_starts, path, source) => self.finish_load(records, flight_starts, path, source),
                    BgTaskResult::LoadedCsv(log, path) => self.finish_load_csv(log, path),
                    BgTaskResult::Exported(path) => {
                        let p = std::path::Path::new(&path);
                        let name = p.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.clone());
                        self.last_export_dir = p.parent()
                            .map(|d| d.to_string_lossy().to_string());
                        self.status_message = Some(format!("EXPORTED TO {}", name.to_uppercase()));
                    }
                    BgTaskResult::Cancelled => {
                        self.status_message = Some("EXPORT CANCELLED".to_string());
                    }
                    BgTaskResult::Error(e) => self.error = Some(e),
                }
                true
            }
            Err(mpsc::TryRecvError::Empty) => true,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.bg_receiver = None;
                self.bg_label = None;
                self.bg_progress = None;
                self.bg_total = 0;
                self.bg_start_time = None;
                self.bg_cancel = None;
                self.error = Some("BACKGROUND TASK FAILED".to_string());
                false
            }
        }
    }

    pub fn is_busy(&self) -> bool {
        self.bg_receiver.is_some()
    }

    pub fn progress_fraction(&self) -> f32 {
        match (&self.bg_progress, self.bg_total) {
            (Some(p), total) if total > 0 => {
                (p.load(Ordering::Relaxed) as f32 / total as f32).min(1.0)
            }
            _ => 0.0,
        }
    }

    pub fn progress_current(&self) -> usize {
        self.bg_progress.as_ref().map_or(0, |p| p.load(Ordering::Relaxed))
    }

    pub fn progress_eta_secs(&self) -> Option<f64> {
        let frac = self.progress_fraction() as f64;
        if frac <= 0.0 || frac >= 1.0 {
            return None;
        }
        let elapsed = self.bg_start_time?.elapsed().as_secs_f64();
        let total_est = elapsed / frac;
        Some((total_est - elapsed).max(0.0))
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
        self.mag_x.reserve(count);
        self.mag_y.reserve(count);
        self.mag_z.reserve(count);
        self.gps_altitude.reserve(count);
        self.pressure.reserve(count);
        self.temperature.reserve(count);
        self.battery.reserve(count);
        self.vel_x.reserve(count);
        self.vel_y.reserve(count);
        self.vel_z.reserve(count);
        self.pos_x.reserve(count);
        self.pos_y.reserve(count);
        self.pos_z.reserve(count);
        self.quat_w.reserve(count);
        self.quat_x.reserve(count);
        self.quat_y.reserve(count);
        self.quat_z.reserve(count);
        self.kalman_pz_sigma.reserve(count);
        self.baro_altitude.reserve(count);
        self.baro_altitude_raw.reserve(count);
        self.baro_velocity.reserve(count);

        let ref_pressure = Self::compute_reference_pressure(records);

        const R: f64 = 287.0;
        const G: f64 = 9.80665;
        const ALPHA: f64 = 0.1;
        const ALPHA_VEL: f64 = 0.05;
        let mut filtered_alt: Option<f64> = None;
        let mut filtered_vel: Option<f64> = None;
        let mut prev_alt: Option<f64> = None;
        let mut prev_tick: Option<u32> = None;

        for r in records {
            self.timestamps.push(r.tick as f64 / TICK_RATE_HZ);
            self.accel_x.push(r.accel[0]);
            self.accel_y.push(r.accel[1]);
            self.accel_z.push(r.accel[2]);
            self.gyro_x.push(r.gyro[0]);
            self.gyro_y.push(r.gyro[1]);
            self.gyro_z.push(r.gyro[2]);
            self.mag_x.push(r.mag[0]);
            self.mag_y.push(r.mag[1]);
            self.mag_z.push(r.mag[2]);
            self.vel_x.push(r.vel[0]);
            self.vel_y.push(r.vel[1]);
            self.vel_z.push(r.vel[2]);
            self.pos_x.push(r.pos[0]);
            self.pos_y.push(r.pos[1]);
            self.pos_z.push(r.pos[2]);
            self.quat_w.push(r.quat[0]);
            self.quat_x.push(r.quat[1]);
            self.quat_y.push(r.quat[2]);
            self.quat_z.push(r.quat[3]);
            self.kalman_pz_sigma.push(r.kalman_pz.max(0.0).sqrt());
            self.gps_altitude.push(r.gps_altitude);
            self.pressure.push(r.pressure_pa);
            self.temperature.push(r.temperature_c);
            self.battery.push(r.battery_voltage);
            if r.latitude != 0.0 || r.longitude != 0.0 {
                self.gps_trail.push_back((r.latitude, r.longitude));
            }

            if r.pressure_pa > 0.0 && ref_pressure > 0.0 {
                let temp_k = r.temperature_c + 273.15;
                let raw_alt = (R * temp_k / G) * (ref_pressure / r.pressure_pa).ln();
                self.baro_altitude_raw.push(raw_alt);
                let filt = match filtered_alt {
                    Some(prev) => prev + ALPHA * (raw_alt - prev),
                    None => raw_alt,
                };
                filtered_alt = Some(filt);

                let raw_vel = match (prev_alt, prev_tick) {
                    (Some(pa), Some(pt)) => {
                        let dt = (r.tick.wrapping_sub(pt)) as f64 / TICK_RATE_HZ;
                        if dt > 0.0 { (filt - pa) / dt } else { 0.0 }
                    }
                    _ => 0.0,
                };
                let vel = match filtered_vel {
                    Some(fv) => fv + ALPHA_VEL * (raw_vel - fv),
                    None => raw_vel,
                };
                filtered_vel = Some(vel);
                prev_alt = Some(filt);
                prev_tick = Some(r.tick);

                self.baro_altitude.push(filt);
                self.baro_velocity.push(vel);
            } else {
                self.baro_altitude.push(0.0);
                self.baro_altitude_raw.push(0.0);
                self.baro_velocity.push(0.0);
            }
        }
    }

    fn compute_reference_pressure(records: &[SdRecord]) -> f64 {
        let prelaunch: Vec<f64> = records.iter()
            .filter(|r| r.state == FlightState::Prelaunch || r.state == FlightState::Calibration)
            .take(2000)
            .map(|r| r.pressure_pa)
            .filter(|&p| p > 0.0)
            .collect();
        if prelaunch.is_empty() {
            records.iter()
                .take(2000)
                .map(|r| r.pressure_pa)
                .filter(|&p| p > 0.0)
                .sum::<f64>()
                / records.iter().take(2000).filter(|r| r.pressure_pa > 0.0).count().max(1) as f64
        } else {
            prelaunch.iter().sum::<f64>() / prelaunch.len() as f64
        }
    }

    fn compute_extremes(&mut self, records: &[SdRecord]) {
        let mut max_baro = f64::NEG_INFINITY;
        let mut max_gps = f64::NEG_INFINITY;
        let mut max_vel = f64::NEG_INFINITY;
        let mut max_accel = 0.0_f64;
        let mut min_bat = f64::INFINITY;
        let mut max_bat = f64::NEG_INFINITY;
        let mut has_any = false;

        for (i, r) in records.iter().enumerate() {
            has_any = true;
            let baro = self.baro_altitude.get(i).copied().unwrap_or(0.0);
            max_baro = max_baro.max(baro);
            max_gps = max_gps.max(r.gps_altitude);
            let bv = self.baro_velocity.get(i).copied().unwrap_or(0.0);
            max_vel = max_vel.max(-bv);
            let mag = (r.accel[0] * r.accel[0] + r.accel[1] * r.accel[1] + r.accel[2] * r.accel[2]).sqrt();
            max_accel = max_accel.max(mag);
            if r.battery_voltage > 0.0 {
                min_bat = min_bat.min(r.battery_voltage);
                max_bat = max_bat.max(r.battery_voltage);
            }
        }

        if has_any {
            self.max_baro_altitude = max_baro;
            self.max_gps_altitude = max_gps;
            self.max_vert_velocity = max_vel;
            self.max_accel_magnitude = max_accel;
            self.min_battery_voltage = if min_bat.is_finite() { min_bat } else { 0.0 };
            self.max_battery_voltage = if max_bat.is_finite() { max_bat } else { 0.0 };
        }
    }

    fn detect_gaps(&mut self, records: &[SdRecord]) {
        if records.len() < 2 {
            return;
        }

        let mut deltas: Vec<u32> = Vec::with_capacity(records.len() - 1);
        for w in records.windows(2) {
            deltas.push(w[1].tick.wrapping_sub(w[0].tick));
        }

        let expected = {
            let mut sorted = deltas.clone();
            sorted.sort_unstable();
            sorted[sorted.len() / 2]
        };

        for (i, &delta) in deltas.iter().enumerate() {
            if delta > expected * 2 {
                let start = records[i].tick as f64 / TICK_RATE_HZ;
                let end = records[i + 1].tick as f64 / TICK_RATE_HZ;
                let dropped = (delta / expected).saturating_sub(1) as u64;
                self.gaps.push(GapInfo {
                    start,
                    end,
                    dropped,
                    delta_ticks: delta,
                    expected_ticks: expected,
                });
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
                    Command::DeepCalibration => ("CMD: DEEP CAL", egui::Color32::from_rgb(160, 120, 200)),
                    Command::HilStart => ("CMD: HIL START", egui::Color32::from_rgb(100, 180, 220)),
                    Command::RequestTelemA | Command::RequestTelemB => continue,
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

    pub fn has_full_data(&self) -> bool {
        match self.data_source {
            DataSource::SdLog => true,
            DataSource::FlashLog => false,
            DataSource::Csv => self.csv_layout.full,
        }
    }

    pub fn has_mag(&self) -> bool {
        match self.data_source {
            DataSource::SdLog => true,
            DataSource::FlashLog => false,
            DataSource::Csv => self.csv_layout.mag,
        }
    }

    pub fn has_pressure(&self) -> bool {
        match self.data_source {
            DataSource::SdLog => true,
            DataSource::FlashLog => false,
            DataSource::Csv => self.csv_layout.pressure,
        }
    }

    pub fn has_utc(&self) -> bool {
        match self.data_source {
            DataSource::SdLog => true,
            DataSource::FlashLog => false,
            DataSource::Csv => self.csv_layout.time,
        }
    }

    pub fn has_kalman(&self) -> bool {
        match self.data_source {
            DataSource::SdLog => true,
            DataSource::FlashLog => false,
            DataSource::Csv => self.vel_x.iter().any(|&v| v != 0.0),
        }
    }

    pub fn has_raw(&self) -> bool {
        self.data_source != DataSource::Csv
    }

    pub fn start_export_trimmed_bin(&mut self, path: &str) {
        let start = self.trim_start;
        let end = (self.trim_end + 1).min(self.records.len());
        if start >= end {
            self.error = Some("INVALID TRIM RANGE".to_string());
            return;
        }
        self.error = None;
        self.bg_label = Some("EXPORTING BIN".to_string());
        self.bg_start_time = Some(Instant::now());
        let count = end - start;
        self.bg_total = count;
        let progress = Arc::new(AtomicUsize::new(0));
        self.bg_progress = Some(Arc::clone(&progress));
        let cancel = Arc::new(AtomicBool::new(false));
        self.bg_cancel = Some(Arc::clone(&cancel));

        let (tx, rx) = mpsc::channel();
        let records: Vec<Vec<u8>> = self.records[start..end].iter().map(|r| r.raw.clone()).collect();
        let path_owned = path.to_string();
        std::thread::spawn(move || {
            let mut file = match std::fs::File::create(&path_owned) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(BgTaskResult::Error(format!("Failed to create file: {}", e)));
                    return;
                }
            };
            for (i, raw) in records.iter().enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    let _ = tx.send(BgTaskResult::Cancelled);
                    return;
                }
                if let Err(e) = file.write_all(raw) {
                    let _ = tx.send(BgTaskResult::Error(format!("Write error: {}", e)));
                    return;
                }
                progress.store(i + 1, Ordering::Relaxed);
            }
            let _ = tx.send(BgTaskResult::Exported(path_owned));
        });
        self.bg_receiver = Some(rx);
    }

    pub fn start_export_trimmed_csv(&mut self, path: &str) {
        let start = self.trim_start;
        let end = (self.trim_end + 1).min(self.records.len());
        if start >= end {
            self.error = Some("INVALID TRIM RANGE".to_string());
            return;
        }
        self.error = None;
        self.bg_label = Some("EXPORTING CSV".to_string());
        self.bg_start_time = Some(Instant::now());
        let count = end - start;
        self.bg_total = count;
        let progress = Arc::new(AtomicUsize::new(0));
        self.bg_progress = Some(Arc::clone(&progress));
        let cancel = Arc::new(AtomicBool::new(false));
        self.bg_cancel = Some(Arc::clone(&cancel));

        let (tx, rx) = mpsc::channel();
        let records: Vec<SdRecord> = self.records[start..end].to_vec();
        let baro_altitude: Vec<f64> = self.baro_altitude.get(start..end).unwrap_or(&[]).to_vec();
        let baro_velocity: Vec<f64> = self.baro_velocity.get(start..end).unwrap_or(&[]).to_vec();
        let source = self.data_source;
        let path_owned = path.to_string();
        std::thread::spawn(move || {
            let result = export_csv_to_file(&path_owned, &records, &baro_altitude, &baro_velocity, source, Some(&progress), Some(&cancel));
            match result {
                Ok(true) => { let _ = tx.send(BgTaskResult::Exported(path_owned)); }
                Ok(false) => { let _ = tx.send(BgTaskResult::Cancelled); }
                Err(e) => { let _ = tx.send(BgTaskResult::Error(e)); }
            }
        });
        self.bg_receiver = Some(rx);
    }

    pub fn close_file(&mut self) {
        self.records.clear();
        self.file_path = None;
        self.error = None;
        self.replay_playing = false;
        self.replay_last_wall = None;
        self.trim_active = false;
        self.trim_start = 0;
        self.trim_end = 0;
        self.clear_series();
    }

    fn clear_series(&mut self) {
        self.detached_charts.clear();
        self.state_segments.clear();
        self.timeline_markers.clear();
        self.timestamps.clear();
        self.accel_x.clear();
        self.accel_y.clear();
        self.accel_z.clear();
        self.gyro_x.clear();
        self.gyro_y.clear();
        self.gyro_z.clear();
        self.mag_x.clear();
        self.mag_y.clear();
        self.mag_z.clear();
        self.gps_altitude.clear();
        self.pressure.clear();
        self.temperature.clear();
        self.battery.clear();
        self.vel_x.clear();
        self.vel_y.clear();
        self.vel_z.clear();
        self.pos_x.clear();
        self.pos_y.clear();
        self.pos_z.clear();
        self.quat_w.clear();
        self.quat_x.clear();
        self.quat_y.clear();
        self.quat_z.clear();
        self.kalman_pz_sigma.clear();
        self.baro_altitude.clear();
        self.baro_altitude_raw.clear();
        self.baro_velocity.clear();
        self.gaps.clear();
        self.gps_trail.clear();
        self.selected_index = 0;
    }

}

fn export_csv_to_file(path: &str, records: &[SdRecord], baro_altitude: &[f64], baro_velocity: &[f64], source: DataSource, progress: Option<&Arc<AtomicUsize>>, cancel: Option<&Arc<AtomicBool>>) -> Result<bool, String> {
    let mut file = std::fs::File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
    match source {
        DataSource::SdLog => {
            writeln!(file, "tick,accel_x,accel_y,accel_z,gyro_x,gyro_y,gyro_z,mag_x,mag_y,mag_z,pressure_pa,temperature_c,latitude,longitude,gps_altitude,unix_time,milliseconds,satellites,vel_x,vel_y,vel_z,pos_x,pos_y,pos_z,quat_w,quat_x,quat_y,quat_z,kalman_pz,baro_altitude,baro_velocity,flags,battery_voltage,state,relay,last_command")
                .map_err(|e| format!("Write error: {}", e))?;
        }
        DataSource::FlashLog | DataSource::Csv => {
            writeln!(file, "tick,accel_x,accel_y,accel_z,gyro_x,gyro_y,gyro_z,state")
                .map_err(|e| format!("Write error: {}", e))?;
        }
    }
    for (i, r) in records.iter().enumerate() {
        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Ok(false);
            }
        }
        match source {
            DataSource::SdLog => {
                let relay_val = r.relay.drogue_fired as u8 | ((r.relay.parachute_fired as u8) << 1);
                writeln!(
                    file,
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    r.tick,
                    r.accel[0], r.accel[1], r.accel[2],
                    r.gyro[0], r.gyro[1], r.gyro[2],
                    r.mag[0], r.mag[1], r.mag[2],
                    r.pressure_pa,
                    r.temperature_c,
                    r.latitude,
                    r.longitude,
                    r.gps_altitude,
                    r.unix_time,
                    r.milliseconds,
                    r.satellites,
                    r.vel[0], r.vel[1], r.vel[2],
                    r.pos[0], r.pos[1], r.pos[2],
                    r.quat[0], r.quat[1], r.quat[2], r.quat[3],
                    r.kalman_pz,
                    baro_altitude.get(i).copied().unwrap_or(0.0),
                    baro_velocity.get(i).copied().unwrap_or(0.0),
                    r.flags,
                    r.battery_voltage,
                    r.state as u8,
                    relay_val,
                    r.last_command as u8,
                ).map_err(|e| format!("Write error: {}", e))?;
            }
            DataSource::FlashLog | DataSource::Csv => {
                writeln!(
                    file,
                    "{},{},{},{},{},{},{},{}",
                    r.tick,
                    r.accel[0], r.accel[1], r.accel[2],
                    r.gyro[0], r.gyro[1], r.gyro[2],
                    r.state as u8,
                ).map_err(|e| format!("Write error: {}", e))?;
            }
        }
        if let Some(p) = progress {
            p.store(i + 1, Ordering::Relaxed);
        }
    }
    Ok(true)
}
