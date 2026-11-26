//! The "Reusable modules" window: define a module once, with declared ports, and use it as many
//! times as needed with its own parameter values and inputs. Instances have no box on the
//! diagram; their variables are written `Instance.Variable`, can be plotted from here, and can be
//! read in any equation as "Instance.Variable".

use egui::RichText;
use dynamist_core::kinds::{kind_label, kind_options};
use dynamist_core::model::Kind;
use dynamist_core::modules::{DefVar, Instance, ModuleDef, PortDef, PortDirection, PortKind};

use crate::app::App;

fn kind_combo(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, k: &mut Kind) -> bool {
    let before = *k;
    egui::ComboBox::from_id_salt(id).width(86.0).selected_text(kind_label(*k)).show_ui(ui, |ui| {
        for opt in kind_options() {
            ui.selectable_value(k, opt, kind_label(opt));
        }
    });
    before != *k
}

impl App {
    pub fn modules_window(&mut self, ctx: &egui::Context) {
        if !self.modules_open {
            return;
        }
        let pal = self.pal;
        let mut open = true;
        let mut changed = false;
        let mut defs = self.model.defs.clone();
        let mut insts = self.model.instances.clone();
        let problems: Vec<(String, String)> = dynamist_core::modules::check_defs(&self.model).into_iter().chain(dynamist_core::modules::check_instances(&self.model)).collect();
        let warnings = dynamist_core::modules::unit_warnings(&self.model);
        let mut plot: Option<String> = None;
        egui::Window::new("Reusable modules").open(&mut open).default_width(900.0).default_height(640.0).show(ctx, |ui| {
            ui.label(RichText::new("Define a piece of model once, with named ports, and use it several times. An instance has its own parameter values and its own inputs; its variables are called Instance.Variable.").small().color(pal.muted));
            ui.add_space(4.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                // ---------------------------------------------------------- definitions
                ui.label(RichText::new("Modules").strong());
                let mut remove_def = None;
                for (di, d) in defs.iter_mut().enumerate() {
                    egui::Frame::new().fill(pal.surface).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(180.0).hint_text("Module name")).changed();
                            changed |= ui.add(egui::TextEdit::singleline(&mut d.description).desired_width(330.0).hint_text("What it is for")).changed();
                            if crate::icons::tool(ui, crate::icons::Ic::Trash, "Delete this module", pal.muted, pal.accent).clicked() {
                                remove_def = Some(di);
                            }
                        });
                        ui.label(RichText::new("VARIABLES").small().color(pal.muted));
                        let mut remove_var = None;
                        {
                            for (vi, v) in d.vars.iter_mut().enumerate() {
                              ui.horizontal(|ui| {
                                changed |= ui.add(egui::TextEdit::singleline(&mut v.name).desired_width(140.0)).changed();
                                changed |= kind_combo(ui, ("kind", di, vi), &mut v.kind);
                                changed |= ui.add(egui::TextEdit::singleline(&mut v.equation).desired_width(240.0).font(egui::TextStyle::Monospace)).changed();
                                changed |= ui.add(egui::TextEdit::singleline(&mut v.units).desired_width(100.0).hint_text("units")).changed();
                                if v.kind == Kind::Flow {
                                    let mut ends = format!("{} → {}", v.from.clone().unwrap_or_default(), v.to.clone().unwrap_or_default());
                                    let r = ui.add(egui::TextEdit::singleline(&mut ends).desired_width(150.0).hint_text("from → to"));
                                    if r.changed() {
                                        let (a, b) = ends.split_once('→').unwrap_or((&ends, ""));
                                        v.from = Some(a.trim().to_string()).filter(|x| !x.is_empty());
                                        v.to = Some(b.trim().to_string()).filter(|x| !x.is_empty());
                                        changed = true;
                                    }
                                } else {
                                    ui.label("");
                                }
                                if crate::icons::tool(ui, crate::icons::Ic::Close, "Remove this variable", pal.muted, pal.accent).clicked() {
                                    remove_var = Some(vi);
                                }
                              });
                            }
                        }
                        if let Some(i) = remove_var {
                            d.vars.remove(i);
                            changed = true;
                        }
                        if ui.small_button("+ Variable").clicked() {
                            d.vars.push(DefVar { name: format!("Variable {}", d.vars.len() + 1), kind: Kind::Aux, equation: "0".into(), units: String::new(), doc: String::new(), from: None, to: None, non_negative: false });
                            changed = true;
                        }
                        ui.label(RichText::new("PORTS").small().color(pal.muted));
                        let mut remove_port = None;
                        {
                            for (pi, p) in d.ports.iter_mut().enumerate() {
                              ui.horizontal(|ui| {
                                changed |= ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(140.0)).changed();
                                let before = p.direction;
                                egui::ComboBox::from_id_salt(("dir", di, pi)).width(60.0).selected_text(if p.direction == PortDirection::In { "in" } else { "out" }).show_ui(ui, |ui| {
                                    ui.selectable_value(&mut p.direction, PortDirection::In, "in");
                                    ui.selectable_value(&mut p.direction, PortDirection::Out, "out");
                                });
                                changed |= before != p.direction;
                                let before = p.kind;
                                egui::ComboBox::from_id_salt(("pk", di, pi)).width(100.0).selected_text(if p.kind == PortKind::Material { "material" } else { "information" }).show_ui(ui, |ui| {
                                    ui.selectable_value(&mut p.kind, PortKind::Material, "material");
                                    ui.selectable_value(&mut p.kind, PortKind::Information, "information");
                                });
                                changed |= before != p.kind;
                                changed |= ui.add(egui::TextEdit::singleline(&mut p.units).desired_width(100.0).hint_text("units")).changed();
                                let known = d.vars.iter().any(|v| dynamist_core::expr::normalize(&v.name) == dynamist_core::expr::normalize(&p.var));
                                let col = if known || p.var.is_empty() { pal.text } else { pal.error };
                                changed |= ui.add(egui::TextEdit::singleline(&mut p.var).desired_width(140.0).hint_text("variable").text_color(col)).changed();
                                if crate::icons::tool(ui, crate::icons::Ic::Close, "Remove this port", pal.muted, pal.accent).clicked() {
                                    remove_port = Some(pi);
                                }
                              });
                            }
                        }
                        if let Some(i) = remove_port {
                            d.ports.remove(i);
                            changed = true;
                        }
                        if ui.small_button("+ Port").clicked() {
                            d.ports.push(PortDef { name: format!("Port {}", d.ports.len() + 1), direction: PortDirection::In, kind: PortKind::Information, units: String::new(), var: String::new() });
                            changed = true;
                        }
                    });
                    ui.add_space(4.0);
                }
                if let Some(i) = remove_def {
                    defs.remove(i);
                    changed = true;
                }
                if ui.button("+ New module").clicked() {
                    defs.push(ModuleDef { name: format!("Module {}", defs.len() + 1), description: String::new(), vars: Vec::new(), ports: Vec::new() });
                    changed = true;
                }
                // ---------------------------------------------------------- instances
                ui.add_space(10.0);
                ui.label(RichText::new("Instances").strong());
                let mut remove_inst = None;
                for (ii, inst) in insts.iter_mut().enumerate() {
                    egui::Frame::new().fill(pal.surface).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::TextEdit::singleline(&mut inst.name).desired_width(140.0).hint_text("Instance name")).changed();
                            ui.label("is a");
                            egui::ComboBox::from_id_salt(("inst-def", ii)).width(160.0).selected_text(&inst.def).show_ui(ui, |ui| {
                                for d in defs.iter() {
                                    if ui.selectable_label(inst.def == d.name, &d.name).clicked() {
                                        inst.def = d.name.clone();
                                        changed = true;
                                    }
                                }
                            });
                            if crate::icons::tool(ui, crate::icons::Ic::Trash, "Delete this instance", pal.muted, pal.accent).clicked() {
                                remove_inst = Some(ii);
                            }
                        });
                        let Some(d) = defs.iter().find(|d| dynamist_core::expr::normalize(&d.name) == dynamist_core::expr::normalize(&inst.def)) else {
                            ui.label(RichText::new("Choose a module").color(pal.error));
                            return;
                        };
                        {
                            for v in d.vars.iter().filter(|v| v.kind == Kind::Constant) {
                              ui.horizontal(|ui| {
                                ui.label(RichText::new(&v.name).small());
                                ui.label(RichText::new("value").small().color(pal.muted));
                                let mut t = inst.params.get(&v.name).cloned().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut t).desired_width(160.0).hint_text(format!("{} (as defined)", v.equation))).changed() {
                                    if t.trim().is_empty() {
                                        inst.params.remove(&v.name);
                                    } else {
                                        inst.params.insert(v.name.clone(), t);
                                    }
                                    changed = true;
                                }
                              });
                            }
                            for p in d.ports.iter().filter(|p| p.direction == PortDirection::In) {
                              ui.horizontal(|ui| {
                                ui.label(RichText::new(&p.name).small());
                                ui.label(RichText::new(format!("input, {}", if p.units.is_empty() { "no units" } else { &p.units })).small().color(pal.muted));
                                let mut t = inst.inputs.get(&p.name).cloned().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut t).desired_width(260.0).font(egui::TextStyle::Monospace).hint_text("formula from the rest of the model")).changed() {
                                    if t.trim().is_empty() {
                                        inst.inputs.remove(&p.name);
                                    } else {
                                        inst.inputs.insert(p.name.clone(), t);
                                    }
                                    changed = true;
                                }
                              });
                            }
                        }
                        // outputs: the value now, and a button to put it on the graph
                        for p in d.ports.iter().filter(|p| p.direction == PortDirection::Out) {
                            let flat = dynamist_core::modules::flat(&inst.name, &p.var);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{} (output)", p.name)).small());
                                let v = self.run.as_ref().and_then(|r| r.get(&flat)).and_then(|s| s.last()).map(|x| crate::theme::fmt_num(*x)).unwrap_or_else(|| "–".into());
                                ui.label(RichText::new(format!("{v} {}", p.units)).monospace().small().color(pal.accent));
                                if ui.small_button(if self.plotted.contains(&flat) { "on the graph" } else { "plot" }).clicked() {
                                    plot = Some(flat.clone());
                                }
                                ui.label(RichText::new(format!("read it as \"{flat}\"")).small().color(pal.muted));
                            });
                        }
                    });
                    ui.add_space(4.0);
                }
                if let Some(i) = remove_inst {
                    insts.remove(i);
                    changed = true;
                }
                if ui.add_enabled(!defs.is_empty(), egui::Button::new("+ New instance")).clicked() {
                    insts.push(Instance { name: format!("Instance {}", insts.len() + 1), def: defs[0].name.clone(), params: Default::default(), inputs: Default::default() });
                    changed = true;
                }
                // ---------------------------------------------------------- what is wrong
                if !problems.is_empty() || !warnings.is_empty() {
                    ui.add_space(8.0);
                    for (who, msg) in &problems {
                        ui.label(RichText::new(format!("● {who}: {msg}")).color(pal.error));
                    }
                    for (who, msg) in &warnings {
                        ui.label(RichText::new(format!("• {who}: {msg}")).color(pal.aux));
                    }
                } else if !insts.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("✓ Every instance is wired correctly.").color(pal.ok));
                }
            });
        });
        if let Some(flat) = plot {
            if let Some(i) = self.plotted.iter().position(|n| *n == flat) {
                self.plotted.remove(i);
            } else {
                self.plotted.push(flat);
            }
        }
        if changed {
            self.checkpoint();
            self.model.defs = defs;
            self.model.instances = insts;
            self.changed();
        }
        if !open {
            self.modules_open = false;
        }
    }
}
