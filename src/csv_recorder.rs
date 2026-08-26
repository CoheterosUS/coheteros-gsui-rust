use std::fs::File;
use std::time::SystemTime;

use crate::telemetry::packet::Telemetry;

pub struct CsvRecorder {
    writer: csv::Writer<File>,
}

impl CsvRecorder {
    pub fn new() -> std::io::Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("flight_{}.csv", timestamp);
        let mut writer = csv::Writer::from_path(&filename)?;
        writer.write_record([
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
        ])?;
        writer.flush()?;
        Ok(Self { writer })
    }

    pub fn record(&mut self, t: &Telemetry) {
        let ground_ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let _ = self.writer.write_record(&[
            ground_ts.to_string(),
            t.tick.to_string(),
            t.accel[0].to_string(), t.accel[1].to_string(), t.accel[2].to_string(),
            t.gyro[0].to_string(), t.gyro[1].to_string(), t.gyro[2].to_string(),
            t.mag[0].to_string(), t.mag[1].to_string(), t.mag[2].to_string(),
            t.pressure_pa.to_string(), t.temperature_c.to_string(),
            t.latitude.to_string(), t.longitude.to_string(), t.gps_altitude.to_string(),
            t.satellites.to_string(),
            t.baro_altitude.to_string(), t.baro_velocity.to_string(),
            t.velocity[0].to_string(), t.velocity[1].to_string(), t.velocity[2].to_string(),
            t.flags.to_string(), t.battery_voltage.to_string(),
            (t.state as u8).to_string(),
            (t.relay.drogue_fired as u8 | ((t.relay.parachute_fired as u8) << 1)).to_string(),
            (t.last_command as u8).to_string(),
        ]);
        let _ = self.writer.flush();
    }
}
