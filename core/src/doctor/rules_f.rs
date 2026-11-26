//! Stage C3, rules F: findings of the interval analysis ("can" findings, `confirmed: false` until the base run or an
//! experiment confirms them) and of the signs of the loops.
//!
//! F1: an outflow can take more out of a stock in a step than the stock holds. F2: the same for a stock that is not held at
//! zero, so it can go negative. F3: a division by an interval that contains 0. F8: a lookup whose input can leave the table.
//! F12: a gap that grows (a reinforcing loop through a variable that should close a gap). F14: a ratio, share or
//! probability whose interval leaves [0, 1].

use super::base::expr_text;
use super::fixes::clean_factor;
use super::interval::Iv;
use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::expr::{self, BinOp, Expr};
use crate::model::Kind;
use crate::ops::Op;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "F1", stage: Stage::C3, about: "An outflow of a never-negative stock that can ask for more than the stock holds in one step.", f: f1 }),
        Box::new(FnRule { id: "F2", stage: Stage::C3, about: "An outflow that can drain a stock below zero because nothing holds the stock at zero.", f: f2 }),
        Box::new(FnRule { id: "F3", stage: Stage::C3, about: "A division by something that can be zero.", f: f3 }),
        Box::new(FnRule { id: "F8", stage: Stage::C3, about: "A lookup whose input can leave the range of its table.", f: f8 }),
        Box::new(FnRule { id: "F12", stage: Stage::C3, about: "A gap, error or shortfall in a reinforcing loop: it grows instead of closing.", f: f12 }),
        Box::new(FnRule { id: "F5", stage: Stage::C3, about: "IF THEN ELSE, MIN or MAX on a quantity that changes gradually in reality.", f: f5 }),
        Box::new(FnRule { id: "F6", stage: Stage::C3, about: "A lookup that is not monotonic.", f: f6 }),
        Box::new(FnRule { id: "F7", stage: Stage::C3, about: "A multiplier lookup that does not pass through 1 at a relative input of 1.", f: f7 }),
        Box::new(FnRule { id: "F9", stage: Stage::C3, about: "Time used inside a behavioural equation.", f: f9 }),
        Box::new(FnRule { id: "F10", stage: Stage::C3, about: "An information delay on a material flow, or a material delay used for a perception.", f: f10 }),
        Box::new(FnRule { id: "F11", stage: Stage::C3, about: "DELAY FIXED with a delay time that varies.", f: f11 }),
        Box::new(FnRule { id: "F13", stage: Stage::C3, about: "A desired value computed from the stock it controls.", f: f13 }),
        Box::new(FnRule { id: "F15", stage: Stage::C3, about: "A parameter value written in the documentation text.", f: f15 }),
        Box::new(FnRule { id: "F14", stage: Stage::C3, about: "A ratio, share, fraction or probability that can leave 0 to 1.", f: f14 }),
    ]
}

/// The interval of the initial value of a stock.
fn init_iv(ctx: &Ctx, v: &crate::model::Var) -> Iv {
    let facts = ctx.facts();
    match expr::parse(&v.equation) {
        Ok(Expr::Num(n)) => Iv::point(n),
        Ok(Expr::Var(n)) => facts.iv.get(&n).copied().unwrap_or(super::interval::ALL),
        _ => super::interval::ALL,
    }
}

/// (flow, stock, flow interval, content bound per step) of every outflow whose largest value is more than the stock can hold.
fn over_draining<'a>(ctx: &'a Ctx<'a>) -> Vec<(&'a crate::model::Var, &'a crate::model::Var, Iv, f64)> {
    let m = ctx.model;
    let facts = ctx.facts();
    let mut out = Vec::new();
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let has_in = m.vars.iter().any(|f| f.kind == Kind::Flow && f.to == Some(s.id));
        // the content bound: a stock with no inflow never holds more than it starts with
        let hi = if has_in { f64::INFINITY } else { init_iv(ctx, s).hi };
        if !hi.is_finite() {
            continue;
        }
        let per_step = hi / m.settings.time_step;
        for f in m.vars.iter().filter(|f| f.kind == Kind::Flow && f.from == Some(s.id) && f.to != Some(s.id)) {
            let iv = facts.iv.get(&expr::normalize(&f.name)).copied().unwrap_or(super::interval::ALL);
            // a flow that is itself limited by the stock (it reads the stock's level) is a control, not a runaway
            let mut refs = Vec::new();
            if let Ok(e) = expr::parse(&f.equation) {
                expr::references(&e, &mut refs);
            }
            if refs.contains(&expr::normalize(&s.name)) {
                continue;
            }
            if iv.hi > per_step + 1e-12 {
                out.push((f, s, iv, per_step));
            }
        }
    }
    out
}

