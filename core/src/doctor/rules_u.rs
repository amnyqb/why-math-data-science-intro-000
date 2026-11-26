//! Stage C3, rules U that the units checks of `units_rules` do not cover: U9 (dated data against the model clock), U10 (a
//! constant that only makes units balance), U11 (a lookup with units on its input or output), U12 (a time constant with no time
//! unit). U1 to U8 are raised by `units_rules` (see `rule_of` for their codes).

use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::model::Kind;
use crate::ops::Op;
use crate::units;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "U9", stage: Stage::C3, about: "Dated data (a lookup read against Time in a model with a start date) with gaps, repeated times or times going backwards.", f: u9 }),
        Box::new(FnRule { id: "U10", stage: Stage::C3, about: "A constant of value 1 that carries units only to make the units balance.", f: u10 }),
        Box::new(FnRule { id: "U11", stage: Stage::C3, about: "A lookup table whose input or output carries units.", f: u11 }),
        Box::new(FnRule { id: "U12", stage: Stage::C3, about: "A time constant (adjustment time, delay, lifetime) declared without a time unit.", f: u12 }),
    ]
}

fn u9(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if m.settings.start.is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| v.kind == Kind::Lookup && v.table.len() >= 3) {
        // only a table that is data against time: its x values run over the model's own time range
        let (lo, hi) = (v.table.first().unwrap()[0], v.table.last().unwrap()[0]);
        if hi < m.settings.initial_time || lo > m.settings.final_time || !v.name.to_lowercase().contains("data") {
            continue;
        }
        let obs: Vec<crate::calendar::Obs> = v.table.iter().map(|p| crate::calendar::Obs { at: p[0], value: p[1] }).collect();
        let mut gaps: Vec<f64> = obs.windows(2).map(|w| w[1].at - w[0].at).filter(|d| *d > 0.0).collect();
        gaps.sort_by(|a, b| a.total_cmp(b));
        let step = gaps.get(gaps.len() / 2).copied().unwrap_or(1.0);
        let issues = crate::calendar::data_issues(&obs, step);
        if let Some(first) = issues.first() {
            let what = match first {
                crate::calendar::DataIssue::Gap(i, n) => format!("{n} missing step(s) after the point at time {}", super::num(obs[*i].at)),
                crate::calendar::DataIssue::Duplicate(i) => format!("two points at time {}", super::num(obs[*i].at)),
                crate::calendar::DataIssue::Backwards(i) => format!("time goes backwards at the point {}", i + 1),
            };
            let mut f = Finding::new("U9", Severity::Warning, Some(&v.name), format!("The data in “{}” do not line up with the model clock: {what}", v.name));
            f.source = "Dynamist units and time specification".into();
            if matches!(first, crate::calendar::DataIssue::Duplicate(_) | crate::calendar::DataIssue::Backwards(_)) {
                let mut pts = v.table.clone();
                pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
                pts.dedup_by(|a, b| a[0] == b[0]);
                f.fixes.push(Fix { title: "Sort the points by time and keep the first of any repeated time".into(), ops: vec![Op::SetTable { name: v.name.clone(), points: pts }], keeps_numbers: false, needs_decision: false });
            }
            out.push(f);
        }
    }
    out
}

fn u10(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let ictx = units::UnitCtx::of(m);
    m.vars
        .iter()
        .filter(|v| v.kind == Kind::Constant && v.equation.trim() == "1" && !v.units.contains('/'))
        .filter(|v| units::parse_unit_in(&v.units, &ictx).is_ok_and(|u| !u.is_dimensionless() && !(u.dims.len() == 1 && u.dims.get("s") == Some(&1))))
        .map(|v| {
            let mut f = Finding::new("U10", Severity::Warning, Some(&v.name), format!("“{}” is 1 {} and seems to be there only to make the units balance: name a real quantity (a normal value, an adjustment time)", v.name, v.units.trim()));
            f.source = "Morecroft (2007), Strategic Modelling and Business Dynamics".into();
            f
        })
        .collect()
}

fn u11(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let ictx = units::UnitCtx::of(m);
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| v.kind == Kind::Lookup) {
        let out_units = units::parse_unit_in(&v.units, &ictx).is_ok_and(|u| !u.is_dimensionless());
        // the input: the variable the lookup is called with
        let mut input_units = false;
        for u in m.vars.iter() {
            if let Ok(e) = crate::expr::parse(&u.equation) {
                fn find(e: &crate::expr::Expr, name: &str, out: &mut Vec<crate::expr::Expr>) {
                    use crate::expr::Expr;
                    match e {
                        Expr::LookupCall(n, a) => {
                            if n == name {
                                out.push((**a).clone());
                            }
                            find(a, name, out);
                        }
                        Expr::Bin(_, a, b) => {
                            find(a, name, out);
                            find(b, name, out);
                        }
                        Expr::Neg(a) | Expr::Not(a) | Expr::WithLookup(a, _) => find(a, name, out),
                        Expr::Call(_, args) => args.iter().for_each(|a| find(a, name, out)),
                        _ => {}
                    }
                }
                let mut args = Vec::new();
                find(&e, &crate::expr::normalize(&v.name), &mut args);
                for a in args {
                    if let crate::expr::Expr::Var(n) = a {
                        if m.by_name(&n).is_some_and(|x| units::parse_unit_in(&x.units, &ictx).is_ok_and(|u| !u.is_dimensionless())) {
                            input_units = true;
                        }
                    }
                }
            }
        }
        if out_units || input_units {
            let mut f = Finding::new("U11", Severity::Warning, Some(&v.name), format!("The lookup “{}” has units on its {}: divide the input by a normal value so that it is a ratio, and make the output a multiplier", v.name, if out_units { "output" } else { "input" }));
            f.source = "MIT D-4587: a table function example".into();
            out.push(f);
        }
    }
    out
}

fn u12(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let ictx = units::UnitCtx::of(m);
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| v.kind == Kind::Constant) {
        let n = v.name.to_lowercase();
        // the name ends in what it is: Adjustment Time, Delay, Average Lifetime
        if !["time", "delay", "lifetime", "duration", "period"].iter().any(|w| n.split_whitespace().last() == Some(*w)) {
            continue;
        }
        let ok = units::parse_unit_in(&v.units, &ictx).is_ok_and(|u| u.dims.len() == 1 && u.dims.get("s") == Some(&1));
        if !ok {
            let mut f = Finding::new("U12", Severity::Error, Some(&v.name), format!("“{}” is a time constant but is not declared in a unit of time ({})", v.name, if v.units.trim().is_empty() { "no units".to_string() } else { v.units.trim().to_string() }));
            f.source = "Sterman (2000), ch. 21".into();
            f.fixes.push(Fix { title: format!("Declare “{}” in {}", v.name, m.settings.time_units.trim()), ops: vec![Op::SetUnits { name: v.name.clone(), units: m.settings.time_units.trim().to_string() }], keeps_numbers: true, needs_decision: false });
            out.push(f);
        }
    }
    out
}
