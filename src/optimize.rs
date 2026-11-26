//! Optimization made simple: pick a goal, tick the levers that may change, add limits
//! that must hold, press Optimize. The search runs in the background (random sampling
//! followed by Nelder–Mead), so the window stays responsive and shows progress live.

use std::sync::atomic::Ordering;

use egui::{Color32, CornerRadius, RichText, Stroke, vec2};
use egui_plot::{Line, LineStyle, Plot, PlotPoints};

use crate::app::{App, SavedRun};
use crate::expr;
use crate::model::{Kind, Lever, Limit, Measure, Model, OptSpec};
use crate::sim::{self, Run};
use crate::theme;

pub use dynamist_core::optimize::*;

#[derive(Default)]
pub struct OptState {
    pub job: Option<Job>,
    /// The latest finished (or running) result, by sheet.
    pub last: Option<(u64, Progress)>,
}

fn card<R>(
    ui: &mut egui::Ui,
    pal: theme::Palette,
    title: &str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(pal.panel)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(15.0).strong());
            ui.add_space(6.0);
            add(ui)
        })
        .inner
}

impl App {
    /// Called every frame: pick up progress from a running search.
    pub fn poll_optimizer(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.opt.job else { return };
        let snap = job.shared.lock().unwrap().clone();
        let sheet = job.sheet;
        let done = snap.done;
        self.opt.last = Some((sheet, snap));
        if done {
            self.opt.job = None;
            if let Some((_, p)) = &self.opt.last {
                self.status = match &p.error {
                    Some(e) => format!("Optimization stopped: {e}"),
                    None => format!("Optimization finished after {} tries", p.evals),
                };
            }
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    fn opt_spec(&self) -> OptSpec {
        self.model
            .sheets
            .iter()
            .find(|s| s.id == self.sheet)
            .map(|s| s.opt.clone())
            .unwrap_or_default()
    }

    pub fn optimize_sheet(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let mut spec = self.opt_spec();
        let before = serde_json::to_string(&spec).unwrap_or_default();
        let running = self.opt.job.as_ref().is_some_and(|j| j.sheet == self.sheet);
        let outputs: Vec<(String, Kind)> = self
            .model
            .vars
            .iter()
            .filter(|v| !(v.kind == Kind::Lookup && v.equation.trim().is_empty()))
            .map(|v| (v.name.clone(), v.kind))
            .collect();
        let constants: Vec<(String, f64, Option<[f64; 2]>)> = self
            .model
            .vars
            .iter()
            .filter(|v| v.kind == Kind::Constant)
            .filter_map(|v| {
                v.equation
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .map(|x| (v.name.clone(), x, v.slider))
            })
            .collect();
        // every constant appears as a possible lever
        for (n, x, sl) in &constants {
            if !spec.levers.iter().any(|l| &l.name == n) {
                let [a, b] = sl.unwrap_or([x * 0.5, if *x == 0.0 { 1.0 } else { x * 1.5 }]);
                spec.levers.push(Lever {
                    name: n.clone(),
                    on: false,
                    min: a.min(b),
                    max: a.max(b),
                });
            }
        }
        spec.levers
            .retain(|l| constants.iter().any(|c| c.0 == l.name));
        if spec.goal.is_empty() {
            spec.goal = outputs
                .iter()
                .find(|o| o.1 == Kind::Stock)
                .or(outputs.first())
                .map(|o| o.0.clone())
                .unwrap_or_default();
        }

        egui::Frame::new().fill(pal.canvas).inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Optimize").size(20.0).strong());
                ui.label(RichText::new("Say what you want, what may change and what must stay true — then press Optimize.").color(pal.muted));
            });
            ui.add_space(10.0);
            let w = ui.available_width();
            let left_w = (w * 0.52).max(420.0);
            ui.horizontal_top(|ui| {
                // ------------------------------------------------ setup
                ui.vertical(|ui| {
                    ui.set_width(left_w);
                    egui::ScrollArea::vertical().id_salt("opt-setup").auto_shrink([false, false]).show(ui, |ui| {
                        card(ui, pal, "①  What do you want?", |ui| {
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut spec.maximize, false, "Make it as small as possible");
                                ui.selectable_value(&mut spec.maximize, true, "Make it as large as possible");
                            });
                            ui.horizontal(|ui| {
                                ui.label("the");
                                egui::ComboBox::from_id_salt("goal-measure").selected_text(spec.measure.label()).show_ui(ui, |ui| {
                                    for m in [Measure::Peak, Measure::Final, Measure::Lowest, Measure::Total, Measure::Average] {
                                        ui.selectable_value(&mut spec.measure, m, m.label());
                                    }
                                });
                                ui.label("of");
                                egui::ComboBox::from_id_salt("goal-var").width(200.0).selected_text(&spec.goal).show_ui(ui, |ui| {
                                    for (n, k) in &outputs {
                                        ui.selectable_value(&mut spec.goal, n.clone(), RichText::new(n).color(pal.kind(*k)));
                                    }
                                });
                            });
                            if let Some(v) = self.run.as_ref().and_then(|r| measure(r, &spec.goal, spec.measure, false)) {
                                ui.label(RichText::new(format!("Right now: {}", theme::fmt_num(v))).small().color(pal.muted));
                            }
                        });
                        ui.add_space(10.0);
                        card(ui, pal, "②  What may change?", |ui| {
                            ui.label(RichText::new("Tick the levers the search may move, and how far.").small().color(pal.muted));
                            egui::Grid::new("levers").num_columns(5).spacing([10.0, 6.0]).show(ui, |ui| {
                                ui.label("");
                                ui.label(RichText::new("lever").small().color(pal.muted));
                                ui.label(RichText::new("now").small().color(pal.muted));
                                ui.label(RichText::new("lowest").small().color(pal.muted));
                                ui.label(RichText::new("highest").small().color(pal.muted));
                                ui.end_row();
                                for l in spec.levers.iter_mut() {
                                    ui.checkbox(&mut l.on, "");
                                    ui.label(RichText::new(&l.name).color(if l.on { pal.constant } else { pal.muted }));
                                    let now = constants.iter().find(|c| c.0 == l.name).map(|c| c.1).unwrap_or(0.0);
                                    ui.label(RichText::new(theme::fmt_num(now)).monospace());
                                    let sp = ((l.max - l.min).abs() / 100.0).max(1e-4);
                                    ui.add_enabled(l.on, egui::DragValue::new(&mut l.min).speed(sp));
                                    ui.add_enabled(l.on, egui::DragValue::new(&mut l.max).speed(sp));
                                    ui.end_row();
                                }
                            });
                            if spec.levers.is_empty() {
                                ui.label(RichText::new("The model has no constants yet — add some on a diagram sheet.").color(pal.muted));
                            }
                        });
                        ui.add_space(10.0);
                        card(ui, pal, "③  What must stay true?", |ui| {
                            ui.label(RichText::new("Optional limits. Results that break them are rejected.").small().color(pal.muted));
                            let mut remove = None;
                            for (i, l) in spec.limits.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt(("lim-var", i)).width(150.0).selected_text(&l.var).show_ui(ui, |ui| {
                                        for (n, k) in &outputs {
                                            ui.selectable_value(&mut l.var, n.clone(), RichText::new(n).color(pal.kind(*k)));
                                        }
                                    });
                                    egui::ComboBox::from_id_salt(("lim-m", i)).width(120.0).selected_text(l.measure.label()).show_ui(ui, |ui| {
                                        for m in [Measure::Final, Measure::Peak, Measure::Lowest, Measure::Total, Measure::Average, Measure::Always] {
                                            ui.selectable_value(&mut l.measure, m, m.label());
                                        }
                                    });
                                    ui.selectable_value(&mut l.at_most, true, "≤");
                                    ui.selectable_value(&mut l.at_most, false, "≥");
                                    let sp = (l.value.abs() / 100.0).max(0.01);
                                    ui.add(egui::DragValue::new(&mut l.value).speed(sp));
                                    if let Some(v) = self.run.as_ref().and_then(|r| measure(r, &l.var, l.measure, l.at_most)) {
                                        let ok = violation(l, v) <= 1e-9;
                                        ui.label(RichText::new(format!("{} now {}", if ok { "✓" } else { "✗" }, theme::fmt_num(v))).small().color(if ok { pal.ok } else { pal.error }));
                                    }
                                    if ui.small_button("×").clicked() {
                                        remove = Some(i);
                                    }
                                });
                            }
                            if let Some(i) = remove {
                                spec.limits.remove(i);
                            }
                            if ui.button("+ Add a limit").clicked() {
                                let var = outputs.first().map(|o| o.0.clone()).unwrap_or_default();
                                let value = self.run.as_ref().and_then(|r| measure(r, &var, Measure::Final, true)).unwrap_or(0.0);
                                spec.limits.push(Limit { var, measure: Measure::Final, at_most: true, value });
                            }
                        });
                        ui.add_space(10.0);
                        card(ui, pal, "④  Go", |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Search").color(pal.muted));
                                ui.selectable_value(&mut spec.effort, 1, "Quick");
                                ui.selectable_value(&mut spec.effort, 2, "Balanced");
                                ui.selectable_value(&mut spec.effort, 3, "Thorough");
                            });
                            ui.add_space(6.0);
                            if running {
                                if ui.add(egui::Button::new(RichText::new("■  Stop").strong()).min_size(vec2(160.0, 38.0))).clicked() {
                                    if let Some(j) = &self.opt.job {
                                        j.cancel.store(true, Ordering::Relaxed);
                                    }
                                }
                            } else {
                                let go = egui::Button::new(RichText::new("◎  Optimize").size(16.0).strong().color(Color32::WHITE)).fill(pal.accent).corner_radius(CornerRadius::same(10)).min_size(vec2(200.0, 40.0));
                                let can = spec.levers.iter().any(|l| l.on) && self.opt.job.is_none();
                                if ui.add_enabled(can, go).on_disabled_hover_text("Tick at least one lever in ②").clicked() {
                                    self.opt.job = Some(start(self.model.clone(), spec.clone(), self.sheet));
                                    self.status = "Optimizing…".into();
                                }
                            }
                        });
                    });
                });
                ui.add_space(14.0);
                // ------------------------------------------------ results
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    self.opt_results(ui, &spec, running);
                });
            });
        });

        if serde_json::to_string(&spec).unwrap_or_default() != before {
            if let Some(s) = self.model.sheet_mut(self.sheet) {
                s.opt = spec;
            }
        }
    }

    fn opt_results(&mut self, ui: &mut egui::Ui, spec: &OptSpec, running: bool) {
        let pal = self.pal;
        let res = self
            .opt
            .last
            .as_ref()
            .filter(|(s, _)| *s == self.sheet)
            .map(|(_, p)| p.clone());
        card(ui, pal, "Results", |ui| {
            let Some(p) = res else {
                ui.label(
                    RichText::new(
                        "Nothing yet. Set up the three steps on the left and press Optimize.",
                    )
                    .color(pal.muted),
                );
                ui.add_space(6.0);
                ui.label(RichText::new("The search tries many combinations of your levers, keeps those that respect every limit, and homes in on the best one.").small().color(pal.muted));
                return;
            };
            if let Some(e) = &p.error {
                ui.label(RichText::new(format!("⚠ {e}")).color(pal.error));
                return;
            }
            if running {
                ui.label(
                    RichText::new(format!(
                        "Trying combination {} of about {}…",
                        p.evals, p.budget
                    ))
                    .color(pal.accent),
                );
                ui.add(
                    egui::ProgressBar::new((p.evals as f32 / p.budget.max(1) as f32).min(1.0))
                        .animate(true),
                );
            } else {
                ui.label(
                    RichText::new(format!(
                        "{} after {} tries",
                        if p.stopped { "Stopped" } else { "Done" },
                        p.evals
                    ))
                    .color(pal.ok),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} of {} — before",
                            spec.measure.label(),
                            spec.goal
                        ))
                        .small()
                        .color(pal.muted),
                    );
                    ui.label(
                        RichText::new(theme::fmt_num(p.start_goal))
                            .size(22.0)
                            .color(pal.muted),
                    );
                    if !spec.limits.is_empty() {
                        ui.label(
                            RichText::new(if p.start_ok {
                                "met the limits"
                            } else {
                                "broke a limit"
                            })
                            .small()
                            .color(if p.start_ok {
                                pal.muted
                            } else {
                                pal.error
                            }),
                        );
                    }
                });
                ui.label(RichText::new("→").size(22.0).color(pal.muted));
                ui.vertical(|ui| {
                    ui.label(RichText::new("best found").small().color(pal.muted));
                    ui.label(
                        RichText::new(theme::fmt_num(p.best_goal))
                            .size(28.0)
                            .strong()
                            .color(if p.best_ok { pal.text } else { pal.error }),
                    );
                });
                if p.start_goal.abs() > 1e-12 {
                    let pct = (p.best_goal - p.start_goal) / p.start_goal.abs() * 100.0;
                    let good = if spec.maximize {
                        pct >= 0.0
                    } else {
                        pct <= 0.0
                    };
                    ui.label(
                        RichText::new(format!("{pct:+.1}%"))
                            .size(16.0)
                            .color(if good { pal.ok } else { pal.error }),
                    );
                }
            });
            if !p.best_ok {
                ui.label(RichText::new("⚠ No combination met every limit yet — this is the closest one. Try wider lever ranges or looser limits.").color(pal.error));
            }

            ui.add_space(8.0);
            ui.label(RichText::new("Best lever settings").strong());
            egui::Grid::new("best")
                .num_columns(3)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("lever").small().color(pal.muted));
                    ui.label(RichText::new("before").small().color(pal.muted));
                    ui.label(RichText::new("best").small().color(pal.muted));
                    ui.end_row();
                    for (i, n) in p.names.iter().enumerate() {
                        ui.label(RichText::new(n).color(pal.constant));
                        ui.label(
                            RichText::new(theme::fmt_num(p.start_x[i]))
                                .monospace()
                                .color(pal.muted),
                        );
                        ui.label(
                            RichText::new(theme::fmt_num(
                                p.best_x.get(i).copied().unwrap_or(f64::NAN),
                            ))
                            .monospace()
                            .strong(),
                        );
                        ui.end_row();
                    }
                });
            if !spec.limits.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("Limits at the best settings").strong());
                for (i, l) in spec.limits.iter().enumerate() {
                    let v = p.best_measures.get(i).copied().unwrap_or(f64::NAN);
                    let ok = violation(l, v) <= 1e-9;
                    ui.label(
                        RichText::new(format!(
                            "{} {} {} {} {} — is {}",
                            if ok { "✓" } else { "✗" },
                            l.var,
                            l.measure.label(),
                            if l.at_most { "≤" } else { "≥" },
                            theme::fmt_num(l.value),
                            theme::fmt_num(v)
                        ))
                        .color(if ok { pal.ok } else { pal.error }),
                    );
                }
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !running,
                        egui::Button::new(
                            RichText::new("✓ Use these values in the model").strong(),
                        ),
                    )
                    .on_hover_text("Sets each lever's constant to the best value (⌘Z undoes)")
                    .clicked()
                {
                    self.checkpoint();
                    for (n, x) in p.names.iter().zip(&p.best_x) {
                        let key = expr::normalize(n);
                        if let Some(v) = self
                            .model
                            .vars
                            .iter_mut()
                            .find(|v| expr::normalize(&v.name) == key)
                        {
                            v.equation = format!("{}", (x * 1e6).round() / 1e6);
                        }
                    }
                    self.changed();
                    self.status = "Applied the optimized values".into();
                }
                if ui
                    .add_enabled(
                        !running && p.best_run.is_some(),
                        egui::Button::new("Keep as comparison run"),
                    )
                    .clicked()
                {
                    if let Some(mut r) = p.best_run.clone() {
                        r.name = "Optimized".into();
                        self.saved_runs.push(SavedRun {
                            run: r,
                            visible: true,
                        });
                        self.status =
                            "Kept the optimized run — it shows dashed in every graph".into();
                    }
                }
            });

            ui.add_space(10.0);
            let half = ((ui.available_height() - 40.0) / 2.0).max(120.0);
            use crate::export::ChartRef;
            let sheet = self.sheet;
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} — before (dashed) and best (solid)", spec.goal)).small().color(pal.muted));
                self.chart_copy_button(ui, ChartRef::OptGoal(sheet));
            });
            let r = self.opt_goal_plot(ui, &p, &spec.goal, half, "opt-goal");
            self.chart_menu(&r, ChartRef::OptGoal(sheet));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Best result so far, by try").small().color(pal.muted));
                self.chart_copy_button(ui, ChartRef::OptProgress(sheet));
            });
            let r = self.opt_progress_plot(ui, &p, half.min(160.0), "opt-progress");
            self.chart_menu(&r, ChartRef::OptProgress(sheet));
        });
    }
}

