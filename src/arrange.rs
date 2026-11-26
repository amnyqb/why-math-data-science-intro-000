//! Editing many things at once: multi-selection, copy & paste (also across sheets),
//! groups, modules (submodels with inputs and outputs), alignment and distribution,
//! nudging and snapping to the grid.

use std::collections::HashMap;

use egui::{Rect, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};

use crate::app::{App, Sel};
use crate::expr;
use crate::model::{CldLink, CldNode, Group, Kind, Link, Module, Note, Sheet, SheetKind, Var};

pub const GRID: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    CenterH,
    Right,
    Top,
    Middle,
    Bottom,
    DistributeH,
    DistributeV,
}

impl Align {
    pub const ALL: [Align; 8] = [
        Align::Left,
        Align::CenterH,
        Align::Right,
        Align::Top,
        Align::Middle,
        Align::Bottom,
        Align::DistributeH,
        Align::DistributeV,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Align::Left => "Align left",
            Align::CenterH => "Align centers (vertical line)",
            Align::Right => "Align right",
            Align::Top => "Align top",
            Align::Middle => "Align middles (horizontal line)",
            Align::Bottom => "Align bottom",
            Align::DistributeH => "Distribute horizontally",
            Align::DistributeV => "Distribute vertically",
        }
    }
    pub fn shortcut(self) -> &'static str {
        match self {
            Align::Left => "⌥⌘←",
            Align::Right => "⌥⌘→",
            Align::Top => "⌥⌘↑",
            Align::Bottom => "⌥⌘↓",
            _ => "",
        }
    }
}

/// What Copy puts aside for Paste.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Clip {
    pub vars: Vec<Var>,
    pub links: Vec<Link>,
    pub notes: Vec<Note>,
    pub shapes: Vec<crate::model::Shape>,
    pub controls: Vec<crate::model::Control>,
    /// The picture files those shapes show.
    pub images: Vec<crate::model::ImageAsset>,
    /// The sheet it was copied from, to offset pastes onto the same sheet.
    pub from_view: u64,
}

fn snap(v: f32) -> f32 {
    (v / GRID).round() * GRID
}

impl App {
    // ------------------------------------------------------------------ selection

    pub fn is_sel(&self, s: Sel) -> bool {
        self.sel == Some(s) || self.multi.contains(&s)
    }

