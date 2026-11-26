//! Optimization: pick a goal, the levers that may change and limits that must hold; the search
//! (random sampling, then Nelder–Mead) runs on its own thread and reports progress as it goes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::expr;
use crate::model::{Kind, Lever, Limit, Measure, Model, OptSpec};
use crate::sim::{self, Run};

#[derive(Clone, Default)]
pub struct Progress {
    pub evals: usize,
    pub budget: usize,
    pub names: Vec<String>,
    pub start_x: Vec<f64>,
    pub start_goal: f64,
    pub start_ok: bool,
    pub best_x: Vec<f64>,
    pub best_goal: f64,
    pub best_ok: bool,
    pub best_measures: Vec<f64>,
    /// (try number, best goal so far)
    pub history: Vec<[f64; 2]>,
    pub best_run: Option<Run>,
    pub start_run: Option<Run>,
    pub done: bool,
    pub stopped: bool,
    pub error: Option<String>,
}

pub struct Job {
    pub sheet: u64,
    pub shared: Arc<Mutex<Progress>>,
    pub cancel: Arc<AtomicBool>,
}

/// One number out of a run, as described by `m`.
pub fn measure(run: &Run, name: &str, m: Measure, at_most: bool) -> Option<f64> {
    let s = run.get(name)?;
    if s.is_empty() {
        return None;
    }
    let dt = if run.times.len() > 1 {
        run.times[1] - run.times[0]
    } else {
        1.0
    };
    Some(match m {
        Measure::Final => *s.last().unwrap(),
        Measure::Peak => s.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        Measure::Lowest => s.iter().copied().fold(f64::INFINITY, f64::min),
        Measure::Total => s[..s.len() - 1].iter().sum::<f64>() * dt,
        Measure::Average => s.iter().sum::<f64>() / s.len() as f64,
        Measure::Always => {
            if at_most {
                s.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            } else {
                s.iter().copied().fold(f64::INFINITY, f64::min)
            }
        }
    })
}

/// How far a measured value breaks a limit, relative to the limit's size (0 = fine).
pub fn violation(l: &Limit, v: f64) -> f64 {
    let over = if l.at_most { v - l.value } else { l.value - v };
    if !v.is_finite() {
        return 1e6;
    }
    over.max(0.0) / l.value.abs().max(1e-6)
}

struct Eval {
    goal: f64,
    viol: f64,
    measures: Vec<f64>,
    run: Run,
}

fn evaluate(
    model: &mut Model,
    spec: &OptSpec,
    names: &[String],
    x: &[f64],
) -> Result<Eval, String> {
    for (n, v) in names.iter().zip(x) {
        let key = expr::normalize(n);
        if let Some(var) = model
            .vars
            .iter_mut()
            .find(|q| expr::normalize(&q.name) == key)
        {
            var.equation = format!("{v}");
        }
    }
    let c = sim::compile(model).map_err(|p| {
        p.first()
            .map(|p| format!("{}: {}", p.var.clone().unwrap_or_default(), p.message))
            .unwrap_or_default()
    })?;
    let run = sim::simulate(&c, model, "Optimized")?;
    let goal = measure(&run, &spec.goal, spec.measure, false)
        .ok_or_else(|| format!("'{}' has no values to optimize", spec.goal))?;
    let mut viol = 0.0;
    let mut measures = Vec::new();
    for l in &spec.limits {
        let v = measure(&run, &l.var, l.measure, l.at_most).unwrap_or(f64::NAN);
        measures.push(v);
        viol += violation(l, v);
    }
    Ok(Eval {
        goal,
        viol,
        measures,
        run,
    })
}

