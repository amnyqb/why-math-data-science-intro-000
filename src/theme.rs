//! Colours, visuals and the little drawn icons used by the toolbar.

use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, Vec2, pos2, vec2};

use crate::model::Kind;

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub canvas: Color32,
    pub grid: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub stock: Color32,
    pub stock_fill: Color32,
    pub flow: Color32,
    pub aux: Color32,
    pub constant: Color32,
    pub lookup: Color32,
    pub link: Color32,
    pub error: Color32,
    pub ok: Color32,
}

impl Palette {
    pub fn light() -> Self {
        Palette {
            dark: false,
            canvas: Color32::from_rgb(248, 249, 252),
            grid: Color32::from_rgb(222, 226, 236),
            panel: Color32::from_rgb(255, 255, 255),
            surface: Color32::from_rgb(244, 245, 250),
            border: Color32::from_rgb(228, 231, 240),
            text: Color32::from_rgb(28, 32, 46),
            muted: Color32::from_rgb(110, 118, 140),
            accent: Color32::from_rgb(79, 70, 229),
            accent_soft: Color32::from_rgb(234, 233, 255),
            stock: Color32::from_rgb(79, 70, 229),
            stock_fill: Color32::from_rgb(241, 241, 255),
            flow: Color32::from_rgb(13, 148, 136),
            aux: Color32::from_rgb(234, 138, 12),
            constant: Color32::from_rgb(22, 163, 74),
            lookup: Color32::from_rgb(219, 39, 119),
            link: Color32::from_rgb(120, 130, 156),
            error: Color32::from_rgb(220, 38, 38),
            ok: Color32::from_rgb(22, 163, 74),
        }
    }
    pub fn dark() -> Self {
        Palette {
            dark: true,
            canvas: Color32::from_rgb(17, 19, 26),
            grid: Color32::from_rgb(38, 42, 54),
            panel: Color32::from_rgb(24, 26, 34),
            surface: Color32::from_rgb(32, 35, 46),
            border: Color32::from_rgb(44, 48, 62),
            text: Color32::from_rgb(230, 233, 242),
            muted: Color32::from_rgb(140, 148, 170),
            accent: Color32::from_rgb(129, 120, 255),
            accent_soft: Color32::from_rgb(45, 42, 86),
            stock: Color32::from_rgb(140, 132, 255),
            stock_fill: Color32::from_rgb(34, 33, 60),
            flow: Color32::from_rgb(45, 212, 191),
            aux: Color32::from_rgb(251, 170, 60),
            constant: Color32::from_rgb(74, 222, 128),
            lookup: Color32::from_rgb(244, 114, 182),
            link: Color32::from_rgb(130, 140, 168),
            error: Color32::from_rgb(248, 113, 113),
            ok: Color32::from_rgb(74, 222, 128),
        }
    }
    /// Set colours by name ("stock", "flow", "link"…) from "#rrggbb" strings; unknown names are ignored.
    pub fn set_colors(&mut self, colors: &std::collections::BTreeMap<String, String>) {
        for (k, v) in colors {
            let Ok([r, g, b]) = dynamist_core::figstyle::parse_hex(v) else { continue };
            let c = Color32::from_rgb(r, g, b);
            match k.as_str() {
                "text" => self.text = c,
                "muted" => self.muted = c,
                "accent" => self.accent = c,
                "accent_soft" => self.accent_soft = c,
                "stock" => self.stock = c,
                "stock_fill" => self.stock_fill = c,
                "flow" => self.flow = c,
                "aux" => self.aux = c,
                "constant" => self.constant = c,
                "lookup" => self.lookup = c,
                "link" => self.link = c,
                "error" => self.error = c,
                "ok" => self.ok = c,
                "panel" => self.panel = c,
                "surface" => self.surface = c,
                "border" => self.border = c,
                "grid" => self.grid = c,
                _ => {}
            }
        }
    }
    pub fn kind(&self, k: Kind) -> Color32 {
        match k {
            Kind::Stock => self.stock,
            Kind::Flow => self.flow,
            Kind::Aux => self.aux,
            Kind::Constant => self.constant,
            Kind::Lookup => self.lookup,
        }
    }
    /// Distinct line colours for graphs.
    pub fn series(&self, i: usize) -> Color32 {
        const C: [(u8, u8, u8); 8] = [
            (79, 70, 229),
            (13, 148, 136),
            (234, 88, 12),
            (219, 39, 119),
            (37, 99, 235),
            (101, 163, 13),
            (147, 51, 234),
            (202, 138, 4),
        ];
        let (r, g, b) = C[i % C.len()];
        if self.dark {
            Color32::from_rgb(
                r.saturating_add(50),
                g.saturating_add(50),
                b.saturating_add(50),
            )
        } else {
            Color32::from_rgb(r, g, b)
        }
    }
}

