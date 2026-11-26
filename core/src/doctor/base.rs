//! The base checks of phase 1 that are not about units (see `units_rules` for those): magic numbers, division by zero,
//! the time step against the model's time constants, `Time =` tests, DT sensitivity, lookups and ratios outside their
//! range, equilibrium, benchmarks, inferred units, and the structure checks that `validate` already had.

use std::collections::HashMap;

use super::fixes::clean_factor;
use super::{Finding, Fix, Options, Severity};
use crate::expr::{self, BinOp, Expr, Tok};
use crate::model::{Kind, Model, Var};
use crate::ops::Op;
use crate::sim;

/// `base` is the base run, made once by the caller (None when the model does not run).
pub fn check(m: &Model, opts: &Options, base: Option<&sim::Run>) -> Vec<Finding> {
    let mut out = Vec::new();
    let timing = std::env::var("DYN_TIMING").is_ok();
    macro_rules! step {
        ($name:expr, $e:expr) => {{
            let t = std::time::Instant::now();
            $e;
            if timing {
                eprintln!("      {:>8.0} ms  base::{}", t.elapsed().as_secs_f64() * 1000.0, $name);
            }
        }};
    }
    step!("magic", magic_numbers(m, &mut out));
    step!("time_equality", time_equality(m, &mut out));
    step!("dt_too_large", dt_too_large(m, base, &mut out));
    step!("inferred_units", inferred_units(m, opts, &mut out));
    let Some(base) = base else { return out };
    step!("structure", structure(m, base, &mut out));
    // one run answers both questions about expressions: divisors that reach zero, lookup inputs outside their tables
    step!("probes", probes(m, &mut out));
    step!("ratio_outside", ratio_outside(m, base, &mut out));
    step!("equilibrium", equilibrium(m, base, &mut out));
    step!("dt_sensitivity", dt_sensitivity(m, base, &mut out));
    out
}

/// A conversion constant the model declares: in the group Conversions, or documented as "Conversion …" (a group with a single
/// member does not survive loading, the documentation does).
pub fn is_conversion(m: &Model, v: &Var) -> bool {
    v.comment.trim_start().starts_with("Conversion") || m.groups.iter().any(|g| g.name.eq_ignore_ascii_case("Conversions") && g.members.contains(&v.id))
}

// ---------------------------------------------------------------------------------------------- structure (validate)

fn structure(m: &Model, base: &sim::Run, out: &mut Vec<Finding>) {
    // the checks the model states about itself, and conservation, against the base run
    for f in crate::checks::all(m, base) {
        let code = if f.message.starts_with("Stock ") { "conservation" } else { "check" };
        let var = f.var.and_then(|id| m.var(id)).map(|v| v.name.clone());
        let mut x = Finding::new(code, Severity::Error, var.as_deref(), f.message);
        x.source = "Dynamist validation".into();
        out.push(x);
    }
}

// ------------------------------------------------------------------------------------------------------ magic number

/// Every numeric literal of an equation other than 0 and 1, with the variable it multiplies or divides (if any).
fn literals(e: &Expr, near: Option<(String, bool)>, out: &mut Vec<(f64, Option<(String, bool)>)>) {
    match e {
        Expr::Num(n) => {
            if *n != 0.0 && *n != 1.0 {
                out.push((*n, near));
            }
        }
        Expr::Var(_) => {}
        Expr::Neg(a) | Expr::Not(a) => literals(a, None, out),
        Expr::Bin(BinOp::Mul, a, b) => {
            literals(a, var_of(b).map(|v| (v, true)), out);
            literals(b, var_of(a).map(|v| (v, true)), out);
        }
        Expr::Bin(BinOp::Div, a, b) => {
            literals(a, None, out);
            literals(b, var_of(a).map(|v| (v, false)), out);
        }
        Expr::Bin(BinOp::Pow, a, b) => {
            literals(a, None, out);
            // an integer exponent (a square, a cube) is not a magic number
            if !matches!(**b, Expr::Num(n) if n.fract() == 0.0) {
                literals(b, None, out);
            }
        }
        Expr::Bin(_, a, b) => {
            literals(a, None, out);
            literals(b, None, out);
        }
        // the engine forms of LEVELISED, LOAN, TAX and VALUE carry option codes as numbers: not magic
        Expr::Call(f, args) if f.ends_with(" STATE") || f == "NPV AT" => args.iter().filter(|a| !matches!(a, Expr::Num(_))).for_each(|a| literals(a, None, out)),
        Expr::Call(_, args) => args.iter().for_each(|a| literals(a, None, out)),
        Expr::LookupCall(_, a) => literals(a, None, out),
        Expr::WithLookup(a, _) => literals(a, None, out),
    }
}

