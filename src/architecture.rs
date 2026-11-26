//! The Architecture sheet: the purpose on top, then the layers as lanes, outermost first. Each lane
//! is as tall as its content needs and folds to one line; inside it the subsystems (groups) share
//! the width, each a dashed frame holding a miniature of its part of the diagram. Arrows between
//! lanes are the contracts. Every symbol says what a variable is: exogenous, lever, constant,
//! lookup, switch, loop member, or the KPI of an objective.
//!
//! It is drawn from the model (`dynamist_core::architecture`), so it is always current.

use std::collections::{HashMap, HashSet};

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, pos2, vec2};
use dynamist_core::architecture::{self, Architecture, Lane, Subsystem, VarInfo};
use dynamist_core::model::{Kind, Var};
use dynamist_core::ops::Op;

use crate::app::{App, Sel};

const TAB_W: f32 = 210.0;
const PAD: f32 = 24.0;
const GAP: f32 = 14.0;
const MIN_FRAME_W: f32 = 250.0;
const COLLAPSED_FRAME_W: f32 = 170.0;
const LANE_COLLAPSED_H: f32 = 46.0;
/// Every open lane starts this tall and grows only when its content needs more.
const NORMAL_LANE_H: f32 = 210.0;
const RED: Color32 = Color32::from_rgb(184, 50, 39);

#[derive(Default)]
pub struct State {
    pub collapsed_layers: HashSet<u64>,
    /// Group ids, or `OTHER_BIT | layer id` for the variables nobody grouped.
    pub collapsed_subs: HashSet<u64>,
    pub purpose_open: bool,
    pub bufs: HashMap<String, String>,
}

const OTHER_BIT: u64 = 1 << 40;
/// Vertical room for one contract's channel in the gap between two lanes.
const CHANNEL: f32 = 22.0;

/// How many contracts run through the gap below lane i.
fn gap_counts(a: &Architecture) -> Vec<usize> {
    let mut n = vec![0usize; a.lanes.len() + 1];
    for c in &a.contracts {
        let g = if c.to > c.from { c.from } else { c.from.saturating_sub(1) };
        n[g] += 1;
    }
    n
}

/// Which channel (0 = nearest the lane above) each contract of one gap runs in, chosen so that
/// as few lines as possible cross. Each item is (x at the source, x at the consumer, runs down?).
fn order_channels(items: &[(f32, f32, bool)]) -> Vec<usize> {
    let n = items.len();
    let cost = |ch: &[usize]| -> usize {
        let mut crossings = 0;
        for i in 0..n {
            let (a0, a1) = (items[i].0.min(items[i].1), items[i].0.max(items[i].1));
            for j in 0..n {
                if i == j {
                    continue;
                }
                // j's two vertical stubs: from the source to its channel, and from its channel to the consumer
                let (sx, dx, down) = items[j];
                let c = ch[j] as isize;
                let (ys, yd) = if down { ((-1, c), (c, isize::MAX)) } else { ((c, isize::MAX), (-1, c)) };
                for (x, (lo, hi)) in [(sx, ys), (dx, yd)] {
                    if x > a0 + 1.0 && x < a1 - 1.0 && (ch[i] as isize) > lo && (ch[i] as isize) < hi {
                        crossings += 1;
                    }
                }
            }
        }
        crossings
    };
    let mut best: Vec<usize> = (0..n).collect();
    if n > 1 && n <= 7 {
        let mut cur: Vec<usize> = (0..n).collect();
        let mut best_cost = cost(&best);
        permute(&mut cur, 0, &mut |p| {
            let c = cost(p);
            if c < best_cost {
                best_cost = c;
                best = p.to_vec();
            }
        });
    }
    best
}

fn permute(v: &mut Vec<usize>, k: usize, f: &mut dyn FnMut(&[usize])) {
    if k == v.len() {
        f(v);
        return;
    }
    for i in k..v.len() {
        v.swap(k, i);
        permute(v, k + 1, f);
        v.swap(k, i);
    }
}

fn sub_key(layer: u64, s: &Subsystem) -> u64 {
    s.id.unwrap_or(OTHER_BIT | layer)
}

/// Where a subsystem frame goes and where each of its variables lands.
struct Frame {
    sub: usize,
    rect: Rect,
    collapsed: bool,
    pos: HashMap<u64, Pos2>,
}

struct LaneLayout {
    rect: Rect,
    collapsed: bool,
    frames: Vec<Frame>,
}

fn kind_order(k: Kind) -> u8 {
    match k {
        Kind::Stock => 0,
        Kind::Flow => 1,
        Kind::Aux => 2,
        Kind::Constant => 3,
        Kind::Lookup => 4,
    }
}

impl App {
    /// The positions the variables of a subsystem would have in a frame of this size: the real
    /// diagram, scaled to fit. Returns the positions (relative to the frame's top-left) and the height it needs.
    fn place_vars(&self, vars: &[u64], w: f32) -> (HashMap<u64, Pos2>, f32) {
        let m = &self.model;
        let vs: Vec<&Var> = vars.iter().filter_map(|id| m.var(*id)).collect();
        if vs.is_empty() {
            return (HashMap::new(), 60.0);
        }
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        let mut grow = |p: [f32; 2]| {
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        };
        for v in &vs {
            grow(v.pos);
            if v.kind == Kind::Flow {
                if v.from.is_none() && v.from_pt != [0.0, 0.0] {
                    grow(v.from_pt);
                }
                if v.to.is_none() && v.to_pt != [0.0, 0.0] {
                    grow(v.to_pt);
                }
            }
        }
        let (bw, bh) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
        let inner_w = (w - 100.0).max(60.0);
        let s = (inner_w / bw).clamp(0.5, 1.6);
        let mut out = HashMap::new();
        for v in &vs {
            out.insert(v.id, pos2(50.0 + (v.pos[0] - x0) * s, 52.0 + (v.pos[1] - y0) * s));
        }
        // keep every symbol and its name clear of the others: push later ones down until nothing overlaps
        let width = |v: &Var| -> f32 { (v.name.chars().count() as f32 * 6.0 + 34.0).max(80.0) };
        let mut order: Vec<&Var> = vs.clone();
        order.sort_by(|a, b| out[&a.id].y.partial_cmp(&out[&b.id].y).unwrap().then(out[&a.id].x.partial_cmp(&out[&b.id].x).unwrap()));
        for _ in 0..40 {
            let mut moved = false;
            for i in 0..order.len() {
                for j in 0..i {
                    let (pi, pj) = (out[&order[i].id], out[&order[j].id]);
                    let need_dy = if order[j].kind == Kind::Flow || order[i].kind == Kind::Flow { 40.0 } else { 30.0 };
                    let need_dx = (width(order[i]) + width(order[j])) / 2.0;
                    if (pi.x - pj.x).abs() < need_dx && (pi.y - pj.y).abs() < need_dy {
                        out.get_mut(&order[i].id).unwrap().y = pj.y + need_dy;
                        moved = true;
                    }
                }
            }
            if !moved {
                break;
            }
        }
        let h = out.values().map(|p| p.y).fold(0.0, f32::max) + 46.0;
        let _ = bh;
        (out, h.max(96.0))
    }

