//! Checks that live inside the model: limits you state about a variable ("shortage at the end
//! stays between 0 and 2000 MB") and a conservation check that every stock changed by exactly
//! what flowed in and out of it. They are tested against the current run, and a failure shows in
//! the Problems tab, so a model can say what "behaves sensibly" means and keep saying it after
//! every edit.

use serde::{Deserialize, Serialize};

use crate::expr;
use crate::model::{Kind, Method, Model, Rollup};
use crate::sim::Run;

/// When a limit applies.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckWhen {
    /// At every moment of the run.
    #[default]
    Always,
    /// At the last moment only.
    End,
}

/// "Variable stays between Min and Max" (either end may be left out).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Check {
    pub name: String,
    pub var: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub when: CheckWhen,
}

/// One thing that did not hold. `var` is the variable to jump to, when there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub var: Option<u64>,
    pub message: String,
}

fn num(x: f64) -> String {
    crate::theme::fmt_num(x)
}

/// The limits that were broken in this run.
pub fn limits(m: &Model, run: &Run) -> Vec<Failure> {
    let mut out = Vec::new();
    for c in &m.checks {
        let label = if c.name.trim().is_empty() { c.var.clone() } else { c.name.clone() };
        let Some(v) = m.by_name(&expr::normalize(&c.var)) else {
            out.push(Failure { var: None, message: format!("Check “{label}”: there is no variable called “{}”", c.var) });
            continue;
        };
        let Some(series) = run.get(&c.var) else {
            out.push(Failure { var: Some(v.id), message: format!("Check “{label}”: “{}” has no results yet", c.var) });
            continue;
        };
        let at: Vec<(usize, f64)> = match c.when {
            CheckWhen::Always => series.iter().cloned().enumerate().collect(),
            CheckWhen::End => series.last().map(|x| vec![(series.len() - 1, *x)]).unwrap_or_default(),
        };
        let bad = at.iter().find(|(_, x)| !x.is_finite() || c.min.is_some_and(|lo| *x < lo) || c.max.is_some_and(|hi| *x > hi));
        if let Some(&(i, x)) = bad {
            let range = match (c.min, c.max) {
                (Some(lo), Some(hi)) => format!("between {} and {}", num(lo), num(hi)),
                (Some(lo), None) => format!("at least {}", num(lo)),
                (None, Some(hi)) => format!("at most {}", num(hi)),
                (None, None) => "finite".into(),
            };
            let when = match c.when {
                CheckWhen::End => "at the end".to_string(),
                CheckWhen::Always => format!("at {} {}", run.time_units, num(run.times.get(i).copied().unwrap_or(0.0))),
            };
            out.push(Failure { var: Some(v.id), message: format!("Check “{label}”: {} should be {range}, but is {} {when}", c.var, num(x)) });
        }
    }
    out
}

/// Every stock should change by exactly what flowed in minus what flowed out. Stocks that are
/// held at zero ("can't go negative"), averaged, or filled by flows of a different set are left
/// out, since the engine or the roll-up deliberately breaks the balance there.
pub fn conservation(m: &Model, run: &Run) -> Vec<Failure> {
    let mut out = Vec::new();
    // Euler steps are exact; the higher-order methods use mid-step values the run does not keep
    let tol = if m.settings.method == Method::Euler { 1e-6 } else { 2e-2 };
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        if s.non_negative || s.rollup == Rollup::Average {
            continue;
        }
        let flows: Vec<_> = m.vars.iter().filter(|f| f.kind == Kind::Flow && (f.from == Some(s.id) || f.to == Some(s.id))).collect();
        if flows.is_empty() || flows.iter().any(|f| f.set != s.set || f.rollup == Rollup::Average) {
            continue;
        }
        let Some(level) = run.get(&s.name) else { continue };
        let series: Vec<(&crate::model::Var, &Vec<f64>)> = flows.iter().filter_map(|f| run.get(&f.name).map(|x| (*f, x))).collect();
        if series.len() != flows.len() || level.len() != run.times.len() || level.len() < 2 {
            continue;
        }
        let net = |i: usize| -> f64 { series.iter().map(|(f, x)| if f.to == Some(s.id) && f.from != Some(s.id) { x[i] } else if f.from == Some(s.id) && f.to != Some(s.id) { -x[i] } else { 0.0 }).sum() };
        let mut moved = 0.0;
        let mut scale: f64 = level.iter().fold(0.0, |a, x| a.max(x.abs()));
        for i in 0..level.len() - 1 {
            let dt = run.times[i + 1] - run.times[i];
            let step = if m.settings.method == Method::Euler { net(i) * dt } else { 0.5 * (net(i) + net(i + 1)) * dt };
            moved += step;
            scale = scale.max(step.abs());
        }
        let change = level[level.len() - 1] - level[0];
        let gap = (change - moved).abs();
        if gap > tol * scale.max(1e-9) && gap.is_finite() {
            out.push(Failure {
                var: Some(s.id),
                message: format!("Stock “{}” changed by {} but its flows add up to {}: something other than its flows moves it", s.name, num(change), num(moved)),
            });
        }
    }
    out
}

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
                list.push(Check { name: String::new(), var, min: Some(0.0), max: None, when: CheckWhen::Always });
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