fn control_fix(ctx: &Ctx, flow: &crate::model::Var, stock: &crate::model::Var) -> Fix {
    let m = ctx.model;
    let min_time = format!("{} Minimum Time", stock.name);
    let mut ops = Vec::new();
    if m.by_name(&expr::normalize(&min_time)).is_none() {
        ops.push(Op::AddVar { name: min_time.clone(), kind: "constant".into(), equation: clean_factor(2.0 * m.settings.time_step), units: m.settings.time_units.clone(), doc: format!("How fast {} can be drawn down: twice the time step", stock.name), from: None, to: None, non_negative: false, layer: None });
    }
    ops.push(Op::SetEquation { name: flow.name.clone(), equation: format!("MIN({}, {} / {})", flow.equation.trim(), stock.name, min_time) });
    Fix { title: format!("Limit {} by what {} holds: MIN(desired, {} / {})", flow.name, stock.name, stock.name, min_time), ops, keeps_numbers: false, needs_decision: false }
}

fn f1(ctx: &Ctx) -> Vec<Finding> {
    over_draining(ctx)
        .into_iter()
        .filter(|(_, s, _, _)| s.non_negative)
        .map(|(f, s, iv, cap)| {
            let mut x = Finding::new("F1", Severity::Error, Some(&f.name), format!("“{}” can ask for up to {} per time unit but “{}” holds at most {} per step: the outflow will be cut back (the constraint log records it)", f.name, super::num(iv.hi), s.name, super::num(cap)));
            x.source = "Sterman (2000), §13.2.3: a flow out of a stock is limited by what the stock holds".into();
            x.confirmed = ctx.base_if_asked().is_some_and(|r| r.constraints.iter().any(|c| c.stock == s.name));
            x.fixes.push(control_fix(ctx, f, s));
            x
        })
        .collect()
}

fn f2(ctx: &Ctx) -> Vec<Finding> {
    over_draining(ctx)
        .into_iter()
        .filter(|(_, s, _, _)| !s.non_negative)
        .map(|(f, s, iv, cap)| {
            let mut x = Finding::new("F2", Severity::Error, Some(&s.name), format!("“{}” can be drained by “{}” (up to {} per time unit) faster than it holds ({} per step), so it can go negative", s.name, f.name, super::num(iv.hi), super::num(cap)));
            x.source = "Sterman (2000), §13.2.3: a stock of material cannot be negative".into();
            x.confirmed = ctx.base_if_asked().and_then(|r| r.get(&s.name)).is_some_and(|v| v.iter().any(|y| *y < -1e-9));
            x.fixes.push(control_fix(ctx, f, s));
            x
        })
        .collect()
}

fn f3(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for (owner, text, iv) in &ctx.facts().div_zero {
        let mut f = Finding::new("F3", Severity::Error, Some(owner), format!("“{owner}” divides by {text}, which can be zero (it ranges over {} to {})", super::num(iv.lo), super::num(iv.hi)));
        f.source = "Vensim and Stella manuals: ZIDZ and XIDZ for divisions that can meet zero".into();
        f.confirmed = false;
        if let Some(v) = ctx.model.by_name(&expr::normalize(owner)) {
            if let Ok(Expr::Bin(BinOp::Div, a, b)) = expr::parse(&v.equation) {
                f.fixes.push(Fix { title: format!("Use XIDZ so that {owner} is 0 when the divisor is 0"), ops: vec![Op::SetEquation { name: owner.clone(), equation: format!("XIDZ({}, {}, 0)", expr_text(&a, ctx.model), expr_text(&b, ctx.model)) }], keeps_numbers: false, needs_decision: false });
            }
        }
        out.push(f);
    }
    out
}