fn var_of(e: &Expr) -> Option<String> {
    if let Expr::Var(n) = e { Some(n.clone()) } else { None }
}

fn magic_numbers(m: &Model, out: &mut Vec<Finding>) {
    for v in &m.vars {
        if !matches!(v.kind, Kind::Aux | Kind::Flow) {
            continue;
        }
        // a conversion the model declares is not a magic number
        if is_conversion(m, v) {
            continue;
        }
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut lits = Vec::new();
        literals(&e, None, &mut lits);
        // pi is a constant of mathematics, not a modeller's number
        for (value, near) in lits.into_iter().filter(|(x, _)| (x - std::f64::consts::PI).abs() > 1e-4).take(1) {
            let near_name = near.as_ref().map(|(n, _)| m.by_name(n).map(|x| x.name.clone()).unwrap_or_else(|| n.clone()));
            let mult = near.as_ref().map(|(_, mu)| *mu).unwrap_or(true);
            let new_name = match (&near_name, mult) {
                (Some(n), true) if value.abs() < 1.0 => format!("{n} fraction"),
                (Some(n), true) => format!("{n} factor"),
                (Some(n), false) => format!("{n} divisor"),
                (None, _) => format!("{} constant", v.name),
            };
            // the calendar (7, 12, 24, 52, 60, 365, 365.25), a percent as 100 and a unit conversion as 1000 are known numbers: a note, with the named constant to offer
            let percent = v.units.contains('%') || v.units.to_lowercase().contains("percent");
            let known = [7.0, 12.0, 24.0, 52.0, 60.0, 365.0, 365.25].contains(&value.abs()) || (value.abs() == 100.0 && percent) || (value.abs() == 1000.0 && near.is_some());
            let mut f = Finding::new(
                "magic-number",
                if known { Severity::Info } else { Severity::Warning },
                Some(&v.name),
                format!("“{}” has the number {} written into its equation (“{}”): give it a name, a unit and a place to be changed", v.name, super::num(value), v.equation.trim()),
            );
            f.source = "Modelling practice: no unnamed constants in equations (Sterman 2000, ch. 21)".into();
            if m.by_name(&expr::normalize(&new_name)).is_none() {
                if let Some(eq) = replace_literal(&v.equation, value, &new_name) {
                    f.fixes.push(Fix {
                        title: format!("Make the constant “{}” = {} and use it in {}", new_name, clean_factor(value), v.name),
                        ops: vec![
                            Op::AddVar { name: new_name.clone(), kind: "constant".into(), equation: clean_factor(value), units: if near.is_some() && value.abs() < 1.0 { "dmnl".into() } else { String::new() }, doc: format!("Was written as {} in the equation of {}", clean_factor(value), v.name), from: None, to: None, non_negative: false, layer: None },
                            Op::SetEquation { name: v.name.clone(), equation: eq },
                        ],
                        keeps_numbers: true, needs_decision: false,
                    });
                }
            }
            out.push(f);
        }
    }
}

/// The equation with its first number token equal to `value` replaced by `name`.
fn replace_literal(src: &str, value: f64, name: &str) -> Option<String> {
    let toks = expr::tokenize(src).ok()?;
    for (t, span) in toks {
        if let Tok::Num(n) = t {
            if n == value {
                return Some(format!("{}{}{}", &src[..span.start], name, &src[span.end..]));
            }
        }
    }
    None
}

