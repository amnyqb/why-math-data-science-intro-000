//! The outline: the whole model as a tree, by module — a map for big models. Click a name to go
//! to it on its diagram; tick the box to plot it. Modules open and close like folders.

use egui::{CornerRadius, RichText, Sense, vec2};

use crate::app::App;
use crate::model::{Kind, Var};

impl App {
    pub fn outline(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Outline").strong().size(15.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::icons::tool(ui, crate::icons::Ic::ChevronLeft, "Hide the outline", pal.muted, pal.accent).clicked() {
                    self.show_outline = false;
                }
            });
        });
        ui.label(RichText::new("Click to go to it · tick to plot").small().color(pal.muted));
        ui.add_space(4.0);
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            let root = self.model.first_diagram();
            self.outline_sheet(ui, root, 0);
        });
    }

    /// The modules and variables that sit on one diagram sheet.
    fn outline_sheet(&mut self, ui: &mut egui::Ui, sheet: u64, depth: usize) {
        let pal = self.pal;
        let mods: Vec<(u64, String, u64)> = self.model.modules.iter().filter(|m| m.view == sheet).map(|m| (m.id, m.name.clone(), m.inner)).collect();
        for (mid, name, inner) in mods {
            let parts = self.model.vars.iter().filter(|v| self.inside(v.id, mid)).count();
            let open_default = depth == 0 && self.sheet == inner;
            egui::CollapsingHeader::new(RichText::new(format!("{name}  ·  {parts}")).color(pal.accent))
                .id_salt(("outline-mod", mid))
                .default_open(open_default)
                .show(ui, |ui| {
                    self.outline_sheet(ui, inner, depth + 1);
                });
        }
        let mut vars: Vec<Var> = self.model.vars.iter().filter(|v| v.view == sheet).cloned().collect();
        let order = |k: Kind| match k {
            Kind::Constant => 0,
            Kind::Lookup => 1,
            Kind::Stock => 2,
            Kind::Flow => 3,
            Kind::Aux => 4,
        };
        vars.sort_by_key(|v| order(v.kind));
        for v in vars {
            ui.horizontal(|ui| {
                let can_plot = !(v.kind == Kind::Lookup && v.equation.trim().is_empty());
                let mut on = self.plotted.contains(&v.name);
                if can_plot && ui.checkbox(&mut on, "").changed() {
                    if on {
                        self.plotted.push(v.name.clone());
                    } else {
                        self.plotted.retain(|n| n != &v.name);
                    }
                }
                let (r, _) = ui.allocate_exact_size(vec2(10.0, 14.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 3.5, pal.kind(v.kind));
                let mut text = RichText::new(&v.name).size(13.0);
                if self.is_sel(crate::app::Sel::Var(v.id)) {
                    text = text.color(pal.accent).strong();
                }
                if v.lever {
                    text = text.italics();
                }
                let r = ui.add(egui::Label::new(text).sense(Sense::click()));
                if let Some(sn) = &v.set {
                    ui.label(RichText::new(format!("×{}", self.model.sets.iter().find(|s| &s.name == sn).map(|s| s.items.len()).unwrap_or(0))).small().color(pal.muted));
                }
                if r.clicked() {
                    self.reveal_var(v.id);
                }
                r.on_hover_text(format!("{} · {}", v.kind.label(), if v.units.is_empty() { "no units".to_string() } else { v.units.clone() }));
            });
        }
        let _ = CornerRadius::ZERO;
    }
}