/// Which look the person chose: always light, always dark, or whatever the system uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum ThemeChoice {
    #[default]
    Light,
    Dark,
    System,
}

impl ThemeChoice {
    /// Whether this choice means dark, given what the system says (if it says anything).
    pub fn is_dark(self, system: Option<egui::Theme>) -> bool {
        match self {
            ThemeChoice::Light => false,
            ThemeChoice::Dark => true,
            ThemeChoice::System => system == Some(egui::Theme::Dark),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::Light => "Light",
            ThemeChoice::Dark => "Dark",
            ThemeChoice::System => "System",
        }
    }
}

pub fn apply(ctx: &egui::Context, p: &Palette) {
    // pin egui's own theme to the palette, so a change in the system setting never swaps in
    // egui's stock visuals behind our back (following the system is done by the app itself)
    ctx.set_theme(if p.dark { egui::Theme::Dark } else { egui::Theme::Light });
    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.extreme_bg_color = p.surface;
    v.faint_bg_color = p.surface;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.selection.bg_fill = p.accent.gamma_multiply(if p.dark { 0.6 } else { 0.35 });
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
        &mut v.widgets.noninteractive,
    ] {
        w.corner_radius = CornerRadius::same(6);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.weak_bg_fill = p.surface;
    v.widgets.inactive.bg_fill = p.surface;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.hovered.weak_bg_fill = p.accent_soft;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.6));
    v.widgets.active.weak_bg_fill = p.accent_soft;
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(10.0, 4.0);
        s.spacing.interact_size.y = 26.0;
    });
}

/// Toolbar icons, drawn rather than loaded from files.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Select,
    Stock,
    Flow,
    Aux,
    Constant,
    Lookup,
    Link,
    Note,
    Delete,
    Loop,
}

