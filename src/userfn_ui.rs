//! The "My functions" window.

use egui::RichText;

use crate::app::App;
use crate::expr;
use crate::userfn::*;

impl App {
    /// The "My functions" window.
    pub fn functions_window(&mut self, ctx: &egui::Context) {
        if !self.functions_open {
            return;
        }
        let pal = self.pal;
        let mut open = true;
        let mut changed = false;
        let mut fns = self.model.functions.clone();
        set(&fns);
        egui::Window::new("My functions").open(&mut open).default_width(560.0).default_height(520.0).show(ctx, |ui| {
            ui.label(RichText::new("Define a formula once and use it like a built-in: Name(value, …). Write the formula with the parameter names; it can use other functions, variables and TIME.").small().color(pal.muted));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                let mut remove = None;
                for (i, f) in fns.iter_mut().enumerate() {
                    egui::Frame::new().fill(pal.surface).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::TextEdit::singleline(&mut f.name).desired_width(170.0).hint_text("Name")).changed();
                            ui.label("(");
                            let mut ps = f.params.join(", ");
                            if ui.add(egui::TextEdit::singleline(&mut ps).desired_width(190.0).hint_text("parameters, separated by commas")).changed() {
                                f.params = ps.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                                changed = true;
                            }
                            ui.label(")");
                            if crate::icons::tool(ui, crate::icons::Ic::Trash, "Delete this function", pal.muted, pal.accent).clicked() {
                                remove = Some(i);
                            }
                        });
                        changed |= ui.add(egui::TextEdit::multiline(&mut f.body).desired_rows(2).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace).hint_text("formula, e.g.  1 - 0.3 * LN(p)")).changed();
                        changed |= ui.add(egui::TextEdit::singleline(&mut f.description).desired_width(f32::INFINITY).hint_text("What it is for")).changed();
                        // does a call of it read correctly?
                        if !f.name.trim().is_empty() && !f.body.trim().is_empty() {
                            let call = format!("{}({})", f.name, f.params.iter().map(|_| "1").collect::<Vec<_>>().join(", "));
                            let verdict = expand_text(&call).and_then(|t| expr::parse(&t).map(|_| ()));
                            match verdict {
                                Ok(()) => ui.label(RichText::new("✓ reads correctly").small().color(pal.ok)),
                                Err(e) => ui.label(RichText::new(format!("⚠ {e}")).small().color(pal.error)),
                            };
                        }
                    });
                    ui.add_space(4.0);
                }
                if let Some(i) = remove {
                    fns.remove(i);
                    changed = true;
                }
            });
            if ui.button("+ New function").clicked() {
                fns.push(UserFn { name: format!("My function {}", fns.len() + 1), params: vec!["x".into()], body: "x".into(), description: String::new() });
                changed = true;
            }
        });
        if changed {
            self.checkpoint();
            self.model.functions = fns;
            set(&self.model.functions);
            self.changed();
        } else {
            set(&self.model.functions);
        }
        if !open {
            self.functions_open = false;
        }
    }
}