fn rand01(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

/// Start a search in the background.
pub fn start(model: Model, spec: OptSpec, sheet: u64) -> Job {
    let shared = Arc::new(Mutex::new(Progress::default()));
    let cancel = Arc::new(AtomicBool::new(false));
    let (sh, cn) = (shared.clone(), cancel.clone());
    std::thread::spawn(move || {
        let res = search(model, &spec, &sh, &cn);
        let mut p = sh.lock().unwrap();
        if let Err(e) = res {
            p.error = Some(e);
        }
        p.done = true;
    });
    Job {
        sheet,
        shared,
        cancel,
    }
}

fn search(
    mut model: Model,
    spec: &OptSpec,
    shared: &Arc<Mutex<Progress>>,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let levers: Vec<&Lever> = spec.levers.iter().filter(|l| l.on).collect();
    if levers.is_empty() {
        return Err("Tick at least one lever that the search may change".into());
    }
    if spec.goal.is_empty() {
        return Err("Choose what to optimize".into());
    }
    for l in &levers {
        if !(l.max > l.min) {
            return Err(format!(
                "'{}': the highest value must be above the lowest",
                l.name
            ));
        }
    }
    let n = levers.len();
    let names: Vec<String> = levers.iter().map(|l| l.name.clone()).collect();
    let lo: Vec<f64> = levers.iter().map(|l| l.min).collect();
    let hi: Vec<f64> = levers.iter().map(|l| l.max).collect();
    let to_x = |u: &[f64]| -> Vec<f64> {
        u.iter()
            .enumerate()
            .map(|(i, v)| lo[i] + v.clamp(0.0, 1.0) * (hi[i] - lo[i]))
            .collect()
    };
    let budget = [150usize, 450, 1500][spec.effort.clamp(1, 3) as usize - 1];

    // where we start: today's values, kept inside the ranges
    let start_x: Vec<f64> = names
        .iter()
        .enumerate()
        .map(|(i, nm)| {
            model
                .by_name(&expr::normalize(nm))
                .and_then(|v| v.equation.trim().parse::<f64>().ok())
                .unwrap_or((lo[i] + hi[i]) / 2.0)
                .clamp(lo[i], hi[i])
        })
        .collect();
    let base = evaluate(&mut model.clone(), spec, &names, &start_x)?;
    let sign = if spec.maximize { -1.0 } else { 1.0 };
    let penalty = 1e3 * (base.goal.abs() + 1.0);
    {
        let mut p = shared.lock().unwrap();
        *p = Progress {
            budget,
            names: names.clone(),
            start_x: start_x.clone(),
            start_goal: base.goal,
            start_ok: base.viol <= 1e-9,
            best_x: start_x.clone(),
            best_goal: base.goal,
            best_ok: base.viol <= 1e-9,
            best_measures: base.measures.clone(),
            history: vec![[0.0, base.goal]],
            best_run: Some(base.run.clone()),
            start_run: Some(base.run.clone()),
            ..Default::default()
        };
    }
    let mut best_key = (
        if base.viol <= 1e-9 { 0 } else { 1 },
        if base.viol <= 1e-9 {
            sign * base.goal
        } else {
            base.viol
        },
    );
    let mut evals = 1usize;

    // the scalar the search minimises; also records improvements
    let mut f = |u: &[f64], evals: &mut usize| -> f64 {
        *evals += 1;
        let x = to_x(u);
        let Ok(e) = evaluate(&mut model, spec, &names, &x) else {
            return f64::INFINITY;
        };
        let ok = e.viol <= 1e-9;
        let key = (
            if ok { 0 } else { 1 },
            if ok { sign * e.goal } else { e.viol },
        );
        let mut p = shared.lock().unwrap();
        p.evals = *evals;
        if key.0 < best_key.0 || (key.0 == best_key.0 && key.1 < best_key.1) {
            best_key = key;
            p.best_x = x;
            p.best_goal = e.goal;
            p.best_ok = ok;
            p.best_measures = e.measures;
            p.best_run = Some(e.run);
        }
        let g = p.best_goal;
        p.history.push([*evals as f64, g]);
        sign * e.goal + penalty * e.viol
    };

    // 1) scatter tries across the whole range
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let samples = (20 + 10 * n).min(budget / 3);
    let mut best_u: Vec<f64> = start_x
        .iter()
        .enumerate()
        .map(|(i, v)| (v - lo[i]) / (hi[i] - lo[i]))
        .collect();
    let mut best_f = sign * base.goal + penalty * base.viol;
    for _ in 0..samples {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let u: Vec<f64> = (0..n).map(|_| rand01(&mut seed)).collect();
        let v = f(&u, &mut evals);
        if v < best_f {
            best_f = v;
            best_u = u;
        }
    }

    // 2) Nelder–Mead from the best, then once more with a smaller simplex
    for size in [0.25, 0.06] {
        let mut simplex: Vec<(Vec<f64>, f64)> = vec![(best_u.clone(), best_f)];
        for i in 0..n {
            let mut u = best_u.clone();
            u[i] = if u[i] + size <= 1.0 {
                u[i] + size
            } else {
                u[i] - size
            };
            let v = f(&u, &mut evals);
            simplex.push((u, v));
        }
        let stop_at = if size > 0.1 { budget * 3 / 4 } else { budget };
        while evals < stop_at && !cancel.load(Ordering::Relaxed) {
            simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
            let spread = (0..n)
                .map(|i| {
                    simplex
                        .iter()
                        .map(|s| s.0[i])
                        .fold(f64::NEG_INFINITY, f64::max)
                        - simplex.iter().map(|s| s.0[i]).fold(f64::INFINITY, f64::min)
                })
                .fold(0.0, f64::max);
            if spread < 1e-5 {
                break;
            }
            let worst = simplex[n].clone();
            let centroid: Vec<f64> = (0..n)
                .map(|i| simplex[..n].iter().map(|s| s.0[i]).sum::<f64>() / n as f64)
                .collect();
            let at = |t: f64| -> Vec<f64> {
                (0..n)
                    .map(|i| (centroid[i] + t * (worst.0[i] - centroid[i])).clamp(0.0, 1.0))
                    .collect()
            };
            let r = at(-1.0);
            let fr = f(&r, &mut evals);
            if fr < simplex[0].1 {
                let e = at(-2.0);
                let fe = f(&e, &mut evals);
                simplex[n] = if fe < fr { (e, fe) } else { (r, fr) };
            } else if fr < simplex[n - 1].1 {
                simplex[n] = (r, fr);
            } else {
                let c = if fr < worst.1 { at(-0.5) } else { at(0.5) };
                let fc = f(&c, &mut evals);
                if fc < worst.1.min(fr) {
                    simplex[n] = (c, fc);
                } else {
                    // shrink toward the best
                    let b = simplex[0].0.clone();
                    for s in simplex.iter_mut().skip(1) {
                        let u: Vec<f64> = (0..n).map(|i| b[i] + 0.5 * (s.0[i] - b[i])).collect();
                        let v = f(&u, &mut evals);
                        *s = (u, v);
                    }
                }
            }
        }
        simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
        if simplex[0].1 < best_f {
            best_f = simplex[0].1;
            best_u = simplex[0].0.clone();
        }
    }
    let mut p = shared.lock().unwrap();
    p.evals = evals;
    p.stopped = cancel.load(Ordering::Relaxed);
    Ok(())
}

/// Run a search to the end and return how it went (blocks the calling thread).
pub fn run_blocking(model: Model, spec: OptSpec) -> Result<Progress, String> {
    let job = start(model, spec, 0);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));
        if job.shared.lock().unwrap().done {
            break;
        }
    }
    let p = job.shared.lock().unwrap().clone();
    match p.error.clone() {
        Some(e) => Err(e),
        None => Ok(p),
    }
}