    fn layout_lane(&self, lane: &Lane, x0: f32, y0: f32, w: f32) -> LaneLayout {
        let collapsed = self.arch.collapsed_layers.contains(&lane.layer.id);
        if collapsed {
            return LaneLayout { rect: Rect::from_min_size(pos2(x0, y0), vec2(w, LANE_COLLAPSED_H)), collapsed, frames: Vec::new() };
        }
        let area_x = x0 + TAB_W + 14.0;
        let area_w = w - TAB_W - 14.0 - 12.0;
        let n = lane.subsystems.len();
        let shut: Vec<bool> = lane.subsystems.iter().map(|s| self.arch.collapsed_subs.contains(&sub_key(lane.layer.id, s))).collect();
        // rows of frames
        let mut rows: Vec<Vec<usize>> = Vec::new();
        let mut used = 0.0;
        for i in 0..n {
            let need = if shut[i] { COLLAPSED_FRAME_W } else { MIN_FRAME_W };
            if rows.is_empty() || used + need + GAP > area_w {
                rows.push(Vec::new());
                used = 0.0;
            }
            rows.last_mut().unwrap().push(i);
            used += need + GAP;
        }
        let mut frames = Vec::new();
        let mut y = y0 + 16.0;
        for row in &rows {
            let weight = |i: usize| -> f32 {
                if shut[i] {
                    COLLAPSED_FRAME_W
                } else {
                    let bw = lane.subsystems[i].vars.iter().filter_map(|id| self.model.var(*id)).map(|v| v.pos[0]).fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(x), b.max(x)));
                    (bw.1 - bw.0).max(MIN_FRAME_W).min(900.0)
                }
            };
            let total: f32 = row.iter().map(|i| weight(*i)).sum();
            let free = area_w - GAP * (row.len() as f32 - 1.0);
            let mut x = area_x;
            let mut built: Vec<(usize, f32, HashMap<u64, Pos2>, f32)> = Vec::new();
            for &i in row {
                let fw = if shut[i] { COLLAPSED_FRAME_W } else { (free - shut_total(row, &shut)) * weight(i) / (total - shut_total(row, &shut)).max(1.0) };
                let fw = if shut[i] { fw } else { fw.max(MIN_FRAME_W) };
                let (pos, h) = if shut[i] { (HashMap::new(), 40.0) } else { self.place_vars(&lane.subsystems[i].vars, fw) };
                built.push((i, fw, pos, h));
            }
            let row_h = built.iter().map(|b| b.3).fold(NORMAL_LANE_H - 40.0, f32::max);
            for (i, fw, pos, _) in built {
                let rect = Rect::from_min_size(pos2(x, y), vec2(fw, row_h));
                let pos = pos.into_iter().map(|(k, p)| (k, rect.min + p.to_vec2())).collect();
                frames.push(Frame { sub: i, rect, collapsed: shut[i], pos });
                x += fw + GAP;
            }
            y += row_h + 18.0;
        }
        let h = (y - y0 + 6.0).max(NORMAL_LANE_H);
        LaneLayout { rect: Rect::from_min_size(pos2(x0, y0), vec2(w, h)), collapsed, frames }
    }

    /// One variable's symbol: its shape says its kind, its outline its role.
    fn arch_glyph(&self, p: &egui::Painter, c: Pos2, v: &Var, info: &VarInfo, col: Color32) {
        let pal = self.pal;
        let st = Stroke::new(1.8, col);
        match v.kind {
            Kind::Stock => {
                let r = Rect::from_center_size(c, vec2(78.0, 30.0));
                p.rect_filled(r, CornerRadius::same(5), pal.surface);
                p.rect_stroke(r, CornerRadius::same(5), Stroke::new(2.4, col), StrokeKind::Inside);
            }
            Kind::Flow => {
                p.circle(c, 7.0, pal.surface, st);
                p.line_segment([c + vec2(-3.5, -3.5), c + vec2(3.5, 3.5)], Stroke::new(1.4, col));
                p.line_segment([c + vec2(-3.5, 3.5), c + vec2(3.5, -3.5)], Stroke::new(1.4, col));
            }
            _ if info.lever => {
                let r = Rect::from_center_size(c, vec2(30.0, 9.0));
                p.rect_filled(r, CornerRadius::same(4), pal.surface);
                p.rect_stroke(r, CornerRadius::same(4), st, StrokeKind::Inside);
                p.circle_filled(c + vec2(6.0, 0.0), 6.0, col);
            }
            _ if info.switch => {
                let r = Rect::from_center_size(c, vec2(30.0, 14.0));
                p.rect_filled(r, CornerRadius::same(7), col.gamma_multiply(0.3));
                p.rect_stroke(r, CornerRadius::same(7), st, StrokeKind::Inside);
                p.circle(c + vec2(8.0, 0.0), 5.0, pal.surface, st);
            }
            Kind::Constant => {
                let d = 11.0;
                p.add(Shape::convex_polygon(vec![c + vec2(0.0, -d), c + vec2(d, 0.0), c + vec2(0.0, d), c + vec2(-d, 0.0)], pal.surface, st));
            }
            Kind::Lookup => {
                p.circle(c, 11.0, pal.surface, st);
                let pts: Vec<Pos2> = (0..=8).map(|i| {
                    let t = i as f32 / 8.0;
                    pos2(c.x - 6.0 + 12.0 * t, c.y + 4.0 - 8.0 * (t * std::f32::consts::PI).sin().max(0.0) * 0.5 - 4.0 * t)
                }).collect();
                p.add(Shape::line(pts, Stroke::new(1.5, col)));
            }
            _ => {
                if info.exogenous {
                    p.circle_filled(c, 11.0, pal.surface);
                    let pts: Vec<Pos2> = (0..=24).map(|i| c + vec2(11.0 * (i as f32 / 24.0 * std::f32::consts::TAU).cos(), 11.0 * (i as f32 / 24.0 * std::f32::consts::TAU).sin())).collect();
                    p.extend(Shape::dashed_line(&pts, st, 3.5, 2.5));
                } else {
                    p.circle(c, 11.0, pal.surface, st);
                }
            }
        }
        if let Some(l) = info.loops.first() {
            let t = c + vec2(if v.kind == Kind::Stock { 38.0 } else { 12.0 }, -13.0);
            p.circle_filled(t, 8.0, pal.text);
            p.text(t, Align2::CENTER_CENTER, l, FontId::proportional(8.0), pal.panel);
        }
        if let Some(k) = info.kpi {
            let t = c + vec2(if v.kind == Kind::Stock { 38.0 } else { 14.0 }, if info.loops.is_empty() { -13.0 } else { 6.0 });
            p.circle(t, 8.0, pal.surface, Stroke::new(1.8, RED));
            p.circle_filled(t, 2.6, RED);
            p.text(t + vec2(12.0, 0.0), Align2::LEFT_CENTER, k.to_string(), FontId::proportional(12.0), RED);
        }
    }

    pub fn architecture_sheet(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let a: Architecture = architecture::build(&self.model);
        ui.painter().rect_filled(ui.max_rect(), 0.0, pal.canvas);
        let mut toggle_layer: Option<u64> = None;
        let mut toggle_sub: Option<u64> = None;
        let mut select: Option<u64> = None;
        let mut edit_purpose = false;
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            let w = (ui.available_width() - 6.0).max(980.0);
            let origin = ui.cursor().min + vec2(PAD, 16.0);
            let lw = w - 2.0 * PAD;
            // layout pass
            let purpose_h = if self.model.purpose.is_some() { 196.0 } else { 56.0 };
            let mut y = origin.y + purpose_h + 20.0;
            let mut lanes: Vec<LaneLayout> = Vec::new();
            let gaps = gap_counts(&a);
            for (i, lane) in a.lanes.iter().enumerate() {
                let l = self.layout_lane(lane, origin.x, y, lw);
                y = l.rect.bottom() + 26.0 + CHANNEL * gaps.get(i).copied().unwrap_or(0) as f32;
                lanes.push(l);
            }
            let legend_y = y + 6.0;
            let total_h = legend_y + 90.0 + (a.loops.len().min(8) as f32) * 18.0 - ui.cursor().min.y;
            let (area, _) = ui.allocate_exact_size(vec2(w, total_h.max(200.0)), Sense::hover());
            let p = ui.painter_at(area);
            // purpose
            let pr = Rect::from_min_size(origin, vec2(lw, purpose_h));
            self.paint_purpose(ui, &p, pr, &mut edit_purpose);
            // lanes
            let mut vpos: HashMap<u64, Pos2> = HashMap::new();
            for (li, (lane, lay)) in a.lanes.iter().zip(&lanes).enumerate() {
                let col = self.layer_colour(li, &lane.layer);
                let r = lay.rect;
                p.rect_filled(r, CornerRadius::same(10), col.gamma_multiply(0.10));
                p.rect_stroke(r, CornerRadius::same(10), Stroke::new(1.4, col.gamma_multiply(0.55)), StrokeKind::Inside);
                let tab = Rect::from_min_size(r.min, vec2(TAB_W, r.height()));
                p.rect_filled(tab, CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, col);
                let head = lane.heading();
                let (num, title) = match head.split_once(": ") {
                    Some((n, t)) => (n.to_string(), t.to_string()),
                    None => (head.clone(), String::new()),
                };
                let white = Color32::WHITE;
                let id = ui.id().with(("lane-tab", lane.layer.id));
                if ui.interact(tab, id, Sense::click()).on_hover_text("Click to fold or unfold this layer").clicked() {
                    toggle_layer = Some(lane.layer.id);
                }
                p.text(r.min + vec2(14.0, 20.0), Align2::LEFT_CENTER, format!("{} {}", if lay.collapsed { "▸" } else { "▾" }, num), FontId::proportional(12.0), white.gamma_multiply(0.85));
                if !title.is_empty() {
                    p.text(r.min + vec2(14.0, 42.0), Align2::LEFT_CENTER, title, FontId::proportional(19.0), white);
                }
                // which objectives this layer serves: the ones measured by one of its variables
                let serves: Vec<String> = self.model.purpose.iter().flat_map(|pu| pu.objectives.iter().enumerate()).filter(|(_, o)| lane.subsystems.iter().flat_map(|s| &s.vars).filter_map(|id| self.model.var(*id)).any(|v| v.name == o.var)).map(|(i, _)| (i + 1).to_string()).collect();
                if !serves.is_empty() {
                    p.text(pos2(tab.right() - 12.0, r.top() + 20.0), Align2::RIGHT_CENTER, format!("serves {}", serves.join(", ")), FontId::proportional(10.5), white);
                }
                if lay.collapsed {
                    let chips: Vec<String> = lane.subsystems.iter().map(|s| s.name.clone()).collect();
                    p.text(pos2(tab.right() + 14.0, r.center().y), Align2::LEFT_CENTER, format!("{} variable(s) in {}", lane.var_count(), chips.join(", ")), FontId::proportional(12.5), pal.muted);
                    continue;
                }
                if !lane.layer.question.is_empty() {
                    let g = p.layout(lane.layer.question.clone(), FontId::proportional(12.5), white, TAB_W - 28.0);
                    p.galley(r.min + vec2(14.0, 62.0), g, white);
                }
                let chip_y = r.bottom() - 42.0;
                if chip_y > r.top() + 100.0 {
                    let h = lane.layer.horizon.map(|h| format!("Horizon {h} {}s", self.model.settings.time_units.to_lowercase().trim_end_matches('s'))).unwrap_or_else(|| "No horizon".into());
                    p.text(pos2(r.left() + 14.0, chip_y), Align2::LEFT_CENTER, h, FontId::proportional(11.0), white.gamma_multiply(0.9));
                    p.text(pos2(r.left() + 14.0, chip_y + 16.0), Align2::LEFT_CENTER, format!("{} · {} variables", lane.layer.status.label(), lane.var_count()), FontId::proportional(11.0), white.gamma_multiply(0.9));
                }
                for fr in &lay.frames {
                    let sub = &lane.subsystems[fr.sub];
                    let key = sub_key(lane.layer.id, sub);
                    p.rect_filled(fr.rect, CornerRadius::same(8), pal.panel.gamma_multiply(0.55));
                    let corners = [fr.rect.left_top(), fr.rect.right_top(), fr.rect.right_bottom(), fr.rect.left_bottom(), fr.rect.left_top()];
                    for s in corners.windows(2) {
                        p.extend(Shape::dashed_line(&[s[0], s[1]], Stroke::new(1.5, col), 6.0, 4.0));
                    }
                    let title = format!("{} {}   {}", if fr.collapsed { "▸" } else { "▾" }, sub.name, sub.vars.len());
                    let tw = title.chars().count() as f32 * 7.0 + 14.0;
                    let tr = Rect::from_min_size(fr.rect.min + vec2(8.0, -10.0), vec2(tw, 20.0));
                    p.rect_filled(tr, CornerRadius::same(10), pal.panel);
                    p.rect_stroke(tr, CornerRadius::same(10), Stroke::new(1.3, col), StrokeKind::Inside);
                    p.text(tr.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, title, FontId::proportional(11.5), col);
                    if ui.interact(tr, ui.id().with(("sub-tab", key)), Sense::click()).on_hover_text("Click to fold or unfold").clicked() {
                        toggle_sub = Some(key);
                    }
                    if fr.collapsed {
                        continue;
                    }
                    // pipes and links first, then the symbols on top
                    for &id in &sub.vars {
                        let Some(v) = self.model.var(id) else { continue };
                        if v.kind != Kind::Flow {
                            continue;
                        }
                        let here = fr.pos.get(&id).copied().unwrap_or(fr.rect.center());
                        let end = |s: Option<u64>, pt: [f32; 2], toward: Pos2| -> Pos2 {
                            match s.and_then(|sid| fr.pos.get(&sid).copied()) {
                                Some(q) => q,
                                None if s.is_some() => toward,
                                None => {
                                    let d = if pt == [0.0, 0.0] { vec2(-1.0, 0.0) } else { vec2(pt[0] - v.pos[0], pt[1] - v.pos[1]).normalized() };
                                    here + d * 46.0
                                }
                            }
                        };
                        let a0 = end(v.from, v.from_pt, here + vec2(-40.0, 0.0));
                        let a1 = end(v.to, v.to_pt, here + vec2(40.0, 0.0));
                        let pipe = Stroke::new(5.0, col.gamma_multiply(0.45));
                        p.line_segment([a0, here], pipe);
                        p.line_segment([here, a1], pipe);
                        if v.from.is_none() {
                            p.circle(a0, 6.0, pal.surface, Stroke::new(1.2, pal.muted));
                        }
                        if v.to.is_none() {
                            p.circle(a1, 6.0, pal.surface, Stroke::new(1.2, pal.muted));
                        }
                    }
                    for l in &self.model.links {
                        if let (Some(f), Some(t)) = (fr.pos.get(&l.from), fr.pos.get(&l.to)) {
                            p.extend(Shape::dashed_line(&[*f, *t], Stroke::new(1.0, pal.muted), 3.0, 3.0));
                        }
                    }
                    let mut ids = sub.vars.clone();
                    ids.sort_by_key(|id| self.model.var(*id).map(|v| kind_order(v.kind)).unwrap_or(9));
                    for id in ids {
                        let (Some(v), Some(c)) = (self.model.var(id), fr.pos.get(&id).copied()) else { continue };
                        let info = a.info.get(&id).cloned().unwrap_or_default();
                        self.arch_glyph(&p, c, v, &info, col);
                        let (anchor, at) = match v.kind {
                            Kind::Stock => (Align2::CENTER_CENTER, c),
                            Kind::Flow => (Align2::CENTER_TOP, c + vec2(0.0, 11.0)),
                            _ => (Align2::LEFT_CENTER, c + vec2(15.0, 0.0)),
                        };
                        let font = if v.kind == Kind::Stock { FontId::proportional(12.0) } else { FontId::proportional(11.0) };
                        p.text(at, anchor, &v.name, font, pal.text);
                        vpos.insert(id, c);
                        let hit = Rect::from_center_size(c, vec2(60.0, 30.0));
                        if ui.interact(hit, ui.id().with(("arch-var", id)), Sense::click()).on_hover_text(format!("{} = {}", v.name, v.equation)).clicked() {
                            select = Some(id);
                        }
                    }
                }
            }
            // contracts: out of the source, along a channel in the gap, into the consumer
            struct Route {
                gap: usize,
                src: Pos2,
                dst: Pos2,
                down: bool,
                label: String,
            }
            let mut routes: Vec<Route> = Vec::new();
            for c in &a.contracts {
                let Some(src) = c.var_id.and_then(|id| vpos.get(&id).copied()).or_else(|| lanes.get(c.from).map(|l| pos2(l.rect.right() - 80.0, l.rect.center().y))) else { continue };
                let target_var = c.used_by.iter().filter_map(|n| self.model.vars.iter().find(|v| v.name == *n)).find_map(|v| vpos.get(&v.id).copied());
                let dst = target_var.unwrap_or_else(|| pos2(lanes[c.to].rect.left() + TAB_W + 60.0, lanes[c.to].rect.center().y));
                let down = c.to > c.from;
                routes.push(Route { gap: if down { c.from } else { c.from - 1 }, src: src + vec2(0.0, if down { 14.0 } else { -14.0 }), dst: dst + vec2(0.0, if down { -14.0 } else { 14.0 }), down, label: c.var.clone() });
            }
            let mut by_gap: HashMap<usize, Vec<usize>> = HashMap::new();
            for (i, r) in routes.iter().enumerate() {
                by_gap.entry(r.gap).or_default().push(i);
            }
            let mut placed: Vec<(Pos2, Pos2, f32, String)> = Vec::new();
            for (g, ids) in by_gap {
                let top = lanes[g].rect.bottom();
                let bottom = lanes.get(g + 1).map(|l| l.rect.top()).unwrap_or(top + 30.0);
                let spans: Vec<(f32, f32, bool)> = ids.iter().map(|i| (routes[*i].src.x, routes[*i].dst.x, routes[*i].down)).collect();
                let order = order_channels(&spans);
                for (k, i) in ids.iter().enumerate() {
                    let y = (top + 16.0 + CHANNEL * order[k] as f32).min(bottom - 8.0);
                    let r = &routes[*i];
                    placed.push((r.src, r.dst, y, r.label.clone()));
                }
            }
            let stroke = Stroke::new(2.2, pal.text);
            for (src, dst, y, label) in &placed {
                p.line_segment([*src, pos2(src.x, *y)], stroke);
                p.line_segment([pos2(src.x, *y), pos2(dst.x, *y)], stroke);
                p.line_segment([pos2(dst.x, *y), *dst], stroke);
                let dir = (*dst - pos2(dst.x, *y)).normalized();
                let n = vec2(-dir.y, dir.x);
                p.add(Shape::convex_polygon(vec![*dst, *dst - dir * 9.0 + n * 4.5, *dst - dir * 9.0 - n * 4.5], pal.text, Stroke::NONE));
            }
            for (src, dst, y, label) in &placed {
                let tw = label.chars().count() as f32 * 6.4 + 16.0;
                let pr = Rect::from_center_size(pos2((src.x + dst.x) / 2.0, *y), vec2(tw, 17.0));
                p.rect_filled(pr, CornerRadius::same(9), pal.panel);
                p.rect_stroke(pr, CornerRadius::same(9), Stroke::new(1.1, pal.text), StrokeKind::Inside);
                p.text(pr.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), pal.text);
            }
            // legend
            let mut ly = legend_y;
            p.text(pos2(origin.x, ly), Align2::LEFT_CENTER, "What the symbols mean", FontId::proportional(12.0), pal.text);
            ly += 26.0;
            let samples = [("stock", Kind::Stock, VarInfo::default()), ("flow", Kind::Flow, VarInfo::default()), ("variable in a loop", Kind::Aux, VarInfo { loops: vec!["R".into()], ..Default::default() }), ("exogenous input", Kind::Aux, VarInfo { exogenous: true, ..Default::default() }), ("constant", Kind::Constant, VarInfo::default()), ("policy lever", Kind::Constant, VarInfo { lever: true, ..Default::default() }), ("lookup", Kind::Lookup, VarInfo::default()), ("switch", Kind::Constant, VarInfo { switch: true, ..Default::default() }), ("KPI of an objective", Kind::Aux, VarInfo { kpi: Some(1), ..Default::default() })];
            let mut x = origin.x + 40.0;
            for (label, kind, info) in samples {
                let mut v = Var::new(0, label, kind, [0.0, 0.0]);
                v.lever = info.lever;
                self.arch_glyph(&p, pos2(x, ly), &v, &VarInfo { loops: Vec::new(), kpi: None, ..info.clone() }, pal.muted);
                if !info.loops.is_empty() || info.kpi.is_some() {
                    let t = pos2(x + 12.0, ly - 13.0);
                    if info.kpi.is_some() {
                        p.circle(t, 7.0, pal.surface, Stroke::new(1.6, RED));
                        p.circle_filled(t, 2.4, RED);
                    } else {
                        p.circle_filled(t, 7.0, pal.text);
                    }
                }
                let tx = x + if kind == Kind::Stock { 48.0 } else { 20.0 };
                p.text(pos2(tx, ly), Align2::LEFT_CENTER, label, FontId::proportional(11.0), pal.muted);
                x = tx + label.chars().count() as f32 * 6.2 + 34.0;
            }
            ly += 30.0;
            for l in a.loops.iter().take(8) {
                p.text(pos2(origin.x, ly), Align2::LEFT_CENTER, format!("{}  {}  {}", l.label, if l.reinforcing { "reinforcing" } else { "balancing" }, l.vars.join(" → ")), FontId::proportional(11.5), pal.muted);
                ly += 18.0;
            }
        });
        if let Some(id) = select {
            self.sel = Some(Sel::Var(id));
            self.multi = vec![Sel::Var(id)];
        }
        if let Some(id) = toggle_layer {
            if !self.arch.collapsed_layers.remove(&id) {
                self.arch.collapsed_layers.insert(id);
            }
        }
        if let Some(id) = toggle_sub {
            if !self.arch.collapsed_subs.remove(&id) {
                self.arch.collapsed_subs.insert(id);
            }
        }
        if edit_purpose {
            self.arch.purpose_open = true;
        }
        let ctx = ui.ctx().clone();
        self.purpose_window(&ctx);
    }

    fn paint_purpose(&self, ui: &mut egui::Ui, p: &egui::Painter, r: Rect, edit: &mut bool) {
        let pal = self.pal;
        let dk = Color32::from_rgb(58, 63, 71);
        p.rect_filled(r, CornerRadius::same(10), dk.gamma_multiply(0.10));
        p.rect_stroke(r, CornerRadius::same(10), Stroke::new(1.4, dk.gamma_multiply(0.5)), StrokeKind::Inside);
        let tab = Rect::from_min_size(r.min, vec2(TAB_W, r.height()));
        p.rect_filled(tab, CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, dk);
        p.text(r.min + vec2(14.0, 24.0), Align2::LEFT_CENTER, "Purpose", FontId::proportional(19.0), Color32::WHITE);
        let g = p.layout("Why the model exists, and what counts as success.".into(), FontId::proportional(12.5), Color32::WHITE, TAB_W - 28.0);
        p.galley(r.min + vec2(14.0, 44.0), g, Color32::WHITE);
        let btn = Rect::from_min_size(r.min + vec2(14.0, r.height() - 38.0), vec2(120.0, 24.0));
        p.rect_filled(btn, CornerRadius::same(12), Color32::WHITE);
        p.text(btn.center(), Align2::CENTER_CENTER, if self.model.purpose.is_some() { "Edit purpose" } else { "Write purpose" }, FontId::proportional(12.0), dk);
        if ui.interact(btn, ui.id().with("purpose-edit"), Sense::click()).clicked() {
            *edit = true;
        }
        let Some(pu) = &self.model.purpose else {
            p.text(pos2(tab.right() + 16.0, r.center().y), Align2::LEFT_CENTER, "No purpose yet: say what problem the model is for, and how you will know it works.", FontId::proportional(13.0), pal.muted);
            return;
        };
        let x = tab.right() + 16.0;
        p.text(pos2(x, r.top() + 22.0), Align2::LEFT_CENTER, "PROBLEM", FontId::proportional(10.0), pal.muted);
        let g = p.layout(pu.problem.clone(), FontId::proportional(14.5), pal.text, 360.0);
        p.galley(pos2(x, r.top() + 34.0), g, pal.text);
        let rows = [("TYPE", pu.kind.clone()), ("OUT OF SCOPE", pu.out_of_scope.clone())];
        for (i, (k, v)) in rows.iter().enumerate() {
            let y = r.top() + 108.0 + i as f32 * 24.0;
            p.text(pos2(x, y), Align2::LEFT_CENTER, k, FontId::proportional(10.0), pal.muted);
            p.text(pos2(x + 92.0, y), Align2::LEFT_CENTER, if v.is_empty() { "-" } else { v.as_str() }, FontId::proportional(12.5), pal.text);
        }
        let levers: Vec<&str> = self.model.vars.iter().filter(|v| v.lever).map(|v| v.name.as_str()).collect();
        p.text(pos2(x, r.top() + 156.0), Align2::LEFT_CENTER, "LEVERS", FontId::proportional(10.0), pal.muted);
        p.text(pos2(x + 92.0, r.top() + 156.0), Align2::LEFT_CENTER, if levers.is_empty() { "none marked".to_string() } else { levers.join(", ") }, FontId::proportional(12.5), pal.text);
        let ox = x + 460.0;
        let cw = ((r.right() - ox - 12.0) / 2.0 - 8.0).clamp(220.0, 360.0);
        for (i, o) in pu.objectives.iter().enumerate().take(4) {
            let cr = Rect::from_min_size(pos2(ox + (cw + 10.0) * (i % 2) as f32, r.top() + 10.0 + 92.0 * (i / 2) as f32), vec2(cw, 84.0));
            if cr.right() > r.right() - 4.0 {
                break;
            }
            p.rect_filled(cr, CornerRadius::same(8), pal.panel);
            p.rect_stroke(cr, CornerRadius::same(8), Stroke::new(1.4, RED), StrokeKind::Inside);
            let t = cr.min + vec2(18.0, 18.0);
            p.circle(t, 8.0, pal.surface, Stroke::new(1.8, RED));
            p.circle_filled(t, 2.6, RED);
            p.text(t + vec2(14.0, 0.0), Align2::LEFT_CENTER, format!("{}  {}", i + 1, o.title), FontId::proportional(13.5), pal.text);
            p.text(cr.min + vec2(12.0, 38.0), Align2::LEFT_CENTER, if o.kind.is_empty() { String::new() } else { format!("{}{}", o.kind, if o.var.is_empty() { String::new() } else { format!("  ·  measured by {}", o.var) }) }, FontId::proportional(10.5), RED);
            let g = p.layout(o.test.clone(), FontId::proportional(11.5), pal.text, cw - 24.0);
            p.galley(cr.min + vec2(12.0, 50.0), g, pal.text);
        }
    }

    fn purpose_window(&mut self, ctx: &egui::Context) {
        if !self.arch.purpose_open {
            return;
        }
        let mut open = true;
        let mut ops: Vec<Op> = Vec::new();
        let pu = self.model.purpose.clone().unwrap_or_default();
        let mut bufs = std::mem::take(&mut self.arch.bufs);
        let pal = self.pal;
        egui::Window::new("Purpose").open(&mut open).default_width(560.0).show(ctx, |ui| {
            ui.label(RichText::new("Say why the model exists before drawing more of it. Each objective has a test anyone can check, and the variable that measures it.").small().color(pal.muted));
            let mut changed = false;
            let mut field = |ui: &mut egui::Ui, bufs: &mut HashMap<String, String>, key: &str, cur: &str, w: f32, hint: &str| -> Option<String> {
                let buf = bufs.entry(key.to_string()).or_insert_with(|| cur.to_string());
                let r = ui.add(egui::TextEdit::singleline(buf).desired_width(w).hint_text(hint));
                let out = (r.lost_focus() && buf.as_str() != cur).then(|| buf.clone());
                if !r.has_focus() {
                    *buf = cur.to_string();
                }
                out
            };
            let (mut problem, mut kind, mut scope) = (pu.problem.clone(), pu.kind.clone(), pu.out_of_scope.clone());
            ui.horizontal(|ui| {
                ui.label("Problem");
                if let Some(v) = field(ui, &mut bufs, "p:problem", &pu.problem, 440.0, "Can a young firm stay alive while it builds capacity?") {
                    problem = v;
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Type");
                if let Some(v) = field(ui, &mut bufs, "p:kind", &pu.kind, 240.0, "explain a behaviour, design a policy, forecast, learn") {
                    kind = v;
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Out of scope");
                if let Some(v) = field(ui, &mut bufs, "p:scope", &pu.out_of_scope, 400.0, "what is left out on purpose") {
                    scope = v;
                    changed = true;
                }
            });
            if changed {
                ops.push(Op::SetPurpose { problem, kind, out_of_scope: scope });
            }
            ui.separator();
            ui.label(RichText::new("Objectives").strong());
            for (i, o) in pu.objectives.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}  {}", i + 1, o.title)).strong());
                    ui.label(RichText::new(format!("{}  ·  {}", o.kind, o.test)).small().color(pal.muted));
                    if !o.var.is_empty() {
                        ui.label(RichText::new(format!("KPI: {}", o.var)).small().color(RED));
                    }
                    if ui.small_button("Remove").clicked() {
                        ops.push(Op::RemoveObjective { title: o.title.clone() });
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                for (k, w, h) in [("o:title", 110.0, "Title"), ("o:kind", 150.0, "grow, reach a target, stay within a band…"), ("o:test", 220.0, "the test, in words"), ("o:var", 100.0, "KPI variable")] {
                    let buf = bufs.entry(k.to_string()).or_default();
                    ui.add(egui::TextEdit::singleline(buf).desired_width(w).hint_text(h));
                }
                if ui.button("+ Add objective").clicked() {
                    let g = |k: &str| bufs.get(k).cloned().unwrap_or_default();
                    ops.push(Op::AddObjective { title: g("o:title"), kind: g("o:kind"), test: g("o:test"), var: g("o:var") });
                    for k in ["o:title", "o:kind", "o:test", "o:var"] {
                        bufs.insert(k.to_string(), String::new());
                    }
                }
            });
        });
        self.arch.bufs = bufs;
        self.layer_ops(ops);
        if !open {
            self.arch.purpose_open = false;
        }
    }
}

