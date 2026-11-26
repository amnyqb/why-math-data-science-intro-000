//! Stage C4, rules N: numerical health of the base run.
//!
//! N1: a value that is not a number or is infinite (the first variable and time). N2: the time step is more than a quarter
//! of the fastest time constant of the model (eigenvalues of the Jacobian at 5 evenly spaced times). N4: a Runge-Kutta
//! method with a function that jumps. N5: the time step is longer than the fastest time constant, so Euler is unstable.
//! N7: a stock's net flow changes sign on more than 40% of the steps in a window of 20. N8: a stock whose time constant is
//! more than 10 times the length of the run (it hardly moves). N3 (halving the time step) is `dt-sensitivity` in `base`.

use super::fixes::clean_factor;
use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::expr::{self, Expr};
use crate::model::{Kind, Method};
use crate::ops::Op;
use crate::sim;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "N1", stage: Stage::C4, about: "A value that is not a number or is infinite in the base run.", f: n1 }),
        Box::new(FnRule { id: "N2", stage: Stage::C4, about: "The time step is more than a quarter of the fastest time constant.", f: n2n5 }),
        Box::new(FnRule { id: "N4", stage: Stage::C4, about: "A Runge-Kutta method with step, pulse, fixed-delay or time-test functions that jump.", f: n4 }),
        Box::new(FnRule { id: "N7", stage: Stage::C4, about: "A net flow that changes sign on most steps: the run chatters.", f: n7 }),
        Box::new(FnRule { id: "N8", stage: Stage::C4, about: "A stock so slow that it hardly moves in the length of the run.", f: n8 }),
    ]
}

fn n1(ctx: &Ctx) -> Vec<Finding> {
    let Some(r) = ctx.base() else { return Vec::new() };
    for v in &ctx.model.vars {
        if let Some(s) = r.get(&v.name) {
            let eq = v.equation.to_uppercase();
            if ["LEVELISED", "LCOE", "PAYBACK", "RATE("].iter().any(|w| eq.contains(w)) {
                continue;
            }
            if let Some(i) = s.iter().position(|x| !x.is_finite()) {
                let mut f = Finding::new("N1", Severity::Error, Some(&v.name), format!("“{}” is {} at time {}: the first value that is not a number in the run", v.name, if s[i].is_nan() { "not a number" } else { "infinite" }, super::num(r.times[i])));
                f.source = "IEEE 754: NaN and infinity spread through every value they touch".into();
                return vec![f];
            }
        }
    }
    Vec::new()
}

/// The fastest time constant: the smallest 1/|Re λ| over eigenvalues with |Re λ| > 1e-9, over 5 times.
pub fn fastest_tau(ctx: &Ctx) -> Option<f64> {
    let m = ctx.model;
    let (t0, t1) = (m.settings.initial_time, m.settings.final_time);
    let mut tau: Option<f64> = None;
    for k in 0..5 {
        let at = t0 + (t1 - t0) * k as f64 / 4.0;
        let Ok(snap) = sim::jacobian_at(m, at, None) else { continue };
        let Ok(ev) = crate::eigen::eigenvalues(&snap.jac) else { continue };
        for (re, _) in ev {
            // a time constant a thousandth of the step is the finite difference meeting a jump (STEP, a switch), not a loop
            if re.abs() > 1e-9 && re.is_finite() && 1.0 / re.abs() > 1e-3 * m.settings.time_step {
                let t = 1.0 / re.abs();
                tau = Some(tau.map_or(t, |x: f64| x.min(t)));
            }
        }
    }
    tau
}

fn largest_power_of_half_not_above(x: f64) -> f64 {
    let mut p = 1.0;
    while p > x && p > 1e-12 {
        p /= 2.0;
    }
    p
}

fn n2n5(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if ctx.base().is_none() {
        return Vec::new();
    }
    let Some(tau) = fastest_tau(ctx) else { return Vec::new() };
    let dt = m.settings.time_step;
    let good = largest_power_of_half_not_above(tau / 4.0 * (1.0 + 1e-6));
    // a step that looks too long is confirmed only by reruns at DT/2 and DT/4; one that moves no stock by 1% of its range is not reported
    let rerun = match ctx.base() {
        Some(b) if tau < dt || dt > tau / 4.0 * (1.0 + 1e-6) => super::base::dt_rerun_check(m, b),
        _ => super::base::DtCheck::NotRun,
    };
    let (moved, confirmed) = match rerun {
        super::base::DtCheck::Fine => return Vec::new(),
        super::base::DtCheck::Moved(name, rel) => (format!(". Rerun at a quarter of the step, “{name}” moves by {} of its range", super::base::pct(rel)), true),
        super::base::DtCheck::NotRun => (String::new(), false),
    };
    let fix = Fix { title: format!("Set the time step to {}", clean_factor(good)), ops: vec![Op::SetSetting { key: "time_step".into(), value: clean_factor(good) }], keeps_numbers: false, needs_decision: false };
    if tau < dt && m.settings.method == Method::Euler {
        let mut f = Finding::new("N5", Severity::Error, None, format!("The time step {} is longer than the fastest time constant {} of the model: Euler integration is unstable{moved}", super::num(dt), super::num(tau)));
        f.confirmed = confirmed;
        f.source = "Sterman (2000), §A.2: the step must be shorter than the fastest time constant".into();
        f.fixes.push(fix);
        return vec![f];
    }
    if dt > tau / 4.0 * (1.0 + 1e-6) {
        let mut f = Finding::new("N2", Severity::Warning, None, format!("The time step {} is more than a quarter of the fastest time constant {} of the model{moved}", super::num(dt), super::num(tau)));
        f.confirmed = confirmed;
        f.source = "Sterman (2000), §A.2: choose the step at most a quarter to a half of the smallest time constant".into();
        f.fixes.push(fix);
        return vec![f];
    }
    Vec::new()
}