// ------------------------------------------------------------------------------------- probing expressions in a run

/// Run the model with each expression added as a variable, and return their series.
pub(crate) fn probe(m: &Model, exprs: &[String]) -> Option<Vec<Vec<f64>>> {
    let mut copy = m.clone();
    let mut names = Vec::new();
    for (i, e) in exprs.iter().enumerate() {
        let id = copy.new_id();
        let name = format!("zz doctor probe {i}");
        let mut v = Var::new(id, &name, Kind::Aux, [0.0, 0.0]);
        v.equation = e.clone();
        copy.vars.push(v);
        names.push(name);
    }
    let c = sim::compile(&copy).ok()?;
    let run = sim::simulate(&c, &copy, "probe").ok()?;
    names.iter().map(|n| run.get(n).cloned()).collect()
}

fn denominators(e: &Expr, out: &mut Vec<Expr>) {
    match e {
        Expr::Bin(BinOp::Div, a, b) => {
            denominators(a, out);
            denominators(b, out);
            if !matches!(**b, Expr::Num(_)) {
                out.push((**b).clone());
            }
        }
        Expr::Bin(_, a, b) => {
            denominators(a, out);
            denominators(b, out);
        }
        Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => denominators(a, out),
        Expr::Call(f, args) => {
            // the safe divisions are the answer, not the problem
            if !matches!(f.as_str(), "ZIDZ" | "XIDZ") {
                args.iter().for_each(|a| denominators(a, out));
            }
        }
        _ => {}
    }
}

pub(crate) fn expr_text(e: &Expr, m: &Model) -> String {
    match e {
        Expr::Num(n) => clean_factor(*n),
        Expr::Var(n) => m.by_name(n).map(|v| v.name.clone()).unwrap_or_else(|| n.clone()),
        Expr::Neg(a) => format!("-{}", expr_text(a, m)),
        Expr::Not(a) => format!("NOT {}", expr_text(a, m)),
        Expr::Bin(op, a, b) => {
            let s = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Pow => "^",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::Eq => "=",
                BinOp::Ne => "<>",
                BinOp::And => ":AND:",
                BinOp::Or => ":OR:",
            };
            format!("({} {} {})", expr_text(a, m), s, expr_text(b, m))
        }
        Expr::Call(f, args) => format!("{}({})", f, args.iter().map(|a| expr_text(a, m)).collect::<Vec<_>>().join(", ")),
        Expr::LookupCall(n, a) => format!("{}({})", m.by_name(n).map(|v| v.name.clone()).unwrap_or_else(|| n.clone()), expr_text(a, m)),
        Expr::WithLookup(a, _) => format!("WITH LOOKUP({}, …)", expr_text(a, m)),
    }
}

fn zero_sites(m: &Model) -> Vec<(String, Expr)> {
    let mut sites: Vec<(String, Expr)> = Vec::new();
    for v in &m.vars {
        if matches!(v.kind, Kind::Lookup) {
            continue;
        }
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut d = Vec::new();
        denominators(&e, &mut d);
        for x in d {
            sites.push((v.name.clone(), x));
        }
    }
    sites
}

fn zero_report(m: &Model, sites: &[(String, Expr)], series: &[Vec<f64>], out: &mut Vec<Finding>) {
    let texts: Vec<String> = sites.iter().map(|(_, e)| expr_text(e, m)).collect();
    for (((owner, _), text), s) in sites.iter().zip(&texts).zip(series) {
        let hit = s.iter().position(|x| x.abs() < 1e-12 || !x.is_finite());
        if let Some(i) = hit {
            let mut f = Finding::new("zero-division", Severity::Warning, Some(owner), format!("“{owner}” divides by {text}, which reaches 0 during the run (step {i})"));
            f.source = "Numerical safety (Vensim and Stella manuals: ZIDZ and XIDZ)".into();
            if let Some(v) = m.by_name(&expr::normalize(owner)) {
                if let Ok(Expr::Bin(BinOp::Div, a, b)) = expr::parse(&v.equation) {
                    f.fixes.push(Fix {
                        title: format!("Use ZIDZ so that {owner} is 0 when the divisor is 0"),
                        ops: vec![Op::SetEquation { name: owner.clone(), equation: format!("ZIDZ({}, {})", expr_text(&a, m), expr_text(&b, m)) }],
                        keeps_numbers: false, needs_decision: false,
                    });
                }
            }
            out.push(f);
        }
    }
}

