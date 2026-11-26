//! A function from Python, turned into a lookup curve.
//!
//! Anything Python's standard library can compute — `math.erf(x)`, `statistics.NormalDist().cdf(x)`,
//! `math.comb(10, 3)` with a loop… — is sampled once, here, over a range, and stored as an ordinary
//! lookup (a curve of points) that equations use like any other: `Normal curve(Z)`.
//! Python runs only when you press the button in this window, on your own computer, on the
//! expression you typed. Model files never contain code and never run any, so opening or importing
//! a model cannot run Python.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use egui::RichText;

use crate::app::App;
use crate::model::{Kind, Var};

pub struct PyForm {
    pub name: String,
    pub expr: String,
    pub x0: f64,
    pub x1: f64,
    pub n: usize,
    pub input: String,
    pub status: String,
    pub preview: Vec<[f64; 2]>,
}

impl Default for PyForm {
    fn default() -> Self {
        PyForm {
            name: "Normal curve".into(),
            expr: "statistics.NormalDist(0, 1).cdf(x)".into(),
            x0: -4.0,
            x1: 4.0,
            n: 41,
            input: String::new(),
            status: String::new(),
            preview: Vec::new(),
        }
    }
}

const SCRIPT: &str = r#"
import sys, json, math, statistics
expr, x0, x1, n = sys.argv[1], float(sys.argv[2]), float(sys.argv[3]), int(sys.argv[4])
safe = {k: __builtins__.__dict__[k] if hasattr(__builtins__, '__dict__') else __builtins__[k]
        for k in ('abs', 'min', 'max', 'sum', 'round', 'pow', 'float', 'int', 'range', 'len', 'sorted', 'list', 'map', 'zip', 'enumerate', 'any', 'all', 'divmod')}
env = {'__builtins__': safe, 'math': math, 'statistics': statistics}
out = []
for i in range(n):
    x = x0 + (x1 - x0) * i / (n - 1)
    y = float(eval(expr, env, {'x': x}))
    if y != y or y in (float('inf'), float('-inf')):
        raise ValueError('the expression gave %r at x = %g' % (y, x))
    out.append([x, y])
print(json.dumps(out))
"#;

