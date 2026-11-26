//! The bottom panel: trend graphs, the table of values, causes/uses tracing, problems and runs.

use egui::{CornerRadius, RichText, Sense, Stroke, vec2};
use egui_plot::{Line, Plot, PlotPoints, VLine};

use crate::app::{App, BottomTab, Sel};
use crate::expr;
use crate::model::Kind;
use crate::theme;

impl App {
    pub fn results(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let n_problems = self.problems.len() + self.warnings().len() + self.check_failures().len();
        ui.horizontal(|ui| {
            use crate::icons::Ic;
            let runs = self.saved_runs.len() + self.run.is_some() as usize;
            for (tab, ic, label) in [
                (BottomTab::Trends, Ic::Graph, "Trends".to_string()),
                (BottomTab::Table, Ic::Table, "Table".to_string()),
                (BottomTab::Causes, Ic::Trace, "Causes & uses".to_string()),
                (BottomTab::Problems, if n_problems > 0 { Ic::Warning } else { Ic::Check }, if n_problems > 0 { format!("Problems ({n_problems})") } else { "Problems".to_string() }),
                (BottomTab::Runs, Ic::History, format!("Runs & scenarios ({})", runs + self.model.scenarios.iter().filter(|s| s.enabled).count())),
            ] {
                let active = self.bottom_tab == tab;
                let col = if tab == BottomTab::Problems && n_problems > 0 && !active { pal.aux } else { pal.muted };
                if crate::icons::button(ui, ic, Some(&label), active, col, pal.accent).clicked() {
                    self.bottom_tab = tab;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::icons::tool(ui, crate::icons::Ic::ChevronDown, "Hide the results panel (⌘J) — the diagram gets the room", pal.text, pal.accent).clicked() {
                    self.show_bottom = false;
                }
                if self.bottom_tab == BottomTab::Trends {
                    let line_like = matches!(
                        self.trend_chart,
                        crate::model::ChartKind::Line
                            | crate::model::ChartKind::Area
                            | crate::model::ChartKind::Step
                    );
                    if line_like {
                        crate::app::toggle(ui, &mut self.separate_charts, "Split", pal).on_hover_text("One chart per variable");
                    }
                    egui::ComboBox::from_id_salt("trend-chart")
                        .width(90.0)
                        .selected_text(self.trend_chart.short())
                        .show_ui(ui, |ui| {
                            for k in crate::model::ChartKind::ALL {
                                ui.selectable_value(&mut self.trend_chart, k, k.label());
                            }
                        });
                    if crate::icons::tool(ui, crate::icons::Ic::Sliders, "Customize this chart: type, titles, axes, legend, colours", pal.text, pal.accent).clicked() {
                        self.customize_chart(crate::chartstyle::ChartTarget::Trends);
                    }
                    if self.run.is_some()
                        && crate::icons::tool(ui, crate::icons::Ic::Image, "Copy this graph as a transparent picture for Word or PowerPoint", pal.text, pal.accent).clicked()
                    {
                        self.request_export(
                            crate::export::ExportWhat::Trends,
                            crate::export::ExportTo::Clipboard,
                        );
                    }
                    if self.run.is_some()
                        && crate::icons::tool(ui, crate::icons::Ic::Replay, "Replay — watch the run unfold over time again", pal.text, pal.accent).clicked()
                    {
                        let ctx = ui.ctx().clone();
                        self.reveal_start = Some(ctx.input(|i| i.time));
                        self.reveal_p = Some(0.0);
                    }
                    if !self.plotted.is_empty() && crate::icons::tool(ui, crate::icons::Ic::Close, "Remove all variables from the graph", pal.text, pal.accent).clicked() {
                        self.plotted.clear();
                    }
                    ui.menu_button("+ Add", |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                for v in self.model.vars.iter().filter(|v| {
                                    !(v.kind == Kind::Lookup && v.equation.trim().is_empty())
                                }) {
                                    let on = self.plotted.contains(&v.name);
                                    if ui
                                        .selectable_label(
                                            on,
                                            RichText::new(&v.name).color(pal.kind(v.kind)),
                                        )
                                        .clicked()
                                    {
                                        if on {
                                            self.plotted.retain(|n| n != &v.name)
                                        } else {
                                            self.plotted.push(v.name.clone())
                                        }
                                    }
                                    // a set variable: pick single items to draw next to (or instead of) the total
                                    if let Some(sn) = &v.set {
                                        let items = self.model.items_of(v);
                                        ui.menu_button(RichText::new(format!("      ▸ one {sn} at a time ({})", items.len())).small().color(pal.muted), |ui| {
                                            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                                for it in &items {
                                                    let name = format!("{}[{}]", v.name, it);
                                                    let on = self.plotted.contains(&name);
                                                    if ui.selectable_label(on, RichText::new(it).color(pal.kind(v.kind))).clicked() {
                                                        if on {
                                                            self.plotted.retain(|n| n != &name)
                                                        } else {
                                                            self.plotted.push(name)
                                                        }
                                                    }
                                                }
                                            });
                                        });
                                    }
                                }
                            });
                    });
                }
            });
        });
        ui.add_space(4.0);
        match self.bottom_tab {
            BottomTab::Trends => self.trends(ui),
            BottomTab::Table => self.table(ui),
            BottomTab::Causes => self.causes(ui),
            BottomTab::Problems => self.problems_tab(ui),
            BottomTab::Runs => self.runs_tab(ui),
        }
    }

    /// Variables shown in graphs and tables: the pinned ones, plus the selection.
    pub fn shown_vars(&self) -> Vec<(String, bool)> {
        let mut v: Vec<(String, bool)> = self
            .plotted
            .iter()
            .filter(|n| self.model.by_name(&expr::normalize(n)).is_some())
            .map(|n| (n.clone(), true))
            .collect();
        if let Some(Sel::Var(id)) = self.sel {
            if let Some(s) = self.model.var(id) {
                if !(s.kind == Kind::Lookup && s.equation.trim().is_empty())
                    && !v.iter().any(|(n, _)| n == &s.name)
                {
                    v.push((s.name.clone(), false));
                }
            }
        }
        v
    }

    fn trends(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let shown = self.shown_vars();
        // chips
        ui.horizontal_wrapped(|ui| {
            for (i, (n, pinned)) in shown.iter().enumerate() {
                let c = pal.series(i);
                let txt = if *pinned {
                    format!("● {n}  ×")
                } else {
                    format!("○ {n}  + pin")
                };
                let b = egui::Button::new(RichText::new(txt).small().color(c))
                    .fill(c.gamma_multiply(0.08))
                    .stroke(Stroke::new(1.0, c.gamma_multiply(0.35)))
                    .corner_radius(CornerRadius::same(10));
                if ui
                    .add(b)
                    .on_hover_text(if *pinned {
                        "Remove from graph"
                    } else {
                        "Keep on the graph"
                    })
                    .clicked()
                {
                    if *pinned {
                        self.plotted.retain(|x| x != n)
                    } else {
                        self.plotted.push(n.clone())
                    }
                }
            }
            if shown.is_empty() {
                ui.label(
                    RichText::new("Select a variable or use “Add to graph” to plot it.")
                        .color(pal.muted),
                );
            }
        });
        let Some(run) = self.run.clone() else {
            ui.label(
                RichText::new("No results yet — fix the problems or press Run.").color(pal.muted),
            );
            return;
        };
        let scrub_t = self.time_index().map(|i| run.times[i]);
        let saved = self.compare_runs();
        let series = |name: &str, r: &crate::sim::Run| -> Option<Vec<[f64; 2]>> {
            r.get(name)
                .map(|v| r.times.iter().zip(v).map(|(t, y)| [*t, *y]).collect())
        };
        let mut clicked_t = None;
        let mut menus: Vec<(egui::Response, crate::export::ChartRef)> = Vec::new();
        let kind = self.trend_chart;
        let line_like = matches!(kind, crate::model::ChartKind::Line | crate::model::ChartKind::Area | crate::model::ChartKind::Step);
        if !(self.separate_charts && shown.len() > 1 && line_like) {
            let names: Vec<String> = shown.iter().map(|(n, _)| n.clone()).collect();
            let h = ui.available_height();
            let style = self.model.trend_style.clone();
            clicked_t = self.chart_styled(ui, "trend", kind, &names, h, &style, Some(crate::chartstyle::ChartTarget::Trends));
        } else if self.separate_charts && shown.len() > 1 {
            let cols = if shown.len() <= 2 {
                shown.len()
            } else if shown.len() <= 4 {
                2
            } else {
                3
            };
            let rows = shown.len().div_ceil(cols);
            let w = (ui.available_width() - 10.0 * (cols as f32 - 1.0)) / cols as f32;
            let h = ((ui.available_height() - 6.0 * rows as f32) / rows as f32).max(90.0);
            egui::Grid::new("small multiples")
                .num_columns(cols)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    for (i, (n, _)) in shown.iter().enumerate() {
                        let c = pal.series(i);
                        ui.vertical(|ui| {
                            ui.set_width(w);
                            let unit = self
                                .model
                                .by_name(&expr::normalize(n))
                                .map(|v| v.units.clone())
                                .unwrap_or_default();
                            ui.label(
                                RichText::new(format!("{n}  {unit}"))
                                    .small()
                                    .strong()
                                    .color(c),
                            );
                            let plot = self.hold_axes(
                                Plot::new(("sm", n.as_str()))
                                    .width(w)
                                    .height(h - 20.0)
                                    .allow_scroll(false),
                                &run,
                                std::slice::from_ref(n),
                            );
                            let r = plot.show(ui, |pu| {
                                for s in &saved {
                                    if let Some(p) = series(n, &s.run) {
                                        let (col, style, _) = self.compare_style(s, c, true);
                                        pu.line(
                                            Line::new(s.run.name.clone(), PlotPoints::from(p))
                                                .color(col)
                                                .style(style),
                                        );
                                    }
                                }
                                if let Some(p) = self.plot_pts(&run, n) {
                                    self.draw_head(pu, &p, c);
                                    pu.line(
                                        Line::new(n.clone(), PlotPoints::from(p))
                                            .color(c)
                                            .width(2.2),
                                    );
                                }
                                if let Some(t) = scrub_t {
                                    pu.vline(
                                        VLine::new("", t).color(pal.muted.gamma_multiply(0.5)),
                                    );
                                }
                                pu.pointer_coordinate()
                                    .filter(|_| pu.response().clicked())
                                    .map(|p| p.x)
                            });
                            menus.push((r.response.clone(), crate::export::ChartRef::Vars { kind: crate::model::ChartKind::Line, vars: vec![n.clone()], title: format!("{n}  {unit}") }));
                            if let Some(t) = r.inner {
                                clicked_t = Some(t);
                            }
                        });
                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        }
        for (r, c) in menus {
            self.chart_menu(&r, c);
        }
        // clicking a graph moves the diagram's time to that moment
        if let Some(t) = clicked_t {
            let i = run
                .times
                .partition_point(|x| *x < t)
                .min(run.times.len() - 1);
            self.scrub = Some(i);
        }
    }

    fn table(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let Some(run) = self.run.clone() else {
            ui.label(RichText::new("No results yet").color(pal.muted));
            return;
        };
        let mut shown: Vec<String> = self.shown_vars().into_iter().map(|(n, _)| n).collect();
        if shown.is_empty() {
            shown = self
                .model
                .vars
                .iter()
                .filter(|v| run.get(&v.name).is_some())
                .map(|v| v.name.clone())
                .collect();
        }
        let col_w = 120.0;
        let row_h = 20.0;
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [80.0, row_h],
                    egui::Label::new(
                        RichText::new(format!("Time ({})", run.time_units))
                            .strong()
                            .small(),
                    ),
                );
                for n in &shown {
                    ui.add_sized(
                        [col_w, row_h],
                        egui::Label::new(RichText::new(n).strong().small()).truncate(),
                    );
                }
            });
            ui.separator();
            let cur = self.time_index();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(ui, row_h, run.times.len(), |ui, range| {
                    for i in range {
                        let (r, _) = ui.allocate_exact_size(
                            vec2(80.0 + (col_w + 8.0) * shown.len() as f32, row_h),
                            Sense::hover(),
                        );
                        if Some(i) == cur {
                            ui.painter()
                                .rect_filled(r, CornerRadius::same(4), pal.accent_soft);
                        } else if i % 2 == 0 {
                            ui.painter().rect_filled(r, CornerRadius::ZERO, pal.surface);
                        }
                        let mut x = r.left() + 4.0;
                        ui.painter().text(
                            egui::pos2(x, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            theme::fmt_num(run.times[i]),
                            egui::FontId::monospace(11.0),
                            pal.muted,
                        );
                        x += 80.0 + 4.0;
                        for n in &shown {
                            let v = run.get(n).and_then(|s| s.get(i)).copied();
                            ui.painter().text(
                                egui::pos2(x + col_w - 8.0, r.center().y),
                                egui::Align2::RIGHT_CENTER,
                                v.map(theme::fmt_num).unwrap_or("".into()),
                                egui::FontId::monospace(11.0),
                                pal.text,
                            );
                            x += col_w + 8.0;
                        }
                    }
                });
        });
    }

    /// What a variable's value is computed from: (name, sign) where sign marks flows into (+) or out of (−) a stock.
    fn causes_of(&self, id: u64) -> Vec<(u64, &'static str)> {
        let Some(v) = self.model.var(id) else {
            return vec![];
        };
        let mut out: Vec<(u64, &'static str)> = Vec::new();
        if v.kind == Kind::Stock {
            for f in self.model.vars.iter().filter(|f| f.kind == Kind::Flow) {
                if f.to == Some(id) {
                    out.push((f.id, "+"));
                }
                if f.from == Some(id) {
                    out.push((f.id, "−"));
                }
            }
        }
        if let Ok(e) = expr::parse(&v.equation) {
            let mut refs = Vec::new();
            expr::references(&e, &mut refs);
            for r in refs {
                if let Some(s) = self.model.by_name(&r) {
                    if s.id != id && !out.iter().any(|(x, _)| *x == s.id) {
                        out.push((s.id, if v.kind == Kind::Stock { "initial" } else { "" }));
                    }
                }
            }
        }
        out
    }

    fn uses_of(&self, id: u64) -> Vec<(u64, &'static str)> {
        let all: Vec<u64> = self.model.vars.iter().map(|v| v.id).collect();
        all.into_iter()
            .filter(|o| *o != id)
            .filter_map(|o| {
                self.causes_of(o)
                    .into_iter()
                    .find(|(c, _)| *c == id)
                    .map(|(_, s)| (o, s))
            })
            .collect()
    }

    fn tree_row(&mut self, ui: &mut egui::Ui, id: u64, sign: &str, depth: usize) {
        let pal = self.pal;
        let Some(v) = self.model.var(id).cloned() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 18.0);
            if depth > 0 {
                ui.label(RichText::new("└").color(pal.border));
            }
            if !sign.is_empty() {
                ui.label(RichText::new(sign).small().color(if sign == "−" {
                    pal.error
                } else {
                    pal.ok
                }));
            }
            let c = pal.kind(v.kind);
            if ui
                .add(egui::Button::new(RichText::new(&v.name).color(c)).frame(false))
                .on_hover_text(format!("{} · {}", v.kind.label(), v.equation))
                .clicked()
            {
                self.sel = Some(Sel::Var(id));
            }
            if let Some(val) = self.value_now(&v.name) {
                ui.label(
                    RichText::new(theme::fmt_num(val))
                        .monospace()
                        .small()
                        .color(pal.muted),
                );
            }
            if let Some(vals) = self.run.as_ref().and_then(|r| r.get(&v.name)) {
                let (r, _) = ui.allocate_exact_size(vec2(70.0, 16.0), Sense::hover());
                theme::sparkline(ui.painter(), r, vals, c, None, false);
            }
        });
    }

    fn causes(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let Some(Sel::Var(id)) = self.sel else {
            ui.label(
                RichText::new(
                    "Select a variable on the diagram to trace what drives it and what it affects.",
                )
                .color(pal.muted),
            );
            return;
        };
        let name = self
            .model
            .var(id)
            .map(|v| v.name.clone())
            .unwrap_or_default();
        ui.columns(2, |cols| {
            cols[0].label(RichText::new(format!("What drives {name}")).strong());
            egui::ScrollArea::vertical()
                .id_salt("causes")
                .show(&mut cols[0], |ui| {
                    self.tree_row(ui, id, "", 0);
                    let mut seen = vec![id];
                    self.walk(ui, id, 1, &mut seen, true);
                });
            cols[1].label(RichText::new(format!("What {name} affects")).strong());
            egui::ScrollArea::vertical()
                .id_salt("uses")
                .show(&mut cols[1], |ui| {
                    self.tree_row(ui, id, "", 0);
                    let mut seen = vec![id];
                    self.walk(ui, id, 1, &mut seen, false);
                });
        });
    }

    fn walk(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        depth: usize,
        seen: &mut Vec<u64>,
        causes: bool,
    ) {
        if depth > 3 {
            return;
        }
        let next = if causes {
            self.causes_of(id)
        } else {
            self.uses_of(id)
        };
        for (c, sign) in next {
            self.tree_row(ui, c, sign, depth);
            if !seen.contains(&c) {
                seen.push(c);
                self.walk(ui, c, depth + 1, seen, causes);
            }
        }
    }

    /// Limits the model states about itself that this run breaks, and stocks that don't balance.
    pub fn check_failures(&self) -> Vec<crate::checks::Failure> {
        match &self.run {
            Some(r) => crate::checks::all(&self.model, r),
            None => Vec::new(),
        }
    }

    /// Advice that doesn't stop the run: arrows not used, inputs without arrows, units.
    pub fn warnings(&self) -> Vec<(u64, String)> {
        if let Some((rev, w)) = &*self.warn_cache.borrow() {
            if *rev == self.rev {
                return w.clone();
            }
        }
        let w = crate::validate::suggestions(&self.model);
        *self.warn_cache.borrow_mut() = Some((self.rev, w.clone()));
        w
    }

    fn problems_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let warnings = self.warnings();
        let failures = self.check_failures();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let layer_errs = self.layer_errors();
            if self.problems.is_empty() && warnings.is_empty() && failures.is_empty() && layer_errs.is_empty() {
                let n = self.model.checks.len();
                let extra = if n > 0 { format!(", {n} check{} pass", if n == 1 { "" } else { "s" }) } else { String::new() };
                ui.label(RichText::new(format!("✓ Everything checks out: every equation is complete, every arrow is used, every stock balances{extra}.")).color(pal.ok));
            }
            for f in failures {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(pal.error));
                    if ui.add(egui::Button::new(RichText::new(&f.message).color(pal.text)).frame(false)).clicked() {
                        if let Some(id) = f.var {
                            self.sel = Some(Sel::Var(id));
                        }
                    }
                });
            }
            for (var, msg) in layer_errs {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(pal.error));
                    ui.label(RichText::new("Layers").strong());
                    if ui.add(egui::Button::new(RichText::new(msg).color(pal.text)).frame(false)).clicked() {
                        match var.and_then(|n| self.model.by_name(&expr::normalize(&n)).map(|v| v.id)) {
                            Some(id) => self.sel = Some(Sel::Var(id)),
                            None => self.layers.open = true,
                        }
                    }
                });
            }
            for p in self.problems.clone() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(pal.error));
                    if let Some(n) = &p.var {
                        if ui.link(RichText::new(n).strong()).clicked() {
                            self.sel = self.model.by_name(&expr::normalize(n)).map(|v| Sel::Var(v.id));
                        }
                    }
                    ui.label(&p.message);
                });
            }
            if !warnings.is_empty() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Suggestions").strong().color(pal.muted));
                    if ui.small_button("Add all missing arrows").clicked() {
                        self.checkpoint();
                        let n = crate::canvas::sync_links(&mut self.model);
                        self.status = format!("Added {n} arrow(s)");
                    }
                });
                for (id, w) in warnings {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("•").color(pal.aux));
                        if ui.add(egui::Button::new(RichText::new(w).color(pal.text)).frame(false)).clicked() {
                            self.sel = Some(Sel::Var(id));
                        }
                    });
                }
            }
        });
    }

    fn runs_tab(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        // scenarios first: they re-run with every change
        ui.horizontal(|ui| {
            ui.label(RichText::new("Scenarios").strong());
            ui.checkbox(&mut self.scenarios_everywhere, "show in every chart");
            if crate::icons::button(ui, crate::icons::Ic::Scenarios, Some("Open the Scenarios sheet"), false, pal.accent, pal.accent).clicked() {
                if let Some(id) = self.model.sheets.iter().find(|s| s.kind == crate::model::SheetKind::Scenarios).map(|s| s.id) {
                    self.sheet = id;
                } else {
                    self.add_sheet_pub(crate::model::SheetKind::Scenarios, crate::model::TableMode::Variables);
                }
            }
        });
        let mut toggled = false;
        let ids: Vec<(u64, String, bool, String)> = self
            .model
            .scenarios
            .iter()
            .map(|s| (s.id, s.name.clone(), s.enabled, s.changes.iter().map(|c| format!("{} = {}", c.var, c.value)).collect::<Vec<_>>().join(", ")))
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (id, name, on, what) in &ids {
                let c = self.scenario_color(*id);
                let mut v = *on;
                if ui.checkbox(&mut v, RichText::new(name).color(c)).on_hover_text(if what.is_empty() { "No changes yet".to_string() } else { what.clone() }).changed() {
                    if let Some(s) = self.model.scenarios.iter_mut().find(|s| s.id == *id) {
                        s.enabled = v;
                    }
                    toggled = true;
                }
            }
            if ids.is_empty() {
                ui.label(RichText::new("None yet — add what-ifs on a Scenarios sheet.").color(pal.muted));
            }
        });
        if toggled {
            self.changed();
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Keep current run").on_hover_text("Freeze this result so you can compare it with what happens after you change something").clicked() {
                self.keep_run();
            }
            if !self.saved_runs.is_empty() && ui.button("Remove all kept runs").clicked() {
                self.saved_runs.clear();
            }
        });
        ui.add_space(4.0);
        if let Some(r) = &self.run {
            ui.label(
                RichText::new(format!("● Current — {} points, solid lines", r.times.len()))
                    .color(pal.accent),
            );
        }
        let mut remove = None;
        for (i, s) in self.saved_runs.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.visible, "");
                ui.add(egui::TextEdit::singleline(&mut s.run.name).desired_width(200.0));
                ui.label(RichText::new("dashed lines").small().color(pal.muted));
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            self.saved_runs.remove(i);
        }
    }
}