pub fn draw_icon(p: &Painter, icon: Icon, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let st = Stroke::new(1.7, color);
    match icon {
        Icon::Select => {
            let pts = vec![
                c + vec2(-0.22, -0.32) * s,
                c + vec2(0.22, 0.06) * s,
                c + vec2(0.02, 0.1) * s,
                c + vec2(-0.08, 0.32) * s,
                c + vec2(-0.16, 0.28) * s,
                c + vec2(-0.06, 0.06) * s,
                c + vec2(-0.22, 0.2) * s,
            ];
            p.add(egui::Shape::convex_polygon(
                pts,
                color.gamma_multiply(0.15),
                st,
            ));
        }
        Icon::Stock => {
            let r = Rect::from_center_size(c, vec2(0.62, 0.4) * s);
            p.rect_filled(r, CornerRadius::same(3), color.gamma_multiply(0.15));
            p.rect_stroke(r, CornerRadius::same(3), st, egui::StrokeKind::Middle);
        }
        Icon::Flow => {
            let a = c + vec2(-0.34, 0.0) * s;
            let b = c + vec2(0.34, 0.0) * s;
            p.line_segment(
                [a + vec2(0.0, -0.06 * s), b + vec2(-0.08 * s, -0.06 * s)],
                st,
            );
            p.line_segment([a + vec2(0.0, 0.06 * s), b + vec2(-0.08 * s, 0.06 * s)], st);
            p.add(egui::Shape::convex_polygon(
                vec![b, b + vec2(-0.12, -0.12) * s, b + vec2(-0.12, 0.12) * s],
                color,
                Stroke::NONE,
            ));
            valve(p, c, 0.13 * s, color, color.gamma_multiply(0.2));
        }
        Icon::Aux => {
            p.circle_stroke(c, 0.2 * s, st);
            p.circle_filled(c, 0.07 * s, color);
        }
        Icon::Constant => {
            let pts = vec![
                c + vec2(0.0, -0.26) * s,
                c + vec2(0.26, 0.0) * s,
                c + vec2(0.0, 0.26) * s,
                c + vec2(-0.26, 0.0) * s,
            ];
            p.add(egui::Shape::convex_polygon(
                pts,
                color.gamma_multiply(0.15),
                st,
            ));
        }
        Icon::Lookup => {
            let r = Rect::from_center_size(c, vec2(0.6, 0.5) * s);
            p.line_segment(
                [r.left_top(), r.left_bottom()],
                Stroke::new(1.2, color.gamma_multiply(0.6)),
            );
            p.line_segment(
                [r.left_bottom(), r.right_bottom()],
                Stroke::new(1.2, color.gamma_multiply(0.6)),
            );
            let pts: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0;
                    let y = 1.0 / (1.0 + (-(t - 0.5) * 10.0).exp());
                    pos2(r.left() + t * r.width(), r.bottom() - y * r.height())
                })
                .collect();
            p.add(egui::Shape::line(pts, st));
        }
        Icon::Link => {
            let a = c + vec2(-0.3, 0.2) * s;
            let b = c + vec2(0.3, -0.16) * s;
            let ctrl = c + vec2(-0.1, -0.3) * s;
            let pts: Vec<Pos2> = (0..=12)
                .map(|i| quad(a, ctrl, b, i as f32 / 12.0))
                .collect();
            p.add(egui::Shape::line(pts, st));
            arrow_head(
                p,
                b,
                (b - quad(a, ctrl, b, 0.85)).normalized(),
                0.16 * s,
                color,
            );
        }
        Icon::Loop => {
            let r = 0.24 * s;
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let t = -1.9 + 5.0 * i as f32 / 24.0;
                    c + vec2(t.cos(), t.sin()) * r
                })
                .collect();
            p.add(egui::Shape::line(pts.clone(), st));
            let tip = *pts.last().unwrap();
            arrow_head(
                p,
                tip,
                (tip - pts[pts.len() - 3]).normalized(),
                0.16 * s,
                color,
            );
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                "R",
                egui::FontId::proportional(0.26 * s),
                color,
            );
        }
        Icon::Note => {
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                "T",
                egui::FontId::proportional(0.5 * s),
                color,
            );
        }
        Icon::Delete => {
            let r = Rect::from_center_size(c + vec2(0.0, 0.04 * s), vec2(0.36, 0.42) * s);
            p.rect_stroke(r, CornerRadius::same(2), st, egui::StrokeKind::Middle);
            p.line_segment(
                [
                    pos2(r.left() - 0.06 * s, r.top() - 0.04 * s),
                    pos2(r.right() + 0.06 * s, r.top() - 0.04 * s),
                ],
                st,
            );
        }
    }
}

pub fn quad(a: Pos2, c: Pos2, b: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    pos2(
        u * u * a.x + 2.0 * u * t * c.x + t * t * b.x,
        u * u * a.y + 2.0 * u * t * c.y + t * t * b.y,
    )
}

