//! One chart renderer for every graph in the app: line, area, stacked area, step,
//! bars at the current time, phase plots and donuts.

use egui::{Align2, Color32, FontId, Pos2, RichText, Sense, Stroke, pos2, vec2};
use egui_plot::{Bar, BarChart, Legend, Line, LineStyle, Plot, PlotPoint, PlotPoints, Points, Polygon, Text, VLine};

use crate::app::App;
use crate::chartstyle::ChartTarget;
use crate::export::ChartRef;
use crate::model::{ChartKind, ChartStyle, LegendPos};
use crate::theme;

impl App {
    /// Draw `names` as a chart of `kind` with the default look.
    pub fn chart(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, kind: ChartKind, names: &[String], height: f32) -> Option<f64> {
        self.chart_styled(ui, id, kind, names, height, &ChartStyle::default(), None)
    }

    /// Draw `names` as a chart of `kind`, styled by `style`. `target` says which saved chart
    /// this is, so its menu can offer "Customize chart…". Returns a time the person clicked.
    #[allow(clippy::too_many_arguments)]
    pub fn chart_styled(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, kind: ChartKind, names: &[String], height: f32, style: &ChartStyle, target: Option<ChartTarget>) -> Option<f64> {
        let pal = self.pal;
        // titles and the caption come out of the same height budget, so the chart never outgrows its space
        let top = ui.cursor().top();
        let budget = height;
        if !style.title.is_empty() {
            ui.label(RichText::new(&style.title).size(15.5).strong().color(pal.text));
        }
        if !style.subtitle.is_empty() {
            ui.label(RichText::new(&style.subtitle).size(12.5).color(pal.muted));
        }
        let Some(run) = self.run.clone() else {
            ui.label(RichText::new("No results yet").color(pal.muted));
            return None;
        };
        if names.is_empty() {
            ui.label(RichText::new("Pick variables to show").color(pal.muted));
            return None;
        }
        let cref = ChartRef::Styled { kind, vars: names.to_vec(), style: style.clone(), target };
        let ti = self.time_index().unwrap_or(run.times.len().saturating_sub(1));
        let scrub_t = run.times.get(ti).copied().filter(|_| style.time_marker);
        // kept runs and scenarios, drawn next to the current run
        let compare = if style.compare { self.compare_runs() } else { Vec::new() };
        let saved: Vec<(String, crate::sim::Run)> = compare.iter().map(|c| (c.run.name.clone(), c.run.clone())).collect();
        let shows_compare = !compare.is_empty() && matches!(kind, ChartKind::Line | ChartKind::Area | ChartKind::Step | ChartKind::Bars | ChartKind::Phase);
        let single = names.len() == 1;
        // the legend says what each line is: a variable of this model, a scenario or a kept run
        // …and, for a set variable shown for one tank or region, whose it is
        let whose = |n: &str| -> String {
            if n.contains('[') {
                return String::new();
            }
            self.model
                .by_name(&crate::expr::normalize(n))
                .and_then(|v| crate::sets::focus_label(&self.model, v, &self.focus))
                .map(|l| format!(" · {l}"))
                .unwrap_or_default()
        };
        let var_label = |n: &str| if shows_compare { format!("{n}{} (current model)", whose(n)) } else { format!("{n}{}", whose(n)) };
        let cmp_label = |k: usize, n: &str| {
            let sn = &saved[k].0;
            match (compare[k].color.is_some(), single) {
                (true, _) => format!("Scenario: {sn}"),
                (false, true) => format!("Kept run: {sn}"),
                (false, false) => format!("Kept run: {sn} · {n}"),
            }
        };
        let color_of = |i: usize, n: &str| style.color_for(n).map(|[r, g, b]| Color32::from_rgb(r, g, b)).unwrap_or(pal.series(i));
        let lw = style.line_width.clamp(0.3, 4.0);
        let units = run.time_units.clone();
        let x_label = if style.x_label.is_empty() { format!("Time ({units})") } else { style.x_label.clone() };
        let y_label = if !style.y_label.is_empty() {
            style.y_label.clone()
        } else {
            // the variables' units, when they share one
            let us: Vec<String> = names.iter().filter_map(|n| self.model.by_name(&crate::expr::normalize(n)).map(|v| v.units.clone())).filter(|u| !u.is_empty()).collect();
            if !us.is_empty() && us.iter().all(|u| *u == us[0]) { us[0].clone() } else { String::new() }
        };
        let caption_h = 22.0;
        let used = ui.cursor().top() - top;
        let height = (height - used - caption_h - 4.0).max(80.0);
        // a legend taller than the chart would spill out: hide it and say so below
        let n_scen = compare.iter().filter(|c| c.color.is_some()).count();
        let n_kept = compare.len() - n_scen;
        let entries = names.len() + if shows_compare { n_scen + n_kept * if single { 1 } else { names.len() } } else { 0 };
        let legend_fits = matches!(kind, ChartKind::Donut) || (entries as f32) * 16.5 + 16.0 < height - caption_h - 30.0;
        let legend_pos = if legend_fits { style.legend } else { LegendPos::Hidden };
        let base = |plot: Plot<'static>| -> Plot<'static> {
            let mut p = plot.height(height).allow_scroll(false).show_grid(style.grid);
            p = match legend_pos {
                LegendPos::Hidden => p,
                LegendPos::TopRight => p.legend(Legend::default().position(egui_plot::Corner::RightTop)),
                LegendPos::TopLeft => p.legend(Legend::default().position(egui_plot::Corner::LeftTop)),
                LegendPos::BottomRight => p.legend(Legend::default().position(egui_plot::Corner::RightBottom)),
                LegendPos::BottomLeft => p.legend(Legend::default().position(egui_plot::Corner::LeftBottom)),
            };
            p
        };
        let (xr, yr) = ((style.x_min, style.x_max), (style.y_min, style.y_max));
        // fixed ranges the person asked for
        let fix_bounds = move |pu: &mut egui_plot::PlotUi| {
            let b = pu.plot_bounds();
            if xr.0.is_some() || xr.1.is_some() {
                pu.set_plot_bounds_x(xr.0.unwrap_or(b.min()[0])..=xr.1.unwrap_or(b.max()[0]));
            }
            if yr.0.is_some() || yr.1.is_some() {
                pu.set_plot_bounds_y(yr.0.unwrap_or(b.min()[1])..=yr.1.unwrap_or(b.max()[1]));
            }
        };
        let id = egui::Id::unique(id);
        // where a layer's horizon ends before the run does: shaded, and said so
        let bands = horizon_bands(&self.model, names, run.times.last().copied().unwrap_or(0.0));
        let (data_lo, data_hi) = names.iter().filter_map(|n| run.get(n)).flat_map(|v| v.iter().copied()).filter(|y| y.is_finite()).fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), y| (l.min(y), h.max(y)));
        let out = match kind {
            ChartKind::Line | ChartKind::Area | ChartKind::Step => {
                let plot = self.hold_axes(base(Plot::new(id)).x_axis_label(x_label).y_axis_label(y_label), &run, names);
                let r = plot.show(ui, |pu| {
                    fix_bounds(pu);
                    if data_lo.is_finite() && data_hi.is_finite() {
                        let pad = ((data_hi - data_lo) * 0.08).max(1e-9);
                        let b = pu.plot_bounds();
                        let (by0, by1) = (b.min()[1], b.max()[1]);
                        let (y0, y1) = if by0.is_finite() && by1 > by0 { (by0, by1) } else { (data_lo.min(0.0) - pad, data_hi + pad) };
                        for (x0, x1, label, deep) in &bands {
                            let a = if *deep { 0.16 } else { 0.08 };
                            pu.polygon(Polygon::new("", PlotPoints::from(vec![[*x0, y0], [*x1, y0], [*x1, y1], [*x0, y1]])).fill_color(pal.muted.gamma_multiply(a)).stroke(Stroke::NONE));
                            pu.text(Text::new("", PlotPoint::new(*x0 + (*x1 - *x0) * 0.02, y1), RichText::new(label).size(11.0).color(pal.muted)).anchor(Align2::LEFT_TOP));
                        }
                    }
                    for (i, n) in names.iter().enumerate() {
                        let c = color_of(i, n);
                        for (k, (_, sr)) in saved.iter().enumerate() {
                            if let Some(v) = sr.get(n) {
                                let pts: Vec<[f64; 2]> = sr.times.iter().zip(v).map(|(t, y)| [*t, *y]).collect();
                                let pts = if kind == ChartKind::Step { stepped(&pts) } else { pts };
                                let (col, st, w) = self.compare_style(&compare[k], c, single);
                                pu.line(Line::new(cmp_label(k, n), PlotPoints::from(pts)).color(col).style(st).width(w * lw));
                            }
                        }
                        if let Some(pts) = self.plot_pts(&run, n) {
                            self.draw_head(pu, &pts, c);
                            if style.markers {
                                let step = (pts.len() / 30).max(1);
                                pu.points(Points::new("", pts.iter().step_by(step).copied().collect::<Vec<_>>()).radius(2.8 * lw.sqrt()).color(c));
                            }
                            let shown = if kind == ChartKind::Step { stepped(&pts) } else { pts };
                            let mut l = Line::new(var_label(n), PlotPoints::from(shown)).color(c).width(2.4 * lw);
                            if kind == ChartKind::Area {
                                l = l.fill(0.0).fill_alpha(0.18);
                            }
                            pu.line(l);
                        }
                    }
                    if let Some(t) = scrub_t {
                        pu.vline(VLine::new("", t).color(pal.muted.gamma_multiply(0.5)));
                    }
                    pu.pointer_coordinate().filter(|_| pu.response().clicked()).map(|p| p.x)
                });
                self.chart_menu(&r.response, cref.clone());
                r.inner
            }
            ChartKind::Stacked => {
                // layers add up; drawn from the top layer down so each covers the one above's lower part
                let upto = self.reveal_len(run.times.len());
                let mut cum = vec![0.0; run.times.len()];
                let mut layers: Vec<(String, Vec<[f64; 2]>, Color32)> = Vec::new();
                for (i, n) in names.iter().enumerate() {
                    let Some(v) = run.get(n) else { continue };
                    for (k, y) in v.iter().enumerate() {
                        cum[k] += y;
                    }
                    let pts: Vec<[f64; 2]> = run.times.iter().zip(&cum).take(upto).map(|(t, y)| [*t, *y]).collect();
                    layers.push((n.clone(), pts, color_of(i, n)));
                }
                let top = cum.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let mut plot = base(Plot::new(id)).x_axis_label(x_label).y_axis_label(y_label).include_y(0.0);
                if self.reveal_p.is_some() && top.is_finite() {
                    plot = plot.include_y(top).include_x(run.times[0]).include_x(*run.times.last().unwrap());
                }
                let bg = pal.panel;
                let r = plot.show(ui, |pu| {
                    fix_bounds(pu);
                    for (_, pts, c) in layers.iter().rev() {
                        pu.line(Line::new("", PlotPoints::from(pts.clone())).color(blend(*c, bg, 0.7)).width(0.0).fill(0.0).fill_alpha(1.0));
                    }
                    for (n, pts, c) in layers.iter().rev() {
                        pu.line(Line::new(n.clone(), PlotPoints::from(pts.clone())).color(*c).width(1.8 * lw));
                    }
                    if let Some(t) = scrub_t {
                        pu.vline(VLine::new("", t).color(pal.muted.gamma_multiply(0.5)));
                    }
                    pu.pointer_coordinate().filter(|_| pu.response().clicked()).map(|p| p.x)
                });
                self.chart_menu(&r.response, cref.clone());
                r.inner
            }
            ChartKind::Bars => {
                let groups = 1 + saved.len();
                let w = 0.8 / groups as f64;
                let labels: Vec<String> = names.to_vec();
                let fmt_labels = labels.clone();
                let mut plot = base(Plot::new(id)).allow_drag(false).include_y(0.0).x_axis_formatter(move |m, _| {
                    let i = m.value.round();
                    if (m.value - i).abs() < 1e-6 && i >= 0.0 { fmt_labels.get(i as usize).cloned().unwrap_or_default() } else { String::new() }
                });
                if !y_label.is_empty() {
                    plot = plot.y_axis_label(y_label);
                }
                if !style.x_label.is_empty() {
                    plot = plot.x_axis_label(style.x_label.clone());
                }
                let t_now = theme::fmt_num(run.times.get(ti).copied().unwrap_or(0.0));
                let r = plot.show(ui, |pu| {
                    fix_bounds(pu);
                    let mut add = |name: String, r: &crate::sim::Run, k: usize, c: Color32| {
                        let idx = ti.min(r.times.len().saturating_sub(1));
                        let bars: Vec<Bar> = labels.iter().enumerate().filter_map(|(i, n)| r.get(n).and_then(|v| v.get(idx)).map(|y| Bar::new(i as f64 - 0.4 + w * (k as f64 + 0.5), *y).width(w * 0.9).name(n))).collect();
                        pu.bar_chart(BarChart::new(name, bars).color(c));
                    };
                    add(format!("Current model · t={t_now}"), &run, 0, color_of(0, &names[0]));
                    for (k, (sn, sr)) in saved.iter().enumerate() {
                        let name = if compare[k].color.is_some() { format!("Scenario: {sn}") } else { format!("Kept run: {sn}") };
                        add(name, sr, k + 1, compare[k].color.unwrap_or(pal.series(0).gamma_multiply(0.45)));
                    }
                });
                self.chart_menu(&r.response, cref.clone());
                None
            }
            ChartKind::Phase => {
                if names.len() < 2 {
                    ui.label(RichText::new("A phase plot needs two variables: the first goes across, the second up").color(pal.muted));
                    return None;
                }
                let (xn, yn) = (&names[0], &names[1]);
                let upto = self.reveal_len(run.times.len());
                let path = |r: &crate::sim::Run, n: usize| -> Option<Vec<[f64; 2]>> { Some(r.get(xn)?.iter().zip(r.get(yn)?).take(n).map(|(x, y)| [*x, *y]).collect()) };
                let xl = if style.x_label.is_empty() { xn.clone() } else { style.x_label.clone() };
                let yl = if style.y_label.is_empty() { yn.clone() } else { style.y_label.clone() };
                let c = color_of(1, yn);
                let r = base(Plot::new(id)).x_axis_label(xl).y_axis_label(yl).show(ui, |pu| {
                    fix_bounds(pu);
                    for (k, (sn, sr)) in saved.iter().enumerate() {
                        if let Some(p) = path(sr, usize::MAX) {
                            let name = if compare[k].color.is_some() { format!("Scenario: {sn}") } else { format!("Kept run: {sn}") };
                            pu.line(Line::new(name, PlotPoints::from(p)).color(compare[k].color.unwrap_or(pal.muted)).style(LineStyle::dashed_loose()).width(1.5 * lw));
                        }
                    }
                    if let Some(p) = path(&run, upto) {
                        if let Some(first) = p.first() {
                            pu.points(Points::new("", vec![*first]).radius(4.0).color(pal.muted));
                        }
                        if let Some(now) = path(&run, usize::MAX).and_then(|all| all.get(ti).copied()) {
                            pu.points(Points::new("", vec![now]).radius(5.0).color(c));
                        }
                        pu.line(Line::new(if shows_compare { format!("{yn} vs {xn} — current model") } else { format!("{yn} vs {xn}") }, PlotPoints::from(p)).color(c).width(2.4 * lw));
                    }
                });
                self.chart_menu(&r.response, cref.clone());
                None
            }
            ChartKind::Donut => {
                let vals: Vec<(String, f64, Color32)> = names.iter().enumerate().filter_map(|(i, n)| run.get(n).and_then(|v| v.get(ti)).map(|y| (n.clone(), y.abs(), color_of(i, n)))).collect();
                let total: f64 = vals.iter().map(|v| v.1).sum();
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
                self.chart_menu(&resp, cref.clone());
                let p = ui.painter_at(rect);
                let radius = (rect.height() * 0.46).min(rect.width() * 0.26);
                let c = pos2(rect.left() + radius + 10.0, rect.center().y);
                if total <= 0.0 {
                    p.text(c, Align2::CENTER_CENTER, "nothing to show", FontId::proportional(12.0), pal.muted);
                    return None;
                }
                let mut a0 = -std::f32::consts::FRAC_PI_2;
                for (_, v, col) in &vals {
                    let sweep = (*v / total) as f32 * std::f32::consts::TAU;
                    ring(&p, c, radius, radius * 0.6, a0, a0 + sweep, *col, pal.panel);
                    a0 += sweep;
                }
                p.text(c - vec2(0.0, 7.0), Align2::CENTER_CENTER, theme::fmt_num(total), FontId::proportional(16.0), pal.text);
                p.text(c + vec2(0.0, 11.0), Align2::CENTER_CENTER, "total", FontId::proportional(10.5), pal.muted);
                if style.legend != LegendPos::Hidden {
                    let mut y = rect.center().y - vals.len() as f32 * 11.0;
                    let lx = c.x + radius + 18.0;
                    for (n, v, col) in &vals {
                        p.circle_filled(pos2(lx + 5.0, y + 7.0), 5.0, *col);
                        p.text(pos2(lx + 16.0, y), Align2::LEFT_TOP, n, FontId::proportional(12.0), pal.text);
                        p.text(pos2(rect.right() - 4.0, y), Align2::RIGHT_TOP, format!("{:.0}%", v / total * 100.0), FontId::monospace(11.5), pal.muted);
                        y += 22.0;
                    }
                }
                None
            }
        };
        if shows_compare || !legend_fits {
            // spell out what the line styles mean
            let scen = compare.iter().any(|c| c.color.is_some());
            let kept = compare.iter().any(|c| c.color.is_none());
            let mut parts = vec![if kind == ChartKind::Bars { "First bar: the current model".to_string() } else { "Solid: variables in the current model".to_string() }];
            if scen {
                parts.push(if single || kind == ChartKind::Bars { "coloured: scenarios".into() } else { "dashed in a scenario's colour: that scenario".into() });
            }
            if kept {
                parts.push("faint dashed: kept runs".into());
            }
            if !legend_fits && style.legend != LegendPos::Hidden {
                parts.push("legend hidden — make the chart taller to see it".into());
            }
            ui.label(RichText::new(parts.join("  ·  ")).size(11.5).color(pal.muted));
        }
        // fill exactly the space given: panels shrink to their contents, so leaving a gap
        // would make the chart smaller every frame
        let rest = top + budget - ui.cursor().top();
        if rest > 0.0 {
            ui.add_space(rest);
        }
        out
    }
}

