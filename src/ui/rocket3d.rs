use std::f64::consts::PI;

const BODY_RADIUS: f64 = 0.3;
const BODY_HEIGHT: f64 = 2.0;
const NOSE_HEIGHT: f64 = 0.8;
const FIN_SPAN: f64 = 0.5;
const FIN_HEIGHT: f64 = 0.6;
const SIDES: usize = 12;

struct Vert([f64; 3]);

fn quat_rotate(q: [f64; 4], p: [f64; 3]) -> [f64; 3] {
    let (w, x, y, z) = (q[0], q[1], q[2], q[3]);
    let (px, py, pz) = (p[0], p[1], p[2]);
    let cx = y * pz - z * py;
    let cy = z * px - x * pz;
    let cz = x * py - y * px;
    let cx2 = y * cz - z * cy;
    let cy2 = z * cx - x * cz;
    let cz2 = x * cy - y * cx;
    [
        px + 2.0 * (w * cx + cx2),
        py + 2.0 * (w * cy + cy2),
        pz + 2.0 * (w * cz + cz2),
    ]
}

fn project(p: [f64; 3]) -> (f64, f64) {
    let az: f64 = 30.0 * PI / 180.0;
    let el: f64 = 25.0 * PI / 180.0;
    let (sa, ca) = az.sin_cos();
    let (se, ce) = el.sin_cos();
    let x = p[0] * ca - p[1] * sa;
    let y = -(p[0] * sa * se + p[1] * ca * se) + p[2] * ce;
    (x, y)
}

fn build_rocket() -> (Vec<Vert>, Vec<(usize, usize)>) {
    let mut verts = Vec::new();
    let mut edges = Vec::new();

    for i in 0..SIDES {
        let angle = (i as f64) * 2.0 * PI / (SIDES as f64);
        let (s, c) = angle.sin_cos();
        let x = BODY_RADIUS * c;
        let y = BODY_RADIUS * s;
        verts.push(Vert([x, y, 0.0]));
        verts.push(Vert([x, y, BODY_HEIGHT]));
    }

    for i in 0..SIDES {
        let next = (i + 1) % SIDES;
        let b = i * 2;
        let t = i * 2 + 1;
        let nb = next * 2;
        let nt = next * 2 + 1;
        edges.push((b, nb));
        edges.push((t, nt));
        edges.push((b, t));
    }

    let nose = verts.len();
    verts.push(Vert([0.0, 0.0, BODY_HEIGHT + NOSE_HEIGHT]));
    for i in 0..SIDES {
        let t = i * 2 + 1;
        edges.push((t, nose));
    }

    let fin_angles = [0.0, 2.0 * PI / 3.0, 4.0 * PI / 3.0];
    for &fa in &fin_angles {
        let (s, c) = fa.sin_cos();
        let base_inner = verts.len();
        verts.push(Vert([BODY_RADIUS * c, BODY_RADIUS * s, 0.0]));
        let base_outer = verts.len();
        verts.push(Vert([(BODY_RADIUS + FIN_SPAN) * c, (BODY_RADIUS + FIN_SPAN) * s, 0.0]));
        let tip = verts.len();
        verts.push(Vert([BODY_RADIUS * c, BODY_RADIUS * s, FIN_HEIGHT]));
        edges.push((base_inner, base_outer));
        edges.push((base_outer, tip));
        edges.push((tip, base_inner));
    }

    (verts, edges)
}

pub fn rocket_attitude(
    ui: &mut egui::Ui,
    quat: [f64; 4],
    dark_mode: bool,
) {
    let width = ui.available_width();
    let size = width;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(width, width),
        egui::Sense::hover(),
    );

    let painter = ui.painter_at(rect);
    let center = rect.center();

    let bg = if dark_mode {
        egui::Color32::from_rgb(20, 20, 30)
    } else {
        egui::Color32::from_rgb(230, 230, 240)
    };
    painter.rect_filled(rect, 4.0, bg);

    let q_len = (quat[0] * quat[0] + quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3]).sqrt();
    let q = if q_len > 1e-6 {
        [quat[0] / q_len, quat[1] / q_len, quat[2] / q_len, quat[3] / q_len]
    } else {
        [1.0, 0.0, 0.0, 0.0]
    };

    let (verts, edges) = build_rocket();

    let half_h = (BODY_HEIGHT + NOSE_HEIGHT) / 2.0;
    let scale = (size * 0.35) as f64;

    let projected: Vec<(f64, f64)> = verts
        .iter()
        .map(|v| {
            let centered = [v.0[0], v.0[1], v.0[2] - half_h];
            let rotated = quat_rotate(q, centered);
            let (px, py) = project(rotated);
            (px * scale, py * scale)
        })
        .collect();

    let edge_color = if dark_mode {
        egui::Color32::from_rgb(100, 200, 255)
    } else {
        egui::Color32::from_rgb(30, 100, 180)
    };
    let nose_color = egui::Color32::from_rgb(255, 80, 80);

    let nose_idx = SIDES * 2;

    for &(a, b) in &edges {
        let pa = egui::pos2(
            center.x + projected[a].0 as f32,
            center.y - projected[a].1 as f32,
        );
        let pb = egui::pos2(
            center.x + projected[b].0 as f32,
            center.y - projected[b].1 as f32,
        );
        let color = if a == nose_idx || b == nose_idx {
            nose_color
        } else {
            edge_color
        };
        painter.line_segment([pa, pb], egui::Stroke::new(1.5, color));
    }

    let label_color = if dark_mode {
        egui::Color32::from_rgb(180, 180, 180)
    } else {
        egui::Color32::from_rgb(80, 80, 80)
    };
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        format!("W:{:.2} X:{:.2}\nY:{:.2} Z:{:.2}", q[0], q[1], q[2], q[3]),
        egui::FontId::monospace(10.0),
        label_color,
    );
}
