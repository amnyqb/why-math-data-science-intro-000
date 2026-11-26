//! Sensitivity runs (Monte Carlo): vary the key levers at random within a spread, run the model
//! many times, and show the band of outcomes for one variable — "how sure are we?".
//!
//! Each run draws every lever from `value × (1 ± spread)` (uniformly). A per-set lever (Max fill
//! for every tank) moves all its items together through its all-items scale. Without any levers
//! flagged, every numeric constant is varied. The draws use a fixed seed, so a run is repeatable.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use egui::{Color32, RichText};
use egui_plot::{Legend, Line, Plot, PlotPoints, Polygon};

use crate::app::App;
use crate::model::{Kind, Model};

pub use dynamist_core::sensitivity::{SensResult, run_sensitivity, varied};

pub struct SensJob {
    progress: Arc<AtomicUsize>,
    total: usize,
    out: Arc<Mutex<Option<Result<SensResult, String>>>>,
}

pub struct SensState {
    pub var: String,
    pub runs: usize,
    pub spread_pct: f64,
    pub job: Option<SensJob>,
    pub result: Option<SensResult>,
    pub error: Option<String>,
}

impl Default for SensState {
    fn default() -> Self {
        SensState { var: String::new(), runs: 100, spread_pct: 20.0, job: None, result: None, error: None }
    }
}

