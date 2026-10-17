use std::collections::BTreeMap;
use egui::Align2;
use egui_plot::{Bar, BarChart, Corner, Legend, Line, Plot, PlotPoints, VLine};
use crate::sd_viewer::state::{GapInfo, StateSegment, TimelineMarker};

const LINK_GROUP: &str = "sd_x_link";

fn series_points(timestamps: &[f64], values: &[f64], max_points: usize) -> Vec<[f64; 2]> {
    let len = timestamps.len().min(values.len());
    if len == 0 {
        return vec![];
    }
    let step = (len / max_points.max(1)).max(1);
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

fn sd_plot(id: &str, link_axes: bool, reset: bool) -> Plot<'_> {
    let mut plot = Plot::new(id)
        .height(150.0)
        .allow_drag(true)
        .allow_zoom(true)
        .allow_scroll(false)
        .show_axes(true)
        .show_crosshair(true);
    if link_axes {
        plot = plot.link_axis(egui::Id::new(LINK_GROUP), [true, false]);
    }
    if reset {
        plot = plot.reset();
    }
    plot
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

fn cursor_line(ctx: &egui::Context, selected_t: Option<f64>) -> Option<VLine> {
    let color = if ctx.global_style().visuals.dark_mode {
        egui::Color32::from_rgba_premultiplied(200, 200, 200, 120)
    } else {
        egui::Color32::BLACK
    };
    selected_t.map(|t| {
        VLine::new("", t)
            .color(color)
            .width(1.0)
            .style(egui_plot::LineStyle::Dashed { length: 4.0 })
    })
}

pub fn single_series_chart(
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    unit: &str,
    timestamps: &[f64],
    values: &[f64],
    max_points: usize,
    selected_t: Option<f64>,
    zoom_x: Option<(f64, f64)>,
    link_axes: bool,
    reset: bool,
) -> Option<f64> {
    let points: PlotPoints = series_points(timestamps, values, max_points).into();
    let ts = timestamps;
    let vals = values;
    let n = name.to_string();
    let u = unit.to_string();
    let plot_response = sd_plot(id, link_axes, reset)
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            let val = lookup_by_timestamp(ts, vals, x)?;
            Some(format!("{}: {:.2} {}", n, val, u))
        })
        .show(ui, |plot_ui| {
            if let Some((lo, hi)) = zoom_x {
                plot_ui.set_plot_bounds_x(lo..=hi);
            }
            plot_ui.line(Line::new(name, points));
            if let Some(vline) = cursor_line(plot_ui.ctx(), selected_t) {
                plot_ui.vline(vline);
            }
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
    max_points: usize,
    selected_t: Option<f64>,
    zoom_x: Option<(f64, f64)>,
    link_axes: bool,
    reset: bool,
) -> Option<f64> {
    let px: PlotPoints = series_points(timestamps, x_vals, max_points).into();
    let py: PlotPoints = series_points(timestamps, y_vals, max_points).into();
    let pz: PlotPoints = series_points(timestamps, z_vals, max_points).into();
    let ts = timestamps;
    let xv = x_vals;
    let yv = y_vals;
    let zv = z_vals;
    let u = unit.to_string();
    let plot_response = sd_plot(id, link_axes, reset)
        .legend(Legend::default().position(Corner::LeftTop))
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
            if let Some((lo, hi)) = zoom_x {
                plot_ui.set_plot_bounds_x(lo..=hi);
            }
            plot_ui.line(Line::new("X", px));
            plot_ui.line(Line::new("Y", py));
            plot_ui.line(Line::new("Z", pz));
            if let Some(vline) = cursor_line(plot_ui.ctx(), selected_t) {
                plot_ui.vline(vline);
            }
        });
    handle_click(&plot_response.response, &plot_response.transform)
}