    /// Everything selected that can be moved: variables, notes and modules on this sheet.
    pub fn selected_items(&self) -> Vec<Sel> {
        let view = self.diagram_view();
        let mut out: Vec<Sel> = Vec::new();
        let mut push = |s: Sel| {
            if !out.contains(&s) {
                out.push(s);
            }
        };
        for s in self.multi.iter().copied().chain(self.sel) {
            match s {
                Sel::Var(id) if self.model.var(id).is_some_and(|v| v.view == view) => push(s),
                Sel::Note(id)
                    if self
                        .model
                        .notes
                        .iter()
                        .any(|n| n.id == id && n.view == view) =>
                {
                    push(s)
                }
                Sel::Module(id) if self.model.module(id).is_some_and(|m| m.view == view) => push(s),
                Sel::Shape(id) if self.model.shapes.iter().any(|x| x.id == id && x.view == view) => push(s),
                Sel::Control(id) if self.model.controls.iter().any(|x| x.id == id && x.view == view) => push(s),
                Sel::Group(g) => {
                    if let Some(g) = self.model.groups.iter().find(|x| x.id == g) {
                        for m in &g.members {
                            if let Some(x) = self.item_sel(*m) {
                                push(x);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// The selection kind for a raw id.
    pub fn item_sel(&self, id: u64) -> Option<Sel> {
        if self.model.var(id).is_some() {
            Some(Sel::Var(id))
        } else if self.model.notes.iter().any(|n| n.id == id) {
            Some(Sel::Note(id))
        } else if self.model.module(id).is_some() {
            Some(Sel::Module(id))
        } else if self.model.shapes.iter().any(|s| s.id == id) {
            Some(Sel::Shape(id))
        } else if self.model.controls.iter().any(|s| s.id == id) {
            Some(Sel::Control(id))
        } else {
            None
        }
    }

    pub fn sel_id(s: Sel) -> u64 {
        match s {
            Sel::Var(i) | Sel::Note(i) | Sel::Module(i) | Sel::Link(i) | Sel::Group(i) | Sel::Shape(i) | Sel::Control(i) => i,
        }
    }

    pub fn group_of(&self, id: u64) -> Option<u64> {
        self.model
            .groups
            .iter()
            .find(|g| g.members.contains(&id))
            .map(|g| g.id)
    }

    pub fn select_one(&mut self, s: Option<Sel>) {
        self.sel = s;
        self.multi.clear();
        if let Some(Sel::Group(g)) = s {
            if let Some(g) = self.model.groups.iter().find(|x| x.id == g) {
                self.multi = g.members.iter().filter_map(|m| self.item_sel(*m)).collect();
            }
        }
    }

    /// Shift-click: add or remove one item (a grouped item brings its whole group).
    pub fn toggle_sel(&mut self, s: Sel) {
        let mut items = self.selected_items();
        let ids: Vec<Sel> = match self.group_of(Self::sel_id(s)) {
            Some(g) => self
                .model
                .groups
                .iter()
                .find(|x| x.id == g)
                .map(|g| g.members.iter().filter_map(|m| self.item_sel(*m)).collect())
                .unwrap_or_default(),
            None => vec![s],
        };
        if items.contains(&s) {
            items.retain(|x| !ids.contains(x));
        } else {
            for i in ids {
                if !items.contains(&i) {
                    items.push(i);
                }
            }
        }
        self.sel = items.last().copied();
        self.multi = items;
    }

    pub fn select_all(&mut self) {
        let view = self.diagram_view();
        let mut all: Vec<Sel> = self
            .model
            .vars
            .iter()
            .filter(|v| v.view == view)
            .map(|v| Sel::Var(v.id))
            .collect();
        all.extend(
            self.model
                .notes
                .iter()
                .filter(|n| n.view == view)
                .map(|n| Sel::Note(n.id)),
        );
        all.extend(
            self.model
                .modules
                .iter()
                .filter(|m| m.view == view)
                .map(|m| Sel::Module(m.id)),
        );
        all.extend(self.model.shapes.iter().filter(|s| s.view == view).map(|s| Sel::Shape(s.id)));
        all.extend(self.model.controls.iter().filter(|s| s.view == view).map(|s| Sel::Control(s.id)));
        self.sel = all.first().copied();
        self.multi = all;
        self.status = format!("Selected {} item(s)", self.multi.len());
    }

    // ------------------------------------------------------------------ geometry

    pub fn item_rect(&self, s: Sel) -> Option<Rect> {
        match s {
            Sel::Var(id) | Sel::Module(id) => self.canvas.rect_of(id),
            Sel::Note(id) => self.canvas.note_rect_of(id),
            Sel::Shape(id) => self.shape_rect(id),
            Sel::Control(id) => self.control_rect(id),
            _ => None,
        }
    }

    /// Move an item. Flows move their loose (cloud) ends; with `exact`, a flow tied to one
    /// stock moves its loose end twice as far so its valve lands exactly `d` away.
    pub fn move_item(&mut self, s: Sel, d: Vec2, exact: bool) {
        match s {
            Sel::Var(id) => {
                let (free_from, free_to) = match self.model.var(id) {
                    Some(v) if v.kind == Kind::Flow => {
                        let here = |e: Option<u64>| {
                            e.and_then(|x| self.model.var(x))
                                .is_some_and(|st| st.view == v.view)
                        };
                        (!here(v.from), !here(v.to))
                    }
                    _ => (false, false),
                };
                let Some(v) = self.model.var_mut(id) else {
                    return;
                };
                if v.kind != Kind::Flow {
                    v.pos[0] += d.x;
                    v.pos[1] += d.y;
                    return;
                }
                let k = if exact && free_from != free_to {
                    2.0
                } else {
                    1.0
                };
                if free_from {
                    v.from_pt[0] += d.x * k;
                    v.from_pt[1] += d.y * k;
                }
                if free_to {
                    v.to_pt[0] += d.x * k;
                    v.to_pt[1] += d.y * k;
                }
            }
            Sel::Note(id) => {
                if let Some(n) = self.model.notes.iter_mut().find(|n| n.id == id) {
                    n.pos[0] += d.x;
                    n.pos[1] += d.y;
                }
            }
            Sel::Module(id) => {
                if let Some(m) = self.model.modules.iter_mut().find(|m| m.id == id) {
                    m.pos[0] += d.x;
                    m.pos[1] += d.y;
                }
            }
            Sel::Shape(id) => {
                if let Some(s) = self.model.shapes.iter_mut().find(|s| s.id == id) {
                    s.pos[0] += d.x;
                    s.pos[1] += d.y;
                }
            }
            Sel::Control(id) => {
                if let Some(s) = self.model.controls.iter_mut().find(|s| s.id == id) {
                    s.pos[0] += d.x;
                    s.pos[1] += d.y;
                }
            }
            _ => {}
        }
    }

    /// Snap the moved items onto the grid.
    pub fn snap_items(&mut self, items: &[Sel]) {
        if !self.snap {
            return;
        }
        for s in items {
            match *s {
                Sel::Var(id) => {
                    if let Some(v) = self.model.var_mut(id) {
                        if v.kind == Kind::Flow {
                            v.from_pt = [snap(v.from_pt[0]), snap(v.from_pt[1])];
                            v.to_pt = [snap(v.to_pt[0]), snap(v.to_pt[1])];
                        } else {
                            v.pos = [snap(v.pos[0]), snap(v.pos[1])];
                        }
                    }
                }
                Sel::Note(id) => {
                    if let Some(n) = self.model.notes.iter_mut().find(|n| n.id == id) {
                        n.pos = [snap(n.pos[0]), snap(n.pos[1])];
                    }
                }
                Sel::Module(id) => {
                    if let Some(m) = self.model.modules.iter_mut().find(|m| m.id == id) {
                        m.pos = [snap(m.pos[0]), snap(m.pos[1])];
                    }
                }
                Sel::Shape(id) => {
                    if let Some(s) = self.model.shapes.iter_mut().find(|s| s.id == id) {
                        s.pos = [snap(s.pos[0]), snap(s.pos[1])];
                    }
                }
                Sel::Control(id) => {
                    if let Some(s) = self.model.controls.iter_mut().find(|s| s.id == id) {
                        s.pos = [snap(s.pos[0]), snap(s.pos[1])];
                    }
                }
                _ => {}
            }
        }
    }

    pub fn nudge(&mut self, d: Vec2) {
        let items = self.selected_items();
        if items.is_empty() {
            return;
        }
        self.checkpoint();
        for s in items {
            self.move_item(s, d, true);
        }
    }

    // ------------------------------------------------------------------ align & distribute

    pub fn align(&mut self, op: Align) {
        let items: Vec<(Sel, Rect)> = self
            .selected_items()
            .into_iter()
            .filter_map(|s| self.item_rect(s).map(|r| (s, r)))
            .collect();
        let need = if matches!(op, Align::DistributeH | Align::DistributeV) {
            3
        } else {
            2
        };
        if items.len() < need {
            self.status = format!(
                "Select at least {need} items to {}",
                op.label().to_lowercase()
            );
            return;
        }
        self.checkpoint();
        let all = items.iter().fold(Rect::NOTHING, |a, (_, r)| a.union(*r));
        let mut moves: Vec<(Sel, Vec2)> = Vec::new();
        match op {
            Align::Left => {
                moves = items
                    .iter()
                    .map(|(s, r)| (*s, vec2(all.left() - r.left(), 0.0)))
                    .collect()
            }
            Align::Right => {
                moves = items
                    .iter()
                    .map(|(s, r)| (*s, vec2(all.right() - r.right(), 0.0)))
                    .collect()
            }
            Align::CenterH => {
                moves = items
                    .iter()
                    .map(|(s, r)| (*s, vec2(all.center().x - r.center().x, 0.0)))
                    .collect()
            }
            Align::Top => {
                moves = items
                    .iter()
                    .map(|(s, r)| (*s, vec2(0.0, all.top() - r.top())))
                    .collect()
            }
            Align::Bottom => {
                moves = items
                    .iter()
                    .map(|(s, r)| (*s, vec2(0.0, all.bottom() - r.bottom())))
                    .collect()
            }
            Align::Middle => {
                moves = items
                    .iter()
                    .map(|(s, r)| (*s, vec2(0.0, all.center().y - r.center().y)))
                    .collect()
            }
            Align::DistributeH | Align::DistributeV => {
                // equal gaps between neighbours, outer items stay put
                let horiz = op == Align::DistributeH;
                let mut sorted = items.clone();
                sorted.sort_by(|a, b| {
                    if horiz {
                        a.1.center().x.total_cmp(&b.1.center().x)
                    } else {
                        a.1.center().y.total_cmp(&b.1.center().y)
                    }
                });
                let span = |r: &Rect| if horiz { r.width() } else { r.height() };
                let total: f32 = sorted.iter().map(|(_, r)| span(r)).sum();
                let (start, end) = if horiz {
                    (all.left(), all.right())
                } else {
                    (all.top(), all.bottom())
                };
                let gap = ((end - start) - total) / (sorted.len() - 1) as f32;
                let mut at = start;
                for (s, r) in sorted {
                    let cur = if horiz { r.left() } else { r.top() };
                    moves.push((
                        s,
                        if horiz {
                            vec2(at - cur, 0.0)
                        } else {
                            vec2(0.0, at - cur)
                        },
                    ));
                    at += span(&r) + gap;
                }
            }
        }
        for (s, d) in moves {
            if d.length() > 0.01 {
                self.move_item(s, d, true);
            }
        }
        self.status = op.label().to_string();
    }

    // ------------------------------------------------------------------ clipboard

    pub fn copy(&mut self, cut: bool) -> bool {
        let items = self.selected_items();
        let ids: Vec<u64> = items
            .iter()
            .filter_map(|s| if let Sel::Var(i) = s { Some(*i) } else { None })
            .collect();
        let notes: Vec<Note> = items
            .iter()
            .filter_map(|s| {
                if let Sel::Note(i) = s {
                    self.model.notes.iter().find(|n| n.id == *i).cloned()
                } else {
                    None
                }
            })
            .collect();
        let shapes: Vec<crate::model::Shape> = items
            .iter()
            .filter_map(|s| if let Sel::Shape(i) = s { self.model.shapes.iter().find(|x| x.id == *i).cloned() } else { None })
            .collect();
        let images: Vec<crate::model::ImageAsset> = self
            .model
            .images
            .iter()
            .filter(|a| shapes.iter().any(|s| s.image == Some(a.id)))
            .cloned()
            .collect();
        let controls: Vec<crate::model::Control> = items
            .iter()
            .filter_map(|s| if let Sel::Control(i) = s { self.model.controls.iter().find(|x| x.id == *i).cloned() } else { None })
            .collect();
        if ids.is_empty() && notes.is_empty() && shapes.is_empty() && controls.is_empty() {
            if items.iter().any(|s| matches!(s, Sel::Module(_))) {
                self.status =
                    "Modules can't be copied yet — open the module and copy its parts".into();
            }
            return false;
        }
        let mut vars: Vec<Var> = Vec::new();
        for id in &ids {
            let mut v = self.model.var(*id).unwrap().clone();
            if v.kind == Kind::Flow {
                // a flow whose stock stays behind keeps that end as a cloud where the stock was
                for (end, pt) in [(v.from, 0), (v.to, 1)] {
                    if let Some(sid) = end {
                        if !ids.contains(&sid) {
                            let p = self.model.var(sid).map(|s| s.pos).unwrap_or(v.pos);
                            if pt == 0 {
                                v.from = None;
                                v.from_pt = [p[0] + 70.0, p[1]];
                            } else {
                                v.to = None;
                                v.to_pt = [p[0] - 70.0, p[1]];
                            }
                        }
                    }
                }
                let (a, b) = (
                    v.from
                        .and_then(|s| self.model.var(s))
                        .map(|s| s.pos)
                        .unwrap_or(v.from_pt),
                    v.to.and_then(|s| self.model.var(s))
                        .map(|s| s.pos)
                        .unwrap_or(v.to_pt),
                );
                v.pos = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            }
            vars.push(v);
        }
        let links: Vec<Link> = self
            .model
            .links
            .iter()
            .filter(|l| ids.contains(&l.from) && ids.contains(&l.to))
            .cloned()
            .collect();
        let n = vars.len() + notes.len() + shapes.len() + controls.len();
        self.clip = Some(Clip {
            vars,
            links,
            notes,
            shapes,
            controls,
            images,
            from_view: self.diagram_view(),
        });
        self.paste_count = 0;
        if cut {
            self.delete_selection();
            self.status = format!("Cut {n} item(s)");
        } else {
            self.status = format!("Copied {n} item(s) — paste on any diagram or causal loop sheet");
        }
        true
    }

    pub fn paste(&mut self) {
        let Some(clip) = self.clip.clone() else {
            self.status = "Nothing to paste yet — select something and copy it first".into();
            return;
        };
        match self.active_sheet_kind() {
            SheetKind::Diagram => self.paste_diagram(&clip),
            SheetKind::CausalLoop => self.paste_cld(&clip),
            _ => self.status = "Paste onto a diagram or causal loop sheet".into(),
        }
    }

    fn paste_diagram(&mut self, clip: &Clip) {
        self.checkpoint();
        let view = self.diagram_view();
        self.paste_count += 1;
        let off = if clip.from_view == view {
            36.0 * self.paste_count as f32
        } else {
            0.0
        };
        let mut idmap: HashMap<u64, u64> = HashMap::new();
        let mut names: Vec<(String, String)> = Vec::new();
        for v in &clip.vars {
            let nid = self.model.new_id();
            idmap.insert(v.id, nid);
            let base = v.name.clone();
            let mut name = base.clone();
            let mut k = 2;
            while self.model.by_name(&expr::normalize(&name)).is_some()
                || names
                    .iter()
                    .any(|(_, n)| expr::normalize(n) == expr::normalize(&name))
            {
                name = format!("{base} {k}");
                k += 1;
            }
            names.push((base, name));
        }
        let mut new_sel = Vec::new();
        for (v, (_, new_name)) in clip.vars.iter().zip(&names) {
            let mut nv = v.clone();
            nv.id = idmap[&v.id];
            nv.name = new_name.clone();
            nv.view = view;
            nv.pos = [v.pos[0] + off, v.pos[1] + off];
            nv.from_pt = [v.from_pt[0] + off, v.from_pt[1] + off];
            nv.to_pt = [v.to_pt[0] + off, v.to_pt[1] + off];
            nv.from = v.from.and_then(|s| idmap.get(&s).copied());
            nv.to = v.to.and_then(|s| idmap.get(&s).copied());
            // references to copied variables follow their new names
            for (old, new) in &names {
                if old != new {
                    nv.equation = expr::rename_in(&nv.equation, &expr::normalize(old), new);
                }
            }
            new_sel.push(Sel::Var(nv.id));
            self.model.vars.push(nv);
        }
        for l in &clip.links {
            let id = self.model.new_id();
            let mut nl = l.clone();
            nl.id = id;
            nl.from = idmap[&l.from];
            nl.to = idmap[&l.to];
            self.model.links.push(nl);
        }
        for n in &clip.notes {
            let id = self.model.new_id();
            self.model.notes.push(Note {
                id,
                text: n.text.clone(),
                pos: [n.pos[0] + off, n.pos[1] + off],
                view,
                fmt: n.fmt.clone(),
            });
            new_sel.push(Sel::Note(id));
        }
        // pictures: reuse a file already in this model, add the others (under a new id if theirs is taken)
        let mut image_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        for im in &clip.images {
            if self.model.images.iter().any(|a| a.id == im.id && a.data == im.data) {
                image_map.insert(im.id, im.id);
                continue;
            }
            let mut im = im.clone();
            let old = im.id;
            if self.model.images.iter().any(|a| a.id == old) {
                im.id = self.model.new_id();
            }
            image_map.insert(old, im.id);
            self.model.images.push(im);
        }
        for s in &clip.shapes {
            let id = self.model.new_id();
            let mut ns = s.clone();
            ns.id = id;
            ns.view = view;
            ns.pos = [s.pos[0] + off, s.pos[1] + off];
            ns.image = s.image.and_then(|i| image_map.get(&i).copied());
            self.model.shapes.push(ns);
            new_sel.push(Sel::Shape(id));
        }
        for k in &clip.controls {
            let id = self.model.new_id();
            let mut nk = k.clone();
            nk.id = id;
            nk.view = view;
            nk.pos = [k.pos[0] + off, k.pos[1] + off];
            self.model.controls.push(nk);
            new_sel.push(Sel::Control(id));
        }
        self.sel = new_sel.first().copied();
        self.multi = new_sel;
        self.changed();
        let renamed = names.iter().filter(|(a, b)| a != b).count();
        self.status = if renamed > 0 {
            format!(
                "Pasted {} item(s); {renamed} renamed to keep names unique",
                self.multi.len()
            )
        } else {
            format!("Pasted {} item(s)", self.multi.len())
        };
    }

    /// Variables pasted onto a causal loop sheet become words, arrows keep their signs.
    fn paste_cld(&mut self, clip: &Clip) {
        self.checkpoint();
        let Some(si) = self.model.sheets.iter().position(|s| s.id == self.sheet) else {
            return;
        };
        let mut idmap = HashMap::new();
        let mut nodes = Vec::new();
        for v in &clip.vars {
            let id = self.model.new_id();
            idmap.insert(v.id, id);
            nodes.push(CldNode {
                id,
                text: v.name.clone(),
                pos: v.pos,
            });
        }
        let mut links = Vec::new();
        for l in &clip.links {
            let id = self.model.new_id();
            links.push(CldLink {
                id,
                from: idmap[&l.from],
                to: idmap[&l.to],
                bend: l.bend,
                polarity: if l.polarity.is_empty() {
                    "+".into()
                } else {
                    l.polarity.clone()
                },
                delay: l.delay_mark,
            });
        }
        // flows between copied stocks push them up or down
        for v in clip.vars.iter().filter(|v| v.kind == Kind::Flow) {
            for (end, sign) in [(v.to, "+"), (v.from, "-")] {
                if let Some(s) = end.and_then(|s| idmap.get(&s)) {
                    let id = self.model.new_id();
                    links.push(CldLink {
                        id,
                        from: idmap[&v.id],
                        to: *s,
                        bend: 20.0,
                        polarity: sign.into(),
                        delay: false,
                    });
                }
            }
        }
        let n = nodes.len();
        let d = &mut self.model.sheets[si].cld;
        d.nodes.extend(nodes);
        d.links.extend(links);
        self.status = format!("Pasted {n} variable(s) as causal loop words");
    }

    pub fn duplicate(&mut self) {
        if self.copy(false) {
            self.paste();
        }
    }

    // ------------------------------------------------------------------ groups

    pub fn group_selection(&mut self) {
        let items = self.selected_items();
        if items.len() < 2 {
            self.status = "Select two or more items to group them".into();
            return;
        }
        self.checkpoint();
        let ids: Vec<u64> = items.iter().map(|s| Self::sel_id(*s)).collect();
        for g in self.model.groups.iter_mut() {
            g.members.retain(|m| !ids.contains(m));
        }
        self.model.groups.retain(|g| g.members.len() > 1);
        let id = self.model.new_id();
        let n = self.model.groups.len() + 1;
        self.model.groups.push(Group {
            id,
            name: format!("Group {n}"),
            members: ids,
            view: self.diagram_view(),
            color: None,
        });
        self.select_one(Some(Sel::Group(id)));
        self.status = "Grouped — they now select and move together".into();
    }

    pub fn ungroup(&mut self) {
        let gid = match self.sel {
            Some(Sel::Group(g)) => Some(g),
            Some(s) => self.group_of(Self::sel_id(s)),
            None => None,
        };
        let Some(gid) = gid else {
            self.status = "Select a group to ungroup it".into();
            return;
        };
        self.checkpoint();
        let members = self
            .model
            .groups
            .iter()
            .find(|g| g.id == gid)
            .map(|g| g.members.clone())
            .unwrap_or_default();
        self.model.groups.retain(|g| g.id != gid);
        self.multi = members.iter().filter_map(|m| self.item_sel(*m)).collect();
        self.sel = self.multi.first().copied();
        self.status = "Ungrouped".into();
    }

    // ------------------------------------------------------------------ modules

    /// The item standing for `var` on sheet `view`: the variable itself, the module that
    /// (perhaps several levels down) contains it, or None if it lives elsewhere.
    pub fn proxy(&self, var: u64, view: u64) -> Option<u64> {
        let v = self.model.var(var)?;
        let (mut sheet, mut last) = (v.view, var);
        for _ in 0..32 {
            if sheet == view {
                return Some(last);
            }
            let m = self.model.modules.iter().find(|m| m.inner == sheet)?;
            last = m.id;
            sheet = m.view;
        }
        None
    }

    /// Is `var` somewhere inside module `mid`?
    pub fn inside(&self, var: u64, mid: u64) -> bool {
        let Some(m) = self.model.module(mid) else {
            return false;
        };
        self.proxy(var, m.view) == Some(mid)
    }

    /// (inputs: outside variables the module uses, outputs: inside variables used outside).
    pub fn module_ports(&self, mid: u64) -> (Vec<u64>, Vec<u64>) {
        let (mut ins, mut outs) = (Vec::new(), Vec::new());
        let add = |v: &mut Vec<u64>, x: u64| {
            if !v.contains(&x) {
                v.push(x);
            }
        };
        for l in &self.model.links {
            let (a, b) = (self.inside(l.from, mid), self.inside(l.to, mid));
            if b && !a {
                add(&mut ins, l.from);
            }
            if a && !b {
                add(&mut outs, l.from);
            }
        }
        for f in self.model.vars.iter().filter(|f| f.kind == Kind::Flow) {
            let fi = self.inside(f.id, mid);
            for s in [f.from, f.to].into_iter().flatten() {
                let si = self.inside(s, mid);
                if fi && !si {
                    add(&mut outs, f.id);
                }
                if si && !fi {
                    add(&mut ins, f.id);
                }
            }
        }
        (ins, outs)
    }

    /// Wrap the selection into a module: its parts move to their own sheet and a single
    /// box with input and output ports takes their place.
    pub fn make_module(&mut self) {
        let items: Vec<Sel> = self.selected_items();
        if !items
            .iter()
            .any(|s| matches!(s, Sel::Var(_) | Sel::Module(_)))
        {
            self.status = "Select the variables that should form the module first".into();
            return;
        }
        self.checkpoint();
        let parent = self.diagram_view();
        let rect = items
            .iter()
            .filter_map(|s| self.item_rect(*s))
            .fold(Rect::NOTHING, |a, r| a.union(r));
        let center = if rect.is_positive() {
            rect.center()
        } else {
            // not drawn yet: use the parts' own positions
            let pts: Vec<[f32; 2]> = items
                .iter()
                .filter_map(|s| {
                    if let Sel::Var(i) = s {
                        self.model.var(*i).map(|v| v.pos)
                    } else {
                        None
                    }
                })
                .collect();
            let k = pts.len().max(1) as f32;
            pos2(
                pts.iter().map(|p| p[0]).sum::<f32>() / k,
                pts.iter().map(|p| p[1]).sum::<f32>() / k,
            )
        };
        let n = self.model.modules.len() + 1;
        let name = format!("Module {n}");
        let sid = self.model.new_id();
        let mid = self.model.new_id();
        let mut sheet = Sheet::new(sid, &name, SheetKind::Diagram);
        sheet.module = Some(mid);
        let at = self
            .model
            .sheets
            .iter()
            .position(|s| s.id == parent)
            .map(|i| i + 1)
            .unwrap_or(self.model.sheets.len());
        self.model.sheets.insert(at, sheet);
        for s in &items {
            match *s {
                Sel::Var(id) => {
                    if let Some(v) = self.model.var_mut(id) {
                        v.view = sid;
                    }
                }
                Sel::Note(id) => {
                    if let Some(x) = self.model.notes.iter_mut().find(|x| x.id == id) {
                        x.view = sid;
                    }
                }
                Sel::Module(id) => {
                    if let Some(x) = self.model.modules.iter_mut().find(|x| x.id == id) {
                        x.view = sid;
                    }
                }
                _ => {}
            }
        }
        let ids: Vec<u64> = items.iter().map(|s| Self::sel_id(*s)).collect();
        for g in self
            .model
            .groups
            .iter_mut()
            .filter(|g| g.members.iter().all(|m| ids.contains(m)))
        {
            g.view = sid;
        }
        self.model.modules.push(Module {
            id: mid,
            name,
            view: parent,
            inner: sid,
            pos: [center.x, center.y],
            description: String::new(),
            collapsed: false,
            layer: None,
        });
        self.select_one(Some(Sel::Module(mid)));
        let (i, o) = self.module_ports(mid);
        self.status = format!(
            "Made a module of {} part(s) with {} input(s) and {} output(s) — double-click it to open",
            items.len(),
            i.len(),
            o.len()
        );
        self.changed();
    }

    /// Put a module's parts back on its parent sheet and remove the module.
    pub fn dissolve_module(&mut self, mid: u64) {
        let Some(m) = self.model.module(mid).cloned() else {
            return;
        };
        self.checkpoint();
        // keep the parts' layout, centred where the module box was
        let mut pts: Vec<[f32; 2]> = self
            .model
            .vars
            .iter()
            .filter(|v| v.view == m.inner && v.kind != Kind::Flow)
            .map(|v| v.pos)
            .collect();
        pts.extend(
            self.model
                .notes
                .iter()
                .filter(|n| n.view == m.inner)
                .map(|n| n.pos),
        );
        let c = if pts.is_empty() {
            m.pos
        } else {
            [
                pts.iter().map(|p| p[0]).sum::<f32>() / pts.len() as f32,
                pts.iter().map(|p| p[1]).sum::<f32>() / pts.len() as f32,
            ]
        };
        let d = [m.pos[0] - c[0], m.pos[1] - c[1]];
        let mut back = Vec::new();
        for v in self.model.vars.iter_mut().filter(|v| v.view == m.inner) {
            v.view = m.view;
            v.pos = [v.pos[0] + d[0], v.pos[1] + d[1]];
            v.from_pt = [v.from_pt[0] + d[0], v.from_pt[1] + d[1]];
            v.to_pt = [v.to_pt[0] + d[0], v.to_pt[1] + d[1]];
            back.push(Sel::Var(v.id));
        }
        for n in self.model.notes.iter_mut().filter(|n| n.view == m.inner) {
            n.view = m.view;
            n.pos = [n.pos[0] + d[0], n.pos[1] + d[1]];
            back.push(Sel::Note(n.id));
        }
        for x in self.model.modules.iter_mut().filter(|x| x.view == m.inner) {
            x.view = m.view;
            x.pos = [x.pos[0] + d[0], x.pos[1] + d[1]];
            back.push(Sel::Module(x.id));
        }
        for g in self.model.groups.iter_mut().filter(|g| g.view == m.inner) {
            g.view = m.view;
        }
        self.model.modules.retain(|x| x.id != mid);
        self.model.sheets.retain(|s| s.id != m.inner);
        self.model.ensure_sheets();
        if self.sheet == m.inner {
            self.sheet = m.view;
        }
        self.sel = back.first().copied();
        self.multi = back;
        self.status = format!("Dissolved '{}' — its parts are back on this sheet", m.name);
        self.changed();
    }

    pub fn open_module(&mut self, mid: u64) {
        if let Some(m) = self.model.module(mid) {
            self.sheet = m.inner;
            self.select_one(None);
            self.fit_pending = true;
        }
    }

    /// Go to wherever a variable lives and select it.
    pub fn reveal_var(&mut self, id: u64) {
        if let Some(v) = self.model.var(id) {
            if v.view != self.sheet {
                self.sheet = v.view;
                self.fit_pending = true;
            }
            self.select_one(Some(Sel::Var(id)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(&egui::Context::default(), None)
    }
    fn id(a: &App, name: &str) -> u64 {
        a.model.by_name(&expr::normalize(name)).unwrap().id
    }

    #[test]
    fn copy_paste_renames_and_keeps_links() {
        let mut a = app();
        let (fi, tot) = (id(&a, "Fraction Infected"), id(&a, "Total Population"));
        a.select_one(Some(Sel::Var(fi)));
        a.toggle_sel(Sel::Var(tot));
        assert!(a.copy(false));
        let links = a.model.links.len();
        a.paste();
        let pasted = a.model.by_name("fraction infected 2").unwrap().clone();
        assert!(a.model.by_name("total population 2").is_some());
        // the pasted equation points at the pasted copy, not the original
        assert_eq!(
            expr::normalize(&pasted.equation),
            "infected / total population 2"
        );
        assert_eq!(a.model.links.len(), links + 1);
        let c = crate::sim::compile(&a.model);
        assert!(c.is_ok());
    }

    #[test]
    fn module_has_ports_and_dissolves() {
        let mut a = app();
        let parts = [
            "Fraction Infected",
            "Total Population",
            "Effect of Awareness",
        ];
        a.select_one(None);
        for p in parts {
            let i = id(&a, p);
            a.toggle_sel(Sel::Var(i));
        }
        a.make_module();
        let m = a.model.modules[0].clone();
        assert!(
            parts
                .iter()
                .all(|p| a.model.by_name(&expr::normalize(p)).unwrap().view == m.inner)
        );
        let (ins, outs) = a.module_ports(m.id);
        let names = |v: &[u64]| {
            v.iter()
                .map(|i| a.model.var(*i).unwrap().name.clone())
                .collect::<Vec<_>>()
        };
        assert!(
            names(&ins).contains(&"Infected".to_string()),
            "{:?}",
            names(&ins)
        );
        assert!(
            names(&outs).contains(&"Fraction Infected".to_string()),
            "{:?}",
            names(&outs)
        );
        // the simulation doesn't care where things are drawn
        assert!(crate::sim::compile(&a.model).is_ok());
        a.dissolve_module(m.id);
        assert!(a.model.modules.is_empty());
        assert!(a.model.sheets.iter().all(|s| s.id != m.inner));
    }

    #[test]
    fn groups_select_together() {
        let mut a = app();
        let cr = id(&a, "Contact Rate");
        let g = a.group_of(cr).expect("example groups the policy levers");
        a.select_one(Some(Sel::Group(g)));
        assert_eq!(a.selected_items().len(), 2);
        a.ungroup();
        assert!(a.group_of(cr).is_none());
    }
}
