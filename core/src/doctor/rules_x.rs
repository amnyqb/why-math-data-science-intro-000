//! Stage C5, rules X: automatic experiments. They run with `--experiments` (and under the publish gate).
//!
//! X1: each constant (one with a declared range, a lever of the manifest or a slider, at the two ends of that range only) at 0 times its value (a time constant at one time step), 10 times, and the ends of its slider range:
//! the invariants must hold (no number that is not a number, no stock declared never negative below zero, fractions within
//! 0 to 1, conservation).
//! X2: every source inflow (a flow from outside the model) set to 0: no stock may rise.
//! X3: a constant raised by 10%: the sign of the change in the net flow of the stock it feeds must be the sign of the arrow
//! the modeller drew. X6: the modeller's own checks must also hold in the extreme cases of X1.
//! Not implemented: X4 (loop knockout) and X5 (equilibrium, then a step in the largest input). Above 200 cases only the
//! first 200 are run and the rest are reported as not tested; the Morris screening the prompt asks for is not used.

use std::collections::HashSet;

use super::experiments::{Case, first_break, run_cases, with_value};
use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::expr::{self, BinOp, Expr};
use crate::model::Kind;
use crate::ops::Op;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "X1", stage: Stage::C5, about: "A constant at zero, ten times its value or an end of its range makes the model break an invariant; a constant with a declared range (a lever or a slider) is tried at the two ends of that range only.", f: x1 }),
        Box::new(FnRule { id: "X2", stage: Stage::C5, about: "A stock rises although every source inflow is zero.", f: x2 }),
        Box::new(FnRule { id: "X3", stage: Stage::C5, about: "An arrow whose drawn sign disagrees with what raising its constant does.", f: x3 }),
        Box::new(FnRule { id: "X4", stage: Stage::C5, about: "Cutting a feedback loop changes nothing: the loop is inert.", f: x4 }),
        Box::new(FnRule { id: "X5", stage: Stage::C5, about: "From equilibrium, a step in the largest input does not settle although every loop is balancing.", f: x5 }),
        Box::new(FnRule { id: "X6", stage: Stage::C5, about: "One of the modeller's own checks fails in an extreme case that passes in the base run; the worst breach is reported, a diverging run is named, and a check marked as a validity range or a promise gives information.", f: x6 }),
    ]
}

fn plain_constants(ctx: &Ctx) -> Vec<(String, f64, Option<[f64; 2]>)> {
    ctx.model
        .vars
        .iter()
        .filter(|v| v.kind == Kind::Constant)
        .filter_map(|v| match expr::parse(&v.equation) {
            Ok(Expr::Num(n)) => Some((v.name.clone(), n, v.slider)),
            _ => None,
        })
        .collect()
}