pub fn quad_series_chart(
    ui: &mut egui::Ui,
    id: &str,
    timestamps: &[f64],
    w_vals: &[f64],
    x_vals: &[f64],
    y_vals: &[f64],
    z_vals: &[f64],
    max_points: usize,
    selected_t: Option<f64>,
    zoom_x: Option<(f64, f64)>,
    link_axes: bool,
    reset: bool,
) -> Option<f64> {
    let pw: PlotPoints = series_points(timestamps, w_vals, max_points).into();
    let px: PlotPoints = series_points(timestamps, x_vals, max_points).into();
    let py: PlotPoints = series_points(timestamps, y_vals, max_points).into();
    let pz: PlotPoints = series_points(timestamps, z_vals, max_points).into();
    let ts = timestamps;
    let wv = w_vals;
    let xv = x_vals;
    let yv = y_vals;
    let zv = z_vals;
    let plot_response = sd_plot(id, link_axes, reset)
        .legend(Legend::default().position(Corner::LeftTop))
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            let mut lines = Vec::new();
            for (name, vals) in [("W", wv), ("X", xv), ("Y", yv), ("Z", zv)] {
                if let Some(val) = lookup_by_timestamp(ts, vals, x) {
                    lines.push(format!("{}: {:.4}", name, val));
                }
            }
            if lines.is_empty() { None } else { Some(lines.join("\n")) }
        })
        .show(ui, |plot_ui| {
            if let Some((lo, hi)) = zoom_x {
                plot_ui.set_plot_bounds_x(lo..=hi);
            }
            plot_ui.line(Line::new("W", pw));
            plot_ui.line(Line::new("X", px));
            plot_ui.line(Line::new("Y", py));
            plot_ui.line(Line::new("Z", pz));
            if let Some(vline) = cursor_line(plot_ui.ctx(), selected_t) {
                plot_ui.vline(vline);
            }
        });
    handle_click(&plot_response.response, &plot_response.transform)
}

