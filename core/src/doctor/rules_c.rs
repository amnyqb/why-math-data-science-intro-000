//! Stage C6, rules C: data and calibration. They read the calibration record of the model (`Model::calibration`, written
//! by the `set_calibration` operation): the fitted parameters with their plausible ranges, the data series with how well
//! each was matched, and the correlations of the estimates.
//!
//! C1: more than 5 parameters fitted at once (partitions by data series proposed, after Oliva's graph partition).
//! C2: pairs of estimates correlated above 0.9 (not identifiable separately). C3: a fit reported without Theil's shares.
//! C4: a calibrated value outside its plausible range. C5: a data series whose units or timing do not fit the model.

use std::collections::HashMap;

use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::expr;
use crate::model::{CalFit, Calibration, Kind};
use crate::ops::Op;
use crate::units;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "C1", stage: Stage::C6, about: "More than 5 parameters calibrated at once, where partitions by data series are possible.", f: c1 }),
        Box::new(FnRule { id: "C2", stage: Stage::C6, about: "Parameter estimates that are correlated above 0.9: they cannot be told apart.", f: c2 }),
        Box::new(FnRule { id: "C3", stage: Stage::C6, about: "A fit reported without Theil's decomposition of the error.", f: c3 }),
        Box::new(FnRule { id: "C4", stage: Stage::C6, about: "A calibrated value outside its plausible range.", f: c4 }),
        Box::new(FnRule { id: "C5", stage: Stage::C6, about: "A data series whose units or times do not fit the model.", f: c5 }),
    ]
}

fn cal<'a>(ctx: &'a Ctx) -> Option<&'a Calibration> {
    ctx.model.calibration.as_ref()
}

/// Which variables each variable reads (names normalised), for walking from a parameter to the data it reaches first.
fn readers(m: &crate::model::Model) -> HashMap<String, Vec<String>> {
    let mut users: HashMap<String, Vec<String>> = HashMap::new();
    for v in &m.vars {
        if let Ok(e) = expr::parse(&v.equation) {
            let mut r = Vec::new();
            expr::references(&e, &mut r);
            for x in r {
                users.entry(x).or_default().push(expr::normalize(&v.name));
            }
        }
        if v.kind == Kind::Flow {
            for id in [v.from, v.to].into_iter().flatten() {
                if let Some(s) = m.var(id) {
                    users.entry(expr::normalize(&v.name)).or_default().push(expr::normalize(&s.name));
                }
            }
        }
    }
    users
}

fn c1(ctx: &Ctx) -> Vec<Finding> {
    let Some(c) = cal(ctx) else { return Vec::new() };
    if c.params.len() <= 5 || c.fits.is_empty() {
        return Vec::new();
    }
    let users = readers(ctx.model);
    // each parameter belongs to the data series it reaches first (breadth first)
    let mut groups: Vec<(String, Vec<String>)> = c.fits.iter().map(|f| (f.var.clone(), Vec::new())).collect();
    for p in &c.params {
        let mut seen = vec![expr::normalize(&p.name)];
        let mut frontier = seen.clone();
        let mut home: Option<usize> = None;
        while home.is_none() && !frontier.is_empty() {
            let mut next = Vec::new();
            for n in &frontier {
                for u in users.get(n).into_iter().flatten() {
                    if !seen.contains(u) {
                        seen.push(u.clone());
                        next.push(u.clone());
                    }
                }
            }
            home = c.fits.iter().position(|f| next.contains(&expr::normalize(&f.var)));
            frontier = next;
        }
        if let Some(i) = home.or(if expr::normalize(&p.name) == expr::normalize(&c.fits[0].var) { Some(0) } else { None }) {
            groups[i].1.push(p.name.clone());
        }
    }
    let parts: Vec<String> = groups.iter().filter(|g| !g.1.is_empty()).map(|g| format!("{} fitted to {}", g.1.join(", "), g.0)).collect();
    let mut f = Finding::new("C1", Severity::Info, None, format!("{} parameters are calibrated at once. Calibrating in parts, each against the data it reaches first, is better conditioned: {}", c.params.len(), parts.join("; ")));
    f.source = "Oliva (2003); Homer (1983)".into();
    vec![f]
}

fn c2(ctx: &Ctx) -> Vec<Finding> {
    let Some(c) = cal(ctx) else { return Vec::new() };
    c.correlations
        .iter()
        .filter(|(_, _, r)| r.abs() > 0.9)
        .map(|(a, b, r)| {
            let mut f = Finding::new("C2", Severity::Warning, Some(a), format!("The estimates of “{a}” and “{b}” are correlated at {:.2}: the data cannot tell them apart. Fix one of them, or add data that separates them", r));
            f.source = "Statistics literature on identifiability".into();
            f
        })
        .collect()
}

