//! The Entities sheet: the model's *structure* as cards, like the tables of a class or ER
//! diagram. One card per set of similar things (Tank ×57, Region ×4) and one for the single
//! values; each card lists the variables that exist once for every item of it, grouped into
//! inputs, state, flows and calculations, with the value for what the Showing bar selects and
//! the formula. Lines join a set to the set it belongs to (a plain arrow, tank → region).
//!
//! It is built from the model, so it is always current; clicking a row selects that variable.

use std::collections::HashMap;

use egui::{Color32, CornerRadius, Margin, Rect, RichText, Sense, Stroke, pos2, vec2};

use crate::app::{App, Sel};
use crate::model::{ItemSet, Kind, Var};
use crate::theme;

const CARD_W: f32 = 500.0;
const COL_GAP: f32 = 170.0;

impl App {
    pub fn entities_sheet(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let sets: Vec<ItemSet> = self.model.sets.clone();
        // a set sits one column to the right of the set it belongs to
        let depth = |s: &ItemSet| -> usize {
            let (mut d, mut cur) = (0, s.clone());
            while let Some(p) = cur.parent.as_ref().and_then(|p| sets.iter().find(|x| &x.name == p)) {
                d += 1;
                cur = p.clone();
                if d > 6 {
                    break;
                }
            }
            d
        };
        let max_depth = sets.iter().map(|s| depth(s)).max().unwrap_or(0);
        let mut cards: Vec<(String, Option<ItemSet>, usize)> = sets.iter().map(|s| (s.name.clone(), Some(s.clone()), depth(s))).collect();
        cards.sort_by_key(|c| c.2);
        let has_single = self.model.vars.iter().any(|v| v.set.is_none());
        if has_single {
            cards.push(("Single values".to_string(), None, if sets.is_empty() { 0 } else { max_depth + 1 }));
        }

        ui.painter().rect_filled(ui.max_rect(), 0.0, pal.canvas);
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            let origin = ui.cursor().min + vec2(24.0, 20.0);
            let mut y_of_col: HashMap<usize, f32> = HashMap::new();
            let mut rects: HashMap<String, Rect> = HashMap::new();
            for (name, set, col) in &cards {
                let y = *y_of_col.entry(*col).or_insert(0.0);
                let pos = origin + vec2(*col as f32 * (CARD_W + COL_GAP), y);
                let used = ui
                    .scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_size(pos, vec2(CARD_W, 10.0))), |ui| {
                        self.entity_card(ui, name, set.as_ref());
                    })
                    .response
                    .rect;
                rects.insert(name.clone(), used);
                *y_of_col.get_mut(col).unwrap() += used.height() + 26.0;
            }
            // "belongs to": a plain arrow from a set to the set it belongs to, drawn like the
            // diagram's arrows (same shape and tip as chosen in View → Arrows)
            for s in &sets {
                let (Some(p), Some(child)) = (s.parent.as_ref(), rects.get(&s.name)) else { continue };
                let Some(parent) = rects.get(p) else { continue };
                let classic = self.model.diagram_style == crate::model::DiagramStyle::Classic;
                let col = if classic { pal.text.gamma_multiply(0.8) } else { pal.muted };
                let a = pos2(child.left(), child.top() + 26.0);
                let b = pos2(parent.right() + 3.0, parent.top() + 26.0);
                let path = match self.model.arrow_shape {
                    crate::model::ArrowShape::Curved => theme::arc_points(a, b, 0.0, 40),
                    crate::model::ArrowShape::Straight => theme::arc_points(a, b, 0.0, 40),
                    crate::model::ArrowShape::Elbow => theme::elbow_points(a, b, 40),
                };
                let tip = *path.last().unwrap();
                let back = path[path.len().saturating_sub(3)];
                let dir = (tip - back).normalized();
                let size = 9.0;
                let mut line = path.clone();
                if let Some(last) = line.last_mut() {
                    *last = tip - dir * size * 0.8;
                }
                let painter = ui.painter();
                painter.add(egui::Shape::line(line, Stroke::new(if classic { 1.0 } else { 1.5 }, col)));
                theme::link_head(painter, self.model.arrow_head, tip, dir, size, col);
                painter.text(pos2((a.x + b.x) / 2.0, a.y - 6.0), egui::Align2::CENTER_BOTTOM, "belongs to", egui::FontId::proportional(12.0), pal.muted);
            }
            let bottom = y_of_col.values().cloned().fold(0.0, f32::max);
            let width = (max_depth + 2) as f32 * (CARD_W + COL_GAP);
            ui.allocate_rect(Rect::from_min_size(origin, vec2(width, bottom + 60.0)), Sense::hover());
            ui.painter().text(
                origin + vec2(0.0, bottom + 20.0),
                egui::Align2::LEFT_TOP,
                "Each card is one kind of thing; its rows exist once for every item of it. Click a row to edit it. The Showing bar above picks whose values are shown.",
                egui::FontId::proportional(12.5),
                pal.muted,
            );
        });
    }

    fn entity_card(&mut self, ui: &mut egui::Ui, name: &str, set: Option<&ItemSet>) {
        let pal = self.pal;
        let vars: Vec<Var> = self
            .model
            .vars
            .iter()
            .filter(|v| match set {
                Some(s) => v.set.as_deref() == Some(s.name.as_str()),
                None => v.set.is_none(),
            })
            .cloned()
            .collect();
        egui::Frame::new()
            .fill(pal.panel)
            .stroke(Stroke::new(1.2, pal.border))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                ui.set_width(CARD_W);
                egui::Frame::new()
                    .fill(pal.accent.gamma_multiply(if pal.dark { 0.28 } else { 0.12 }))
                    .corner_radius(CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 })
                    .inner_margin(Margin::symmetric(14, 9))
                    .show(ui, |ui| {
                        ui.set_width(CARD_W - 28.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).strong().size(17.0).color(pal.text));
                            if let Some(s) = set {
                                ui.label(RichText::new(format!("×{}", s.items.len())).monospace().color(pal.accent));
                                if let Some(p) = &s.parent {
                                    ui.label(RichText::new(format!("each belongs to one {p}")).small().color(pal.muted));
                                }
                            } else {
                                ui.label(RichText::new("one value each").small().color(pal.muted));
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(format!("{} rows", vars.len())).small().color(pal.muted));
                            });
                        });
                    });
                egui::Frame::new().inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.set_width(CARD_W - 28.0);
                    for (title, kinds) in [
                        ("Inputs", [Kind::Constant, Kind::Lookup].as_slice()),
                        ("State", [Kind::Stock].as_slice()),
                        ("Flows", [Kind::Flow].as_slice()),
                        ("Calculated", [Kind::Aux].as_slice()),
                    ] {
                        let rows: Vec<&Var> = vars.iter().filter(|v| kinds.contains(&v.kind)).collect();
                        if rows.is_empty() {
                            continue;
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new(title.to_uppercase()).small().color(pal.muted));
                        egui::Grid::new(("entity-rows", name, title)).num_columns(4).spacing([10.0, 3.0]).striped(true).show(ui, |ui| {
                            for v in rows {
                                self.entity_row(ui, v);
                                ui.end_row();
                            }
                        });
                    }
                    if vars.is_empty() {
                        ui.label(RichText::new("Nothing is per this set yet: select a variable and choose it under Details → One for each.").small().color(pal.muted));
                    }
                });
            });
    }

    fn entity_row(&mut self, ui: &mut egui::Ui, v: &Var) {
        let pal = self.pal;
        let col = pal.kind(v.kind);
        let selected = self.is_sel(Sel::Var(v.id));
        // marker, as on the diagram
        let (mr, _) = ui.allocate_exact_size(vec2(14.0, 16.0), Sense::hover());
        match v.kind {
            Kind::Constant => {
                let c = mr.center();
                let s = 4.5;
                ui.painter().add(egui::Shape::convex_polygon(vec![c + vec2(0.0, -s), c + vec2(s, 0.0), c + vec2(0.0, s), c + vec2(-s, 0.0)], col, Stroke::NONE));
            }
            Kind::Stock => {
                ui.painter().rect_stroke(Rect::from_center_size(mr.center(), vec2(10.0, 8.0)), CornerRadius::same(2), Stroke::new(1.6, col), egui::StrokeKind::Inside);
            }
            Kind::Flow => {
                ui.painter().arrow(pos2(mr.left(), mr.center().y), vec2(12.0, 0.0), Stroke::new(1.8, col));
            }
            Kind::Lookup => theme::sparkline(ui.painter(), Rect::from_center_size(mr.center(), vec2(12.0, 9.0)), &[0.0, 0.2, 0.8, 1.0], col, None, false),
            Kind::Aux => {
                ui.painter().circle_filled(mr.center(), 4.2, col.gamma_multiply(0.35));
                ui.painter().circle_filled(mr.center(), 2.4, col);
            }
        }
        let name = RichText::new(&v.name).color(if selected { pal.accent } else { pal.text }).strong();
        let r = ui.add(egui::Label::new(name).sense(Sense::click()));
        if r.clicked() {
            self.select_one(Some(Sel::Var(v.id)));
        }
        let focused = crate::sets::focus_label(&self.model, v, &self.focus).is_some();
        let set_items = self.model.items_of(v);
        let value = if v.kind == Kind::Lookup && v.equation.trim().is_empty() {
            format!("curve · {} pts", v.table.len())
        } else if v.set.is_some() && v.kind == Kind::Constant && !focused {
            // a constant differs by item: show its range instead of a meaningless total
            let num = |t: &str| t.trim().parse::<f64>().ok();
            let vals: Vec<f64> = set_items
                .iter()
                .filter_map(|it| v.by_item.iter().find(|o| &o.item == it).filter(|o| !o.equation.trim().is_empty()).and_then(|o| num(&o.equation)).or_else(|| num(&v.equation)))
                .collect();
            match (vals.iter().cloned().reduce(f64::min), vals.iter().cloned().reduce(f64::max)) {
                (Some(a), Some(b)) if (a - b).abs() > 1e-12 => format!("{} – {}", theme::fmt_num(a), theme::fmt_num(b)),
                (Some(a), _) => theme::fmt_num(a),
                _ => "–".into(),
            }
        } else {
            self.value_now(&v.name).map(theme::fmt_num).unwrap_or_else(|| "–".into())
        };
        let sign = if v.set.is_some() && v.kind != Kind::Constant && !focused {
            if v.rollup == crate::model::Rollup::Average { "avg " } else { "Σ " }
        } else {
            ""
        };
        let unit = if v.units.is_empty() { String::new() } else { format!(" {}", v.units) };
        ui.label(RichText::new(format!("{sign}{value}{unit}")).monospace().small().color(col));
        let n_diff = v.by_item.iter().filter(|o| !o.equation.trim().is_empty() || !o.table.is_empty()).count();
        let eq = match v.kind {
            Kind::Constant if n_diff > 0 => format!("set for {n_diff} of {} items", set_items.len()),
            Kind::Stock => format!("starts at {}", v.equation),
            Kind::Lookup if v.equation.trim().is_empty() => String::new(),
            Kind::Lookup => format!("curve of {}", v.equation),
            _ => v.equation.clone(),
        };
        let short: String = if eq.chars().count() > 52 { eq.chars().take(51).collect::<String>() + "…" } else { eq.clone() };
        r.on_hover_text(if v.comment.is_empty() { eq.clone() } else { format!("{}\n\n{}", v.comment, eq) });
        ui.label(RichText::new(short).small().color(pal.muted));
        let _: Color32 = col;
    }
}