pub fn quaternion_orientation_viewer(
    ui: &mut egui::Ui,
    qw: f64, qx: f64, qy: f64, qz: f64,
    dark_mode: bool,
) {
    let size = egui::vec2(ui.available_width(), ui.available_height().max(260.0));
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let center = response.rect.center();

    let body_rgb = if dark_mode { 185.0 } else { 150.0 };
    let body_color = if dark_mode { egui::Color32::from_rgb(180, 180, 180) } else { egui::Color32::from_rgb(60, 60, 60) };
    let fin_color = if dark_mode { egui::Color32::from_rgb(100, 100, 100) } else { egui::Color32::from_rgb(120, 120, 120) };
    // wireframe rocket by default; toggle button (drawn last) switches to the STL mesh
    let stl_id = response.id.with("stl");
    let show_stl = ui.data(|d| d.get_temp::<bool>(stl_id).unwrap_or(false));
    let ref_color = if dark_mode { egui::Color32::from_rgb(45, 45, 45) } else { egui::Color32::from_rgb(205, 205, 205) };
    let label_color = if dark_mode { egui::Color32::from_rgb(90, 90, 90) } else { egui::Color32::from_rgb(150, 150, 150) };
    let ax_n = egui::Color32::from_rgb(200, 70, 70);
    let ax_e = egui::Color32::from_rgb(70, 175, 90);
    let ax_u = egui::Color32::from_rgb(85, 135, 225);

    // orbit camera state: azimuth, polar (from Up axis), distance, pan
    let default_cam = (0.35_f32, 1.1_f32, 3.2_f32, egui::Vec2::ZERO);
    let mut cam_st: (f32, f32, f32, egui::Vec2) =
        ui.data_mut(|d| *d.get_temp_mut_or(response.id, default_cam));
    if response.double_clicked() {
        cam_st = default_cam;
    }
    let drag = response.drag_delta();
    if response.dragged_by(egui::PointerButton::Primary) {
        cam_st.0 += drag.x * 0.008;
        cam_st.1 = (cam_st.1 - drag.y * 0.008).clamp(0.25, 2.9);
    }
    if response.dragged_by(egui::PointerButton::Secondary) || response.dragged_by(egui::PointerButton::Middle) {
        cam_st.3 += egui::vec2(-drag.x, drag.y) * cam_st.2 * 0.0035;
    }
    if response.hovered() {
        let sc = ui.input_mut(|i| {
            let s = i.smooth_scroll_delta.y;
            if s != 0.0 {
                i.smooth_scroll_delta = egui::Vec2::ZERO;
            }
            s
        });
        if sc != 0.0 {
            cam_st.2 = (cam_st.2 * (-sc * 0.0015).exp()).clamp(1.3, 14.0);
        }
    }
    ui.data_mut(|d| d.insert_temp(response.id, cam_st));

    // body frame -> NED via quaternion
    let rot = |px: f64, py: f64, pz: f64| -> [f64; 3] {
        let (w, x, y, z) = (qw, qx, qy, qz);
        [
            (1.0 - 2.0 * (y * y + z * z)) * px + 2.0 * (x * y - w * z) * py + 2.0 * (x * z + w * y) * pz,
            2.0 * (x * y + w * z) * px + (1.0 - 2.0 * (x * x + z * z)) * py + 2.0 * (y * z - w * x) * pz,
            2.0 * (x * z - w * y) * px + 2.0 * (y * z + w * x) * py + (1.0 - 2.0 * (x * x + y * y)) * pz,
        ]
    };

    // perspective orbit camera orbiting the Up axis (-Z in NED) so U faces screen-up.
    // az = azimuth in XY plane, pol = angle from -Z (0=top, π/2=equator, π=bottom).
    let (az, pol, dist) = (cam_st.0 as f64, cam_st.1 as f64, cam_st.2 as f64);
    // camera direction from origin (spherical → NED cartesian, pole = -Z)
    let cdir = [pol.sin() * az.cos(), pol.sin() * az.sin(), -pol.cos()];
    let fwd = [-cdir[0], -cdir[1], -cdir[2]];
    // up reference = -Z (Up in NED)
    let up_ref = [0.0_f64, 0.0, -1.0];
    let right = {
        // cross(fwd, up_ref)
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
    // target sits on the origin so the North axis stays exactly vertical on screen under perspective
    let (px, py) = (cam_st.3.x as f64, cam_st.3.y as f64);
    let tgt = [
        right[0] * px + up[0] * py,
        right[1] * px + up[1] * py,
        right[2] * px + up[2] * py,
    ];
    let cam = [tgt[0] + dist * cdir[0], tgt[1] + dist * cdir[1], tgt[2] + dist * cdir[2]];
    let fs = size.y as f64 * 1.15;
    let project = |p: [f64; 3]| -> (egui::Pos2, f64) {
        let d = [p[0] - cam[0], p[1] - cam[1], p[2] - cam[2]];
        let cx = d[0] * right[0] + d[1] * right[1] + d[2] * right[2];
        let cy = d[0] * up[0] + d[1] * up[1] + d[2] * up[2];
        let cz = d[0] * fwd[0] + d[1] * fwd[1] + d[2] * fwd[2];
        let k = fs / cz.max(1e-6);
        (egui::pos2(center.x + (cx * k) as f32, center.y - (cy * k) as f32), cz)
    };

    // depth-sorted segment buffer: far drawn first
    let mut segs: Vec<(f64, egui::Pos2, egui::Pos2, egui::Stroke)> = Vec::new();
    let seg = |a: [f64; 3], b: [f64; 3], s: egui::Stroke| {
        let (pa, za) = project(a);
        let (pb, zb) = project(b);
        if za < 0.05 || zb < 0.05 { return None; }
        Some(((za + zb) * 0.5, pa, pb, s))
    };
    let mut labels: Vec<(egui::Pos2, &str, egui::Color32)> = Vec::new();

    // wireframe sphere: three great circles (XY, XZ, YZ planes)
    let ref_stroke = egui::Stroke::new(0.5, ref_color);
    let steps = 48;
    for i in 0..steps {
        let a0 = std::f64::consts::TAU * i as f64 / steps as f64;
        let a1 = std::f64::consts::TAU * (i + 1) as f64 / steps as f64;
        // XY plane (equator / ground ring)
        segs.extend(seg([a0.cos(), a0.sin(), 0.0], [a1.cos(), a1.sin(), 0.0], ref_stroke));
        // XZ plane (vertical N-S)
        segs.extend(seg([a0.cos(), 0.0, a0.sin()], [a1.cos(), 0.0, a1.sin()], ref_stroke));
        // YZ plane (vertical E-W)
        segs.extend(seg([0.0, a0.cos(), a0.sin()], [0.0, a1.cos(), a1.sin()], ref_stroke));
    }

    // three world axes, drawn dim so they read as reference, not as the vehicle triad
    for (dir, color, name) in [
        ([1.45_f64, 0.0, 0.0], label_color, "N"),
        ([0.0, 1.45, 0.0], label_color, "E"),
        ([0.0, 0.0, 1.45], label_color, "D"),
    ] {
        segs.extend(seg([0.0, 0.0, 0.0], dir, egui::Stroke::new(1.2, color.gamma_multiply(0.75))));
        let (tip, z) = project(dir);
        if z >= 0.05 {
            labels.push((tip, name, color));
        }
    }

    // longitudinal axis = body +X (PROTOCOL: pitch is angle of this axis above horizontal)
    let mut tris: Vec<(f64, [egui::Pos2; 3], egui::Color32)> = Vec::new();
    if !show_stl {
        let nose = rot(1.0, 0.0, 0.0);
        let br = 0.12;
        let bt_x = 0.5;
        let bb_x = -0.3;
        let fin_x = -0.45;
        let fin_r = 0.3;
        let ring: [(f64, f64); 8] = std::array::from_fn(|i| {
            let a = std::f64::consts::TAU * i as f64 / 8.0;
            (br * a.cos(), br * a.sin())
        });
        let bt: Vec<_> = ring.iter().map(|&(cy, cz)| rot(bt_x, cy, cz)).collect();
        let bb: Vec<_> = ring.iter().map(|&(cy, cz)| rot(bb_x, cy, cz)).collect();
        let fin_dirs: [(f64, f64); 4] = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        let ft: Vec<_> = fin_dirs.iter().map(|&(dy, dz)| rot(fin_x, fin_r * dy, fin_r * dz)).collect();
        let fin_root: Vec<_> = fin_dirs.iter().map(|&(dy, dz)| rot(bb_x, br * dy, br * dz)).collect();
        let fin_base: Vec<_> = fin_dirs.iter().map(|&(dy, dz)| rot(fin_x, br * dy, br * dz)).collect();

        let body_s = egui::Stroke::new(1.8, body_color);
        let fin_s = egui::Stroke::new(1.5, fin_color);

        for i in 0..8 {
            let j = (i + 1) % 8;
            segs.extend(seg(nose, bt[i], body_s));
            segs.extend(seg(bt[i], bt[j], body_s));
            segs.extend(seg(bb[i], bb[j], body_s));
            segs.extend(seg(bt[i], bb[i], body_s));
        }
        for i in 0..4 {
            segs.extend(seg(fin_root[i], ft[i], fin_s));
            segs.extend(seg(ft[i], fin_base[i], fin_s));
            segs.extend(seg(fin_base[i], fin_root[i], fin_s));
        }
    } else {
        // STL mesh: flat-shaded with a headlight from upper-left of the camera, back faces culled
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let light = {
            let l = [0, 1, 2].map(|k| -fwd[k] + 0.6 * up[k] - 0.4 * right[k]);
            let n = dot(l, l).sqrt();
            l.map(|v| v / n)
        };
        for t in rocket_mesh() {
            let [a, b, c] = t.map(|p| rot(p[0], p[1], p[2]));
            let (e1, e2) = (sub(b, a), sub(c, a));
            let nrm = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
            if dot(nrm, sub(cam, a)) <= 0.0 { continue; }
            let ((pa, za), (pb, zb), (pc, zc)) = (project(a), project(b), project(c));
            if za < 0.05 || zb < 0.05 || zc < 0.05 { continue; }
            let shade = 0.35 + 0.65 * (dot(nrm, light) / dot(nrm, nrm).sqrt().max(1e-12)).max(0.0);
            let g = (body_rgb * shade) as u8;
            tris.push(((za + zb + zc) / 3.0, [pa, pb, pc], egui::Color32::from_rgb(g, g, g)));
        }
    }

    // body-frame triad: X runs along the rocket (longitudinal/roll axis), Y right, Z down.
    // Drawn over the mesh so attitude stays readable.
    let mut top = Vec::new();
    for (v, color, name) in [
        ([1.25_f64, 0.0, 0.0], ax_n, "X"),
        ([0.0, 0.55, 0.0], ax_e, "Y"),
        ([0.0, 0.0, 0.55], ax_u, "Z"),
    ] {
        let tip3 = rot(v[0], v[1], v[2]);
        top.extend(seg([0.0, 0.0, 0.0], tip3, egui::Stroke::new(2.2, color)));
        let (tip, z) = project(tip3);
        if z >= 0.05 {
            labels.push((tip, name, color));
        }
    }

    // painter's algorithm: merge depth-sorted segments and triangles, far first
    segs.sort_by(|a, b| b.0.total_cmp(&a.0));
    tris.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut tri_it = tris.iter().peekable();
    let mut mesh = egui::Mesh::default();
    let fill = |mesh: &mut egui::Mesh, (_, p, c): &(f64, [egui::Pos2; 3], egui::Color32)| {
        let i = mesh.vertices.len() as u32;
        p.iter().for_each(|q| mesh.colored_vertex(*q, *c));
        mesh.add_triangle(i, i + 1, i + 2);
    };
    for (d, a, b, s) in &segs {
        while let Some(t) = tri_it.next_if(|t| t.0 >= *d) {
            fill(&mut mesh, t);
        }
        if !mesh.is_empty() {
            painter.add(std::mem::take(&mut mesh));
        }
        painter.line_segment([*a, *b], *s);
    }
    tri_it.for_each(|t| fill(&mut mesh, t));
    painter.add(mesh);
    for (_, a, b, s) in &top {
        painter.line_segment([*a, *b], *s);
    }
    let lf = egui::FontId::new(10.0, egui::FontFamily::Monospace);
    for (pos, name, color) in labels {
        painter.text(pos, egui::Align2::LEFT_BOTTOM, name, lf.clone(), color);
    }

    // euler angles: pitch = nose above horizontal, yaw = heading, roll = body-X spin
    // Computed from rotation matrix columns (body axes in NED)
    let bx = rot(1.0, 0.0, 0.0); // body-X (nose) in NED
    let by = rot(0.0, 1.0, 0.0); // body-Y in NED
    let bz = rot(0.0, 0.0, 1.0); // body-Z in NED
    // pitch: angle of nose above horizontal. In NED, -Z is up.
    let pitch_deg = (-bx[2]).atan2((bx[0] * bx[0] + bx[1] * bx[1]).sqrt()).to_degrees();
    // yaw: heading of nose projected onto horizontal plane
    let yaw_deg = bx[1].atan2(bx[0]).to_degrees();
    // roll: rotation about body-X. atan2 of body-Y down vs body-Z down.
    let roll = (-by[2]).atan2(-bz[2]).to_degrees();

    let text_c = if dark_mode { egui::Color32::from_rgb(130, 130, 130) } else { egui::Color32::from_rgb(90, 90, 90) };
    let vf = egui::FontId::new(11.0, egui::FontFamily::Monospace);
    let rect = response.rect;
    let m = 6.0;
    painter.text(
        egui::pos2(rect.left() + m, rect.top() + m),
        egui::Align2::LEFT_TOP,
        format!("R {:+7.1}\u{00b0}  P {:+7.1}\u{00b0}  Y {:+7.1}\u{00b0}", roll, pitch_deg, yaw_deg),
        vf,
        text_c,
    );

    let btn_rect = egui::Rect::from_min_size(egui::pos2(rect.right() - m - 48.0, rect.top() + m), egui::vec2(48.0, 18.0));
    if ui.put(btn_rect, egui::Button::new("STL").selected(show_stl)).clicked() {
        ui.data_mut(|d| d.insert_temp(stl_id, !show_stl));
    }
}

/// Rocket triangles in body frame (+X = nose), parsed once from the embedded STL.
fn rocket_mesh() -> &'static [[[f64; 3]; 3]] {
    static MESH: std::sync::OnceLock<Vec<[[f64; 3]; 3]>> = std::sync::OnceLock::new();
    MESH.get_or_init(|| parse_stl(include_bytes!("../../assets/chicken.stl")))
}