/// Points along a circular arc from `a` to `b` that bulges sideways by `sagitta`
/// (positive = to the left of a→b). A tiny sagitta gives a straight line.
pub fn arc_points(a: Pos2, b: Pos2, sagitta: f32, n: usize) -> Vec<Pos2> {
    let chord = b - a;
    let c = chord.length();
    if c < 1e-3 || sagitta.abs() < 0.5 {
        return (0..=n).map(|i| a + chord * (i as f32 / n as f32)).collect();
    }
    let s = sagitta.clamp(-c * 1.2, c * 1.2);
    let dir = chord / c;
    let perp = vec2(-dir.y, dir.x);
    let mid = a + chord * 0.5;
    let r = (c * c / 4.0 + s * s) / (2.0 * s.abs());
    let center = mid + perp * (s - s.signum() * r);
    let ang = |p: Pos2| (p.y - center.y).atan2(p.x - center.x);
    let wrap = |x: f32| {
        let mut x = x % std::f32::consts::TAU;
        if x > std::f32::consts::PI {
            x -= std::f32::consts::TAU;
        }
        if x < -std::f32::consts::PI {
            x += std::f32::consts::TAU;
        }
        x
    };
    let a0 = ang(a);
    let am = ang(mid + perp * s);
    let a1 = ang(b);
    let total = wrap(am - a0) + wrap(a1 - am);
    (0..=n)
        .map(|i| {
            let t = a0 + total * i as f32 / n as f32;
            center + vec2(t.cos(), t.sin()) * r
        })
        .collect()
}

/// A narrow filled arrowhead, as used on causal arrows.
pub fn arrow_head_slim(p: &Painter, tip: Pos2, dir: Vec2, size: f32, color: Color32) {
    let n = vec2(-dir.y, dir.x);
    let base = tip - dir * size;
    let notch = tip - dir * size * 0.78;
    p.add(egui::Shape::convex_polygon(
        vec![tip, base + n * size * 0.36, notch],
        color,
        Stroke::NONE,
    ));
    p.add(egui::Shape::convex_polygon(
        vec![tip, notch, base - n * size * 0.36],
        color,
        Stroke::NONE,
    ));
}

pub fn arrow_head(p: &Painter, tip: Pos2, dir: Vec2, size: f32, color: Color32) {
    let n = vec2(-dir.y, dir.x);
    let base = tip - dir * size;
    p.add(egui::Shape::convex_polygon(
        vec![tip, base + n * size * 0.5, base - n * size * 0.5],
        color,
        Stroke::NONE,
    ));
}

/// The bow-tie valve drawn on flows.
pub fn valve(p: &Painter, c: Pos2, r: f32, color: Color32, fill: Color32) {
    let st = Stroke::new(1.5, color);
    p.add(egui::Shape::convex_polygon(
        vec![c, c + vec2(-r, -r), c + vec2(r, -r)],
        fill,
        st,
    ));
    p.add(egui::Shape::convex_polygon(
        vec![c, c + vec2(r, r), c + vec2(-r, r)],
        fill,
        st,
    ));
}

/// A small cloud: the source or sink outside the model boundary.
pub fn cloud(p: &Painter, c: Pos2, s: f32, color: Color32, fill: Color32) {
    for (dx, dy, r) in [
        (-0.45, 0.1, 0.38),
        (0.0, -0.15, 0.48),
        (0.45, 0.1, 0.38),
        (0.05, 0.2, 0.4),
    ] {
        p.circle(c + vec2(dx, dy) * s, r * s, fill, Stroke::new(1.2, color));
    }
    for (dx, dy, r) in [
        (-0.45, 0.1, 0.38),
        (0.0, -0.15, 0.48),
        (0.45, 0.1, 0.38),
        (0.05, 0.2, 0.4),
    ] {
        p.circle_filled(c + vec2(dx, dy) * s, r * s - 1.2, fill);
    }
}

/// A sparkline of `vals` inside `rect`, with an optional marker at index `mark`.
pub fn sparkline(
    p: &Painter,
    rect: Rect,
    vals: &[f64],
    color: Color32,
    mark: Option<usize>,
    fill: bool,
) {
    sparkline_upto(p, rect, vals, color, mark, fill, vals.len());
}

