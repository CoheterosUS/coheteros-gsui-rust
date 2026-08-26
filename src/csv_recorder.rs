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
        writer.write_record(Telemetry::csv_header())?;
        writer.flush()?;
        Ok(Self { writer })
    }

    pub fn record(&mut self, t: &Telemetry) {
        let ground_ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let _ = self.writer.write_record(t.csv_values(ground_ts));
        let _ = self.writer.flush();
    }
}
