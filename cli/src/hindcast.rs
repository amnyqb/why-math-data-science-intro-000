//! `dyn hindcast FILE --data CSV --cut DATE [--fit]`: cut the data at a past date, fit only on what came before, run forward,
//! and score against what followed and against simple guesses (the R21 measures: last value, straight-line trend, smoothed
//! trend, same time last season). The share of actual points inside the 50% and 90% bands of a `sens` ensemble is reported
//! next to the nominal shares.
//!
//! The fit uses nothing after the cut: the data handed to the fit is filtered at the cut before anything else happens.
//! The bands are those of `dyn sens`: they come from the spread of the levers only, not from noise in the data, so a
//! well-behaved model can still be outside them more often than nominal. The report says so with the numbers.

use dynamist_core::calendar::{self, CalCtx};
use dynamist_core::model::{Kind, Lever, Measure, Model, OptSpec};
use dynamist_core::ops::{Op, Session};
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Debug, Serialize)]
pub struct Coverage {
    pub nominal: f64,
    pub observed: f64,
    pub inside: usize,
    pub points: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Score {
    pub method: String,
    pub rmse: f64,
    pub mae: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub variable: String,
    pub cut: f64,
    pub train_points: usize,
    pub test_points: usize,
    pub fitted: Vec<(String, f64)>,
    pub started_from: Vec<(String, f64)>,
    pub scores: Vec<Score>,
    pub verdict: String,
    pub beats_all: bool,
    pub train_rmse: f64,
    pub coverage: Vec<Coverage>,
    pub runs: usize,
    pub spread: f64,
    pub seed: u64,
    pub notes: Vec<String>,
}

pub struct Spec {
    pub var: Option<String>,
    pub cut: String,
    pub fit: bool,
    pub levers: Vec<Lever>,
    pub runs: usize,
    pub spread: f64,
    pub seed: u64,
    pub season: f64,
    pub effort: u8,
}

fn day_of(s: calendar::Stamp) -> f64 {
    s.days as f64 + s.secs / 86_400.0
}

/// A time written as a number of model time, or as an ISO date, in the model's time.
pub fn parse_time(m: &Model, s: &str) -> Result<f64, String> {
    let s = s.trim();
    if let Ok(x) = s.parse::<f64>() {
        return Ok(x);
    }
    let stamp = calendar::parse_iso(s).ok_or_else(|| format!("“{s}” is neither a number nor a date like 2026-11-22"))?;
    let cal = CalCtx::of(m).ok_or("a date needs the model to have a start date (the setting “start”); write the time as a number of model time instead")?;
    let target = day_of(stamp);
    let (t0, t1) = (m.settings.initial_time, m.settings.final_time);
    let at = |t: f64| day_of(cal.stamp_at(t, t0));
    if target < at(t0) - 1e-9 || target > at(t1) + 1e-9 {
        return Err(format!("{s} lies outside the model's run ({} to {})", fmt_date(&cal, t0, t0), fmt_date(&cal, t1, t0)));
    }
    let (mut lo, mut hi) = (t0, t1);
    for _ in 0..80 {
        let mid = (lo + hi) / 2.0;
        if at(mid) < target { lo = mid } else { hi = mid }
    }
    Ok((lo + hi) / 2.0)
}

fn fmt_date(cal: &CalCtx, t: f64, t0: f64) -> String {
    let (y, m, d) = calendar::civil_from_days(cal.stamp_at(t, t0).days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The two columns of a CSV: a time (number or date) and a value. Returns the name of the value column and the points,
/// in model time, sorted. A first line that is not data is the header.
pub fn read_csv(text: &str, m: &Model) -> Result<(String, Vec<[f64; 2]>), String> {
    let mut name = String::new();
    let mut pts = Vec::new();
    for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
        let cells: Vec<&str> = line.split([',', ';', '\t']).map(|c| c.trim().trim_matches('"')).collect();
        if cells.len() < 2 {
            return Err(format!("line {}: the data needs two columns, time and value", i + 1));
        }
        match (parse_time(m, cells[0]), cells[1].parse::<f64>()) {
            (Ok(t), Ok(v)) => pts.push([t, v]),
            _ if pts.is_empty() && name.is_empty() => name = cells[1].to_string(),
            (Err(e), _) => return Err(format!("line {}: {e}", i + 1)),
            (_, Err(_)) => return Err(format!("line {}: “{}” is not a number", i + 1, cells[1])),
        }
    }
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    if pts.len() < 4 {
        return Err(format!("the data has {} point(s); a hindcast needs at least 4", pts.len()));
    }
    Ok((name, pts))
}

/// Fit the levers on the points up to the cut, through `optimize`. Nothing after the cut is read: the filter is the first line.
pub fn fit(m: &Model, var: &str, data: &[[f64; 2]], cut: f64, levers: &[Lever], effort: u8) -> Result<Vec<(String, f64)>, String> {
    let train: Vec<[f64; 2]> = data.iter().copied().filter(|p| p[0] <= cut + 1e-9).collect();
    if train.len() < 2 {
        return Err("fewer than 2 data points before the cut: there is nothing to fit on".into());
    }
    let table = train.iter().map(|p| format!("({},{})", p[0], p[1])).collect::<Vec<_>>().join(",");
    let (first, name) = (train[0][0], "Hindcast fit");
    let ops: Vec<Op> = serde_json::from_value(json!([
        {"op": "add_var", "name": format!("{name} error"), "kind": "stock", "equation": "0"},
        {"op": "add_var", "name": format!("{name} squared gap"), "kind": "aux", "equation": format!("IF THEN ELSE(AND(Time >= {first}, Time <= {cut}), ({var} - WITH LOOKUP(Time, ({table})))^2, 0)")},
        {"op": "add_var", "name": format!("{name} error rate"), "kind": "flow", "equation": format!("{name} squared gap"), "to": format!("{name} error")},
    ]))
    .map_err(|e| e.to_string())?;
    let mut s = Session::new(m.clone());
    s.gate = false;
    s.apply("hindcast", &ops).map_err(|r| format!("could not set up the fit: {r:?}"))?;
    let spec = OptSpec { maximize: false, goal: format!("{name} error"), measure: Measure::Final, levers: levers.to_vec(), limits: Vec::new(), effort };
    let p = dynamist_core::optimize::run_blocking(s.model, spec)?;
    Ok(p.names.into_iter().zip(p.best_x).collect())
}

fn interp(times: &[f64], v: &[f64], t: f64) -> Option<f64> {
    if times.is_empty() || t < times[0] - 1e-9 || t > times[times.len() - 1] + 1e-9 {
        return None;
    }
    let i = times.partition_point(|x| *x <= t).clamp(1, times.len() - 1);
    let (a, b) = (times[i - 1], times[i]);
    Some(if (b - a).abs() < 1e-12 { v[i - 1] } else { v[i - 1] + (v[i] - v[i - 1]) * (t - a) / (b - a) })
}

/// The share of `actual` points inside the band [lo, hi] (given at `times`).
pub fn coverage(times: &[f64], lo: &[f64], hi: &[f64], actual: &[[f64; 2]], nominal: f64) -> Coverage {
    let (mut inside, mut points) = (0, 0);
    for p in actual {
        if let (Some(l), Some(h)) = (interp(times, lo, p[0]), interp(times, hi, p[0])) {
            points += 1;
            if p[1] >= l - 1e-12 && p[1] <= h + 1e-12 {
                inside += 1;
            }
        }
    }
    Coverage { nominal, observed: if points == 0 { f64::NAN } else { inside as f64 / points as f64 }, inside, points }
}

pub fn run(m: &Model, data_text: &str, spec: &Spec) -> Result<(Report, String), String> {
    let (header, data) = read_csv(data_text, m)?;
    let var = spec.var.clone().or(Some(header).filter(|h| !h.is_empty())).ok_or("the variable to compare is not named: give --var NAME or a header in the data")?;
    if m.by_name(&dynamist_core::expr::normalize(&var)).is_none() {
        return Err(format!("the model has no variable “{var}”"));
    }
    let cut = parse_time(m, &spec.cut)?;
    let mut notes = Vec::new();
    let mut model = m.clone();
    let (mut fitted, mut started) = (Vec::new(), Vec::new());
    if spec.fit {
        let levers: Vec<Lever> = if !spec.levers.is_empty() {
            spec.levers.clone()
        } else {
            m.manifest.levers.iter().filter(|l| m.by_name(&dynamist_core::expr::normalize(&l.name)).is_some_and(|v| v.kind == Kind::Constant)).map(|l| Lever { name: l.name.clone(), on: true, min: l.min, max: l.max }).collect()
        };
        if levers.is_empty() {
            return Err("--fit needs levers: give --lever Name:min:max, or declare levers with ranges in the manifest".into());
        }
        started = levers.iter().map(|l| (l.name.clone(), m.by_name(&dynamist_core::expr::normalize(&l.name)).and_then(|v| v.equation.trim().parse::<f64>().ok()).unwrap_or(f64::NAN))).collect();
        fitted = fit(m, &var, &data, cut, &levers, spec.effort)?;
        for (n, v) in &fitted {
            model.apply_change(n, &format!("{v}"));
        }
        notes.push(format!("the levers were fitted on the {} points up to the cut, through optimize (effort {})", data.iter().filter(|p| p[0] <= cut + 1e-9).count(), spec.effort));
    } else {
        notes.push("the model is scored as it stands; no lever was fitted (use --fit)".into());
    }
    let run = dynamist_core::runner::run_with(&model, &[])?;
    let series = run.get(&var).ok_or_else(|| format!("no results for “{var}”"))?.clone();
    let inside: Vec<[f64; 2]> = data.iter().copied().filter(|p| p[0] >= run.times[0] - 1e-9 && p[0] <= run.times[run.times.len() - 1] + 1e-9).collect();
    if inside.len() < data.len() {
        notes.push(format!("{} data point(s) lie outside the model's run and were left out", data.len() - inside.len()));
    }
    let cmp = dynamist::forecast::compare(&run.times, &series, &inside, cut, spec.season)?;
    let sens = dynamist_core::sensitivity::run_sensitivity(&model, &var, spec.runs, spec.spread, spec.seed, 4, &std::sync::atomic::AtomicUsize::new(0))?;
    let after: Vec<[f64; 2]> = inside.iter().copied().filter(|p| p[0] > cut + 1e-9).collect();
    let coverage = vec![coverage(&sens.times, &sens.p25, &sens.p75, &after, 0.5), coverage(&sens.times, &sens.p5, &sens.p95, &after, 0.9)];
    notes.push(format!("the bands are the 25th to 75th and the 5th to 95th percentile of {} runs of dyn sens with every lever drawn within ±{:.0}% (seed {}); they hold lever uncertainty only, not noise in the data", spec.runs, spec.spread * 100.0, spec.seed));
    let svg = dynamist_core::plotsvg::fan(
        &format!("Hindcast of {var}, cut at {}", dynamist_core::doctor::num(cut)),
        "",
        &sens.times,
        &[dynamist_core::plotsvg::Band { share: 0.9, lo: sens.p5.clone(), hi: sens.p95.clone() }, dynamist_core::plotsvg::Band { share: 0.5, lo: sens.p25.clone(), hi: sens.p75.clone() }],
        &sens.p50,
        &inside.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>(),
        cut,
    );
    let report = Report {
        variable: var,
        cut,
        train_points: cmp.n_train,
        test_points: cmp.points.len(),
        fitted,
        started_from: started,
        scores: cmp.rows.iter().map(|r| Score { method: r.method.to_string(), rmse: r.rmse, mae: r.mae }).collect(),
        verdict: cmp.verdict.clone(),
        beats_all: cmp.beats_all,
        train_rmse: cmp.train_rmse,
        coverage,
        runs: spec.runs,
        spread: spec.spread,
        seed: spec.seed,
        notes,
    };
    Ok((report, svg))
}

pub fn markdown(r: &Report) -> String {
    let n = dynamist_core::doctor::num;
    let mut s = format!("# Hindcast of {}\n\nCut at time {}. {} points before the cut, {} after it.\n\n", r.variable, n(r.cut), r.train_points, r.test_points);
    s.push_str(&format!("{}\n\n", r.verdict));
    if !r.fitted.is_empty() {
        s.push_str("Levers fitted on the data before the cut:\n\n| Lever | Before | After the fit |\n|---|---|---|\n");
        for ((name, v), (_, b)) in r.fitted.iter().zip(&r.started_from) {
            s.push_str(&format!("| {name} | {} | {} |\n", n(*b), n(*v)));
        }
        s.push('\n');
    }
    s.push_str(&format!("Error on the data it saw (before the cut): RMSE {}.\n\n## Scores after the cut\n\n| Method | RMSE | MAE |\n|---|---|---|\n", n(r.train_rmse)));
    for x in &r.scores {
        s.push_str(&format!("| {} | {} | {} |\n", x.method, n(x.rmse), n(x.mae)));
    }
    s.push_str("\n## How often the actual points fell inside the bands\n\n| Band | Nominal share | Observed share | Points |\n|---|---|---|---|\n");
    for c in &r.coverage {
        s.push_str(&format!("| {:.0}% | {:.0}% | {} | {} of {} |\n", c.nominal * 100.0, c.nominal * 100.0, if c.observed.is_nan() { "n/a".into() } else { format!("{:.0}%", c.observed * 100.0) }, c.inside, c.points));
    }
    s.push_str("\n## Notes\n\n");
    for t in &r.notes {
        s.push_str(&format!("- {t}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use dynamist_core::stats::Rng;

    fn ramp(rate: &str) -> Model {
        let mut s = Session::new(Model::default());
        s.gate = false;
        let ops: Vec<Op> = serde_json::from_value(json!([
            {"op": "add_var", "name": "Tank", "kind": "stock", "equation": "0"},
            {"op": "add_var", "name": "Rate", "kind": "constant", "equation": rate},
            {"op": "add_var", "name": "Fill", "kind": "flow", "equation": "Rate", "to": "Tank"}]))
        .unwrap();
        s.apply("t", &ops).unwrap();
        let mut m = s.model;
        m.settings.initial_time = 0.0;
        m.settings.final_time = 20.0;
        m.settings.time_step = 1.0;
        m
    }

    fn lever() -> Vec<Lever> {
        vec![Lever { name: "Rate".into(), on: true, min: 0.0, max: 10.0 }]
    }

    #[test]
    fn changing_any_data_after_the_cut_leaves_the_fitted_levers_unchanged() {
        let m = ramp("1");
        let truth: Vec<[f64; 2]> = (0..=20).map(|t| [t as f64, 3.0 * t as f64]).collect();
        let a = fit(&m, "Tank", &truth, 10.0, &lever(), 2).unwrap();
        // every point after the cut is changed, and a wild one added
        let mut other = truth.clone();
        for p in other.iter_mut().filter(|p| p[0] > 10.0) {
            p[1] = p[1] * 7.0 + 1000.0;
        }
        other.push([15.5, -5e6]);
        let b = fit(&m, "Tank", &other, 10.0, &lever(), 2).unwrap();
        assert_eq!(a, b, "the fit moved when only data after the cut changed");
        // and the fit itself works: the rate that made the data is found
        assert!((a[0].1 - 3.0).abs() < 0.05, "{a:?}");
        // changing a point before the cut does move it
        let mut early = truth.clone();
        early[5][1] += 40.0;
        assert_ne!(fit(&m, "Tank", &early, 10.0, &lever(), 2).unwrap(), a);
    }

    #[test]
    fn a_hindcast_scores_the_model_and_the_guesses() {
        let m = ramp("1");
        let csv: String = std::iter::once("time,Tank\n".to_string()).chain((0..=20).map(|t| format!("{t},{}\n", 3 * t))).collect();
        let spec = Spec { var: None, cut: "10".into(), fit: true, levers: lever(), runs: 100, spread: 0.1, seed: 1, season: 0.0, effort: 2 };
        let (r, svg) = run(&m, &csv, &spec).unwrap();
        assert_eq!((r.train_points, r.test_points), (11, 10));
        assert!(r.beats_all || r.scores[0].rmse < 0.5, "{r:?}");
        assert!(r.scores.iter().any(|s| s.method == "Last value") && r.scores.iter().any(|s| s.method == "Straight-line trend"));
        assert!(svg.contains("<polygon") && markdown(&r).contains("Nominal share"));
    }

    #[test]
    fn coverage_is_close_to_nominal_on_a_model_with_known_noise() {
        // Tank = Rate x time, Rate drawn uniformly within 20% of 1 by the ensemble. The "world" draws its own Rate from the same
        // distribution 300 times; each draw gives one actual path. Tolerance: 8 points at 50% and 5 points at 90% (about 2.8
        // standard errors of a share from 300 independent draws).
        let m = ramp("1");
        let sens = dynamist_core::sensitivity::run_sensitivity(&m, "Tank", 1000, 0.2, 5, 4, &std::sync::atomic::AtomicUsize::new(0)).unwrap();
        let mut rng = Rng::new(99);
        let (mut inner, mut outer) = ((0, 0), (0, 0));
        for _ in 0..300 {
            let rate = 1.0 + 0.2 * (2.0 * rng.uniform() - 1.0);
            let path: Vec<[f64; 2]> = (11..=20).map(|t| [t as f64, rate * t as f64]).collect();
            let c50 = coverage(&sens.times, &sens.p25, &sens.p75, &path, 0.5);
            let c90 = coverage(&sens.times, &sens.p5, &sens.p95, &path, 0.9);
            inner = (inner.0 + c50.inside, inner.1 + c50.points);
            outer = (outer.0 + c90.inside, outer.1 + c90.points);
        }
        let (s50, s90) = (inner.0 as f64 / inner.1 as f64, outer.0 as f64 / outer.1 as f64);
        eprintln!("coverage 50%: {s50:.3} (n={}), 90%: {s90:.3} (n={})", inner.1, outer.1);
        assert!((s50 - 0.5).abs() < 0.08, "50% band: {s50}");
        assert!((s90 - 0.9).abs() < 0.05, "90% band: {s90}");
    }

    #[test]
    fn dates_are_read_through_the_model_start() {
        let mut m = ramp("1");
        m.settings.start = Some("2020-01-01".into());
        m.settings.time_units = "Day".into();
        let t = parse_time(&m, "2020-01-11").unwrap();
        assert!((t - 10.0).abs() < 1e-6, "{t}");
        assert!(parse_time(&m, "2031-01-01").is_err());
        assert!(parse_time(&ramp("1"), "2020-01-11").is_err());
    }
}
