//! The Scenarios sheet: named what-ifs edited in a table, all run at once, and compared
//! over time, as a summary per scenario, and in a table of key numbers.

use egui::{Color32, CornerRadius, RichText, Sense, Stroke, vec2};
use egui_plot::{Bar, BarChart, Legend, Line, Plot, PlotPoints, VLine};

use crate::app::App;
use crate::expr;
use crate::model::{Change, Kind, Measure, Scenario};
use crate::optimize::measure;
use crate::sim::Run;
use crate::theme;

fn card<R>(ui: &mut egui::Ui, pal: theme::Palette, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
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
    /// Baseline plus every enabled scenario that ran: (name, run, colour).
    pub fn scenario_lineup(&self) -> Vec<(String, Run, Color32)> {
        let mut v = Vec::new();
        if let Some(r) = &self.run {
            v.push(("Baseline".to_string(), r.clone(), self.pal.series(0)));
        }
        for (id, r) in &self.scenario_runs {
            if let (Ok(run), Some(sc)) = (r, self.model.scenarios.iter().find(|s| s.id == *id)) {
                v.push((sc.name.clone(), run.clone(), self.scenario_color(*id)));
            }
        }
        v
    }

    /// One variable across the baseline and all scenarios, over time.
    pub fn scenario_lines(&self, ui: &mut egui::Ui, var: &str, height: f32, id: &str) -> egui::Response {
        let pal = self.pal;
        let lineup = self.scenario_lineup();
        let upto = |r: &Run| self.reveal_len(r.times.len());
        let scrub_t = self.time_index().and_then(|i| self.run.as_ref().map(|r| r.times[i]));
        let units = self.run.as_ref().map(|r| r.time_units.clone()).unwrap_or_default();
        let mut plot = Plot::new(id).height(height).legend(Legend::default().position(egui_plot::Corner::RightTop)).allow_scroll(false).x_axis_label(format!("Time ({units})"));
        if self.reveal_p.is_some() {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for (_, r, _) in &lineup {
                for v in r.get(var).into_iter().flatten() {
                    lo = lo.min(*v);
                    hi = hi.max(*v);
                }
            }
            if let Some(r) = &self.run {
                plot = plot.include_x(r.times[0]).include_x(*r.times.last().unwrap());
            }
            if lo.is_finite() {
                plot = plot.include_y(lo).include_y(hi);
            }
        }
        plot.show(ui, |pu| {
            for (i, (name, r, c)) in lineup.iter().enumerate() {
                if let Some(v) = r.get(var) {
                    let pts: Vec<[f64; 2]> = r.times.iter().zip(v).take(upto(r)).map(|(t, y)| [*t, *y]).collect();
                    self.draw_head(pu, &pts, *c);
                    // the legend says which line is the model itself and which are scenarios
                    let label = if i == 0 { "Baseline (current model)".to_string() } else { format!("Scenario: {name}") };
                    pu.line(Line::new(label, PlotPoints::from(pts)).color(*c).width(if i == 0 { 2.8 } else { 2.2 }));
                }
            }
            if let Some(t) = scrub_t {
                pu.vline(VLine::new("", t).color(pal.muted.gamma_multiply(0.5)));
            }
        })
        .response
    }

    /// One number per scenario (the chosen measure of `var`), as bars.
    pub fn scenario_bars(&self, ui: &mut egui::Ui, var: &str, m: Measure, height: f32, id: &str) -> egui::Response {
        let lineup = self.scenario_lineup();
        let names: Vec<String> = lineup.iter().map(|x| x.0.clone()).collect();
        let fmt_names = names.clone();
        Plot::new(id)
            .height(height)
            .allow_scroll(false)
            .allow_drag(false)
            .include_y(0.0)
            .y_axis_label(format!("{} of {var}", m.label()))
            .x_axis_formatter(move |mark, _| {
                let i = mark.value.round();
                if (mark.value - i).abs() < 1e-6 && i >= 0.0 { fmt_names.get(i as usize).cloned().unwrap_or_default() } else { String::new() }
            })
            .show(ui, |pu| {
                for (i, (name, r, c)) in lineup.iter().enumerate() {
                    if let Some(val) = measure(r, var, m, false) {
                        pu.bar_chart(BarChart::new(name.clone(), vec![Bar::new(i as f64, val).width(0.6).name(name)]).color(*c));
                    }
                }
            })
            .response
    }

    pub fn scenarios_sheet(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let sheet_id = self.sheet;
        let Some(sheet) = self.model.sheets.iter().find(|s| s.id == sheet_id).cloned() else { return };
        egui::Frame::new().fill(pal.canvas).inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&sheet.name).size(20.0).strong());
                    ui.label(RichText::new("What happens if…? Each scenario changes a few values; all of them run together and update as you edit.").color(pal.muted));
                });
                ui.add_space(10.0);
                self.scenario_table(ui, &sheet);
                ui.add_space(12.0);
                self.scenario_compare(ui, &sheet);
            });
        });
    }

    fn scenario_table(&mut self, ui: &mut egui::Ui, sheet: &crate::model::Sheet) {
        let pal = self.pal;
        let levers: Vec<String> = sheet.columns.clone();
        // values each lever has in the model itself
        let base: Vec<String> = levers.iter().map(|l| self.model.by_name(&expr::normalize(l)).map(|v| v.equation.clone()).unwrap_or_else(|| "?".into())).collect();
        let mut edits: Vec<Scenario> = self.model.scenarios.clone();
        let before = serde_json::to_string(&edits).unwrap_or_default();
        let mut remove = None;
        let mut duplicate = None;
        let mut add_lever: Option<String> = None;
        let mut drop_lever: Option<usize> = None;
        card(ui, pal, "Scenarios", |ui| {
            egui::Grid::new("scen-table").num_columns(levers.len() + 4).spacing([14.0, 8.0]).striped(true).show(ui, |ui| {
                ui.label("");
                ui.label(RichText::new("Scenario").strong().color(pal.muted));
                for (i, l) in levers.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let k = self.model.by_name(&expr::normalize(l)).map(|v| v.kind).unwrap_or(Kind::Constant);
                        ui.label(RichText::new(l).strong().color(pal.kind(k)));
                        if crate::icons::tool(ui, crate::icons::Ic::Close, "Stop varying this value", pal.muted, pal.accent).clicked() {
                            drop_lever = Some(i);
                        }
                    });
                }
                ui.menu_button(RichText::new("+ Value to vary").color(pal.accent), |ui| {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for v in self.model.vars.iter().filter(|v| matches!(v.kind, Kind::Constant | Kind::Aux | Kind::Stock) && !levers.contains(&v.name)) {
                            let hint = if v.kind == Kind::Stock { "starting value" } else { v.kind.label() };
                            if ui.button(RichText::new(format!("{}   ({hint})", v.name)).color(pal.kind(v.kind))).clicked() {
                                add_lever = Some(v.name.clone());
                                ui.close();
                            }
                        }
                    });
                });
                ui.label("");
                ui.end_row();

                // the model itself
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 6.0, pal.series(0));
                ui.label(RichText::new("Baseline — the model as it is").strong());
                for b in &base {
                    ui.label(RichText::new(b).monospace().color(pal.muted));
                }
                ui.label("");
                ui.label("");
                ui.end_row();

                for (i, sc) in edits.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut sc.enabled, "").on_hover_text("Run and show this scenario");
                        let c = self.scenario_color(sc.id);
                        let (r, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                        ui.painter().circle_filled(r.center(), 6.0, c);
                        resp.on_hover_text("Colour (pick another in the menu ⋯)");
                    });
                    ui.add(egui::TextEdit::singleline(&mut sc.name).desired_width(170.0));
                    for (j, l) in levers.iter().enumerate() {
                        let mut val = sc.value_of(l).unwrap_or("").to_string();
                        let r = ui.add(egui::TextEdit::singleline(&mut val).desired_width(90.0).font(egui::TextStyle::Monospace).hint_text(&base[j]));
                        if r.changed() {
                            sc.changes.retain(|c| expr::normalize(&c.var) != expr::normalize(l));
                            if !val.trim().is_empty() {
                                sc.changes.push(Change { var: l.clone(), value: val });
                            }
                        }
                    }
                    ui.menu_button("⋯", |ui| {
                        if ui.button("Duplicate").clicked() {
                            duplicate = Some(i);
                            ui.close();
                        }
                        ui.label(RichText::new("Colour").small().color(pal.muted));
                        ui.horizontal(|ui| {
                            for k in 0..8 {
                                let c = pal.series(k);
                                let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                                ui.painter().circle_filled(r.center(), 7.0, c);
                                if resp.clicked() {
                                    sc.color = Some([c.r(), c.g(), c.b()]);
                                }
                            }
                        });
                        ui.separator();
                        if ui.button(RichText::new("Delete scenario").color(pal.error)).clicked() {
                            remove = Some(i);
                            ui.close();
                        }
                    });
                    ui.label("");
                    ui.end_row();
                    if let Some((_, Err(e))) = self.scenario_runs.iter().find(|(id, _)| *id == sc.id) {
                        ui.label("");
                        ui.label(RichText::new(format!("⚠ {e}")).small().color(pal.error));
                        ui.end_row();
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                use crate::icons::{Ic, button};
                if button(ui, Ic::Plus, Some("New scenario"), false, pal.accent, pal.accent).clicked() {
                    let id = self.model.new_id();
                    let n = edits.len() + 1;
                    edits.push(Scenario { id, name: format!("Scenario {n}"), color: None, enabled: true, changes: Vec::new(), notes: String::new() });
                }
                if button(ui, Ic::Play, Some("Run all"), false, pal.text, pal.accent).on_hover_text("Run the baseline and every scenario, and watch them unfold").clicked() {
                    let ctx = ui.ctx().clone();
                    self.run_and_play(&ctx);
                }
                ui.checkbox(&mut self.scenarios_everywhere, "Show scenarios in every chart");
                ui.label(RichText::new("Leave a cell empty to keep the baseline value. A cell can also hold a formula, e.g. STEP(2, 10).").small().color(pal.muted));
            });
        });
        if let Some(i) = duplicate {
            let mut c = edits[i].clone();
            c.id = self.model.new_id();
            c.name = format!("{} copy", c.name);
            c.color = None;
            edits.insert(i + 1, c);
        }
        if let Some(i) = remove {
            edits.remove(i);
        }
        if serde_json::to_string(&edits).unwrap_or_default() != before {
            self.checkpoint();
            self.model.scenarios = edits;
            self.changed();
        }
        if add_lever.is_some() || drop_lever.is_some() {
            self.checkpoint();
            if let Some(s) = self.model.sheet_mut(sheet.id) {
                if let Some(l) = add_lever {
                    s.columns.push(l);
                }
                if let Some(i) = drop_lever {
                    s.columns.remove(i);
                }
            }
        }
    }

    fn scenario_compare(&mut self, ui: &mut egui::Ui, sheet: &crate::model::Sheet) {
        let pal = self.pal;
        let mut vars = sheet.compare_vars.clone();
        if vars.is_empty() {
            vars = self.model.vars.iter().filter(|v| v.kind == Kind::Stock).take(1).map(|v| v.name.clone()).collect();
        }
        let mut m = sheet.compare_measure.unwrap_or(Measure::Final);
        let mut changed = false;
        card(ui, pal, "Compare", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Variables").color(pal.muted));
                for (i, n) in vars.clone().iter().enumerate() {
                    let b = egui::Button::new(RichText::new(format!("{n}  ×")).color(pal.series(i))).fill(pal.series(i).gamma_multiply(0.08)).corner_radius(CornerRadius::same(10));
                    if ui.add(b).on_hover_text("Stop comparing this variable").clicked() {
                        vars.retain(|x| x != n);
                        changed = true;
                    }
                }
                ui.menu_button(RichText::new("+ Add").color(pal.accent), |ui| {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        let choices: Vec<(String, Kind)> = self.model.vars.iter().filter(|v| !(v.kind == Kind::Lookup && v.equation.trim().is_empty()) && !vars.contains(&v.name)).map(|v| (v.name.clone(), v.kind)).collect();
                        for (n, k) in choices {
                            if ui.button(RichText::new(&n).color(pal.kind(k))).clicked() {
                                vars.push(n);
                                changed = true;
                                ui.close();
                            }
                        }
                    });
                });
                ui.separator();
                ui.label(RichText::new("Summary").color(pal.muted));
                egui::ComboBox::from_id_salt("scen-measure").selected_text(m.label()).show_ui(ui, |ui| {
                    for x in [Measure::Final, Measure::Peak, Measure::Lowest, Measure::Total, Measure::Average] {
                        if ui.selectable_value(&mut m, x, x.label()).changed() {
                            changed = true;
                        }
                    }
                });
            });
            ui.add_space(8.0);
            if self.run.is_none() {
                ui.label(RichText::new("Fix the model's problems first — nothing has run yet.").color(pal.muted));
                return;
            }
            use crate::export::ChartRef;
            for (i, v) in vars.iter().enumerate() {
                if i > 0 {
                    ui.add_space(12.0);
                }
                let w = ui.available_width();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(v).size(15.0).strong().color(pal.text));
                    let unit = self.model.by_name(&expr::normalize(v)).map(|x| x.units.clone()).unwrap_or_default();
                    ui.label(RichText::new(unit).color(pal.muted));
                });
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(w * 0.62);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Over time").small().color(pal.muted));
                            self.chart_copy_button(ui, ChartRef::ScenarioLines(v.clone()));
                        });
                        let r = self.scenario_lines(ui, v, 280.0, &format!("scen-lines-{v}"));
                        self.chart_menu(&r, ChartRef::ScenarioLines(v.clone()));
                    });
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{} by scenario", m.label())).small().color(pal.muted));
                            self.chart_copy_button(ui, ChartRef::ScenarioBars(v.clone(), m));
                        });
                        let r = self.scenario_bars(ui, v, m, 280.0, &format!("scen-bars-{v}"));
                        self.chart_menu(&r, ChartRef::ScenarioBars(v.clone(), m));
                    });
                });
            }
            // the key numbers side by side
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Key numbers").strong());
                if crate::icons::tool(ui, crate::icons::Ic::Table, "Copy this table (for Excel)", pal.muted, pal.accent).clicked() {
                    let t = self.scenario_table_tsv(&vars);
                    ui.ctx().copy_text(t);
                    self.status = "Copied the scenario comparison — paste into any spreadsheet".into();
                }
            });
            let lineup = self.scenario_lineup();
            egui::Grid::new("scen-numbers").striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                ui.label(RichText::new("Scenario").strong().color(pal.muted));
                for v in &vars {
                    for h in ["final", "peak", "peak at", "vs baseline"] {
                        ui.label(RichText::new(format!("{v} · {h}")).small().strong().color(pal.muted));
                    }
                }
                ui.end_row();
                let base: Vec<Option<f64>> = vars.iter().map(|v| lineup.first().and_then(|(_, r, _)| measure(r, v, m, false))).collect();
                for (name, r, c) in &lineup {
                    ui.horizontal(|ui| {
                        let (rr, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                        ui.painter().circle_filled(rr.center(), 5.0, *c);
                        ui.label(RichText::new(name).strong());
                    });
                    for (k, v) in vars.iter().enumerate() {
                        let fin = measure(r, v, Measure::Final, false);
                        let peak = measure(r, v, Measure::Peak, false);
                        let peak_t = r.get(v).and_then(|s| s.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| r.times[i]));
                        let this = measure(r, v, m, false);
                        let vs = match (this, base[k]) {
                            (Some(a), Some(b)) if b.abs() > 1e-12 && name != "Baseline" => format!("{:+.1}%", (a - b) / b.abs() * 100.0),
                            _ => "—".into(),
                        };
                        for t in [fin.map(theme::fmt_num), peak.map(theme::fmt_num), peak_t.map(theme::fmt_num)] {
                            ui.label(RichText::new(t.unwrap_or_default()).monospace());
                        }
                        ui.label(RichText::new(vs).monospace().color(pal.muted)).on_hover_text(format!("{} compared with the baseline", m.label()));
                    }
                    ui.end_row();
                }
            });
        });
        if changed {
            if let Some(s) = self.model.sheet_mut(sheet.id) {
                s.compare_vars = vars;
                s.compare_measure = Some(m);
            }
        }
    }

    pub fn scenario_table_tsv(&self, vars: &[String]) -> String {
        let mut s = String::from("Scenario");
        for v in vars {
            s.push_str(&format!("\t{v} final\t{v} peak\t{v} total"));
        }
        s.push('\n');
        for (name, r, _) in self.scenario_lineup() {
            s.push_str(&name);
            for v in vars {
                for m in [Measure::Final, Measure::Peak, Measure::Total] {
                    s.push('\t');
                    s.push_str(&measure(&r, v, m, false).map(|x| x.to_string()).unwrap_or_default());
                }
            }
            s.push('\n');
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenarios_run_and_differ() {
        let app = App::new(&egui::Context::default(), None);
        assert_eq!(app.scenario_runs.len(), 3, "the example's three scenarios ran");
        let lineup = app.scenario_lineup();
        let peak = |name: &str| measure(&lineup.iter().find(|x| x.0 == name).unwrap().1, "Infected", Measure::Peak, false).unwrap();
        assert!(peak("Distancing") < peak("Baseline") * 0.8);
        assert!(peak("Both") < peak("Distancing"));
        let t = app.scenario_table_tsv(&["Infected".into()]);
        assert_eq!(t.lines().count(), 5);
    }
}
