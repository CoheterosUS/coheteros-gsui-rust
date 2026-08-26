use std::collections::VecDeque;
use egui_plot::{Line, Plot, PlotPoints};

use crate::app::AppState;

fn telemetry_plot(id: &str) -> Plot<'_> {
    Plot::new(id)
        .height(150.0)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .allow_double_click_reset(true)
        .show_crosshair(true)
}

fn lookup(data: &VecDeque<f64>, x: f64) -> Option<f64> {
    let idx = x.round() as usize;
    if idx < data.len() { Some(data[idx]) } else { None }
}

fn multi_series_formatter<'a>(
    series: Vec<(&'a str, &'a VecDeque<f64>, &'a str)>,
) -> impl Fn(&egui_plot::HoverPosition<'_>) -> Option<String> + 'a {
    move |hover| {
        let x = match hover {
            egui_plot::HoverPosition::NearDataPoint { position, .. } => position.x,
            egui_plot::HoverPosition::Elsewhere { position } => position.x,
        };
        let mut lines = Vec::new();
        for (name, data, unit) in &series {
            if let Some(val) = lookup(data, x) {
                lines.push(format!("{}: {:.2} {}", name, val, unit));
            }
        }
        if lines.is_empty() { None } else { Some(lines.join("\n")) }
    }
}

pub fn altitude_chart(ui: &mut egui::Ui, state: &AppState) {
    let baro: PlotPoints = state.baro_altitude.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    let gps: PlotPoints = state.gps_altitude.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    telemetry_plot("altitude_plot")
        .label_formatter(multi_series_formatter(vec![
            ("Barometric", &state.baro_altitude, "m"),
            ("GPS", &state.gps_altitude, "m"),
        ]))
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new("Barometric", baro));
            plot_ui.line(Line::new("GPS", gps));
        });
}

pub fn acceleration_chart(ui: &mut egui::Ui, state: &AppState) {
    let x: PlotPoints = state.accel_x.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    let y: PlotPoints = state.accel_y.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    let z: PlotPoints = state.accel_z.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    telemetry_plot("accel_plot")
        .label_formatter(multi_series_formatter(vec![
            ("Accel X", &state.accel_x, "m/s\u{00b2}"),
            ("Accel Y", &state.accel_y, "m/s\u{00b2}"),
            ("Accel Z", &state.accel_z, "m/s\u{00b2}"),
        ]))
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new("X", x));
            plot_ui.line(Line::new("Y", y));
            plot_ui.line(Line::new("Z", z));
        });
}

pub fn gyroscope_chart(ui: &mut egui::Ui, state: &AppState) {
    let x: PlotPoints = state.gyro_x.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    let y: PlotPoints = state.gyro_y.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    let z: PlotPoints = state.gyro_z.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    telemetry_plot("gyro_plot")
        .label_formatter(multi_series_formatter(vec![
            ("Gyro X", &state.gyro_x, "\u{00b0}/s"),
            ("Gyro Y", &state.gyro_y, "\u{00b0}/s"),
            ("Gyro Z", &state.gyro_z, "\u{00b0}/s"),
        ]))
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new("X", x));
            plot_ui.line(Line::new("Y", y));
            plot_ui.line(Line::new("Z", z));
        });
}

pub fn velocity_chart(ui: &mut egui::Ui, state: &AppState) {
    let v: PlotPoints = state.baro_velocity.iter().enumerate().map(|(i, &v)| [i as f64, v]).collect();
    telemetry_plot("velocity_plot")
        .label_formatter(multi_series_formatter(vec![
            ("Vertical", &state.baro_velocity, "m/s"),
        ]))
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new("Vertical", v));
        });
}