/// Sample `expr(x)` at `n` points from `x0` to `x1` with Python 3.
pub fn run_python(expr: &str, x0: f64, x1: f64, n: usize) -> Result<Vec<[f64; 2]>, String> {
    if expr.trim().is_empty() {
        return Err("Type an expression in x, for example math.sin(x)".into());
    }
    if !(2..=1000).contains(&n) || !(x1 > x0) {
        return Err("Use 2 to 1000 points and a range where 'to' is above 'from'".into());
    }
    let mut child = Command::new("python3")
        .args(["-I", "-c", SCRIPT, expr, &x0.to_string(), &x1.to_string(), &n.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Python 3 wasn't found ({e}). Install it, or write the function in the equation language (Edit → My functions…)."))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > Duration::from_secs(15) => {
                let _ = child.kill();
                return Err("Python took longer than 15 seconds; try fewer points".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("Python reported an error");
        return Err(last.trim().to_string());
    }
    serde_json::from_slice::<Vec<[f64; 2]>>(&out.stdout).map_err(|e| format!("Unexpected answer from Python: {e}"))
}

impl App {
    pub fn python_window(&mut self, ctx: &egui::Context) {
        if !self.py_open {
            return;
        }
        let pal = self.pal;
        let mut open = true;
        let mut make = false;
        let mut try_it = false;
        let mut form = std::mem::take(&mut self.py_form);
        egui::Window::new("Function from Python").open(&mut open).default_width(540.0).show(ctx, |ui| {
            ui.label(RichText::new("Sample anything Python's standard library can compute into a curve (a lookup) that equations use like any other. Python runs only when you press a button here, on this computer, on what you typed — model files never run code.").small().color(pal.muted));
            ui.add_space(6.0);
            egui::Grid::new("py-grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                ui.label("Name of the curve");
                ui.add(egui::TextEdit::singleline(&mut form.name).desired_width(f32::INFINITY));
                ui.end_row();
                ui.label("Python expression in x");
                ui.add(egui::TextEdit::singleline(&mut form.expr).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace).hint_text("math.erf(x)"));
                ui.end_row();
                ui.label("x from … to …");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut form.x0).speed(0.1));
                    ui.label("to");
                    ui.add(egui::DragValue::new(&mut form.x1).speed(0.1));
                    ui.label("in");
                    ui.add(egui::DragValue::new(&mut form.n).range(2..=1000));
                    ui.label("points");
                });
                ui.end_row();
                ui.label("Input");
                ui.add(egui::TextEdit::singleline(&mut form.input).desired_width(f32::INFINITY).hint_text("empty = use it as a function: Name(x)"));
                ui.end_row();
            });
            ui.label(RichText::new("Available: math, statistics, abs, min, max, sum, round, pow, range, sorted…  e.g. math.erf(x) · statistics.NormalDist(0, 1).cdf(x) · math.gamma(x + 1) · sum(1 / k for k in range(1, 6)) * x").small().color(pal.muted));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                try_it = ui.button("Try it").clicked();
                make = ui.add(egui::Button::new(RichText::new("Create the curve").color(egui::Color32::WHITE)).fill(pal.accent)).clicked();
            });
            if !form.status.is_empty() {
                let bad = form.status.starts_with("⚠");
                ui.label(RichText::new(&form.status).color(if bad { pal.error } else { pal.ok }));
            }
            if !form.preview.is_empty() {
                let show: Vec<String> = form.preview.iter().step_by((form.preview.len() / 6).max(1)).map(|p| format!("({:.3}, {:.4})", p[0], p[1])).collect();
                ui.label(RichText::new(show.join("  ")).monospace().small().color(pal.muted));
            }
        });
        if try_it || make {
            match run_python(&form.expr, form.x0, form.x1, form.n) {
                Ok(pts) => {
                    form.preview = pts.clone();
                    form.status = format!("✓ {} points, y from {:.4} to {:.4}", pts.len(), pts.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min), pts.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max));
                    if make && !form.name.trim().is_empty() {
                        self.checkpoint();
                        let comment = format!("From Python: {} for x from {} to {} ({} points)", form.expr.trim(), form.x0, form.x1, pts.len());
                        let key = crate::expr::normalize(&form.name);
                        match self.model.vars.iter_mut().find(|v| crate::expr::normalize(&v.name) == key && v.kind == Kind::Lookup) {
                            Some(v) => {
                                v.table = pts.clone();
                                v.comment = comment;
                                v.equation = form.input.trim().to_string();
                            }
                            None => {
                                let id = self.model.new_id();
                                let view = self.diagram_view();
                                let mut v = Var::new(id, form.name.trim(), Kind::Lookup, [160.0, 160.0]);
                                v.table = pts.clone();
                                v.comment = comment;
                                v.equation = form.input.trim().to_string();
                                v.view = view;
                                self.model.vars.push(v);
                            }
                        }
                        self.changed();
                        form.status = format!("✓ Created “{}” with {} points. {}", form.name.trim(), pts.len(), if form.input.trim().is_empty() { format!("Use it in an equation as {}(value).", form.name.trim()) } else { "It is a variable of that input.".into() });
                    }
                }
                Err(e) => {
                    form.preview.clear();
                    form.status = format!("⚠ {e}");
                }
            }
        }
        self.py_form = form;
        if !open {
            self.py_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_samples_a_function_into_points() {
        let Ok(pts) = run_python("math.erf(x)", 0.0, 1.0, 3) else {
            return; // no Python on this machine: nothing to check
        };
        assert_eq!(pts.len(), 3);
        assert!((pts[1][1] - 0.520_499_877_8).abs() < 1e-9);
        assert!((pts[2][1] - 0.842_700_792_9).abs() < 1e-9);
        // mistakes come back as plain messages, and the sandbox has no open() or import
        assert!(run_python("1 / 0", 0.0, 1.0, 3).is_err());
        assert!(run_python("open('/etc/passwd')", 0.0, 1.0, 3).is_err());
        assert!(run_python("__import__('os').getcwd()", 0.0, 1.0, 3).is_err());
    }

    #[test]
    fn the_built_in_erf_agrees_with_python() {
        let Ok(pts) = run_python("math.erf(x)", -2.0, 2.0, 9) else { return };
        for p in pts {
            // the app's own ERF is good to about 1.5e-7
            let mut m = crate::model::Model::default();
            m.settings = crate::model::Settings { initial_time: 0.0, final_time: 1.0, time_step: 1.0, time_units: "s".into(), method: crate::model::Method::Euler, ..Default::default() };
            let mut v = Var::new(1, "e", Kind::Aux, [0.0, 0.0]);
            v.equation = format!("ERF({})", p[0]);
            m.vars.push(v);
            let c = crate::sim::compile(&m).ok().unwrap();
            let got = crate::sim::simulate(&c, &m, "t").unwrap().get("e").unwrap()[0];
            assert!((got - p[1]).abs() < 2e-7, "ERF({}) = {got}, Python {}", p[0], p[1]);
        }
    }
}
