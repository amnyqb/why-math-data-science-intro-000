//! The diagram: drawing the stock-and-flow picture and editing it with the mouse.

use std::collections::HashMap;

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
    pos2, vec2,
};

use crate::app::{App, BottomTab, Sel, Tool};
use crate::expr;
use crate::model::{Kind, Model, Note, Var};
use crate::theme;

#[derive(Clone, Copy, Debug)]
pub enum Drag {
    Pan,
    Move,
    Cloud { id: u64, to_end: bool },
    Bend { id: u64 },
    Via { id: u64, idx: usize },
    /// Dragging a key-lever circle up or down changes its value.
    Lever { id: u64 },
    NewFlow { from: Option<u64>, start: Pos2 },
    NewLink { from: u64 },
    Marquee { start: Pos2 },
    /// Drawing a new shape from `start`.
    NewShape { start: Pos2 },
    /// Pulling a corner or an end of a shape.
    ShapeHandle { id: u64, handle: u8 },
    /// Operating a control (`part`: 1 the main part, 2 minus, 3 plus).
    Control { id: u64, part: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hit {
    Var(u64),
    Shape(u64),
    ShapeHandle(u64, u8),
    /// A part of a control: 0 its body, 1 what is operated, 2 minus, 3 plus.
    Control(u64, u8),
    Cloud(u64, bool),
    Bend(u64),
    /// A turning point of an arrow, and the place between two of them where a new one can be dragged out.
    Via(u64, usize),
    AddVia(u64, usize),
    /// The fold chevron in a module card's header.
    Fold(u64),
    /// A key-lever circle.
    Lever(u64),
    Note(u64),
    Link(u64),
    Module(u64),
    GroupTab(u64),
    /// A module's input drawn on its own sheet (the id is the outside variable).
    Ghost(u64),
}

#[derive(Default)]
pub struct CanvasState {
    pub drag: Option<Drag>,
    /// Node outlines in diagram units, from the last frame.
    rects: HashMap<u64, Rect>,
    note_rects: HashMap<u64, Rect>,
    link_paths: HashMap<u64, Vec<Pos2>>,
    ctx_target: Option<Hit>,
    pub playing: bool,
    play_t: f64,
    pub focus_equation: bool,
    group_tabs: HashMap<u64, Rect>,
    ghosts: Vec<(Rect, u64)>,
    /// Key-lever circles drawn outside the module boxes this frame: centre, radius, variable.
    lever_nodes: Vec<(Pos2, f32, u64)>,
    /// True while the sheet shows its levers as circles (the floating card steps aside).
    pub levers_shown: bool,
}

impl CanvasState {
    pub fn rect_of(&self, id: u64) -> Option<Rect> {
        self.rects.get(&id).copied()
    }
    pub fn note_rect_of(&self, id: u64) -> Option<Rect> {
        self.note_rects.get(&id).copied()
    }
}

pub const MODULE_W: f32 = 230.0;

fn hit_sel(h: Hit) -> Option<Sel> {
    match h {
        Hit::Var(i) | Hit::Cloud(i, _) => Some(Sel::Var(i)),
        Hit::Note(i) => Some(Sel::Note(i)),
        Hit::Shape(i) | Hit::ShapeHandle(i, _) => Some(Sel::Shape(i)),
        Hit::Control(i, _) => Some(Sel::Control(i)),
        Hit::Module(i) => Some(Sel::Module(i)),
        Hit::GroupTab(i) => Some(Sel::Group(i)),
        Hit::Link(i) | Hit::Bend(i) | Hit::Via(i, _) | Hit::AddVia(i, _) => Some(Sel::Link(i)),
        Hit::Fold(i) => Some(Sel::Module(i)),
        Hit::Lever(i) => Some(Sel::Var(i)),
        Hit::Ghost(_) => None,
    }
}

pub const STOCK_SIZE: Vec2 = vec2(140.0, 72.0);
/// How far a loose end of a pipe may lie from its stock.
const CLOUD_REACH: f32 = 200.0;

/// Add an arrow for every variable an equation uses but isn't linked yet. Returns how many were added.
pub fn sync_links(m: &mut Model) -> usize {
    let mut pairs = Vec::new();
    for v in &m.vars {
        let Ok(e) = expr::parse(&v.equation) else {
            continue;
        };
        let mut refs = Vec::new();
        expr::references(&e, &mut refs);
        for r in refs {
            if let Some(src) = m.by_name(&r) {
                if src.id != v.id && !m.links.iter().any(|l| l.from == src.id && l.to == v.id) {
                    pairs.push((src.id, v.id));
                }
            }
        }
    }
    let n = pairs.len();
    for (a, b) in pairs {
        let len = match (m.var(a), m.var(b)) {
            (Some(x), Some(y)) => {
                ((x.pos[0] - y.pos[0]).powi(2) + (x.pos[1] - y.pos[1]).powi(2)).sqrt()
            }
            _ => 0.0,
        };
        if m.add_link(a, b) {
            m.links.last_mut().unwrap().bend = len * 0.12;
        }
    }
    n
}

thread_local! {
    /// Figures (`dyn render`) draw a pipe where the model puts it: the valve halfway between the ends, a cloud where it was
    /// placed, nothing moved or shortened. The window straightens pipes and keeps clouds near their stock.
    static RAW_PIPES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Draw pipes as the model has them (on this thread), or straightened as the window does.
pub fn set_raw_pipes(on: bool) {
    RAW_PIPES.with(|c| c.set(on));
}

pub fn raw_pipes() -> bool {
    RAW_PIPES.with(|c| c.get())
}

/// A flow's two ends; a stock on another sheet counts as a cloud here. A flow is never drawn
/// diagonally: a pipe is horizontal or vertical, whichever its ends are nearer to.
fn flow_ends(m: &Model, f: &Var) -> (Pos2, Pos2) {
    if raw_pipes() {
        return flow_ends_raw(m, f);
    }
    let (mut a, mut b) = flow_ends_raw(m, f);
    let attached = |id: Option<u64>| id.and_then(|i| m.var(i)).is_some_and(|s| s.view == f.view);
    let (da, db) = (attached(f.from), attached(f.to));
    let d = b - a;
    let horizontal = d.x.abs() >= d.y.abs();
    match (da, db) {
        // both ends are stocks: one line through the part of both that lines up
        (true, true) => {
            if horizontal && d.y.abs() <= STOCK_SIZE.y * 0.45 {
                let y = (a.y + b.y) / 2.0;
                (a.y, b.y) = (y, y);
            } else if !horizontal && d.x.abs() <= STOCK_SIZE.x * 0.45 {
                let x = (a.x + b.x) / 2.0;
                (a.x, b.x) = (x, x);
            }
        }
        // a cloud lines up with the stock it hangs on, and does not wander off across the diagram
        (true, false) => {
            if horizontal {
                b.y = a.y;
                b.x = a.x + (b.x - a.x).clamp(-CLOUD_REACH, CLOUD_REACH);
            } else {
                b.x = a.x;
                b.y = a.y + (b.y - a.y).clamp(-CLOUD_REACH, CLOUD_REACH);
            }
        }
        (false, true) => {
            if horizontal {
                a.y = b.y;
                a.x = b.x + (a.x - b.x).clamp(-CLOUD_REACH, CLOUD_REACH);
            } else {
                a.x = b.x;
                a.y = b.y + (a.y - b.y).clamp(-CLOUD_REACH, CLOUD_REACH);
            }
        }
        (false, false) => {
            if horizontal {
                b.y = a.y
            } else {
                b.x = a.x
            }
        }
    }
    (a, b)
}

fn flow_ends_raw(m: &Model, f: &Var) -> (Pos2, Pos2) {
    let a = f
        .from
        .and_then(|id| m.var(id))
        .filter(|s| s.view == f.view)
        .map(|s| s.pos)
        .unwrap_or(f.from_pt);
    let b =
        f.to.and_then(|id| m.var(id))
            .filter(|s| s.view == f.view)
            .map(|s| s.pos)
            .unwrap_or(f.to_pt);
    (pos2(a[0], a[1]), pos2(b[0], b[1]))
}

/// Where the segment from `inside` toward `outside` leaves `r`.
fn clip_rect(r: Rect, inside: Pos2, outside: Pos2) -> Pos2 {
    let d = outside - inside;
    let mut t = 1.0f32;
    if d.x.abs() > 1e-6 {
        let tx = if d.x > 0.0 {
            (r.right() - inside.x) / d.x
        } else {
            (r.left() - inside.x) / d.x
        };
        t = t.min(tx);
    }
    if d.y.abs() > 1e-6 {
        let ty = if d.y > 0.0 {
            (r.bottom() - inside.y) / d.y
        } else {
            (r.top() - inside.y) / d.y
        };
        t = t.min(ty);
    }
    inside + d * t.clamp(0.0, 1.0)
}

fn dist_to_polyline(p: Pos2, pts: &[Pos2]) -> f32 {
    pts.windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let ab = b - a;
            let t = if ab.length_sq() > 0.0 {
                ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a + ab * t).distance(p)
        })
        .fold(f32::INFINITY, f32::min)
}

impl App {
    /// Make every flow's loose ends line up with their stock (and, when `move_stocks`, stocks
    /// joined by a flow that do not line up at all are moved so the pipe is straight). Pipes are
    /// never diagonal.
    pub fn straighten_flows(&mut self, move_stocks: bool) {
        let ids: Vec<u64> = self.model.vars.iter().filter(|v| v.kind == Kind::Flow).map(|v| v.id).collect();
        for id in ids {
            let Some(f) = self.model.var(id).cloned() else { continue };
            let stock = |s: Option<u64>| s.and_then(|i| self.model.var(i)).filter(|x| x.view == f.view).map(|x| (x.id, pos2(x.pos[0], x.pos[1])));
            let (a, b) = (stock(f.from), stock(f.to));
            if move_stocks {
                if let (Some((_, pa)), Some((ib, pb))) = (a, b) {
                    let d = pb - pa;
                    if d.x.abs() >= d.y.abs() && d.y.abs() > STOCK_SIZE.y * 0.45 {
                        if let Some(s) = self.model.var_mut(ib) {
                            s.pos[1] = pa.y;
                        }
                    } else if d.x.abs() < d.y.abs() && d.x.abs() > STOCK_SIZE.x * 0.45 {
                        if let Some(s) = self.model.var_mut(ib) {
                            s.pos[0] = pa.x;
                        }
                    }
                }
            }
            // loose ends: where the pipe would end, kept as saved points too
            let (ea, eb) = flow_ends(&self.model, &f);
            if let Some(v) = self.model.var_mut(id) {
                if a.is_none() {
                    v.from_pt = [ea.x, ea.y];
                }
                if b.is_none() {
                    v.to_pt = [eb.x, eb.y];
                }
            }
        }
    }

    /// The diagram sheet being shown (the active sheet, or the first diagram).
    pub fn diagram_view(&self) -> u64 {
        match self.model.sheets.iter().find(|s| s.id == self.sheet) {
            Some(s) if s.kind == crate::model::SheetKind::Diagram => s.id,
            _ => self.model.first_diagram(),
        }
    }
    /// Is a flow end attached to a stock drawn on the same sheet?
    fn end_here(&self, f: &Var, end: Option<u64>) -> bool {
        end.and_then(|id| self.model.var(id))
            .is_some_and(|s| s.view == f.view)
    }
    pub(crate) fn to_screen(&self, origin: Pos2, p: Pos2) -> Pos2 {
        origin + (p.to_vec2() + self.pan) * self.zoom
    }
    pub(crate) fn to_diagram(&self, origin: Pos2, p: Pos2) -> Pos2 {
        ((p - origin) / self.zoom - self.pan).to_pos2()
    }

