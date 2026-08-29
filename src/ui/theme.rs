pub struct ThemeColors {
    pub accent: egui::Color32,
    pub red_accent: egui::Color32,
    pub green: egui::Color32,
    pub yellow: egui::Color32,
    pub label_color: egui::Color32,
    pub value_color: egui::Color32,
    pub box_bg: egui::Color32,
    pub border_subtle: egui::Color32,
    pub panel_bg: egui::Color32,
    pub window_bg: egui::Color32,
    pub text_color: egui::Color32,
    pub modal_bg: egui::Color32,
    pub modal_stroke: egui::Color32,
}

pub const DARK: ThemeColors = ThemeColors {
    accent: egui::Color32::from_rgb(200, 200, 200),
    red_accent: egui::Color32::from_rgb(170, 40, 40),
    green: egui::Color32::from_rgb(0, 145, 65),
    yellow: egui::Color32::from_rgb(255, 220, 50),
    label_color: egui::Color32::from_rgb(110, 110, 110),
    value_color: egui::Color32::from_rgb(190, 190, 190),
    box_bg: egui::Color32::from_rgb(10, 10, 10),
    border_subtle: egui::Color32::from_rgb(30, 30, 30),
    panel_bg: egui::Color32::from_rgb(6, 6, 6),
    window_bg: egui::Color32::from_rgb(8, 8, 8),
    text_color: egui::Color32::from_rgb(180, 180, 180),
    modal_bg: egui::Color32::from_rgb(14, 14, 14),
    modal_stroke: egui::Color32::from_rgb(50, 50, 50),
};

pub const LIGHT: ThemeColors = ThemeColors {
    accent: egui::Color32::from_rgb(40, 40, 40),
    red_accent: egui::Color32::from_rgb(200, 40, 40),
    green: egui::Color32::from_rgb(0, 130, 55),
    yellow: egui::Color32::from_rgb(180, 140, 0),
    label_color: egui::Color32::from_rgb(100, 100, 100),
    value_color: egui::Color32::from_rgb(30, 30, 30),
    box_bg: egui::Color32::from_rgb(245, 245, 245),
    border_subtle: egui::Color32::from_rgb(210, 210, 210),
    panel_bg: egui::Color32::from_rgb(235, 235, 235),
    window_bg: egui::Color32::from_rgb(240, 240, 240),
    text_color: egui::Color32::from_rgb(30, 30, 30),
    modal_bg: egui::Color32::from_rgb(245, 245, 245),
    modal_stroke: egui::Color32::from_rgb(200, 200, 200),
};

pub fn current_theme(dark_mode: bool) -> &'static ThemeColors {
    if dark_mode { &DARK } else { &LIGHT }
}

pub fn apply_visuals(ctx: &egui::Context, dark_mode: bool) {
    let t = current_theme(dark_mode);
    let mut visuals = if dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() };
    visuals.override_text_color = Some(t.text_color);
    visuals.panel_fill = t.panel_bg;
    visuals.window_fill = t.window_bg;

    if dark_mode {
        visuals.extreme_bg_color = egui::Color32::from_rgb(3, 3, 3);
        visuals.faint_bg_color = egui::Color32::from_rgb(10, 10, 10);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(14, 14, 14);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(30, 30, 30));
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(20, 20, 20);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.5, egui::Color32::TRANSPARENT);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 60, 60);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(140, 140, 140));
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(240, 240, 240));
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 80);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(180, 180, 180));
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.selection.bg_fill = egui::Color32::from_rgb(50, 50, 50);
    } else {
        visuals.extreme_bg_color = egui::Color32::from_rgb(250, 250, 250);
        visuals.faint_bg_color = egui::Color32::from_rgb(240, 240, 240);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(230, 230, 230);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(200, 200, 200));
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(220, 220, 220);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.5, egui::Color32::TRANSPARENT);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(200, 200, 200);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 120, 120));
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 20, 20));
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(180, 180, 180);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 80));
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
        visuals.selection.bg_fill = egui::Color32::from_rgb(180, 210, 240);
    }

    visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    visuals.hyperlink_color = egui::Color32::from_rgb(100, 149, 237);
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);

    ctx.set_visuals(visuals);
}

pub fn bordered_section(ui: &mut egui::Ui, title: &str, title_color: egui::Color32, dark_mode: bool, add_contents: impl FnOnce(&mut egui::Ui)) {
    let t = current_theme(dark_mode);
    let frame = egui::Frame::new()
        .fill(t.box_bg)
        .stroke(egui::Stroke::new(1.0, t.border_subtle))
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

pub fn data_row(ui: &mut egui::Ui, label: &str, value: &str, dark_mode: bool) {
    let t = current_theme(dark_mode);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(t.label_color).size(13.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(t.value_color).family(egui::FontFamily::Name("Bold".into())).size(13.5));
        });
    });
}

pub fn data_row_colored(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32, dark_mode: bool) {
    let t = current_theme(dark_mode);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(t.label_color).size(13.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(color).family(egui::FontFamily::Name("Bold".into())).size(13.5));
        });
    });
}

pub fn setup_fonts_and_style(cc: &eframe::CreationContext<'_>) {
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
    cc.egui_ctx.set_style_of(egui::Theme::Dark, style.clone());
    cc.egui_ctx.set_style_of(egui::Theme::Light, style);
}