// ------------------------------------------------------------------------------------------------- time step checks

/// 1, 1/2, 1/4, … (and 2, 4, … which are exact too).
fn is_power_of_two(dt: f64) -> bool {
    dt > 0.0 && (dt.log2() - dt.log2().round()).abs() < 1e-12
}

/// The largest 1, 1/2, 1/4, … not above `x`.
fn largest_power_of_half_below(x: f64) -> f64 {
    let mut p = 1.0;
    while p > x && p > 1e-12 {
        p /= 2.0;
    }
    p
}

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

fn time_equality(m: &Model, out: &mut Vec<Finding>) {
    let dt = m.settings.time_step;
    if is_power_of_two(dt) {
        return;
    }
    for v in &m.vars {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut hit = false;
        walk(&e, &mut |x| {
            if let Expr::Bin(BinOp::Eq | BinOp::Ne, a, b) = x {
                if matches!(&**a, Expr::Var(n) if n == "time") || matches!(&**b, Expr::Var(n) if n == "time") {
                    hit = true;
                }
            }
        });
        if hit {
            let good = largest_power_of_half_below(dt);
            let mut f = Finding::new("time-equality", Severity::Warning, Some(&v.name), format!("“{}” tests Time for equality, but the time step {} is not a power of 1/2, so Time may never equal the value exactly (rounding)", v.name, super::num(dt)));
            f.source = "Numerical practice: an exact comparison of Time needs a binary time step".into();
            f.fixes.push(Fix { title: format!("Set the time step to {}", clean_factor(good)), ops: vec![Op::SetSetting { key: "time_step".into(), value: clean_factor(good) }], keeps_numbers: false, needs_decision: false });
            out.push(f);
        }
    }
}

/// The time constants the equations show: SMOOTH-like second arguments, and Stock / constant ratios in flows.
fn time_constants(m: &Model) -> Vec<(String, String, f64)> {
    let mut found = Vec::new();
    let value_of = |e: &Expr| -> Option<f64> {
        match e {
            Expr::Num(n) => Some(*n),
            Expr::Var(n) => {
                let v = m.by_name(n)?;
                if v.kind == Kind::Constant { v.equation.trim().replace(',', "").parse::<f64>().ok() } else { None }
            }
            _ => None,
        }
    };
    for v in &m.vars {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        walk(&e, &mut |x| match x {
            Expr::Call(name, args) if args.len() >= 2 && matches!(name.as_str(), "SMOOTH" | "SMOOTHI" | "SMOOTH3" | "SMOOTH3I" | "DELAY1" | "DELAY1I" | "DELAY3" | "DELAY3I" | "DELAY" | "DELAYMTR") => {
                if let Some(t) = value_of(&args[1]) {
                    if t > 0.0 {
                        found.push((v.name.clone(), name.clone(), t));
                    }
                }
            }
            Expr::Bin(BinOp::Div, a, b) if v.kind == Kind::Flow => {
                if let (Expr::Var(s), Some(t)) = (&**a, value_of(b)) {
                    if m.by_name(s).is_some_and(|x| x.kind == Kind::Stock) && t > 0.0 {
                        found.push((v.name.clone(), "stock / time".into(), t));
                    }
                }
            }
            _ => {}
        });
    }
    found
}