fn c3(ctx: &Ctx) -> Vec<Finding> {
    let Some(c) = cal(ctx) else { return Vec::new() };
    let Some(base) = ctx.base() else { return Vec::new() };
    let mut out = Vec::new();
    for fit in c.fits.iter().filter(|f| f.shares.is_none()) {
        let mut f = Finding::new("C3", Severity::Info, Some(&fit.var), format!("The fit of “{}” is reported without the bias, variance and covariance shares of its error (Theil)", fit.var));
        f.source = "Sterman (1984)".into();
        if let (Some(sim), true) = (base.get(&fit.var), !fit.data.is_empty()) {
            let (mut a, mut s) = (Vec::new(), Vec::new());
            for p in &fit.data {
                if let Some(i) = base.times.iter().position(|t| (*t - p[0]).abs() < 1e-9) {
                    a.push(p[1]);
                    s.push(sim[i]);
                }
            }
            if a.len() >= 2 {
                let t = crate::verify::theil(&a, &s);
                let rmse = (a.iter().zip(&s).map(|(x, y)| (x - y).powi(2)).sum::<f64>() / a.len() as f64).sqrt();
                let mut cal2 = c.clone();
                if let Some(x) = cal2.fits.iter_mut().find(|x| x.var == fit.var) {
                    x.rmse = Some(rmse);
                    x.shares = Some([t.um, t.us, t.uc]);
                }
                f.fixes.push(Fix { title: format!("Add RMSE {} and the shares {:.2}, {:.2}, {:.2}", super::num(rmse), t.um, t.us, t.uc), ops: vec![Op::SetCalibration { calibration: cal2 }], keeps_numbers: true, needs_decision: false });
            }
        }
        out.push(f);
    }
    out
}

fn c4(ctx: &Ctx) -> Vec<Finding> {
    let Some(c) = cal(ctx) else { return Vec::new() };
    c.params
        .iter()
        .filter(|p| p.min.is_some_and(|lo| p.value < lo) || p.max.is_some_and(|hi| p.value > hi))
        .map(|p| {
            let (lo, hi) = (p.min.unwrap_or(f64::NEG_INFINITY), p.max.unwrap_or(f64::INFINITY));
            let mut f = Finding::new("C4", Severity::Warning, Some(&p.name), format!("The calibrated value of “{}” is {}, outside its plausible range {} to {}", p.name, super::num(p.value), super::num(lo), super::num(hi)));
            f.source = "Oliva (2003)".into();
            let mut cal2 = c.clone();
            if let Some(x) = cal2.params.iter_mut().find(|x| x.name == p.name) {
                x.min = Some(x.min.map_or(p.value, |m| m.min(p.value)));
                x.max = Some(x.max.map_or(p.value, |m| m.max(p.value)));
            }
            f.fixes.push(Fix { title: "Widen the range to include the value (and write down why)".into(), ops: vec![Op::SetCalibration { calibration: cal2 }], keeps_numbers: true, needs_decision: false });
            f
        })
        .collect()
}

fn c5(ctx: &Ctx) -> Vec<Finding> {
    let Some(c) = cal(ctx) else { return Vec::new() };
    let m = ctx.model;
    let ictx = units::UnitCtx::of(m);
    let mut out = Vec::new();
    for fit in &c.fits {
        let Some(v) = m.by_name(&expr::normalize(&fit.var)) else { continue };
        let want = units::parse_unit_in(&v.units, &ictx);
        let got = units::parse_unit_in(&fit.units, &ictx);
        let mismatch = match (&want, &got) {
            (Ok(a), Ok(b)) => (!v.units.trim().is_empty() && !fit.units.trim().is_empty()).then_some(a != b).unwrap_or(false),
            _ => false,
        };
        let outside = !fit.data.is_empty() && fit.data.iter().all(|p| p[0] < m.settings.initial_time - 1e-9 || p[0] > m.settings.final_time + 1e-9);
        let _ = CalFit::default;
        if mismatch || outside {
            let why = if mismatch { format!("the data are in {} but the model has {} in {}", fit.units.trim(), fit.var, v.units.trim()) } else { "all its times lie outside the run".to_string() };
            let mut f = Finding::new("C5", Severity::Error, Some(&fit.var), format!("The data series for “{}” does not fit the model: {why}", fit.var));
            f.source = "Dynamist units and time specification".into();
            if mismatch {
                if let (Ok(a), Ok(b)) = (&want, &got) {
                    if a.dims == b.dims {
                        let k = b.scale / a.scale;
                        let mut cal2 = c.clone();
                        if let Some(x) = cal2.fits.iter_mut().find(|x| x.var == fit.var) {
                            x.data.iter_mut().for_each(|p| p[1] *= k);
                            x.units = v.units.clone();
                        }
                        f.fixes.push(Fix { title: format!("Convert the data to {} (multiply by {})", v.units.trim(), super::fixes::clean_factor(k)), ops: vec![Op::SetCalibration { calibration: cal2 }], keeps_numbers: true, needs_decision: false });
                    }
                }
            }
            out.push(f);
        }
    }
    out
}
