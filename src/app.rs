use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender};

use crate::serial::worker::{SerialCommand, SerialEvent};
use crate::telemetry::packet::{self, Command, FlightState, Telemetry};
use crate::ui;

const MAX_DATA_POINTS: usize = 500;

pub struct AppState {
    pub latest: Option<Telemetry>,
    pub baro_altitude: VecDeque<f64>,
    pub gps_altitude: VecDeque<f64>,
    pub baro_velocity: VecDeque<f64>,
    pub accel_x: VecDeque<f64>,
    pub accel_y: VecDeque<f64>,
    pub accel_z: VecDeque<f64>,
    pub gyro_x: VecDeque<f64>,
    pub gyro_y: VecDeque<f64>,
    pub gyro_z: VecDeque<f64>,
    pub gps_trail: VecDeque<(f64, f64)>,
    pub packet_count: u64,
    pub bytes_received: u64,
    pub throughput_kbps: f64,
    throughput_bytes_window: u64,
    throughput_last_update: Instant,
    pub connected: bool,
    pub available_ports: Vec<String>,
    pub selected_port: String,
    pub selected_baud: u32,
    pub errors: VecDeque<String>,
    pub ground_pos: Option<(f64, f64)>,
}

impl AppState {
    fn new() -> Self {
        Self {
            latest: None,
            baro_altitude: VecDeque::with_capacity(MAX_DATA_POINTS),
            gps_altitude: VecDeque::with_capacity(MAX_DATA_POINTS),
            baro_velocity: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_x: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_y: VecDeque::with_capacity(MAX_DATA_POINTS),
            accel_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_x: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_y: VecDeque::with_capacity(MAX_DATA_POINTS),
            gyro_z: VecDeque::with_capacity(MAX_DATA_POINTS),
            gps_trail: VecDeque::with_capacity(MAX_DATA_POINTS),
            packet_count: 0,
            bytes_received: 0,
            throughput_kbps: 0.0,
            throughput_bytes_window: 0,
            throughput_last_update: Instant::now(),
            connected: false,
            available_ports: Vec::new(),
            selected_port: String::new(),
            selected_baud: 115200,
            errors: VecDeque::with_capacity(20),
            ground_pos: None,
        }
    }

    fn push_telemetry(&mut self, t: Telemetry) {
        fn push_bounded(buf: &mut VecDeque<f64>, val: f64) {
            if buf.len() >= MAX_DATA_POINTS {
                buf.pop_front();
            }
            buf.push_back(val);
        }

        push_bounded(&mut self.baro_altitude, t.baro_altitude);
        push_bounded(&mut self.gps_altitude, t.gps_altitude);
        push_bounded(&mut self.baro_velocity, t.baro_velocity);
        push_bounded(&mut self.accel_x, t.accel[0]);
        push_bounded(&mut self.accel_y, t.accel[1]);
        push_bounded(&mut self.accel_z, t.accel[2]);
        push_bounded(&mut self.gyro_x, t.gyro[0]);
        push_bounded(&mut self.gyro_y, t.gyro[1]);
        push_bounded(&mut self.gyro_z, t.gyro[2]);
        if t.latitude != 0.0 || t.longitude != 0.0 {
            if self.gps_trail.len() >= MAX_DATA_POINTS {
                self.gps_trail.pop_front();
            }
            self.gps_trail.push_back((t.latitude, t.longitude));
        }
        self.packet_count += 1;
        self.bytes_received += packet::PACKET_SIZE as u64;
        self.throughput_bytes_window += packet::PACKET_SIZE as u64;
        let elapsed = self.throughput_last_update.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.throughput_kbps = self.throughput_bytes_window as f64 / elapsed;
            self.throughput_bytes_window = 0;
            self.throughput_last_update = Instant::now();
        }
        self.latest = Some(t);
    }

    fn push_error(&mut self, msg: String) {
        if self.errors.len() >= 20 {
            self.errors.pop_front();
        }
        self.errors.push_back(msg);
    }
}

pub struct GroundStationApp {
    state: AppState,
    cmd_tx: Sender<SerialCommand>,
    evt_rx: Receiver<SerialEvent>,
    map_state: ui::map::MapState,
    pending_command: Option<Command>,
    device_pos: Arc<Mutex<Option<(f64, f64)>>>,
}