impl App {
    /// The goal variable before and after optimizing.
    pub fn opt_goal_plot(&self, ui: &mut egui::Ui, p: &Progress, goal: &str, height: f32, id: &str) -> egui::Response {
        let pal = self.pal;
        Plot::new(id)
            .height(height)
            .allow_scroll(false)
            .legend(egui_plot::Legend::default())
            .show(ui, |pu| {
                for (r, best) in [(&p.start_run, false), (&p.best_run, true)] {
                    if let Some(v) = r.as_ref().and_then(|r| r.get(goal).map(|v| (r, v))) {
                        let pts: Vec<[f64; 2]> = v.0.times.iter().zip(v.1).map(|(t, y)| [*t, *y]).collect();
                        let mut l = Line::new(if best { "best" } else { "before" }, PlotPoints::from(pts)).color(pal.series(0)).width(if best { 2.4 } else { 1.5 });
                        if !best {
                            l = l.style(LineStyle::dashed_loose()).color(pal.muted);
                        }
                        pu.line(l);
                    }
                }
            })
            .response
    }

    /// How the best result improved with each try.
    pub fn opt_progress_plot(&self, ui: &mut egui::Ui, p: &Progress, height: f32, id: &str) -> egui::Response {
        let pal = self.pal;
        Plot::new(id)
            .height(height)
            .allow_scroll(false)
            .allow_drag(false)
            .x_axis_label("try")
            .show(ui, |pu| {
                pu.line(Line::new("best", PlotPoints::from(p.history.clone())).color(pal.accent).width(2.0));
            })
            .response
    }

    /// The latest optimization result for a sheet, if any.
    pub fn opt_result(&self, sheet: u64) -> Option<(Progress, String)> {
        let p = self.opt.last.as_ref().filter(|(s, _)| *s == sheet).map(|(_, p)| p.clone())?;
        let goal = self.model.sheets.iter().find(|s| s.id == sheet).map(|s| s.opt.goal.clone()).unwrap_or_default();
        Some((p, goal))
    }
}