fn f8(ctx: &Ctx) -> Vec<Finding> {
    ctx.facts()
        .lookup_over
        .iter()
        .map(|(owner, lookup, iv, xr)| {
            let mut f = Finding::new("F8", Severity::Warning, Some(owner), format!("The input of {lookup} in “{owner}” ranges over {} to {} but the table covers {} to {}: beyond its ends the curve is held flat", super::num(iv.lo), super::num(iv.hi), super::num(xr.0), super::num(xr.1)));
            f.source = "Sterman (2000), §14.3: a table must cover the whole range its input can take".into();
            f.confirmed = false;
            f
        })
        .collect()
}

fn f12(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    // only models that have a gap, discrepancy, shortfall or deviation to look at (the polarity pass is a whole run)
    if !m.vars.iter().any(|v| ["gap", "discrepancy", "shortfall", "deviation"].iter().any(|w| v.name.to_lowercase().contains(w))) {
        return Vec::new();
    }
    let mut copy = m.clone();
    let d = crate::cld::from_model(&mut copy);
    let found = crate::cld::find_loops(&d);
    let pol = crate::polarity::loop_polarities(m, &d, &found);
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (l, p) in found.iter().zip(&pol) {
        if p.label != "reinforcing" {
            continue;
        }
        for n in &l.nodes {
            // a gap that also sits in a balancing loop is doing its job there
            if found.iter().zip(&pol).any(|(l2, p2)| p2.label == "balancing" && l2.nodes.contains(n)) {
                continue;
            }
            let Some(node) = d.nodes.iter().find(|x| x.id == *n) else { continue };
            let lname = node.text.to_lowercase();
            if !["gap", "discrepancy", "shortfall", "deviation"].iter().any(|w| lname.contains(w)) || seen.contains(&node.text) {
                continue;
            }
            let Some(v) = m.by_name(&expr::normalize(&node.text)) else { continue };
            seen.push(node.text.clone());
            let mut f = Finding::new("F12", Severity::Error, Some(&v.name), format!("“{}” is a gap but it sits in a reinforcing loop, so it grows instead of closing: the sign of the correction is probably the wrong way round", v.name));
            f.source = "Sterman (2000), §4.3: a goal-seeking loop corrects the gap, which makes it balancing".into();
            if let Ok(Expr::Bin(BinOp::Sub, a, b)) = expr::parse(&v.equation) {
                f.fixes.push(Fix { title: format!("Flip the sign of {}: {} - {}", v.name, expr_text(&b, m), expr_text(&a, m)), ops: vec![Op::SetEquation { name: v.name.clone(), equation: format!("{} - {}", expr_text(&b, m), expr_text(&a, m)) }], keeps_numbers: false, needs_decision: false });
            }
            out.push(f);
        }
    }
    out
}

fn f14(ctx: &Ctx) -> Vec<Finding> {
    let facts = ctx.facts();
    let mut out = Vec::new();
    for v in ctx.model.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Flow)) {
        let n = v.name.to_lowercase();
        if !["fraction", "share", "utilisation", "utilization", "ratio", "probability"].iter().any(|w| n.contains(w)) || v.units.contains('%') {
            continue;
        }
        let dimless = v.units.trim().is_empty() || crate::units::parse_unit_exact(&v.units).map(|u| u.is_dimensionless()).unwrap_or(false);
        let iv = facts.iv.get(&expr::normalize(&v.name)).copied().unwrap_or(super::interval::ALL);
        if dimless && iv.is_finite() && (iv.lo < -1e-9 || iv.hi > 1.0 + 1e-9) {
            let mut f = Finding::new("F14", Severity::Warning, Some(&v.name), format!("“{}” is a ratio or share but it can range over {} to {}, outside 0 to 1", v.name, super::num(iv.lo), super::num(iv.hi)));
            f.source = "Modelling practice: shares, fractions and probabilities lie in [0, 1]".into();
            f.confirmed = false;
            f.fixes.push(Fix { title: format!("Bound {} to 0 to 1: MAX(0, MIN(1, …))", v.name), ops: vec![Op::SetEquation { name: v.name.clone(), equation: format!("MAX(0, MIN(1, {}))", v.equation.trim()) }], keeps_numbers: false, needs_decision: false });
            out.push(f);
        }
    }
    out
}

