use crate::telemetry::packet::{PACKET_FIELDS, PACKET_SIZE};
use super::theme::LABEL_COLOR;

fn byte_color(offset: usize) -> egui::Color32 {
    for field in PACKET_FIELDS {
        if offset >= field.offset && offset < field.offset + field.length {
            return egui::Color32::from_rgb(field.color[0], field.color[1], field.color[2]);
        }
    }
    LABEL_COLOR
}

fn byte_field(offset: usize) -> &'static str {
    for field in PACKET_FIELDS {
        if offset >= field.offset && offset < field.offset + field.length {
            return field.name;
        }
    }
    "?"
}

pub fn hex_viewer(ui: &mut egui::Ui, raw: &[u8; PACKET_SIZE]) {
    let font = egui::FontId::monospace(12.0);
    let cols = 16;
    let rows = raw.len().div_ceil(cols);

    ui.horizontal_top(|ui| {
        egui::Grid::new("hex_grid")
            .num_columns(cols + 1)
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                for row in 0..rows {
                    ui.label(egui::RichText::new(format!("{:02X}:", row * cols)).font(font.clone()).color(LABEL_COLOR));
                    for col in 0..cols {
                        let idx = row * cols + col;
                        if idx < raw.len() {
                            let color = byte_color(idx);
                            let text = format!("{:02X}", raw[idx]);
                            let label = ui.label(egui::RichText::new(text).font(font.clone()).color(color));
                            label.on_hover_text(format!("{} [{}]", byte_field(idx), idx));
                        }
                    }
                    ui.end_row();
                }
            });

        ui.add_space(16.0);

        let legend_cols = 4;
        egui::Grid::new("hex_legend")
            .num_columns(legend_cols)
            .spacing([12.0, 1.0])
            .show(ui, |ui| {
                for (i, field) in PACKET_FIELDS.iter().enumerate() {
                    let color = egui::Color32::from_rgb(field.color[0], field.color[1], field.color[2]);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(egui::RichText::new("\u{25A0}").color(color).font(font.clone()));
                        ui.label(egui::RichText::new(field.name).color(LABEL_COLOR).font(font.clone()));
                    });
                    if (i + 1) % legend_cols == 0 {
                        ui.end_row();
                    }
                }
            });
    });
}