    fn hit(&self, d: Pos2) -> Option<Hit> {
        let view = self.diagram_view();
        if let Some((id, h)) = self.shape_handle_hit(d) {
            return Some(Hit::ShapeHandle(id, h));
        }
        if let Some(Sel::Link(id)) = self.sel {
            let tol = 9.0 / self.zoom.max(0.5);
            let vias: Vec<Pos2> = self.model.links.iter().find(|l| l.id == id).map(|l| l.via.iter().map(|v| pos2(v[0], v[1])).collect()).unwrap_or_default();
            if let (false, Some(path)) = (vias.is_empty(), self.canvas.link_paths.get(&id)) {
                for (i, v) in vias.iter().enumerate() {
                    if v.distance(d) < tol {
                        return Some(Hit::Via(id, i));
                    }
                }
                for (j, a) in via_handles(path, &vias).iter().enumerate() {
                    if a.distance(d) < tol {
                        return Some(Hit::AddVia(id, j));
                    }
                }
            } else if let Some(path) = self.canvas.link_paths.get(&id) {
                if path[path.len() / 2].distance(d) < tol {
                    return Some(Hit::Bend(id));
                }
            }
        }
        for (r, id) in &self.canvas.ghosts {
            if r.contains(d) {
                return Some(Hit::Ghost(*id));
            }
        }
        for (g, r) in &self.canvas.group_tabs {
            if r.contains(d) {
                return Some(Hit::GroupTab(*g));
            }
        }
        for v in self
            .model
            .vars
            .iter()
            .filter(|v| v.kind == Kind::Flow && v.view == view)
        {
            if !self.end_here(v, v.from) && pos2(v.from_pt[0], v.from_pt[1]).distance(d) < 16.0 {
                return Some(Hit::Cloud(v.id, false));
            }
            if !self.end_here(v, v.to) && pos2(v.to_pt[0], v.to_pt[1]).distance(d) < 16.0 {
                return Some(Hit::Cloud(v.id, true));
            }
        }
        // later-drawn things sit on top: small variables, then flows, then stocks
        for kinds in [
            [Kind::Aux, Kind::Constant, Kind::Lookup].as_slice(),
            &[Kind::Flow],
            &[Kind::Stock],
        ] {
            for v in self
                .model
                .vars
                .iter()
                .rev()
                .filter(|v| kinds.contains(&v.kind) && v.view == view)
            {
                if self
                    .canvas
                    .rects
                    .get(&v.id)
                    .is_some_and(|r| r.expand(2.0).contains(d))
                {
                    return Some(Hit::Var(v.id));
                }
            }
        }
        for (c, r, id) in &self.canvas.lever_nodes {
            if c.distance(d) <= *r {
                return Some(Hit::Lever(*id));
            }
        }
        for m in self.model.modules.iter().filter(|m| m.view == view) {
            if let Some(r) = self.canvas.rects.get(&m.id) {
                // the fold chevron at the right end of the header
                let chev = Rect::from_min_size(pos2(r.right() - 30.0, r.top()), vec2(30.0, 30.0));
                if chev.contains(d) {
                    return Some(Hit::Fold(m.id));
                }
                if r.contains(d) {
                    return Some(Hit::Module(m.id));
                }
            }
        }
        for n in self.model.notes.iter().filter(|n| n.view == view) {
            if self
                .canvas
                .note_rects
                .get(&n.id)
                .is_some_and(|r| r.contains(d))
            {
                return Some(Hit::Note(n.id));
            }
        }
        let tol = 6.0 / self.zoom;
        for l in &self.model.links {
            if let Some(path) = self.canvas.link_paths.get(&l.id) {
                if dist_to_polyline(d, path) < tol {
                    return Some(Hit::Link(l.id));
                }
            }
        }
        // controls sit above the shapes and pictures, which are drawn behind everything and found last
        if let Some((id, part)) = self.control_hit(d) {
            return Some(Hit::Control(id, part));
        }
        self.shape_hit(d).map(Hit::Shape)
    }

    /// Where the key-lever circles of a sheet sit: in a row above its module boxes.
    fn lever_layout(&self, view: u64) -> Vec<(Pos2, u64)> {
        let mods: Vec<[f32; 2]> = self.model.modules.iter().filter(|m| m.view == view).map(|m| m.pos).collect();
        if mods.is_empty() {
            return Vec::new();
        }
        let top = mods.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let left = mods.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        self.model
            .vars
            .iter()
            .filter(|v| v.lever && v.kind == Kind::Constant)
            .map(|v| v.id)
            .filter(|id| self.proxy(*id, view).is_some_and(|x| x != *id))
            .enumerate()
            .map(|(i, id)| (pos2(left - 120.0 + (i % 6) as f32 * 180.0, top - 230.0 - (i / 6) as f32 * 120.0), id))
            .collect()
    }

    fn fit(&mut self, view: Rect) {
        let mut b = Rect::NOTHING;
        let sheet = self.diagram_view();
        for (c, _) in self.lever_layout(sheet) {
            b = b.union(Rect::from_center_size(c, vec2(120.0, 120.0)));
        }
        for v in self.model.vars.iter().filter(|v| v.view == sheet) {
            let r = self
                .canvas
                .rects
                .get(&v.id)
                .copied()
                .unwrap_or(Rect::from_center_size(
                    pos2(v.pos[0], v.pos[1]),
                    vec2(140.0, 60.0),
                ));
            b = b.union(r);
            if v.kind == Kind::Flow {
                b = b.union(Rect::from_center_size(
                    pos2(v.from_pt[0], v.from_pt[1]),
                    vec2(30.0, 30.0),
                ));
                b = b.union(Rect::from_center_size(
                    pos2(v.to_pt[0], v.to_pt[1]),
                    vec2(30.0, 30.0),
                ));
            }
        }
        for (g, _) in self.ghost_layout(sheet) {
            b = b.union(g);
        }
        for m in self.model.modules.iter().filter(|m| m.view == sheet) {
            b = b.union(
                self.canvas
                    .rects
                    .get(&m.id)
                    .copied()
                    .unwrap_or(Rect::from_center_size(
                        pos2(m.pos[0], m.pos[1]),
                        vec2(MODULE_W, 90.0),
                    )),
            );
        }
        for s in self.model.shapes.iter().filter(|s| s.view == sheet) {
            if let Some(r) = self.shape_rect(s.id) {
                b = b.union(r);
            }
        }
        for k in self.model.controls.iter().filter(|k| k.view == sheet) {
            if let Some(r) = self.control_rect(k.id) {
                b = b.union(r);
            }
        }
        for n in self.model.notes.iter().filter(|n| n.view == sheet) {
            b = b.union(
                self.canvas
                    .note_rects
                    .get(&n.id)
                    .copied()
                    .unwrap_or(Rect::from_min_size(
                        pos2(n.pos[0], n.pos[1]),
                        vec2(220.0, 40.0),
                    )),
            );
        }
        if !b.is_positive() {
            self.zoom = 1.0;
            self.pan = view.size() / 2.0;
            return;
        }
        // keep clear of the time slider and zoom controls along the bottom
        let view = Rect::from_min_max(view.min + vec2(0.0, 10.0), view.max - vec2(0.0, 66.0));
        let b = b.expand(30.0);
        let z = (view.width() / b.width())
            .min(view.height() / b.height())
            .clamp(0.3, 1.4);
        self.zoom = z;
        self.pan = (view.size() / z - b.size()) / 2.0 - b.min.to_vec2() + vec2(0.0, 10.0) / z;
    }

    pub fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.scope_id().with("canvas"), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, pal.canvas);
        let origin = rect.min;

        if self.fit_pending && self.frame_ready() {
            self.fit(rect);
            self.fit_pending = false;
        }

        // ---- zoom & pan with wheel / trackpad
        if resp.hovered() {
            let (scroll, zoom_d, pointer) =
                ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
            if zoom_d != 1.0 {
                if let Some(p) = pointer {
                    let before = self.to_diagram(origin, p);
                    self.zoom = (self.zoom * zoom_d).clamp(0.2, 3.0);
                    let after = self.to_diagram(origin, p);
                    self.pan += after - before;
                }
            } else if scroll != Vec2::ZERO {
                self.pan += scroll / self.zoom;
            }
        }

        // ---- dot grid
        let step = 24.0 * self.zoom;
        if step > 6.0 {
            let off = vec2(
                (self.pan.x * self.zoom).rem_euclid(step),
                (self.pan.y * self.zoom).rem_euclid(step),
            );
            let mut y = rect.top() + off.y;
            while y < rect.bottom() {
                let mut x = rect.left() + off.x;
                while x < rect.right() {
                    painter.circle_filled(pos2(x, y), 1.0, pal.grid);
                    x += step;
                }
                y += step;
            }
        }

        self.handle_mouse(ui, &resp, origin);
        self.draw(ui, &painter, origin);
        self.overlays(ui, rect);
        let ctx = ui.ctx().clone();
        self.lever_card(&ctx, rect);

