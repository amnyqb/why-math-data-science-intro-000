//! Stage C2 and C4, rules I: initial values.
//!
//! I1: initial values that depend on each other (a loop in the start values). I2: the model says it starts in equilibrium
//! but the net flows of its stocks are not zero at the start; the fix solves the equilibrium (Newton's method on the state
//! derivatives with constant inputs, tolerance 1e-9, at most 50 iterations) and proposes the initial values.

use super::fixes::clean_factor;
use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::expr;
use crate::model::{Kind, Model};
use crate::ops::Op;
use crate::sim;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "I1", stage: Stage::C2, about: "Initial values that depend on each other.", f: i1 }),
        Box::new(FnRule { id: "I3", stage: Stage::C4, about: "The start-up transient dominates the first part of the run.", f: i3 }),
        Box::new(FnRule { id: "I2", stage: Stage::C4, about: "The model starts in equilibrium by its own statement, but its stocks are changing at the start.", f: i2 }),
    ]
}

fn i1(ctx: &Ctx) -> Vec<Finding> {
    ctx.problems()
        .iter()
        .filter(|p| p.message.starts_with("Initial values depend on each other"))
        .map(|p| {
            let mut f = Finding::new("I1", Severity::Error, p.var.as_deref(), p.message.clone());
            f.source = "Sterman (2000), §6.3.2: initial values must be computable from constants".into();
            f
        })
        .collect()
}

/// Solve for the state where every derivative is zero, starting from the initial state. Returns (stock name, value).
pub fn solve_equilibrium(m: &Model) -> Option<Vec<(String, f64)>> {
    let c = sim::compile(m).ok()?;
    let names = sim::state_names(&c);
    let t0 = m.settings.initial_time;
    let mut y = sim::jacobian_at(m, t0, None).ok()?.y;
    for _ in 0..50 {
        let snap = sim::jacobian_at(m, t0, Some(y.clone())).ok()?;
        let f = &snap.dy;
        let scale = y.iter().fold(1.0f64, |a, b| a.max(b.abs()));
        if f.iter().all(|x| x.abs() < 1e-9 * scale) {
            return Some(names.iter().zip(&y).map(|(n, v)| (n.clone(), *v)).collect());
        }
        let dx = solve_linear(&snap.jac, f)?;
        for (yi, d) in y.iter_mut().zip(dx) {
            *yi -= d;
        }
    }
    None
}

/// Gaussian elimination with partial pivoting: J x = f.
fn solve_linear(j: &[Vec<f64>], f: &[f64]) -> Option<Vec<f64>> {
    let n = f.len();
    let mut a: Vec<Vec<f64>> = (0..n).map(|i| { let mut r = j[i].clone(); r.push(f[i]); r }).collect();
    for c in 0..n {
        let p = (c..n).max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs()))?;
        if a[p][c].abs() < 1e-14 {
            return None;
        }
        a.swap(c, p);
        for r in c + 1..n {
            let k = a[r][c] / a[c][c];
            for cc in c..=n {
                let v = a[c][cc];
                a[r][cc] -= k * v;
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n).map(|k| a[i][k] * x[k]).sum();
        x[i] = (a[i][n] - s) / a[i][i];
    }
    Some(x)
}

fn i2(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if !m.settings.starts_in_equilibrium {
        return Vec::new();
    }
    let Some(base) = ctx.base() else { return Vec::new() };
    let mut moving: Vec<String> = Vec::new();
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        if let Some(lv) = base.get(&s.name).filter(|l| l.len() > 1) {
            let rate = (lv[1] - lv[0]) / (base.times[1] - base.times[0]);
            let scale = lv.iter().fold(0.0f64, |a, x| a.max(x.abs())).max(1e-9);
            if rate.abs() > 1e-6 * scale {
                moving.push(s.name.clone());
            }
        }
    }
    if moving.is_empty() {
        return Vec::new();
    }
    let mut f = Finding::new("I2", Severity::Warning, Some(&moving[0]), format!("The model says it starts in equilibrium but {} change at the start", moving.join(", ")));
    f.source = "Sterman (2000), §21.4: a model started in equilibrium should stay there until it is disturbed".into();
    if let Some(sol) = solve_equilibrium(m) {
        let ops: Vec<Op> = sol
            .iter()
            .filter(|(n, _)| m.by_name(&expr::normalize(n)).is_some_and(|v| v.kind == Kind::Stock))
            .map(|(n, v)| Op::SetEquation { name: n.clone(), equation: clean_factor(*v) })
            .collect();
        f.fixes.push(Fix { title: "Start every stock at its equilibrium value".into(), ops, keeps_numbers: false, needs_decision: false });
    }
    vec![f]
}

fn i3(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let Some(base) = ctx.base() else { return Vec::new() };
    let mut out = Vec::new();
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let Some(x) = base.get(&s.name).filter(|x| x.len() >= 20) else { continue };
        let total: f64 = x.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let scale = x.iter().fold(0.0f64, |a, b| a.max(b.abs())).max(1e-9);
        if total < 0.01 * scale {
            continue;
        }
        let early: f64 = x[..x.len() / 10 + 1].windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        if early > 0.8 * total {
            let mut f = Finding::new("I3", Severity::Info, Some(&s.name), format!("{:.0}% of everything “{}” does happens in the first tenth of the run: the start-up transient dominates; start the model in equilibrium or leave a warm-up period out of the results", 100.0 * early / total, s.name));
            f.source = "Vensim help: Initialisation in equilibrium".into();
            if let Some(sol) = solve_equilibrium(m) {
                let ops: Vec<Op> = sol.iter().filter(|(n, _)| m.by_name(&expr::normalize(n)).is_some_and(|v| v.kind == Kind::Stock)).map(|(n, v)| Op::SetEquation { name: n.clone(), equation: clean_factor(*v) }).collect();
                f.fixes.push(Fix { title: "Start every stock at its equilibrium value".into(), ops, keeps_numbers: false, needs_decision: false });
            }
            out.push(f);
        }
    }
    out
}