/// Like [`sparkline`], scaled for the whole series but drawn only up to `upto` points
/// (used while a run is being played in).
pub fn sparkline_upto(
    p: &Painter,
    rect: Rect,
    vals: &[f64],
    color: Color32,
    mark: Option<usize>,
    fill: bool,
    upto: usize,
) {
    if vals.len() < 2 {
        return;
    }
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in vals.iter().filter(|v| v.is_finite()) {
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    if !lo.is_finite() {
        return;
    }
    if (hi - lo).abs() < 1e-12 {
        hi = lo + 1.0;
        lo -= 1.0;
    }
    let n = vals.len();
    let end = upto.clamp(2, n);
    let step = (n / 120).max(1);
    let pt = |i: usize| {
        let v = if vals[i].is_finite() { vals[i] } else { lo };
        pos2(
            rect.left() + rect.width() * i as f32 / (n - 1) as f32,
            rect.bottom() - rect.height() * ((v - lo) / (hi - lo)) as f32,
        )
    };
    let mut pts: Vec<Pos2> = (0..end).step_by(step).map(pt).collect();
    pts.push(pt(end - 1));
    if fill {
        let mut mesh = egui::Mesh::default();
        let fc = color.gamma_multiply(0.14);
        for w in pts.windows(2) {
            let i = mesh.vertices.len() as u32;
            mesh.colored_vertex(w[0], fc);
            mesh.colored_vertex(w[1], fc);
            mesh.colored_vertex(pos2(w[1].x, rect.bottom()), fc);
            mesh.colored_vertex(pos2(w[0].x, rect.bottom()), fc);
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i, i + 2, i + 3);
        }
        p.add(egui::Shape::mesh(mesh));
    }
    p.add(egui::Shape::line(pts, Stroke::new(1.5, color)));
    if let Some(m) = mark {
        if m < n {
            p.circle(pt(m), 3.0, color, Stroke::new(1.5, Color32::WHITE));
        }
    }
}

/// Short human formatting for values shown on the diagram.
pub use dynamist_core::fmt::fmt_num;

/// Icons for the align and distribute buttons: a guide line and two bars.
pub fn draw_align(p: &Painter, r: Rect, op: crate::arrange::Align, color: Color32, guide: Color32) {
    use crate::arrange::Align as A;
    let st = Stroke::new(1.6, guide);
    let bar = |rr: Rect| {
        p.rect_filled(rr, CornerRadius::same(1), color.gamma_multiply(0.85));
    };
    let (w, h) = (r.width(), r.height());
    match op {
        A::Left | A::CenterH | A::Right => {
            let x = match op {
                A::Left => r.left(),
                A::Right => r.right(),
                _ => r.center().x,
            };
            p.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], st);
            for (y, len) in [(r.top() + h * 0.18, w * 0.8), (r.top() + h * 0.58, w * 0.5)] {
                let x0 = match op {
                    A::Left => x + 2.0,
                    A::Right => x - 2.0 - len,
                    _ => x - len / 2.0,
                };
                bar(Rect::from_min_size(pos2(x0, y), vec2(len, h * 0.26)));
            }
        }
        A::Top | A::Middle | A::Bottom => {
            let y = match op {
                A::Top => r.top(),
                A::Bottom => r.bottom(),
                _ => r.center().y,
            };
            p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], st);
            for (x, len) in [
                (r.left() + w * 0.18, h * 0.8),
                (r.left() + w * 0.58, h * 0.5),
            ] {
                let y0 = match op {
                    A::Top => y + 2.0,
                    A::Bottom => y - 2.0 - len,
                    _ => y - len / 2.0,
                };
                bar(Rect::from_min_size(pos2(x, y0), vec2(w * 0.26, len)));
            }
        }
        A::DistributeH => {
            p.line_segment([r.left_top(), r.left_bottom()], st);
            p.line_segment([r.right_top(), r.right_bottom()], st);
            bar(Rect::from_center_size(r.center(), vec2(w * 0.28, h * 0.6)));
        }
        A::DistributeV => {
            p.line_segment([r.left_top(), r.right_top()], st);
            p.line_segment([r.left_bottom(), r.right_bottom()], st);
            bar(Rect::from_center_size(r.center(), vec2(w * 0.6, h * 0.28)));
        }
    }
}