        // context menu
        if resp.secondary_clicked() {
            self.canvas.ctx_target = resp
                .interact_pointer_pos()
                .and_then(|p| self.hit(self.to_diagram(origin, p)));
            if let Some(s) = self.canvas.ctx_target.and_then(hit_sel) {
                if !self.selected_items().contains(&s) {
                    self.select_one(Some(s));
                }
            }
        }
        let target = self.canvas.ctx_target;
        resp.context_menu(|ui| self.context_menu(ui, target));
    }

    /// While exporting a selection, only its items are drawn.
    pub(crate) fn shown(&self, id: u64) -> bool {
        self.export_only.as_ref().is_none_or(|o| o.contains(&id))
    }

    /// Draw this diagram sheet (or just the selected items) off-screen for an export.
    pub fn diagram_export(
        &mut self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        origin: Pos2,
        selection: bool,
    ) -> Option<Rect> {
        let view = self.diagram_view();
        let only: Option<Vec<u64>> = if selection {
            let ids: Vec<u64> = self
                .selected_items()
                .iter()
                .map(|s| App::sel_id(*s))
                .collect();
            if ids.is_empty() {
                return None;
            }
            Some(ids)
        } else {
            None
        };
        let keep = |id: u64| only.as_ref().is_none_or(|o| o.contains(&id));
        let mut b = Rect::NOTHING;
        for v in self
            .model
            .vars
            .iter()
            .filter(|v| v.view == view && keep(v.id))
        {
            if let Some(r) = self.canvas.rects.get(&v.id) {
                b = b.union(*r);
            }
            if v.kind == Kind::Flow {
                for (end, pt) in [(v.from, v.from_pt), (v.to, v.to_pt)] {
                    if !self.end_here(v, end) {
                        b = b.union(Rect::from_center_size(pos2(pt[0], pt[1]), vec2(40.0, 40.0)));
                    } else if let Some(r) = end.and_then(|e| self.canvas.rects.get(&e)) {
                        b = b.union(Rect::from_center_size(r.center(), vec2(2.0, 2.0)));
                    }
                }
            }
        }
        // curved and turning arrows can leave the box the words and pipes make
        for l in self.model.links.iter() {
            let on_sheet = |id: u64| self.model.var(id).is_some_and(|v| v.view == view && keep(v.id));
            if on_sheet(l.from) && on_sheet(l.to) {
                for q in self.canvas.link_paths.get(&l.id).into_iter().flatten() {
                    b = b.union(Rect::from_center_size(*q, vec2(2.0, 2.0)));
                }
            }
        }
        for n in self
            .model
            .notes
            .iter()
            .filter(|n| n.view == view && keep(n.id))
        {
            if let Some(r) = self.canvas.note_rects.get(&n.id) {
                b = b.union(*r);
            }
        }
        for s in self.model.shapes.iter().filter(|s| s.view == view && keep(s.id)) {
            if let Some(r) = self.shape_rect(s.id) {
                b = b.union(r.expand(s.line_weight()));
            }
        }
        for k in self.model.controls.iter().filter(|k| k.view == view && keep(k.id)) {
            if let Some(r) = self.control_rect(k.id) {
                b = b.union(r.expand(4.0));
            }
        }
        for m in self
            .model
            .modules
            .iter()
            .filter(|m| m.view == view && keep(m.id))
        {
            if let Some(r) = self.canvas.rects.get(&m.id) {
                b = b.union(r.expand2(vec2(10.0, 10.0)));
            }
        }
        if only.is_none() {
            for (g, _) in self.ghost_layout(view) {
                b = b.union(g);
            }
        }
        if !b.is_positive() {
            return None;
        }
        // room for group frames, their name tabs and module shadows
        let b = b.expand(24.0);
        let saved = (self.pan, self.zoom, self.canvas.drag);
        self.zoom = 1.0;
        self.pan = -b.min.to_vec2();
        self.canvas.drag = None;
        self.sel = None;
        self.multi.clear();
        self.export_only = only;
        let area = Rect::from_min_size(origin, b.size());
        let ui = self.export_ui(ctx, layer, area);
        let painter = egui::Painter::new(ctx.clone(), layer, Rect::EVERYTHING);
        self.draw(&ui, &painter, origin);
        self.export_only = None;
        (self.pan, self.zoom, self.canvas.drag) = saved;
        Some(area)
    }

    /// Where a module's inputs are drawn on its own sheet: stacked to the left of its parts.
    fn ghost_layout(&self, view: u64) -> Vec<(Rect, u64)> {
        let Some(mid) = self
            .model
            .sheets
            .iter()
            .find(|s| s.id == view)
            .and_then(|s| s.module)
        else {
            return Vec::new();
        };
        let (ins, _) = self.module_ports(mid);
        let content = self
            .model
            .vars
            .iter()
            .filter(|v| v.view == view)
            .filter_map(|v| self.canvas.rects.get(&v.id))
            .fold(Rect::NOTHING, |a, r| a.union(*r));
        if !content.is_positive() {
            return Vec::new();
        }
        let n = ins.len() as f32;
        ins.iter()
            .enumerate()
            .filter_map(|(i, src)| {
                let sv = self.model.var(*src)?;
                let c = pos2(
                    content.left() - 190.0,
                    content.center().y + (i as f32 - (n - 1.0) / 2.0) * 58.0,
                );
                Some((
                    Rect::from_center_size(
                        c,
                        vec2(sv.name.chars().count() as f32 * 7.2 + 34.0, 34.0),
                    ),
                    *src,
                ))
            })
            .collect()
    }

    /// Fit needs node sizes, which exist only after the first drawn frame.
    fn frame_ready(&self) -> bool {
        !self.canvas.rects.is_empty() || self.model.vars.is_empty()
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, target: Option<Hit>) {
        match target {
            Some(Hit::Var(id)) => {
                let Some(v) = self.model.var(id).cloned() else {
                    return;
                };
                ui.label(RichText::new(&v.name).strong());
                if v.kind != Kind::Lookup || !v.equation.is_empty() {
                    let plotted = self.plotted.contains(&v.name);
                    if ui
                        .button(if plotted {
                            "Remove from graph"
                        } else {
                            "Add to graph"
                        })
                        .clicked()
                    {
                        if plotted {
                            self.plotted.retain(|n| n != &v.name)
                        } else {
                            self.plotted.push(v.name.clone())
                        }
                        self.bottom_tab = BottomTab::Trends;
                        ui.close();
                    }
                }
                if ui.button("Show causes and uses").clicked() {
                    self.bottom_tab = BottomTab::Causes;
                    ui.close();
                }
                if ui.button("Copy selection as picture").clicked() {
                    self.request_export(
                        crate::export::ExportWhat::Selection,
                        crate::export::ExportTo::Clipboard,
                    );
                    ui.close();
                }
                let others: Vec<(u64, String)> = self
                    .model
                    .sheets
                    .iter()
                    .filter(|s| s.kind == crate::model::SheetKind::Diagram && s.id != v.view)
                    .map(|s| (s.id, s.name.clone()))
                    .collect();
                if !others.is_empty() {
                    ui.menu_button("Move to sheet", |ui| {
                        for (sid, name) in others {
                            if ui.button(name).clicked() {
                                self.checkpoint();
                                if let Some(x) = self.model.var_mut(id) {
                                    x.view = sid;
                                }
                                self.sel = None;
                                self.status = format!("Moved '{}' to another sheet", v.name);
                                ui.close();
                            }
                        }
                    });
                }
                if matches!(v.kind, Kind::Aux | Kind::Constant | Kind::Lookup) {
                    ui.separator();
                    for k in [Kind::Aux, Kind::Constant, Kind::Lookup] {
                        if k != v.kind
                            && ui
                                .button(format!("Change to {}", k.label().to_lowercase()))
                                .clicked()
                        {
                            self.checkpoint();
                            let var = self.model.var_mut(id).unwrap();
                            var.kind = k;
                            if k == Kind::Lookup && var.table.is_empty() {
                                var.table = vec![[0.0, 0.0], [0.5, 0.7], [1.0, 1.0]];
                            }
                            self.changed();
                            ui.close();
                        }
                    }
                }
                ui.separator();
                if ui.button("Delete").clicked() {
                    self.sel = Some(Sel::Var(id));
                    self.delete_selection();
                    ui.close();
                }
            }
            Some(Hit::Via(id, i)) => {
                if ui.button("Remove this turning point").clicked() {
                    self.checkpoint();
                    if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                        if i < l.via.len() {
                            l.via.remove(i);
                        }
                    }
                    ui.close();
                }
            }
            Some(Hit::Link(id)) | Some(Hit::Bend(id)) | Some(Hit::AddVia(id, _)) => {
                if self.model.links.iter().any(|l| l.id == id && !l.via.is_empty()) && ui.button("Remove all turning points").clicked() {
                    self.checkpoint();
                    if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                        l.via.clear();
                    }
                    ui.close();
                }
                for (p, label) in [
                    ("+", "Mark as reinforcing (+)"),
                    ("-", "Mark as opposing (−)"),
                    ("", "No polarity mark"),
                ] {
                    if ui.button(label).clicked() {
                        self.checkpoint();
                        if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                            l.polarity = p.into();
                        }
                        ui.close();
                    }
                }
                let cur = self
                    .model
                    .links
                    .iter()
                    .find(|l| l.id == id)
                    .map(|l| (l.delay_mark, l.dashed))
                    .unwrap_or_default();
                if ui
                    .button(if cur.0 {
                        "Remove delay mark"
                    } else {
                        "Add delay mark ||"
                    })
                    .clicked()
                {
                    self.checkpoint();
                    if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                        l.delay_mark = !l.delay_mark;
                    }
                    ui.close();
                }
                if ui
                    .button(if cur.1 { "Solid line" } else { "Dashed line" })
                    .clicked()
                {
                    self.checkpoint();
                    if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                        l.dashed = !l.dashed;
                    }
                    ui.close();
                }
                if ui.button("Straighten").clicked() {
                    self.checkpoint();
                    if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                        l.bend = 0.0;
                    }
                    ui.close();
                }
                if ui.button("Delete arrow").clicked() {
                    self.sel = Some(Sel::Link(id));
                    self.delete_selection();
                    ui.close();
                }
            }
            Some(Hit::Module(mid)) => {
                use crate::icons::{Ic, menu_item};
                let name = self.model.module(mid).map(|m| m.name.clone()).unwrap_or_default();
                ui.label(RichText::new(name).strong());
                if menu_item(ui, Ic::Back, "Open module", "double-click", true) {
                    self.open_module(mid);
                }
                let folded = self.model.module(mid).is_some_and(|m| m.collapsed);
                if menu_item(ui, if folded { Ic::ChevronDown } else { Ic::ChevronRight }, if folded { "Show inputs and outputs" } else { "Fold to a short card" }, "click the chevron", true) {
                    self.checkpoint();
                    if let Some(m) = self.model.modules.iter_mut().find(|m| m.id == mid) {
                        m.collapsed = !m.collapsed;
                    }
                    self.changed();
                    ui.close();
                }
                if menu_item(ui, Ic::Export, "Export module…", "", true) {
                    self.select_one(Some(Sel::Module(mid)));
                    self.export_module();
                }
                if menu_item(ui, Ic::Image, "Copy as picture", "", true) {
                    self.select_one(Some(Sel::Module(mid)));
                    self.request_export(crate::export::ExportWhat::Selection, crate::export::ExportTo::Clipboard);
                }
                ui.separator();
                if menu_item(ui, Ic::Ungroup, "Dissolve — put its parts back here", "", true) {
                    self.dissolve_module(mid);
                }
                if menu_item(ui, Ic::Trash, "Delete module and its parts", "", true) {
                    self.select_one(Some(Sel::Module(mid)));
                    self.delete_selection();
                }
            }
            Some(Hit::Control(id, _)) => {
                if ui.button("Delete control").clicked() {
                    self.sel = Some(Sel::Control(id));
                    self.delete_selection();
                    ui.close();
                }
            }
            Some(Hit::Shape(id)) => {
                if ui.button("Delete").clicked() {
                    self.sel = Some(Sel::Shape(id));
                    self.delete_selection();
                    ui.close();
                }
            }
            Some(Hit::Note(id)) => {
                if ui.button("Delete note").clicked() {
                    self.sel = Some(Sel::Note(id));
                    self.delete_selection();
                    ui.close();
                }
            }
            _ => {
                if crate::icons::menu_item(ui, crate::icons::Ic::Import, "Import module…", "", true) {
                    self.import_module();
                }
                if ui.button("Copy sheet as picture").clicked() {
                    self.request_export(
                        crate::export::ExportWhat::Sheet,
                        crate::export::ExportTo::Clipboard,
                    );
                    ui.close();
                }
                if ui.button("Zoom to fit").clicked() {
                    self.fit_pending = true;
                    ui.close();
                }
                if ui.button("Add arrows for every equation input").clicked() {
                    self.checkpoint();
                    let n = sync_links(&mut self.model);
                    self.status = format!("Added {n} arrow(s)");
                    ui.close();
                }
            }
        }
    }

    /// Plain click on an item: a grouped item selects its group first, a second click picks the item itself.
    fn click_select(&mut self, s: Sel, shift: bool) {
        if shift {
            self.toggle_sel(s);
            return;
        }
        match self.group_of(App::sel_id(s)) {
            Some(g) if self.sel != Some(Sel::Group(g)) && !self.multi.contains(&s) => {
                self.select_one(Some(Sel::Group(g)))
            }
            _ => self.select_one(Some(s)),
        }
    }

    fn handle_mouse(&mut self, ui: &egui::Ui, resp: &egui::Response, origin: Pos2) {
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let Some(pp) = pointer else { return };
        let d = self.to_diagram(origin, pp);
        let (shift, space) = ui.input(|i| (i.modifiers.shift, i.key_down(egui::Key::Space)));
        let other_button =
            ui.input(|i| i.pointer.middle_down() || (i.pointer.secondary_down() && resp.dragged()));

        if resp.drag_started() {
            let start =
                self.to_diagram(origin, ui.input(|i| i.pointer.press_origin()).unwrap_or(pp));
            let hit = self.hit(start);
            self.canvas.drag = if other_button || space {
                Some(Drag::Pan)
            } else {
                match (self.tool, hit) {
                    (Tool::Flow, Some(Hit::Var(id)))
                        if self.model.var(id).is_some_and(|v| v.kind == Kind::Stock) =>
                    {
                        Some(Drag::NewFlow {
                            from: Some(id),
                            start,
                        })
                    }
                    (Tool::Flow, None) => Some(Drag::NewFlow { from: None, start }),
                    (Tool::Shape, _) => Some(Drag::NewShape { start }),
                    (_, Some(Hit::Control(id, part))) if part >= 1 => {
                        self.checkpoint();
                        self.select_one(Some(Sel::Control(id)));
                        match part {
                            2 => self.step_control(id, false),
                            3 => self.step_control(id, true),
                            _ => self.operate_control(id, start),
                        }
                        Some(Drag::Control { id, part })
                    }
                    (_, Some(Hit::ShapeHandle(id, handle))) => {
                        self.checkpoint();
                        Some(Drag::ShapeHandle { id, handle })
                    }
                    (Tool::Link, Some(Hit::Var(id))) => Some(Drag::NewLink { from: id }),
                    (_, Some(Hit::Bend(id))) => {
                        self.checkpoint();
                        Some(Drag::Bend { id })
                    }
                    (_, Some(Hit::Via(id, idx))) => {
                        self.checkpoint();
                        Some(Drag::Via { id, idx })
                    }
                    (_, Some(Hit::Lever(id))) => {
                        self.checkpoint();
                        self.select_one(Some(Sel::Var(id)));
                        Some(Drag::Lever { id })
                    }
                    (_, Some(Hit::AddVia(id, idx))) => {
                        // drag a new turning point out of the middle of a stretch
                        self.checkpoint();
                        if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                            l.via.insert(idx.min(l.via.len()), [d.x, d.y]);
                        }
                        Some(Drag::Via { id, idx })
                    }
                    (_, Some(Hit::Cloud(id, to_end))) => {
                        self.checkpoint();
                        Some(Drag::Cloud { id, to_end })
                    }
                    (
                        _,
                        Some(h @ (Hit::Var(_) | Hit::Note(_) | Hit::Module(_) | Hit::GroupTab(_) | Hit::Shape(_) | Hit::Control(_, 0))),
                    ) => {
                        let s = hit_sel(h).unwrap_or(Sel::Var(0));
                        if let Hit::GroupTab(g) = h {
                            self.select_one(Some(Sel::Group(g)));
                        } else if !self.selected_items().contains(&s) {
                            self.click_select(s, shift);
                        }
                        self.checkpoint();
                        Some(Drag::Move)
                    }
                    (Tool::Select, None) | (Tool::Select, Some(Hit::Link(_))) => {
                        Some(Drag::Marquee { start })
                    }
                    _ => Some(Drag::Pan),
                }
            };
        }

        if resp.dragged() {
            let delta = resp.drag_delta() / self.zoom;
            match self.canvas.drag {
                Some(Drag::Pan) => self.pan += delta,
                Some(Drag::Move) => {
                    for s in self.selected_items() {
                        self.move_item(s, delta, false);
                    }
                }
                Some(Drag::ShapeHandle { id, handle }) => self.drag_shape_handle(id, handle, d),
                Some(Drag::Control { id, part: 1 }) => self.drag_control(id, d, delta),
                Some(Drag::Cloud { id, to_end }) => {
                    if let Some(v) = self.model.var_mut(id) {
                        let p = if to_end { &mut v.to_pt } else { &mut v.from_pt };
                        p[0] += delta.x;
                        p[1] += delta.y;
                    }
                }
                Some(Drag::Lever { id }) => {
                    // up raises, down lowers: the whole slider range over about 240 pixels
                    self.nudge_lever(id, -delta.y / 240.0);
                }
                Some(Drag::Via { id, idx }) => {
                    if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                        if let Some(v) = l.via.get_mut(idx) {
                            *v = [d.x, d.y];
                        }
                    }
                }
                Some(Drag::Bend { id }) => {
                    let l = self.model.links.iter().find(|l| l.id == id).cloned();
                    if let Some(l) = l {
                        let (a, b) = (self.center_of(l.from), self.center_of(l.to));
                        if let (Some(a), Some(b)) = (a, b) {
                            let dir = (b - a).normalized();
                            let perp = vec2(-dir.y, dir.x);
                            let mid = a + (b - a) * 0.5;
                            // the handle sits on the arc: its offset from the chord is the bend
                            let bend = (d - mid).dot(perp);
                            if let Some(l) = self.model.links.iter_mut().find(|x| x.id == id) {
                                l.bend = bend;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if resp.drag_stopped() {
            match self.canvas.drag.take() {
                Some(Drag::NewShape { start }) => self.create_shape(start, d),
                Some(Drag::ShapeHandle { .. }) => self.changed(),
                Some(Drag::Control { id, .. }) => self.settle_control(id),
                Some(Drag::NewFlow { from, start }) => self.finish_flow(from, start, d),
                Some(Drag::NewLink { from }) => {
                    if let Some(Hit::Var(to)) = self.hit(d) {
                        let to_kind = self.model.var(to).map(|v| v.kind);
                        if to_kind == Some(Kind::Stock) {
                            self.status = "Stocks change only through flows: draw a flow into the stock instead".into();
                        } else if to == from {
                            self.status = "An arrow needs two different variables".into();
                        } else {
                            self.checkpoint();
                            if self.model.add_link(from, to) {
                                if let (Some(a), Some(b)) =
                                    (self.center_of(from), self.center_of(to))
                                {
                                    if let Some(l) = self.model.links.last_mut() {
                                        l.bend = a.distance(b) * 0.12;
                                    }
                                }
                                let name = self
                                    .model
                                    .var(from)
                                    .map(|v| v.name.clone())
                                    .unwrap_or_default();
                                self.status = format!("Connected — use '{name}' in the equation");
                                self.select_one(Some(Sel::Var(to)));
                            }
                        }
                    }
                    self.tool = Tool::Select;
                }
                Some(Drag::Cloud { id, to_end }) => {
                    // dropping a cloud onto a stock attaches the flow to it
                    let view = self.diagram_view();
                    let target = self
                        .model
                        .vars
                        .iter()
                        .find(|s| {
                            s.kind == Kind::Stock
                                && s.view == view
                                && self.canvas.rects.get(&s.id).is_some_and(|r| r.contains(d))
                        })
                        .map(|s| s.id);
                    if let Some(sid) = target {
                        let other = self
                            .model
                            .var(id)
                            .and_then(|f| if to_end { f.from } else { f.to });
                        if other != Some(sid) {
                            let f = self.model.var_mut(id).unwrap();
                            if to_end {
                                f.to = Some(sid)
                            } else {
                                f.from = Some(sid)
                            }
                            self.status = "Flow connected to the stock".into();
                        }
                    }
                    self.changed();
                }
                Some(Drag::Move) => {
                    let items = self.selected_items();
                    self.snap_items(&items);
                }
                Some(Drag::Marquee { start }) => {
                    let area = Rect::from_two_pos(start, d);
                    let view = self.diagram_view();
                    let mut found: Vec<Sel> = Vec::new();
                    for v in self.model.vars.iter().filter(|v| v.view == view) {
                        if self
                            .canvas
                            .rects
                            .get(&v.id)
                            .is_some_and(|r| r.intersects(area))
                        {
                            found.push(Sel::Var(v.id));
                        }
                    }
                    for n in self.model.notes.iter().filter(|n| n.view == view) {
                        if self
                            .canvas
                            .note_rects
                            .get(&n.id)
                            .is_some_and(|r| r.intersects(area))
                        {
                            found.push(Sel::Note(n.id));
                        }
                    }
                    for s in self.model.shapes.iter().filter(|s| s.view == view) {
                        if self.shape_rect(s.id).is_some_and(|r| r.intersects(area)) {
                            found.push(Sel::Shape(s.id));
                        }
                    }
                    for k in self.model.controls.iter().filter(|k| k.view == view) {
                        if self.control_rect(k.id).is_some_and(|r| r.intersects(area)) {
                            found.push(Sel::Control(k.id));
                        }
                    }
                    for m in self.model.modules.iter().filter(|m| m.view == view) {
                        if self
                            .canvas
                            .rects
                            .get(&m.id)
                            .is_some_and(|r| r.intersects(area))
                        {
                            found.push(Sel::Module(m.id));
                        }
                    }
                    let mut items = if shift {
                        self.selected_items()
                    } else {
                        Vec::new()
                    };
                    for f in found {
                        if !items.contains(&f) {
                            items.push(f);
                        }
                    }
                    self.sel = items.last().copied();
                    self.multi = items;
                    if self.multi.len() > 1 {
                        self.status = format!(
                            "{} items selected — align, group or copy them from the bar above",
                            self.multi.len()
                        );
                    }
                }
                _ => {}
            }
        }

        if resp.clicked() {
            let hit = self.hit(d);
            match self.tool {
                Tool::Select | Tool::Link => match hit {
                    Some(Hit::Ghost(id)) => self.reveal_var(id),
                    Some(Hit::Fold(id)) => {
                        self.checkpoint();
                        if let Some(m) = self.model.modules.iter_mut().find(|m| m.id == id) {
                            m.collapsed = !m.collapsed;
                        }
                        self.changed();
                    }
                    Some(Hit::GroupTab(g)) => self.select_one(Some(Sel::Group(g))),
                    Some(Hit::Control(id, part)) if part >= 1 => {
                        self.checkpoint();
                        self.select_one(Some(Sel::Control(id)));
                        match part {
                            2 => self.step_control(id, false),
                            3 => self.step_control(id, true),
                            _ => self.operate_control(id, d),
                        }
                    }
                    Some(Hit::Link(id)) | Some(Hit::Bend(id)) => {
                        self.select_one(Some(Sel::Link(id)))
                    }
                    Some(h) => {
                        if let Some(s) = hit_sel(h) {
                            self.click_select(s, shift);
                        }
                    }
                    None => {
                        if !shift {
                            self.select_one(None);
                        }
                    }
                },
                Tool::Flow => self.finish_flow(None, d, d),
                Tool::Shape => self.create_shape(d, d),
                Tool::Control => self.create_control(d),
                Tool::Note => {
                    self.checkpoint();
                    let id = self.model.new_id();
                    let view = self.diagram_view();
                    self.model.notes.push(Note {
                        id,
                        text: "New note".into(),
                        pos: [d.x, d.y],
                        view,
                        fmt: Default::default(),
                    });
                    self.select_one(Some(Sel::Note(id)));
                    self.tool = Tool::Select;
                }
                t => {
                    let (kind, base) = match t {
                        Tool::Stock => (Kind::Stock, "Stock"),
                        Tool::Constant => (Kind::Constant, "Constant"),
                        Tool::Lookup => (Kind::Lookup, "Lookup"),
                        _ => (Kind::Aux, "Variable"),
                    };
                    self.checkpoint();
                    let id = self.model.new_id();
                    let name = self.model.unique_name(base);
                    let at = if self.snap {
                        [
                            (d.x / crate::arrange::GRID).round() * crate::arrange::GRID,
                            (d.y / crate::arrange::GRID).round() * crate::arrange::GRID,
                        ]
                    } else {
                        [d.x, d.y]
                    };
                    let mut v = Var::new(id, &name, kind, at);
                    v.view = self.diagram_view();
                    match kind {
                        Kind::Stock => v.equation = "0".into(),
                        Kind::Constant => {
                            v.equation = "1".into();
                            v.slider = Some([0.0, 10.0]);
                        }
                        Kind::Lookup => {
                            v.table = vec![
                                [0.0, 0.0],
                                [0.25, 0.45],
                                [0.5, 0.75],
                                [0.75, 0.92],
                                [1.0, 1.0],
                            ]
                        }
                        _ => {}
                    }
                    self.model.vars.push(v);
                    self.select_one(Some(Sel::Var(id)));
                    self.focus_name = true;
                    self.tool = Tool::Select;
                    self.changed();
                }
            }
        }
        if resp.double_clicked() {
            match self.hit(d) {
                Some(Hit::Var(id)) => {
                    self.select_one(Some(Sel::Var(id)));
                    self.canvas.focus_equation = true;
                }
                Some(Hit::Module(m)) => self.open_module(m),
                Some(Hit::Link(id)) | Some(Hit::Bend(id)) => {
                    // a new turning point where you double-clicked, kept in order along the arrow
                    self.checkpoint();
                    let path = self.canvas.link_paths.get(&id).cloned().unwrap_or_default();
                    let near = |q: Pos2| path.iter().enumerate().min_by(|a, b| a.1.distance(q).total_cmp(&b.1.distance(q))).map(|(i, _)| i).unwrap_or(0);
                    if let Some(l) = self.model.links.iter_mut().find(|l| l.id == id) {
                        let at = near(d);
                        let pos = l.via.iter().filter(|v| near(pos2(v[0], v[1])) < at).count();
                        l.via.insert(pos, [d.x, d.y]);
                    }
                    self.select_one(Some(Sel::Link(id)));
                    self.changed();
                }
                _ => {}
            }
        }
    }

    fn finish_flow(&mut self, from: Option<u64>, start: Pos2, end: Pos2) {
        let to = self
            .model
            .vars
            .iter()
            .find(|s| {
                s.kind == Kind::Stock
                    && self
                        .canvas
                        .rects
                        .get(&s.id)
                        .is_some_and(|r| r.contains(end))
            })
            .map(|s| s.id);
        if from.is_some() && from == to {
            self.status = "A flow needs two different ends".into();
            return;
        }
        let (mut a, mut b) = (start, end);
        if a.distance(b) < 30.0 && from.is_none() && to.is_none() {
            a = start - vec2(80.0, 0.0);
            b = start + vec2(80.0, 0.0);
        }
        self.checkpoint();
        let id = self.model.new_id();
        let name = self.model.unique_name("Flow");
        let mut v = Var::new(
            id,
            &name,
            Kind::Flow,
            [(a.x + b.x) / 2.0, (a.y + b.y) / 2.0],
        );
        v.view = self.diagram_view();
        v.from = from;
        v.to = to;
        v.from_pt = [a.x, a.y];
        v.to_pt = [b.x, b.y];
        v.equation = "1".into();
        self.model.vars.push(v);
        self.select_one(Some(Sel::Var(id)));
        self.focus_name = true;
        self.tool = Tool::Select;
        self.changed();
    }

    fn center_of(&self, id: u64) -> Option<Pos2> {
        if let Some(m) = self.model.module(id) {
            return Some(
                self.canvas
                    .rects
                    .get(&id)
                    .map(|r| r.center())
                    .unwrap_or(pos2(m.pos[0], m.pos[1])),
            );
        }
        let v = self.model.var(id)?;
        if v.kind == Kind::Flow {
            let (a, b) = flow_ends(&self.model, v);
            return Some(a + (b - a) * 0.5);
        }
        Some(
            self.canvas
                .rects
                .get(&id)
                .map(|r| r.center())
                .unwrap_or(pos2(v.pos[0], v.pos[1])),
        )
    }

    // ------------------------------------------------------------------ drawing

    fn draw(&mut self, ui: &egui::Ui, p: &egui::Painter, origin: Pos2) {
        let pal = self.pal;
        let z = self.zoom;
        let idx = self.time_index();
        let sel_var = match self.sel {
            Some(Sel::Var(id)) => Some(id),
            _ => None,
        };

        // flows: keep the valve halfway between the ends
        let view = self.diagram_view();
        self.straighten_flows(false);
        let ends: Vec<(u64, Pos2, Pos2)> = self
            .model
            .vars
            .iter()
            .filter(|v| v.kind == Kind::Flow && v.view == view && self.shown(v.id))
            .map(|v| {
                let (a, b) = flow_ends(&self.model, v);
                (v.id, a, b)
            })
            .collect();
        for (id, a, b) in &ends {
            let m = *a + (*b - *a) * 0.5;
            if let Some(v) = self.model.var_mut(*id) {
                v.pos = [m.x, m.y];
            }
        }

        // measure nodes
        let mut rects = HashMap::new();
        for v in &self.model.vars {
            let c = pos2(v.pos[0], v.pos[1]);
            let r = match v.kind {
                Kind::Stock => Rect::from_center_size(c, STOCK_SIZE),
                Kind::Flow => {
                    let w = (p
                        .layout_no_wrap(v.name.clone(), FontId::proportional(12.0), Color32::WHITE)
                        .size()
                        .x
                        + 8.0)
                        .max(30.0);
                    Rect::from_min_max(
                        pos2(c.x - w / 2.0, c.y - 12.0),
                        pos2(c.x + w / 2.0, c.y + 44.0),
                    )
                }
                _ => {
                    let w = p
                        .layout_no_wrap(v.name.clone(), FontId::proportional(13.0), Color32::WHITE)
                        .size()
                        .x;
                    Rect::from_center_size(c, vec2(w + 40.0, 40.0))
                }
            };
            rects.insert(v.id, r);
        }
        let mods: Vec<(u64, [f32; 2])> = self
            .model
            .modules
            .iter()
            .filter(|m| m.view == view && self.shown(m.id))
            .map(|m| (m.id, m.pos))
            .collect();
        let mut ports: HashMap<u64, (Vec<u64>, Vec<u64>)> = HashMap::new();
        for (id, pos) in &mods {
            let pr = self.module_ports(*id);
            let rows = pr.0.len().max(pr.1.len()).clamp(1, 6) as f32;
            let has_desc = self.model.module(*id).is_some_and(|m| !m.description.trim().is_empty());
            let folded = self.model.module_style == crate::model::ModuleStyle::Compact || self.model.module(*id).is_some_and(|m| m.collapsed);
            let size = if folded {
                vec2(MODULE_W + 40.0, 112.0)
            } else {
                // two columns: inputs on the left half, outputs on the right half
                vec2(MODULE_W + 160.0, 46.0 + rows.min(5.0) * 17.0 + if has_desc { 22.0 } else { 0.0 })
            };
            rects.insert(*id, Rect::from_center_size(pos2(pos[0], pos[1]), size));
            ports.insert(*id, pr);
        }
        self.canvas.rects = rects;

        // ---- shapes and pictures, behind everything
        self.draw_shapes(ui, p, origin);
        self.draw_controls(p, origin);

        // ---- group frames, behind everything
        let mut tabs = HashMap::new();
        for g in self
            .model
            .groups
            .iter()
            .filter(|g| g.view == view && self.export_only.is_none())
        {
            let r = g
                .members
                .iter()
                .filter_map(|m| {
                    self.canvas
                        .rects
                        .get(m)
                        .or(self.canvas.note_rects.get(m))
                        .copied()
                })
                .fold(Rect::NOTHING, |a, r| a.union(r));
            if !r.is_positive() {
                continue;
            }
            let r = r.expand(16.0);
            let sr =
                Rect::from_min_max(self.to_screen(origin, r.min), self.to_screen(origin, r.max));
            let col = g
                .color
                .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                .unwrap_or(pal.muted);
            let selected = self.sel == Some(Sel::Group(g.id));
            p.rect_filled(
                sr,
                CornerRadius::same(14),
                col.gamma_multiply(if pal.dark { 0.10 } else { 0.06 }),
            );
            p.rect_stroke(
                sr,
                CornerRadius::same(14),
                Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    if selected {
                        pal.accent
                    } else {
                        col.gamma_multiply(0.45)
                    },
                ),
                StrokeKind::Inside,
            );
            let g_txt = p.layout_no_wrap(
                g.name.clone(),
                FontId::proportional(12.0),
                if selected { pal.accent } else { col },
            );
            let tab = Rect::from_min_size(
                sr.left_top() + vec2(10.0, -11.0),
                g_txt.size() + vec2(16.0, 6.0),
            );
            p.rect_filled(tab, CornerRadius::same(8), pal.panel);
            p.rect_stroke(
                tab,
                CornerRadius::same(8),
                Stroke::new(
                    1.0,
                    if selected {
                        pal.accent
                    } else {
                        col.gamma_multiply(0.45)
                    },
                ),
                StrokeKind::Inside,
            );
            p.galley(tab.min + vec2(8.0, 3.0), g_txt, col);
            tabs.insert(
                g.id,
                Rect::from_min_max(
                    self.to_diagram(origin, tab.min),
                    self.to_diagram(origin, tab.max),
                ),
            );
        }
        self.canvas.group_tabs = tabs;

        // ---- links: circular arcs through both ends and the bend handle
        let mut paths = HashMap::new();
        let mut proxied_pairs: Vec<(u64, u64)> = Vec::new();
        // between compact modules an arrow says what passes along it
        let mut pass_labels: HashMap<(u64, u64), Vec<String>> = HashMap::new();
        if self.model.module_style == crate::model::ModuleStyle::Compact {
            for l in &self.model.links {
                if let (Some(a), Some(b)) = (self.proxy(l.from, view), self.proxy(l.to, view)) {
                    if (a != l.from || b != l.to) && a != b {
                        if let Some(n) = self.model.var(l.from).map(|v| v.name.clone()) {
                            let e = pass_labels.entry((a, b)).or_default();
                            if !e.contains(&n) {
                                e.push(n);
                            }
                        }
                    }
                }
            }
        }
        let many_links = self.model.links.iter().filter(|l| self.model.var(l.from).is_some_and(|v| v.view == view)).count() > 25;
        for l in &self.model.links {
            // links into or out of a module attach to its box
            let (Some(pf), Some(pt)) = (self.proxy(l.from, view), self.proxy(l.to, view)) else {
                continue;
            };
            if pf == pt || !self.shown(pf) || !self.shown(pt) {
                continue;
            }
            let proxied = pf != l.from || pt != l.to;
            if proxied {
                if proxied_pairs.contains(&(pf, pt)) {
                    continue;
                }
                proxied_pairs.push((pf, pt));
            }
            let (Some(a), Some(b)) = (self.center_of(pf), self.center_of(pt)) else {
                continue;
            };
            let ra = self.link_box(pf);
            let rb = self.link_box(pt);
            let bend = if proxied {
                a.distance(b) * 0.08
            } else {
                l.bend
            };
            let pts = if !l.via.is_empty() && !proxied {
                let mut cp = vec![a];
                cp.extend(l.via.iter().map(|v| pos2(v[0], v[1])));
                cp.push(b);
                theme::smooth_through(&cp, 24)
            } else {
                match l.shape.unwrap_or(self.model.arrow_shape) {
                crate::model::ArrowShape::Curved => theme::arc_points(a, b, bend, 64),
                crate::model::ArrowShape::Straight => theme::arc_points(a, b, 0.0, 64),
                crate::model::ArrowShape::Elbow => theme::elbow_points(a, b, 64),
                }
            };
            let s = pts.iter().position(|q| !ra.contains(*q)).unwrap_or(0);
            let e = pts
                .iter()
                .rposition(|q| !rb.contains(*q))
                .unwrap_or(pts.len() - 1);
            if e <= s + 1 {
                continue;
            }
            let path: Vec<Pos2> = pts[s..=e].to_vec();
            let selected = !proxied && self.sel == Some(Sel::Link(l.id));
            let classic = self.model.diagram_style == crate::model::DiagramStyle::Classic;
            let base = l
                .color
                .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                .unwrap_or(if classic { pal.text.gamma_multiply(0.8) } else { pal.link });
            let weight = if classic { [0.7, 0.9, 1.6][l.weight.min(2) as usize] } else { [0.9, 1.3, 2.2][l.weight.min(2) as usize] };
            let (col, w) = if selected {
                (pal.accent, weight + 0.8)
            } else if sel_var == Some(pt) || self.sel == Some(Sel::Module(pt)) {
                (pal.accent, weight + 0.5)
            } else if sel_var == Some(pf) || self.sel == Some(Sel::Module(pf)) {
                (pal.aux, weight + 0.5)
            } else if sel_var.is_some() {
                (base.gamma_multiply(if many_links { 0.14 } else { 0.45 }), weight)
            } else if many_links && !classic {
                // a crowded diagram: arrows stay quiet until you click something
                (base.gamma_multiply(0.38), weight * 0.8)
            } else {
                (base, weight)
            };
            let sp: Vec<Pos2> = path.iter().map(|q| self.to_screen(origin, *q)).collect();
            let tip = *sp.last().unwrap();
            // direction of the arc right at its end, so the head follows the curve
            let back = sp[sp.len().saturating_sub(3)];
            let dirv = (tip - back).normalized();
            let head = if classic { (5.5 + weight * 1.5) * z.sqrt() } else { (8.0 + weight * 2.0) * z.sqrt() };
            let mut line = sp.clone();
            if let Some(last) = line.last_mut() {
                *last = tip - dirv * head * 0.8;
            }
            let stroke = Stroke::new(w * z.sqrt(), col);
            if l.dashed {
                p.extend(egui::Shape::dashed_line(&line, stroke, 6.0 * z, 4.0 * z));
            } else {
                p.add(egui::Shape::line(line, stroke));
            }
            theme::link_head(p, l.head.unwrap_or(self.model.arrow_head), tip, dirv, head, col);
            let mid_i = sp.len() / 2;
            let mid = sp[mid_i];
            if proxied {
                if let Some(names) = pass_labels.get(&(pf, pt)) {
                    let mut text = names.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
                    if names.len() > 2 {
                        text.push_str(&format!(" +{}", names.len() - 2));
                    }
                    let g = p.layout_no_wrap(text, FontId::proportional(10.5 * z), pal.text);
                    let r = Rect::from_center_size(mid, g.size() + vec2(10.0, 4.0) * z);
                    p.rect_filled(r, CornerRadius::same((6.0 * z).min(255.0) as u8), pal.panel);
                    p.galley(r.min + vec2(5.0, 2.0) * z, g, pal.text);
                }
            }
            let tangent =
                (sp[(mid_i + 1).min(sp.len() - 1)] - sp[mid_i.saturating_sub(1)]).normalized();
            let normal = vec2(-tangent.y, tangent.x);
            if l.delay_mark {
                for off in [-3.0, 3.0] {
                    let c = mid + tangent * off * z;
                    p.line_segment(
                        [c - normal * 7.0 * z, c + normal * 7.0 * z],
                        Stroke::new(1.6 * z.sqrt(), col),
                    );
                }
            }
            if !l.polarity.is_empty() {
                // beside the arrowhead, on the inside of the curve
                let at_i = sp.len().saturating_sub(sp.len() / 7 + 2);
                let at = sp[at_i];
                let t2 =
                    (sp[(at_i + 1).min(sp.len() - 1)] - sp[at_i.saturating_sub(1)]).normalized();
                let n2 = vec2(-t2.y, t2.x);
                let side = if bend > 0.5 { -1.0 } else { 1.0 };
                let c = at + n2 * side * 10.0 * z;
                let pc = if l.polarity == "+" { pal.ok } else { pal.error };
                p.circle_filled(c, 7.0 * z, pal.panel);
                p.text(
                    c,
                    Align2::CENTER_CENTER,
                    if l.polarity == "+" { "+" } else { "−" },
                    FontId::proportional(13.0 * z),
                    pc,
                );
            }
            if selected {
                if l.via.is_empty() {
                    p.circle(mid, 5.5, pal.panel, Stroke::new(2.0, pal.accent));
                    p.circle_filled(mid, 2.0, pal.accent);
                } else {
                    let vias: Vec<Pos2> = l.via.iter().map(|v| pos2(v[0], v[1])).collect();
                    for h in via_handles(&path, &vias) {
                        let c = self.to_screen(origin, h);
                        p.circle(c, 4.0, pal.panel, Stroke::new(1.3, pal.accent.gamma_multiply(0.7)));
                    }
                    for v in vias {
                        let c = self.to_screen(origin, v);
                        p.circle(c, 6.0, pal.accent, Stroke::new(2.0, pal.panel));
                    }
                }
            }
            if !proxied {
                paths.insert(l.id, path);
            }
        }
        self.canvas.link_paths = paths;

        // ---- key levers: circles outside the module boxes, with arrows to the boxes that use them
        self.canvas.lever_nodes.clear();
        self.canvas.levers_shown = false;
        if self.export_only.is_none() {
            let layout = self.lever_layout(view);
            for (c, id) in layout {
                let Some(v) = self.model.var(id).cloned() else { continue };
                self.canvas.levers_shown = true;
                let r = 30.0;
                let classic = self.model.diagram_style == crate::model::DiagramStyle::Classic;
                let col = pal.constant;
                let selected = self.is_sel(Sel::Var(id));
                // arrows: to its own module and to every module that uses it
                let mut targets: Vec<u64> = Vec::new();
                if let Some(h) = self.proxy(id, view) {
                    targets.push(h);
                }
                for l in self.model.links.iter().filter(|l| l.from == id) {
                    if let Some(t) = self.proxy(l.to, view) {
                        if !targets.contains(&t) && self.model.module(t).is_some() {
                            targets.push(t);
                        }
                    }
                }
                for t in targets {
                    let Some(tr) = self.canvas.rects.get(&t).copied() else { continue };
                    let b = tr.center();
                    let pts = match self.model.arrow_shape {
                        crate::model::ArrowShape::Elbow => theme::elbow_points(c, b, 48),
                        crate::model::ArrowShape::Curved => theme::arc_points(c, b, c.distance(b) * 0.06, 48),
                        crate::model::ArrowShape::Straight => theme::arc_points(c, b, 0.0, 48),
                    };
                    let s0 = pts.iter().position(|q| q.distance(c) > r + 2.0).unwrap_or(0);
                    let e0 = pts.iter().position(|q| tr.expand(2.0).contains(*q)).unwrap_or(pts.len() - 1);
                    if e0 <= s0 + 1 {
                        continue;
                    }
                    let sp: Vec<Pos2> = pts[s0..=e0].iter().map(|q| self.to_screen(origin, *q)).collect();
                    let tip = *sp.last().unwrap();
                    let back = sp[sp.len().saturating_sub(3)];
                    let dir = (tip - back).normalized();
                    let head = if classic { 6.5 } else { 9.0 } * z.sqrt();
                    let mut line = sp.clone();
                    if let Some(last) = line.last_mut() {
                        *last = tip - dir * head * 0.8;
                    }
                    let acol = if classic { pal.text.gamma_multiply(0.8) } else { col.gamma_multiply(0.85) };
                    p.add(egui::Shape::line(line, Stroke::new((if classic { 1.0 } else { 1.6 }) * z.sqrt(), acol)));
                    theme::link_head(p, self.model.arrow_head, tip, dir, head, acol);
                }
                // the circle: name above, value inside
                let sc = self.to_screen(origin, c);
                self.canvas.lever_nodes.push((c, r, id));
                if selected {
                    p.circle_filled(sc, (r + 5.0) * z, pal.accent.gamma_multiply(0.18));
                }
                p.circle_filled(sc, r * z, pal.panel);
                p.circle_stroke(sc, r * z, Stroke::new(if selected { 2.6 } else { 1.8 } * z.sqrt(), if selected { pal.accent } else { col }));
                let focused = crate::sets::focus_label(&self.model, &v, &self.focus).is_some() || v.set.is_none();
                let value = if focused {
                    self.value_now(&v.name).map(theme::fmt_num).unwrap_or_else(|| "–".into())
                } else {
                    format!("×{:.2}", v.lever_scale)
                };
                p.text(sc, Align2::CENTER_CENTER, value, FontId::monospace(12.5 * z), col);
                p.text(sc - vec2(0.0, (r + 6.0) * z), Align2::CENTER_BOTTOM, &v.name, FontId::proportional(12.0 * z), pal.text);
                if let (Some(sn), false) = (&v.set, focused) {
                    p.text(sc + vec2(0.0, (r + 4.0) * z), Align2::CENTER_TOP, format!("all {sn} items"), FontId::proportional(9.5 * z), pal.muted);
                }
            }
        }

        // ---- flows (pipes, clouds, valves)
        for (id, a, b) in &ends {
            let v = self.model.var(*id).unwrap().clone();
            let from_here = if self.end_here(&v, v.from) {
                v.from
            } else {
                None
            };
            let to_here = if self.end_here(&v, v.to) { v.to } else { None };
            let a_clip = match from_here.and_then(|s| self.canvas.rects.get(&s)) {
                Some(r) => clip_rect(*r, *a, *b),
                None => *a + (*b - *a).normalized() * 16.0,
            };
            let b_clip = match to_here.and_then(|s| self.canvas.rects.get(&s)) {
                Some(r) => clip_rect(*r, *b, *a),
                None => *b - (*b - *a).normalized() * 18.0,
            };
            let (sa, sb) = (
                self.to_screen(origin, a_clip),
                self.to_screen(origin, b_clip),
            );
            let dirv = (sb - sa).normalized();
            let selected = self.is_sel(Sel::Var(v.id));
            let col = if selected { pal.accent } else { pal.flow };
            let pipe_end = sb - dirv * 12.0 * z;
            p.line_segment([sa, pipe_end], Stroke::new(9.0 * z, col));
            p.line_segment([sa, pipe_end], Stroke::new(5.0 * z, pal.panel));
            theme::arrow_head(p, sb, dirv, 16.0 * z, col);
            for (attached, here, pt) in [(v.from, from_here, *a), (v.to, to_here, *b)] {
                if here.is_none() {
                    let c = self.to_screen(origin, pt);
                    theme::cloud(p, c, 13.0 * z, pal.muted.gamma_multiply(0.8), pal.panel);
                    // the stock lives on another sheet: say which
                    if let Some(st) = attached.and_then(|id| self.model.var(id)) {
                        let sheet = self
                            .model
                            .sheets
                            .iter()
                            .find(|s| s.id == st.view)
                            .map(|s| s.name.clone())
                            .unwrap_or_default();
                        p.text(
                            c + vec2(0.0, 16.0 * z),
                            Align2::CENTER_TOP,
                            format!("{} · {sheet}", st.name),
                            FontId::proportional(10.5 * z),
                            pal.muted,
                        );
                    }
                }
            }
            let c = self.to_screen(origin, pos2(v.pos[0], v.pos[1]));
            if selected {
                p.circle_filled(c, 16.0 * z, pal.accent.gamma_multiply(0.18));
            }
            theme::valve(p, c, 9.0 * z, col, pal.panel);
            p.circle_filled(c, 2.5 * z, col);
            p.text(
                c + vec2(0.0, 14.0 * z),
                Align2::CENTER_TOP,
                &v.name,
                FontId::proportional(12.0 * z),
                pal.text,
            );
            if let Some(val) = self.value_now(&v.name) {
                p.text(
                    c + vec2(0.0, 29.0 * z),
                    Align2::CENTER_TOP,
                    theme::fmt_num(val),
                    FontId::monospace(10.5 * z),
                    col.gamma_multiply(0.9),
                );
            }
        }

        // ---- stocks
        for v in self
            .model
            .vars
            .iter()
            .filter(|v| v.kind == Kind::Stock && v.view == view && self.shown(v.id))
        {
            let r = self.canvas.rects[&v.id];
            let sr =
                Rect::from_min_max(self.to_screen(origin, r.min), self.to_screen(origin, r.max));
            let selected = self.is_sel(Sel::Var(v.id));
            self.set_stack(p, sr, v, z);
            p.rect_filled(
                sr.translate(vec2(0.0, 2.0 * z)),
                CornerRadius::same(10),
                Color32::from_black_alpha(if pal.dark { 60 } else { 14 }),
            );
            p.rect_filled(sr, CornerRadius::same((10.0 * z) as u8), pal.stock_fill);
            if let (Some(run), true) = (&self.run, true) {
                if let Some(vals) = run.get(&v.name) {
                    let sl = Rect::from_min_max(
                        sr.left_bottom() + vec2(8.0 * z, -26.0 * z),
                        sr.right_bottom() - vec2(8.0 * z, 6.0 * z),
                    );
                    theme::sparkline_upto(
                        p,
                        sl,
                        vals,
                        pal.stock.gamma_multiply(0.85),
                        idx,
                        true,
                        self.reveal_len(vals.len()),
                    );
                }
            }
            p.rect_stroke(
                sr,
                CornerRadius::same((10.0 * z) as u8),
                Stroke::new(
                    if selected { 2.5 } else { 1.6 } * z.sqrt(),
                    if selected { pal.accent } else { pal.stock },
                ),
                StrokeKind::Inside,
            );
            self.set_badge(p, sr, v, z);
            self.layer_badge(p, sr, v, z);
            let fs = if v.name.len() > 16 { 12.0 } else { 14.0 };
            p.text(
                sr.center_top() + vec2(0.0, 9.0 * z),
                Align2::CENTER_TOP,
                &v.name,
                FontId::proportional(fs * z),
                pal.text,
            );
            if let Some(val) = self.value_now(&v.name) {
                p.text(
                    sr.center_top() + vec2(0.0, 27.0 * z),
                    Align2::CENTER_TOP,
                    theme::fmt_num(val),
                    FontId::monospace(11.0 * z),
                    pal.stock,
                );
            }
        }

        // ---- variables, constants, lookups
        for v in self.model.vars.iter().filter(|v| {
            matches!(v.kind, Kind::Aux | Kind::Constant | Kind::Lookup)
                && v.view == view
                && self.shown(v.id)
        }) {
            let r = self.canvas.rects[&v.id];
            let sr =
                Rect::from_min_max(self.to_screen(origin, r.min), self.to_screen(origin, r.max));
            let col = pal.kind(v.kind);
            let classic = self.model.diagram_style == crate::model::DiagramStyle::Classic;
            let selected = self.is_sel(Sel::Var(v.id));
            let has_problem = self
                .problems
                .iter()
                .any(|pr| pr.var.as_deref() == Some(v.name.as_str()));
            let rr = CornerRadius::same((10.0 * z).min(255.0) as u8);
            if selected {
                p.rect_filled(
                    sr.expand(4.0 * z),
                    CornerRadius::same((13.0 * z) as u8),
                    pal.accent.gamma_multiply(0.15),
                );
            }
            if !classic {
                self.set_stack(p, sr, v, z);
                p.rect_filled(sr, rr, pal.panel);
            }
            let border = if selected {
                pal.accent
            } else if has_problem {
                pal.error
            } else {
                col.gamma_multiply(0.45)
            };
            if !classic || selected || has_problem {
                p.rect_stroke(
                    sr,
                    rr,
                    Stroke::new(if selected { 2.0 } else { 1.2 } * z.sqrt(), border),
                    StrokeKind::Inside,
                );
            }
            let icon_c = sr.left_center() + vec2(15.0 * z, 0.0);
            match v.kind {
                Kind::Aux => {
                    p.circle_filled(icon_c, 5.0 * z, col.gamma_multiply(0.25));
                    p.circle_filled(icon_c, 2.8 * z, col);
                }
                Kind::Constant => {
                    let s = 5.0 * z;
                    p.add(egui::Shape::convex_polygon(
                        vec![
                            icon_c + vec2(0.0, -s),
                            icon_c + vec2(s, 0.0),
                            icon_c + vec2(0.0, s),
                            icon_c + vec2(-s, 0.0),
                        ],
                        col,
                        Stroke::NONE,
                    ));
                }
                _ => {
                    let ir = Rect::from_center_size(icon_c, vec2(12.0, 10.0) * z);
                    let mut t = v.table.clone();
                    t.sort_by(|a, b| a[0].total_cmp(&b[0]));
                    let ys: Vec<f64> = t.iter().map(|q| q[1]).collect();
                    theme::sparkline(p, ir, &ys, col, None, false);
                }
            }
            let text_x = sr.left() + 26.0 * z;
            p.text(
                pos2(text_x, sr.top() + 6.0 * z),
                Align2::LEFT_TOP,
                &v.name,
                FontId::proportional(13.0 * z),
                pal.text,
            );
            self.set_badge(p, sr, v, z);
            self.layer_badge(p, sr, v, z);
            let sub = if v.kind == Kind::Lookup && v.equation.trim().is_empty() {
                format!("lookup · {} pts", v.table.len())
            } else if has_problem {
                "needs attention".into()
            } else if let (Some(_), Some(x)) = (&v.set, self.value_now(&v.name)) {
                if crate::sets::focus_label(&self.model, v, &self.focus).is_some() {
                    theme::fmt_num(x)
                } else {
                    let sign = if v.rollup == crate::model::Rollup::Average { "avg" } else { "Σ" };
                    format!("{sign} {}", theme::fmt_num(x))
                }
            } else if v.set.is_some() && v.kind != Kind::Constant {
                "–".into()
            } else if v.set.is_some() {
                "differs by item".into()
            } else {
                self.value_now(&v.name)
                    .map(theme::fmt_num)
                    .unwrap_or_else(|| "–".into())
            };
            p.text(
                pos2(text_x, sr.top() + 22.0 * z),
                Align2::LEFT_TOP,
                sub,
                FontId::monospace(10.0 * z),
                if has_problem { pal.error } else { col },
            );
        }

        // ---- notes
        let mut note_rects = HashMap::new();
        for n in self
            .model
            .notes
            .iter()
            .filter(|n| n.view == view && self.shown(n.id))
        {
            let f = &n.fmt;
            let tcol = f.color.map(|c| Color32::from_rgb(c[0], c[1], c[2])).unwrap_or(pal.muted);
            let mut job = egui::text::LayoutJob::single_section(
                n.text.clone(),
                egui::TextFormat {
                    font_id: if f.mono { FontId::monospace(f.points() * z) } else { FontId::proportional(f.points() * z) },
                    color: tcol,
                    italics: f.italic,
                    underline: if f.underline { Stroke::new((1.0 * z).max(0.5), tcol) } else { Stroke::NONE },
                    ..Default::default()
                },
            );
            job.wrap.max_width = 240.0 * z;
            job.halign = match f.align {
                crate::model::TextAlign::Left => egui::Align::LEFT,
                crate::model::TextAlign::Center => egui::Align::Center,
                crate::model::TextAlign::Right => egui::Align::RIGHT,
            };
            let g = p.layout_job(job);
            let sr =
                Rect::from_min_size(self.to_screen(origin, pos2(n.pos[0], n.pos[1])), g.rect.size())
                    .expand(8.0 * z);
            let selected = self.is_sel(Sel::Note(n.id));
            p.rect_filled(
                sr,
                CornerRadius::same(8),
                if let Some(c) = f.fill {
                    Color32::from_rgb(c[0], c[1], c[2])
                } else if pal.dark {
                    pal.surface
                } else {
                    Color32::from_rgb(255, 251, 235)
                },
            );
            p.rect_stroke(
                sr,
                CornerRadius::same(8),
                Stroke::new(1.0, if selected { pal.accent } else { pal.border }),
                StrokeKind::Inside,
            );
            // centred or right-aligned rows start left of the galley's origin
            let at = sr.min + vec2(8.0, 8.0) * z - g.rect.min.to_vec2();
            if f.bold {
                // egui's fonts have one weight: a second pass a hair to the right thickens the strokes
                p.galley(at + vec2(0.6 * z, 0.0), g.clone(), tcol);
            }
            p.galley(at, g, tcol);
            note_rects.insert(
                n.id,
                Rect::from_min_max(
                    self.to_diagram(origin, sr.min),
                    self.to_diagram(origin, sr.max),
                ),
            );
        }
        self.canvas.note_rects = note_rects;

        // ---- modules: a box with its inputs on the left and outputs on the right
        for (mid, _) in &mods {
            let Some(m) = self.model.module(*mid).cloned() else {
                continue;
            };
            let r = self.canvas.rects[mid];
            let sr =
                Rect::from_min_max(self.to_screen(origin, r.min), self.to_screen(origin, r.max));
            let selected = self.is_sel(Sel::Module(m.id));
            let compact = self.model.module_style == crate::model::ModuleStyle::Compact || m.collapsed;
            // what the module is for, from what crosses its boundary: it only gives (inputs),
            // only takes (results) or both (a mechanism)
            let (role, col) = match ports.get(mid) {
                Some((i, o)) if i.is_empty() && !o.is_empty() => ("INPUTS", pal.constant),
                Some((i, o)) if o.is_empty() && !i.is_empty() => ("RESULTS", pal.aux),
                _ => ("MECHANISM", pal.accent),
            };

            let cr = CornerRadius::same((12.0 * z).min(255.0) as u8);
            // stacked-card look: it holds more inside
            p.rect_filled(sr.translate(vec2(6.0, 6.0) * z), cr, pal.panel);
            p.rect_stroke(
                sr.translate(vec2(6.0, 6.0) * z),
                cr,
                Stroke::new(1.0, col.gamma_multiply(0.3)),
                StrokeKind::Inside,
            );
            p.rect_filled(sr, cr, pal.panel);
            let head = Rect::from_min_max(sr.min, pos2(sr.right(), sr.top() + 30.0 * z));
            p.rect_filled(
                head,
                CornerRadius {
                    nw: cr.nw,
                    ne: cr.ne,
                    sw: 0,
                    se: 0,
                },
                col.gamma_multiply(0.12),
            );
            p.rect_stroke(
                sr,
                cr,
                Stroke::new(
                    if selected { 2.5 } else { 1.4 } * z.sqrt(),
                    if selected {
                        pal.accent
                    } else {
                        col.gamma_multiply(0.7)
                    },
                ),
                StrokeKind::Inside,
            );
            let ic = head.left_center() + vec2(16.0 * z, 0.0);
            for k in [1.0, 0.0] {
                let q =
                    Rect::from_center_size(ic + vec2(k * 3.0, -k * 3.0) * z, vec2(11.0, 9.0) * z);
                p.rect_filled(q, CornerRadius::same(2), pal.panel);
                p.rect_stroke(
                    q,
                    CornerRadius::same(2),
                    Stroke::new(1.3, col),
                    StrokeKind::Middle,
                );
            }
            p.text(
                head.left_center() + vec2(30.0 * z, 0.0),
                Align2::LEFT_CENTER,
                &m.name,
                FontId::proportional(14.0 * z),
                pal.text,
            );
            // everything inside, however many levels down
            let parts = self.model.vars.iter().filter(|v| self.inside(v.id, m.id)).count();

            let (ins, outs) = ports.get(mid).cloned().unwrap_or_default();
            let name = |id: u64| {
                self.model
                    .var(id)
                    .map(|v| v.name.clone())
                    .unwrap_or_default()
            };
            // fold chevron: down while the ports are shown, right while folded
            crate::icons::paint(
                p,
                Rect::from_center_size(head.right_center() - vec2(14.0 * z, 0.0), vec2(14.0, 14.0) * z),
                if m.collapsed { crate::icons::Ic::ChevronRight } else { crate::icons::Ic::ChevronDown },
                col,
            );
            if compact {
                // the short card: role, what it is for, how big it is
                p.text(
                    head.right_center() - vec2(30.0 * z, 0.0),
                    Align2::RIGHT_CENTER,
                    role,
                    FontId::proportional(9.5 * z),
                    col,
                );
                let desc = if m.description.trim().is_empty() {
                    format!("{parts} parts · {} in, {} out", ins.len(), outs.len())
                } else {
                    m.description.clone()
                };
                let g = p.layout(desc, FontId::proportional(11.5 * z), pal.muted, sr.width() - 24.0 * z);
                p.galley(pos2(sr.left() + 12.0 * z, head.bottom() + 8.0 * z), g, pal.muted);
                p.text(
                    pos2(sr.left() + 12.0 * z, sr.bottom() - 8.0 * z),
                    Align2::LEFT_BOTTOM,
                    format!("{parts} parts"),
                    FontId::proportional(10.0 * z),
                    pal.muted,
                );
                continue;
            }
            // role, as in the compact card
            p.text(
                head.right_center() - vec2(30.0 * z, 0.0),
                Align2::RIGHT_CENTER,
                format!("{role} · {parts} parts"),
                FontId::proportional(9.5 * z),
                col,
            );
            let half = (sr.width() - 28.0 * z) / 2.0;
            let fit = |text: &str, font: FontId, max_w: f32| -> String {
                if p.layout_no_wrap(text.to_string(), font.clone(), pal.text).size().x <= max_w {
                    return text.to_string();
                }
                let mut t: Vec<char> = text.chars().collect();
                while t.len() > 1 {
                    t.pop();
                    let cut: String = t.iter().collect::<String>().trim_end().to_string() + "…";
                    if p.layout_no_wrap(cut.clone(), font.clone(), pal.text).size().x <= max_w {
                        return cut;
                    }
                }
                text.chars().take(1).collect()
            };
            let mut top = head.bottom();
            if !m.description.trim().is_empty() {
                let f = FontId::proportional(10.5 * z);
                p.text(
                    pos2(sr.left() + 12.0 * z, top + 10.0 * z),
                    Align2::LEFT_CENTER,
                    fit(&m.description, f.clone(), sr.width() - 24.0 * z),
                    f,
                    pal.muted,
                );
                top += 22.0 * z;
            }
            for (i, id) in ins.iter().take(5).enumerate() {
                let y = top + (12.0 + i as f32 * 17.0) * z;
                p.circle(pos2(sr.left(), y), 4.0 * z, pal.panel, Stroke::new(1.5, pal.ok));
                let f = FontId::proportional(11.0 * z);
                p.text(pos2(sr.left() + 10.0 * z, y), Align2::LEFT_CENTER, fit(&name(*id), f.clone(), half - 6.0 * z), f, pal.muted);
            }
            for (i, id) in outs.iter().take(5).enumerate() {
                let y = top + (12.0 + i as f32 * 17.0) * z;
                p.circle_filled(pos2(sr.right(), y), 4.0 * z, pal.aux);
                let val = self.value_now(&name(*id)).map(theme::fmt_num).unwrap_or_default();
                let f = FontId::proportional(11.0 * z);
                let vw = p.layout_no_wrap(format!(" {val}"), f.clone(), pal.text).size().x;
                p.text(
                    pos2(sr.right() - 10.0 * z, y),
                    Align2::RIGHT_CENTER,
                    format!("{} {val}", fit(&name(*id), f.clone(), (half - 6.0 * z - vw).max(20.0 * z))),
                    f,
                    pal.text,
                );
            }
            if ins.is_empty() && outs.is_empty() {
                p.text(
                    pos2(sr.center().x, head.bottom() + 14.0 * z),
                    Align2::CENTER_CENTER,
                    "no inputs or outputs",
                    FontId::proportional(11.0 * z),
                    pal.muted,
                );
            }
            let more = ins.len().max(outs.len());
            if more > 5 {
                p.text(
                    sr.center_bottom() - vec2(0.0, 4.0 * z),
                    Align2::CENTER_BOTTOM,
                    format!("+{} more", more - 5),
                    FontId::proportional(10.0 * z),
                    pal.muted,
                );
            }
        }

        // ---- inside a module: its inputs as ghosts on the left, outputs tagged
        let ghosts = if self.export_only.is_some() {
            Vec::new()
        } else {
            self.ghost_layout(view)
        };
        if let Some(mid) = self
            .model
            .sheets
            .iter()
            .find(|s| s.id == view)
            .and_then(|s| s.module)
        {
            let (_, outs) = self.module_ports(mid);
            for (g, src) in &ghosts {
                let Some(sv) = self.model.var(*src) else {
                    continue;
                };
                let sr = Rect::from_min_max(
                    self.to_screen(origin, g.min),
                    self.to_screen(origin, g.max),
                );
                p.rect_filled(sr, CornerRadius::same((9.0 * z) as u8), pal.surface);
                let outline = [
                    sr.left_top(),
                    sr.right_top(),
                    sr.right_bottom(),
                    sr.left_bottom(),
                    sr.left_top(),
                ];
                p.extend(egui::Shape::dashed_line(
                    &outline,
                    Stroke::new(1.2, pal.ok),
                    5.0,
                    3.0,
                ));
                p.text(
                    sr.left_top() + vec2(8.0, 2.0) * z,
                    Align2::LEFT_TOP,
                    "input",
                    FontId::proportional(9.0 * z),
                    pal.ok,
                );
                p.text(
                    sr.center() + vec2(0.0, 4.0 * z),
                    Align2::CENTER_CENTER,
                    &sv.name,
                    FontId::proportional(12.5 * z),
                    pal.muted,
                );
                // arrows to what uses it here
                for l in self.model.links.iter().filter(|l| l.from == *src) {
                    let Some(t) = self.proxy(l.to, view) else {
                        continue;
                    };
                    let (Some(b), rb) = (self.center_of(t), self.link_box(t)) else {
                        continue;
                    };
                    let pts = theme::arc_points(g.center(), b, g.center().distance(b) * 0.08, 48);
                    let s0 = pts
                        .iter()
                        .position(|q| !g.expand(2.0).contains(*q))
                        .unwrap_or(0);
                    let e0 = pts
                        .iter()
                        .rposition(|q| !rb.contains(*q))
                        .unwrap_or(pts.len() - 1);
                    if e0 <= s0 + 1 {
                        continue;
                    }
                    let sp: Vec<Pos2> = pts[s0..=e0]
                        .iter()
                        .map(|q| self.to_screen(origin, *q))
                        .collect();
                    let tip = *sp.last().unwrap();
                    let dirv = (tip - sp[sp.len() - 3]).normalized();
                    p.extend(egui::Shape::dashed_line(
                        &sp,
                        Stroke::new(1.2 * z.sqrt(), pal.ok.gamma_multiply(0.8)),
                        6.0 * z,
                        4.0 * z,
                    ));
                    theme::arrow_head_slim(
                        p,
                        tip,
                        dirv,
                        10.0 * z.sqrt(),
                        pal.ok.gamma_multiply(0.8),
                    );
                }
            }
            for o in outs.into_iter().filter(|o| self.shown(*o)) {
                if let Some(r) = self
                    .canvas
                    .rects
                    .get(&o)
                    .filter(|_| self.model.var(o).is_some_and(|v| v.view == view))
                {
                    let at = self.to_screen(origin, r.right_top());
                    let tag = Rect::from_min_size(at + vec2(-8.0, -10.0) * z, vec2(48.0, 16.0) * z);
                    p.rect_filled(tag, CornerRadius::same(8), pal.aux);
                    p.text(
                        tag.center(),
                        Align2::CENTER_CENTER,
                        "output",
                        FontId::proportional(9.5 * z),
                        Color32::WHITE,
                    );
                }
            }
        }
        self.canvas.ghosts = ghosts;

        // ---- in-progress drags
        self.draw_shape_handles(p, origin);
        let pointer = ui.input(|i| i.pointer.interact_pos());
        match (self.canvas.drag, pointer) {
            (Some(Drag::NewShape { start }), Some(pp)) => {
                let a = self.to_screen(origin, start);
                let st = Stroke::new(1.5, pal.accent);
                if self.shape_kind.is_line() {
                    p.line_segment([a, pp], st);
                } else if self.shape_kind == crate::model::ShapeKind::Ellipse {
                    let r = Rect::from_two_pos(a, pp);
                    let pts: Vec<Pos2> = (0..48).map(|i| { let t = i as f32 / 48.0 * std::f32::consts::TAU; pos2(r.center().x + r.width() / 2.0 * t.cos(), r.center().y + r.height() / 2.0 * t.sin()) }).collect();
                    p.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, st));
                } else {
                    p.rect_stroke(Rect::from_two_pos(a, pp), CornerRadius::ZERO, st, StrokeKind::Inside);
                }
            }
            (Some(Drag::NewFlow { start, .. }), Some(pp)) => {
                let a = self.to_screen(origin, start);
                p.line_segment([a, pp], Stroke::new(8.0 * z, pal.flow.gamma_multiply(0.5)));
                p.line_segment([a, pp], Stroke::new(4.0 * z, pal.canvas));
                theme::arrow_head(p, pp, (pp - a).normalized(), 14.0 * z, pal.flow);
            }
            (Some(Drag::NewLink { from }), Some(pp)) => {
                if let Some(c) = self.center_of(from) {
                    let a = self.to_screen(origin, c);
                    p.add(egui::Shape::dashed_line(
                        &[a, pp],
                        Stroke::new(1.6, pal.accent),
                        6.0,
                        4.0,
                    ));
                    theme::arrow_head(p, pp, (pp - a).normalized(), 10.0, pal.accent);
                }
            }
            (Some(Drag::Marquee { start }), Some(pp)) => {
                let r = Rect::from_two_pos(self.to_screen(origin, start), pp);
                p.rect_filled(r, CornerRadius::same(2), pal.accent.gamma_multiply(0.08));
                p.rect_stroke(
                    r,
                    CornerRadius::same(2),
                    Stroke::new(1.0, pal.accent),
                    StrokeKind::Inside,
                );
            }
            _ => {}
        }
        // a dashed outline around a multi-selection
        let items = self.selected_items();
        if items.len() > 1 {
            let b = items
                .iter()
                .filter_map(|s| self.item_rect(*s))
                .fold(Rect::NOTHING, |a, r| a.union(r));
            if b.is_positive() {
                let sr = Rect::from_min_max(
                    self.to_screen(origin, b.min),
                    self.to_screen(origin, b.max),
                )
                .expand(8.0);
                let o = [
                    sr.left_top(),
                    sr.right_top(),
                    sr.right_bottom(),
                    sr.left_bottom(),
                    sr.left_top(),
                ];
                p.extend(egui::Shape::dashed_line(
                    &o,
                    Stroke::new(1.0, pal.accent.gamma_multiply(0.7)),
                    5.0,
                    4.0,
                ));
            }
        }
    }

    /// The area an arrow should stop at, in diagram units.
    fn link_box(&self, id: u64) -> Rect {
        if self.model.module(id).is_some() {
            return self
                .canvas
                .rects
                .get(&id)
                .copied()
                .unwrap_or(Rect::NOTHING)
                .expand(3.0);
        }
        let Some(v) = self.model.var(id) else {
            return Rect::NOTHING;
        };
        if v.kind == Kind::Flow {
            // the valve plus its name underneath, so arrows never cross the label
            let valve = Rect::from_center_size(pos2(v.pos[0], v.pos[1]), vec2(24.0, 24.0));
            return self
                .canvas
                .rects
                .get(&id)
                .map(|r| r.union(valve))
                .unwrap_or(valve)
                .expand(2.0);
        }
        self.canvas
            .rects
            .get(&id)
            .copied()
            .unwrap_or(Rect::NOTHING)
            .expand(3.0)
    }

    // ------------------------------------------------------------------ overlays

    fn overlays(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let pal = self.pal;
        let card = egui::Frame::new()
            .fill(pal.panel)
            .stroke(Stroke::new(1.0, pal.border))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .shadow(egui::Shadow {
                offset: [0, 2],
                blur: 10,
                spread: 0,
                color: Color32::from_black_alpha(if pal.dark { 80 } else { 18 }),
            });

        // inside a module: where am I, and the way back up
        let view = self.diagram_view();
        let module_here = self
            .model
            .sheets
            .iter()
            .find(|s| s.id == view)
            .and_then(|s| s.module)
            .and_then(|m| self.model.module(m).cloned());
        let mut top = 12.0;
        if let Some(m) = &module_here {
            let parent = self
                .model
                .sheets
                .iter()
                .find(|s| s.id == m.view)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            let (ins, outs) = self.module_ports(m.id);
            egui::Area::new(ui.scope_id().with("crumb"))
                .fixed_pos(rect.left_top() + vec2(14.0, 12.0))
                .order(egui::Order::Middle)
                .show(ui.ctx(), |ui| {
                    card.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("‹ {parent}")).color(pal.accent),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("Back to the sheet that holds this module")
                                .clicked()
                            {
                                self.sheet = m.view;
                                self.select_one(Some(Sel::Module(m.id)));
                                self.fit_pending = true;
                            }
                            ui.label(RichText::new("/").color(pal.muted));
                            ui.label(RichText::new(format!("Module · {}", m.name)).strong());
                            ui.label(
                                RichText::new(format!(
                                    "{} input(s) · {} output(s)",
                                    ins.len(),
                                    outs.len()
                                ))
                                .small()
                                .color(pal.muted),
                            );
                        });
                    });
                });
            top += 44.0;
        }

        // the arrange bar for a multi-selection
        let n_sel = self.selected_items().len();
        if n_sel >= 2 && self.canvas.drag.is_none() {
            egui::Area::new(ui.scope_id().with("selbar")).fixed_pos(pos2(rect.center().x - 330.0, rect.top() + top)).order(egui::Order::Middle).show(ui.ctx(), |ui| {
                card.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{n_sel} selected")).strong().color(pal.accent));
                        ui.separator();
                        for (i, op) in crate::arrange::Align::ALL.into_iter().enumerate() {
                            if i == 3 || i == 6 {
                                ui.separator();
                            }
                            let enabled = if i >= 6 { n_sel >= 3 } else { true };
                            let (r, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                            if resp.hovered() && enabled {
                                ui.painter().rect_filled(r, CornerRadius::same(6), pal.accent_soft);
                            }
                            theme::draw_align(ui.painter(), r.shrink(6.0), op, if enabled { pal.text } else { pal.muted.gamma_multiply(0.5) }, pal.accent);
                            let tip = if op.shortcut().is_empty() { op.label().to_string() } else { format!("{}  {}", op.label(), op.shortcut()) };
                            if resp.on_hover_text(tip).clicked() && enabled {
                                self.align(op);
                            }
                        }
                        ui.separator();
                        use crate::icons::{Ic, tool};
                        if tool(ui, Ic::Group, "Group — select and move them together (⌘G)", pal.text, pal.accent).clicked() {
                            self.group_selection();
                        }
                        if tool(ui, Ic::Module, "Make a module — a submodel with inputs and outputs (⇧⌘M)", pal.text, pal.accent).clicked() {
                            self.make_module();
                        }
                        if tool(ui, Ic::Export, "Export these as a module file…", pal.text, pal.accent).clicked() {
                            self.export_module();
                        }
                        ui.separator();
                        if tool(ui, Ic::Copy, "Copy (⌘C) — paste on any diagram sheet, or as a picture in Word / PowerPoint", pal.text, pal.accent).clicked() {
                            if self.copy(false) {
                                self.request_export(crate::export::ExportWhat::Selection, crate::export::ExportTo::Clipboard);
                            }
                        }
                        if tool(ui, Ic::Image, "Copy only a transparent picture of the selection, for Office", pal.text, pal.accent).clicked() {
                            self.request_export(crate::export::ExportWhat::Selection, crate::export::ExportTo::Clipboard);
                        }
                        if tool(ui, Ic::Trash, "Delete (⌫)", pal.error, pal.error).clicked() {
                            self.delete_selection();
                        }
                    });
                });
            });
        }

        // tool hint
        if self.tool != Tool::Select {
            let (name, help, _) = self.tool.help();
            let area = egui::Area::new(ui.scope_id().with("hint"))
                .fixed_pos(rect.left_top() + vec2(14.0, top))
                .order(egui::Order::Middle);
            area.show(ui.ctx(), |ui| {
                card.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).strong().color(pal.accent));
                        ui.label(RichText::new(help).color(pal.muted));
                        ui.label(RichText::new("Esc to cancel").small().color(pal.muted));
                    });
                });
            });
        }
        if self.show_help {
            egui::Area::new(ui.scope_id().with("guide"))
                .fixed_pos(rect.left_top() + vec2(14.0, 56.0))
                .order(egui::Order::Middle)
                .show(ui.ctx(), |ui| {
                    card.show(ui, |ui| {
                        ui.set_max_width(300.0);
                        ui.label(RichText::new("Quick guide").strong());
                        for line in [
                            "Pick a tool on the left, then click the canvas to place it.",
                            "Flows: drag from a stock to another stock or to empty space.",
                            "Arrows: drag from a cause to the variable that uses it.",
                            "Select anything to edit its equation on the right.",
                            "Scroll to pan, pinch or ⌘-scroll to zoom, 0 fits everything.",
                            "Right-click for more options. ⌘Z undoes.",
                        ] {
                            ui.label(RichText::new(format!("•  {line}")).color(pal.muted));
                        }
                    });
                });
        }

        // time scrubber
        if let Some(run) = self.run.clone() {
            let n = run.times.len();
            if n > 1 {
                if self.canvas.playing {
                    let dt = ui.input(|i| i.stable_dt) as f64;
                    self.canvas.play_t += dt / 5.0;
                    if self.canvas.play_t >= 1.0 {
                        self.canvas.play_t = 1.0;
                        self.canvas.playing = false;
                    }
                    self.scrub = Some(((n - 1) as f64 * self.canvas.play_t).round() as usize);
                    ui.ctx().request_repaint();
                }
                egui::Area::new(ui.scope_id().with("scrub"))
                    .fixed_pos(pos2(rect.left() + 14.0, rect.bottom() - 52.0))
                    .order(egui::Order::Middle)
                    .show(ui.ctx(), |ui| {
                        card.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let ic = if self.canvas.playing { crate::icons::Ic::Pause } else { crate::icons::Ic::Play };
                                if crate::icons::tool(ui, ic, "Play the run on the diagram", pal.accent, pal.accent).clicked() {
                                    if !self.canvas.playing && self.time_index() == Some(n - 1) {
                                        self.canvas.play_t = 0.0;
                                    } else {
                                        self.canvas.play_t =
                                            self.time_index().unwrap_or(0) as f64 / (n - 1) as f64;
                                    }
                                    self.canvas.playing = !self.canvas.playing;
                                }
                                let mut i = self.time_index().unwrap_or(n - 1);
                                ui.spacing_mut().slider_width =
                                    (rect.width() * 0.32).clamp(140.0, 360.0);
                                if ui
                                    .add(egui::Slider::new(&mut i, 0..=n - 1).show_value(false))
                                    .changed()
                                {
                                    self.scrub = Some(i);
                                    self.canvas.playing = false;
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "Time {} {}",
                                        theme::fmt_num(run.times[i]),
                                        run.time_units
                                    ))
                                    .monospace()
                                    .color(pal.text),
                                );
                                if self.scrub.is_some()
                                    && crate::icons::tool(ui, crate::icons::Ic::Close, "Back to the final values", pal.muted, pal.accent).clicked()
                                {
                                    self.scrub = None;
                                    self.canvas.playing = false;
                                }
                            });
                        });
                    });
            }
        }

        // mini-map: the whole sheet in small, with the visible part outlined; click or drag to move there
        if self.show_minimap {
            let view = self.diagram_view();
            let mut items: Vec<(Rect, Color32)> = Vec::new();
            for v in self.model.vars.iter().filter(|v| v.view == view) {
                if let Some(r) = self.canvas.rects.get(&v.id) {
                    items.push((*r, pal.kind(v.kind)));
                }
            }
            for m in self.model.modules.iter().filter(|m| m.view == view) {
                if let Some(r) = self.canvas.rects.get(&m.id) {
                    items.push((*r, pal.accent));
                }
            }
            for n in self.model.notes.iter().filter(|n| n.view == view) {
                if let Some(r) = self.canvas.note_rects.get(&n.id) {
                    items.push((*r, pal.muted));
                }
            }
            let content = items.iter().fold(Rect::NOTHING, |a, (r, _)| a.union(*r));
            let visible = Rect::from_min_max(
                self.to_diagram(rect.min, rect.min),
                self.to_diagram(rect.min, rect.max),
            );
            // only when part of the sheet is out of view
            if content.is_positive() && items.len() > 2 && !visible.contains_rect(content) {
                let world = content.union(visible).expand(40.0);
                let size = vec2(196.0, 124.0);
                let at = pos2(
                    rect.right() - 14.0 - size.x - 8.0,
                    rect.bottom() - 62.0 - size.y - 12.0,
                );
                egui::Area::new(ui.scope_id().with("minimap"))
                    .fixed_pos(at)
                    .order(egui::Order::Middle)
                    .show(ui.ctx(), |ui| {
                        card.show(ui, |ui| {
                            let (r, resp) = ui.allocate_exact_size(
                                size - vec2(4.0, 0.0),
                                Sense::click_and_drag(),
                            );
                            let scale =
                                (r.width() / world.width()).min(r.height() / world.height());
                            let map = |q: Pos2| r.center() + (q - world.center()) * scale;
                            let p = ui.painter_at(r);
                            for (ir, c) in &items {
                                p.rect_filled(
                                    Rect::from_min_max(map(ir.min), map(ir.max)),
                                    CornerRadius::same(1),
                                    c.gamma_multiply(0.55),
                                );
                            }
                            let vr = Rect::from_min_max(map(visible.min), map(visible.max));
                            p.rect_filled(
                                vr,
                                CornerRadius::same(2),
                                pal.accent.gamma_multiply(0.08),
                            );
                            p.rect_stroke(
                                vr,
                                CornerRadius::same(2),
                                Stroke::new(1.5, pal.accent),
                                StrokeKind::Inside,
                            );
                            if resp.clicked() || resp.dragged() {
                                if let Some(pp) = resp.interact_pointer_pos() {
                                    let target = world.center() + (pp - r.center()) / scale;
                                    self.pan =
                                        (rect.center() - rect.min) / self.zoom - target.to_vec2();
                                }
                            }
                            resp.on_hover_text("Mini-map — click or drag to move around");
                        });
                    });
            }
        }

        // zoom controls
        egui::Area::new(ui.scope_id().with("zoom"))
            .fixed_pos(pos2(rect.right() - 170.0, rect.bottom() - 52.0))
            .order(egui::Order::Middle)
            .show(ui.ctx(), |ui| {
                card.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let c = rect.center();
                        let zoom_by = |app: &mut App, f: f32| {
                            let before = app.to_diagram(rect.min, c);
                            app.zoom = (app.zoom * f).clamp(0.2, 3.0);
                            let after = app.to_diagram(rect.min, c);
                            app.pan += after - before;
                        };
                        if crate::icons::tool(ui, crate::icons::Ic::Minus, "Zoom out", pal.text, pal.accent).clicked() {
                            zoom_by(self, 1.0 / 1.2);
                        }
                        ui.label(
                            RichText::new(format!("{:>3.0}%", self.zoom * 100.0))
                                .monospace()
                                .color(pal.muted),
                        );
                        if crate::icons::tool(ui, crate::icons::Ic::Plus, "Zoom in", pal.text, pal.accent).clicked() {
                            zoom_by(self, 1.2);
                        }
                        if crate::icons::tool(ui, crate::icons::Ic::Fit, "Zoom to fit (0)", pal.text, pal.accent).clicked() {
                            self.fit_pending = true;
                        }
                    });
                });
            });

        if self.model.vars.is_empty() {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Start by placing a stock — press S, then click here",
                FontId::proportional(16.0),
                pal.muted,
            );
        }
    }
}