fn dt_too_large(m: &Model, base: Option<&sim::Run>, out: &mut Vec<Finding>) {
    let dt = m.settings.time_step;
    let tcs = time_constants(m);
    let Some((owner, what, tau)) = tcs.iter().min_by(|a, b| a.2.total_cmp(&b.2)).cloned() else { return };
    if dt > tau / 3.0 + 1e-12 {
        // confirmed only by reruns at DT/2 and DT/4; a step that changes no stock by 1% of its range is not reported
        let check = match base {
            Some(b) => dt_rerun_check(m, b),
            None => DtCheck::NotRun,
        };
        let moved = match check {
            DtCheck::Moved(name, rel) => Some((name, rel)),
            DtCheck::Fine => return,
            DtCheck::NotRun => None,
        };
        let good = largest_power_of_half_below(tau / 4.0);
        let mut f = Finding::new(
            "dt-too-large",
            Severity::Warning,
            Some(&owner),
            format!("The time step {} is more than a third of the smallest time constant of the model ({} in {} of “{owner}”): the integration will be inaccurate or unstable", super::num(dt), super::num(tau), what),
        );
        f.source = "Sterman (2000), Business Dynamics, §A.2: the time step should be at most a quarter to a half of the smallest time constant".into();
        match moved {
            Some((name, rel)) => f.message.push_str(&format!(". Rerun at a quarter of the step, “{name}” moves by {} of its range", pct(rel))),
            None => f.confirmed = false,
        }
        f.fixes.push(Fix { title: format!("Set the time step to {}", clean_factor(good)), ops: vec![Op::SetSetting { key: "time_step".into(), value: clean_factor(good) }], keeps_numbers: false, needs_decision: false });
        out.push(f);
    }
}

/// Text of a fraction as a percent, "not a number" for the values that are not.
pub(crate) fn pct(x: f64) -> String {
    if x.is_finite() { format!("{:.1}%", x * 100.0) } else { "more than any number can say (the rerun is not a number or is infinite)".into() }
}

/// How far the stocks of `b` (the same model at a shorter step) are from those of `a`, as a
/// fraction of the range of each stock over the run (the largest value if the stock is flat): the worst stock and the fraction.
pub(crate) fn stock_movement(m: &Model, a: &sim::Run, b: &sim::Run) -> Option<(String, f64)> {
    // for each saved time of `a`, the point of `b` at the same time (both lists are sorted)
    let mut at: Vec<Option<usize>> = Vec::with_capacity(a.times.len());
    let mut j = 0;
    for t in &a.times {
        while j + 1 < b.times.len() && b.times[j + 1] <= *t + 1e-9 * t.abs().max(1.0) {
            j += 1;
        }
        at.push(if (b.times.get(j).copied().unwrap_or(f64::NAN) - t).abs() <= 1e-9 * t.abs().max(1.0) { Some(j) } else { None });
    }
    let mut worst: Option<(String, f64)> = None;
    for v in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let (Some(x), Some(y)) = (a.get(&v.name), b.get(&v.name)) else { continue };
        let (mut lo, mut hi, mut big) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64);
        for z in x.iter().chain(y.iter()) {
            if z.is_finite() {
                lo = lo.min(*z);
                hi = hi.max(*z);
                big = big.max(z.abs());
            }
        }
        let range = if hi - lo > 1e-12 * big.max(1e-300) { hi - lo } else { big };
        let mut here: f64 = 0.0;
        for (i, p) in x.iter().enumerate() {
            let Some(q) = at.get(i).copied().flatten().and_then(|k| y.get(k)) else { continue };
            let d = if p.is_finite() && q.is_finite() { (p - q).abs() / range.max(1e-300) } else if p.to_bits() == q.to_bits() { 0.0 } else { f64::INFINITY };
            here = here.max(d);
        }
        if here.is_nan() {
            here = f64::INFINITY;
        }
        if worst.as_ref().is_none_or(|w| here > w.1) {
            worst = Some((v.name.clone(), here));
        }
    }
    worst
}