/// The measure named in words, as the command line and the tools write it.
pub fn parse_measure(s: &str) -> Result<Measure, String> {
    Ok(match s.trim().to_lowercase().as_str() {
        "final" | "end" => Measure::Final,
        "peak" | "max" => Measure::Peak,
        "lowest" | "min" => Measure::Lowest,
        "total" | "sum" => Measure::Total,
        "average" | "mean" => Measure::Average,
        "always" => Measure::Always,
        other => return Err(format!("unknown measure “{other}” (final, peak, lowest, total, average, always)")),
    })
}

/// "Stock>=0" or "Shortage<=1000:final": a limit that must hold.
pub fn parse_limit(s: &str) -> Result<Limit, String> {
    let (cond, measure) = match s.rsplit_once(':') {
        Some((c, m)) if parse_measure(m).is_ok() => (c, parse_measure(m)?),
        _ => (s, Measure::Final),
    };
    let (at_most, (var, value)) = if let Some(p) = cond.split_once("<=") { (true, p) } else if let Some(p) = cond.split_once(">=") { (false, p) } else { return Err(format!("a limit is written Name<=value or Name>=value, not “{s}”")) };
    let value: f64 = value.trim().parse().map_err(|_| format!("the limit “{s}” needs a number after the comparison"))?;
    Ok(Limit { var: var.trim().to_string(), measure: if measure == Measure::Final && at_most && false { Measure::Always } else { measure }, at_most, value })
}

