use std::collections::BTreeMap;
use egui::Align2;
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints, VLine};
use crate::sd_viewer::state::{StateSegment, TimelineMarker};

const MAX_CHART_POINTS: usize = 10_000;

fn decimated_points(timestamps: &[f64], values: &[f64]) -> Vec<[f64; 2]> {
    let len = timestamps.len().min(values.len());
    if len == 0 {
        return vec![];
    }
    let step = (len / MAX_CHART_POINTS).max(1);
    (0..len)
        .step_by(step)
        .map(|i| [timestamps[i], values[i]])
        .collect()
}

fn lookup_by_timestamp(timestamps: &[f64], values: &[f64], t: f64) -> Option<f64> {
    let idx = timestamp_to_index(timestamps, t);
    values.get(idx).copied()
}

fn hover_x(hover: &egui_plot::HoverPosition<'_>) -> f64 {
    match hover {
        egui_plot::HoverPosition::NearDataPoint { position, .. } => position.x,
        egui_plot::HoverPosition::Elsewhere { position } => position.x,
    }
}

fn sd_plot(id: &str) -> Plot<'_> {
    Plot::new(id)
        .height(150.0)
        .allow_drag(true)
        .allow_zoom(true)
        .allow_scroll(false)
        .show_axes(true)
        .show_crosshair(true)
}

fn handle_click(response: &egui::Response, transform: &egui_plot::PlotTransform) -> Option<f64> {
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let plot_point = transform.value_from_position(pos);
            return Some(plot_point.x);
        }
    }
    None
}

pub fn timestamp_to_index(timestamps: &[f64], t: f64) -> usize {
    match timestamps.binary_search_by(|v| v.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(i) => i,
        Err(i) => {
            if i == 0 {
                0
            } else if i >= timestamps.len() {
                timestamps.len().saturating_sub(1)
            } else {
                let before = (timestamps[i - 1] - t).abs();
                let after = (timestamps[i] - t).abs();
                if before <= after { i - 1 } else { i }
            }
        }
    }
}

pub fn single_series_chart(
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    unit: &str,
    timestamps: &[f64],
    values: &[f64],
) -> Option<f64> {
    let points: PlotPoints = decimated_points(timestamps, values).into();
    let ts = timestamps;
    let vals = values;
    let n = name.to_string();
    let u = unit.to_string();
    let plot_response = sd_plot(id)
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            let val = lookup_by_timestamp(ts, vals, x)?;
            Some(format!("{}: {:.2} {}", n, val, u))
        })
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new(name, points));
        });
    handle_click(&plot_response.response, &plot_response.transform)
}

pub fn triple_series_chart(
    ui: &mut egui::Ui,
    id: &str,
    unit: &str,
    timestamps: &[f64],
    x_vals: &[f64],
    y_vals: &[f64],
    z_vals: &[f64],
) -> Option<f64> {
    let px: PlotPoints = decimated_points(timestamps, x_vals).into();
    let py: PlotPoints = decimated_points(timestamps, y_vals).into();
    let pz: PlotPoints = decimated_points(timestamps, z_vals).into();
    let ts = timestamps;
    let xv = x_vals;
    let yv = y_vals;
    let zv = z_vals;
    let u = unit.to_string();
    let plot_response = sd_plot(id)
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            let mut lines = Vec::new();
            for (name, vals) in [("X", xv), ("Y", yv), ("Z", zv)] {
                if let Some(val) = lookup_by_timestamp(ts, vals, x) {
                    lines.push(format!("{}: {:.2} {}", name, val, u));
                }
            }
            if lines.is_empty() { None } else { Some(lines.join("\n")) }
        })
        .show(ui, |plot_ui| {
            plot_ui.line(Line::new("X", px));
            plot_ui.line(Line::new("Y", py));
            plot_ui.line(Line::new("Z", pz));
        });
    handle_click(&plot_response.response, &plot_response.transform)
}

pub fn state_timeline_chart(
    ui: &mut egui::Ui,
    segments: &[StateSegment],
    markers: &[TimelineMarker],
) -> Option<f64> {
    let mut by_state: BTreeMap<u8, (&str, egui::Color32, Vec<Bar>)> = BTreeMap::new();
    for seg in segments {
        let key = seg.state as u8;
        let label = seg.label();
        let entry = by_state.entry(key).or_insert_with(|| (label, seg.color, Vec::new()));
        let duration = seg.end - seg.start;
        let bar = Bar::new(0.0, duration)
            .base_offset(seg.start)
            .fill(seg.color)
            .stroke(egui::Stroke::new(0.5, seg.color))
            .name(label);
        entry.2.push(bar);
    }

    let plot_response = Plot::new("sd_state_timeline")
        .height(80.0)
        .allow_drag(true)
        .allow_zoom(true)
        .allow_scroll(false)
        .show_axes([true, false])
        .y_axis_label("")
        .include_y(-1.0)
        .include_y(1.0)
        .show_crosshair(false)
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            for seg in segments {
                if x >= seg.start && x <= seg.end {
                    let dur = seg.end - seg.start;
                    return Some(format!("{}\n{:.1}s", seg.label(), dur));
                }
            }
            None
        })
        .show(ui, |plot_ui| {
            for (_key, (name, color, bars)) in by_state {
                plot_ui.bar_chart(
                    BarChart::new(name.to_string(), bars)
                        .horizontal()
                        .width(1.0)
                        .color(color),
                );
            }
            let mut stack_index: std::collections::HashMap<u64, f64> = std::collections::HashMap::new();
            for marker in markers {
                plot_ui.vline(
                    VLine::new(marker.label, marker.timestamp)
                        .color(marker.color)
                        .width(2.0)
                        .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
                );
                let key = marker.timestamp.to_bits();
                let offset = stack_index.entry(key).or_insert(0.0);
                let y = 0.5 - *offset * 0.35;

                let text = if marker.is_command {
                    egui::RichText::new(format!(" {} ", marker.label))
                        .size(12.0)
                        .color(marker.color)
                        .background_color(egui::Color32::from_rgb(30, 30, 30))
                } else {
                    egui::RichText::new(format!(" {} ", marker.label))
                        .size(12.0)
                        .color(egui::Color32::WHITE)
                        .background_color(marker.color)
                };

                plot_ui.text(
                    egui_plot::Text::new(
                        format!("{}_label", marker.label),
                        egui_plot::PlotPoint::new(marker.timestamp, y),
                        text,
                    )
                    .anchor(Align2::LEFT_BOTTOM),
                );
                *offset += 1.0;
            }
        });
    handle_click(&plot_response.response, &plot_response.transform)
}
