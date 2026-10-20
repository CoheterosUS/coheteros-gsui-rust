use crate::sd_viewer::state::{FlightStateExt, SdViewerState};

/// A 3D point in local ENU (East, North, Up) meters relative to the first valid GPS fix.
struct EnuPoint {
    east: f64,
    north: f64,
    up: f64,
    #[allow(dead_code)]
    timestamp: f64,
    state_color: egui::Color32,
}

/// Convert lat/lon/alt to local ENU meters relative to a reference point.
fn geodetic_to_enu(lat: f64, lon: f64, alt: f64, ref_lat: f64, ref_lon: f64, ref_alt: f64) -> (f64, f64, f64) {
    let dlat = (lat - ref_lat).to_radians();
    let dlon = (lon - ref_lon).to_radians();
    let ref_lat_rad = ref_lat.to_radians();
    let r = 6_371_000.0;
    let east = dlon * ref_lat_rad.cos() * r;
    let north = dlat * r;
    let up = alt - ref_alt;
    (east, north, up)
}

/// Build interpolated ENU trajectory from SD records.
/// GPS gives horizontal (lat/lon), baro_altitude gives vertical, both at different rates.
/// We sample at every record tick and interpolate GPS horizontally between valid fixes.
fn build_trajectory(v: &SdViewerState) -> (Vec<EnuPoint>, Option<(f64, f64, f64)>) {
    if v.records.is_empty() {
        return (Vec::new(), None);
    }

    // Find first valid GPS fix as reference origin
    let ref_fix = v.records.iter().find(|r| r.latitude != 0.0 || r.longitude != 0.0);
    let (ref_lat, ref_lon, ref_alt) = match ref_fix {
        Some(r) => (r.latitude, r.longitude, r.gps_altitude),
        None => return (Vec::new(), None),
    };

    // Collect GPS fixes with their indices for interpolation
    let gps_fixes: Vec<(usize, f64, f64)> = v.records.iter().enumerate()
        .filter(|(_, r)| r.latitude != 0.0 || r.longitude != 0.0)
        .map(|(i, r)| {
            let (e, n, _) = geodetic_to_enu(r.latitude, r.longitude, r.gps_altitude, ref_lat, ref_lon, ref_alt);
            (i, e, n)
        })
        .collect();

    if gps_fixes.is_empty() {
        return (Vec::new(), None);
    }

    // Downsample: take every Nth record to keep point count manageable
    let total = v.records.len();
    let max_points = 2000;
    let step = (total / max_points).max(1);

    let mut points = Vec::with_capacity(total / step + 1);
    let mut fix_cursor = 0;

    for i in (0..total).step_by(step) {
        // Interpolate horizontal from GPS fixes
        // Advance cursor to bracket current index
        while fix_cursor + 1 < gps_fixes.len() && gps_fixes[fix_cursor + 1].0 <= i {
            fix_cursor += 1;
        }

        let (east, north) = if fix_cursor + 1 < gps_fixes.len() {
            let (i0, e0, n0) = gps_fixes[fix_cursor];
            let (i1, e1, n1) = gps_fixes[fix_cursor + 1];
            let span = (i1 - i0) as f64;
            if span > 0.0 {
                let t = (i - i0) as f64 / span;
                let t = t.clamp(0.0, 1.0);
                (e0 + t * (e1 - e0), n0 + t * (n1 - n0))
            } else {
                (e0, n0)
            }
        } else {
            let (_, e, n) = gps_fixes[fix_cursor];
            (e, n)
        };

        // Vertical: prefer baro_altitude (AGL), fall back to gps_altitude
        let up = if let Some(&ba) = v.baro_altitude.get(i) {
            if ba != 0.0 { ba } else { v.records[i].gps_altitude - ref_alt }
        } else {
            v.records[i].gps_altitude - ref_alt
        };

        let timestamp = v.timestamps.get(i).copied().unwrap_or(0.0);
        let state_color = v.records[i].state.timeline_color();

        points.push(EnuPoint { east, north, up, timestamp, state_color });
    }

    (points, Some((ref_lat, ref_lon, ref_alt)))
}

