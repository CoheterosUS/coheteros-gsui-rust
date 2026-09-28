use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::telemetry::packet::{Command, FlightState, RelayState};
use super::record::SdRecord;

/// Which optional column groups were present in the loaded CSV.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CsvLayout {
    /// Pressure/GPS/battery/relay columns present (telemetry or SD export, not flash export).
    pub full: bool,
    pub mag: bool,
    /// `unix_time` or `ground_timestamp` present.
    pub time: bool,
    /// Pre-computed `baro_altitude` / `baro_velocity` present.
    pub baro: bool,
}

pub struct CsvLog {
    pub records: Vec<SdRecord>,
    pub baro_altitude: Vec<f64>,
    pub baro_velocity: Vec<f64>,
    pub layout: CsvLayout,
}

struct Columns(HashMap<String, usize>);

impl Columns {
    fn has(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    fn get<'a>(&self, row: &'a csv::StringRecord, name: &str) -> Option<&'a str> {
        self.0.get(name).and_then(|&i| row.get(i)).map(str::trim)
    }

    fn f64(&self, row: &csv::StringRecord, name: &str) -> f64 {
        self.get(row, name).and_then(|s| s.parse().ok()).unwrap_or(0.0)
    }

    fn u64(&self, row: &csv::StringRecord, name: &str) -> u64 {
        self.get(row, name)
            .and_then(|s| s.parse::<u64>().ok().or_else(|| s.parse::<f64>().ok().map(|f| f as u64)))
            .unwrap_or(0)
    }
}

/// Parses a CSV written by the live telemetry recorder or the SD/Flash viewer export.
/// Columns are matched by header name, so any subset/order is accepted as long as `tick` exists.
pub fn parse_csv_file_with_progress(data: &[u8], progress: Option<&Arc<AtomicUsize>>) -> Result<CsvLog, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(data);

    let headers = reader.headers().map_err(|e| format!("Invalid CSV header: {}", e))?;
    let cols = Columns(
        headers.iter()
            .enumerate()
            .map(|(i, h)| (h.trim().to_ascii_lowercase(), i))
            .collect(),
    );
    if !cols.has("tick") {
        return Err("CSV has no \"tick\" column".to_string());
    }

    let layout = CsvLayout {
        full: cols.has("pressure_pa"),
        mag: cols.has("mag_x"),
        time: cols.has("unix_time") || cols.has("ground_timestamp"),
        baro: cols.has("baro_altitude") && cols.has("baro_velocity"),
    };

    let mut records = Vec::new();
    let mut baro_altitude = Vec::new();
    let mut baro_velocity = Vec::new();
    let mut row = csv::StringRecord::new();

    loop {
        match reader.read_record(&mut row) {
            Ok(true) => {}
            Ok(false) => break,
            Err(e) => return Err(format!("CSV parse error: {}", e)),
        }
        if let Some(p) = progress {
            p.store(row.position().map_or(0, |pos| pos.byte() as usize), Ordering::Relaxed);
        }

        let Some(tick) = cols.get(&row, "tick").and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };

        let (unix_time, milliseconds) = if cols.has("unix_time") {
            (cols.u64(&row, "unix_time") as u32, cols.u64(&row, "milliseconds") as u16)
        } else {
            let ground_ms = cols.u64(&row, "ground_timestamp");
            ((ground_ms / 1000) as u32, (ground_ms % 1000) as u16)
        };

        records.push(SdRecord {
            raw: Vec::new(),
            tick,
            accel: [cols.f64(&row, "accel_x"), cols.f64(&row, "accel_y"), cols.f64(&row, "accel_z")],
            gyro: [cols.f64(&row, "gyro_x"), cols.f64(&row, "gyro_y"), cols.f64(&row, "gyro_z")],
            mag: [cols.f64(&row, "mag_x"), cols.f64(&row, "mag_y"), cols.f64(&row, "mag_z")],
            pressure_pa: cols.f64(&row, "pressure_pa"),
            temperature_c: cols.f64(&row, "temperature_c"),
            latitude: cols.f64(&row, "latitude"),
            longitude: cols.f64(&row, "longitude"),
            gps_altitude: cols.f64(&row, "gps_altitude"),
            unix_time,
            milliseconds,
            satellites: cols.u64(&row, "satellites") as u8,
            flags: cols.u64(&row, "flags") as u32,
            battery_voltage: cols.f64(&row, "battery_voltage"),
            state: FlightState::from_u8(cols.u64(&row, "state") as u8).unwrap_or(FlightState::Idle),
            relay: RelayState::from_u8(cols.u64(&row, "relay") as u8),
            last_command: Command::from_u8(cols.u64(&row, "last_command") as u8).unwrap_or(Command::None),
        });
        if layout.baro {
            baro_altitude.push(cols.f64(&row, "baro_altitude"));
            baro_velocity.push(cols.f64(&row, "baro_velocity"));
        }
    }

    if records.is_empty() {
        return Err("No valid rows found in CSV".to_string());
    }
    if let Some(p) = progress {
        p.store(data.len(), Ordering::Relaxed);
    }

    Ok(CsvLog { records, baro_altitude, baro_velocity, layout })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_telemetry_recorder_csv() {
        let data = b"ground_timestamp,tick,accel_x,accel_y,accel_z,gyro_x,gyro_y,gyro_z,pressure_pa,temperature_c,latitude,longitude,gps_altitude,satellites,baro_altitude,baro_velocity,flags,battery_voltage,state,relay,last_command\n\
1700000000123,500,1,2,3,4,5,6,101325,20.5,37.1,-5.9,100,9,12.5,-1.5,0,4.1,4,1,3\n";
        let log = parse_csv_file_with_progress(data, None).unwrap();
        assert_eq!(log.records.len(), 1);
        let r = &log.records[0];
        assert_eq!(r.tick, 500);
        assert_eq!(r.unix_time, 1_700_000_000);
        assert_eq!(r.milliseconds, 123);
        assert_eq!(r.state, FlightState::Coast);
        assert!(r.relay.drogue_fired && !r.relay.parachute_fired);
        assert_eq!(r.last_command, Command::Calibration);
        assert_eq!(log.baro_altitude, vec![12.5]);
        assert_eq!(log.layout, CsvLayout { full: true, mag: false, time: true, baro: true });
    }

    #[test]
    fn parses_flash_export_csv() {
        let data = b"tick,accel_x,accel_y,accel_z,gyro_x,gyro_y,gyro_z,state\n100,1,2,3,4,5,6,2\n200,1,2,3,4,5,6,3\n";
        let log = parse_csv_file_with_progress(data, None).unwrap();
        assert_eq!(log.records.len(), 2);
        assert_eq!(log.layout, CsvLayout::default());
        assert!(log.baro_altitude.is_empty());
    }

    #[test]
    fn rejects_csv_without_tick() {
        assert!(parse_csv_file_with_progress(b"a,b\n1,2\n", None).is_err());
    }
}