fn n4(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if m.settings.method != Method::RungeKutta4 {
        return Vec::new();
    }
    let mut hit: Option<(String, &str)> = None;
    fn walk(e: &Expr, f: &mut dyn FnMut(&Expr)) {
        f(e);
        match e {
            Expr::Bin(_, a, b) => {
                walk(a, f);
                walk(b, f);
            }
            Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => walk(a, f),
            Expr::Call(_, args) => args.iter().for_each(|a| walk(a, f)),
            _ => {}
        }
    }
    for v in &m.vars {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        walk(&e, &mut |x| match x {
            Expr::Call(n, _) if matches!(n.as_str(), "STEP" | "PULSE" | "PULSE TRAIN" | "DELAY FIXED") && hit.is_none() => hit = Some((v.name.clone(), if n == "STEP" { "STEP" } else if n == "PULSE" { "PULSE" } else if n == "PULSE TRAIN" { "PULSE TRAIN" } else { "DELAY FIXED" })),
            Expr::Bin(_, a, b) if hit.is_none() && matches!((&**a, &**b), (Expr::Var(t), _) | (_, Expr::Var(t)) if t == "time") && matches!(x, Expr::Bin(op, _, _) if matches!(op, expr::BinOp::Lt | expr::BinOp::Le | expr::BinOp::Gt | expr::BinOp::Ge | expr::BinOp::Eq)) => hit = Some((v.name.clone(), "a test on Time")),
            _ => {}
        });
    }
    let Some((owner, what)) = hit else { return Vec::new() };
    let mut f = Finding::new("N4", Severity::Warning, Some(&owner), format!("The method is RK4 but “{owner}” uses {what}, which jumps: the higher-order steps assume smooth functions and give wrong values around the jump"));
    f.source = "Numerical analysis: Runge-Kutta orders need continuous derivatives".into();
    f.fixes.push(Fix { title: "Switch the method to Euler".into(), ops: vec![Op::SetSetting { key: "method".into(), value: "euler".into() }], keeps_numbers: false, needs_decision: false });
    vec![f]
}

fn n7(ctx: &Ctx) -> Vec<Finding> {
    let (m, Some(r)) = (ctx.model, ctx.base()) else { return Vec::new() };
    // noise chatters by design
    if m.vars.iter().any(|v| { let e = v.equation.to_uppercase(); e.contains("RANDOM") || e.contains("NORMAL(") || e.contains("UNIFORM(") || e.contains("POISSON(") || e.contains("EXPRND(") }) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let Some(x) = r.get(&s.name) else { continue };
        if x.len() < 22 {
            continue;
        }
        let net: Vec<f64> = x.windows(2).map(|w| w[1] - w[0]).collect();
        let scale = x.iter().fold(0.0f64, |a, b| a.max(b.abs())).max(1e-12);
        let sign: Vec<i8> = net.iter().map(|d| if d.abs() < 1e-9 * scale { 0 } else if *d > 0.0 { 1 } else { -1 }).collect();
        let flips: Vec<bool> = sign.windows(2).map(|w| w[0] != 0 && w[1] != 0 && w[0] != w[1]).collect();
        if flips.windows(20).any(|w| w.iter().filter(|b| **b).count() as f64 > 0.4 * 20.0) {
            let mut f = Finding::new("N7", Severity::Error, Some(&s.name), format!("The net flow of “{}” changes sign on most steps: the run chatters, which usually means the time step is too long for a fast loop", s.name));
            f.source = "Sterman (2000), §A.2: oscillation from the step, not from the structure".into();
            let good = match fastest_tau(ctx) {
                Some(tau) => largest_power_of_half_not_above(tau / 4.0).min(m.settings.time_step / 2.0),
                None => m.settings.time_step / 2.0,
            };
            f.fixes.push(Fix { title: format!("Set the time step to {}", clean_factor(good)), ops: vec![Op::SetSetting { key: "time_step".into(), value: clean_factor(good) }], keeps_numbers: false, needs_decision: false });
            out.push(f);
        }
    }
    out
}

fn n8(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if ctx.base().is_none() {
        return Vec::new();
    }
    let Ok(snap) = sim::jacobian_at(m, m.settings.initial_time, None) else { return Vec::new() };
    let Ok(c) = sim::compile(m) else { return Vec::new() };
    let names = sim::state_names(&c);
    let run_len = m.settings.final_time - m.settings.initial_time;
    let mut out = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let Some(v) = m.by_name(&expr::normalize(n)).filter(|v| v.kind == Kind::Stock) else { continue };
        let d = snap.jac[i][i].abs();
        if d > 1e-12 && 1.0 / d > 10.0 * run_len {
            let mut f = Finding::new("N8", Severity::Info, Some(&v.name), format!("“{}” has a time constant of {}, more than 10 times the run of {}: it hardly responds to its own level in this run", v.name, super::num(1.0 / d), super::num(run_len)));
            f.source = "Sterman (2000), §21.4: the run must be long enough for the structure to show".into();
            out.push(f);
        }
    }
    out
}