/// The same model at the step divided by `div`, run; None when it is too long to be worth it or does not run.
pub(crate) fn rerun_at(m: &Model, div: usize) -> Option<sim::Run> {
    let steps = (m.settings.final_time - m.settings.initial_time) / (m.settings.time_step / div as f64);
    if !(steps.is_finite()) || steps > 400_000.0 {
        return None;
    }
    let mut c = m.clone();
    c.settings.time_step = m.settings.time_step / div as f64;
    // the saved points follow the step, so that the runs line up
    let comp = sim::compile(&c).ok()?;
    sim::simulate(&comp, &c, "dt-rerun").ok()
}

/// What the reruns at DT/2 and DT/4 say about a time step that looks too long.
pub(crate) enum DtCheck {
    /// the reruns were made and no stock moves by 1% of its range
    Fine,
    /// the reruns were made: the stock that moves most at DT/4 and by how much of its range
    Moved(String, f64),
    /// a rerun could not be made (the run is too long or does not run): nothing is confirmed
    NotRun,
}

pub(crate) fn dt_rerun_check(m: &Model, base: &sim::Run) -> DtCheck {
    let (Some(half), Some(quarter)) = (rerun_at(m, 2), rerun_at(m, 4)) else { return DtCheck::NotRun };
    let (Some(h), Some(q)) = (stock_movement(m, base, &half), stock_movement(m, base, &quarter)) else { return DtCheck::NotRun };
    let worst = if q.1 >= h.1 { q } else { h };
    if worst.1 > 0.01 { DtCheck::Moved(worst.0, worst.1) } else { DtCheck::Fine }
}

fn dt_sensitivity(m: &Model, base: &sim::Run, out: &mut Vec<Finding>) {
    let Some(r2) = rerun_at(m, 2) else { return };
    let Some((name, rel)) = stock_movement(m, base, &r2).filter(|w| w.1 > 0.01) else { return };
    {
        let mut f = Finding::new("dt-sensitivity", Severity::Warning, Some(&name), format!("Halving the time step changes “{name}” by {} of its range (more than 1%): the answer depends on the step", pct(rel)));
        f.source = "Numerical practice: results should not change with DT (Sterman 2000, §A.2)".into();
        // halve the step until halving it again changes the stocks by 1% of their range or less
        let mut good = m.settings.time_step / 2.0;
        for _ in 0..6 {
            if !sensitive(m, good) {
                break;
            }
            good /= 2.0;
        }
        f.fixes.push(Fix { title: format!("Set the time step to {}", clean_factor(good)), ops: vec![Op::SetSetting { key: "time_step".into(), value: clean_factor(good) }], keeps_numbers: false, needs_decision: false });
        out.push(f);
    }
}

/// Does halving the time step `dt` still change a stock by more than 1% of its range?
fn sensitive(m: &Model, dt: f64) -> bool {
    // a very long run is not worth rerunning again and again
    if (m.settings.final_time - m.settings.initial_time) / dt > 2_500.0 {
        return false;
    }
    let mut a = m.clone();
    a.settings.time_step = dt;
    let Some(ra) = rerun_at(&a, 1) else { return false };
    let Some(rb) = rerun_at(&a, 2) else { return false };
    stock_movement(m, &ra, &rb).is_some_and(|w| w.1 > 0.01)
}

// ------------------------------------------------------------------------------------------------ lookups and ratios

type LookupSite = (String, String, String, f64, f64);

fn lookup_sites(m: &Model) -> Vec<LookupSite> {
    // (owner, lookup, argument text, x range)
    let mut sites: Vec<(String, String, String, f64, f64)> = Vec::new();
    for v in &m.vars {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        walk(&e, &mut |x| match x {
            Expr::LookupCall(n, a) => {
                if let Some(l) = m.by_name(n).filter(|l| l.table.len() >= 2) {
                    let lo = l.table.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                    let hi = l.table.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
                    sites.push((v.name.clone(), l.name.clone(), expr_text(a, m), lo, hi));
                }
            }
            Expr::WithLookup(a, t) if t.len() >= 2 => {
                let lo = t.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                let hi = t.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
                sites.push((v.name.clone(), "WITH LOOKUP".into(), expr_text(a, m), lo, hi));
            }
            _ => {}
        });
    }
    sites
}