/// Binary STL -> triangles. STL +Z (long axis) maps to body +X spanning -0.6..1.0,
/// centered on the mean vertex XY so the roll axis runs through the body.
fn parse_stl(b: &[u8]) -> Vec<[[f64; 3]; 3]> {
    let tris: Vec<[[f64; 3]; 3]> = b.get(84..).unwrap_or(&[]).chunks_exact(50)
        .map(|c| {
            let f = |o: usize| f32::from_le_bytes([c[o], c[o + 1], c[o + 2], c[o + 3]]) as f64;
            std::array::from_fn(|v| std::array::from_fn(|k| f(12 + v * 12 + k * 4)))
        })
        .collect();
    let n = (tris.len() * 3).max(1) as f64;
    let (mut cx, mut cy, mut z0, mut z1) = (0.0, 0.0, f64::MAX, f64::MIN);
    for p in tris.iter().flatten() {
        cx += p[0] / n;
        cy += p[1] / n;
        z0 = z0.min(p[2]);
        z1 = z1.max(p[2]);
    }
    let s = 1.6 / (z1 - z0);
    tris.iter().map(|t| t.map(|p| [(p[2] - z0) * s - 0.6, (p[0] - cx) * s, (p[1] - cy) * s])).collect()
}

pub enum TimelineClick {
    Segment { start: f64, end: f64 },
    Point(f64),
}