impl GroundStationApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 45, 45);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 100, 100));
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(55, 55, 55);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 120, 120));
        visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
        visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
        visuals.selection.bg_fill = egui::Color32::from_rgb(50, 50, 50);
        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = 28.0;
        style.spacing.item_spacing.y = 2.0;
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);

        let (cmd_tx, evt_rx) = crate::serial::worker::spawn(cc.egui_ctx.clone());
        let device_pos = Arc::new(Mutex::new(None));
        crate::geolocation::spawn_location_poller(device_pos.clone());
        Self {
            state: AppState::new(),
            cmd_tx,
            evt_rx,
            map_state: ui::map::MapState::new(&cc.egui_ctx),
            pending_command: None,
            device_pos,
        }
    }
}

const ACCENT: egui::Color32 = egui::Color32::from_rgb(200, 200, 200);
const RED_ACCENT: egui::Color32 = egui::Color32::from_rgb(170, 40, 40);
const GREEN: egui::Color32 = egui::Color32::from_rgb(0, 145, 65);
const LABEL_COLOR: egui::Color32 = egui::Color32::from_rgb(110, 110, 110);
const VALUE_COLOR: egui::Color32 = egui::Color32::from_rgb(190, 190, 190);
const BOX_BG: egui::Color32 = egui::Color32::from_rgb(10, 10, 10);
const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(30, 30, 30);

fn bordered_section(ui: &mut egui::Ui, title: &str, title_color: egui::Color32, add_contents: impl FnOnce(&mut egui::Ui)) {
    let frame = egui::Frame::new()
        .fill(BOX_BG)
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
        .corner_radius(3.0)
        .inner_margin(8.0);

    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.spacing_mut().interact_size.y = 14.0;
        ui.colored_label(title_color, egui::RichText::new(title).strong().size(12.0));
        ui.add_space(1.0);
        add_contents(ui);
    });
}

fn data_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(LABEL_COLOR).size(13.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(VALUE_COLOR).strong().size(13.5));
        });
    });
}

fn data_row_colored(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(LABEL_COLOR).size(13.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(color).strong().size(13.5));
        });
    });
}

