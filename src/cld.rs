//! Causal loop diagram sheets: words joined by signed, curved arrows, with
//! reinforcing (R) and balancing (B) loop markers and automatic loop finding.

use std::collections::HashMap;

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
    pos2, vec2,
};

use crate::app::App;
use crate::model::{CldData, CldLink, CldLoop, CldNode, Kind, Model};
use crate::theme::{self, Icon};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CldTool {
    #[default]
    Select,
    Variable,
    Link,
    Loop,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CldSel {
    Node(u64),
    Link(u64),
    Loop(u64),
}

#[derive(Clone, Copy, Debug)]
enum CldDrag {
    Pan,
    Move(CldSel),
    Bend(u64),
    NewLink(u64),
}

pub struct CldState {
    pub tool: CldTool,
    pub sel: Option<CldSel>,
    drag: Option<CldDrag>,
    pan: Vec2,
    zoom: f32,
    shown_sheet: u64,
    fit_pending: bool,
    /// Index of the found loop being highlighted.
    pub highlight: Option<usize>,
    pub focus_name: bool,
    rects: HashMap<u64, Rect>,
    paths: HashMap<u64, Vec<Pos2>>,
}

impl Default for CldState {
    fn default() -> Self {
        CldState {
            tool: CldTool::Select,
            sel: None,
            drag: None,
            pan: Vec2::ZERO,
            zoom: 1.0,
            shown_sheet: 0,
            fit_pending: true,
            highlight: None,
            focus_name: false,
            rects: HashMap::new(),
            paths: HashMap::new(),
        }
    }
}

pub use dynamist_core::cld::{FoundLoop, find_loops, from_model};

fn loop_color(app: &App, reinforcing: Option<bool>) -> Color32 {
    match reinforcing {
        Some(true) => app.pal.aux,
        Some(false) => app.pal.accent,
        None => app.pal.muted,
    }
}

impl App {
    fn cld_index(&self) -> Option<usize> {
        self.model.sheets.iter().position(|s| s.id == self.sheet)
    }
    fn cld_data(&self) -> CldData {
        self.cld_index()
            .map(|i| self.model.sheets[i].cld.clone())
            .unwrap_or_default()
    }
    fn cld_mut(&mut self) -> Option<&mut CldData> {
        let i = self.cld_index()?;
        Some(&mut self.model.sheets[i].cld)
    }

    pub fn cld_delete(&mut self) {
        let Some(sel) = self.cld.sel else { return };
        self.checkpoint();
        if let Some(d) = self.cld_mut() {
            match sel {
                CldSel::Node(id) => {
                    d.nodes.retain(|n| n.id != id);
                    d.links.retain(|l| l.from != id && l.to != id);
                }
                CldSel::Link(id) => d.links.retain(|l| l.id != id),
                CldSel::Loop(id) => d.loops.retain(|l| l.id != id),
            }
        }
        self.cld.sel = None;
        self.cld.highlight = None;
    }

    pub fn cld_tools(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            for (t, icon, name, help, col) in [
                (
                    CldTool::Select,
                    Icon::Select,
                    "Select & move",
                    "Click to select, drag to move. Drag empty space to pan.",
                    pal.text,
                ),
                (
                    CldTool::Variable,
                    Icon::Note,
                    "Variable",
                    "Click to add a word or phrase.",
                    pal.text,
                ),
                (
                    CldTool::Link,
                    Icon::Link,
                    "Causal arrow",
                    "Drag from a cause to its effect. New arrows start as +.",
                    pal.link,
                ),
                (
                    CldTool::Loop,
                    Icon::Loop,
                    "Loop marker",
                    "Click inside a loop to label it R (reinforcing) or B (balancing).",
                    pal.aux,
                ),
            ] {
                let (r, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
                let active = self.cld.tool == t;
                if active {
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(9), pal.accent_soft);
                } else if resp.hovered() {
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(9), pal.surface);
                }
                theme::draw_icon(ui.painter(), icon, r.shrink(7.0), col);
                if resp
                    .on_hover_ui(|ui| {
                        ui.label(RichText::new(name).strong());
                        ui.label(help);
                    })
                    .clicked()
                {
                    self.cld.tool = t;
                }
                ui.add_space(2.0);
            }
            ui.add_space(10.0);
            let (r, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
            theme::draw_icon(
                ui.painter(),
                Icon::Delete,
                r.shrink(8.0),
                if self.cld.sel.is_some() {
                    pal.error
                } else {
                    pal.muted.gamma_multiply(0.5)
                },
            );
            if resp.on_hover_text("Delete the selection (⌫)").clicked() {
                self.cld_delete();
            }
        });
    }

    fn cld_to_screen(&self, origin: Pos2, p: Pos2) -> Pos2 {
        origin + (p.to_vec2() + self.cld.pan) * self.cld.zoom
    }
    fn cld_to_diagram(&self, origin: Pos2, p: Pos2) -> Pos2 {
        ((p - origin) / self.cld.zoom - self.cld.pan).to_pos2()
    }

    fn cld_hit(&self, d: &CldData, p: Pos2) -> Option<(CldSel, bool)> {
        if let Some(CldSel::Link(id)) = self.cld.sel {
            if let Some(l) = d.links.iter().find(|l| l.id == id) {
                if let Some(h) = self.cld_handle(d, l) {
                    if h.distance(p) < 10.0 / self.cld.zoom.max(0.5) {
                        return Some((CldSel::Link(id), true));
                    }
                }
            }
        }
        for lp in d.loops.iter().rev() {
            if pos2(lp.pos[0], lp.pos[1]).distance(p) < 22.0 {
                return Some((CldSel::Loop(lp.id), false));
            }
        }
        for n in d.nodes.iter().rev() {
            if self.cld.rects.get(&n.id).is_some_and(|r| r.contains(p)) {
                return Some((CldSel::Node(n.id), false));
            }
        }
        let tol = 6.0 / self.cld.zoom;
        for l in &d.links {
            if let Some(path) = self.cld.paths.get(&l.id) {
                let near = path.windows(2).any(|w| {
                    let ab = w[1] - w[0];
                    let t = if ab.length_sq() > 0.0 {
                        ((p - w[0]).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    (w[0] + ab * t).distance(p) < tol
                });
                if near {
                    return Some((CldSel::Link(l.id), false));
                }
            }
        }
        None
    }

    fn cld_center(&self, d: &CldData, id: u64) -> Option<Pos2> {
        let n = d.nodes.iter().find(|n| n.id == id)?;
        Some(pos2(n.pos[0], n.pos[1]))
    }

    /// The bend handle: the arc's middle.
    fn cld_handle(&self, d: &CldData, l: &CldLink) -> Option<Pos2> {
        let (a, b) = (self.cld_center(d, l.from)?, self.cld_center(d, l.to)?);
        let dir = (b - a).normalized();
        Some(a + (b - a) * 0.5 + vec2(-dir.y, dir.x) * l.bend)
    }

    fn cld_fit(&mut self, d: &CldData, view: Rect) {
        let mut b = Rect::NOTHING;
        for n in &d.nodes {
            b = b.union(
                self.cld
                    .rects
                    .get(&n.id)
                    .copied()
                    .unwrap_or(Rect::from_center_size(
                        pos2(n.pos[0], n.pos[1]),
                        vec2(120.0, 30.0),
                    )),
            );
        }
        for l in &d.loops {
            b = b.union(Rect::from_center_size(
                pos2(l.pos[0], l.pos[1]),
                vec2(50.0, 60.0),
            ));
        }
        if !b.is_positive() {
            self.cld.zoom = 1.0;
            self.cld.pan = view.size() / 2.0;
            return;
        }
        let b = b.expand(50.0);
        let z = (view.width() / b.width())
            .min(view.height() / b.height())
            .clamp(0.3, 1.5);
        self.cld.zoom = z;
        self.cld.pan = (view.size() / z - b.size()) / 2.0 - b.min.to_vec2();
    }

    pub fn cld_canvas(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let rect = ui.max_rect();
        let origin = rect.min;
        let resp = ui.interact(rect, ui.scope_id().with("cld"), Sense::click_and_drag());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, CornerRadius::ZERO, pal.canvas);
        if self.cld.shown_sheet != self.sheet {
            self.cld.shown_sheet = self.sheet;
            self.cld.fit_pending = true;
            self.cld.sel = None;
            self.cld.highlight = None;
            self.cld.rects.clear();
        }
        let d = self.cld_data();
        if self.cld.fit_pending && (!self.cld.rects.is_empty() || d.nodes.is_empty()) {
            self.cld_fit(&d, rect);
            self.cld.fit_pending = false;
        }

        // wheel: pan; pinch / ⌘-wheel: zoom
        if resp.hovered() {
            let (scroll, zd, hover) =
                ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
            if zd != 1.0 {
                if let Some(h) = hover {
                    let before = self.cld_to_diagram(origin, h);
                    self.cld.zoom = (self.cld.zoom * zd).clamp(0.2, 3.0);
                    let after = self.cld_to_diagram(origin, h);
                    self.cld.pan += after - before;
                }
            } else if scroll != Vec2::ZERO {
                self.cld.pan += scroll / self.cld.zoom;
            }
        }

        // dot grid
        let step = 24.0 * self.cld.zoom;
        if step > 6.0 {
            let off = vec2(
                (self.cld.pan.x * self.cld.zoom).rem_euclid(step),
                (self.cld.pan.y * self.cld.zoom).rem_euclid(step),
            );
            let mut y = rect.top() + off.y;
            while y < rect.bottom() {
                let mut x = rect.left() + off.x;
                while x < rect.right() {
                    p.circle_filled(pos2(x, y), 1.0, pal.grid);
                    x += step;
                }
                y += step;
            }
        }

        self.cld_mouse(ui, &resp, origin, &d);
        let d = self.cld_data();
        self.cld_draw(ui, &p, origin, &d);

        if d.nodes.is_empty() {
            p.text(
                rect.center() - vec2(0.0, 20.0),
                Align2::CENTER_CENTER,
                "An empty causal loop diagram",
                FontId::proportional(18.0),
                pal.text,
            );
            p.text(
                rect.center() + vec2(0.0, 10.0),
                Align2::CENTER_CENTER,
                "Pick the T tool and click to add words, or use “Build from model” on the right.",
                FontId::proportional(14.0),
                pal.muted,
            );
        }
        if self.cld.tool != CldTool::Select {
            let hint = match self.cld.tool {
                CldTool::Variable => "Click to add a variable",
                CldTool::Link => "Drag from a cause to its effect",
                _ => "Click inside a loop to mark it",
            };
            p.text(
                rect.left_top() + vec2(16.0, 16.0),
                Align2::LEFT_TOP,
                format!("{hint}  ·  Esc to stop"),
                FontId::proportional(13.0),
                pal.accent,
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.cld.tool = CldTool::Select;
            }
        }
    }

    fn cld_mouse(&mut self, ui: &egui::Ui, resp: &egui::Response, origin: Pos2, d: &CldData) {
        let Some(pp) = ui.input(|i| i.pointer.interact_pos()) else {
            return;
        };
        let pd = self.cld_to_diagram(origin, pp);
        if resp.drag_started() {
            let start =
                self.cld_to_diagram(origin, ui.input(|i| i.pointer.press_origin()).unwrap_or(pp));
            let hit = self.cld_hit(d, start);
            let other_button = ui.input(|i| i.pointer.middle_down() || i.pointer.secondary_down());
            self.cld.drag = match (other_button, self.cld.tool, hit) {
                (true, _, _) => Some(CldDrag::Pan),
                (_, CldTool::Link, Some((CldSel::Node(id), _))) => Some(CldDrag::NewLink(id)),
                (_, _, Some((CldSel::Link(id), true))) => {
                    self.checkpoint();
                    Some(CldDrag::Bend(id))
                }
                (_, _, Some((s @ (CldSel::Node(_) | CldSel::Loop(_)), _))) => {
                    self.checkpoint();
                    self.cld.sel = Some(s);
                    Some(CldDrag::Move(s))
                }
                _ => Some(CldDrag::Pan),
            };
        }
        if resp.dragged() {
            let delta = resp.drag_delta() / self.cld.zoom;
            let drag = self.cld.drag;
            let handle_base =
                match drag {
                    Some(CldDrag::Bend(id)) => d.links.iter().find(|l| l.id == id).and_then(|l| {
                        Some((self.cld_center(d, l.from)?, self.cld_center(d, l.to)?))
                    }),
                    _ => None,
                };
            match drag {
                Some(CldDrag::Pan) => self.cld.pan += delta,
                Some(CldDrag::Move(CldSel::Node(id))) => {
                    if let Some(n) = self
                        .cld_mut()
                        .and_then(|d| d.nodes.iter_mut().find(|n| n.id == id))
                    {
                        n.pos[0] += delta.x;
                        n.pos[1] += delta.y;
                    }
                }
                Some(CldDrag::Move(CldSel::Loop(id))) => {
                    if let Some(n) = self
                        .cld_mut()
                        .and_then(|d| d.loops.iter_mut().find(|n| n.id == id))
                    {
                        n.pos[0] += delta.x;
                        n.pos[1] += delta.y;
                    }
                }
                Some(CldDrag::Bend(id)) => {
                    if let Some((a, b)) = handle_base {
                        let dir = (b - a).normalized();
                        let bend = (pd - (a + (b - a) * 0.5)).dot(vec2(-dir.y, dir.x));
                        if let Some(l) = self
                            .cld_mut()
                            .and_then(|d| d.links.iter_mut().find(|l| l.id == id))
                        {
                            l.bend = bend;
                        }
                    }
                }
                _ => {}
            }
        }
        if resp.drag_stopped() {
            if let Some(CldDrag::NewLink(from)) = self.cld.drag {
                if let Some((CldSel::Node(to), _)) = self.cld_hit(d, pd) {
                    if to != from && !d.links.iter().any(|l| l.from == from && l.to == to) {
                        self.checkpoint();
                        let id = self.model.new_id();
                        let len = self
                            .cld_center(d, from)
                            .zip(self.cld_center(d, to))
                            .map(|(a, b)| a.distance(b))
                            .unwrap_or(100.0);
                        if let Some(dd) = self.cld_mut() {
                            dd.links.push(CldLink {
                                id,
                                from,
                                to,
                                bend: len * 0.15,
                                polarity: "+".into(),
                                delay: false,
                            });
                        }
                        self.cld.sel = Some(CldSel::Link(id));
                        self.status =
                            "Arrow added as + (same direction). Change its sign on the right."
                                .into();
                    }
                }
            }
            self.cld.drag = None;
        }
        if resp.clicked() {
            match self.cld.tool {
                CldTool::Select | CldTool::Link => {
                    self.cld.sel = self.cld_hit(d, pd).map(|h| h.0);
                }
                CldTool::Variable => {
                    self.checkpoint();
                    let id = self.model.new_id();
                    if let Some(dd) = self.cld_mut() {
                        dd.nodes.push(CldNode {
                            id,
                            text: "New variable".into(),
                            pos: [pd.x, pd.y],
                        });
                    }
                    self.cld.sel = Some(CldSel::Node(id));
                    self.cld.focus_name = true;
                    self.cld.tool = CldTool::Select;
                }
                CldTool::Loop => {
                    self.checkpoint();
                    let id = self.model.new_id();
                    let n = d.loops.len() + 1;
                    if let Some(dd) = self.cld_mut() {
                        dd.loops.push(CldLoop {
                            id,
                            pos: [pd.x, pd.y],
                            reinforcing: true,
                            label: format!("R{n}"),
                            clockwise: true,
                        });
                    }
                    self.cld.sel = Some(CldSel::Loop(id));
                    self.cld.tool = CldTool::Select;
                }
            }
        }
        if resp.double_clicked() {
            if let Some((CldSel::Node(id), _)) = self.cld_hit(d, pd) {
                self.cld.sel = Some(CldSel::Node(id));
                self.cld.focus_name = true;
            }
        }
    }

    fn cld_draw(&mut self, ui: &egui::Ui, p: &egui::Painter, origin: Pos2, d: &CldData) {
        let pal = self.pal;
        let z = self.cld.zoom;
        let found = find_loops(d);
        let hl = self.cld.highlight.and_then(|i| found.get(i)).cloned();
        let hl_col = hl.as_ref().map(|f| loop_color(self, f.reinforcing));

        // measure words
        let mut rects = HashMap::new();
        for n in &d.nodes {
            let g = p.layout_no_wrap(n.text.clone(), FontId::proportional(15.0), Color32::WHITE);
            rects.insert(
                n.id,
                Rect::from_center_size(pos2(n.pos[0], n.pos[1]), g.size() + vec2(18.0, 10.0)),
            );
        }
        self.cld.rects = rects;

        // arrows (their signs are drawn after the words so nothing covers them)
        let mut signs: Vec<(Pos2, &str, Color32)> = Vec::new();
        let mut paths = HashMap::new();
        for l in &d.links {
            let (Some(a), Some(b)) = (self.cld_center(d, l.from), self.cld_center(d, l.to)) else {
                continue;
            };
            let (Some(ra), Some(rb)) = (
                self.cld.rects.get(&l.from).copied(),
                self.cld.rects.get(&l.to).copied(),
            ) else {
                continue;
            };
            let pts = theme::arc_points(a, b, l.bend, 64);
            let s = pts
                .iter()
                .position(|q| !ra.expand(2.0).contains(*q))
                .unwrap_or(0);
            let e = pts
                .iter()
                .rposition(|q| !rb.expand(4.0).contains(*q))
                .unwrap_or(pts.len() - 1);
            if e <= s + 1 {
                continue;
            }
            let path: Vec<Pos2> = pts[s..=e].to_vec();
            let selected = self.cld.sel == Some(CldSel::Link(l.id));
            let in_hl = hl.as_ref().is_some_and(|f| f.links.contains(&l.id));
            let (col, w) = if selected {
                (pal.accent, 2.4)
            } else if in_hl {
                (hl_col.unwrap(), 2.6)
            } else if hl.is_some() {
                (pal.link.gamma_multiply(0.3), 1.2)
            } else {
                (pal.link, 1.5)
            };
            let sp: Vec<Pos2> = path
                .iter()
                .map(|q| self.cld_to_screen(origin, *q))
                .collect();
            let tip = *sp.last().unwrap();
            let dirv = (tip - sp[sp.len().saturating_sub(3)]).normalized();
            let head = 11.0 * z.sqrt();
            let mut line = sp.clone();
            *line.last_mut().unwrap() = tip - dirv * head * 0.8;
            p.add(egui::Shape::line(line, Stroke::new(w * z.sqrt(), col)));
            theme::arrow_head_slim(p, tip, dirv, head, col);
            let mid_i = sp.len() / 2;
            let tangent =
                (sp[(mid_i + 1).min(sp.len() - 1)] - sp[mid_i.saturating_sub(1)]).normalized();
            let normal = vec2(-tangent.y, tangent.x);
            if l.delay {
                for off in [-3.0, 3.0] {
                    let c = sp[mid_i] + tangent * off * z;
                    p.line_segment(
                        [c - normal * 8.0 * z, c + normal * 8.0 * z],
                        Stroke::new(1.8 * z.sqrt(), col),
                    );
                }
            }
            // sign beside the head, on the inside of the curve
            let at_i = sp.len().saturating_sub(sp.len() / 7 + 2);
            let t2 = (sp[(at_i + 1).min(sp.len() - 1)] - sp[at_i.saturating_sub(1)]).normalized();
            let side = if l.bend > 0.5 { -1.0 } else { 1.0 };
            let c = sp[at_i] + vec2(-t2.y, t2.x) * side * 12.0 * z;
            let (txt, pc) = match l.polarity.as_str() {
                "+" => ("+", pal.ok),
                "-" => ("−", pal.error),
                _ => ("?", pal.muted),
            };
            signs.push((c, txt, pc));
            if selected {
                if let Some(h) = self.cld_handle(d, l) {
                    let hs = self.cld_to_screen(origin, h);
                    p.circle(hs, 5.5, pal.panel, Stroke::new(2.0, pal.accent));
                }
            }
            paths.insert(l.id, path);
        }
        self.cld.paths = paths;

        // words
        for n in &d.nodes {
            let r = self.cld.rects[&n.id];
            let sr = Rect::from_min_max(
                self.cld_to_screen(origin, r.min),
                self.cld_to_screen(origin, r.max),
            );
            let selected = self.cld.sel == Some(CldSel::Node(n.id));
            let in_hl = hl.as_ref().is_some_and(|f| f.nodes.contains(&n.id));
            if selected {
                p.rect_filled(sr, CornerRadius::same(8), pal.accent_soft);
                p.rect_stroke(
                    sr,
                    CornerRadius::same(8),
                    Stroke::new(1.5, pal.accent),
                    StrokeKind::Inside,
                );
            } else if in_hl {
                p.rect_filled(
                    sr,
                    CornerRadius::same(8),
                    hl_col.unwrap().gamma_multiply(0.12),
                );
            } else {
                p.rect_filled(sr, CornerRadius::same(8), pal.canvas.gamma_multiply(0.85));
            }
            let col = if hl.is_some() && !in_hl {
                pal.muted
            } else {
                pal.text
            };
            p.text(
                sr.center(),
                Align2::CENTER_CENTER,
                &n.text,
                FontId::proportional(15.0 * z),
                col,
            );
        }

        for (c, txt, pc) in signs {
            p.circle_filled(c, 8.0 * z, pal.panel);
            p.text(
                c,
                Align2::CENTER_CENTER,
                txt,
                FontId::proportional(16.0 * z),
                pc,
            );
        }

        // loop markers: a circular arrow with R or B inside
        for lp in &d.loops {
            let c = self.cld_to_screen(origin, pos2(lp.pos[0], lp.pos[1]));
            let col = loop_color(self, Some(lp.reinforcing));
            let r = 17.0 * z;
            let dir = if lp.clockwise { 1.0 } else { -1.0 };
            let a0 = -1.9f32;
            let span = 5.2f32;
            let pts: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let t = a0 + dir * span * i as f32 / 40.0;
                    c + vec2(t.cos(), t.sin()) * r
                })
                .collect();
            if self.cld.sel == Some(CldSel::Loop(lp.id)) {
                p.circle_filled(c, r + 8.0 * z, pal.accent.gamma_multiply(0.15));
            }
            p.circle_filled(c, r - 2.0, pal.panel);
            p.add(egui::Shape::line(
                pts.clone(),
                Stroke::new(2.2 * z.sqrt(), col),
            ));
            let tip = *pts.last().unwrap();
            theme::arrow_head(
                p,
                tip,
                (tip - pts[pts.len() - 3]).normalized(),
                9.0 * z.sqrt(),
                col,
            );
            p.text(
                c,
                Align2::CENTER_CENTER,
                if lp.reinforcing { "R" } else { "B" },
                FontId::proportional(17.0 * z),
                col,
            );
            if !lp.label.is_empty() {
                p.text(
                    c + vec2(0.0, r + 6.0 * z),
                    Align2::CENTER_TOP,
                    &lp.label,
                    FontId::proportional(12.0 * z),
                    col,
                );
            }
        }

        // arrow being drawn
        if let (Some(CldDrag::NewLink(from)), Some(pp)) =
            (self.cld.drag, ui.input(|i| i.pointer.interact_pos()))
        {
            if let Some(c) = self.cld_center(d, from) {
                let a = self.cld_to_screen(origin, c);
                p.add(egui::Shape::dashed_line(
                    &[a, pp],
                    Stroke::new(1.6, pal.accent),
                    6.0,
                    4.0,
                ));
                theme::arrow_head_slim(p, pp, (pp - a).normalized(), 11.0, pal.accent);
            }
        }
    }

    /// Draw this causal loop diagram off-screen for an export.
    pub fn cld_export(
        &mut self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        origin: Pos2,
    ) -> Option<Rect> {
        let d = self.cld_data();
        let mut b = Rect::NOTHING;
        for n in &d.nodes {
            if let Some(r) = self.cld.rects.get(&n.id) {
                b = b.union(*r);
            }
        }
        for l in &d.loops {
            b = b.union(Rect::from_center_size(
                pos2(l.pos[0], l.pos[1] + 10.0),
                vec2(60.0, 70.0),
            ));
        }
        // a curved arrow can leave the box the words make; keep room for its sign beside it too
        for path in self.cld.paths.values() {
            for q in path {
                b = b.union(Rect::from_center_size(*q, vec2(24.0, 24.0)));
            }
        }
        if !b.is_positive() {
            return None;
        }
        let b = b.expand(30.0);
        let saved = (
            self.cld.pan,
            self.cld.zoom,
            self.cld.sel,
            self.cld.highlight,
            self.cld.drag,
        );
        self.cld.zoom = 1.0;
        self.cld.pan = -b.min.to_vec2();
        self.cld.sel = None;
        self.cld.highlight = None;
        self.cld.drag = None;
        let area = Rect::from_min_size(origin, b.size());
        let ui = self.export_ui(ctx, layer, area);
        let painter = egui::Painter::new(ctx.clone(), layer, Rect::EVERYTHING);
        self.cld_draw(&ui, &painter, origin, &d);
        (
            self.cld.pan,
            self.cld.zoom,
            self.cld.sel,
            self.cld.highlight,
            self.cld.drag,
        ) = saved;
        Some(area)
    }

    pub fn cld_side(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let d = self.cld_data();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.label(RichText::new("Causal loop diagram").size(18.0).strong());
            ui.label(RichText::new("A picture of what drives what. Arrows marked + move in the same direction, − in the opposite one.").color(pal.muted));
            ui.add_space(6.0);

            // ---- selection
            match self.cld.sel {
                Some(CldSel::Node(id)) => {
                    if let Some(n) = d.nodes.iter().find(|n| n.id == id) {
                        crate::inspector::section(ui, pal, "Variable");
                        let mut text = n.text.clone();
                        let r = ui.add(egui::TextEdit::singleline(&mut text).font(FontId::proportional(16.0)).desired_width(f32::INFINITY));
                        if self.cld.focus_name {
                            r.request_focus();
                            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                                st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(text.chars().count()))));
                                st.store(ui.ctx(), r.id);
                            }
                            self.cld.focus_name = false;
                        }
                        if r.gained_focus() {
                            self.checkpoint();
                        }
                        if text != n.text {
                            if let Some(x) = self.cld_mut().and_then(|d| d.nodes.iter_mut().find(|x| x.id == id)) {
                                x.text = text;
                            }
                        }
                        let ins = d.links.iter().filter(|l| l.to == id).count();
                        let outs = d.links.iter().filter(|l| l.from == id).count();
                        ui.label(RichText::new(format!("{ins} cause(s) · {outs} effect(s)")).small().color(pal.muted));
                    }
                }
                Some(CldSel::Link(id)) => {
                    if let Some(l) = d.links.iter().find(|l| l.id == id).cloned() {
                        let name = |nid: u64| d.nodes.iter().find(|n| n.id == nid).map(|n| n.text.clone()).unwrap_or_default();
                        crate::inspector::section(ui, pal, "Causal arrow");
                        ui.label(RichText::new(format!("{}  →  {}", name(l.from), name(l.to))).strong());
                        let mut e = l.clone();
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut e.polarity, "+".to_string(), RichText::new("+ same").color(pal.ok));
                            ui.selectable_value(&mut e.polarity, "-".to_string(), RichText::new("− opposite").color(pal.error));
                            ui.selectable_value(&mut e.polarity, String::new(), "? unsure");
                        });
                        ui.checkbox(&mut e.delay, "Delay mark ||  (the effect takes time)");
                        ui.horizontal(|ui| {
                            for (label, b) in [("Straight", 0.0f32), ("Gentle", 30.0), ("Round", 70.0)] {
                                if ui.selectable_label((e.bend.abs() - b).abs() < 1.0, label).clicked() {
                                    e.bend = if e.bend < 0.0 { -b } else { b };
                                }
                            }
                            if ui.button("⇅ Flip").clicked() {
                                e.bend = -e.bend;
                            }
                        });
                        if e.polarity != l.polarity || e.delay != l.delay || e.bend != l.bend {
                            self.checkpoint();
                            if let Some(x) = self.cld_mut().and_then(|d| d.links.iter_mut().find(|x| x.id == id)) {
                                *x = e;
                            }
                        }
                    }
                }
                Some(CldSel::Loop(id)) => {
                    if let Some(lp) = d.loops.iter().find(|l| l.id == id).cloned() {
                        crate::inspector::section(ui, pal, "Loop marker");
                        let mut e = lp.clone();
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut e.reinforcing, true, RichText::new("R  reinforcing").color(pal.aux));
                            ui.selectable_value(&mut e.reinforcing, false, RichText::new("B  balancing").color(pal.accent));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Label").color(pal.muted));
                            ui.text_edit_singleline(&mut e.label);
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut e.clockwise, true, "↻ clockwise");
                            ui.selectable_value(&mut e.clockwise, false, "↺ counter-clockwise");
                        });
                        if serde_json::to_string(&e).ok() != serde_json::to_string(&lp).ok() {
                            if let Some(x) = self.cld_mut().and_then(|d| d.loops.iter_mut().find(|x| x.id == id)) {
                                *x = e;
                            }
                        }
                    }
                }
                None => {}
            }
            if self.cld.sel.is_some() && ui.button(RichText::new("Delete").color(pal.error)).clicked() {
                self.cld_delete();
            }

            // ---- loops
            let found = find_loops(&d);
            crate::inspector::section(ui, pal, &format!("Feedback loops found ({})", found.len()));
            if found.is_empty() {
                ui.label(RichText::new("No closed loops yet. Draw arrows that lead back to where they started.").color(pal.muted));
            }
            let (mut rn, mut bn) = (0, 0);
            for (i, f) in found.iter().enumerate() {
                let (tag, col) = match f.reinforcing {
                    Some(true) => {
                        rn += 1;
                        (format!("R{rn}"), pal.aux)
                    }
                    Some(false) => {
                        bn += 1;
                        (format!("B{bn}"), pal.accent)
                    }
                    None => ("?".to_string(), pal.muted),
                };
                let mut names: Vec<String> = f.nodes.iter().filter_map(|id| d.nodes.iter().find(|n| n.id == *id)).map(|n| n.text.clone()).collect();
                if let Some(first) = names.first().cloned() {
                    names.push(first);
                }
                let on = self.cld.highlight == Some(i);
                let resp = egui::Frame::new()
                    .fill(if on { col.gamma_multiply(0.12) } else { pal.surface })
                    .stroke(Stroke::new(1.0, if on { col } else { pal.border }))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&tag).strong().color(col));
                            ui.label(RichText::new(match f.reinforcing {
                                Some(true) => "reinforcing — amplifies change",
                                Some(false) => "balancing — pushes back",
                                None => "unknown — an arrow has no sign",
                            }).small().color(pal.muted));
                        });
                        ui.label(RichText::new(names.join(" → ")).small());
                        if on && ui.small_button("Add a marker for this loop").clicked() {
                            let pts: Vec<[f32; 2]> = f.nodes.iter().filter_map(|id| d.nodes.iter().find(|n| n.id == *id)).map(|n| n.pos).collect();
                            let c = pts.iter().fold([0.0, 0.0], |a, q| [a[0] + q[0], a[1] + q[1]]);
                            let k = pts.len().max(1) as f32;
                            self.checkpoint();
                            let id = self.model.new_id();
                            if let Some(dd) = self.cld_mut() {
                                dd.loops.push(CldLoop { id, pos: [c[0] / k, c[1] / k], reinforcing: f.reinforcing.unwrap_or(true), label: tag.clone(), clockwise: true });
                            }
                            self.cld.sel = Some(CldSel::Loop(id));
                        }
                    })
                    .response;
                if resp.interact(Sense::click()).on_hover_text("Click to highlight this loop on the diagram").clicked() {
                    self.cld.highlight = if on { None } else { Some(i) };
                }
                ui.add_space(4.0);
            }

            crate::inspector::section(ui, pal, "Start from the model");
            ui.label(RichText::new("Turn the stock-and-flow model into a causal loop diagram. This replaces what's on this sheet (⌘Z undoes).").small().color(pal.muted));
            if ui.button("⟲  Build from model").clicked() {
                self.checkpoint();
                let mut m = std::mem::take(&mut self.model);
                let data = from_model(&mut m);
                self.model = m;
                if let Some(dd) = self.cld_mut() {
                    *dd = data;
                }
                self.cld.fit_pending = true;
                self.cld.rects.clear();
                self.cld.sel = None;
                self.cld.highlight = None;
            }
        });
    }
}