fn walk_calls(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    match e {
        Expr::Bin(_, a, b) => {
            walk_calls(a, f);
            walk_calls(b, f);
        }
        Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => walk_calls(a, f),
        Expr::Call(_, args) => args.iter().for_each(|a| walk_calls(a, f)),
        _ => {}
    }
}

fn mentions_time(e: &Expr) -> bool {
    let mut t = false;
    walk_calls(e, &mut |x| {
        if matches!(x, Expr::Var(n) if n == "time") {
            t = true;
        }
    });
    t
}

fn f5(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Flow)) {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut hit = false;
        walk_calls(&e, &mut |x| {
            if let Expr::Call(n, args) = x {
                if n == "IF THEN ELSE" && !args.is_empty() && !mentions_time(&args[0]) {
                    let mut r = Vec::new();
                    expr::references(&args[0], &mut r);
                    // a switch on a level or a rate that moves gradually, not on a constant policy setting
                    if r.iter().any(|n| m.by_name(n).is_some_and(|x| matches!(x.kind, Kind::Stock | Kind::Aux | Kind::Flow))) {
                        hit = true;
                    }
                }
            }
        });
        if hit {
            let mut f = Finding::new("F5", Severity::Info, Some(&v.name), format!("“{}” switches on a quantity that changes gradually in reality: a lookup or a smooth form is usually closer to it. Keep the switch if it is a true policy rule", v.name));
            f.source = "MIT D-4851 checklist, item 10; Rahmandad, MIT 15.879".into();
            out.push(f);
        }
    }
    out
}

fn f6(ctx: &Ctx) -> Vec<Finding> {
    ctx.model
        .vars
        .iter()
        .filter(|v| v.table.len() >= 3)
        .filter(|v| {
            let mut ys = v.table.iter().map(|p| p[1]);
            let first = ys.next().unwrap();
            let dirs: Vec<f64> = std::iter::once(first).chain(ys).collect::<Vec<_>>().windows(2).map(|w| w[1] - w[0]).filter(|d| d.abs() > 1e-12).collect();
            dirs.iter().any(|d| *d > 0.0) && dirs.iter().any(|d| *d < 0.0)
        })
        .map(|v| {
            let mut f = Finding::new("F6", Severity::Warning, Some(&v.name), format!("The lookup “{}” is not monotonic: it goes up and down. Separate causal effects, or confirm that it is meant", v.name));
            f.source = "Sterman (2000), §14.4.3".into();
            f
        })
        .collect()
}

fn f7(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for v in ctx.model.vars.iter().filter(|v| v.kind == Kind::Lookup && v.table.len() >= 2) {
        let n = v.name.to_lowercase();
        if !(n.contains("effect") || n.contains("multiplier")) {
            continue;
        }
        let xs: Vec<f64> = v.table.iter().map(|p| p[0]).collect();
        if !(xs.iter().cloned().fold(f64::INFINITY, f64::min) <= 1.0 && xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) >= 1.0) {
            continue;
        }
        let y1 = expr::lookup(&v.table, 1.0);
        if (y1 - 1.0).abs() > 1e-9 && y1.abs() > 1e-12 {
            let mut f = Finding::new("F7", Severity::Warning, Some(&v.name), format!("The multiplier “{}” gives {} at a relative input of 1, not 1: with the input at its normal value the effect should leave the normal value unchanged", v.name, super::num(y1)));
            f.source = "MIT D-4587: a table function example".into();
            let pts: Vec<[f64; 2]> = v.table.iter().map(|p| [p[0], p[1] / y1]).collect();
            f.fixes.push(Fix { title: format!("Rescale the table so that it passes through 1 at 1"), ops: vec![Op::SetTable { name: v.name.clone(), points: pts }], keeps_numbers: false, needs_decision: false });
            out.push(f);
        }
    }
    out
}

