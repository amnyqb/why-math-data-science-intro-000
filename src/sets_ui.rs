//! The "Showing" bar.

use crate::model::{ItemSet, Model};
use crate::sets::*;
use crate::app::App;

impl App {
    /// The "Showing" bar: one dropdown per set. Pick a tank and the whole model — numbers on the
    /// diagram, graphs, sliders — is about that tank; pick a region and it is about its tanks.
    pub fn focus_bar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let sets: Vec<ItemSet> = self.model.sets.clone();
        let mut changed = false;
        ui.horizontal_centered(|ui| {
            ui.label(egui::RichText::new("Showing").small().color(pal.muted));
            // groups first (Region), then the sets that belong to them (Tank)
            let mut order: Vec<&ItemSet> = sets.iter().filter(|s| s.parent.is_none()).collect();
            order.extend(sets.iter().filter(|s| s.parent.is_some()));
            for set in order {
                let cur = self.focus.get(&set.name).cloned();
                // only the items of the chosen group when the set belongs to one
                let allowed: Vec<&String> = (0..set.items.len())
                    .filter(|&i| match (&set.parent, set.parent.as_ref().and_then(|p| self.focus.get(p))) {
                        (Some(_), Some(g)) => set.parent_item(i) == Some(g.as_str()),
                        _ => true,
                    })
                    .map(|i| &set.items[i])
                    .collect();
                ui.label(egui::RichText::new(&set.name).color(pal.text));
                egui::ComboBox::from_id_salt(("focus", set.id))
                    .width(190.0)
                    .selected_text(cur.clone().unwrap_or_else(|| format!("All ({})", allowed.len())))
                    .show_ui(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                            if ui.selectable_label(cur.is_none(), "All (totals)").clicked() {
                                self.focus.remove(&set.name);
                                changed = true;
                            }
                            for it in allowed {
                                if ui.selectable_label(cur.as_ref() == Some(it), it).clicked() {
                                    self.focus.insert(set.name.clone(), it.clone());
                                    // choosing a tank also shows its region
                                    if let Some(i) = set.items.iter().position(|x| x == it) {
                                        if let (Some(p), Some(g)) = (&set.parent, set.parent_item(i)) {
                                            self.focus.insert(p.clone(), g.to_string());
                                        }
                                    }
                                    changed = true;
                                }
                            }
                        });
                    });
            }
            if !self.focus.is_empty() && crate::icons::tool(ui, crate::icons::Ic::Close, "Back to totals", pal.muted, pal.accent).clicked() {
                self.focus.clear();
                changed = true;
            }
            ui.label(egui::RichText::new(if self.focus.is_empty() { "— each box shows the total of its items" } else { "— numbers, graphs and sliders are for this selection" }).small().color(pal.muted));
        });
        if changed {
            // a region change can leave a tank from another region selected
            let sets = self.model.sets.clone();
            for set in &sets {
                if let (Some(item), Some(p)) = (self.focus.get(&set.name).cloned(), &set.parent) {
                    if let (Some(g), Some(i)) = (self.focus.get(p).cloned(), set.items.iter().position(|x| *x == item)) {
                        if set.parent_item(i) != Some(g.as_str()) {
                            self.focus.remove(&set.name);
                        }
                    }
                }
            }
            self.simulate();
        }
    }
}
