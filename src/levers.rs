//! Key levers: the few variables that drive the model, with their controls. They are drawn in a
//! card on the diagram itself (so the model can be played with from the overview) and at the top
//! of the side panel.
//!
//! - a lever that is a single value gets a slider;
//! - a lever that is per a set gets, with an item picked in the Showing bar, a slider for that
//!   item's own value, and with nothing picked an "all items ×" slider that nudges every item.

use egui::{Align2, RichText, Stroke, vec2};

use crate::app::App;
use crate::model::{ItemValue, Kind};

impl App {
    /// Does the model have any key levers?
    pub fn has_levers(&self) -> bool {
        self.model.vars.iter().any(|v| v.lever && v.kind == Kind::Constant)
    }

    /// One row per lever. Returns true if anything changed.
    pub fn lever_rows(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let ids: Vec<u64> = self.model.vars.iter().filter(|v| v.lever && v.kind == Kind::Constant).map(|v| v.id).collect();
        for id in ids {
            let v = self.model.var(id).unwrap().clone();
            let item = v
                .set
                .as_ref()
                .and_then(|sn| self.focus.get(sn))
                .filter(|i| self.model.items_of(&v).contains(i))
                .cloned();
            match (&v.set, item) {
                (None, _) => {
                    let mut tmp = v.clone();
                    if self.slider_row_pub(ui, &mut tmp) {
                        *self.model.var_mut(id).unwrap() = tmp;
                        self.changed();
                    }
                }
                (Some(_), Some(item)) => {
                    let mut tmp = v.clone();
                    tmp.set = None;
                    tmp.by_item.clear();
                    if let Some(o) = v.by_item.iter().find(|o| o.item == item).filter(|o| !o.equation.trim().is_empty()) {
                        tmp.equation = o.equation.clone();
                    }
                    tmp.name = format!("{} · {}", v.name, item);
                    if self.slider_row_pub(ui, &mut tmp) {
                        let x = self.model.var_mut(id).unwrap();
                        match x.by_item.iter_mut().find(|o| o.item == item) {
                            Some(o) => o.equation = tmp.equation.clone(),
                            None => x.by_item.push(ItemValue { item: item.clone(), equation: tmp.equation.clone(), table: Vec::new() }),
                        }
                        if tmp.slider != v.slider {
                            x.slider = tmp.slider;
                        }
                        self.changed();
                    }
                }
                (Some(set), None) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&v.name).size(15.0));
                        ui.label(RichText::new(format!("all {set} items")).small().color(pal.muted));
                        if (v.lever_scale - 1.0).abs() > 1e-9 && ui.small_button("reset").clicked() {
                            self.model.var_mut(id).unwrap().lever_scale = 1.0;
                            self.changed();
                        }
                    });
                    let mut f = v.lever_scale;
                    let r = ui.add(egui::Slider::new(&mut f, 0.5..=1.5).fixed_decimals(2).prefix("× ").clamping(egui::SliderClamping::Never));
                    if r.drag_started() {
                        self.checkpoint();
                    }
                    if r.changed() {
                        self.model.var_mut(id).unwrap().lever_scale = f;
                        self.changed();
                    }
                }
            }
        }
    }

    /// Move a lever by a fraction of its range (dragging its circle). A single value moves within
    /// its slider range; a per-set lever moves the picked item's value, or every item's together.
    pub fn nudge_lever(&mut self, id: u64, frac: f32) {
        let Some(v) = self.model.var(id).cloned() else { return };
        let frac = frac as f64;
        let num = |t: &str| t.trim().parse::<f64>().ok();
        let range = |cur: f64| -> (f64, f64) { v.slider.map(|s| (s[0], s[1])).unwrap_or((0.0, (cur.abs() * 2.0).max(1.0))) };
        let fmt = |x: f64| -> String { if (x - x.round()).abs() < 1e-9 { format!("{}", x.round() as i64) } else { format!("{:.4}", x).trim_end_matches('0').trim_end_matches('.').to_string() } };
        let item = v.set.as_ref().and_then(|sn| self.focus.get(sn)).filter(|i| self.model.items_of(&v).contains(i)).cloned();
        match (&v.set, item) {
            (None, _) => {
                let cur = num(&v.equation).unwrap_or(0.0);
                let (lo, hi) = range(cur);
                let x = (cur + frac * (hi - lo)).clamp(lo.min(hi), hi.max(lo));
                self.model.var_mut(id).unwrap().equation = fmt(x);
            }
            (Some(_), Some(item)) => {
                let cur = v.by_item.iter().find(|o| o.item == item).and_then(|o| num(&o.equation)).or_else(|| num(&v.equation)).unwrap_or(0.0);
                let (lo, hi) = range(cur);
                let x = (cur + frac * (hi - lo)).clamp(lo.min(hi), hi.max(lo));
                let var = self.model.var_mut(id).unwrap();
                match var.by_item.iter_mut().find(|o| o.item == item) {
                    Some(o) => o.equation = fmt(x),
                    None => var.by_item.push(ItemValue { item, equation: fmt(x), table: Vec::new() }),
                }
            }
            (Some(_), None) => {
                let var = self.model.var_mut(id).unwrap();
                var.lever_scale = (var.lever_scale + frac).clamp(0.5, 1.5);
            }
        }
        self.changed();
    }

    /// The control card on the diagram: the levers, always within reach.
    pub fn lever_card(&mut self, ctx: &egui::Context, canvas: egui::Rect) {
        // when the sheet draws its levers as circles outside the boxes, the card steps aside
        if !self.has_levers() || self.export_only.is_some() || self.exporting || self.canvas.levers_shown {
            return;
        }
        let pal = self.pal;
        egui::Area::new(egui::Id::new("lever-card"))
            .order(egui::Order::Middle)
            .fixed_pos(canvas.left_top() + vec2(14.0, 14.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(pal.panel)
                    .stroke(Stroke::new(1.2, pal.border))
                    .corner_radius(egui::CornerRadius::same(10))
                    .shadow(egui::Shadow { offset: [0, 3], blur: 10, spread: 0, color: egui::Color32::from_black_alpha(if pal.dark { 90 } else { 28 }) })
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_max_width(300.0);
                        ui.horizontal(|ui| {
                            let chev = if self.levers_open { crate::icons::Ic::ChevronDown } else { crate::icons::Ic::ChevronRight };
                            if crate::icons::tool(ui, chev, "Fold or unfold the key levers", pal.accent, pal.accent).clicked() {
                                self.levers_open = !self.levers_open;
                            }
                            ui.label(RichText::new("Key levers").strong().color(pal.text));
                            if !self.levers_open {
                                ui.label(RichText::new("— the controls that drive the model").small().color(pal.muted));
                            }
                        });
                        if self.levers_open {
                            ui.set_width(300.0);
                            self.lever_rows(ui);
                        }
                    });
            });
        let _ = Align2::LEFT_TOP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(&egui::Context::default(), None)
    }

    #[test]
    fn dragging_a_lever_moves_it_within_its_slider_range() {
        let mut a = app();
        let id = a.model.by_name("contact rate").unwrap().id;
        let (lo, hi) = {
            let s = a.model.var(id).unwrap().slider.unwrap_or([0.0, 12.0]);
            (s[0], s[1])
        };
        a.model.var_mut(id).unwrap().lever = true;
        let before: f64 = a.model.var(id).unwrap().equation.trim().parse().unwrap();
        a.nudge_lever(id, 0.1);
        let after: f64 = a.model.var(id).unwrap().equation.trim().parse().unwrap();
        assert!((after - (before + 0.1 * (hi - lo))).abs() < 1e-3, "{before} -> {after}");
        a.nudge_lever(id, 50.0);
        assert_eq!(a.model.var(id).unwrap().equation.trim().parse::<f64>().unwrap(), hi, "stops at the end of the range");
    }
}
