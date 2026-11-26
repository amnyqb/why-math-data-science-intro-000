//! Sensitivity runs (Monte Carlo): vary the key levers at random within a spread, run the model
//! many times, and report the band of outcomes for one variable: "how sure are we?".
//!
//! Each run draws every lever from `value × (1 ± spread)` (uniformly). A per-set lever moves all
//! its items together through its all-items scale. Without any levers flagged, every numeric
//! constant is varied. The draws use a fixed seed, so a run is repeatable.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::model::{Kind, Model};

#[derive(Clone)]
pub struct SensResult {
    pub var: String,
    pub times: Vec<f64>,
    /// 5th, 25th, 50th, 75th and 95th percentile at each time, plus min and max.
    pub p5: Vec<f64>,
    pub p25: Vec<f64>,
    pub p50: Vec<f64>,
    pub p75: Vec<f64>,
    pub p95: Vec<f64>,
    pub min: Vec<f64>,
    pub max: Vec<f64>,
    pub base: Vec<f64>,
    pub varied: Vec<String>,
    pub runs: usize,
}

fn xorshift(s: &mut u64) -> f64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    (*s >> 11) as f64 / (1u64 << 53) as f64
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let pos = q * (sorted.len() - 1) as f64;
    let (i, f) = (pos.floor() as usize, pos - pos.floor());
    if i + 1 < sorted.len() { sorted[i] * (1.0 - f) + sorted[i + 1] * f } else { sorted[i] }
}

/// What gets varied: (variable id, is a per-set lever).
pub fn varied(m: &Model) -> Vec<(u64, bool)> {
    let levers: Vec<(u64, bool)> = m.vars.iter().filter(|v| v.lever && v.kind == Kind::Constant).map(|v| (v.id, v.set.is_some())).collect();
    if !levers.is_empty() {
        return levers;
    }
    m.vars
        .iter()
        .filter(|v| v.kind == Kind::Constant && v.set.is_none() && v.equation.trim().parse::<f64>().is_ok_and(|x| x != 0.0))
        .map(|v| (v.id, false))
        .collect()
}

/// Run the model `runs` times with the levers drawn at random; summarise one variable.
/// `progress` counts finished runs (for a progress bar); `workers` threads share the work.
pub fn run_sensitivity(model: &Model, var: &str, runs: usize, spread: f64, seed: u64, workers: usize, progress: &AtomicUsize) -> Result<SensResult, String> {
    let ids = varied(model);
    if ids.is_empty() {
        return Err("Nothing to vary: mark some constants as key levers (or give a constant a non-zero value)".into());
    }
    let key = crate::expr::normalize(var);
    // the baseline, and the time axis
    let base_run = {
        let c = crate::sim::compile(model).map_err(|p| p.first().map(|x| x.message.clone()).unwrap_or_default())?;
        crate::sim::simulate(&c, model, "base")?
    };
    let base = base_run.series.get(&key).cloned().ok_or_else(|| format!("No variable called '{var}' has a series to watch"))?;
    let times = base_run.times.clone();
    // each run's draws are fixed in advance, so the answer does not depend on the thread count
    let mut s = seed | 1;
    let draws: Vec<Vec<f64>> = (0..runs).map(|_| ids.iter().map(|_| 1.0 + spread * (2.0 * xorshift(&mut s) - 1.0)).collect()).collect();
    let results: Mutex<Vec<(usize, Vec<f64>)>> = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..workers.max(1) {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= runs {
                    break;
                }
                let mut m = model.clone();
                for ((id, per_set), f) in ids.iter().zip(&draws[i]) {
                    if let Some(v) = m.var_mut(*id) {
                        if *per_set {
                            v.lever_scale *= f;
                        } else if let Ok(x) = v.equation.trim().parse::<f64>() {
                            v.equation = format!("{}", x * f);
                        }
                    }
                }
                let series = crate::sim::compile(&m).ok().and_then(|c| crate::sim::simulate(&c, &m, "s").ok()).and_then(|r| r.series.get(&key).cloned());
                if let Some(ser) = series {
                    results.lock().unwrap().push((i, ser));
                }
                progress.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    let mut all = results.into_inner().unwrap();
    if all.is_empty() {
        return Err("Every run failed".into());
    }
    all.sort_by_key(|(i, _)| *i);
    let n = times.len();
    let (mut p5, mut p25, mut p50, mut p75, mut p95, mut mn, mut mx) = (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    for t in 0..n {
        let mut col: Vec<f64> = all.iter().filter_map(|(_, s)| s.get(t).copied()).filter(|x| x.is_finite()).collect();
        col.sort_by(|a, b| a.total_cmp(b));
        p5.push(percentile(&col, 0.05));
        p25.push(percentile(&col, 0.25));
        p50.push(percentile(&col, 0.5));
        p75.push(percentile(&col, 0.75));
        p95.push(percentile(&col, 0.95));
        mn.push(col.first().copied().unwrap_or(f64::NAN));
        mx.push(col.last().copied().unwrap_or(f64::NAN));
    }
    Ok(SensResult {
        var: var.to_string(),
        times,
        p5,
        p25,
        p50,
        p75,
        p95,
        min: mn,
        max: mx,
        base,
        varied: ids.iter().filter_map(|(id, _)| model.var(*id).map(|v| v.name.clone())).collect(),
        runs: all.len(),
    })
}