/// Constants that are only ever used as a divisor.
fn only_divisors(ctx: &Ctx) -> HashSet<String> {
    let m = ctx.model;
    fn uses(e: &Expr, name: &str, divisor: &mut bool, other: &mut bool, in_div: bool) {
        match e {
            Expr::Var(n) if n == name => {
                if in_div { *divisor = true } else { *other = true }
            }
            Expr::Bin(BinOp::Div, a, b) => {
                uses(a, name, divisor, other, in_div);
                uses(b, name, divisor, other, true);
            }
            Expr::Bin(_, a, b) => {
                uses(a, name, divisor, other, in_div);
                uses(b, name, divisor, other, in_div);
            }
            Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => uses(a, name, divisor, other, in_div),
            Expr::Call(_, args) => args.iter().for_each(|a| uses(a, name, divisor, other, in_div)),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    for (n, _, _) in plain_constants(ctx) {
        let key = expr::normalize(&n);
        let (mut d, mut o) = (false, false);
        for v in &m.vars {
            if let Ok(e) = expr::parse(&v.equation) {
                uses(&e, &key, &mut d, &mut o, false);
            }
        }
        if d && !o {
            out.insert(n);
        }
    }
    out
}

/// The constants that can reach a stock (through equations or flows): the others cannot change anything the invariants watch.
fn reaching_a_stock(ctx: &Ctx) -> HashSet<String> {
    let m = ctx.model;
    let users = super::rules_g::users(m);
    let mut out = HashSet::new();
    for (i, v) in m.vars.iter().enumerate() {
        if v.kind != Kind::Constant {
            continue;
        }
        let mut seen = vec![false; m.vars.len()];
        let mut stack = vec![i];
        while let Some(k) = stack.pop() {
            if std::mem::replace(&mut seen[k], true) {
                continue;
            }
            if m.vars[k].kind == Kind::Stock {
                out.insert(v.name.clone());
                break;
            }
            stack.extend(users[k].iter().copied());
        }
    }
    out
}

/// Above 200 constants: a one-at-a-time screening (each raised by 10%, in parallel) ranks them by how much the stocks move,
/// and only the top 20 get the extreme cases. Returns the names kept and how many were left out.
fn screen_top(ctx: &Ctx, names: &[(String, f64, Option<[f64; 2]>)]) -> (Vec<String>, usize) {
    let Some(base) = ctx.base() else { return (names.iter().map(|n| n.0.clone()).collect(), 0) };
    let cases: Vec<Case> = names.iter().map(|(n, v, _)| Case { label: n.clone(), model: with_value(ctx.model, n, if *v == 0.0 { 1e-3 } else { v * 1.1 }) }).collect();
    let results = run_cases(cases, ctx.opts.jobs);
    let mut scored: Vec<(f64, String)> = results
        .iter()
        .map(|(label, model, run)| {
            let moved = run.as_ref().map_or(f64::INFINITY, |r| {
                model.vars.iter().filter(|v| v.kind == Kind::Stock).map(|v| match (base.get(&v.name), r.get(&v.name)) {
                    (Some(a), Some(b)) => a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max) / a.iter().fold(1e-9f64, |s, x| s.max(x.abs())),
                    _ => 0.0,
                }).fold(0.0, f64::max)
            });
            (moved, label.clone())
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let keep: Vec<String> = scored.iter().take(20).map(|x| x.1.clone()).collect();
    let left = names.len().saturating_sub(keep.len());
    (keep, left)
}

/// The cases of X1: (constant, case), the constants whose zero case was skipped, how many constants were not tested.
fn x1_cases(ctx: &Ctx) -> (Vec<(String, Case)>, Vec<String>, usize) {
    let m = ctx.model;
    let skip0 = only_divisors(ctx);
    let reach = reaching_a_stock(ctx);
    let mut consts: Vec<(String, f64, Option<[f64; 2]>)> = plain_constants(ctx).into_iter().filter(|c| reach.contains(&c.0)).collect();
    let mut untested = 0;
    if consts.len() > 200 {
        let (keep, left) = screen_top(ctx, &consts);
        consts.retain(|c| keep.contains(&c.0));
        untested = left;
    }
    let mut cases = Vec::new();
    let mut skipped = Vec::new();
    for (name, value, slider) in consts {
        let mut vals: Vec<(String, f64)> = Vec::new();
        // a constant with a declared range (a lever of the manifest, or a slider) is tried at the two ends of that range and
        // nowhere else: what lies outside it is not a case the modeller offers
        let declared = m.manifest.levers.iter().find(|l| expr::normalize(&l.name) == expr::normalize(&name)).map(|l| [l.min, l.max]).or(slider);
        if let Some(r) = declared {
            vals.push(("the lower end of its declared range".into(), r[0]));
            vals.push(("the upper end of its declared range".into(), r[1]));
            for (what, x) in vals {
                cases.push((name.clone(), Case { label: format!("{name} at {what} ({})", super::num(x)), model: with_value(m, &name, x) }));
            }
            continue;
        }
        let time_constant = { let n = name.to_lowercase(); ["time", "delay", "lifetime", "duration", "period", "length", "horizon"].iter().any(|w| n.split_whitespace().any(|x| x == *w)) };
        if skip0.contains(&name) {
            skipped.push(name.clone());
        } else if time_constant {
            // a time constant of zero divides by zero by definition: the shortest one that makes sense is one time step
            vals.push(("one time step".into(), m.settings.time_step));
        } else {
            vals.push(("0 times".into(), 0.0));
        }
        vals.push(("10 times".into(), value * 10.0));
        if let Some(s) = slider {
            vals.push(("the lower end of its range".into(), s[0]));
            vals.push(("the upper end of its range".into(), s[1]));
        }
        for (what, x) in vals {
            cases.push((name.clone(), Case { label: format!("{name} at {what} ({})", super::num(x)), model: with_value(m, &name, x) }));
        }
    }
    (cases, skipped, untested)
}

fn x1(ctx: &Ctx) -> Vec<Finding> {
    let Some(base) = ctx.base() else { return Vec::new() };
    let (cases, skipped, untested) = x1_cases(ctx);
    let names: Vec<String> = cases.iter().map(|c| c.0.clone()).collect();
    let results = run_cases(cases.into_iter().map(|c| c.1).collect(), ctx.opts.jobs);
    let mut out = Vec::new();
    let mut reported: HashSet<String> = HashSet::new();
    for ((label, model, run), constant) in results.iter().zip(&names) {
        if reported.contains(constant) {
            continue;
        }
        let Some(run) = run else {
            let mut f = Finding::new("X1", Severity::Error, Some(constant), format!("With {label} the model does not run"));
            f.source = "Sterman (2000), §21.4.2: extreme condition tests".into();
            reported.insert(constant.clone());
            out.push(f);
            continue;
        };
        if let Some((var, what)) = first_break(model, base, run) {
            let mut f = Finding::new("X1", Severity::Warning, Some(constant), format!("With {label}, “{var}” {what}: the model does not hold in this extreme condition"));
            f.source = "Sterman (2000), §21.4.2: extreme condition tests".into();
            reported.insert(constant.clone());
            out.push(f);
        }
    }
    for s in skipped {
        out.push(Finding::new("X1", Severity::Info, Some(&s), format!("“{s}” is only used as a divisor, so the case of 0 times its value was not run")));
    }
    if untested > 0 {
        out.push(Finding::new("X1", Severity::Info, None, format!("{untested} constants were not tested: above 200 constants only the 20 that move the stocks most in a one-at-a-time screening get the extreme cases")));
    }
    out
}

fn x2(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let sources: Vec<&crate::model::Var> = m.vars.iter().filter(|v| v.kind == Kind::Flow && v.from.is_none() && v.to.is_some()).collect();
    if sources.is_empty() {
        return Vec::new();
    }
    let mut c = m.clone();
    for s in &sources {
        if let Some(v) = c.var_mut(s.id) {
            v.equation = "0".into();
        }
    }
    let Ok(comp) = crate::sim::compile(&c) else { return Vec::new() };
    let Ok(run) = crate::sim::simulate(&comp, &c, "x2") else { return Vec::new() };
    let mut out = Vec::new();
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let Some(x) = run.get(&s.name) else { continue };
        let scale = x.iter().fold(0.0f64, |a, b| a.max(b.abs())).max(1.0);
        if let Some(i) = x.iter().position(|v| *v > x[0] + 1e-9 * scale) {
            let mut f = Finding::new("X2", Severity::Warning, Some(&s.name), format!("“{}” rises (to {} at time {}) although every source inflow is set to zero: something creates it from nothing", s.name, super::num(x[i]), super::num(run.times[i])));
            f.source = "Conservation of material: a stock cannot gain what no flow brings".into();
            out.push(f);
        }
    }
    out
}

fn x3(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let Some(base) = ctx.base() else { return Vec::new() };
    let mut out = Vec::new();
    for l in &m.links {
        let sign = match l.polarity.as_str() {
            "+" => 1.0,
            "-" => -1.0,
            _ => continue,
        };
        let (Some(p), Some(f)) = (m.var(l.from), m.var(l.to)) else { continue };
        if p.kind != Kind::Constant || f.kind != Kind::Flow {
            continue;
        }
        let Ok(Expr::Num(x)) = expr::parse(&p.equation) else { continue };
        if x == 0.0 {
            continue;
        }
        let (stock, dir) = match (f.to.and_then(|id| m.var(id)), f.from.and_then(|id| m.var(id))) {
            (Some(s), _) => (s, 1.0),
            (None, Some(s)) => (s, -1.0),
            _ => continue,
        };
        let Ok(comp) = crate::sim::compile(&with_value(m, &p.name, x * 1.1)) else { continue };
        let moved_model = with_value(m, &p.name, x * 1.1);
        let Ok(run) = crate::sim::simulate(&comp, &moved_model, "x3") else { continue };
        let (Some(a), Some(b)) = (base.get(&stock.name), run.get(&stock.name)) else { continue };
        if a.len() < 2 || b.len() < 2 {
            continue;
        }
        let delta = (b[1] - b[0]) - (a[1] - a[0]);
        if delta.abs() < 1e-12 * a[0].abs().max(1.0) {
            continue;
        }
        if delta.signum() != sign * dir {
            let mut fx = Finding::new("X3", Severity::Warning, Some(&p.name), format!("The arrow from “{}” to “{}” is drawn {}, but raising {} by 10% {} the net flow of “{}”", p.name, f.name, l.polarity, p.name, if delta > 0.0 { "raises" } else { "lowers" }, stock.name));
            fx.source = "Sterman (2000), §5.3: the sign of each arrow must agree with the equations".into();
            fx.fixes.push(Fix { title: format!("Draw the arrow from {} to {} as {}", p.name, f.name, if sign > 0.0 { "-" } else { "+" }), ops: vec![Op::Connect { from: p.name.clone(), to: f.name.clone(), polarity: if sign > 0.0 { "-".into() } else { "+".into() } }], keeps_numbers: true, needs_decision: false });
            out.push(fx);
        }
    }
    out
}

/// How far a check is broken in a run: the largest amount by which its variable leaves the allowed values (0 for kinds
/// that have no amount, such as conservation).
fn breach(c: &crate::checks::Check, run: &crate::sim::Run) -> f64 {
    let Some(s) = run.get(&c.var) else { return 0.0 };
    let lo = match c.kind {
        crate::checks::CheckKind::NonNeg => Some(0.0),
        crate::checks::CheckKind::Bounds => c.min,
        _ => None,
    };
    let hi = if c.kind == crate::checks::CheckKind::Bounds { c.max } else { None };
    s.iter().filter(|x| x.is_finite()).map(|x| lo.map_or(0.0, |l| (l - x).max(0.0)).max(hi.map_or(0.0, |h| (x - h).max(0.0)))).fold(0.0, f64::max)
}

/// A run that has blown up: the variable that is furthest from anything the base run had, and how far (a factor).
fn divergence(base: &crate::sim::Run, run: &crate::sim::Run) -> Option<(String, f64)> {
    let mut worst: Option<(String, f64)> = None;
    for (name, s) in &run.series {
        let big = s.iter().filter(|x| x.is_finite()).fold(0.0f64, |a, x| a.max(x.abs()));
        let reference = base.series.get(name).map_or(1.0, |b| b.iter().filter(|x| x.is_finite()).fold(1.0f64, |a, x| a.max(x.abs())));
        let ratio = big / reference;
        if ratio > 1e9 && worst.as_ref().is_none_or(|w| ratio > w.1) {
            worst = Some((name.clone(), big));
        }
    }
    worst
}

fn x6(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if m.checks.is_empty() {
        return Vec::new();
    }
    let Some(base) = ctx.base() else { return Vec::new() };
    if !crate::checks::limits(m, base).is_empty() {
        return Vec::new();
    }
    let (cases, _, _) = x1_cases(ctx);
    let results = run_cases(cases.into_iter().map(|c| c.1).collect(), ctx.opts.jobs);
    // for each case: the divergence of the run, if any, and the checks it breaks with how far
    struct Hit {
        label: String,
        diverges: Option<(String, f64)>,
        worst: (f64, String),
        /// the worst broken check is a validity range or a promise
        declared: bool,
    }
    let mut hits: Vec<Hit> = Vec::new();
    for (label, model, run) in results {
        let Some(run) = run else { continue };
        // only the invariants (conservation, balance, bounds and non-negative that hold at every time) must hold in an
        // extreme case: a check with a source or an `at`, or a comparison with a reference, is a scenario of the base case
        let invariants: Vec<&crate::checks::Check> = model.checks.iter().filter(|c| c.source.trim().is_empty() && c.at.is_none() && c.kind != crate::checks::CheckKind::Reference).collect();
        // an invariant that breaks is reported before a validity range or a promise that breaks
        let (mut worst, mut worst_declared): (Option<(f64, String, bool)>, Option<(f64, String, bool)>) = (None, None);
        for c in invariants {
            let single = crate::model::Model { checks: vec![c.clone()], ..model.clone() };
            // a check that fails because a value is not a number (a divisor constant set to 0) is X1's finding, not a second one
            let Some(f) = crate::checks::limits(&single, &run).into_iter().find(|f| !f.message.contains('–') && !f.message.contains('∞')) else { continue };
            let size = breach(c, &run);
            let slot = if c.role.is_empty() { &mut worst } else { &mut worst_declared };
            if slot.as_ref().is_none_or(|w| size > w.0) {
                *slot = Some((size, f.message.replace('–', "not a number"), !c.role.is_empty()));
            }
        }
        if let Some((size, message, declared)) = worst.or(worst_declared) {
            hits.push(Hit { label, diverges: divergence(base, &run), worst: (size, message), declared });
        }
    }
    // a run that diverges is reported before one that only breaks a check; among those, the one that goes furthest
    let pick = hits.iter().filter(|h| h.diverges.is_some()).max_by(|a, b| a.diverges.as_ref().unwrap().1.total_cmp(&b.diverges.as_ref().unwrap().1)).or_else(|| hits.iter().find(|h| !h.declared)).or_else(|| hits.first());
    let Some(h) = pick else { return Vec::new() };
    let text = match &h.diverges {
        Some((var, big)) => format!("With {} the run diverges: “{var}” reaches {}. The worst broken check: {}", h.label, super::num(*big), h.worst.1),
        None => format!("With {}: {}", h.label, h.worst.1),
    };
    // a validity range or a promise is meant to break outside the world the model describes: information, not an error
    let mut x = if h.declared && h.diverges.is_none() {
        Finding::new("X6", Severity::Info, None, format!("{text} (outside the declared range: this check is a validity range or a promise, and it is meant to break here)"))
    } else {
        Finding::new("X6", Severity::Error, None, text)
    };
    x.source = "Sterman (2000), §21.4: the modeller's reality checks".into();
    vec![x]
}

fn x4(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let Some(base) = ctx.base() else { return Vec::new() };
    let mut copy = m.clone();
    let d = crate::cld::from_model(&mut copy);
    let found = crate::cld::find_loops(&d);
    let mut out = Vec::new();
    for l in found.iter().take(8) {
        let names: Vec<String> = l.nodes.iter().filter_map(|n| d.nodes.iter().find(|x| x.id == *n)).map(|n| n.text.clone()).collect();
        // cut the loop at its first variable that is not a stock or a flow: hold it at its starting value
        let Some(cut) = names.iter().filter_map(|n| m.by_name(&expr::normalize(n))).find(|v| matches!(v.kind, Kind::Aux)) else { continue };
        let mut c = m.clone();
        if let Some(v) = c.var_mut(cut.id) {
            v.equation = format!("INITIAL({})", cut.equation.trim());
        }
        let Ok(comp) = crate::sim::compile(&c) else { continue };
        let Ok(run) = crate::sim::simulate(&comp, &c, "x4") else { continue };
        let stocks: Vec<&str> = names.iter().filter_map(|n| m.by_name(&expr::normalize(n))).filter(|v| v.kind == Kind::Stock).map(|v| v.name.as_str()).collect();
        if stocks.is_empty() {
            continue;
        }
        let same = stocks.iter().all(|s| match (base.get(s), run.get(s)) {
            (Some(a), Some(b)) => a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 1e-9 * x.abs().max(1.0)),
            _ => true,
        });
        if same {
            let mut f = Finding::new("X4", Severity::Info, Some(&cut.name), format!("Cutting the loop {} (holding “{}” at its starting value) changes nothing: the loop is inert, or it was drawn but is not in the equations", names.join(" > "), cut.name));
            f.source = "Rahmandad, MIT 15.879; the book, chapter 17".into();
            out.push(f);
        }
    }
    out
}

fn x5(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut copy = m.clone();
    let d = crate::cld::from_model(&mut copy);
    let found = crate::cld::find_loops(&d);
    if found.is_empty() {
        return Vec::new();
    }
    let pol = crate::polarity::loop_polarities(m, &d, &found);
    if pol.iter().any(|p| p.label != "balancing" && p.label != "inactive") {
        return Vec::new();
    }
    // start from equilibrium where there is one, then raise the largest plain-number input by 10%
    let mut start = m.clone();
    if let Some(sol) = super::rules_i::solve_equilibrium(m) {
        for (n, v) in sol {
            if let Some(x) = start.by_name(&expr::normalize(&n)).filter(|x| x.kind == Kind::Stock).map(|x| x.id) {
                start.var_mut(x).unwrap().equation = format!("{v}");
            }
        }
    }
    let largest = plain_constants(ctx).into_iter().filter(|(_, v, _)| *v != 0.0).max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
    let Some((name, value, _)) = largest else { return Vec::new() };
    let stepped = with_value(&start, &name, value * 1.1);
    let Ok(comp) = crate::sim::compile(&stepped) else { return Vec::new() };
    let Ok(run) = crate::sim::simulate(&comp, &stepped, "x5") else { return Vec::new() };
    let mut out = Vec::new();
    // a pure accumulator (no outflow and no feedback through it) never settles by design: it is not what this test is about
    let in_a_loop = |name: &str| found.iter().any(|l| l.nodes.iter().any(|n| d.nodes.iter().any(|x| x.id == *n && expr::normalize(&x.text) == expr::normalize(name))));
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let has_outflow = m.vars.iter().any(|f| f.kind == Kind::Flow && f.from == Some(s.id));
        if !has_outflow && !in_a_loop(&s.name) {
            continue;
        }
        let Some(x) = run.get(&s.name).filter(|x| x.len() > 10) else { continue };
        let n = x.len();
        let early = x[..n / 4].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f64, f64::max);
        let late = x[3 * n / 4..].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f64, f64::max);
        if early > 1e-9 * x.iter().fold(1.0f64, |a, b| a.max(b.abs())) && late > 0.5 * early {
            let mut f = Finding::new("X5", Severity::Info, Some(&s.name), format!("From equilibrium, a 10% step in “{}” does not die out in the stock “{}” although every loop is balancing: it should settle", m.by_name(&expr::normalize(&name)).map_or(name.as_str(), |v| v.name.as_str()), s.name));
            f.source = "MIT 15.874: the equilibrium and step tests".into();
            out.push(f);
            break;
        }
    }
    out
}
