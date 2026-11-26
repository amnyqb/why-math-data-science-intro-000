//! Module files: save a module (a submodel with inputs and outputs) to a `.flowmodule`
//! file and bring it into any other model, connecting its inputs on the way in.
#[cfg(not(feature = "gui"))]
use crate::rfd;

use std::collections::HashMap;

use egui::{RichText, pos2};
use serde::{Deserialize, Serialize};

use crate::app::{App, Sel};
use crate::expr;
use crate::model::{Group, Kind, Link, Module, Note, Sheet, Var};

pub const FORMAT: &str = "dynamist-module";

/// An arrow from an input into the module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InLink {
    pub to: u64,
    pub bend: f32,
    pub polarity: String,
}

/// Something the module needs from outside.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortIn {
    pub name: String,
    pub units: String,
    /// A sensible default if nothing in the new model provides it.
    pub value: String,
    pub links: Vec<InLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleFile {
    pub format: String,
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The module's own sheet; nested modules bring their sheets too.
    pub root: u64,
    pub sheets: Vec<Sheet>,
    pub vars: Vec<Var>,
    pub links: Vec<Link>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub modules: Vec<Module>,
    pub inputs: Vec<PortIn>,
    pub outputs: Vec<String>,
}

/// How one input gets connected when importing.
#[derive(Clone, Debug)]
pub struct InputChoice {
    /// An existing variable to use, or None to create a new constant.
    pub target: Option<u64>,
    pub value: String,
}

pub struct ImportDialog {
    pub file: ModuleFile,
    pub choices: Vec<InputChoice>,
}

impl App {
    /// Everything a module holds, as a file.
    pub fn module_file(&self, mid: u64) -> Option<ModuleFile> {
        let m = self.model.module(mid)?.clone();
        // its sheet and the sheets of modules nested inside it
        let mut sheets = vec![m.inner];
        let mut i = 0;
        while i < sheets.len() {
            let s = sheets[i];
            for x in self.model.modules.iter().filter(|x| x.view == s) {
                if !sheets.contains(&x.inner) {
                    sheets.push(x.inner);
                }
            }
            i += 1;
        }
        let inside = |view: u64| sheets.contains(&view);
        let ids: Vec<u64> = self.model.vars.iter().filter(|v| inside(v.view)).map(|v| v.id).collect();
        let mut vars = Vec::new();
        for v in self.model.vars.iter().filter(|v| inside(v.view)) {
            let mut v = v.clone();
            // a flow tied to a stock that stays behind keeps a cloud at that end
            for to_end in [false, true] {
                let end = if to_end { v.to } else { v.from };
                if let Some(sid) = end.filter(|s| !ids.contains(s)) {
                    let p = self.model.var(sid).map(|s| s.pos).unwrap_or(v.pos);
                    if to_end {
                        v.to = None;
                        v.to_pt = [p[0] - 70.0, p[1]];
                    } else {
                        v.from = None;
                        v.from_pt = [p[0] + 70.0, p[1]];
                    }
                }
            }
            vars.push(v);
        }
        let links: Vec<Link> = self.model.links.iter().filter(|l| ids.contains(&l.from) && ids.contains(&l.to)).cloned().collect();
        let mut inputs: Vec<PortIn> = Vec::new();
        for l in self.model.links.iter().filter(|l| ids.contains(&l.to) && !ids.contains(&l.from)) {
            let Some(src) = self.model.var(l.from) else { continue };
            let value = if src.kind == Kind::Constant { src.equation.clone() } else { self.value_now(&src.name).map(|x| format!("{x}")).unwrap_or_else(|| "0".into()) };
            let entry = match inputs.iter_mut().position(|p| p.name == src.name) {
                Some(i) => &mut inputs[i],
                None => {
                    inputs.push(PortIn { name: src.name.clone(), units: src.units.clone(), value, links: Vec::new() });
                    inputs.last_mut().unwrap()
                }
            };
            entry.links.push(InLink { to: l.to, bend: l.bend, polarity: l.polarity.clone() });
        }
        let outputs = self.module_ports(mid).1.iter().filter_map(|i| self.model.var(*i)).map(|v| v.name.clone()).collect();
        Some(ModuleFile {
            format: FORMAT.into(),
            version: 1,
            name: m.name.clone(),
            description: m.description.clone(),
            root: m.inner,
            sheets: self.model.sheets.iter().filter(|s| sheets.contains(&s.id)).cloned().collect(),
            vars,
            links,
            notes: self.model.notes.iter().filter(|n| inside(n.view)).cloned().collect(),
            groups: self.model.groups.iter().filter(|g| inside(g.view)).cloned().collect(),
            modules: self.model.modules.iter().filter(|x| inside(x.view)).cloned().collect(),
            inputs,
            outputs,
        })
    }

