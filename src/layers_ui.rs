//! Layers in the app: the Layers window (edit the stack, move things between layers, declare
//! interfaces, run a layer alone, the decoupling test) and the coloured tags on the diagram.
//! Every change is made with the operations language, so it is logged and can be undone.

use std::collections::HashMap;
use std::rc::Rc;

use egui::{Color32, RichText};
use dynamist_core::layers::{self, LayerReport, Sev};
use dynamist_core::model::Kind;
use dynamist_core::ops::Op;

use crate::app::{App, Sel};

/// What the diagram needs to know about layers, worked out once per change of the model.
#[derive(Default)]
pub struct LayerView {
    /// variable id → (layer index in `colours`, names of the layers it feeds, undeclared crossing?)
    pub tags: HashMap<u64, (usize, Vec<String>, bool)>,
    pub names: Vec<String>,
    pub colours: Vec<Color32>,
    pub report: LayerReport,
}

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub bufs: HashMap<String, String>,
    pub move_to: Option<u64>,
    /// The last result of "Run alone" or the decoupling test: (heading, lines).
    pub output: Option<(String, Vec<String>)>,
}

fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.trim().trim_start_matches('#');
    (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten().map(|v| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

impl App {
    pub fn layer_colour(&self, idx: usize, l: &layers::Layer) -> Color32 {
        parse_hex(&l.colour).unwrap_or_else(|| self.pal.series(idx))
    }

    /// The layer tags of the diagram (cached per revision of the model).
    pub fn layer_view(&self) -> Rc<LayerView> {
        if let Some((rev, v)) = &*self.layer_cache.borrow() {
            if *rev == self.rev {
                return v.clone();
            }
        }
        let m = &self.model;
        let mut lv = LayerView::default();
        if !m.layers.is_empty() {
            let mut order: Vec<&layers::Layer> = m.layers.iter().collect();
            order.sort_by_key(|l| (l.order, l.id));
            let idx_of: HashMap<u64, usize> = order.iter().enumerate().map(|(i, l)| (l.id, i)).collect();
            lv.names = order.iter().map(|l| l.name.clone()).collect();
            lv.colours = order.iter().enumerate().map(|(i, l)| self.layer_colour(i, l)).collect();
            lv.report = layers::check(m);
            let bad: Vec<&str> = lv.report.issues.iter().filter(|i| i.code == "layer-crossing").filter_map(|i| i.var.as_deref()).collect();
            for v in &m.vars {
                if let Some(i) = layers::layer_of(m, v).and_then(|id| idx_of.get(&id).copied()) {
                    lv.tags.insert(v.id, (i, Vec::new(), false));
                }
            }
            for c in layers::crossings(m) {
                if let Some(v) = m.vars.iter().find(|v| dynamist_core::expr::normalize(&v.name) == dynamist_core::expr::normalize(&c.var)) {
                    if let Some(t) = lv.tags.get_mut(&v.id) {
                        let to = m.layer_name(c.to);
                        if !t.1.contains(&to) {
                            t.1.push(to);
                        }
                        t.2 |= bad.contains(&c.var.as_str());
                    }
                }
            }
        }
        let v = Rc::new(lv);
        *self.layer_cache.borrow_mut() = Some((self.rev, v.clone()));
        v
    }

    /// The small coloured tag under a variable's top edge: its layer, and where it feeds.
    pub fn layer_badge(&self, p: &egui::Painter, sr: egui::Rect, v: &dynamist_core::model::Var, z: f32) {
        if self.model.layers.is_empty() {
            return;
        }
        let lv = self.layer_view();
        let Some((i, feeds, undeclared)) = lv.tags.get(&v.id) else { return };
        let col = lv.colours[*i];
        let mut text = lv.names[*i].clone();
        if !feeds.is_empty() {
            text.push_str(&format!(" → {}", feeds.join(", ")));
        }
        let g = p.layout_no_wrap(text, egui::FontId::proportional(9.0 * z), self.pal.text);
        let r = egui::Rect::from_min_size(egui::pos2(sr.left() + 6.0 * z, sr.bottom() - 4.0 * z), g.size() + egui::vec2(10.0 * z, 3.0 * z));
        p.rect_filled(r, egui::CornerRadius::same((6.0 * z).min(255.0) as u8), col.gamma_multiply(0.30));
        let edge = if *undeclared { self.pal.error } else { col };
        p.rect_stroke(r, egui::CornerRadius::same((6.0 * z).min(255.0) as u8), egui::Stroke::new(if *undeclared { 1.6 } else { 1.0 }, edge), egui::StrokeKind::Inside);
        p.galley(r.min + egui::vec2(5.0 * z, 1.5 * z), g, self.pal.text);
    }

    /// Layer errors for the Problems tab: (variable, message). Unassigned variables are left to the Layers window.
    pub fn layer_errors(&self) -> Vec<(Option<String>, String)> {
        if self.model.layers.is_empty() {
            return Vec::new();
        }
        let lv = self.layer_view();
        lv.report.issues.iter().filter(|i| i.severity == Sev::Error && i.code != "layer-unassigned").map(|i| (i.var.clone(), i.message.clone())).collect()
    }

    /// Apply layer operations as a batch (not gated: layering is metadata, and the window shows
    /// what is wrong), logged and undoable.
    pub fn layer_ops(&mut self, ops: Vec<Op>) {
        if ops.is_empty() {
            return;
        }
        let mut s = dynamist_core::ops::Session::new(self.model.clone());
        s.gate = false;
        match s.apply("user:app", &ops) {
            Ok(entry) => {
                let mut e = entry.clone();
                e.seq = self.history.log.last().map(|x| x.seq + 1).unwrap_or(1);
                self.checkpoint();
                self.model = s.model;
                self.changed();
                self.history.log.push(e);
            }
            Err(r) => self.status = format!("Layers: {}", r.reason),
        }
    }

    fn selected_var_names(&self) -> Vec<String> {
        let mut ids: Vec<u64> = self.multi.iter().filter_map(|s| if let Sel::Var(i) = s { Some(*i) } else { None }).collect();
        if let Some(Sel::Var(i)) = self.sel {
            if !ids.contains(&i) {
                ids.push(i);
            }
        }
        ids.into_iter().filter_map(|i| self.model.var(i)).map(|v| v.name.clone()).collect()
    }

    fn layer_test(&mut self, id: u64, decouple: bool) {
        let Some(l) = self.model.layers.iter().find(|l| l.id == id).cloned() else { return };
        let m = self.model.clone();
        let res: Result<(String, Vec<String>), String> = (|| {
            if decouple {
                let v = layers::decouple(&m, &l, 0.05)?;
                let f = |k: &str| v[k].as_f64().unwrap_or(0.0);
                let mut lines = vec![
                    format!("Reads {} variable(s) from other layers.", v["inputs_from_other_layers"]),
                    format!("Alone, on inputs recorded from the full run: gap {:.2e} of range → {}", f("plumbing_nrmse"), if v["ok"].as_bool() == Some(true) { "reproduces its behaviour ✓" } else { "does NOT reproduce it ✗" }),
                    format!("When its own constants change, the other layers react by up to {:.1}% of range → {}", f("feedback_nrmse") * 100.0, if v["decoupled"].as_bool() == Some(true) { "decoupled ✓" } else { "feedback across the boundary" }),
                ];
                for r in v["feedback_by_constant"].as_array().into_iter().flatten().take(6) {
                    lines.push(format!("   change {} → {:.1}% (most affected: {})", r["changed"].as_str().unwrap_or(""), r["nrmse"].as_f64().unwrap_or(0.0) * 100.0, r["most_affected"].as_str().unwrap_or("-")));
                }
                Ok((format!("Decoupling test: {}", l.name), lines))
            } else {
                let full = dynamist_core::runner::run_with(&m, &[])?;
                let inputs = layers::recorded_inputs(&m, l.id, &full)?;
                let run = layers::run_layer(&m, &l, &inputs, &[])?;
                let mut lines = Vec::new();
                for n in layers::behaviour_vars(&m, l.id).into_iter().take(10) {
                    if let Some(s) = run.get(&n) {
                        let (lo, hi) = s.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
                        lines.push(format!("{n}: start {}, end {}, between {} and {}", crate::theme::fmt_num(s[0]), crate::theme::fmt_num(*s.last().unwrap()), crate::theme::fmt_num(lo), crate::theme::fmt_num(hi)));
                    }
                }
                Ok((format!("{} run alone on inputs recorded from the full run", l.name), lines))
            }
        })();
        self.layers.output = Some(res.unwrap_or_else(|e| (format!("{}: could not run", l.name), vec![e])));
    }

    pub fn layers_window(&mut self, ctx: &egui::Context) {
        if !self.layers.open {
            return;
        }
        let pal = self.pal;
        let mut open = true;
        let mut ops: Vec<Op> = Vec::new();
        let mut select: Option<u64> = None;
        let mut test: Option<(u64, bool)> = None;
        let mut bufs = std::mem::take(&mut self.layers.bufs);
        let mut move_to = self.layers.move_to;
        let mut order: Vec<layers::Layer> = self.model.layers.clone();
        order.sort_by_key(|l| (l.order, l.id));
        let counts: HashMap<u64, usize> = order.iter().map(|l| (l.id, self.model.vars.iter().filter(|v| layers::layer_of(&self.model, v) == Some(l.id)).count())).collect();
        let unassigned = self.model.vars.iter().filter(|v| layers::layer_of(&self.model, v).is_none()).count();
        let selected = self.selected_var_names();
        let sel_module = if let Some(Sel::Module(id)) = self.sel { self.model.modules.iter().find(|m| m.id == id).map(|m| m.name.clone()) } else { None };
        let sheet_name = self.model.sheets.iter().find(|s| s.id == self.sheet).map(|s| s.name.clone());
        let lv = self.layer_view();
        let report = if self.model.layers.is_empty() { LayerReport::default() } else { lv.report.clone() };
        let n_contracts = self.model.interfaces.len();
        let output = self.layers.output.clone();
        let mut dismiss_output = false;
        let statuses = ["planned", "stub", "built", "tested", "calibrated"];
        egui::Window::new("Layers").open(&mut open).default_width(640.0).default_height(560.0).show(ctx, |ui| {
            ui.label(RichText::new("A large model is a stack of layers, outermost (slow, uncertain) to innermost (fast, detailed). Each layer has its own question, horizon and state of completion, and layers meet only through declared variables. Layers never change the equations.").small().color(pal.muted));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for (idx, l) in order.iter().enumerate() {
                    let col = self.layer_colour(idx, l);
                    egui::Frame::new().fill(pal.surface).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).stroke(egui::Stroke::new(1.5, col.gamma_multiply(0.8))).show(ui, |ui| {
                        let set = |ops: &mut Vec<Op>, key: &str, value: String| ops.push(Op::SetLayer { name: l.name.clone(), key: key.into(), value });
                        ui.horizontal(|ui| {
                            let mut c = col;
                            if ui.color_edit_button_srgba(&mut c).changed() {
                                set(&mut ops, "colour", hex(c));
                            }
                            if let Some(v) = text_field(ui, &mut bufs, format!("{}:name", l.id), &l.name, 170.0, "Layer name") {
                                if !v.trim().is_empty() {
                                    set(&mut ops, "name", v);
                                }
                            }
                            let mut st = l.status;
                            egui::ComboBox::from_id_salt(("layer-status", l.id)).selected_text(l.status.label()).width(100.0).show_ui(ui, |ui| {
                                for s in statuses {
                                    let sv = layers::LayerStatus::parse(s).unwrap();
                                    ui.selectable_value(&mut st, sv, s);
                                }
                            });
                            if st != l.status {
                                set(&mut ops, "status", st.label().into());
                            }
                            ui.label(RichText::new(format!("{} variable(s)", counts[&l.id])).small().color(pal.muted));
                            if crate::icons::tool(ui, crate::icons::Ic::Trash, "Delete this layer (its variables become unassigned)", pal.muted, pal.accent).clicked() {
                                ops.push(Op::RemoveLayer { name: l.name.clone() });
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Question");
                            if let Some(v) = text_field(ui, &mut bufs, format!("{}:question", l.id), &l.question, 430.0, "The one question this layer answers") {
                                set(&mut ops, "question", v);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Trusted for");
                            let cur = l.horizon.map(|x| format!("{x}")).unwrap_or_default();
                            if let Some(v) = text_field(ui, &mut bufs, format!("{}:horizon", l.id), &cur, 60.0, "∞") {
                                if v.trim().is_empty() || v.trim().parse::<f64>().is_ok() {
                                    set(&mut ops, "horizon", v);
                                }
                            }
                            ui.label(format!("{} from the start; structure only after", self.model.settings.time_units.to_lowercase()));
                            let cur = l.structural_limit.map(|x| format!("{x}")).unwrap_or_default();
                            if let Some(v) = text_field(ui, &mut bufs, format!("{}:limit", l.id), &cur, 60.0, "∞") {
                                if v.trim().is_empty() || v.trim().parse::<f64>().is_ok() {
                                    set(&mut ops, "structural_limit", v);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.small_button("Select its variables").clicked() {
                                select = Some(l.id);
                            }
                            if ui.small_button("Run alone").on_hover_text("Run this layer by itself on the values the other layers gave it in the full run").clicked() {
                                test = Some((l.id, false));
                            }
                            if ui.small_button("Decoupling test").on_hover_text("Does the layer reproduce itself alone, and do the other layers react when its constants change?").clicked() {
                                test = Some((l.id, true));
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
                if order.is_empty() {
                    ui.label(RichText::new("No layers yet. Add the outermost one first (for example “World”), then the ones inside it.").color(pal.muted));
                }
            });
            if ui.button("+ New layer").clicked() {
                let n = order.len();
                ops.push(Op::AddLayer { name: format!("Layer {}", n + 1), order: Some(n as i32), question: String::new(), horizon: None, structural_limit: None, status: None, colour: String::new(), description: String::new(), spheres: Vec::new() });
            }
            if !order.is_empty() {
                ui.separator();
                ui.label(RichText::new("Put things in a layer").strong());
                ui.horizontal_wrapped(|ui| {
                    let cur = move_to.filter(|id| order.iter().any(|l| l.id == *id)).unwrap_or(order[0].id);
                    let mut chosen = cur;
                    egui::ComboBox::from_id_salt("layer-move-to").selected_text(order.iter().find(|l| l.id == cur).map(|l| l.name.clone()).unwrap_or_default()).show_ui(ui, |ui| {
                        for l in &order {
                            ui.selectable_value(&mut chosen, l.id, &l.name);
                        }
                    });
                    move_to = Some(chosen);
                    let lname = order.iter().find(|l| l.id == chosen).map(|l| l.name.clone()).unwrap_or_default();
                    ui.add_enabled_ui(!selected.is_empty(), |ui| {
                        if ui.button(format!("Move {} selected variable(s)", selected.len())).clicked() {
                            ops.push(Op::AssignLayer { layer: lname.clone(), vars: selected.clone(), sheet: None, module: None });
                        }
                    });
                    if let Some(s) = &sheet_name {
                        if ui.button(format!("Move sheet “{s}”")).on_hover_text("Everything on the sheet joins the layer unless it says otherwise").clicked() {
                            ops.push(Op::AssignLayer { layer: lname.clone(), vars: Vec::new(), sheet: Some(s.clone()), module: None });
                        }
                    }
                    if let Some(mn) = &sel_module {
                        if ui.button(format!("Move submodel “{mn}”")).clicked() {
                            ops.push(Op::AssignLayer { layer: lname.clone(), vars: Vec::new(), sheet: None, module: Some(mn.clone()) });
                        }
                    }
                    if unassigned > 0 && ui.button(format!("Move all {unassigned} unassigned")).clicked() {
                        let names: Vec<String> = self.model.vars.iter().filter(|v| layers::layer_of(&self.model, v).is_none()).map(|v| v.name.clone()).collect();
                        ops.push(Op::AssignLayer { layer: lname.clone(), vars: names, sheet: None, module: None });
                    }
                });
                ui.label(RichText::new("Select variables on the diagram first (click, or drag a box). Each variable shows a tag with its layer and the layers it feeds; a red tag is a crossing nobody declared.").small().color(pal.muted));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Interfaces").strong());
                    ui.label(RichText::new(format!("{} crossing(s), {} declared", report.crossings, n_contracts)).color(pal.muted));
                    if ui.button("Declare the crossings as interfaces").on_hover_text("Write down every variable that passes between layers. After that, a new undeclared crossing is an error.").clicked() {
                        ops.push(Op::FreezeInterfaces {});
                    }
                });
                egui::ScrollArea::vertical().id_salt("layer-issues").max_height(110.0).show(ui, |ui| {
                    if report.issues.is_empty() {
                        ui.label(RichText::new("✓ Every variable is in a layer and every crossing is declared.").color(pal.ok));
                    }
                    for i in &report.issues {
                        let (mark, col) = if i.severity == Sev::Error { ("●", pal.error) } else { ("▲", pal.aux) };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(mark).color(col));
                            ui.label(RichText::new(&i.message).small());
                        });
                    }
                });
                if !report.cross_layer_loops.is_empty() {
                    ui.label(RichText::new(format!("Layers that feed each other: {}", report.cross_layer_loops.iter().map(|g| g.join(" ↔ ")).collect::<Vec<_>>().join("; "))).small().color(pal.muted));
                }
            }
            if let Some((head, lines)) = &output {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(head).strong());
                    if ui.small_button("Close").clicked() {
                        dismiss_output = true;
                    }
                });
                for l in lines {
                    ui.label(RichText::new(l).monospace().small());
                }
            }
        });
        self.layers.bufs = bufs;
        self.layers.move_to = move_to;
        if dismiss_output {
            self.layers.output = None;
        }
        if let Some(id) = select {
            let ids: Vec<Sel> = self.model.vars.iter().filter(|v| layers::layer_of(&self.model, v) == Some(id)).map(|v| Sel::Var(v.id)).collect();
            self.sel = ids.first().copied();
            self.multi = ids;
        }
        if let Some((id, d)) = test {
            self.layer_test(id, d);
        }
        self.layer_ops(ops);
        if !open {
            self.layers.open = false;
        }
    }
}

/// A one-line text box that reports its new text only when the person leaves it.
fn text_field(ui: &mut egui::Ui, bufs: &mut HashMap<String, String>, key: String, current: &str, width: f32, hint: &str) -> Option<String> {
    let buf = bufs.entry(key).or_insert_with(|| current.to_string());
    let r = ui.add(egui::TextEdit::singleline(buf).desired_width(width).hint_text(hint));
    let out = (r.lost_focus() && buf.as_str() != current).then(|| buf.clone());
    if !r.has_focus() {
        *buf = current.to_string();
    }
    out
}

#[allow(dead_code)]
fn _kinds(_: Kind) {}

#[cfg(test)]
mod tests {
    use super::*;
    use dynamist_core::model::Model;

    fn app() -> App {
        let ctx = egui::Context::default();
        let mut a = App::new(&ctx, None);
        let mut m = Model::default();
        m.settings.final_time = 30.0;
        m.settings.time_step = 1.0;
        m.ensure_sheets();
        let batch: Vec<Op> = serde_json::from_str(
            r#"[{"op":"add_var","name":"Firm","kind":"stock","equation":"10"},
                {"op":"add_var","name":"Climate","kind":"aux","equation":"1 + 0.02 * TIME"},
                {"op":"add_var","name":"Rate","kind":"constant","equation":"0.05"},
                {"op":"add_var","name":"Growth","kind":"flow","equation":"Firm * Rate * Climate","to":"Firm"}]"#,
        )
        .unwrap();
        for op in &batch {
            dynamist_core::ops::apply(&mut m, op).unwrap();
        }
        a.finish_open(m);
        a
    }

    #[test]
    fn the_panel_builds_a_stack_assigns_declares_and_tests() {
        let mut a = app();
        a.layer_ops(vec![
            Op::AddLayer { name: "World".into(), order: Some(0), question: String::new(), horizon: Some(30.0), structural_limit: None, status: Some("built".into()), colour: String::new(), description: String::new(), spheres: vec![] },
            Op::AddLayer { name: "Firm".into(), order: Some(1), question: String::new(), horizon: None, structural_limit: None, status: Some("built".into()), colour: "#336699".into(), description: String::new(), spheres: vec![] },
        ]);
        assert_eq!(a.model.layers.len(), 2);
        a.layer_ops(vec![Op::AssignLayer { layer: "World".into(), vars: vec!["Climate".into()], sheet: None, module: None }, Op::AssignLayer { layer: "Firm".into(), vars: vec!["Firm".into(), "Rate".into(), "Growth".into()], sheet: None, module: None }]);
        assert!(a.layer_errors().is_empty());
        let lv = a.layer_view();
        let climate = a.model.vars.iter().find(|v| v.name == "Climate").unwrap().id;
        assert_eq!(lv.tags[&climate].1, vec!["Firm".to_string()], "Climate is tagged as feeding Firm");
        // declare, then break it: an undeclared crossing is an error shown in the problems list
        a.layer_ops(vec![Op::FreezeInterfaces {}]);
        assert_eq!(a.model.interfaces.len(), 1);
        a.layer_ops(vec![Op::SetEquation { name: "Climate".into(), equation: "1 + 0.0001 * Firm".into() }]);
        assert!(a.layer_errors().iter().any(|(_, m)| m.contains("Firm") && m.contains("World")));
        // undo takes the whole step back
        a.undo();
        assert_eq!(a.model.vars.iter().find(|v| v.name == "Climate").unwrap().equation, "1 + 0.02 * TIME");
        // the tests behind the buttons
        let firm = a.model.layers[1].id;
        a.layer_test(firm, true);
        let (head, lines) = a.layers.output.clone().unwrap();
        assert!(head.starts_with("Decoupling test"), "{head}");
        assert!(lines.iter().any(|l| l.contains("reproduces")), "{lines:?}");
        a.layer_test(firm, false);
        assert!(a.layers.output.clone().unwrap().1.iter().any(|l| l.starts_with("Firm:")));
        // the log has the batches, made by the app
        assert!(a.history.log.iter().any(|e| e.actor == "user:app"));
    }

    #[test]
    fn pipes_are_never_diagonal() {
        let mut a = app();
        // a flow whose loose end was saved off to the side
        let id = a.model.vars.iter().find(|v| v.name == "Growth").unwrap().id;
        a.model.var_mut(id).unwrap().from_pt = [-300.0, -170.0];
        a.straighten_flows(true);
        let f = a.model.var(id).unwrap().clone();
        let s = a.model.var(f.to.unwrap()).unwrap();
        assert!(f.from_pt[0] == s.pos[0] || f.from_pt[1] == s.pos[1], "the cloud lines up with its stock: {:?} vs {:?}", f.from_pt, s.pos);
    }
}
