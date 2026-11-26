//! A small line-icon set, drawn as vectors so it stays crisp at any size and in exports,
//! plus the buttons and menu items that use it.

use egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ic {
    Play,
    Pause,
    Replay,
    Pin,
    Undo,
    Redo,
    New,
    Open,
    Save,
    Import,
    Export,
    Image,
    Copy,
    Paste,
    Duplicate,
    Trash,
    Settings,
    Search,
    Help,
    Plus,
    Minus,
    Fit,
    Graph,
    Table,
    Dashboard,
    Diagram,
    Loop,
    Target,
    Module,
    Group,
    Ungroup,
    Sun,
    Moon,
    Close,
    Back,
    Keyboard,
    Sliders,
    Download,
    Warning,
    Check,
    Trace,
    History,
    Csv,
    Align,
    Grid,
    Scenarios,
    Mail,
    PanelRight,
    PanelBottom,
    ChevronRight,
    ChevronLeft,
    ChevronDown,
    ChevronUp,
}

/// Draw icon `ic` inside `r` (square works best).
pub fn paint(p: &Painter, r: Rect, ic: Ic, color: Color32) {
    let s = r.width().min(r.height());
    let o = r.center() - vec2(s, s) / 2.0;
    let at = |x: f32, y: f32| -> Pos2 { o + vec2(x, y) * s };
    let w = (s * 0.085).max(1.2);
    let st = Stroke::new(w, color);
    let line = |pts: &[(f32, f32)]| {
        p.add(Shape::line(pts.iter().map(|(x, y)| at(*x, *y)).collect(), st));
    };
    let closed = |pts: &[(f32, f32)]| {
        p.add(Shape::closed_line(pts.iter().map(|(x, y)| at(*x, *y)).collect(), st));
    };
    let fill = |pts: &[(f32, f32)]| {
        p.add(Shape::convex_polygon(pts.iter().map(|(x, y)| at(*x, *y)).collect(), color, Stroke::NONE));
    };
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32, rad: f32| {
        p.rect_stroke(Rect::from_min_max(at(x0, y0), at(x1, y1)), CornerRadius::same((rad * s) as u8), st, StrokeKind::Middle);
    };
    let circle = |x: f32, y: f32, rad: f32| {
        p.circle_stroke(at(x, y), rad * s, st);
    };
    let dot = |x: f32, y: f32, rad: f32| {
        p.circle_filled(at(x, y), rad * s, color);
    };
    let arc = |cx: f32, cy: f32, rad: f32, a0: f32, a1: f32| -> Vec<Pos2> {
        (0..=24).map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / 24.0;
            at(cx + rad * a.cos(), cy + rad * a.sin())
        }).collect()
    };
    let head = |tip: (f32, f32), from: (f32, f32), size: f32| {
        let d = (at(tip.0, tip.1) - at(from.0, from.1)).normalized();
        let n = vec2(-d.y, d.x);
        let t = at(tip.0, tip.1);
        p.add(Shape::line(vec![t - d * size * s + n * size * s * 0.8, t, t - d * size * s - n * size * s * 0.8], st));
    };
    use std::f32::consts::PI;
    match ic {
        Ic::Play => fill(&[(0.3, 0.2), (0.8, 0.5), (0.3, 0.8)]),
        Ic::Pause => {
            p.rect_filled(Rect::from_min_max(at(0.28, 0.2), at(0.42, 0.8)), CornerRadius::same(1), color);
            p.rect_filled(Rect::from_min_max(at(0.58, 0.2), at(0.72, 0.8)), CornerRadius::same(1), color);
        }
        Ic::Replay => {
            p.add(Shape::line(arc(0.5, 0.52, 0.3, -PI * 0.9, PI * 0.75), st));
            head((0.22, 0.38), (0.24, 0.6), 0.14);
            fill(&[(0.18, 0.2), (0.32, 0.36), (0.14, 0.42)]);
        }
        Ic::PanelRight => {
            p.rect_filled(Rect::from_min_max(at(0.62, 0.22), at(0.86, 0.78)), CornerRadius::same(1), color.gamma_multiply(0.35));
            rect(0.14, 0.2, 0.86, 0.8, 0.1);
            line(&[(0.62, 0.2), (0.62, 0.8)]);
        }
        Ic::PanelBottom => {
            p.rect_filled(Rect::from_min_max(at(0.16, 0.58), at(0.84, 0.78)), CornerRadius::same(1), color.gamma_multiply(0.35));
            rect(0.14, 0.2, 0.86, 0.8, 0.1);
            line(&[(0.14, 0.58), (0.86, 0.58)]);
        }
        Ic::ChevronRight => line(&[(0.38, 0.24), (0.66, 0.5), (0.38, 0.76)]),
        Ic::ChevronLeft => line(&[(0.62, 0.24), (0.34, 0.5), (0.62, 0.76)]),
        Ic::ChevronDown => line(&[(0.24, 0.38), (0.5, 0.66), (0.76, 0.38)]),
        Ic::ChevronUp => line(&[(0.24, 0.62), (0.5, 0.34), (0.76, 0.62)]),
        Ic::Pin => {
            line(&[(0.5, 0.62), (0.5, 0.9)]);
            closed(&[(0.32, 0.15), (0.68, 0.15), (0.62, 0.42), (0.75, 0.62), (0.25, 0.62), (0.38, 0.42)]);
        }
        Ic::Undo | Ic::Redo => {
            let flip = if ic == Ic::Redo { -1.0 } else { 1.0 };
            let fx = |x: f32| if flip < 0.0 { 1.0 - x } else { x };
            let mut pts: Vec<(f32, f32)> = vec![(fx(0.22), 0.42)];
            for i in 0..=16 {
                let a = -PI / 2.0 + PI * i as f32 / 16.0;
                pts.push((fx(0.55 + 0.22 * a.cos()), 0.64 + 0.22 * a.sin()));
            }
            pts.push((fx(0.4), 0.86));
            pts[0] = (fx(0.22), 0.42);
            line(&[(fx(0.22), 0.42), (fx(0.55), 0.42)]);
            p.add(Shape::line(pts[1..].iter().map(|(x, y)| at(*x, *y)).collect(), st));
            line(&[(fx(0.36), 0.26), (fx(0.2), 0.42), (fx(0.36), 0.58)]);
        }
        Ic::New => {
            closed(&[(0.22, 0.1), (0.6, 0.1), (0.8, 0.3), (0.8, 0.9), (0.22, 0.9)]);
            line(&[(0.6, 0.1), (0.6, 0.3), (0.8, 0.3)]);
            line(&[(0.51, 0.48), (0.51, 0.74)]);
            line(&[(0.38, 0.61), (0.64, 0.61)]);
        }
        Ic::Open => {
            closed(&[(0.1, 0.22), (0.38, 0.22), (0.46, 0.32), (0.88, 0.32), (0.88, 0.8), (0.1, 0.8)]);
            line(&[(0.1, 0.44), (0.88, 0.44)]);
        }
        Ic::Save => {
            closed(&[(0.15, 0.15), (0.72, 0.15), (0.85, 0.28), (0.85, 0.85), (0.15, 0.85)]);
            rect(0.3, 0.15, 0.62, 0.36, 0.0);
            rect(0.3, 0.56, 0.7, 0.85, 0.0);
        }
        Ic::Import | Ic::Export | Ic::Download => {
            line(&[(0.15, 0.6), (0.15, 0.85), (0.85, 0.85), (0.85, 0.6)]);
            if ic == Ic::Export {
                line(&[(0.5, 0.65), (0.5, 0.12)]);
                head((0.5, 0.12), (0.5, 0.65), 0.17);
            } else {
                line(&[(0.5, 0.12), (0.5, 0.65)]);
                head((0.5, 0.65), (0.5, 0.12), 0.17);
            }
        }
        Ic::Image => {
            rect(0.12, 0.18, 0.88, 0.82, 0.08);
            dot(0.34, 0.38, 0.07);
            line(&[(0.14, 0.76), (0.42, 0.52), (0.58, 0.66), (0.7, 0.56), (0.86, 0.72)]);
        }
        Ic::Copy | Ic::Duplicate => {
            rect(0.32, 0.32, 0.86, 0.86, 0.1);
            line(&[(0.68, 0.32), (0.68, 0.14), (0.14, 0.14), (0.14, 0.68), (0.32, 0.68)]);
            if ic == Ic::Duplicate {
                line(&[(0.59, 0.46), (0.59, 0.72)]);
                line(&[(0.46, 0.59), (0.72, 0.59)]);
            }
        }
        Ic::Paste => {
            rect(0.2, 0.18, 0.8, 0.88, 0.08);
            rect(0.36, 0.1, 0.64, 0.26, 0.05);
        }
        Ic::Trash => {
            line(&[(0.14, 0.26), (0.86, 0.26)]);
            line(&[(0.38, 0.26), (0.38, 0.14), (0.62, 0.14), (0.62, 0.26)]);
            closed(&[(0.22, 0.26), (0.78, 0.26), (0.72, 0.88), (0.28, 0.88)]);
            line(&[(0.42, 0.42), (0.42, 0.72)]);
            line(&[(0.58, 0.42), (0.58, 0.72)]);
        }
        Ic::Settings => {
            circle(0.5, 0.5, 0.13);
            for i in 0..8 {
                let a = i as f32 * PI / 4.0;
                let (c, sn) = (a.cos(), a.sin());
                line(&[(0.5 + 0.24 * c, 0.5 + 0.24 * sn), (0.5 + 0.38 * c, 0.5 + 0.38 * sn)]);
            }
            circle(0.5, 0.5, 0.26);
        }
        Ic::Search => {
            circle(0.43, 0.43, 0.25);
            line(&[(0.62, 0.62), (0.86, 0.86)]);
        }
        Ic::Help => {
            circle(0.5, 0.5, 0.38);
            p.add(Shape::line(arc(0.5, 0.42, 0.12, -PI, PI * 0.4), st));
            line(&[(0.54, 0.53), (0.5, 0.58), (0.5, 0.62)]);
            dot(0.5, 0.72, 0.035);
        }
        Ic::Plus => {
            line(&[(0.5, 0.2), (0.5, 0.8)]);
            line(&[(0.2, 0.5), (0.8, 0.5)]);
        }
        Ic::Minus => line(&[(0.2, 0.5), (0.8, 0.5)]),
        Ic::Fit => {
            for (x, y, dx, dy) in [(0.15, 0.15, 1.0, 1.0), (0.85, 0.15, -1.0, 1.0), (0.15, 0.85, 1.0, -1.0), (0.85, 0.85, -1.0, -1.0)] {
                line(&[(x + dx * 0.22, y), (x, y), (x, y + dy * 0.22)]);
            }
        }
        Ic::Graph => {
            line(&[(0.14, 0.14), (0.14, 0.86), (0.86, 0.86)]);
            line(&[(0.26, 0.68), (0.44, 0.44), (0.6, 0.56), (0.84, 0.24)]);
        }
        Ic::Table => {
            rect(0.12, 0.16, 0.88, 0.84, 0.08);
            line(&[(0.12, 0.38), (0.88, 0.38)]);
            line(&[(0.12, 0.61), (0.88, 0.61)]);
            line(&[(0.4, 0.16), (0.4, 0.84)]);
        }
        Ic::Dashboard => {
            rect(0.12, 0.12, 0.46, 0.52, 0.06);
            rect(0.54, 0.12, 0.88, 0.36, 0.06);
            rect(0.54, 0.44, 0.88, 0.88, 0.06);
            rect(0.12, 0.6, 0.46, 0.88, 0.06);
        }
        Ic::Diagram => {
            rect(0.08, 0.36, 0.38, 0.64, 0.06);
            rect(0.62, 0.36, 0.92, 0.64, 0.06);
            line(&[(0.38, 0.5), (0.62, 0.5)]);
            fill(&[(0.5, 0.5), (0.44, 0.42), (0.56, 0.42)]);
            fill(&[(0.5, 0.5), (0.44, 0.58), (0.56, 0.58)]);
        }
        Ic::Loop => {
            p.add(Shape::line(arc(0.5, 0.5, 0.32, -PI * 0.15, PI * 1.55), st));
            head((0.5 + 0.32 * (PI * 1.55).cos(), 0.5 + 0.32 * (PI * 1.55).sin()), (0.5 + 0.32 * (PI * 1.4).cos(), 0.5 + 0.32 * (PI * 1.4).sin()), 0.14);
            p.text(at(0.5, 0.52), Align2::CENTER_CENTER, "R", FontId::proportional(s * 0.36), color);
        }
        Ic::Target => {
            circle(0.5, 0.5, 0.36);
            circle(0.5, 0.5, 0.2);
            dot(0.5, 0.5, 0.06);
        }
        Ic::Module => {
            rect(0.28, 0.12, 0.88, 0.62, 0.08);
            line(&[(0.2, 0.3), (0.12, 0.3), (0.12, 0.88), (0.7, 0.88), (0.7, 0.72)]);
            dot(0.28, 0.3, 0.05);
            dot(0.88, 0.45, 0.05);
        }
        Ic::Group | Ic::Ungroup => {
            let dashes = [(0.1, 0.1, 0.3, 0.1), (0.42, 0.1, 0.58, 0.1), (0.7, 0.1, 0.9, 0.1), (0.1, 0.9, 0.3, 0.9), (0.42, 0.9, 0.58, 0.9), (0.7, 0.9, 0.9, 0.9), (0.1, 0.1, 0.1, 0.3), (0.1, 0.42, 0.1, 0.58), (0.1, 0.7, 0.1, 0.9), (0.9, 0.1, 0.9, 0.3), (0.9, 0.42, 0.9, 0.58), (0.9, 0.7, 0.9, 0.9)];
            if ic == Ic::Group {
                for (a, b, c, d) in dashes {
                    line(&[(a, b), (c, d)]);
                }
            } else {
                for (a, b, c, d) in dashes.iter().step_by(2) {
                    line(&[(*a, *b), (*c, *d)]);
                }
            }
            rect(0.26, 0.26, 0.52, 0.52, 0.04);
            rect(0.48, 0.48, 0.74, 0.74, 0.04);
        }
        Ic::Sun => {
            circle(0.5, 0.5, 0.17);
            for i in 0..8 {
                let a = i as f32 * PI / 4.0;
                line(&[(0.5 + 0.28 * a.cos(), 0.5 + 0.28 * a.sin()), (0.5 + 0.4 * a.cos(), 0.5 + 0.4 * a.sin())]);
            }
        }
        Ic::Moon => {
            let mut pts = arc(0.5, 0.5, 0.36, PI * 0.3, PI * 1.75);
            pts.extend(arc(0.68, 0.38, 0.27, PI * 1.6, PI * 0.45).into_iter().skip(1));
            p.add(Shape::line(pts, st));
        }
        Ic::Close => {
            line(&[(0.24, 0.24), (0.76, 0.76)]);
            line(&[(0.76, 0.24), (0.24, 0.76)]);
        }
        Ic::Back => line(&[(0.62, 0.2), (0.32, 0.5), (0.62, 0.8)]),
        Ic::Keyboard => {
            rect(0.08, 0.24, 0.92, 0.76, 0.08);
            for x in [0.24, 0.4, 0.56, 0.72] {
                dot(x, 0.4, 0.035);
            }
            line(&[(0.3, 0.6), (0.7, 0.6)]);
        }
        Ic::Sliders => {
            for (y, x) in [(0.25, 0.65), (0.5, 0.35), (0.75, 0.58)] {
                line(&[(0.12, y), (0.88, y)]);
                p.circle(at(x, y), 0.08 * s, color, Stroke::NONE);
            }
        }
        Ic::Warning => {
            closed(&[(0.5, 0.12), (0.9, 0.84), (0.1, 0.84)]);
            line(&[(0.5, 0.38), (0.5, 0.6)]);
            dot(0.5, 0.72, 0.035);
        }
        Ic::Mail => {
            rect(0.1, 0.22, 0.9, 0.78, 0.08);
            line(&[(0.12, 0.26), (0.5, 0.54), (0.88, 0.26)]);
        }
        Ic::Check => line(&[(0.18, 0.52), (0.4, 0.74), (0.84, 0.28)]),
        Ic::Trace => {
            circle(0.2, 0.5, 0.1);
            circle(0.8, 0.22, 0.1);
            circle(0.8, 0.78, 0.1);
            line(&[(0.3, 0.46), (0.7, 0.26)]);
            line(&[(0.3, 0.54), (0.7, 0.74)]);
        }
        Ic::History => {
            p.add(Shape::line(arc(0.5, 0.5, 0.36, -PI * 0.75, PI * 1.1), st));
            fill(&[(0.13, 0.18), (0.3, 0.3), (0.12, 0.38)]);
            line(&[(0.5, 0.3), (0.5, 0.52), (0.66, 0.62)]);
        }
        Ic::Csv => {
            closed(&[(0.22, 0.1), (0.6, 0.1), (0.8, 0.3), (0.8, 0.9), (0.22, 0.9)]);
            line(&[(0.34, 0.5), (0.68, 0.5)]);
            line(&[(0.34, 0.64), (0.68, 0.64)]);
            line(&[(0.34, 0.78), (0.6, 0.78)]);
        }
        Ic::Align => {
            line(&[(0.14, 0.1), (0.14, 0.9)]);
            rect(0.24, 0.22, 0.82, 0.42, 0.04);
            rect(0.24, 0.58, 0.6, 0.78, 0.04);
        }
        Ic::Scenarios => {
            // one path branching into three futures
            line(&[(0.1, 0.55), (0.38, 0.55)]);
            p.add(Shape::line((0..=12).map(|i| { let t = i as f32 / 12.0; at(0.38 + 0.5 * t, 0.55 - 0.35 * t * t) }).collect(), st));
            line(&[(0.38, 0.55), (0.88, 0.55)]);
            p.add(Shape::line((0..=12).map(|i| { let t = i as f32 / 12.0; at(0.38 + 0.5 * t, 0.55 + 0.3 * t * t) }).collect(), st));
            dot(0.88, 0.2, 0.06);
            dot(0.88, 0.55, 0.06);
            dot(0.88, 0.85, 0.06);
        }
        Ic::Grid => {
            for v in [0.2, 0.5, 0.8] {
                for u in [0.2, 0.5, 0.8] {
                    dot(u, v, 0.05);
                }
            }
        }
    }
}