fn f9(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    // an exogenous input: a constant, a number, or an expression of numbers
    fn exogenous(e: &Expr, m: &crate::model::Model) -> bool {
        match e {
            Expr::Num(_) => true,
            Expr::Var(n) => m.by_name(n).is_some_and(|v| v.kind == Kind::Constant) || matches!(n.as_str(), "initial time" | "final time" | "time step"),
            Expr::Neg(a) => exogenous(a, m),
            Expr::Bin(_, a, b) => exogenous(a, m) && exogenous(b, m),
            _ => false,
        }
    }
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Flow)) {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        // Time inside a test input function (STEP, PULSE, RAMP...) is how an input is written; a period index (Time inside
        // MODULO, INTEGER, INT) and a test of Time against a constant (Time >= 5) are inputs too; anywhere else it is behaviour
        let mut bare = false;
        fn go(e: &Expr, bare: &mut bool, m: &crate::model::Model) {
            match e {
                Expr::Var(n) if n == "time" => *bare = true,
                Expr::Call(n, _) if matches!(n.as_str(), "STEP" | "PULSE" | "PULSE TRAIN" | "RAMP" | "AT TIME" | "EVERY" | "PROFILE" | "PROFILE STEP" | "SEASON" | "DAYLIGHT" | "ESCALATE" | "MODULO" | "INTEGER" | "INT") => {}
                Expr::Call(_, a) => a.iter().for_each(|x| go(x, bare, m)),
                Expr::Bin(op, a, b) if matches!(op, BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne) && ((matches!(&**a, Expr::Var(t) if t == "time") && exogenous(b, m)) || (matches!(&**b, Expr::Var(t) if t == "time") && exogenous(a, m))) => {}
                Expr::Bin(_, a, b) => {
                    go(a, bare, m);
                    go(b, bare, m);
                }
                Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => go(a, bare, m),
                _ => {}
            }
        }
        go(&e, &mut bare, m);
        if bare {
            let mut f = Finding::new("F9", Severity::Warning, Some(&v.name), format!("“{}” uses Time inside its equation: behaviour should come from the structure, with Time only in test inputs or data", v.name));
            f.source = "Sterman (2000), ch. 21".into();
            out.push(f);
        }
    }
    out
}

fn swap_function(src: &str, from: &str, to: &str) -> String {
    let up = src.to_uppercase();
    match up.find(&format!("{from}(")) {
        Some(i) => format!("{}{}{}", &src[..i], to, &src[i + from.len()..]),
        None => src.to_string(),
    }
}

fn f10(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Flow)) {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let lname = v.name.to_lowercase();
        let perception = ["perceived", "perception", "belief", "believed", "expected", "anticipated", "reported"].iter().any(|w| lname.contains(w));
        let material = v.kind == Kind::Flow && (v.from.is_some() || v.to.is_some());
        let (mut smooth, mut delay) = (false, false);
        walk_calls(&e, &mut |x| {
            if let Expr::Call(n, _) = x {
                match n.as_str() {
                    "SMOOTH" | "SMOOTH3" | "SMOOTHI" | "SMOOTH3I" => smooth = true,
                    "DELAY1" | "DELAY3" | "DELAY1I" | "DELAY3I" | "DELAY FIXED" => delay = true,
                    _ => {}
                }
            }
        });
        if smooth && material {
            let mut f = Finding::new("F10", Severity::Warning, Some(&v.name), format!("“{}” is a material flow but is smoothed: smoothing is an information delay; material that goes in must come out later (DELAY)", v.name));
            f.source = "Sterman (2000), ch. 11".into();
            let eq = swap_function(&swap_function(&v.equation, "SMOOTH3", "DELAY3"), "SMOOTH", "DELAY1");
            if eq != v.equation {
                f.fixes.push(Fix { title: "Use a material delay (DELAY1 or DELAY3)".into(), ops: vec![Op::SetEquation { name: v.name.clone(), equation: eq }], keeps_numbers: false, needs_decision: false });
            }
            out.push(f);
        } else if delay && perception && !material {
            let mut f = Finding::new("F10", Severity::Warning, Some(&v.name), format!("“{}” is a perception but uses a material delay: a perception is an information delay (SMOOTH)", v.name));
            f.source = "Sterman (2000), ch. 11".into();
            let eq = swap_function(&swap_function(&v.equation, "DELAY3", "SMOOTH3"), "DELAY1", "SMOOTH");
            if eq != v.equation {
                f.fixes.push(Fix { title: "Use an information delay (SMOOTH or SMOOTH3)".into(), ops: vec![Op::SetEquation { name: v.name.clone(), equation: eq }], keeps_numbers: false, needs_decision: false });
            }
            out.push(f);
        }
    }
    out
}

