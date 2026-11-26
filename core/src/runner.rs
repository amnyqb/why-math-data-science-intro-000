//! Running a model from outside the desktop app: apply what-if overrides, simulate, and turn the
//! result into JSON or CSV.

use serde_json::{Value, json};

use crate::model::Model;
use crate::sim::{self, Run};

/// Run a copy of the model with some values or formulas replaced (the way a scenario does).
pub fn run_with(m: &Model, overrides: &[(String, String)]) -> Result<Run, String> {
    let mut copy = m.clone();
    for (name, value) in overrides {
        if !copy.apply_change(name, value) {
            return Err(format!("there is nothing called “{name}” to override"));
        }
    }
    let c = sim::compile(&copy).map_err(|p| p.iter().map(|x| format!("{}: {}", x.var.clone().unwrap_or_default(), x.message)).collect::<Vec<_>>().join("; "))?;
    sim::simulate(&c, &copy, "run")
}

fn pick<'a>(run: &'a Run, series: &[String]) -> Vec<(String, &'a Vec<f64>)> {
    let mut names: Vec<&String> = run.series.keys().collect();
    names.sort();
    names
        .into_iter()
        .filter(|n| series.is_empty() || series.iter().any(|s| crate::expr::normalize(s) == **n))
        .map(|n| (n.clone(), &run.series[n]))
        .collect()
}

/// Summary of a run: each series' first, last, lowest and highest value. `series` empty = all.
pub fn summary_json(run: &Run, series: &[String]) -> Value {
    let rows: Vec<Value> = pick(run, series)
        .into_iter()
        .map(|(n, s)| {
            let (lo, hi) = s.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
            json!({"name": n, "first": s.first(), "last": s.last(), "min": lo, "max": hi})
        })
        .collect();
    json!({"time_units": run.time_units, "start": run.times.first(), "end": run.times.last(), "steps": run.times.len(), "series": rows})
}

/// Full values of the chosen series, every `every`th step.
pub fn series_json(run: &Run, series: &[String], every: usize) -> Value {
    let every = every.max(1);
    let idx: Vec<usize> = (0..run.times.len()).step_by(every).collect();
    json!({
        "times": idx.iter().map(|i| run.times[*i]).collect::<Vec<_>>(),
        "series": pick(run, series).into_iter().map(|(n, s)| (n, idx.iter().map(|i| s[*i]).collect::<Vec<_>>())).collect::<std::collections::BTreeMap<_, _>>(),
    })
}

pub fn to_csv(run: &Run, series: &[String]) -> String {
    let cols = pick(run, series);
    let mut out = String::from("time");
    for (n, _) in &cols {
        out.push(',');
        out.push_str(&if n.contains(',') { format!("\"{n}\"") } else { n.clone() });
    }
    out.push('\n');
    for (i, t) in run.times.iter().enumerate() {
        out.push_str(&format!("{t}"));
        for (_, s) in &cols {
            out.push_str(&format!(",{}", s[i]));
        }
        out.push('\n');
    }
    out
}

/// Monte Carlo run: the band of outcomes for `var` when the key levers (or every constant) are
/// varied at random by ± `spread` (0.2 = 20%). The summary gives the band at the end of the run
/// and at a few evenly spaced times.
pub fn sensitivity_json(m: &Model, var: &str, runs: usize, spread: f64, seed: u64) -> Result<Value, String> {
    let progress = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
    let r = crate::sensitivity::run_sensitivity(m, var, runs, spread, seed, workers, &progress)?;
    let n = r.times.len();
    let at = |i: usize| json!({"time": r.times[i], "p5": r.p5[i], "p25": r.p25[i], "median": r.p50[i], "p75": r.p75[i], "p95": r.p95[i], "min": r.min[i], "max": r.max[i], "base": r.base[i]});
    let picks: Vec<usize> = (0..5).map(|k| (n - 1) * k / 4).collect();
    Ok(json!({"variable": r.var, "runs": r.runs, "varied": r.varied, "spread": spread, "seed": seed, "at": picks.into_iter().map(at).collect::<Vec<_>>(), "end": at(n - 1)}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_change_the_run_and_unknown_ones_are_refused() {
        let m = Model::example();
        let base = run_with(&m, &[]).unwrap();
        let name = m.vars.iter().find(|v| v.kind == crate::model::Kind::Constant).unwrap().name.clone();
        let other = run_with(&m, &[(name.clone(), "0".into())]).unwrap();
        assert!(!crate::diff::diff_runs(&base, &other, 1e-9).is_same());
        assert!(run_with(&m, &[("No such thing".into(), "1".into())]).is_err());
        let csv = to_csv(&base, &[]);
        assert!(csv.starts_with("time,") && csv.lines().count() == base.times.len() + 1);
    }
}
