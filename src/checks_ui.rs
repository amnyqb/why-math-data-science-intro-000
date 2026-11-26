//! The "Model checks" window.

use crate::checks::*;
use crate::expr;

impl crate::app::App {
    /// The "Model checks" window: limits the results must respect, and whether they hold now.
    pub fn checks_window(&mut self, ctx: &egui::Context) {
        if !self.checks_open {
            return;
        }
        let pal = self.pal;
        let mut open = true;
        let mut changed = false;
        let mut list = self.model.checks.clone();
        let failing: Vec<Failure> = self.check_failures();
        let names: Vec<String> = self.model.vars.iter().map(|v| v.name.clone()).collect();
        egui::Window::new("Model checks").open(&mut open).default_width(620.0).default_height(460.0).show(ctx, |ui| {
            ui.label(egui::RichText::new("Say what “behaves sensibly” means: a variable stays between two limits, always or at the end. A broken limit shows in the Problems tab. Every stock is also checked to change by exactly what flows in and out.").small().color(pal.muted));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                let mut remove = None;
                for (i, c) in list.iter_mut().enumerate() {
                    if c.kind != dynamist_core::checks::CheckKind::Bounds {
                        // other kinds are written with the operations language (Edit → Run operations)
                        let broken = failing.iter().any(|f| f.message.starts_with(&format!("Check “{}”", if c.name.trim().is_empty() { &c.var } else { &c.name })));
                        egui::Frame::new().fill(pal.surface).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(if broken { "●" } else { "✓" }).color(if broken { pal.error } else { pal.ok }));
                                ui.label(egui::RichText::new(if c.name.is_empty() { c.describe() } else { format!("{}: {}", c.name, c.describe()) }));
                                if crate::icons::tool(ui, crate::icons::Ic::Trash, "Delete this check", pal.muted, pal.accent).clicked() {
                                    remove = Some(i);
                                }
                            });
                        });
                        ui.add_space(4.0);
                        continue;
                    }
                    let broken = failing.iter().any(|f| f.message.starts_with(&format!("Check “{}”", if c.name.trim().is_empty() { &c.var } else { &c.name })));
                    egui::Frame::new().fill(pal.surface).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(if broken { "●" } else { "✓" }).color(if broken { pal.error } else { pal.ok }));
                            changed |= ui.add(egui::TextEdit::singleline(&mut c.name).desired_width(260.0).hint_text("What this check says")).changed();
                            if crate::icons::tool(ui, crate::icons::Ic::Trash, "Delete this check", pal.muted, pal.accent).clicked() {
                                remove = Some(i);
                            }
                        });
                        ui.horizontal(|ui| {
                            let known = names.iter().any(|n| expr::normalize(n) == expr::normalize(&c.var));
                            let col = if known || c.var.is_empty() { pal.text } else { pal.error };
                            changed |= ui.add(egui::TextEdit::singleline(&mut c.var).desired_width(190.0).hint_text("Variable").text_color(col)).changed();
                            let mut edit = |ui: &mut egui::Ui, v: &mut Option<f64>, hint: &str| {
                                let mut t = v.map(|x| format!("{x}")).unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut t).desired_width(80.0).hint_text(hint)).changed() {
                                    *v = t.trim().parse().ok();
                                    changed = true;
                                }
                            };
                            ui.label("from");
                            edit(ui, &mut c.min, "no lower limit");
                            ui.label("to");
                            edit(ui, &mut c.max, "no upper limit");
                            let before = c.when;
                            egui::ComboBox::from_id_salt(("check-when", i)).selected_text(if c.when == CheckWhen::End { "at the end" } else { "at all times" }).show_ui(ui, |ui| {
                                ui.selectable_value(&mut c.when, CheckWhen::Always, "at all times");
                                ui.selectable_value(&mut c.when, CheckWhen::End, "at the end");
                            });
                            changed |= before != c.when;
                        });
                        if let Some(f) = failing.iter().find(|f| f.message.starts_with(&format!("Check “{}”", if c.name.trim().is_empty() { &c.var } else { &c.name }))) {
                            ui.label(egui::RichText::new(f.message.split_once(": ").map(|x| x.1).unwrap_or(&f.message)).small().color(pal.error));
                        }
                    });
                    ui.add_space(4.0);
                }
                if let Some(i) = remove {
                    list.remove(i);
                    changed = true;
                }
            });
            if ui.button("+ New check").clicked() {
                let var = self.sel.and_then(|s| if let crate::app::Sel::Var(id) = s { self.model.var(id).map(|v| v.name.clone()) } else { None }).unwrap_or_default();
                list.push(Check { name: String::new(), var, min: Some(0.0), max: None, when: CheckWhen::Always, ..Default::default() });
                changed = true;
            }
        });
        if changed {
            self.checkpoint();
            self.model.checks = list;
            self.changed();
        }
        if !open {
            self.checks_open = false;
        }
    }
}