fn shut_total(row: &[usize], shut: &[bool]) -> f32 {
    row.iter().filter(|i| shut[**i]).count() as f32 * COLLAPSED_FRAME_W
}

#[cfg(test)]
mod tests {
    use super::*;
    use dynamist_core::model::Model;

    fn app() -> App {
        let ctx = egui::Context::default();
        let mut a = App::new(&ctx, None);
        let mut m = Model::default();
        m.settings.final_time = 20.0;
        m.settings.time_step = 1.0;
        m.ensure_sheets();
        let batch: Vec<Op> = serde_json::from_str(
            r#"[{"op":"add_var","name":"Firm","kind":"stock","equation":"10"},
                {"op":"add_var","name":"Climate","kind":"aux","equation":"1 + 0.02 * TIME"},
                {"op":"add_var","name":"Rate","kind":"constant","equation":"0.05"},
                {"op":"add_var","name":"Growth","kind":"flow","equation":"Firm * Rate * Climate","to":"Firm"},
                {"op":"add_layer","name":"World","order":0},
                {"op":"add_layer","name":"Firm layer","order":1},
                {"op":"assign_layer","layer":"World","vars":["Climate"]},
                {"op":"assign_layer","layer":"Firm layer","vars":["Firm","Rate","Growth"]},
                {"op":"add_group","name":"Money","vars":["Firm","Growth"]},
                {"op":"set_purpose","problem":"Does it grow?","kind":"explain a behaviour"},
                {"op":"add_objective","title":"Grow","kind":"grow","test":"Firm rises","var":"Firm"}]"#,
        )
        .unwrap();
        for op in &batch {
            dynamist_core::ops::apply(&mut m, op).unwrap();
        }
        a.finish_open(m);
        a
    }

    fn draw(a: &mut App) -> egui::FullOutput {
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1500.0, 1000.0))), ..Default::default() };
        ctx.run_ui(input, |ui| {
            a.architecture_sheet(ui);
        })
    }

    #[test]
    fn the_sheet_draws_and_lanes_fold() {
        let mut a = app();
        let out = draw(&mut a);
        assert!(!out.shapes.is_empty());
        let arch = architecture::build(&a.model);
        assert_eq!(arch.lanes.len(), 2);
        let open_h = a.layout_lane(&arch.lanes[1], 0.0, 0.0, 1200.0).rect.height();
        a.arch.collapsed_layers.insert(arch.lanes[1].layer.id);
        let shut_h = a.layout_lane(&arch.lanes[1], 0.0, 0.0, 1200.0).rect.height();
        assert!(shut_h < open_h && shut_h <= LANE_COLLAPSED_H + 0.5, "{shut_h} vs {open_h}");
        draw(&mut a);
        // a subsystem folds to a short tile
        let money = arch.lanes[1].subsystems.iter().find(|s| s.name == "Money").unwrap().id.unwrap();
        a.arch.collapsed_layers.clear();
        let before = a.layout_lane(&arch.lanes[1], 0.0, 0.0, 1200.0);
        a.arch.collapsed_subs.insert(money);
        let after = a.layout_lane(&arch.lanes[1], 0.0, 0.0, 1200.0);
        assert!(after.frames.iter().any(|f| f.collapsed));
        assert!(after.rect.height() <= before.rect.height());
    }

    #[test]
    fn a_lane_is_as_tall_as_its_content() {
        let mut a = app();
        let arch = architecture::build(&a.model);
        let world = a.layout_lane(&arch.lanes[0], 0.0, 0.0, 1200.0).rect.height();
        assert_eq!(world, NORMAL_LANE_H, "a lane with little in it keeps the normal height");
        assert_eq!(a.layout_lane(&arch.lanes[1], 0.0, 0.0, 1200.0).rect.height(), NORMAL_LANE_H, "and so does the next, until its content needs more");
        // give the second lane's group a tall diagram: the lane grows, the first does not
        for v in a.model.vars.iter_mut().filter(|v| v.name == "Firm") {
            v.pos = [100.0, 900.0];
        }
        let firm = a.layout_lane(&arch.lanes[1], 0.0, 0.0, 1200.0).rect.height();
        assert!(firm > world, "{firm} vs {world}");
    }

    /// Draws the venture model's architecture to a PNG (run on request: `ARCH_PNG=/path cargo test -- --ignored render_venture`).
    #[test]
    #[ignore]
    fn render_venture_png() {
        let Ok(path) = std::env::var("ARCH_PNG") else { return };
        let vars = [
            ("Capital Climate", "aux", "0.6 + 0.005 * TIME", "", "", "World"),
            ("Capital Pool", "stock", "100", "", "", "Capital"),
            ("Investment", "flow", "Capital Climate * Traction * 5", "", "Capital Pool", "Capital"),
            ("Funds Available", "aux", "Capital Pool * 0.02", "", "", "Capital"),
            ("Cash", "stock", "50", "", "", "Firm"),
            ("Funding", "flow", "Funds Available", "", "Cash", "Firm"),
            ("Revenue", "flow", "Units Sold * Price", "", "Cash", "Firm"),
            ("Spend", "flow", "Fixed Cost", "Cash", "", "Firm"),
            ("Price", "constant", "2", "", "", "Firm"),
            ("Fixed Cost", "constant", "10", "", "", "Firm"),
            ("Traction", "aux", "Cash / (Cash + 100)", "", "", "Firm"),
            ("Capacity", "stock", "10", "", "", "Operations"),
            ("Capacity Growth", "flow", "Cash * 0.001", "", "Capacity", "Operations"),
            ("Capacity Decay", "flow", "Capacity * 0.05", "Capacity", "", "Operations"),
            ("Units Sold", "aux", "Capacity * 0.8", "", "", "Operations"),
        ];
        let q = ["Which outside climate do we live in?", "How much money is around, and who gets it?", "Can the firm stay alive?", "How much can we make and sell?"];
        let mut ops: Vec<serde_json::Value> = vec![serde_json::json!({"op": "set_setting", "key": "final_time", "value": "60"}), serde_json::json!({"op": "set_setting", "key": "time_units", "value": "Month"})];
        for (i, n) in ["World", "Capital", "Firm", "Operations"].iter().enumerate() {
            ops.push(serde_json::json!({"op": "add_layer", "name": n, "order": i, "horizon": 60, "status": "built", "question": q[i]}));
        }
        for pass in ["stock", "other"] {
            for (n, k, e, from, to, layer) in vars {
                if (k == "stock") != (pass == "stock") {
                    continue;
                }
                let mut o = serde_json::json!({"op": "add_var", "name": n, "kind": k, "equation": e, "layer": layer});
                if !from.is_empty() {
                    o["from"] = from.into();
                }
                if !to.is_empty() {
                    o["to"] = to.into();
                }
                ops.push(o);
            }
        }
        for (g, vs) in [("Investment pool", vec!["Capital Pool", "Investment", "Funds Available"]), ("Cash account", vec!["Cash", "Funding", "Revenue", "Spend"]), ("Constants", vec!["Price", "Fixed Cost"]), ("Capacity build", vec!["Capacity", "Capacity Growth", "Capacity Decay"])] {
            ops.push(serde_json::json!({"op": "add_group", "name": g, "vars": vs}));
        }
        ops.push(serde_json::json!({"op": "set_lever", "var": "Price", "on": true}));
        ops.push(serde_json::json!({"op": "set_purpose", "problem": "Can a young firm stay alive while it builds capacity?", "kind": "design a policy", "out_of_scope": "competitors, regulation"}));
        ops.push(serde_json::json!({"op": "add_objective", "title": "Stay alive", "kind": "stay within a band", "test": "Cash stays above zero in every month of the 60.", "var": "Cash"}));
        ops.push(serde_json::json!({"op": "add_objective", "title": "Grow", "kind": "grow to a plateau", "test": "Capacity rises, then levels off, by month 60.", "var": "Capacity"}));
        let ops: Vec<Op> = serde_json::from_value(serde_json::Value::Array(ops)).unwrap();
        let mut m = Model::default();
        m.ensure_sheets();
        for op in &ops {
            dynamist_core::ops::apply(&mut m, op).unwrap();
        }
        let view = m.sheets.iter().find(|s| s.kind == dynamist_core::model::SheetKind::Diagram).unwrap().id;
        crate::spreadsheet::tidy(&mut m, view);
        let id = m.new_id();
        m.sheets.push(dynamist_core::model::Sheet::new(id, "Architecture", dynamist_core::model::SheetKind::Architecture));
        let mut h = crate::headless::Headless::new(m, None);
        assert!(h.show_sheet("Architecture"));
        let png = h.window_png(None, (1700.0, 1500.0), 1.0).unwrap();
        std::fs::write(path, png).unwrap();
    }

    #[test]
    fn channels_are_ordered_so_that_lines_do_not_cross() {
        // two contracts down the same gap whose spans overlap: only one order keeps them apart
        let ch = order_channels(&[(100.0, 500.0, true), (300.0, 700.0, true)]);
        assert!(ch[1] < ch[0], "the one that starts inside the other's span runs nearer the top: {ch:?}");
        // going up, the opposite
        let ch = order_channels(&[(100.0, 500.0, false), (300.0, 700.0, false)]);
        assert!(ch[0] < ch[1], "{ch:?}");
        // disjoint spans: any order is fine, and nothing breaks
        assert_eq!(order_channels(&[(0.0, 50.0, true), (100.0, 150.0, true)]).len(), 2);
    }

    #[test]
    fn symbols_in_a_frame_never_sit_on_each_other() {
        let mut a = app();
        for v in a.model.vars.iter_mut() {
            v.pos = [100.0, 100.0];
        }
        let ids: Vec<u64> = a.model.vars.iter().map(|v| v.id).collect();
        let (pos, _) = a.place_vars(&ids, 400.0);
        let pts: Vec<Pos2> = pos.values().copied().collect();
        for i in 0..pts.len() {
            for j in 0..i {
                assert!((pts[i].x - pts[j].x).abs() > 20.0 || (pts[i].y - pts[j].y).abs() > 20.0, "{:?} {:?}", pts[i], pts[j]);
            }
        }
    }
}
