use crate::telemetry::packet::{PACKET_FIELDS, PACKET_SIZE};
use super::theme;

fn darken(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(
        (c[0] as u16 * 6 / 10) as u8,
        (c[1] as u16 * 6 / 10) as u8,
        (c[2] as u16 * 6 / 10) as u8,
    )
}

fn field_color(offset: usize, dark_mode: bool) -> egui::Color32 {
    for field in PACKET_FIELDS {
        if offset >= field.offset && offset < field.offset + field.length {
            return if dark_mode {
                egui::Color32::from_rgb(field.color[0], field.color[1], field.color[2])
            } else {
                darken(field.color)
            };
        }
    }
    egui::Color32::GRAY
}

fn byte_field(offset: usize) -> &'static str {
    for field in PACKET_FIELDS {
        if offset >= field.offset && offset < field.offset + field.length {
            return field.name;
        }
    }
    "?"
}

pub fn hex_viewer(ui: &mut egui::Ui, raw: &[u8; PACKET_SIZE], dark_mode: bool) {
    let tc = theme::current_theme(dark_mode);
    let font = egui::FontId::monospace(12.0);
    let cols = 16;
    let rows = raw.len().div_ceil(cols);

    ui.horizontal_top(|ui| {
        egui::Grid::new("hex_grid")
            .num_columns(cols + 1)
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                for row in 0..rows {
                    ui.label(egui::RichText::new(format!("{:02X}:", row * cols)).font(font.clone()).color(tc.label_color));
                    for col in 0..cols {
                        let idx = row * cols + col;
                        if idx < raw.len() {
                            let color = field_color(idx, dark_mode);
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
                    let color = if dark_mode {
                        egui::Color32::from_rgb(field.color[0], field.color[1], field.color[2])
                    } else {
                        darken(field.color)
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(egui::RichText::new("\u{25A0}").color(color).font(font.clone()));
                        ui.label(egui::RichText::new(field.name).color(tc.label_color).font(font.clone()));
                    });
                    if (i + 1) % legend_cols == 0 {
                        ui.end_row();
                    }
                }
            });
    });
}