/// Everything that failed, limits first.
pub fn all(m: &Model, run: &Run) -> Vec<Failure> {
    let mut v = limits(m, run);
    v.extend(conservation(m, run));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Var;

    fn tank() -> Model {
        let mut m = Model::default();
        m.settings.final_time = 10.0;
        m.settings.time_step = 0.5;
        let mut s = Var::new(1, "Tank", Kind::Stock, [0.0, 0.0]);
        s.equation = "100".into();
        let mut f = Var::new(2, "Burn", Kind::Flow, [0.0, 0.0]);
        f.equation = "8".into();
        f.from = Some(1);
        let mut g = Var::new(3, "Fill", Kind::Flow, [0.0, 0.0]);
        g.equation = "TIME".into();
        g.to = Some(1);
        m.vars = vec![s, f, g];
        m.next_id = 10;
        m
    }

    fn run(m: &Model) -> Run {
        let c = crate::sim::compile(m).unwrap();
        crate::sim::simulate(&c, m, "x").unwrap()
    }

    #[test]
    fn a_plain_stock_conserves_with_every_method() {
        for method in [Method::Euler, Method::RungeKutta2, Method::RungeKutta4] {
            let mut m = tank();
            m.settings.method = method;
            let r = run(&m);
            assert!(conservation(&m, &r).is_empty(), "{method:?}: {:?}", conservation(&m, &r));
        }
    }

    #[test]
    fn a_stock_moved_by_something_else_is_caught() {
        let m = tank();
        let mut r = run(&m);
        let level = r.series.get_mut("tank").unwrap();
        let n = level.len();
        level[n - 1] += 25.0; // as if something outside its flows had added to it
        let f = conservation(&m, &r);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("Tank"));
    }

    #[test]
    fn limits_say_what_broke_and_when() {
        let mut m = tank();
        m.checks = vec![
            Check { name: "Stays above 80".into(), var: "Tank".into(), min: Some(80.0), max: None, when: CheckWhen::Always },
            Check { name: "Ends high".into(), var: "Tank".into(), min: Some(500.0), max: None, when: CheckWhen::End },
            Check { name: "Fine".into(), var: "Tank".into(), min: Some(-1e6), max: Some(1e6), when: CheckWhen::Always },
            Check { name: "Typo".into(), var: "Tnak".into(), min: Some(0.0), max: None, when: CheckWhen::End },
        ];
        let f = limits(&m, &run(&m));
        assert_eq!(f.len(), 3, "{f:?}");
        assert!(f[0].message.contains("at least 80") && f[0].message.contains(" at "), "{}", f[0].message);
        assert!(f[1].message.contains("at the end"), "{}", f[1].message);
        assert!(f[2].message.contains("no variable"), "{}", f[2].message);
    }

    #[test]
    fn the_fuel_model_conserves_every_stock() {
        let dir = std::path::Path::new(crate::spreadsheet::WIKI_DIR).join("inbox").join("fuel-supply");
        if !dir.exists() {
            return;
        }
        let (m, _) = crate::spreadsheet::model_from_book(&crate::spreadsheet::read_book(&dir).unwrap()).unwrap();
        let f = all(&m, &run(&m));
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn the_fuel_model_states_its_own_limits_and_they_hold() {
        let dir = std::path::Path::new(crate::spreadsheet::WIKI_DIR).join("inbox").join("fuel-supply");
        if !dir.join("Checks.csv").exists() {
            return;
        }
        let (m, notes) = crate::spreadsheet::model_from_book(&crate::spreadsheet::read_book(&dir).unwrap()).unwrap();
        assert_eq!(m.checks.len(), 4, "{notes:?}");
        assert_eq!(m.checks[3].when, CheckWhen::End);
        let f = limits(&m, &run(&m));
        assert!(f.is_empty(), "{f:?}");
        // and a limit that is too tight is reported
        let mut tight = m.clone();
        tight.checks[3].max = Some(100.0);
        assert_eq!(limits(&tight, &run(&tight)).len(), 1);
    }

    #[test]
    fn checks_survive_a_trip_through_the_spreadsheet() {
        let mut m = tank();
        m.checks = vec![
            Check { name: "Stays above 80".into(), var: "Tank".into(), min: Some(80.0), max: None, when: CheckWhen::Always },
            Check { name: String::new(), var: "Tank".into(), min: None, max: Some(1000.5), when: CheckWhen::End },
        ];
        let book = crate::spreadsheet::book_from_model(&m);
        let (back, notes) = crate::spreadsheet::model_from_book(&book).unwrap();
        assert_eq!(back.checks, m.checks, "{notes:?}");
    }
}