/// Duplicate points so a line holds each value until the next time.
fn stepped(pts: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out = Vec::with_capacity(pts.len() * 2);
    for w in pts.windows(2) {
        out.push(w[0]);
        out.push([w[1][0], w[0][1]]);
    }
    if let Some(l) = pts.last() {
        out.push(*l);
    }
    out
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 * (1.0 - t) + y as f32 * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// A ring segment from angle `a0` to `a1`.
fn ring(p: &egui::Painter, c: Pos2, r_out: f32, r_in: f32, a0: f32, a1: f32, col: Color32, bg: Color32) {
    let n = (((a1 - a0).abs() / 0.05).ceil() as usize).max(2);
    let mut mesh = egui::Mesh::default();
    for i in 0..=n {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        let d = vec2(a.cos(), a.sin());
        mesh.colored_vertex(c + d * r_out, col);
        mesh.colored_vertex(c + d * r_in, col);
        if i > 0 {
            let k = (i * 2) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k, k + 1);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    // the thin cut between slices takes the colour of what is behind the ring
    let gap = Stroke::new(2.0, if bg == Color32::TRANSPARENT { Color32::from_white_alpha(200) } else { bg });
    let d0 = vec2(a0.cos(), a0.sin());
    p.line_segment([c + d0 * r_in, c + d0 * r_out], gap);
}

/// The stretches of the run beyond a layer's horizon (labelled "ranges only") and beyond its
/// structural limit ("Illustrative structure, not a forecast") for the layers the variables are in:
/// (from, to, label, beyond the structural limit).
pub fn horizon_bands(m: &crate::model::Model, names: &[String], end: f64) -> Vec<(f64, f64, String, bool)> {
    let mut out: Vec<(f64, f64, String, bool)> = Vec::new();
    let mut layers: Vec<&dynamist_core::layers::Layer> = m.layers.iter().collect();
    layers.sort_by_key(|l| (l.order, l.id));
    for l in layers {
        let owns = names.iter().any(|n| m.by_name(&crate::expr::normalize(n)).is_some_and(|v| dynamist_core::layers::layer_of(m, v) == Some(l.id)));
        if !owns {
            continue;
        }
        let (trusted, structural) = dynamist_core::layers::horizon_times(m, l);
        let Some(t) = trusted.filter(|t| *t < end) else { continue };
        let s = structural.filter(|s| *s < end && *s >= t);
        let (to, deep) = (s.unwrap_or(end), false);
        // layers that stop being trusted at the same moment share one label
        match out.iter_mut().find(|b| b.0 == t && b.1 == to && !b.3) {
            Some(b) => b.2 = b.2.replace(": ranges only", &format!(", {}: ranges only", l.name)),
            None => out.push((t, to, format!("{}: ranges only", l.name), deep)),
        }
        if let Some(s) = s {
            if !out.iter().any(|b| b.3 && b.0 == s) {
                out.push((s, end, "Illustrative structure, not a forecast".into(), true));
            }
        }
    }
    out
}