/// A flat button with an icon and optional label; highlighted when `active`.
pub fn button(ui: &mut Ui, ic: Ic, label: Option<&str>, active: bool, color: Color32, accent: Color32) -> Response {
    let font = FontId::proportional(13.5);
    let size = 16.0;
    let galley = label.map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), color));
    let pad = vec2(8.0, 5.0);
    let w = size + galley.as_ref().map(|g| g.size().x + 6.0).unwrap_or(0.0) + pad.x * 2.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, size + pad.y * 2.0 + 2.0), Sense::click());
    let vis = ui.visuals();
    let hover = resp.hovered();
    if active {
        ui.painter().rect_filled(r, CornerRadius::same(7), accent.gamma_multiply(if vis.dark_mode { 0.3 } else { 0.13 }));
    } else if hover {
        ui.painter().rect_filled(r, CornerRadius::same(7), vis.widgets.hovered.weak_bg_fill);
    }
    let col = if active { accent } else { color };
    let ir = Rect::from_min_size(pos2(r.left() + pad.x, r.center().y - size / 2.0), Vec2::splat(size));
    paint(ui.painter(), ir, ic, col);
    if let Some(g) = galley {
        ui.painter().galley(pos2(ir.right() + 6.0, r.center().y - g.size().y / 2.0), g, col);
    }
    resp
}