    /// Save the selected module — or the selection, wrapped as a module — to a file.
    pub fn export_module(&mut self) {
        let file = match self.sel {
            Some(Sel::Module(mid)) if self.selected_items().len() <= 1 => self.module_file(mid),
            _ => {
                if self.selected_items().is_empty() {
                    self.status = "Select a module, or the parts that should become one, to export it".into();
                    return;
                }
                // wrap the selection for a moment, take the file, then put everything back
                self.make_module();
                let mid = self.model.modules.last().map(|m| m.id);
                let f = mid.and_then(|m| self.module_file(m));
                self.undo();
                self.redo.pop();
                f
            }
        };
        let Some(file) = file else { return };
        let Some(path) = rfd::FileDialog::new().add_filter("Dynamist module", &["flowmodule"]).set_file_name(format!("{}.flowmodule", file.name)).save_file() else { return };
        self.status = match serde_json::to_string_pretty(&file).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string())) {
            Ok(()) => format!("Exported module '{}' ({} parts, {} inputs, {} outputs)", file.name, file.vars.len(), file.inputs.len(), file.outputs.len()),
            Err(e) => format!("Couldn't export the module: {e}"),
        };
    }

    /// Pick a module file and show the dialog that connects its inputs.
    pub fn import_module(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Dynamist module", &["flowmodule", "json"]).pick_file() else { return };
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<ModuleFile>(&t).map_err(|e| e.to_string())) {
            Ok(f) if f.format == FORMAT => self.begin_import(f),
            Ok(_) => self.status = "That file isn't a Dynamist module".into(),
            Err(e) => self.status = format!("Couldn't read the module: {e}"),
        }
    }

    pub fn begin_import(&mut self, file: ModuleFile) {
        let choices = file
            .inputs
            .iter()
            .map(|p| InputChoice { target: self.model.by_name(&expr::normalize(&p.name)).map(|v| v.id), value: p.value.clone() })
            .collect();
        self.import_dialog = Some(ImportDialog { file, choices });
    }

    /// The dialog: for every input, use an existing variable or create a constant.
    pub fn import_window(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.import_dialog.take() else { return };
        let pal = self.pal;
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        let names: Vec<(u64, String, Kind)> = self.model.vars.iter().map(|v| (v.id, v.name.clone(), v.kind)).collect();
        egui::Window::new(format!("Import module · {}", dlg.file.name)).open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
            ui.set_width(520.0);
            if !dlg.file.description.is_empty() {
                ui.label(RichText::new(&dlg.file.description).color(pal.muted));
            }
            ui.label(RichText::new(format!("{} parts · {} inputs · {} outputs", dlg.file.vars.len(), dlg.file.inputs.len(), dlg.file.outputs.len())).small().color(pal.muted));
            ui.add_space(8.0);
            if dlg.file.inputs.is_empty() {
                ui.label("This module needs nothing from outside — it works on its own.");
            } else {
                ui.label(RichText::new("Connect its inputs").strong());
                ui.label(RichText::new("Each input can come from a variable already in your model, or from a new constant.").small().color(pal.muted));
                egui::Grid::new("inputs").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
                    for (p, c) in dlg.file.inputs.iter().zip(dlg.choices.iter_mut()) {
                        ui.label(RichText::new(&p.name).color(pal.ok));
                        let text = match c.target.and_then(|t| names.iter().find(|n| n.0 == t)) {
                            Some(n) => format!("use “{}”", n.1),
                            None => "new constant".into(),
                        };
                        egui::ComboBox::from_id_salt(("in", &p.name)).width(220.0).selected_text(text).show_ui(ui, |ui| {
                            ui.selectable_value(&mut c.target, None, "new constant");
                            ui.separator();
                            for (id, n, k) in &names {
                                ui.selectable_value(&mut c.target, Some(*id), RichText::new(n).color(pal.kind(*k)));
                            }
                        });
                        if c.target.is_none() {
                            ui.add(egui::TextEdit::singleline(&mut c.value).desired_width(90.0)).on_hover_text(format!("Starting value ({})", p.units));
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
            }
            if !dlg.file.outputs.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Gives back: {}", dlg.file.outputs.join(", "))).small().color(pal.aux));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Import").strong().color(egui::Color32::WHITE)).fill(pal.accent)).clicked() {
                    go = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if go {
            self.finish_import(&dlg.file, &dlg.choices);
        } else if open && !cancel {
            self.import_dialog = Some(dlg);
        }
    }

    /// Bring the module in: new ids, unique names, inputs wired up.
    pub fn finish_import(&mut self, f: &ModuleFile, choices: &[InputChoice]) {
        self.checkpoint();
        let parent = self.diagram_view();
        let mut idmap: HashMap<u64, u64> = HashMap::new();
        let mut nid = |m: &mut crate::model::Model, old: u64| *idmap.entry(old).or_insert_with(|| m.new_id());
        // unique names for the imported parts
        let mut renames: Vec<(String, String)> = Vec::new();
        let mut taken: Vec<String> = Vec::new();
        for v in &f.vars {
            let mut name = v.name.clone();
            let mut k = 2;
            while self.model.by_name(&expr::normalize(&name)).is_some() || taken.contains(&expr::normalize(&name)) {
                name = format!("{} {k}", v.name);
                k += 1;
            }
            taken.push(expr::normalize(&name));
            renames.push((v.name.clone(), name));
        }
        // inputs: an existing variable, or a new constant
        let mut in_targets: Vec<(u64, String)> = Vec::new();
        let mut created = Vec::new();
        for (p, c) in f.inputs.iter().zip(choices) {
            match c.target.and_then(|t| self.model.var(t)).map(|v| (v.id, v.name.clone())) {
                Some(t) => in_targets.push(t),
                None => {
                    let mut name = p.name.clone();
                    let mut k = 2;
                    while self.model.by_name(&expr::normalize(&name)).is_some() || taken.contains(&expr::normalize(&name)) {
                        name = format!("{} {k}", p.name);
                        k += 1;
                    }
                    taken.push(expr::normalize(&name));
                    let id = self.model.new_id();
                    let mut v = Var::new(id, &name, Kind::Constant, [0.0, 0.0]);
                    v.equation = if c.value.trim().is_empty() { "0".into() } else { c.value.trim().to_string() };
                    v.units = p.units.clone();
                    v.view = parent;
                    if let Ok(x) = v.equation.parse::<f64>() {
                        v.slider = Some(if x == 0.0 { [0.0, 1.0] } else { [x.min(0.0), x.abs() * 2.0] });
                    }
                    created.push(name.clone());
                    self.model.vars.push(v);
                    in_targets.push((id, name));
                }
            }
        }
        // every reference is renamed in two steps so names can swap without clashing
        let mut steps: Vec<(String, String)> = renames.clone();
        for (p, (_, tname)) in f.inputs.iter().zip(&in_targets) {
            steps.push((p.name.clone(), tname.clone()));
        }
        let rewrite = |eq: &str| -> String {
            let mut e = eq.to_string();
            for (i, (old, _)) in steps.iter().enumerate() {
                e = expr::rename_in(&e, &expr::normalize(old), &format!("zzimport{i}"));
            }
            for (i, (_, new)) in steps.iter().enumerate() {
                e = expr::rename_in(&e, &format!("zzimport{i}"), new);
            }
            e
        };
        let mid = self.model.new_id();
        let mut m = std::mem::take(&mut self.model);
        let root = nid(&mut m, f.root);
        for s in &f.sheets {
            let mut s = s.clone();
            s.id = nid(&mut m, s.id);
            s.module = if s.id == root { Some(mid) } else { s.module.map(|x| nid(&mut m, x)) };
            if s.id == root {
                s.name = f.name.clone();
            }
            let at = m.sheets.iter().position(|x| x.id == parent).map(|i| i + 1).unwrap_or(m.sheets.len());
            m.sheets.insert(at, s);
        }
        for (v, (_, new_name)) in f.vars.iter().zip(&renames) {
            let mut v = v.clone();
            v.id = nid(&mut m, v.id);
            v.view = nid(&mut m, v.view);
            v.from = v.from.map(|x| nid(&mut m, x));
            v.to = v.to.map(|x| nid(&mut m, x));
            v.name = new_name.clone();
            v.equation = rewrite(&v.equation);
            m.vars.push(v);
        }
        for l in &f.links {
            let mut l = l.clone();
            l.id = nid(&mut m, l.id);
            l.from = nid(&mut m, l.from);
            l.to = nid(&mut m, l.to);
            m.links.push(l);
        }
        for n in &f.notes {
            let mut n = n.clone();
            n.id = nid(&mut m, n.id);
            n.view = nid(&mut m, n.view);
            m.notes.push(n);
        }
        for x in &f.modules {
            let mut x = x.clone();
            x.id = nid(&mut m, x.id);
            x.view = nid(&mut m, x.view);
            x.inner = nid(&mut m, x.inner);
            m.modules.push(x);
        }
        for g in &f.groups {
            let mut g = g.clone();
            g.id = nid(&mut m, g.id);
            g.view = nid(&mut m, g.view);
            g.members = g.members.iter().map(|x| nid(&mut m, *x)).collect();
            m.groups.push(g);
        }
        // the module box goes to the right of what's on this sheet, its new constants to its left
        let right = m.vars.iter().filter(|v| v.view == parent).map(|v| v.pos[0]).fold(f32::NEG_INFINITY, f32::max);
        let mid_y = {
            let ys: Vec<f32> = m.vars.iter().filter(|v| v.view == parent).map(|v| v.pos[1]).collect();
            if ys.is_empty() { 0.0 } else { ys.iter().sum::<f32>() / ys.len() as f32 }
        };
        let pos = if right.is_finite() { [right + 320.0, mid_y] } else { [0.0, 0.0] };
        m.modules.push(Module { id: mid, name: f.name.clone(), view: parent, inner: root, pos, description: f.description.clone(), collapsed: false, layer: None });
        let mut row = 0.0;
        for (p, (tid, _)) in f.inputs.iter().zip(&in_targets) {
            if let Some(v) = m.var_mut(*tid) {
                if v.pos == [0.0, 0.0] && v.view == parent {
                    v.pos = [pos[0] - 260.0, pos[1] - 40.0 + row];
                    row += 56.0;
                }
            }
            for il in &p.links {
                if let Some(&to) = idmap.get(&il.to) {
                    if m.add_link(*tid, to) {
                        let l = m.links.last_mut().unwrap();
                        l.bend = il.bend;
                        l.polarity = il.polarity.clone();
                    }
                }
            }
        }
        m.ensure_sheets();
        self.model = m;
        self.select_one(Some(Sel::Module(mid)));
        self.fit_pending = true;
        self.changed();
        let renamed = renames.iter().filter(|(a, b)| a != b).count();
        let mut msg = format!("Imported module '{}' with {} parts", f.name, f.vars.len());
        if !created.is_empty() {
            msg.push_str(&format!("; new constants: {}", created.join(", ")));
        }
        if renamed > 0 {
            msg.push_str(&format!("; {renamed} part(s) renamed to keep names unique"));
        }
        self.status = msg;
        let _ = pos2(0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_round_trip_into_another_model() {
        let ctx = egui::Context::default();
        let mut a = App::new(&ctx, None);
        let parts = ["Fraction Infected", "Total Population", "Effect of Awareness"];
        a.select_one(None);
        for p in parts {
            let id = a.model.by_name(&expr::normalize(p)).unwrap().id;
            a.toggle_sel(Sel::Var(id));
        }
        a.make_module();
        let mid = a.model.modules[0].id;
        let file = a.module_file(mid).unwrap();
        assert_eq!(file.vars.len(), 3);
        let ins: Vec<&str> = file.inputs.iter().map(|p| p.name.as_str()).collect();
        assert!(ins.contains(&"Infected"), "{ins:?}");
        // survives a trip through JSON
        let file: ModuleFile = serde_json::from_str(&serde_json::to_string(&file).unwrap()).unwrap();

        // into a fresh copy of the example: names clash, inputs connect to what's there
        let mut b = App::new(&ctx, None);
        b.begin_import(file.clone());
        let dlg = b.import_dialog.take().unwrap();
        assert!(dlg.choices.iter().all(|c| c.target.is_some()), "inputs found by name");
        b.finish_import(&dlg.file, &dlg.choices);
        let fi2 = b.model.by_name("fraction infected 2").expect("renamed copy");
        assert_eq!(expr::normalize(&fi2.equation), "infected / total population 2");
        assert_eq!(b.model.modules.len(), 1);
        assert!(crate::sim::compile(&b.model).is_ok());

        // into a model that has none of the inputs: constants are created
        let mut c = App::new(&ctx, None);
        c.model = crate::model::Model { title: "empty".into(), ..Default::default() };
        c.model.ensure_sheets();
        c.sheet = c.model.first_diagram();
        c.begin_import(file);
        let dlg = c.import_dialog.take().unwrap();
        assert!(dlg.choices.iter().all(|c| c.target.is_none()));
        c.finish_import(&dlg.file, &dlg.choices);
        assert!(c.model.by_name("infected").is_some_and(|v| v.kind == Kind::Constant));
        assert!(crate::sim::compile(&c.model).is_ok(), "{:?}", crate::sim::compile(&c.model).err().map(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>()));
    }
}