impl App {
    pub fn sensitivity_window(&mut self, ctx: &egui::Context) {
        if !self.sens_open {
            return;
        }
        let pal = self.pal;
        let mut open = true;
        let mut start = false;
        // collect a finished job
        let finished = self.sens.job.as_ref().map(|job| job.out.lock().unwrap().take());
        match finished {
            Some(Some(r)) => {
                match r {
                    Ok(x) => {
                        self.sens.result = Some(x);
                        self.sens.error = None;
                    }
                    Err(e) => self.sens.error = Some(e),
                }
                self.sens.job = None;
            }
            Some(None) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
            None => {}
        }
        if self.sens.var.is_empty() {
            self.sens.var = self.model.vars.iter().find(|v| v.kind == Kind::Stock).map(|v| v.name.clone()).unwrap_or_default();
        }
        let names: Vec<String> = self.model.vars.iter().filter(|v| !(v.kind == Kind::Lookup && v.equation.trim().is_empty())).map(|v| v.name.clone()).collect();
        let n_varied = varied(&self.model).len();
        let lever_names: Vec<String> = varied(&self.model).iter().filter_map(|(id, _)| self.model.var(*id).map(|v| v.name.clone())).collect();
        egui::Window::new("Sensitivity runs").open(&mut open).default_width(720.0).default_height(560.0).show(ctx, |ui| {
            ui.label(RichText::new("How sure are we? Vary the key levers at random, run the model many times, and see the band of outcomes.").small().color(pal.muted));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Watch");
                egui::ComboBox::from_id_salt("sens-var").width(210.0).selected_text(self.sens.var.clone()).show_ui(ui, |ui| {
                    for n in &names {
                        ui.selectable_value(&mut self.sens.var, n.clone(), n);
                    }
                });
                ui.label("runs");
                ui.add(egui::DragValue::new(&mut self.sens.runs).range(10..=1000));
                ui.label("each lever ±");
                ui.add(egui::DragValue::new(&mut self.sens.spread_pct).range(1.0..=90.0).suffix("%"));
                let busy = self.sens.job.is_some();
                start = ui.add_enabled(!busy, egui::Button::new(RichText::new("Run").color(Color32::WHITE)).fill(pal.accent)).clicked();
            });
            ui.label(RichText::new(format!("Varied ({n_varied}): {}", if lever_names.is_empty() { "nothing — mark key levers".to_string() } else { lever_names.join(", ") })).small().color(pal.muted));
            if let Some(job) = &self.sens.job {
                let done = job.progress.load(Ordering::SeqCst);
                ui.add(egui::ProgressBar::new(done as f32 / job.total.max(1) as f32).text(format!("{done} of {} runs", job.total)));
            }
            if let Some(e) = &self.sens.error {
                ui.label(RichText::new(format!("⚠ {e}")).color(pal.error));
            }
            if let Some(r) = &self.sens.result {
                let last = r.times.len().saturating_sub(1);
                ui.label(RichText::new(format!("{} at the end of the run: middle {} · likely range {} to {} · extremes {} to {}  ({} runs)", r.var, crate::theme::fmt_num(r.p50[last]), crate::theme::fmt_num(r.p5[last]), crate::theme::fmt_num(r.p95[last]), crate::theme::fmt_num(r.min[last]), crate::theme::fmt_num(r.max[last]), r.runs)).strong());
                // a band is drawn as thin strips, each a simple four-cornered shape (a long
                // single outline would be filled down to the axis)
                let strips = |lo: &Vec<f64>, hi: &Vec<f64>| -> Vec<Vec<[f64; 2]>> {
                    (0..r.times.len().saturating_sub(1))
                        .map(|i| vec![[r.times[i], lo[i]], [r.times[i + 1], lo[i + 1]], [r.times[i + 1], hi[i + 1]], [r.times[i], hi[i]]])
                        .collect()
                };
                Plot::new("sens-plot").height(340.0).legend(Legend::default()).show(ui, |pu| {
                    for (k, q) in strips(&r.p5, &r.p95).into_iter().enumerate() {
                        let name = if k == 0 { "5–95% of runs" } else { "" };
                        pu.polygon(Polygon::new(name, PlotPoints::from(q)).fill_color(pal.accent.gamma_multiply(0.16)).stroke(egui::Stroke::NONE));
                    }
                    for (k, q) in strips(&r.p25, &r.p75).into_iter().enumerate() {
                        let name = if k == 0 { "25–75% of runs" } else { "" };
                        pu.polygon(Polygon::new(name, PlotPoints::from(q)).fill_color(pal.accent.gamma_multiply(0.30)).stroke(egui::Stroke::NONE));
                    }
                    pu.line(Line::new("Middle (median)", PlotPoints::from(r.times.iter().zip(&r.p50).map(|(t, y)| [*t, *y]).collect::<Vec<_>>())).color(pal.accent).width(2.2));
                    pu.line(Line::new("As set now", PlotPoints::from(r.times.iter().zip(&r.base).map(|(t, y)| [*t, *y]).collect::<Vec<_>>())).color(pal.text).width(1.4).style(egui_plot::LineStyle::dashed_dense()));
                });
            }
        });
        if start {
            let total = self.sens.runs;
            let (model, var, spread) = (self.model.clone(), self.sens.var.clone(), self.sens.spread_pct / 100.0);
            let progress = Arc::new(AtomicUsize::new(0));
            let out = Arc::new(Mutex::new(None));
            let (p2, o2) = (progress.clone(), out.clone());
            let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).min(8);
            std::thread::spawn(move || {
                let r = run_sensitivity(&model, &var, total, spread, 0x9E37_79B9_7F4A_7C15, workers, &p2);
                *o2.lock().unwrap() = Some(r);
            });
            self.sens.job = Some(SensJob { progress, total, out });
            self.sens.error = None;
        }
        if !open {
            self.sens_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Method, Settings, Var};

    fn growth() -> Model {
        let mut m = Model::default();
        m.settings = Settings { initial_time: 0.0, final_time: 10.0, time_step: 0.5, time_units: "Year".into(), method: Method::Euler, ..Default::default() };
        let mut y = Var::new(1, "Pop", Kind::Stock, [0.0, 0.0]);
        y.equation = "100".into();
        let mut g = Var::new(2, "Growth", Kind::Flow, [0.0, 0.0]);
        g.equation = "Pop * Rate".into();
        g.to = Some(1);
        let mut r = Var::new(3, "Rate", Kind::Constant, [0.0, 0.0]);
        r.equation = "0.05".into();
        r.lever = true;
        m.vars.extend([y, g, r]);
        m
    }

    #[test]
    fn the_band_surrounds_the_baseline_and_is_repeatable() {
        let m = growth();
        let p = AtomicUsize::new(0);
        let a = run_sensitivity(&m, "Pop", 60, 0.3, 7, 3, &p).unwrap();
        let b = run_sensitivity(&m, "Pop", 60, 0.3, 7, 1, &AtomicUsize::new(0)).unwrap();
        assert_eq!(p.load(Ordering::SeqCst), 60);
        let last = a.times.len() - 1;
        assert!(a.p5[last] < a.base[last] && a.base[last] < a.p95[last], "{} {} {}", a.p5[last], a.base[last], a.p95[last]);
        assert!(a.min[last] <= a.p5[last] && a.p95[last] <= a.max[last]);
        assert!(a.p25[last] <= a.p50[last] && a.p50[last] <= a.p75[last]);
        // the start is the same in every run
        assert_eq!(a.p5[0], 100.0);
        assert_eq!(a.p95[0], 100.0);
        // the same seed gives the same answer, whatever the number of threads
        assert_eq!(a.p50, b.p50);
        assert_eq!(a.varied, vec!["Rate".to_string()]);
    }

    /// How long do 40 runs of the fuel model take? (`cargo test fuel_sensitivity -- --ignored --nocapture`)
    #[test]
    #[ignore]
    fn fuel_sensitivity_timing() {
        let dir = std::path::Path::new(crate::spreadsheet::WIKI_DIR).join("inbox").join("fuel-supply");
        let (m, _) = crate::spreadsheet::model_from_book(&crate::spreadsheet::read_book(&dir).unwrap()).unwrap();
        let t = std::time::Instant::now();
        let r = run_sensitivity(&m, "Network shortage", 40, 0.1, 3, 4, &AtomicUsize::new(0)).unwrap();
        let last = r.times.len() - 1;
        eprintln!("40 runs in {:?}; shortage {:.1} (5%) .. {:.1} (95%), as set {:.1}; varied {:?}", t.elapsed(), r.p5[last], r.p95[last], r.base[last], r.varied);
    }

    #[test]
    fn a_model_with_nothing_to_vary_says_so() {
        let mut m = growth();
        m.vars.iter_mut().for_each(|v| v.lever = false);
        m.vars.retain(|v| v.kind != Kind::Constant);
        m.vars.iter_mut().find(|v| v.name == "Growth").unwrap().equation = "Pop * 0.05".into();
        assert!(run_sensitivity(&m, "Pop", 10, 0.2, 1, 1, &AtomicUsize::new(0)).is_err());
    }
}
