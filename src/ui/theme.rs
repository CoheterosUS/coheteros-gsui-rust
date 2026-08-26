pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(200, 200, 200);
pub const RED_ACCENT: egui::Color32 = egui::Color32::from_rgb(170, 40, 40);
pub const GREEN: egui::Color32 = egui::Color32::from_rgb(0, 145, 65);
pub const LABEL_COLOR: egui::Color32 = egui::Color32::from_rgb(110, 110, 110);
pub const VALUE_COLOR: egui::Color32 = egui::Color32::from_rgb(190, 190, 190);
pub const BOX_BG: egui::Color32 = egui::Color32::from_rgb(10, 10, 10);
pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(30, 30, 30);

pub fn bordered_section(ui: &mut egui::Ui, title: &str, title_color: egui::Color32, add_contents: impl FnOnce(&mut egui::Ui)) {
    let frame = egui::Frame::new()
        .fill(BOX_BG)
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
        .corner_radius(3.0)
        .inner_margin(8.0);

    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.spacing_mut().interact_size.y = 14.0;
        ui.colored_label(title_color, egui::RichText::new(title).family(egui::FontFamily::Name("Bold".into())).size(12.0));
        ui.add_space(1.0);
        add_contents(ui);
    });
}

pub fn data_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(LABEL_COLOR).size(13.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(VALUE_COLOR).family(egui::FontFamily::Name("Bold".into())).size(13.5));
        });
    });
}

pub fn data_row_colored(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(LABEL_COLOR).size(13.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(color).family(egui::FontFamily::Name("Bold".into())).size(13.5));
        });
    });
}

pub fn setup_visuals(cc: &eframe::CreationContext<'_>) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(180, 180, 180));
    visuals.panel_fill = egui::Color32::from_rgb(6, 6, 6);
    visuals.window_fill = egui::Color32::from_rgb(8, 8, 8);
    visuals.extreme_bg_color = egui::Color32::from_rgb(3, 3, 3);
    visuals.faint_bg_color = egui::Color32::from_rgb(10, 10, 10);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(14, 14, 14);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(30, 30, 30));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(20, 20, 20);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.5, egui::Color32::TRANSPARENT);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 60, 60);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(140, 140, 140));
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(240, 240, 240));
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 80);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(180, 180, 180));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    visuals.hyperlink_color = egui::Color32::from_rgb(100, 149, 237);
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    visuals.selection.bg_fill = egui::Color32::from_rgb(50, 50, 50);
    cc.egui_ctx.set_visuals(visuals);

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "SUSEMono-Regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/SUSEMono-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "SUSEMono-SemiBold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/SUSEMono-SemiBold.ttf"))),
    );
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "SUSEMono-Regular".to_owned());
    fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "SUSEMono-Regular".to_owned());
    fonts.families.insert(egui::FontFamily::Name("Bold".into()), vec!["SUSEMono-SemiBold".to_owned()]);
    cc.egui_ctx.set_fonts(fonts);

    let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 28.0;
    style.spacing.item_spacing.y = 2.0;
    cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
}