impl eframe::App for GroundStationApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(pos) = self.device_pos.try_lock() {
            self.state.ground_pos = *pos;
        }

        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                SerialEvent::Packet(t) => self.state.push_telemetry(t),
                SerialEvent::Connected(name) => {
                    self.state.connected = true;
                    self.state.push_error(format!("Connected to {}", name));
                }
                SerialEvent::Disconnected => {
                    self.state.connected = false;
                    self.state.push_error("Disconnected".into());
                }
                SerialEvent::Error(e) => {
                    self.state.push_error(e);
                }
                SerialEvent::PortList(ports) => {
                    self.state.available_ports = ports;
                }
            }
        }

        let t = self.state.latest.clone();

        // === TOP BAR: State + key values ===
        egui::Panel::top("top_bar")
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(6, 6, 6)).inner_margin(egui::Margin::symmetric(8, 6)))
            .show(root_ui, |ui| {
            ui.horizontal_centered(|ui| {
                egui::ComboBox::from_id_salt("port_combo")
                    .selected_text(if self.state.selected_port.is_empty() {
                        "PORT"
                    } else {
                        self.state.selected_port.as_str()
                    })
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for p in &self.state.available_ports {
                            ui.selectable_value(&mut self.state.selected_port, p.clone(), p);
                        }
                    });

                egui::ComboBox::from_id_salt("baud_combo")
                    .selected_text(format!("{}", self.state.selected_baud))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for &rate in &[9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600] {
                            ui.selectable_value(&mut self.state.selected_baud, rate, format!("{}", rate));
                        }
                    });

                if self.state.connected {
                    if ui.button("DISCONNECT").clicked() {
                        let _ = self.cmd_tx.send(SerialCommand::Disconnect);
                    }
                } else if !self.state.selected_port.is_empty() {
                    if ui.button("CONNECT").clicked() {
                        let _ = self.cmd_tx.send(SerialCommand::Connect {
                            port: self.state.selected_port.clone(),
                            baud: self.state.selected_baud,
                        });
                    }
                }

                if let Some(ref t) = t {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.2} \u{00b0}C", t.temperature_c)).strong().color(egui::Color32::from_rgb(230, 70, 70))); ui.label("TEMP");
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(format!("{:.0} Pa", t.pressure_pa)).strong()); ui.label("PRESSURE");
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(format!("{:.2} V", t.battery_voltage)).strong().color(egui::Color32::YELLOW)); ui.label("BATTERY");
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(format!("{}", t.tick)).strong()); ui.label("TICK");
                    });
                }
            });
        });

        // === COMMAND BAR ===
        egui::Panel::top("cmd_bar")
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(6, 6, 6)).inner_margin(egui::Margin::symmetric(8, 6)))
            .show(root_ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label("FLIGHT COMMANDS");
                ui.add_space(12.0);

                ui.add_enabled_ui(self.state.connected, |ui| {
                    let red_btn = |text: &str| {
                        egui::Button::new(
                            egui::RichText::new(text).color(egui::Color32::WHITE),
                        ).fill(egui::Color32::DARK_RED)
                    };

                    if ui.add(red_btn("CALIBRATION")).clicked() {
                        self.pending_command = Some(Command::Calibration);
                    }
                    if ui.add(red_btn("RESET")).clicked() {
                        self.pending_command = Some(Command::Reset);
                    }
                    if ui.add(red_btn("GROUND ABORT")).clicked() {
                        self.pending_command = Some(Command::GroundAbort);
                    }

                    ui.add_space(12.0);
                    ui.label("ACTIONS");
                    ui.add_space(12.0);

                    if ui.add(red_btn("MARK LANDED")).clicked() {
                        self.pending_command = Some(Command::Landed);
                    }
                    if ui.add(red_btn("DEPLOY DROGUE")).clicked() {
                        self.pending_command = Some(Command::Drogue);
                    }
                });

                ui.add_space(12.0);

                if let Some(ref t) = t {
                    let state_color = match t.state {
                        FlightState::Idle => egui::Color32::GRAY,
                        FlightState::Burn => egui::Color32::ORANGE,
                        FlightState::Apogee | FlightState::Parachute => GREEN,
                        FlightState::GroundAbort | FlightState::DescentAbort => RED_ACCENT,
                        _ => egui::Color32::YELLOW,
                    };
                    let badge = egui::RichText::new(format!(" {} ", t.state))
                        .color(egui::Color32::BLACK)
                        .strong();
                    ui.colored_label(state_color, badge);

                    let faults = packet::active_faults(t.flags);
                    if faults.is_empty() {
                        ui.label("FAULTS: NONE");
                    } else {
                        ui.colored_label(RED_ACCENT, format!("FAULTS: {}", faults.len()));
                    }
                } else {
                    ui.label("NO TELEMETRY");
                }
            });
        });

        // === COMMAND CONFIRMATION ===
        if let Some(cmd) = self.pending_command {
            let modal = egui::Modal::new(egui::Id::new("cmd_confirm"));
            let response = modal.show(root_ui.ctx(), |ui| {
                ui.label(format!("Send {}?", cmd));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("CONFIRM").clicked() {
                        let _ = self.cmd_tx.send(SerialCommand::SendCommand(cmd));
                        self.pending_command = None;
                    }
                    if ui.button("CANCEL").clicked() {
                        self.pending_command = None;
                    }
                });
            });
            if response.should_close() {
                self.pending_command = None;
            }
        }

        // === BOTTOM STATUS BAR ===
        egui::Panel::bottom("status_bar").show(root_ui, |ui| {
            ui.horizontal(|ui| {
                let (status, color) = if self.state.connected {
                    ("CONNECTED", GREEN)
                } else {
                    ("DISCONNECTED", RED_ACCENT)
                };
                ui.colored_label(color, status);
                ui.separator();
                ui.label(format!("PACKETS: {}", self.state.packet_count));
                ui.separator();
                ui.label(format!("{:.0} B/s", self.state.throughput_kbps));
            });
        });

        // === RIGHT PANEL: Map + Vehicle ===
        egui::Panel::right("right_panel")
            .default_size(300.0)
            .min_size(250.0)
            .resizable(true)
            .show(root_ui, |ui| {
                bordered_section(ui, "MAP", ACCENT, |ui| {
                    let current_gps = t.as_ref()
                        .filter(|t| t.latitude != 0.0 || t.longitude != 0.0)
                        .map(|t| (t.latitude, t.longitude));
                    let map_rect = ui::map::gps_map(
                        ui,
                        &self.state.gps_trail,
                        current_gps,
                        self.state.ground_pos,
                        &mut self.map_state,
                    );

                    if let Some(ref t) = t {
                        let overlay_width = 195.0;
                        let overlay_height = 82.0;
                        let overlay_pos = egui::pos2(
                            map_rect.right() - overlay_width - 4.0,
                            map_rect.bottom() - overlay_height - 4.0,
                        );
                        let overlay_rect = egui::Rect::from_min_size(overlay_pos, egui::vec2(overlay_width, overlay_height));

                        let painter = ui.painter();
                        painter.rect_filled(overlay_rect, 2.0, egui::Color32::from_black_alpha(220));

                        let s = 14.0;
                        let mut y = overlay_rect.top() + 5.0;
                        let x_label = overlay_rect.left() + 8.0;
                        let x_value = overlay_rect.left() + 44.0;
                        let line_h = 18.0;

                        let font = egui::FontId::proportional(s);
                        let bold = egui::FontId::proportional(s + 1.0);

                        let rows: &[(&str, String, egui::Color32)] = &[
                            ("LAT", format!("{:.6}\u{00b0}", t.latitude), VALUE_COLOR),
                            ("LON", format!("{:.6}\u{00b0}", t.longitude), VALUE_COLOR),
                            ("ALT", format!("{:.1} m", t.gps_altitude), VALUE_COLOR),
                            ("SAT", format!("{}", t.satellites), if t.satellites >= 4 { GREEN } else { RED_ACCENT }),
                        ];
                        for (label, value, color) in rows {
                            painter.text(egui::pos2(x_label, y), egui::Align2::LEFT_TOP, label, font.clone(), LABEL_COLOR);
                            painter.text(egui::pos2(x_value, y), egui::Align2::LEFT_TOP, value, bold.clone(), *color);
                            y += line_h;
                        }
                    }
                });

                ui.add_space(4.0);
                bordered_section(ui, "VEHICLE", ACCENT, |ui| {
                if let Some(ref t) = t {
                    egui::Grid::new("vehicle_grid")
                        .num_columns(2)
                        .spacing([8.0, 2.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("LATITUDE").color(LABEL_COLOR).size(13.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{:.6} \u{00b0}", t.latitude)).color(VALUE_COLOR).size(13.5));
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("LONGITUDE").color(LABEL_COLOR).size(13.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{:.6} \u{00b0}", t.longitude)).color(VALUE_COLOR).size(13.5));
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("BARO ALT (AGL)").color(LABEL_COLOR).size(13.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{:.1} m", t.baro_altitude)).color(VALUE_COLOR).size(13.5));
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("GPS ALT (ASL)").color(LABEL_COLOR).size(13.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{:.1} m", t.gps_altitude)).color(VALUE_COLOR).size(13.5));
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("BARO VELOCITY").color(LABEL_COLOR).size(13.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{:.1} m/s", t.baro_velocity)).color(VALUE_COLOR).size(13.5));
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("SATELLITES").color(LABEL_COLOR).size(13.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let sat_color = if t.satellites >= 4 { GREEN } else { RED_ACCENT };
                                ui.label(egui::RichText::new(format!("{}", t.satellites)).color(sat_color).size(13.5));
                            });
                            ui.end_row();
                        });
                }
                });
            });

        // === CENTER: Data grid + Altitude chart ===
        egui::CentralPanel::default().show(root_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Row 1: STATUS | ALTITUDE | VELOCITY | POSITION
                ui.columns(4, |cols| {
                    bordered_section(&mut cols[0], "STATUS", RED_ACCENT, |ui| {
                        if let Some(ref t) = t {
                            data_row(ui, "TICK", &format!("{}", t.tick));
                            data_row(ui, "STATE", &format!("{}", t.state));
                            data_row(ui, "FLAGS", &format!("{}", t.flags));
                            data_row(ui, "LAST COMMAND", &format!("{}", t.last_command));
                            let drogue_color = if t.relay.drogue_fired { RED_ACCENT } else { GREEN };
                            data_row_colored(ui, "DROGUE", if t.relay.drogue_fired { "FIRED" } else { "SAFE" }, drogue_color);
                            let chute_color = if t.relay.parachute_fired { RED_ACCENT } else { GREEN };
                            data_row_colored(ui, "PARACHUTE", if t.relay.parachute_fired { "FIRED" } else { "SAFE" }, chute_color);
                        }
                    });

                    bordered_section(&mut cols[1], "ALTITUDE", ACCENT, |ui| {
                        if let Some(ref t) = t {
                            data_row(ui, "BARO ALT (AGL)", &format!("{:.2} m", t.baro_altitude));
                            data_row(ui, "BARO VELOCITY", &format!("{:.2} m/s", t.baro_velocity));
                        }
                    });

                    bordered_section(&mut cols[2], "VELOCITY", ACCENT, |ui| {
                        if let Some(ref t) = t {
                            data_row(ui, "VELOCITY X", &format!("{:.2} m/s", t.velocity[0]));
                            data_row(ui, "VELOCITY Y", &format!("{:.2} m/s", t.velocity[1]));
                            data_row(ui, "VELOCITY Z", &format!("{:.2} m/s", t.velocity[2]));
                        }
                    });

                    bordered_section(&mut cols[3], "POSITION", ACCENT, |ui| {
                        if let Some(ref t) = t {
                            data_row(ui, "GPS ALT (ASL)", &format!("{:.2} m", t.gps_altitude));
                            data_row(ui, "LATITUDE", &format!("{:.6} °", t.latitude));
                            data_row(ui, "LONGITUDE", &format!("{:.6} °", t.longitude));
                            data_row(ui, "SATELLITES", &format!("{}", t.satellites));
                        }
                    });
                });

                ui.add_space(4.0);

                // Row 2: FAULTS | MAGNETOMETER | ACCELERATION | GYROSCOPE
                ui.columns(4, |cols| {
                    bordered_section(&mut cols[0], "FAULTS", RED_ACCENT, |ui| {
                        if let Some(ref t) = t {
                            let fault_list = [
                                ("BMP280", t.flags & 0x03),
                                ("BMP581", t.flags & 0x0C),
                                ("IIM42653", t.flags & 0x30),
                                ("IIS2MDCTR", t.flags & 0xC0),
                                ("SD", t.flags & 0x300),
                            ];
                            for (name, bits) in fault_list {
                                let (status, color) = if bits == 0 { ("OK", GREEN) } else { ("FAIL", RED_ACCENT) };
                                data_row_colored(ui, name, status, color);
                            }
                        }
                    });

                    bordered_section(&mut cols[1], "MAGNETOMETER", ACCENT, |ui| {
                        if let Some(ref t) = t {
                            data_row(ui, "MAG X", &format!("{:.2} mG", t.mag[0]));
                            data_row(ui, "MAG Y", &format!("{:.2} mG", t.mag[1]));
                            data_row(ui, "MAG Z", &format!("{:.2} mG", t.mag[2]));
                        }
                    });

                    bordered_section(&mut cols[2], "ACCELERATION", ACCENT, |ui| {
                        if let Some(ref t) = t {
                            data_row(ui, "ACCEL X", &format!("{:.2} m/s\u{00b2}", t.accel[0]));
                            data_row(ui, "ACCEL Y", &format!("{:.2} m/s\u{00b2}", t.accel[1]));
                            data_row(ui, "ACCEL Z", &format!("{:.2} m/s\u{00b2}", t.accel[2]));
                        }
                    });

                    bordered_section(&mut cols[3], "GYROSCOPE", ACCENT, |ui| {
                        if let Some(ref t) = t {
                            data_row(ui, "GYRO X", &format!("{:.2} \u{00b0}/s", t.gyro[0]));
                            data_row(ui, "GYRO Y", &format!("{:.2} \u{00b0}/s", t.gyro[1]));
                            data_row(ui, "GYRO Z", &format!("{:.2} \u{00b0}/s", t.gyro[2]));
                        }
                    });
                });

                ui.add_space(8.0);

                // Charts
                bordered_section(ui, "ALTITUDE", ACCENT, |ui| {
                    ui::charts::altitude_chart(ui, &self.state);
                });
                ui.add_space(4.0);
                bordered_section(ui, "ACCELERATION", ACCENT, |ui| {
                    ui::charts::acceleration_chart(ui, &self.state);
                });
                ui.add_space(4.0);
                bordered_section(ui, "GYROSCOPE", ACCENT, |ui| {
                    ui::charts::gyroscope_chart(ui, &self.state);
                });
                ui.add_space(4.0);
                bordered_section(ui, "VELOCITY", ACCENT, |ui| {
                    ui::charts::velocity_chart(ui, &self.state);
                });
            });
        });
    }
}
