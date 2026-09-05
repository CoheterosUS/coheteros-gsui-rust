use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};

use crate::csv_recorder::CsvRecorder;
use crate::sd_log::record::SD_RECORD_FIELDS;
use crate::sd_viewer::state::SdViewerState;
use crate::serial::worker::{SerialCommand, SerialEvent};
use crate::state::AppState;
use crate::telemetry::packet::{self, Command, FlightState};
use crate::ui;
use crate::ui::theme;

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActiveTab {
    LiveTelemetry,
    SdViewer,
}

pub struct GroundStationApp {
    state: AppState,
    cmd_tx: Sender<SerialCommand>,
    evt_rx: Receiver<SerialEvent>,
    map_state: ui::map::MapState,
    pending_command: Option<Command>,
    device_pos: Arc<Mutex<Option<(f64, f64)>>>,
    show_about: bool,
    csv_recorder: Option<CsvRecorder>,
    logo_texture: Option<egui::TextureHandle>,
    active_tab: ActiveTab,
    sd_viewer: SdViewerState,
}

impl GroundStationApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::setup_fonts_and_style(cc);
        theme::apply_visuals(&cc.egui_ctx, true);

        let (cmd_tx, evt_rx) = crate::serial::worker::spawn(cc.egui_ctx.clone());
        let device_pos = Arc::new(Mutex::new(None));
        crate::geolocation::spawn_location_poller(device_pos.clone());

        let logo_texture = {
            let png_data = include_bytes!("../assets/logo.png");
            let image = image::load_from_memory(png_data).expect("failed to load logo");
            let rgba = image.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.into_raw();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
            Some(cc.egui_ctx.load_texture("logo", color_image, egui::TextureOptions::LINEAR))
        };

        Self {
            state: AppState::new(),
            cmd_tx,
            evt_rx,
            map_state: ui::map::MapState::new(&cc.egui_ctx),
            pending_command: None,
            device_pos,
            show_about: false,
            csv_recorder: None,
            logo_texture,
            active_tab: ActiveTab::LiveTelemetry,
            sd_viewer: SdViewerState::new(),
        }
    }

    fn render_live_telemetry(&mut self, root_ui: &mut egui::Ui) {
        let dm = self.state.dark_mode;
        let tc = theme::current_theme(dm);
        let t = self.state.latest.clone();

        // === TOP BAR: State + key values ===
        egui::Panel::top("top_bar")
            .frame(egui::Frame::new().fill(tc.panel_bg).inner_margin(egui::Margin::symmetric(8, 6)))
            .show(root_ui, |ui| {
            ui.horizontal_centered(|ui| {
                let no_ports = self.state.available_ports.is_empty();
                ui.add_enabled_ui(!no_ports, |ui| {
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
                });

                if self.state.connected {
                    if ui.button("DISCONNECT").clicked() {
                        let _ = self.cmd_tx.send(SerialCommand::Disconnect);
                    }
                } else {
                    let enabled = !self.state.selected_port.is_empty();
                    if ui.add_enabled(enabled, egui::Button::new("CONNECT")).clicked() {
                        let _ = self.cmd_tx.send(SerialCommand::Connect {
                            port: self.state.selected_port.clone(),
                            baud: self.state.selected_baud,
                        });
                    }
                }

                ui.separator();
                ui.label("EXPECTED Hz");
                egui::ComboBox::from_id_salt("expected_rate")
                    .selected_text(format!("{}", self.state.expected_packet_rate))
                    .width(50.0)
                    .show_ui(ui, |ui| {
                        for &rate in &[10, 20, 25, 50, 100, 200] {
                            ui.selectable_value(&mut self.state.expected_packet_rate, rate, format!("{}", rate));
                        }
                    });

                if let Some(ref t) = t {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.2} \u{00b0}C", t.temperature_c)).family(egui::FontFamily::Name("Bold".into())).color(egui::Color32::from_rgb(230, 70, 70))); ui.label("TEMP");
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(format!("{:.0} Pa", t.pressure_pa)).family(egui::FontFamily::Name("Bold".into()))); ui.label("PRESSURE");
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(format!("{:.2} V", t.battery_voltage)).family(egui::FontFamily::Name("Bold".into())).color(tc.yellow)); ui.label("BATTERY");
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new(format!("{}", t.tick)).family(egui::FontFamily::Name("Bold".into()))); ui.label("TICK");
                    });
                }
            });
        });

        // === COMMAND BAR ===
        egui::Panel::top("cmd_bar")
            .frame(egui::Frame::new().fill(tc.panel_bg).inner_margin(egui::Margin::symmetric(8, 6)))
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
                ui.label("CSV");
                ui.add_space(12.0);

                if self.csv_recorder.is_some() {
                    if ui.add(egui::Button::new(
                        egui::RichText::new("STOP REC").color(egui::Color32::WHITE),
                    ).fill(egui::Color32::from_rgb(220, 40, 40))).clicked() {
                        self.csv_recorder = None;
                        self.state.push_message("Recording stopped");
                    }
                } else if ui.add(egui::Button::new(
                    egui::RichText::new("RECORD").color(egui::Color32::WHITE),
                ).fill(tc.green)).clicked() {
                    match CsvRecorder::new() {
                        Ok(rec) => {
                            self.state.push_message("Recording started");
                            self.csv_recorder = Some(rec);
                        }
                        Err(e) => self.state.push_message(&format!("CSV error: {}", e)),
                    }
                }

                ui.add_space(12.0);

                if let Some(ref t) = t {
                    let state_color = match t.state {
                        FlightState::Idle => egui::Color32::GRAY,
                        FlightState::Boost => egui::Color32::ORANGE,
                        FlightState::Apogee | FlightState::MainParachute => tc.green,
                        FlightState::GroundAbort | FlightState::DescentAbort => tc.red_accent,
                        _ => tc.yellow,
                    };
                    let badge = egui::RichText::new(format!(" {} ", t.state))
                        .color(egui::Color32::BLACK)
                        .family(egui::FontFamily::Name("Bold".into()));
                    ui.colored_label(state_color, badge);

                    let faults = packet::active_faults(t.flags);
                    if faults.is_empty() {
                        ui.label("FAULTS: NONE");
                    } else {
                        ui.colored_label(tc.red_accent, format!("FAULTS: {}", faults.len()));
                    }
                } else {
                    ui.label("NO TELEMETRY");
                }
            });
        });

        // === COMMAND CONFIRMATION ===
        if let Some(cmd) = self.pending_command {
            let modal = egui::Modal::new(egui::Id::new("cmd_confirm"))
                .frame(egui::Frame::new().fill(tc.modal_bg).stroke(egui::Stroke::new(1.0, tc.modal_stroke)).inner_margin(30.0).corner_radius(4.0));
            let response = modal.show(root_ui.ctx(), |ui| {
                ui.label(egui::RichText::new(format!("Send {}?", cmd)).size(18.0).family(egui::FontFamily::Name("Bold".into())));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("CONFIRM").size(15.0)).clicked() {
                        let _ = self.cmd_tx.send(SerialCommand::SendCommand(cmd));
                        self.state.push_command(&format!("{}", cmd));
                        self.pending_command = None;
                    }
                    if ui.button(egui::RichText::new("CANCEL").size(15.0)).clicked() {
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
                    ("CONNECTED", tc.green)
                } else {
                    ("DISCONNECTED", tc.red_accent)
                };
                ui.colored_label(color, status);
                ui.separator();
                ui.label(format!("PACKETS: {}", self.state.packet_count));
                ui.separator();
                ui.label(format!("{:.0} B/s", self.state.throughput_kbps));
                ui.separator();
                let expected = self.state.expected_packet_rate as f64;
                let actual = self.state.packets_per_sec;
                let ratio = if expected > 0.0 { actual / expected } else { 1.0 };
                let rate_color = if ratio >= 0.9 {
                    tc.green
                } else if ratio >= 0.5 {
                    tc.yellow
                } else {
                    tc.red_accent
                };
                ui.colored_label(rate_color, format!("{:.0}/{:.0} Hz ({:.0}%)", actual, expected, ratio * 100.0));
            });
        });

        // === CONSOLE (right, next to map) ===
        egui::Panel::right("console_panel")
            .default_size(220.0)
            .min_size(180.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(tc.panel_bg).inner_margin(4.0))
            .show(root_ui, |ui| {
                ui.set_min_size(ui.available_size());
                theme::bordered_section(ui, "CONSOLE", egui::Color32::from_rgb(255, 180, 50), dm, |ui| {
                    ui.set_min_size(ui.available_size());
                    egui::ScrollArea::vertical().id_salt("console_scroll").stick_to_bottom(true).show(ui, |ui| {
                        use crate::state::ConsoleEntry;
                        let console_gold = egui::Color32::from_rgb(255, 180, 50);
                        for entry in &self.state.console {
                            let (elapsed, prefix, text, color) = match entry {
                                ConsoleEntry::StateChange { state, elapsed_secs } =>
                                    (*elapsed_secs, "STATE > ", format!("{}", state), console_gold),
                                ConsoleEntry::CommandSent { command, elapsed_secs } =>
                                    (*elapsed_secs, "CMD > ", command.clone(), tc.red_accent),
                                ConsoleEntry::Message { text, elapsed_secs } =>
                                    (*elapsed_secs, "", text.clone(), tc.label_color),
                            };
                            let mins = (elapsed / 60.0) as u32;
                            let secs = elapsed % 60.0;
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.label(egui::RichText::new(format!("T+{:02}:{:04.1} ", mins, secs)).color(tc.value_color).size(12.5));
                                ui.label(egui::RichText::new("| ").color(tc.label_color).size(12.5));
                                if !prefix.is_empty() {
                                    ui.label(egui::RichText::new(prefix).color(tc.value_color).size(12.5));
                                }
                                ui.label(egui::RichText::new(&text).color(color).family(egui::FontFamily::Name("Bold".into())).size(12.5));
                            });
                        }
                    });
                });
            });

        // === MAP (rightmost) ===
        egui::Panel::right("right_panel")
            .default_size(300.0)
            .min_size(250.0)
            .resizable(true)
            .show(root_ui, |ui| {
                theme::bordered_section(ui, "MAP", tc.accent, dm, |ui| {
                    let current_gps = t.as_ref()
                        .filter(|t| t.latitude != 0.0 || t.longitude != 0.0)
                        .map(|t| (t.latitude, t.longitude));
                    if self.state.lock_gps {
                        self.map_state.memory.follow_my_position();
                    }
                    ui.horizontal(|ui| {
                        let border_color = if self.state.lock_gps { tc.accent } else { tc.label_color };
                        let checkbox_stroke = egui::Stroke::new(1.5, border_color);
                        ui.scope(|ui| {
                            let visuals = &mut ui.style_mut().visuals;
                            visuals.widgets.inactive.bg_stroke = checkbox_stroke;
                            visuals.widgets.hovered.bg_stroke = checkbox_stroke;
                            visuals.widgets.active.bg_stroke = checkbox_stroke;
                            ui.checkbox(&mut self.state.lock_gps, "LOCK ON GPS");
                        });
                    });
                    ui.add_space(4.0);
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

                        let font = egui::FontId::monospace(s);
                        let bold = egui::FontId::new(s, egui::FontFamily::Name("Bold".into()));

                        let rows: &[(&str, String, egui::Color32)] = &[
                            ("LAT", format!("{:.6}\u{00b0}", t.latitude), tc.value_color),
                            ("LON", format!("{:.6}\u{00b0}", t.longitude), tc.value_color),
                            ("ALT", format!("{:.1} m", t.gps_altitude), tc.value_color),
                            ("SAT", format!("{}", t.satellites), if t.satellites >= 4 { tc.green } else { tc.red_accent }),
                        ];
                        for (label, value, color) in rows {
                            painter.text(egui::pos2(x_label, y), egui::Align2::LEFT_TOP, label, font.clone(), tc.label_color);
                            painter.text(egui::pos2(x_value, y), egui::Align2::LEFT_TOP, value, bold.clone(), *color);
                            y += line_h;
                        }
                    }
                });
            });

        // === CENTER: Data grid (sticky) + Charts (scrollable) ===
        egui::CentralPanel::default().show(root_ui, |ui| {
            ui.columns(3, |cols| {
                    theme::bordered_section(&mut cols[0], "STATUS", tc.red_accent, dm, |ui| {
                        if let Some(ref t) = t {
                            theme::data_row(ui, "TICK", &format!("{}", t.tick), dm);
                            theme::data_row(ui, "STATE", &format!("{}", t.state), dm);
                            theme::data_row(ui, "FLAGS", &format!("{}", t.flags), dm);
                            theme::data_row(ui, "LAST COMMAND", &format!("{}", t.last_command), dm);
                            let drogue_color = if t.relay.drogue_fired { tc.red_accent } else { tc.green };
                            theme::data_row_colored(ui, "DROGUE", if t.relay.drogue_fired { "FIRED" } else { "SAFE" }, drogue_color, dm);
                            let chute_color = if t.relay.parachute_fired { tc.red_accent } else { tc.green };
                            theme::data_row_colored(ui, "PARACHUTE", if t.relay.parachute_fired { "FIRED" } else { "SAFE" }, chute_color, dm);
                        }
                    });

                    theme::bordered_section(&mut cols[1], "ALTITUDE", tc.accent, dm, |ui| {
                        if let Some(ref t) = t {
                            theme::data_row(ui, "BARO ALT (AGL)", &format!("{:.2} m", t.baro_altitude), dm);
                            theme::data_row(ui, "BARO VELOCITY", &format!("{:.2} m/s", t.baro_velocity), dm);
                        }
                    });

                    theme::bordered_section(&mut cols[2], "POSITION", tc.accent, dm, |ui| {
                        if let Some(ref t) = t {
                            theme::data_row(ui, "GPS ALT (ASL)", &format!("{:.2} m", t.gps_altitude), dm);
                            theme::data_row(ui, "LATITUDE", &format!("{:.6} \u{00b0}", t.latitude), dm);
                            theme::data_row(ui, "LONGITUDE", &format!("{:.6} \u{00b0}", t.longitude), dm);
                            theme::data_row(ui, "SATELLITES", &format!("{}", t.satellites), dm);
                        }
                    });
                });

                ui.add_space(4.0);

                ui.columns(3, |cols| {
                    theme::bordered_section(&mut cols[0], "FAULTS", tc.red_accent, dm, |ui| {
                        if let Some(ref t) = t {
                            let fault_list = [
                                ("BMP280", t.flags & 0x03),
                                ("BMP581", t.flags & 0x0C),
                                ("IIM42653", t.flags & 0x30),
                                ("IIS2MDCTR", t.flags & 0xC0),
                                ("SD", t.flags & 0x300),
                            ];
                            for (name, bits) in fault_list {
                                let (status, color) = if bits == 0 { ("OK", tc.green) } else { ("FAIL", tc.red_accent) };
                                theme::data_row_colored(ui, name, status, color, dm);
                            }
                        }
                    });

                    theme::bordered_section(&mut cols[1], "ACCELERATION", tc.accent, dm, |ui| {
                        if let Some(ref t) = t {
                            theme::data_row(ui, "ACCEL X", &format!("{:.2} m/s\u{00b2}", t.accel[0]), dm);
                            theme::data_row(ui, "ACCEL Y", &format!("{:.2} m/s\u{00b2}", t.accel[1]), dm);
                            theme::data_row(ui, "ACCEL Z", &format!("{:.2} m/s\u{00b2}", t.accel[2]), dm);
                        }
                    });

                    theme::bordered_section(&mut cols[2], "GYROSCOPE", tc.accent, dm, |ui| {
                        if let Some(ref t) = t {
                            theme::data_row(ui, "GYRO X", &format!("{:.2} \u{00b0}/s", t.gyro[0]), dm);
                            theme::data_row(ui, "GYRO Y", &format!("{:.2} \u{00b0}/s", t.gyro[1]), dm);
                            theme::data_row(ui, "GYRO Z", &format!("{:.2} \u{00b0}/s", t.gyro[2]), dm);
                        }
                    });
                });

                ui.add_space(4.0);

                if self.state.extremes_initialized {
                    theme::bordered_section(ui, "MAX / MIN", egui::Color32::from_rgb(255, 180, 50), dm, |ui| {
                        ui.columns(4, |cols| {
                            theme::data_row(&mut cols[0], "PEAK BARO ALT", &format!("{:.1} m", self.state.max_baro_altitude), dm);
                            theme::data_row(&mut cols[0], "PEAK GPS ALT", &format!("{:.1} m", self.state.max_gps_altitude), dm);
                            theme::data_row(&mut cols[1], "MAX BARO VEL", &format!("{:.1} m/s", self.state.max_baro_velocity), dm);
                            theme::data_row(&mut cols[1], "MAX ACCEL", &format!("{:.1} m/s\u{00b2}", self.state.max_accel_magnitude), dm);
                            theme::data_row(&mut cols[2], "MAX G-FORCE", &format!("{:.1} G", self.state.max_accel_magnitude / 9.81), dm);
                            theme::data_row(&mut cols[2], "MAX TEMP", &format!("{:.1} \u{00b0}C", self.state.max_temperature), dm);
                            theme::data_row(&mut cols[3], "MIN TEMP", &format!("{:.1} \u{00b0}C", self.state.min_temperature), dm);
                            theme::data_row(&mut cols[3], "BATTERY", &format!("{:.2} - {:.2} V", self.state.min_battery_voltage, self.state.max_battery_voltage), dm);
                        });
                    });
                }

                ui.add_space(4.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                theme::bordered_section(ui, "ALTITUDE", tc.accent, dm, |ui| {
                    ui::charts::altitude_chart(ui, &self.state);
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "ACCELERATION", tc.accent, dm, |ui| {
                    ui::charts::acceleration_chart(ui, &self.state);
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "GYROSCOPE", tc.accent, dm, |ui| {
                    ui::charts::gyroscope_chart(ui, &self.state);
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "VELOCITY", tc.accent, dm, |ui| {
                    ui::charts::velocity_chart(ui, &self.state);
                });

                ui.add_space(4.0);
                theme::bordered_section(ui, "RAW PACKET", tc.accent, dm, |ui| {
                    if let Some(ref t) = t {
                        ui::hex_viewer::hex_viewer(ui, &t.raw, packet::PACKET_FIELDS, dm);
                    } else {
                        ui.label(egui::RichText::new("NO DATA").color(tc.label_color));
                    }
                });

            });
        });
    }

    fn render_sd_viewer(&mut self, root_ui: &mut egui::Ui) {
        let dm = self.state.dark_mode;
        let tc = theme::current_theme(dm);

        self.sd_viewer.init_map(root_ui.ctx());

        if self.sd_viewer.replay_playing {
            let wall_now = root_ui.ctx().input(|i| i.time);
            self.sd_viewer.tick_replay(wall_now);
            root_ui.ctx().request_repaint();
        }

        // === DRAG & DROP ===
        let dropped_file = root_ui.ctx().input(|i| {
            i.raw.dropped_files.iter().find_map(|f| {
                let p = f.path();
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext.eq_ignore_ascii_case("bin") {
                    Some(p.display().to_string())
                } else {
                    None
                }
            })
        });
        if let Some(path) = dropped_file {
            self.sd_viewer.load_file(&path);
        }

        // === SD TOP BAR ===
        egui::Panel::top("sd_top_bar")
            .frame(egui::Frame::new().fill(tc.panel_bg).inner_margin(egui::Margin::symmetric(8, 6)))
            .show(root_ui, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.button("OPEN FILE").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Binary", &["bin"])
                        .add_filter("All", &["*"])
                        .pick_file()
                    {
                        self.sd_viewer.load_file(&path.display().to_string());
                    }
                }

                if let Some(ref path) = self.sd_viewer.file_path.clone() {
                    if ui.button("CLOSE").clicked() {
                        self.sd_viewer.close_file();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(path).family(egui::FontFamily::Monospace));
                    ui.separator();
                    ui.label(format!("{} RECORDS", self.sd_viewer.records.len()));
                    ui.separator();
                    let dur = self.sd_viewer.duration_secs();
                    let mins = (dur / 60.0) as u32;
                    let secs = dur % 60.0;
                    ui.label(format!("DURATION: {:02}:{:04.1}", mins, secs));
                }

                if let Some(ref err) = self.sd_viewer.error {
                    ui.separator();
                    ui.colored_label(tc.red_accent, err.as_str());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let border_color = if self.sd_viewer.link_axes { tc.accent } else { tc.label_color };
                    let checkbox_stroke = egui::Stroke::new(1.5, border_color);
                    ui.scope(|ui| {
                        let visuals = &mut ui.style_mut().visuals;
                        visuals.widgets.inactive.bg_stroke = checkbox_stroke;
                        visuals.widgets.hovered.bg_stroke = checkbox_stroke;
                        visuals.widgets.active.bg_stroke = checkbox_stroke;
                        ui.checkbox(&mut self.sd_viewer.link_axes, "SYNC AXES");
                    });
                });
            });
        });

        if self.sd_viewer.records.is_empty() {
            egui::CentralPanel::default().show(root_ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.3);
                    ui.label(egui::RichText::new("OPEN OR DROP A .BIN FILE TO VIEW SD LOG DATA")
                        .size(18.0)
                        .color(tc.label_color));
                    ui.add_space(20.0);
                    let hint_color = tc.label_color.gamma_multiply(0.6);
                    for hint in [
                        "SCROLL TO ZOOM CHARTS, DRAG TO PAN",
                        "DOUBLE CLICK TO RESET ZOOM",
                        "CLICK A FLIGHT STATE SEGMENT TO ZOOM ALL CHARTS TO IT",
                        "CLICK SAME SEGMENT TO RESET ZOOM",
                        "CLICK ANY CHART TO SELECT A RECORD",
                        "\"SYNC AXES\" CHECKBOX LINKS PAN/ZOOM ACROSS CHARTS",
                    ] {
                        ui.label(egui::RichText::new(hint).size(13.0).color(hint_color));
                    }
                });
            });
            return;
        }

        // === SD BOTTOM: Timeline scrubber + Replay controls ===
        egui::Panel::bottom("sd_scrubber")
            .frame(egui::Frame::new().fill(tc.panel_bg).inner_margin(egui::Margin::symmetric(8, 6)))
            .show(root_ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("RECORD");
                let max = self.sd_viewer.records.len().saturating_sub(1);
                ui.add(egui::Slider::new(&mut self.sd_viewer.selected_index, 0..=max)
                    .show_value(true));

                if let Some(r) = self.sd_viewer.selected_record() {
                    if let Some(dt) = chrono::DateTime::from_timestamp(r.unix_time as i64, r.milliseconds as u32 * 1_000_000) {
                        ui.separator();
                        ui.label(egui::RichText::new(dt.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string())
                            .family(egui::FontFamily::Monospace));
                    }
                    ui.separator();
                    ui.label(format!("TICK: {}", r.tick));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    use crate::sd_viewer::state::REPLAY_SPEEDS;

                    let speed = REPLAY_SPEEDS[self.sd_viewer.replay_speed_index];
                    let speed_text = if speed == speed.floor() {
                        format!("x{:.0}", speed)
                    } else {
                        format!("x{:.1}", speed)
                    };
                    ui.label(egui::RichText::new(speed_text)
                        .family(egui::FontFamily::Name("Bold".into()))
                        .size(12.0)
                        .color(tc.accent));

                    let speed_idx = self.sd_viewer.replay_speed_index;
                    if ui.add_enabled(speed_idx < REPLAY_SPEEDS.len() - 1, egui::Button::new(
                        egui::RichText::new("\u{23E9}").size(13.0),
                    )).on_hover_text("FASTER").clicked() {
                        self.sd_viewer.replay_speed_index = (speed_idx + 1).min(REPLAY_SPEEDS.len() - 1);
                    }

                    if ui.button(egui::RichText::new("\u{23F9}").size(13.0))
                        .on_hover_text("STOP")
                        .clicked()
                    {
                        self.sd_viewer.replay_playing = false;
                        self.sd_viewer.replay_last_wall = None;
                        self.sd_viewer.selected_index = 0;
                    }

                    let play_label = if self.sd_viewer.replay_playing { "\u{23F8}" } else { "\u{25B6}" };
                    if ui.button(egui::RichText::new(play_label).size(13.0))
                        .on_hover_text(if self.sd_viewer.replay_playing { "PAUSE" } else { "PLAY" })
                        .clicked()
                    {
                        self.sd_viewer.replay_playing = !self.sd_viewer.replay_playing;
                        if self.sd_viewer.replay_playing {
                            self.sd_viewer.replay_last_wall = None;
                            if self.sd_viewer.selected_index >= self.sd_viewer.records.len().saturating_sub(1) {
                                self.sd_viewer.selected_index = 0;
                            }
                        }
                    }

                    if ui.add_enabled(speed_idx > 0, egui::Button::new(
                        egui::RichText::new("\u{23EA}").size(13.0),
                    )).on_hover_text("SLOWER").clicked() {
                        self.sd_viewer.replay_speed_index = speed_idx.saturating_sub(1);
                    }
                });
            });
        });

        // === MAP (right panel) ===
        egui::Panel::right("sd_map_panel")
            .default_size(300.0)
            .min_size(250.0)
            .resizable(true)
            .show(root_ui, |ui| {
                theme::bordered_section(ui, "MAP", tc.accent, dm, |ui| {
                    let current_gps = self.sd_viewer.selected_record()
                        .filter(|r| r.latitude != 0.0 || r.longitude != 0.0)
                        .map(|r| (r.latitude, r.longitude));
                    if let Some(ref mut map_state) = self.sd_viewer.map_state {
                        if self.sd_viewer.lock_gps {
                            map_state.memory.follow_my_position();
                        }
                        ui.horizontal(|ui| {
                            let border_color = if self.sd_viewer.lock_gps { tc.accent } else { tc.label_color };
                            let checkbox_stroke = egui::Stroke::new(1.5, border_color);
                            ui.scope(|ui| {
                                let visuals = &mut ui.style_mut().visuals;
                                visuals.widgets.inactive.bg_stroke = checkbox_stroke;
                                visuals.widgets.hovered.bg_stroke = checkbox_stroke;
                                visuals.widgets.active.bg_stroke = checkbox_stroke;
                                ui.checkbox(&mut self.sd_viewer.lock_gps, "LOCK ON GPS");
                            });
                        });
                        ui.add_space(4.0);
                        let partial_trail: std::collections::VecDeque<(f64, f64)> = self.sd_viewer.records[..=self.sd_viewer.selected_index]
                            .iter()
                            .filter(|r| r.latitude != 0.0 || r.longitude != 0.0)
                            .map(|r| (r.latitude, r.longitude))
                            .collect();
                        let map_rect = ui::map::gps_map(
                            ui,
                            &partial_trail,
                            current_gps,
                            None,
                            map_state,
                        );

                        if let Some(r) = self.sd_viewer.selected_record() {
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

                            let font = egui::FontId::monospace(s);
                            let bold = egui::FontId::new(s, egui::FontFamily::Name("Bold".into()));

                            let rows: &[(&str, String, egui::Color32)] = &[
                                ("LAT", format!("{:.6}\u{00b0}", r.latitude), tc.value_color),
                                ("LON", format!("{:.6}\u{00b0}", r.longitude), tc.value_color),
                                ("ALT", format!("{:.1} m", r.gps_altitude), tc.value_color),
                                ("SAT", format!("{}", r.satellites), if r.satellites >= 4 { tc.green } else { tc.red_accent }),
                            ];
                            for (label, value, color) in rows {
                                painter.text(egui::pos2(x_label, y), egui::Align2::LEFT_TOP, label, font.clone(), tc.label_color);
                                painter.text(egui::pos2(x_value, y), egui::Align2::LEFT_TOP, value, bold.clone(), *color);
                                y += line_h;
                            }
                        }
                    }
                });
            });

        // === CENTER: Data grid (sticky) + Charts (scrollable) ===
        egui::CentralPanel::default().show(root_ui, |ui| {
            let r = self.sd_viewer.selected_record().cloned();

            ui.columns(4, |cols| {
                    theme::bordered_section(&mut cols[0], "STATUS", tc.red_accent, dm, |ui| {
                        if let Some(ref r) = r {
                            theme::data_row(ui, "TICK", &format!("{}", r.tick), dm);
                            theme::data_row(ui, "STATE", &format!("{}", r.state), dm);
                            theme::data_row(ui, "FLAGS", &format!("{}", r.flags), dm);
                            theme::data_row(ui, "LAST COMMAND", &format!("{}", r.last_command), dm);
                            let drogue_color = if r.relay.drogue_fired { tc.red_accent } else { tc.green };
                            theme::data_row_colored(ui, "DROGUE", if r.relay.drogue_fired { "FIRED" } else { "SAFE" }, drogue_color, dm);
                            let chute_color = if r.relay.parachute_fired { tc.red_accent } else { tc.green };
                            theme::data_row_colored(ui, "PARACHUTE", if r.relay.parachute_fired { "FIRED" } else { "SAFE" }, chute_color, dm);
                        }
                    });

                    theme::bordered_section(&mut cols[1], "POSITION", tc.accent, dm, |ui| {
                        if let Some(ref r) = r {
                            theme::data_row(ui, "GPS ALT (ASL)", &format!("{:.2} m", r.gps_altitude), dm);
                            theme::data_row(ui, "LATITUDE", &format!("{:.6} \u{00b0}", r.latitude), dm);
                            theme::data_row(ui, "LONGITUDE", &format!("{:.6} \u{00b0}", r.longitude), dm);
                            theme::data_row(ui, "SATELLITES", &format!("{}", r.satellites), dm);
                        }
                    });

                    theme::bordered_section(&mut cols[2], "SENSORS", tc.accent, dm, |ui| {
                        if let Some(ref r) = r {
                            theme::data_row(ui, "PRESSURE", &format!("{:.0} Pa", r.pressure_pa), dm);
                            theme::data_row(ui, "TEMPERATURE", &format!("{:.2} \u{00b0}C", r.temperature_c), dm);
                            theme::data_row(ui, "BATTERY", &format!("{:.2} V", r.battery_voltage), dm);
                        }
                    });

                    theme::bordered_section(&mut cols[3], "MAGNETOMETER", tc.accent, dm, |ui| {
                        if let Some(ref r) = r {
                            theme::data_row(ui, "MAG X", &format!("{:.2} mG", r.mag[0]), dm);
                            theme::data_row(ui, "MAG Y", &format!("{:.2} mG", r.mag[1]), dm);
                            theme::data_row(ui, "MAG Z", &format!("{:.2} mG", r.mag[2]), dm);
                        }
                    });
                });

                ui.add_space(4.0);

                ui.columns(4, |cols| {
                    theme::bordered_section(&mut cols[0], "FAULTS", tc.red_accent, dm, |ui| {
                        if let Some(ref r) = r {
                            let fault_list = [
                                ("BMP280", r.flags & 0x03),
                                ("BMP581", r.flags & 0x0C),
                                ("IIM42653", r.flags & 0x30),
                                ("IIS2MDCTR", r.flags & 0xC0),
                                ("SD", r.flags & 0x300),
                            ];
                            for (name, bits) in fault_list {
                                let (status, color) = if bits == 0 { ("OK", tc.green) } else { ("FAIL", tc.red_accent) };
                                theme::data_row_colored(ui, name, status, color, dm);
                            }
                        }
                    });

                    theme::bordered_section(&mut cols[1], "ACCELERATION", tc.accent, dm, |ui| {
                        if let Some(ref r) = r {
                            theme::data_row(ui, "ACCEL X", &format!("{:.2} m/s\u{00b2}", r.accel[0]), dm);
                            theme::data_row(ui, "ACCEL Y", &format!("{:.2} m/s\u{00b2}", r.accel[1]), dm);
                            theme::data_row(ui, "ACCEL Z", &format!("{:.2} m/s\u{00b2}", r.accel[2]), dm);
                        }
                    });

                    theme::bordered_section(&mut cols[2], "GYROSCOPE", tc.accent, dm, |ui| {
                        if let Some(ref r) = r {
                            theme::data_row(ui, "GYRO X", &format!("{:.2} \u{00b0}/s", r.gyro[0]), dm);
                            theme::data_row(ui, "GYRO Y", &format!("{:.2} \u{00b0}/s", r.gyro[1]), dm);
                            theme::data_row(ui, "GYRO Z", &format!("{:.2} \u{00b0}/s", r.gyro[2]), dm);
                        }
                    });

                    theme::bordered_section(&mut cols[3], "TIMESTAMP", tc.accent, dm, |ui| {
                        if let Some(ref r) = r {
                            theme::data_row(ui, "UNIX TIME", &format!("{}", r.unix_time), dm);
                            theme::data_row(ui, "MILLIS", &format!("{}", r.milliseconds), dm);
                            if let Some(dt) = chrono::DateTime::from_timestamp(r.unix_time as i64, r.milliseconds as u32 * 1_000_000) {
                                theme::data_row(ui, "UTC", &dt.format("%H:%M:%S%.3f").to_string(), dm);
                            }
                        }
                    });
                });

                ui.add_space(4.0);

            egui::ScrollArea::vertical().id_salt("sd_scroll").show(ui, |ui| {
                let selected_t = self.sd_viewer.timestamps.get(self.sd_viewer.selected_index).copied();
                let zoom_x = self.sd_viewer.zoom_x.take();
                let reset = std::mem::take(&mut self.sd_viewer.reset_zoom);
                let link_axes = self.sd_viewer.link_axes;

                use crate::sd_viewer::charts;
                let mut clicked_ts: Option<f64> = None;
                let mut new_zoom: Option<(f64, f64)> = None;

                theme::bordered_section(ui, "FLIGHT STATE", tc.accent, dm, |ui| {
                    if let Some(click) = charts::state_timeline_chart(ui, &self.sd_viewer.state_segments, &self.sd_viewer.timeline_markers, selected_t, zoom_x, link_axes, reset) {
                        match click {
                            charts::TimelineClick::Segment { start, end } => {
                                if self.sd_viewer.zoomed_segment == Some((start, end)) {
                                    self.sd_viewer.zoomed_segment = None;
                                    self.sd_viewer.reset_zoom = true;
                                } else {
                                    let padding = (end - start) * 0.05;
                                    new_zoom = Some((start - padding, end + padding));
                                    self.sd_viewer.zoomed_segment = Some((start, end));
                                }
                            }
                            charts::TimelineClick::Point(t) => {
                                clicked_ts = Some(t);
                            }
                        }
                    }
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "GPS ALTITUDE", tc.accent, dm, |ui| {
                    if let Some(t) = charts::single_series_chart(ui, "sd_gps_alt", "GPS ALT", "m", &self.sd_viewer.timestamps, &self.sd_viewer.gps_altitude, selected_t, zoom_x, link_axes, reset) {
                        clicked_ts = Some(t);
                    }
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "ACCELERATION", tc.accent, dm, |ui| {
                    if let Some(t) = charts::triple_series_chart(ui, "sd_accel", "m/s\u{00b2}", &self.sd_viewer.timestamps, &self.sd_viewer.accel_x, &self.sd_viewer.accel_y, &self.sd_viewer.accel_z, selected_t, zoom_x, link_axes, reset) {
                        clicked_ts = Some(t);
                    }
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "GYROSCOPE", tc.accent, dm, |ui| {
                    if let Some(t) = charts::triple_series_chart(ui, "sd_gyro", "\u{00b0}/s", &self.sd_viewer.timestamps, &self.sd_viewer.gyro_x, &self.sd_viewer.gyro_y, &self.sd_viewer.gyro_z, selected_t, zoom_x, link_axes, reset) {
                        clicked_ts = Some(t);
                    }
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "PRESSURE", tc.accent, dm, |ui| {
                    if let Some(t) = charts::single_series_chart(ui, "sd_pressure", "PRESSURE", "Pa", &self.sd_viewer.timestamps, &self.sd_viewer.pressure, selected_t, zoom_x, link_axes, reset) {
                        clicked_ts = Some(t);
                    }
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "TEMPERATURE", tc.accent, dm, |ui| {
                    if let Some(t) = charts::single_series_chart(ui, "sd_temp", "TEMP", "\u{00b0}C", &self.sd_viewer.timestamps, &self.sd_viewer.temperature, selected_t, zoom_x, link_axes, reset) {
                        clicked_ts = Some(t);
                    }
                });
                ui.add_space(4.0);
                theme::bordered_section(ui, "BATTERY", tc.accent, dm, |ui| {
                    if let Some(t) = charts::single_series_chart(ui, "sd_battery", "BATTERY", "V", &self.sd_viewer.timestamps, &self.sd_viewer.battery, selected_t, zoom_x, link_axes, reset) {
                        clicked_ts = Some(t);
                    }
                });

                if let Some(z) = new_zoom {
                    self.sd_viewer.zoom_x = Some(z);
                }

                if let Some(t) = clicked_ts {
                    self.sd_viewer.selected_index = charts::timestamp_to_index(&self.sd_viewer.timestamps, t);
                }

                ui.add_space(4.0);
                theme::bordered_section(ui, "RAW RECORD", tc.accent, dm, |ui| {
                    if let Some(ref r) = r {
                        ui::hex_viewer::hex_viewer(ui, &r.raw, SD_RECORD_FIELDS, dm);
                    } else {
                        ui.label(egui::RichText::new("NO DATA").color(tc.label_color));
                    }
                });
            }); // ScrollArea
        });
    }
}

impl eframe::App for GroundStationApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let dm = self.state.dark_mode;
        let tc = theme::current_theme(dm);

        if let Ok(pos) = self.device_pos.try_lock() {
            self.state.ground_pos = *pos;
        }

        // Always process serial events regardless of active tab
        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                SerialEvent::Packet(t) => {
                    if let Some(ref mut rec) = self.csv_recorder {
                        rec.record(&t);
                    }
                    self.state.push_telemetry(*t);
                }
                SerialEvent::Connected(name) => {
                    self.state.connected = true;
                    self.state.push_message(&format!("Connected to {}", name));
                }
                SerialEvent::Disconnected => {
                    self.state.connected = false;
                    self.state.push_message("Disconnected");
                }
                SerialEvent::Error(e) => {
                    self.state.push_message(&e);
                }
                SerialEvent::PortList(ports) => {
                    self.state.available_ports = ports;
                }
            }
        }

        // === ABOUT WINDOW ===
        egui::Window::new("About")
            .open(&mut self.show_about)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(egui::Frame::new().fill(tc.modal_bg).stroke(egui::Stroke::new(1.0, tc.modal_stroke)).inner_margin(40.0).corner_radius(4.0))
            .show(root_ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(ref tex) = self.logo_texture {
                        let logo_size = egui::vec2(100.0, 100.0);
                        ui.image(egui::load::SizedTexture::new(tex.id(), logo_size));
                        ui.add_space(12.0);
                    }
                    ui.label(egui::RichText::new("COHETEROS GROUND STATION").family(egui::FontFamily::Name("Bold".into())).size(22.0).color(tc.accent));
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).size(15.0).color(tc.label_color));
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("BUILT BY THE COHETEROS TEAM\nFOR THE EUROPEAN ROCKETRY CHALLENGE").size(14.0).color(tc.value_color));
                    ui.add_space(16.0);
                    let link_color = egui::Color32::from_rgb(100, 149, 237);
                    ui.hyperlink_to(egui::RichText::new("coheteros.com").size(14.0).color(link_color), "https://coheteros.com");
                    ui.hyperlink_to(egui::RichText::new("LinkedIn").size(14.0).color(link_color), "https://www.linkedin.com/company/coheteros-us/");
                    ui.hyperlink_to(egui::RichText::new("GitHub").size(14.0).color(link_color), "https://github.com/CoheterosUS");
                    ui.add_space(24.0);
                    ui.label(egui::RichText::new("ANGELO WAS HERE").size(9.0).color(tc.label_color.gamma_multiply(0.3)));
                });
            });

        // === TAB BAR ===
        egui::Panel::top("tab_bar")
            .frame(egui::Frame::new().fill(tc.panel_bg).inner_margin(egui::Margin::symmetric(8, 4)))
            .show(root_ui, |ui| {
                ui.horizontal(|ui| {
                    let tabs = [
                        (ActiveTab::LiveTelemetry, "LIVE TELEMETRY"),
                        (ActiveTab::SdViewer, "SD VIEWER"),
                    ];

                    for (tab, label) in tabs {
                        let active = self.active_tab == tab;
                        let text_color = if active { tc.accent } else { tc.label_color };
                        let border_color = if active { tc.accent } else { egui::Color32::TRANSPARENT };

                        let button = egui::Button::new(
                            egui::RichText::new(label)
                                .family(egui::FontFamily::Name("Bold".into()))
                                .size(13.0)
                                .color(text_color),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::new(2.0, border_color))
                        .corner_radius(0.0);

                        if ui.add(button).clicked() {
                            self.active_tab = tab;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let theme_label = if self.state.dark_mode { "LIGHT" } else { "DARK" };
                        if ui.button(theme_label).clicked() {
                            self.state.dark_mode = !self.state.dark_mode;
                            theme::apply_visuals(ui.ctx(), self.state.dark_mode);
                        }
                        if ui.button("ABOUT").clicked() {
                            self.show_about = !self.show_about;
                        }
                    });
                });
            });

        match self.active_tab {
            ActiveTab::LiveTelemetry => self.render_live_telemetry(root_ui),
            ActiveTab::SdViewer => self.render_sd_viewer(root_ui),
        }
    }
}