pub fn state_timeline_chart(
    ui: &mut egui::Ui,
    segments: &[StateSegment],
    markers: &[TimelineMarker],
    selected_t: Option<f64>,
    zoom_x: Option<(f64, f64)>,
    link_axes: bool,
    reset: bool,
) -> Option<TimelineClick> {
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

    let mut timeline_plot = Plot::new("sd_state_timeline")
        .height(80.0)
        .allow_drag(true)
        .allow_zoom(true)
        .allow_scroll(false)
        .show_axes([true, false])
        .y_axis_label("")
        .include_y(-1.0)
        .include_y(1.0)
        .show_crosshair(false);
    if link_axes {
        timeline_plot = timeline_plot.link_axis(egui::Id::new(LINK_GROUP), [true, false]);
    }
    if reset {
        timeline_plot = timeline_plot.reset();
    }

    let plot_response = timeline_plot
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            for seg in segments {
                if x >= seg.start && x <= seg.end {
                    let dur = seg.end - seg.start;
                    return Some(format!("{}\n{:.1}s\nClick to zoom", seg.label(), dur));
                }
            }
            None
        })
        .show(ui, |plot_ui| {
            if let Some((lo, hi)) = zoom_x {
                plot_ui.set_plot_bounds_x(lo..=hi);
            }
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
            if let Some(vline) = cursor_line(plot_ui.ctx(), selected_t) {
                plot_ui.vline(vline);
            }
        });

    if let Some(t) = handle_click(&plot_response.response, &plot_response.transform) {
        for seg in segments {
            if t >= seg.start && t <= seg.end {
                return Some(TimelineClick::Segment { start: seg.start, end: seg.end });
            }
        }
        return Some(TimelineClick::Point(t));
    }
    None
}

