#![windows_subsystem = "windows"]

mod app;
mod csv_recorder;
mod geolocation;
mod serial;
mod state;
mod telemetry;
mod ui;

fn load_icon() -> egui::IconData {
    let ico_bytes = include_bytes!("../assets/icon.ico");
    let reader = image::ImageReader::new(std::io::Cursor::new(ico_bytes))
        .with_guessed_format()
        .expect("failed to guess icon format");
    let img = reader.decode().expect("failed to decode icon").into_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Coheteros Ground Station",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_maximized(true)
                .with_icon(std::sync::Arc::new(load_icon())),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(app::GroundStationApp::new(cc)))),
    )
}