impl App {
    /// A variable that exists once per item of a set is drawn as a small stack of cards.
    fn set_stack(&self, p: &egui::Painter, sr: Rect, v: &crate::model::Var, z: f32) {
        if v.set.is_none() {
            return;
        }
        let pal = self.pal;
        for k in [2.0f32, 1.0] {
            let r = sr.translate(vec2(5.0 * k * z, -5.0 * k * z));
            p.rect_filled(r, CornerRadius::same((10.0 * z).min(255.0) as u8), pal.panel);
            p.rect_stroke(r, CornerRadius::same((10.0 * z).min(255.0) as u8), Stroke::new(1.0 * z.sqrt(), pal.muted.gamma_multiply(0.55)), StrokeKind::Inside);
        }
    }

    /// "Tank ×57": which set a variable is per, and how many items it stands for.
    fn set_badge(&self, p: &egui::Painter, sr: Rect, v: &crate::model::Var, z: f32) {
        let Some(sn) = &v.set else { return };
        let n = self.model.sets.iter().find(|s| &s.name == sn).map(|s| s.items.len()).unwrap_or(0);
        let pal = self.pal;
        let text = match crate::sets::focus_label(&self.model, v, &self.focus) {
            Some(item) => item,
            None => format!("{sn} ×{n}"),
        };
        let g = p.layout_no_wrap(text, FontId::proportional(9.5 * z), pal.accent);
        let r = Rect::from_min_size(pos2(sr.right() - g.size().x - 10.0 * z, sr.top() - 7.0 * z), g.size() + vec2(8.0 * z, 3.0 * z));
        p.rect_filled(r, CornerRadius::same((6.0 * z).min(255.0) as u8), pal.panel);
        p.rect_stroke(r, CornerRadius::same((6.0 * z).min(255.0) as u8), Stroke::new(1.0, pal.accent.gamma_multiply(0.7)), StrokeKind::Inside);
        p.galley(r.min + vec2(4.0 * z, 1.5 * z), g, pal.accent);
    }
}

/// Where a new turning point can be dragged out: the middle of each stretch between the ends and
/// the existing turning points, measured along the drawn path.
fn via_handles(path: &[Pos2], vias: &[Pos2]) -> Vec<Pos2> {
    if path.len() < 2 {
        return Vec::new();
    }
    let mut idx = vec![0usize];
    for v in vias {
        let i = path.iter().enumerate().min_by(|a, b| a.1.distance(*v).total_cmp(&b.1.distance(*v))).map(|(i, _)| i).unwrap_or(0);
        idx.push(i);
    }
    idx.push(path.len() - 1);
    idx.windows(2).map(|w| path[(w[0] + w[1]) / 2]).collect()
}