fn f11(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Flow)) {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut varying = false;
        walk_calls(&e, &mut |x| {
            if let Expr::Call(n, args) = x {
                if n == "DELAY FIXED" && args.len() >= 2 {
                    let constant = match &args[1] {
                        Expr::Num(_) => true,
                        Expr::Var(t) => m.by_name(t).is_some_and(|c| c.kind == Kind::Constant),
                        _ => false,
                    };
                    varying |= !constant;
                }
            }
        });
        if varying {
            let mut f = Finding::new("F11", Severity::Warning, Some(&v.name), format!("“{}” uses DELAY FIXED with a delay time that changes during the run: a pipeline delay of changing length loses or duplicates items", v.name));
            f.source = "Vensim help: DELAY FIXED".into();
            if let Expr::Call(n, args) = &e {
                if n == "DELAY FIXED" && args.len() >= 2 {
                    let eq = format!("DELAY3({}, {})", expr_text(&args[0], m), expr_text(&args[1], m));
                    f.fixes.push(Fix { title: "Use DELAY3, whose time may vary".into(), ops: vec![Op::SetEquation { name: v.name.clone(), equation: eq }], keeps_numbers: false, needs_decision: false });
                }
            }
            out.push(f);
        }
    }
    out
}

fn f13(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux)) {
        let n = v.name.to_lowercase();
        if !(n.contains("desired") || n.contains("target") || n.contains("goal")) {
            continue;
        }
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut r = Vec::new();
        expr::references(&e, &mut r);
        let controlled: Vec<&crate::model::Var> = r.iter().filter_map(|x| m.by_name(x)).filter(|x| x.kind == Kind::Stock).collect();
        // the stock must be one that the variable's own correction acts on: a stock that some flow (reading this variable) fills
        let acts = controlled.iter().any(|s| {
            m.vars.iter().any(|f| f.kind == Kind::Flow && (f.to == Some(s.id) || f.from == Some(s.id)) && expr::parse(&f.equation).is_ok_and(|fe| { let mut fr = Vec::new(); expr::references(&fe, &mut fr); fr.contains(&expr::normalize(&v.name)) }))
        });
        if acts {
            let mut f = Finding::new("F13", Severity::Info, Some(&v.name), format!("“{}” is worked out from the stock it controls: if the stock falls the goal falls with it (an eroding goal). Confirm that this is meant, or anchor the goal", v.name));
            f.source = "Sterman (2000), ch. 15 (the eroding goals archetype)".into();
            out.push(f);
        }
    }
    out
}

fn f15(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for v in ctx.model.vars.iter().filter(|v| v.kind == Kind::Constant) {
        let Ok(Expr::Num(x)) = expr::parse(&v.equation) else { continue };
        if x == 0.0 || x == 1.0 {
            continue;
        }
        let text = super::num(x);
        for form in [format!("{x}"), text.clone()] {
            if v.comment.contains(&form) {
                let mut f = Finding::new("F15", Severity::Info, Some(&v.name), format!("The description of “{}” contains its value ({form}): when the value changes the text will be wrong", v.name));
                f.source = "MIT D-4851 checklist, item 4".into();
                f.fixes.push(Fix { title: "Take the number out of the description".into(), ops: vec![Op::SetDoc { name: v.name.clone(), doc: v.comment.replace(&form, "its value").trim().to_string() }], keeps_numbers: true, needs_decision: false });
                out.push(f);
                break;
            }
        }
    }
    out
}
