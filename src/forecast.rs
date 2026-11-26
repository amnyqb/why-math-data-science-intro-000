//! Does the model beat a simple guess? Pick a result variable, a data series for it and a split
//! time. Everything after the split is held out: the model's run is scored on those points
//! against guesses that only see the data up to the split — the last value ("nothing changes"),
//! the same time one season earlier, the straight-line trend, and exponential smoothing with a
//! damped trend (the usual statistical default). A model that cannot
//! beat these is not adding anything over a spreadsheet.
//!
//! The model is scored as it stands (its levers as set, or fitted with Optimize). It is not
//! restarted from the data at the split, so this is the harder, honest test.

use egui::RichText;
use egui_plot::{Line, Plot, PlotPoints, Points, VLine};

use crate::app::App;
use crate::model::Kind;

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub method: &'static str,
    pub rmse: f64,
    pub mae: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Point {
    pub t: f64,
    pub actual: f64,
    pub model: f64,
    pub naive: f64,
    pub seasonal: Option<f64>,
    pub trend: f64,
    pub smoothed: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct Comparison {
    pub n_train: usize,
    pub rows: Vec<Row>,
    pub points: Vec<Point>,
    /// How well the model matches the data it was allowed to see.
    pub train_rmse: f64,
    pub verdict: String,
    pub beats_all: bool,
    /// Data the guesses may see, and the model's whole run, for the picture.
    pub history: Vec<[f64; 2]>,
    pub curve: Vec<[f64; 2]>,
}

#[derive(Default)]
pub struct State {
    pub var: String,
    pub actual: String,
    pub split: f64,
    pub season: f64,
}

/// Linear interpolation of points sorted by time; None outside their range.
fn at(pts: &[[f64; 2]], t: f64) -> Option<f64> {
    let (first, last) = (pts.first()?, pts.last()?);
    if t < first[0] - 1e-9 || t > last[0] + 1e-9 {
        return None;
    }
    let i = pts.partition_point(|p| p[0] <= t);
    if i == 0 {
        return Some(first[1]);
    }
    if i >= pts.len() {
        return Some(last[1]);
    }
    let (a, b) = (pts[i - 1], pts[i]);
    Some(if (b[0] - a[0]).abs() < 1e-12 { a[1] } else { a[1] + (b[1] - a[1]) * (t - a[0]) / (b[0] - a[0]) })
}

/// Exponential smoothing with a damped trend (Holt), fitted by a small grid search on one-step
/// errors over the data we may see. Returns a function: forecast `h` steps after the last point.
fn damped_trend(y: &[f64]) -> Option<(f64, f64, f64)> {
    if y.len() < 4 {
        return None;
    }
    let mut best: Option<(f64, (f64, f64, f64, f64, f64))> = None; // (sse, (level, trend, phi, ..))
    for a in [0.1, 0.2, 0.3, 0.5, 0.7, 0.9] {
        for b in [0.05, 0.1, 0.2, 0.4] {
            for phi in [0.8, 0.9, 0.95, 0.98] {
                let (mut l, mut t, mut sse) = (y[0], y[1] - y[0], 0.0);
                for v in &y[1..] {
                    let pred = l + phi * t;
                    sse += (v - pred) * (v - pred);
                    let nl = a * v + (1.0 - a) * pred;
                    t = b * (nl - l) + (1.0 - b) * phi * t;
                    l = nl;
                }
                if best.as_ref().map_or(true, |(e, _)| sse < *e) {
                    best = Some((sse, (l, t, phi, a, b)));
                }
            }
        }
    }
    best.map(|(_, (l, t, phi, _, _))| (l, t, phi))
}

fn score(method: &'static str, pairs: impl Iterator<Item = (f64, f64)>) -> Row {
    let (mut se, mut ae, mut n) = (0.0, 0.0, 0.0);
    for (a, f) in pairs {
        se += (a - f) * (a - f);
        ae += (a - f).abs();
        n += 1.0;
    }
    Row { method, rmse: (se / n).sqrt(), mae: ae / n }
}

/// Score `model` (times, values) and three guesses against `actual` after `split`.
pub fn compare(times: &[f64], model: &[f64], actual: &[[f64; 2]], split: f64, season: f64) -> Result<Comparison, String> {
    let mut data: Vec<[f64; 2]> = actual.iter().copied().filter(|p| p[0].is_finite() && p[1].is_finite()).collect();
    data.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let train: Vec<[f64; 2]> = data.iter().copied().filter(|p| p[0] <= split + 1e-9).collect();
    if train.len() < 2 {
        return Err(format!("The data has {} point(s) up to the split; the guesses need at least 2", train.len()));
    }
    let series: Vec<[f64; 2]> = times.iter().copied().zip(model.iter().copied()).map(|(t, y)| [t, y]).collect();
    let test: Vec<[f64; 2]> = data.iter().copied().filter(|p| p[0] > split + 1e-9 && at(&series, p[0]).is_some()).collect();
    if test.is_empty() {
        return Err("No data points after the split fall inside the run, so there is nothing to score".into());
    }
    let (first, last) = (train[0], *train.last().unwrap());
    let slope = (last[1] - first[1]) / (last[0] - first[0]).max(1e-12);
    // the data is treated as evenly spaced, one step being its typical gap
    let mut gaps: Vec<f64> = train.windows(2).map(|w| w[1][0] - w[0][0]).collect();
    gaps.sort_by(|a, b| a.total_cmp(b));
    let step = gaps[gaps.len() / 2].max(1e-12);
    let smooth = damped_trend(&train.iter().map(|p| p[1]).collect::<Vec<_>>());
    let mut points = Vec::new();
    for p in &test {
        let seasonal = if season > 0.0 {
            // the same time one or more seasons earlier that still lies in the data we may see
            let k = ((p[0] - split) / season).ceil().max(1.0);
            at(&train, p[0] - k * season)
        } else {
            None
        };
        points.push(Point { t: p[0], actual: p[1], model: at(&series, p[0]).unwrap(), naive: last[1], seasonal, trend: last[1] + slope * (p[0] - last[0]),
            smoothed: smooth.map(|(l, t, phi)| {
                let h = ((p[0] - last[0]) / step).max(0.0);
                l + t * phi * (1.0 - phi.powf(h)) / (1.0 - phi)
            }),
        });
    }
    let mut rows = vec![
        score("Model", points.iter().map(|q| (q.actual, q.model))),
        score("Last value", points.iter().map(|q| (q.actual, q.naive))),
        score("Straight-line trend", points.iter().map(|q| (q.actual, q.trend))),
    ];
    if points.iter().all(|q| q.smoothed.is_some()) {
        rows.push(score("Smoothed trend", points.iter().map(|q| (q.actual, q.smoothed.unwrap()))));
    }
    if season > 0.0 && points.iter().all(|q| q.seasonal.is_some()) {
        rows.push(score("Same time last season", points.iter().map(|q| (q.actual, q.seasonal.unwrap()))));
    }
    let best_guess = rows[1..].iter().min_by(|a, b| a.rmse.total_cmp(&b.rmse)).unwrap().clone();
    let beats_all = rows[0].rmse < best_guess.rmse;
    let verdict = if beats_all {
        format!("The model is closer than every guess: RMSE {} against {} for the best of them ({}).", crate::theme::fmt_num(rows[0].rmse), crate::theme::fmt_num(best_guess.rmse), best_guess.method)
    } else {
        format!("“{}” is closer than the model (RMSE {} against {}): on this data the model adds nothing over that guess.", best_guess.method, crate::theme::fmt_num(best_guess.rmse), crate::theme::fmt_num(rows[0].rmse))
    };
    let train_rmse = score("", train.iter().filter_map(|p| at(&series, p[0]).map(|m| (p[1], m)))).rmse;
    Ok(Comparison { n_train: train.len(), rows, points, train_rmse, verdict, beats_all, history: train, curve: series })
}

impl App {
    /// The "Compare with data" window.
    pub fn compare_window(&mut self, ctx: &egui::Context) {
        if !self.compare_open {
            return;
        }
        let pal = self.pal;
        let mut open = true;
        let data: Vec<String> = self.model.vars.iter().filter(|v| v.kind == Kind::Lookup && !v.table.is_empty()).map(|v| v.name.clone()).collect();
        let results: Vec<String> = self.model.vars.iter().filter(|v| !(v.kind == Kind::Lookup && v.equation.trim().is_empty())).map(|v| v.name.clone()).collect();
        if self.compare.actual.is_empty() {
            self.compare.actual = data.first().cloned().unwrap_or_default();
        }
        if self.compare.var.is_empty() {
            self.compare.var = results.iter().find(|n| self.model.by_name(&crate::expr::normalize(n)).is_some_and(|v| v.kind == Kind::Stock)).or(results.first()).cloned().unwrap_or_default();
        }
        // the data's own span decides a sensible first split: two thirds in
        let span = self.model.by_name(&crate::expr::normalize(&self.compare.actual)).map(|v| (v.table.first().map(|p| p[0]).unwrap_or(0.0), v.table.last().map(|p| p[0]).unwrap_or(1.0)));
        if self.compare.split == 0.0 {
            if let Some((a, b)) = span {
                self.compare.split = a + (b - a) * 2.0 / 3.0;
            }
        }
        let result = (|| -> Result<Comparison, String> {
            let run = self.run.as_ref().ok_or("Run the model first")?;
            let series = run.get(&self.compare.var).ok_or_else(|| format!("“{}” has no results", self.compare.var))?;
            let actual = self.model.by_name(&crate::expr::normalize(&self.compare.actual)).map(|v| v.table.clone()).ok_or("Choose a data series")?;
            compare(&run.times, series, &actual, self.compare.split, self.compare.season)
        })();
        egui::Window::new("Compare with data").open(&mut open).default_width(640.0).default_height(560.0).show(ctx, |ui| {
            ui.label(RichText::new("Does the model beat a simple guess? Data after the split is held out: the model is scored on it against guesses that only see the data before it.").small().color(pal.muted));
            if data.is_empty() {
                ui.add_space(6.0);
                ui.label("This model has no data series. Add a TimeSeries sheet (Time, then one column per series) when importing, or a lookup over Time.");
                return;
            }
            ui.add_space(6.0);
            egui::Grid::new("compare-form").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Model result");
                egui::ComboBox::from_id_salt("cmp-var").width(260.0).selected_text(&self.compare.var).show_ui(ui, |ui| {
                    for n in &results {
                        ui.selectable_value(&mut self.compare.var, n.clone(), n);
                    }
                });
                ui.end_row();
                ui.label("Data");
                egui::ComboBox::from_id_salt("cmp-data").width(260.0).selected_text(&self.compare.actual).show_ui(ui, |ui| {
                    for n in &data {
                        ui.selectable_value(&mut self.compare.actual, n.clone(), n);
                    }
                });
                ui.end_row();
                ui.label("Hold out data after");
                ui.add(egui::DragValue::new(&mut self.compare.split).speed(0.5));
                ui.end_row();
                ui.label("A season lasts");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.compare.season).speed(0.5).range(0.0..=1e6));
                    ui.label(RichText::new("0 = no seasonal guess").small().color(pal.muted));
                });
                ui.end_row();
            });
            ui.add_space(8.0);
            match &result {
                Err(e) => {
                    ui.label(RichText::new(e).color(pal.error));
                }
                Ok(c) => {
                    ui.label(RichText::new(&c.verdict).strong().color(if c.beats_all { pal.ok } else { pal.error }));
                    ui.add_space(4.0);
                    egui::Grid::new("compare-rows").num_columns(3).striped(true).spacing([18.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new("On the held-out data").small().color(pal.muted));
                        ui.label(RichText::new("RMSE").small().color(pal.muted));
                        ui.label(RichText::new("Average miss").small().color(pal.muted));
                        ui.end_row();
                        for r in &c.rows {
                            ui.label(if r.method == "Model" { RichText::new(r.method).strong() } else { RichText::new(r.method) });
                            ui.label(RichText::new(crate::theme::fmt_num(r.rmse)).monospace());
                            ui.label(RichText::new(crate::theme::fmt_num(r.mae)).monospace());
                            ui.end_row();
                        }
                    });
                    ui.label(RichText::new(format!("{} data point(s) before the split, {} after. The model matches the points it could see to within RMSE {}. It is not restarted from the data at the split.", c.n_train, c.points.len(), crate::theme::fmt_num(c.train_rmse))).small().color(pal.muted));
                    ui.add_space(6.0);
                    let line = |f: &dyn Fn(&Point) -> Option<f64>| -> PlotPoints<'static> { c.points.iter().filter_map(|p| f(p).map(|y| [p.t, y])).collect() };
                    Plot::new("compare-plot").height(220.0).legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop)).show(ui, |pl| {
                        pl.vline(VLine::new("Split", self.compare.split).color(pal.muted));
                        pl.line(Line::new("Model", PlotPoints::from_iter(c.curve.iter().copied())).color(pal.accent).width(2.0));
                        pl.points(Points::new("Data seen by the guesses", PlotPoints::from_iter(c.history.iter().copied())).color(pal.muted).radius(3.0));
                        pl.line(Line::new("Last value", line(&|p| Some(p.naive))).color(pal.muted).style(egui_plot::LineStyle::dashed_loose()));
                        pl.line(Line::new("Straight-line trend", line(&|p| Some(p.trend))).color(pal.aux).style(egui_plot::LineStyle::dashed_loose()));
                        if c.points.iter().all(|p| p.smoothed.is_some()) {
                            pl.line(Line::new("Smoothed trend", line(&|p| p.smoothed)).color(pal.error).style(egui_plot::LineStyle::dashed_loose()));
                        }
                        if c.points.iter().all(|p| p.seasonal.is_some()) && self.compare.season > 0.0 {
                            pl.line(Line::new("Same time last season", line(&|p| p.seasonal)).color(pal.ok).style(egui_plot::LineStyle::dashed_loose()));
                        }
                        pl.points(Points::new("Held-out data", line(&|p| Some(p.actual))).color(pal.text).radius(3.5));
                    });
                }
            }
        });
        if !open {
            self.compare_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(f: impl Fn(f64) -> f64) -> (Vec<f64>, Vec<f64>) {
        let t: Vec<f64> = (0..=40).map(|i| i as f64).collect();
        let y = t.iter().map(|x| f(*x)).collect();
        (t, y)
    }

    #[test]
    fn a_model_that_matches_the_data_beats_every_guess() {
        // growth that bends upward: a straight line and "no change" both fall behind
        let (t, y) = grid(|x| 10.0 * (0.05f64 * x).exp());
        let actual: Vec<[f64; 2]> = t.iter().zip(&y).step_by(2).map(|(a, b)| [*a, *b]).collect();
        let c = compare(&t, &y, &actual, 24.0, 0.0).unwrap();
        assert!(c.beats_all, "{:?}", c.rows);
        assert!(c.rows[0].rmse < 1e-9);
        assert_eq!(c.rows.len(), 4);
    }

    #[test]
    fn smoothing_follows_a_steady_trend_and_stays_flat_on_flat_data() {
        let (t, flat) = grid(|_| 50.0);
        let actual: Vec<[f64; 2]> = t.iter().step_by(2).map(|x| [*x, 50.0]).collect();
        let c = compare(&t, &flat, &actual, 24.0, 0.0).unwrap();
        let s = c.rows.iter().find(|r| r.method == "Smoothed trend").expect("smoothing row");
        assert!(s.rmse < 1e-9, "{}", s.rmse);
        // on a rising line it beats "no change" by a wide margin
        let actual: Vec<[f64; 2]> = t.iter().step_by(2).map(|x| [*x, 10.0 + 2.0 * x]).collect();
        let c = compare(&t, &flat, &actual, 24.0, 0.0).unwrap();
        let last = c.rows.iter().find(|r| r.method == "Last value").unwrap().rmse;
        let smooth = c.rows.iter().find(|r| r.method == "Smoothed trend").unwrap().rmse;
        assert!(smooth < last / 2.0, "smoothed {smooth} vs last value {last}");
    }

    #[test]
    fn a_flat_model_loses_to_the_trend_on_trending_data() {
        let (t, flat) = grid(|_| 50.0);
        let actual: Vec<[f64; 2]> = t.iter().step_by(2).map(|x| [*x, 10.0 + 2.0 * x]).collect();
        let c = compare(&t, &flat, &actual, 24.0, 0.0).unwrap();
        assert!(!c.beats_all);
        assert!(c.verdict.contains("Straight-line trend"), "{}", c.verdict);
        let trend = c.rows.iter().find(|r| r.method == "Straight-line trend").unwrap();
        assert!(trend.rmse < 1e-9, "a straight line is predicted exactly by its trend");
    }

    #[test]
    fn the_seasonal_guess_needs_a_full_season_of_history() {
        let (t, y) = grid(|x| 100.0 + 20.0 * (std::f64::consts::TAU * x / 10.0).sin());
        let actual: Vec<[f64; 2]> = t.iter().zip(&y).map(|(a, b)| [*a, *b]).collect();
        let c = compare(&t, &y, &actual, 20.0, 10.0).unwrap();
        let s = c.rows.iter().find(|r| r.method == "Same time last season").expect("seasonal row");
        assert!(s.rmse < 1e-6, "a pure cycle repeats exactly: {}", s.rmse);
        // a season longer than the history: no seasonal row rather than a made-up one
        let c = compare(&t, &y, &actual, 20.0, 50.0).unwrap();
        assert!(c.rows.iter().all(|r| r.method != "Same time last season"));
    }

    #[test]
    fn it_explains_why_it_cannot_score() {
        let (t, y) = grid(|x| x);
        assert!(compare(&t, &y, &[[0.0, 0.0]], 5.0, 0.0).unwrap_err().contains("at least 2"));
        assert!(compare(&t, &y, &[[0.0, 0.0], [1.0, 1.0]], 5.0, 0.0).unwrap_err().contains("nothing to score"));
    }
}