pub fn altitude_chart(
    ui: &mut egui::Ui,
    timestamps: &[f64],
    gps_alt: &[f64],
    baro_alt: &[f64],
    baro_alt_raw: &[f64],
    max_points: usize,
    selected_t: Option<f64>,
    zoom_x: Option<(f64, f64)>,
    link_axes: bool,
    reset: bool,
) -> Option<f64> {
    let pts_gps: PlotPoints = series_points(timestamps, gps_alt, max_points).into();
    let pts_baro: PlotPoints = series_points(timestamps, baro_alt, max_points).into();
    let pts_raw: PlotPoints = series_points(timestamps, baro_alt_raw, max_points).into();
    let ts = timestamps;
    let gv = gps_alt;
    let bv = baro_alt;
    let rv = baro_alt_raw;
    let plot_response = sd_plot("sd_altitude", link_axes, reset)
        .legend(Legend::default().position(Corner::LeftTop))
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            let mut lines = Vec::new();
            if let Some(val) = lookup_by_timestamp(ts, gv, x) {
                lines.push(format!("GPS: {:.2} m", val));
            }
            if let Some(val) = lookup_by_timestamp(ts, rv, x) {
                lines.push(format!("Baro Raw: {:.2} m", val));
            }
            if let Some(val) = lookup_by_timestamp(ts, bv, x) {
                lines.push(format!("Baro Filtered: {:.2} m", val));
            }
            if lines.is_empty() { None } else { Some(lines.join("\n")) }
        })
        .show(ui, |plot_ui| {
            if let Some((lo, hi)) = zoom_x {
                plot_ui.set_plot_bounds_x(lo..=hi);
            }
            plot_ui.line(Line::new("GPS", pts_gps));
            plot_ui.line(Line::new("Baro Raw", pts_raw));
            plot_ui.line(Line::new("Baro Filtered", pts_baro));
            if let Some(vline) = cursor_line(plot_ui.ctx(), selected_t) {
                plot_ui.vline(vline);
            }
        });
    handle_click(&plot_response.response, &plot_response.transform)
}