/// Icon-only button with a tooltip.
pub fn tool(ui: &mut Ui, ic: Ic, tip: &str, color: Color32, accent: Color32) -> Response {
    button(ui, ic, None, false, color, accent).on_hover_text(tip)
}

/// A menu entry with an icon, a label and a right-aligned shortcut. Returns true when clicked.
pub fn menu_item(ui: &mut Ui, ic: Ic, label: &str, shortcut: &str, enabled: bool) -> bool {
    let vis = ui.visuals().clone();
    let text_col = if enabled { vis.text_color() } else { vis.weak_text_color() };
    let width = ui.available_width().max(220.0);
    let (r, resp) = ui.allocate_exact_size(vec2(width, 26.0), if enabled { Sense::click() } else { Sense::hover() });
    if resp.hovered() && enabled {
        ui.painter().rect_filled(r, CornerRadius::same(5), vis.widgets.hovered.weak_bg_fill);
    }
    let ir = Rect::from_min_size(pos2(r.left() + 6.0, r.center().y - 8.0), Vec2::splat(16.0));
    paint(ui.painter(), ir, ic, if enabled { vis.text_color().gamma_multiply(0.85) } else { vis.weak_text_color() });
    ui.painter().text(pos2(ir.right() + 10.0, r.center().y), Align2::LEFT_CENTER, label, FontId::proportional(13.5), text_col);
    if !shortcut.is_empty() {
        ui.painter().text(pos2(r.right() - 8.0, r.center().y), Align2::RIGHT_CENTER, shortcut, FontId::proportional(12.0), vis.weak_text_color());
    }
    let clicked = enabled && resp.clicked();
    if clicked {
        ui.close();
    }
    clicked
}