fn lookup_report(sites: &[LookupSite], series: &[Vec<f64>], out: &mut Vec<Finding>) {
    for (s, vals) in sites.iter().zip(series) {
        let (lo, hi) = vals.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, x| (a.0.min(*x), a.1.max(*x)));
        if lo < s.3 - 1e-9 || hi > s.4 + 1e-9 {
            let mut f = Finding::new("lookup-outside", Severity::Warning, Some(&s.0), format!("The input of {} in “{}” ({}) runs from {} to {}, outside the table’s range {} to {}: the curve is held flat beyond its ends", s.1, s.0, s.2, super::num(lo), super::num(hi), super::num(s.3), super::num(s.4)));
            f.source = "Sterman (2000), §14.3: a table must cover the whole range its input can take".into();
            out.push(f);
        }
    }
}

/// One run with every divisor and every lookup input added as a variable.
fn probes(m: &Model, out: &mut Vec<Finding>) {
    let (z, l) = (zero_sites(m), lookup_sites(m));
    if z.is_empty() && l.is_empty() {
        return;
    }
    let mut texts: Vec<String> = z.iter().map(|(_, e)| expr_text(e, m)).collect();
    texts.extend(l.iter().map(|s| s.2.clone()));
    let Some(series) = probe(m, &texts) else { return };
    zero_report(m, &z, &series[..z.len()], out);
    lookup_report(&l, &series[z.len()..], out);
}

fn ratio_outside(m: &Model, base: &sim::Run, out: &mut Vec<Finding>) {
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Flow | Kind::Constant)) {
        let n = v.name.to_lowercase();
        if !["fraction", "share", "utilisation", "utilization", "ratio", "probability"].iter().any(|w| n.contains(w)) {
            continue;
        }
        let dimless = v.units.trim().is_empty() || crate::units::parse_unit_exact(&v.units).map(|u| u.is_dimensionless()).unwrap_or(false);
        if !dimless || v.units.contains('%') {
            continue;
        }
        let Some(s) = base.get(&v.name) else { continue };
        let (lo, hi) = s.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, x| (a.0.min(*x), a.1.max(*x)));
        if lo < -1e-9 || hi > 1.0 + 1e-9 {
            let mut f = Finding::new("ratio-outside", Severity::Warning, Some(&v.name), format!("“{}” is a fraction, share or ratio but runs from {} to {}, outside 0 to 1", v.name, super::num(lo), super::num(hi)));
            f.source = "Modelling practice: shares, fractions and probabilities lie in [0, 1]".into();
            out.push(f);
        }
    }
}

fn equilibrium(m: &Model, base: &sim::Run, out: &mut Vec<Finding>) {
    if !m.settings.starts_in_equilibrium {
        return;
    }
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let Some(lv) = base.get(&s.name) else { continue };
        if lv.len() < 2 {
            continue;
        }
        let rate = (lv[1] - lv[0]) / (base.times[1] - base.times[0]);
        let scale = lv.iter().fold(0.0f64, |a, x| a.max(x.abs())).max(1e-9);
        if rate.abs() > 1e-6 * scale {
            let mut f = Finding::new("equilibrium", Severity::Warning, Some(&s.name), format!("The model says it starts in equilibrium, but “{}” changes by {} per time unit at the start", s.name, super::num(rate)));
            f.source = "Sterman (2000), §21.4: test the model in equilibrium".into();
            out.push(f);
        }
    }
}