pub fn trajectory_3d_viewer(
    ui: &mut egui::Ui,
    v: &SdViewerState,
    selected_index: usize,
    dark_mode: bool,
) {
    let size = egui::vec2(ui.available_width(), ui.available_height().max(300.0));
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let center = response.rect.center();

    let (points, _ref_origin) = build_trajectory(v);
    if points.is_empty() {
        let tc = if dark_mode { egui::Color32::from_rgb(120, 120, 120) } else { egui::Color32::from_rgb(150, 150, 150) };
        painter.text(center, egui::Align2::CENTER_CENTER, "NO GPS DATA", egui::FontId::proportional(16.0), tc);
        return;
    }

    // Compute bounding box for auto-scaling
    let (mut min_e, mut max_e) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_n, mut max_n) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_u, mut max_u) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in &points {
        min_e = min_e.min(p.east);  max_e = max_e.max(p.east);
        min_n = min_n.min(p.north); max_n = max_n.max(p.north);
        min_u = min_u.min(p.up);    max_u = max_u.max(p.up);
    }
    let center_e = (min_e + max_e) / 2.0;
    let center_n = (min_n + max_n) / 2.0;
    let center_u = (min_u + max_u) / 2.0;
    let extent = (max_e - min_e).max(max_n - min_n).max(max_u - min_u).max(1.0);

    // Orbit camera state
    let default_cam = (0.8_f32, 1.0_f32, 2.5_f32, egui::Vec2::ZERO);
    let mut cam_st: (f32, f32, f32, egui::Vec2) =
        ui.data_mut(|d| *d.get_temp_mut_or(response.id, default_cam));
    if response.double_clicked() {
        cam_st = default_cam;
    }
    let drag = response.drag_delta();
    if response.dragged_by(egui::PointerButton::Primary) {
        cam_st.0 += drag.x * 0.008;
        cam_st.1 = (cam_st.1 - drag.y * 0.008).clamp(0.15, 2.9);
    }
    if response.dragged_by(egui::PointerButton::Secondary) || response.dragged_by(egui::PointerButton::Middle) {
        cam_st.3 += egui::vec2(-drag.x, drag.y) * cam_st.2 * 0.0035;
    }
    if response.hovered() {
        let sc = ui.input_mut(|i| {
            let s = i.smooth_scroll_delta.y;
            if s != 0.0 { i.smooth_scroll_delta = egui::Vec2::ZERO; }
            s
        });
        if sc != 0.0 {
            cam_st.2 = (cam_st.2 * (-sc * 0.0015).exp()).clamp(0.8, 20.0);
        }
    }
    ui.data_mut(|d| d.insert_temp(response.id, cam_st));

    let (az, pol, dist) = (cam_st.0 as f64, cam_st.1 as f64, cam_st.2 as f64);

    // Camera in ENU-like space: Up = +Z on screen
    let cdir = [pol.sin() * az.cos(), pol.sin() * az.sin(), -pol.cos()];
    let fwd = [-cdir[0], -cdir[1], -cdir[2]];
    let up_ref = [0.0_f64, 0.0, -1.0]; // -Z = Up in our rendering space
    let right = {
        let c = [
            fwd[1] * up_ref[2] - fwd[2] * up_ref[1],
            fwd[2] * up_ref[0] - fwd[0] * up_ref[2],
            fwd[0] * up_ref[1] - fwd[1] * up_ref[0],
        ];
        let l = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt().max(1e-9);
        [c[0] / l, c[1] / l, c[2] / l]
    };
    let up = [
        right[1] * fwd[2] - right[2] * fwd[1],
        right[2] * fwd[0] - right[0] * fwd[2],
        right[0] * fwd[1] - right[1] * fwd[0],
    ];

    let (px, py) = (cam_st.3.x as f64, cam_st.3.y as f64);
    let tgt = [
        right[0] * px + up[0] * py,
        right[1] * px + up[1] * py,
        right[2] * px + up[2] * py,
    ];
    let cam = [tgt[0] + dist * cdir[0], tgt[1] + dist * cdir[1], tgt[2] + dist * cdir[2]];
    let fs = size.y as f64 * 1.1;

    // Normalize ENU coordinates to [-1, 1] range based on extent
    let scale = 1.0 / (extent * 0.6);
    let to_scene = |east: f64, north: f64, up_val: f64| -> [f64; 3] {
        // Scene coords: X = East, Y = North, Z = -Up (so Up shows as screen-up)
        [
            (east - center_e) * scale,
            (north - center_n) * scale,
            -(up_val - center_u) * scale,
        ]
    };

    let project = |p: [f64; 3]| -> (egui::Pos2, f64) {
        let d = [p[0] - cam[0], p[1] - cam[1], p[2] - cam[2]];
        let cx = d[0] * right[0] + d[1] * right[1] + d[2] * right[2];
        let cy = d[0] * up[0] + d[1] * up[1] + d[2] * up[2];
        let cz = d[0] * fwd[0] + d[1] * fwd[1] + d[2] * fwd[2];
        let k = fs / cz.max(1e-6);
        (egui::pos2(center.x + (cx * k) as f32, center.y - (cy * k) as f32), cz)
    };

    let ref_color = if dark_mode { egui::Color32::from_rgb(55, 55, 55) } else { egui::Color32::from_rgb(195, 195, 195) };
    let label_color = if dark_mode { egui::Color32::from_rgb(100, 100, 100) } else { egui::Color32::from_rgb(140, 140, 140) };
    let grid_color = if dark_mode { egui::Color32::from_rgb(40, 40, 40) } else { egui::Color32::from_rgb(210, 210, 210) };

    // Draw ground grid at min altitude
    let grid_u = min_u;
    let grid_steps = 5;
    let grid_span_e = max_e - min_e;
    let grid_span_n = max_n - min_n;
    let grid_span = grid_span_e.max(grid_span_n).max(1.0);
    let grid_step = grid_span / grid_steps as f64;

    for i in 0..=grid_steps {
        let t = min_e + i as f64 * grid_step;
        let a = to_scene(t, min_n, grid_u);
        let b = to_scene(t, min_n + grid_span, grid_u);
        let (pa, za) = project(a);
        let (pb, zb) = project(b);
        if za > 0.05 && zb > 0.05 {
            painter.line_segment([pa, pb], egui::Stroke::new(0.5, grid_color));
        }
    }
    for i in 0..=grid_steps {
        let t = min_n + i as f64 * grid_step;
        let a = to_scene(min_e, t, grid_u);
        let b = to_scene(min_e + grid_span, t, grid_u);
        let (pa, za) = project(a);
        let (pb, zb) = project(b);
        if za > 0.05 && zb > 0.05 {
            painter.line_segment([pa, pb], egui::Stroke::new(0.5, grid_color));
        }
    }

    // Axis labels at corners
    let axis_len = extent * scale * 0.25;
    let origin = to_scene(min_e, min_n, grid_u);
    for (dir, label, color) in [
        ([axis_len, 0.0, 0.0], "E", egui::Color32::from_rgb(70, 175, 90)),
        ([0.0, axis_len, 0.0], "N", egui::Color32::from_rgb(200, 70, 70)),
        ([0.0, 0.0, -axis_len], "UP", egui::Color32::from_rgb(85, 135, 225)),
    ] {
        let tip = [origin[0] + dir[0], origin[1] + dir[1], origin[2] + dir[2]];
        let (po, zo) = project(origin);
        let (pt, zt) = project(tip);
        if zo > 0.05 && zt > 0.05 {
            painter.line_segment([po, pt], egui::Stroke::new(1.5, color));
            painter.text(pt, egui::Align2::LEFT_BOTTOM, label, egui::FontId::proportional(11.0), color);
        }
    }

    // Draw vertical drop lines from trajectory to ground (every Nth point)
    let drop_step = (points.len() / 40).max(1);
    for p in points.iter().step_by(drop_step) {
        let top = to_scene(p.east, p.north, p.up);
        let bot = to_scene(p.east, p.north, grid_u);
        let (pt, zt) = project(top);
        let (pb, zb) = project(bot);
        if zt > 0.05 && zb > 0.05 {
            painter.line_segment([pt, pb], egui::Stroke::new(0.5, ref_color));
        }
    }

    // Draw ground shadow (trajectory projected onto ground plane)
    for w in points.windows(2) {
        let a = to_scene(w[0].east, w[0].north, grid_u);
        let b = to_scene(w[1].east, w[1].north, grid_u);
        let (pa, za) = project(a);
        let (pb, zb) = project(b);
        if za > 0.05 && zb > 0.05 {
            painter.line_segment([pa, pb], egui::Stroke::new(1.0, ref_color));
        }
    }

    // Draw 3D trajectory line, colored by flight state
    for w in points.windows(2) {
        let a = to_scene(w[0].east, w[0].north, w[0].up);
        let b = to_scene(w[1].east, w[1].north, w[1].up);
        let (pa, za) = project(a);
        let (pb, zb) = project(b);
        if za > 0.05 && zb > 0.05 {
            painter.line_segment([pa, pb], egui::Stroke::new(2.5, w[1].state_color));
        }
    }

    // Draw cursor position (current selected record)
    let total = v.records.len();
    let max_points_count = 2000;
    let step = (total / max_points_count).max(1);
    let cursor_pt_idx = selected_index / step;
    if let Some(cp) = points.get(cursor_pt_idx.min(points.len().saturating_sub(1))) {
        let pos = to_scene(cp.east, cp.north, cp.up);
        let (screen_pos, z) = project(pos);
        if z > 0.05 {
            // Drop line from cursor to ground
            let ground = to_scene(cp.east, cp.north, grid_u);
            let (gp, gz) = project(ground);
            if gz > 0.05 {
                painter.line_segment([screen_pos, gp], egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 100)));
            }
            painter.circle_filled(screen_pos, 6.0, egui::Color32::BLACK);
            painter.circle_filled(screen_pos, 4.0, egui::Color32::from_rgb(255, 80, 80));

            // Altitude label
            let alt_text = format!("{:.0}m", cp.up);
            painter.text(
                screen_pos + egui::vec2(10.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                &alt_text,
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(255, 255, 100),
            );
        }
    }

    // Draw launch point (first point)
    if let Some(first) = points.first() {
        let pos = to_scene(first.east, first.north, first.up);
        let (sp, sz) = project(pos);
        if sz > 0.05 {
            painter.circle_filled(sp, 5.0, egui::Color32::BLACK);
            painter.circle_filled(sp, 3.0, egui::Color32::from_rgb(0, 220, 0));
        }
    }

    // Scale bar in bottom-left corner
    let nice_scale = nice_round(extent * 0.3);
    if nice_scale > 0.0 {
        let bar_start = to_scene(min_e, min_n, grid_u);
        let bar_end = to_scene(min_e + nice_scale, min_n, grid_u);
        let (ps, zs) = project(bar_start);
        let (pe, ze) = project(bar_end);
        if zs > 0.05 && ze > 0.05 {
            let bar_y = response.rect.bottom() - 20.0;
            let bar_x = response.rect.left() + 15.0;
            let bar_len = ((pe.x - ps.x).powi(2) + (pe.y - ps.y).powi(2)).sqrt().min(150.0);
            let p1 = egui::pos2(bar_x, bar_y);
            let p2 = egui::pos2(bar_x + bar_len, bar_y);
            painter.line_segment([p1, p2], egui::Stroke::new(2.0, label_color));
            painter.line_segment([p1, p1 - egui::vec2(0.0, 4.0)], egui::Stroke::new(1.5, label_color));
            painter.line_segment([p2, p2 - egui::vec2(0.0, 4.0)], egui::Stroke::new(1.5, label_color));
            let label = if nice_scale >= 1000.0 {
                format!("{:.0} km", nice_scale / 1000.0)
            } else {
                format!("{:.0} m", nice_scale)
            };
            painter.text(
                egui::pos2(bar_x + bar_len / 2.0, bar_y - 6.0),
                egui::Align2::CENTER_BOTTOM,
                &label,
                egui::FontId::proportional(11.0),
                label_color,
            );
        }
    }

    // Hint text
    let hint_color = if dark_mode { egui::Color32::from_rgb(70, 70, 70) } else { egui::Color32::from_rgb(170, 170, 170) };
    painter.text(
        egui::pos2(response.rect.right() - 8.0, response.rect.bottom() - 8.0),
        egui::Align2::RIGHT_BOTTOM,
        "DRAG: ORBIT  |  SCROLL: ZOOM  |  DBL-CLICK: RESET",
        egui::FontId::proportional(10.0),
        hint_color,
    );
}

fn nice_round(val: f64) -> f64 {
    if val <= 0.0 { return 0.0; }
    let mag = 10.0_f64.powf(val.log10().floor());
    let norm = val / mag;
    let nice = if norm < 1.5 { 1.0 } else if norm < 3.5 { 2.0 } else if norm < 7.5 { 5.0 } else { 10.0 };
    nice * mag
}