/// The outcome of a search as JSON: values found, goal before and after, limits as measured.
pub fn result_json(spec: &OptSpec, p: &Progress) -> serde_json::Value {
    use serde_json::json;
    json!({
        "goal": spec.goal,
        "measure": spec.measure.label(),
        "maximize": spec.maximize,
        "start": {"values": p.names.iter().cloned().zip(p.start_x.iter().cloned()).collect::<std::collections::BTreeMap<_, _>>(), "goal": p.start_goal, "within_limits": p.start_ok},
        "best": {"values": p.names.iter().cloned().zip(p.best_x.iter().cloned()).collect::<std::collections::BTreeMap<_, _>>(), "goal": p.best_goal, "within_limits": p.best_ok},
        "limits": spec.limits.iter().zip(&p.best_measures).map(|(l, v)| json!({"variable": l.var, "measure": l.measure.label(), "must_be": if l.at_most { "at most" } else { "at least" }, "value": l.value, "measured": v})).collect::<Vec<_>>(),
        "evaluations": p.evals,
        "stopped_early": p.stopped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_and_measures_are_written_in_words_and_a_search_runs_to_the_end() {
        let l = parse_limit("Shortage<=1000:final").unwrap();
        assert_eq!((l.var.as_str(), l.at_most, l.value, l.measure), ("Shortage", true, 1000.0, Measure::Final));
        let l = parse_limit("Stock >= 0 : always").unwrap();
        assert_eq!((l.at_most, l.measure), (false, Measure::Always));
        assert!(parse_limit("Stock is big").is_err() && parse_measure("median").is_err());
        let m = Model::example();
        let spec = m.sheets.iter().find(|s| s.kind == crate::model::SheetKind::Optimize).unwrap().opt.clone();
        let p = run_blocking(m, spec.clone()).unwrap();
        assert!(p.best_ok);
        let j = result_json(&spec, &p);
        assert!(j["best"]["goal"].as_f64().unwrap() < j["start"]["goal"].as_f64().unwrap());
    }

    #[test]
    fn finds_lower_peak_within_limits() {
        let m = Model::example();
        let spec = m
            .sheets
            .iter()
            .find(|s| s.kind == crate::model::SheetKind::Optimize)
            .unwrap()
            .opt
            .clone();
        let job = start(m, spec, 1);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(20));
            if job.shared.lock().unwrap().done {
                break;
            }
        }
        let p = job.shared.lock().unwrap().clone();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert!(p.best_ok);
        assert!(
            p.best_goal < p.start_goal * 0.9,
            "{} vs {}",
            p.best_goal,
            p.start_goal
        );
        assert!(p.best_measures[0] >= 5000.0 - 1e-6);
    }
}