pub fn gap_timeline_chart(
    ui: &mut egui::Ui,
    gaps: &[GapInfo],
    total_duration: f64,
    selected_t: Option<f64>,
    zoom_x: Option<(f64, f64)>,
    link_axes: bool,
    reset: bool,
) -> Option<f64> {
    let gap_slice = gaps;
    let mut gap_plot = Plot::new("sd_gap_timeline")
        .height(50.0)
        .allow_drag(true)
        .allow_zoom(true)
        .allow_scroll(false)
        .show_axes([true, false])
        .y_axis_label("")
        .include_y(-0.5)
        .include_y(0.5)
        .include_x(0.0)
        .include_x(total_duration)
        .show_crosshair(false);
    if link_axes {
        gap_plot = gap_plot.link_axis(egui::Id::new(LINK_GROUP), [true, false]);
    }
    if reset {
        gap_plot = gap_plot.reset();
    }

    let gap_color = egui::Color32::from_rgb(255, 60, 60);

    let plot_response = gap_plot
        .label_formatter(move |hover| {
            let x = hover_x(hover);
            for g in gap_slice {
                if x >= g.start && x <= g.end {
                    let duration_ms = (g.end - g.start) * 1000.0;
                    return Some(format!(
                        "GAP: {:.1}ms\n~{} samples dropped\nΔ {} ticks (expected {})",
                        duration_ms, g.dropped, g.delta_ticks, g.expected_ticks
                    ));
                }
            }
            None
        })
        .show(ui, |plot_ui| {
            if let Some((lo, hi)) = zoom_x {
                plot_ui.set_plot_bounds_x(lo..=hi);
            }
            let bars: Vec<Bar> = gaps.iter().map(|g| {
                let duration = g.end - g.start;
                Bar::new(0.0, duration)
                    .base_offset(g.start)
                    .fill(gap_color)
                    .stroke(egui::Stroke::new(0.5, gap_color))
                    .name("GAP")
            }).collect();
            if !bars.is_empty() {
                plot_ui.bar_chart(
                    BarChart::new("GAPS".to_string(), bars)
                        .horizontal()
                        .width(0.8)
                        .color(gap_color),
                );
            }
            for (i, g) in gaps.iter().enumerate() {
                let mid = (g.start + g.end) / 2.0;
                let text = egui::RichText::new(format!(" ~{} ", g.dropped))
                    .size(10.0)
                    .color(egui::Color32::WHITE)
                    .background_color(gap_color);
                plot_ui.text(
                    egui_plot::Text::new(
                        format!("gap_{}", i),
                        egui_plot::PlotPoint::new(mid, 0.0),
                        text,
                    )
                    .anchor(Align2::CENTER_CENTER),
                );
            }
            if let Some(vline) = cursor_line(plot_ui.ctx(), selected_t) {
                plot_ui.vline(vline);
            }
        });
    handle_click(&plot_response.response, &plot_response.transform)
}

#[cfg(test)]
mod tests {
    #[test]
    fn rocket_mesh_spans_body_x() {
        let m = super::rocket_mesh();
        assert_eq!(m.len(), 4744);
        let xs = m.iter().flatten().map(|p| p[0]);
        let (lo, hi) = xs.fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        assert!((lo + 0.6).abs() < 1e-9 && (hi - 1.0).abs() < 1e-9);
    }
}
