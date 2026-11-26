//! Stage C5: automatic experiments. The model is compiled once per case and the cases run in parallel threads (up to
//! `Options::jobs`). Every case is checked against the invariants: no number that is not a number or is infinite, material
//! stocks not below zero, conservation, ratios within 0 to 1, and the modeller's own checks.

use crate::expr;
use crate::model::{Kind, Model};
use crate::sim::{self, Run};

/// One case: a label and the model with its changes.
pub struct Case {
    pub label: String,
    pub model: Model,
}

/// Set a constant to a value.
pub fn with_value(m: &Model, name: &str, value: f64) -> Model {
    let mut c = m.clone();
    if let Some(v) = c.by_name(&expr::normalize(name)).map(|v| v.id) {
        if let Some(v) = c.var_mut(v) {
            v.equation = format!("{value}");
        }
    }
    c
}

/// Run the cases on up to `jobs` threads.
pub fn run_cases(cases: Vec<Case>, jobs: usize) -> Vec<(String, Model, Option<Run>)> {
    let jobs = jobs.max(1).min(cases.len().max(1));
    let queue = std::sync::Mutex::new(cases.into_iter().enumerate().collect::<Vec<_>>());
    let results = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let next = queue.lock().unwrap().pop();
                let Some((i, case)) = next else { break };
                let run = sim::compile(&case.model).ok().and_then(|c| sim::simulate(&c, &case.model, "case").ok());
                results.lock().unwrap().push((i, case.label, case.model, run));
            });
        }
    });
    let mut v = results.into_inner().unwrap();
    v.sort_by_key(|x| x.0);
    v.into_iter().map(|(_, l, m, r)| (l, m, r)).collect()
}

/// A variable named as a fraction, share, utilisation or probability whose units, if any, are those of a pure number.
fn is_fraction(v: &crate::model::Var) -> bool {
    let n = v.name.to_lowercase();
    if !["fraction", "share", "utilisation", "utilization", "probability"].iter().any(|w| n.contains(w)) {
        return false;
    }
    let u = v.units.trim();
    u.is_empty() || crate::units::parse_unit_exact(u).map(|x| x.is_dimensionless()).unwrap_or(false)
}

/// The first thing that breaks an invariant in `run`, given what the base run looked like: (variable, what).
pub fn first_break(m: &Model, base: &Run, run: &Run) -> Option<(String, String)> {
    // evaluation order is the order of the variables of the model
    for v in &m.vars {
        let Some(s) = run.get(&v.name) else { continue };
        // LEVELISED, PAYBACK and RATE are missing (not a number) by design when there is nothing to divide or no root
        let by_design = {
            let eq = v.equation.to_uppercase();
            ["LEVELISED", "LCOE", "PAYBACK", "RATE("].iter().any(|w| eq.contains(w))
        };
        if let Some(i) = s.iter().position(|x| !x.is_finite()).filter(|_| !by_design) {
            return Some((v.name.clone(), format!("is not a number or is infinite at time {}", super::num(run.times[i]))));
        }
        // only a stock declared "never negative" may not go below zero: Cash, Inventory or Profit may legitimately do so
        if v.kind == Kind::Stock && v.non_negative {
            let scale = s.iter().fold(0.0f64, |a, x| a.max(x.abs())).max(1.0);
            if let Some(i) = s.iter().position(|x| *x < -1e-9 * scale) {
                return Some((v.name.clone(), format!("goes below zero ({}) at time {}", super::num(s[i]), super::num(run.times[i]))));
            }
        }
        // a fraction or a share lies within 0 to 1: by its name, and only when it carries no units but a pure number
        if matches!(v.kind, Kind::Aux | Kind::Flow) && is_fraction(v) {
            let inside = |s: &[f64]| s.iter().all(|x| *x >= -1e-9 && *x <= 1.0 + 1e-9);
            if base.get(&v.name).is_some_and(|b| inside(b)) && !inside(s) {
                return Some((v.name.clone(), "leaves the range 0 to 1".into()));
            }
        }
    }
    if let Some(f) = crate::checks::conservation(m, run).into_iter().next() {
        return Some((f.var.and_then(|id| m.var(id)).map(|v| v.name.clone()).unwrap_or_default(), "no longer conserves what flows through it".into()));
    }
    None
}