/// A path from `a` to `b` with right angles: out along the longer direction, across, and in.
pub fn elbow_points(a: Pos2, b: Pos2, n: usize) -> Vec<Pos2> {
    let poly = if (b.x - a.x).abs() >= (b.y - a.y).abs() {
        let mx = (a.x + b.x) / 2.0;
        vec![a, pos2(mx, a.y), pos2(mx, b.y), b]
    } else {
        let my = (a.y + b.y) / 2.0;
        vec![a, pos2(a.x, my), pos2(b.x, my), b]
    };
    let total: f32 = poly.windows(2).map(|w| w[0].distance(w[1])).sum();
    if total < 1e-3 {
        return vec![a; n + 1];
    }
    let mut out = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let mut want = total * i as f32 / n as f32;
        let mut pt = b;
        for w in poly.windows(2) {
            let l = w[0].distance(w[1]);
            if want <= l || l < 1e-6 {
                pt = if l < 1e-6 { w[0] } else { w[0] + (w[1] - w[0]) * (want / l) };
                break;
            }
            want -= l;
        }
        out.push(pt);
    }
    out
}

/// The tip of a link arrow in the chosen style.
pub fn link_head(p: &Painter, style: crate::model::ArrowHead, tip: Pos2, dir: Vec2, size: f32, color: Color32) {
    use crate::model::ArrowHead;
    match style {
        ArrowHead::Filled => arrow_head_slim(p, tip, dir, size, color),
        ArrowHead::Open => {
            let n = vec2(-dir.y, dir.x);
            let base = tip - dir * size * 1.1;
            let st = Stroke::new((size * 0.16).max(1.2), color);
            p.line_segment([tip, base + n * size * 0.5], st);
            p.line_segment([tip, base - n * size * 0.5], st);
        }
        ArrowHead::Dot => {
            p.circle_filled(tip - dir * size * 0.35, size * 0.38, color);
        }
    }
}

/// A smooth path through all the given points (a Catmull-Rom spline: no handles to manage,
/// the line simply passes through every turning point). `per` samples per segment.
pub fn smooth_through(pts: &[Pos2], per: usize) -> Vec<Pos2> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let n = pts.len();
    let at = |i: isize| pts[i.clamp(0, n as isize - 1) as usize];
    let mut out = Vec::with_capacity((n - 1) * per + 1);
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (at(i as isize - 1), at(i as isize), at(i as isize + 1), at(i as isize + 2));
        for k in 0..per {
            let t = k as f32 / per as f32;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * ((2.0 * b) + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3);
            out.push(pos2(f(p0.x, p1.x, p2.x, p3.x), f(p0.y, p1.y, p2.y, p3.y)));
        }
    }
    out.push(pts[n - 1]);
    out
}

#[cfg(test)]
mod turn_tests {
    use super::*;
    #[test]
    fn smooth_path_passes_through_every_turning_point() {
        let pts = [pos2(0.0, 0.0), pos2(100.0, 60.0), pos2(200.0, -40.0), pos2(300.0, 0.0)];
        let path = smooth_through(&pts, 20);
        assert_eq!(path.first(), Some(&pts[0]));
        assert_eq!(path.last(), Some(&pts[3]));
        for p in &pts[1..3] {
            assert!(path.iter().any(|q| q.distance(*p) < 1e-3), "path misses {p:?}");
        }
        // no sharp corners: neighbouring steps never jump
        assert!(path.windows(2).all(|w| w[0].distance(w[1]) < 40.0));
    }
    #[test]
    fn arrows_keep_their_turning_points_in_a_saved_model() {
        let mut m = crate::model::Model::example();
        m.links[0].via = vec![[10.0, 20.0], [30.0, 40.0]];
        let back: crate::model::Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.links[0].via, vec![[10.0, 20.0], [30.0, 40.0]]);
        // older files without the field still open
        let old: crate::model::Link = serde_json::from_str(r#"{"id":1,"from":2,"to":3}"#).unwrap();
        assert!(old.via.is_empty());
    }
}