#[allow(dead_code)]
fn benchmarks(m: &Model, base: &sim::Run, out: &mut Vec<Finding>) {
    for c in &m.checks {
        if c.source.trim().is_empty() {
            continue;
        }
        let Some(s) = base.get(&c.var) else { continue };
        let x = match c.at {
            Some(t) => base.times.iter().position(|x| (*x - t).abs() < 1e-9).and_then(|i| s.get(i)).copied(),
            None => s.last().copied(),
        };
        if let Some(x) = x {
            if c.min.is_some_and(|lo| x < lo) || c.max.is_some_and(|hi| x > hi) {
                let mut f = Finding::new("benchmark", Severity::Warning, Some(&c.var), format!("“{}” is {} but the benchmark “{}” says {} to {} ({})", c.var, super::num(x), c.name, c.min.map(crate::fmt::fmt_num).unwrap_or_default(), c.max.map(crate::fmt::fmt_num).unwrap_or_default(), c.source));
                f.source = c.source.clone();
                out.push(f);
            }
        }
    }
}

// --------------------------------------------------------------------------------------------------- inferred units

/// The text of the unit an expression would have, from the declared texts of its variables. None if unknown.
fn unit_text(e: &Expr, texts: &HashMap<String, String>) -> Option<String> {
    let wrap = |t: &str| if t.contains('/') || t.contains('*') || t.contains(' ') { format!("({t})") } else { t.to_string() };
    match e {
        Expr::Var(n) => texts.get(n).cloned(),
        Expr::Neg(a) => unit_text(a, texts),
        Expr::Bin(BinOp::Add | BinOp::Sub, a, b) => unit_text(a, texts).or_else(|| unit_text(b, texts)),
        Expr::Bin(BinOp::Mul, a, b) => match (unit_text(a, texts), unit_text(b, texts)) {
            (Some(x), Some(y)) => Some(format!("{}*{}", wrap(&x), wrap(&y))),
            (Some(x), None) if matches!(**b, Expr::Num(_)) => Some(x),
            (None, Some(y)) if matches!(**a, Expr::Num(_)) => Some(y),
            _ => None,
        },
        Expr::Bin(BinOp::Div, a, b) => match (unit_text(a, texts), unit_text(b, texts)) {
            (Some(x), Some(y)) => Some(format!("{}/{}", wrap(&x), wrap(&y))),
            (Some(x), None) if matches!(**b, Expr::Num(_)) => Some(x),
            _ => None,
        },
        Expr::Call(f, args) if matches!(f.as_str(), "SMOOTH" | "SMOOTH3" | "SMOOTHI" | "SMOOTH3I" | "DELAY1" | "DELAY3" | "DELAY FIXED" | "MIN" | "MAX" | "ABS" | "CLAMP" | "STEP") => args.first().and_then(|a| unit_text(a, texts)),
        _ => None,
    }
}

fn inferred_units(m: &Model, _opts: &Options, out: &mut Vec<Finding>) {
    let mut texts: HashMap<String, String> = HashMap::new();
    for v in &m.vars {
        if !v.units.trim().is_empty() {
            texts.insert(expr::normalize(&v.name), v.units.trim().to_string());
        }
    }
    let mut inferred: Vec<(usize, String)> = Vec::new();
    let ctx = crate::units::UnitCtx::of(m);
    for _ in 0..10 {
        let mut changed = false;
        for (i, v) in m.vars.iter().enumerate() {
            let key = expr::normalize(&v.name);
            if texts.contains_key(&key) || !matches!(v.kind, Kind::Aux | Kind::Flow | Kind::Stock) {
                continue;
            }
            let Ok(e) = expr::parse(&v.equation) else { continue };
            if let Some(t) = unit_text(&e, &texts) {
                if crate::units::parse_unit_in(&t, &ctx).is_ok() {
                    texts.insert(key, t.clone());
                    inferred.push((i, t));
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    for (i, t) in inferred {
        let v = &m.vars[i];
        let mut f = Finding::new("units-inferred", Severity::Info, Some(&v.name), format!("“{}” has no units; its equation gives {}", v.name, t));
        f.source = "Dynamist units rules (inference by propagation)".into();
        f.fixes.push(Fix { title: format!("Declare “{}” in {}", v.name, t), ops: vec![Op::SetUnits { name: v.name.clone(), units: t }], keeps_numbers: true, needs_decision: false });
        out.push(f);
    }
}
