//! Checks that live inside the model: limits you state about a variable ("shortage at the end
//! stays between 0 and 2000 MB") and a conservation check that every stock changed by exactly
//! what flowed in and out of it. They are tested against the current run, and a failure shows in
//! the Problems tab, so a model can say what "behaves sensibly" means and keep saying it after
//! every edit.

use serde::{Deserialize, Serialize};

use crate::expr;
use crate::model::{Kind, Method, Model, Rollup};
use crate::sim::Run;

/// When a limit applies.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckWhen {
    /// At every moment of the run.
    #[default]
    Always,
    /// At the last moment only.
    End,
}

/// What a check tests.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckKind {
    /// The variable stays between `min` and `max` (either may be left out).
    #[default]
    #[serde(alias = "bounds")]
    Bounds,
    /// The variable never goes below zero. A *test*: it fails when the run does, unlike a stock
    /// marked "never negative", which the engine holds at zero.
    #[serde(alias = "nonneg")]
    NonNeg,
    /// The variable stays within `tolerance` of a reference series (`reference`: time, value
    /// points; a single point means a constant, i.e. a flat trajectory).
    #[serde(alias = "reference")]
    Reference,
    /// The `stocks` taken together change only by `inflows` minus `outflows` over the run.
    #[serde(alias = "balance")]
    Balance,
    /// Each of the `stocks` (all stocks if none are named) changes over the run by exactly its inflows minus its outflows,
    /// within `tolerance` (relative, default 1e-9). Marks a stock as material: nothing may create or destroy it.
    #[serde(alias = "conservation")]
    Conservation,
}

/// A statement the model makes about its own results.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Check {
    pub name: String,
    /// The variable watched (bounds, non-negative, reference).
    #[serde(default)]
    pub var: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub when: CheckWhen,
    #[serde(default)]
    pub kind: CheckKind,
    /// Reference and balance: how far is still fine (absolute; relative to the size if `relative`).
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub relative: bool,
    /// Reference: (time, value) points, linear between, held beyond the ends.
    #[serde(default)]
    pub reference: Vec<[f64; 2]>,
    /// Balance: names of the stocks and of the flows that cross the boundary.
    #[serde(default)]
    pub stocks: Vec<String>,
    #[serde(default)]
    pub inflows: Vec<String>,
    #[serde(default)]
    pub outflows: Vec<String>,
    /// Bounds: test the variable at this time only (None = per `when`). A check with a `source` is a benchmark: a number
    /// the model must reproduce, with its citation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<f64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// What the check is: "" for an invariant (it must hold whatever the inputs), "validity" for the range the model is
    /// trusted in, or "promise" for a commitment that is meant to break outside the world the model describes. A breach of
    /// such a check under an extreme input is information (X6), not an error; in the base run it still fails.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
}

impl Check {
    /// A one-line description for lists.
    pub fn describe(&self) -> String {
        match self.kind {
            CheckKind::Bounds => format!("{} stays {}{}", self.var, match (self.min, self.max) {
                (Some(a), Some(b)) => format!("between {} and {}", num(a), num(b)),
                (Some(a), None) => format!("at least {}", num(a)),
                (None, Some(b)) => format!("at most {}", num(b)),
                _ => "finite".into(),
            }, if self.when == CheckWhen::End { " at the end" } else { "" }),
            CheckKind::NonNeg => format!("{} never goes below zero", self.var),
            CheckKind::Reference => format!("{} stays within {} of {}", self.var, self.tolerance.map(num).unwrap_or_else(|| "1e-6".into()), if self.reference.len() == 1 { format!("the constant {}", num(self.reference[0][1])) } else { format!("a reference of {} points", self.reference.len()) }),
            CheckKind::Conservation => format!("{} conserve material: each changes only by its flows", if self.stocks.is_empty() { "every stock".to_string() } else { self.stocks.join(", ") }),
            CheckKind::Balance => format!("{} change only by {} minus {}", self.stocks.join(" + "), if self.inflows.is_empty() { "nothing".into() } else { self.inflows.join(" + ") }, if self.outflows.is_empty() { "nothing".into() } else { self.outflows.join(" + ") }),
        }
    }
}

fn interpolate(points: &[[f64; 2]], t: f64) -> f64 {
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    match pts.as_slice() {
        [] => f64::NAN,
        [p] => p[1],
        _ => {
            if t <= pts[0][0] {
                return pts[0][1];
            }
            for w in pts.windows(2) {
                if t <= w[1][0] {
                    let span = w[1][0] - w[0][0];
                    return if span <= 0.0 { w[1][1] } else { w[0][1] + (w[1][1] - w[0][1]) * (t - w[0][0]) / span };
                }
            }
            pts[pts.len() - 1][1]
        }
    }
}

/// One thing that did not hold. `var` is the variable to jump to, when there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub var: Option<u64>,
    pub message: String,
}

fn num(x: f64) -> String {
    crate::fmt::fmt_num(x)
}

/// The limits that were broken in this run.
pub fn limits(m: &Model, run: &Run) -> Vec<Failure> {
    let mut out = Vec::new();
    for c in &m.checks {
        let label = if c.name.trim().is_empty() { c.var.clone() } else { c.name.clone() };
        if c.kind == CheckKind::Balance {
            out.extend(balance_failure(m, run, c, &label));
            continue;
        }
        if c.kind == CheckKind::Conservation {
            let tol = c.tolerance.unwrap_or(if m.settings.method == Method::Euler { 1e-9 } else { 2e-2 });
            let named: Vec<String> = c.stocks.iter().map(|n| expr::normalize(n)).collect();
            for f in stock_balance(m, run, tol, |s| named.is_empty() || named.contains(&expr::normalize(&s.name))) {
                out.push(Failure { var: f.var, message: format!("Check “{label}”: {}", f.message) });
            }
            continue;
        }
        let Some(v) = m.by_name(&expr::normalize(&c.var)) else {
            out.push(Failure { var: None, message: format!("Check “{label}”: there is no variable called “{}”", c.var) });
            continue;
        };
        let Some(series) = run.get(&c.var) else {
            out.push(Failure { var: Some(v.id), message: format!("Check “{label}”: “{}” has no results yet", c.var) });
            continue;
        };
        let at: Vec<(usize, f64)> = match c.when {
            _ if c.at.is_some() => {
                // a benchmark: the variable at that time (the nearest saved time)
                let t = c.at.unwrap();
                run.times.iter().position(|x| (*x - t).abs() <= 1e-9 * t.abs().max(1.0) + 1e-9).and_then(|i| series.get(i).map(|x| vec![(i, *x)])).unwrap_or_default()
            }
            CheckWhen::Always => series.iter().cloned().enumerate().collect(),
            CheckWhen::End => series.last().map(|x| vec![(series.len() - 1, *x)]).unwrap_or_default(),
        };
        if c.kind == CheckKind::Reference {
            let tol = c.tolerance.unwrap_or(1e-6);
            let bad = at.iter().find(|(i, x)| {
                let r = interpolate(&c.reference, run.times.get(*i).copied().unwrap_or(0.0));
                let allowed = if c.relative { tol * r.abs().max(1e-12) } else { tol };
                !x.is_finite() || !r.is_finite() || (x - r).abs() > allowed
            });
            if let Some(&(i, x)) = bad {
                let t = run.times.get(i).copied().unwrap_or(0.0);
                let r = interpolate(&c.reference, t);
                out.push(Failure { var: Some(v.id), message: format!("Check “{label}”: {} should be within {}{} of the reference ({}) at {} {}, but is {}", c.var, num(tol), if c.relative { " (relative)" } else { "" }, num(r), run.time_units, num(t), num(x)) });
            }
            continue;
        }
        let (min, max) = if c.kind == CheckKind::NonNeg { (Some(-1e-9), None) } else { (c.min, c.max) };
        let bad = at.iter().find(|(_, x)| !x.is_finite() || min.is_some_and(|lo| *x < lo) || max.is_some_and(|hi| *x > hi));
        if c.kind == CheckKind::NonNeg {
            if let Some(&(i, x)) = bad {
                let when = match c.when {
                    CheckWhen::End => "at the end".to_string(),
                    CheckWhen::Always => format!("at {} {}", run.time_units, num(run.times.get(i).copied().unwrap_or(0.0))),
                };
                out.push(Failure { var: Some(v.id), message: format!("Check “{label}”: {} should never go below zero, but is {} {when}", c.var, num(x)) });
            }
            continue;
        }
        if let Some(&(i, x)) = bad {
            let range = match (c.min, c.max) {
                (Some(lo), Some(hi)) => format!("between {} and {}", num(lo), num(hi)),
                (Some(lo), None) => format!("at least {}", num(lo)),
                (None, Some(hi)) => format!("at most {}", num(hi)),
                (None, None) => "finite".into(),
            };
            let when = match c.when {
                CheckWhen::End => "at the end".to_string(),
                CheckWhen::Always => format!("at {} {}", run.time_units, num(run.times.get(i).copied().unwrap_or(0.0))),
            };
            let src = if c.source.trim().is_empty() { String::new() } else { format!(" (benchmark source: {})", c.source.trim()) };
            out.push(Failure { var: Some(v.id), message: format!("Check “{label}”: {} should be {range}, but is {} {when}{src}", c.var, num(x)) });
        }
    }
    out
}

/// The balance test: the chosen stocks together change only by the boundary flows.
fn balance_failure(m: &Model, run: &Run, c: &Check, label: &str) -> Option<Failure> {
    let series = |names: &[String]| -> Result<Vec<&Vec<f64>>, String> {
        names.iter().map(|n| run.get(n).ok_or_else(|| format!("“{n}” has no results"))).collect()
    };
    let (stocks, ins, outs) = match (series(&c.stocks), series(&c.inflows), series(&c.outflows)) {
        (Ok(a), Ok(b), Ok(d)) => (a, b, d),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return Some(Failure { var: None, message: format!("Check “{label}”: {e}") }),
    };
    if stocks.is_empty() || run.times.len() < 2 {
        return Some(Failure { var: None, message: format!("Check “{label}”: a balance needs at least one stock and a run") });
    }
    let euler = m.settings.method == Method::Euler;
    let net = |i: usize| -> f64 { ins.iter().map(|s| s[i]).sum::<f64>() - outs.iter().map(|s| s[i]).sum::<f64>() };
    let total = |i: usize| -> f64 { stocks.iter().map(|s| s[i]).sum() };
    let (mut moved, mut scale) = (0.0, (0..run.times.len()).map(|i| total(i).abs()).fold(0.0, f64::max));
    for i in 0..run.times.len() - 1 {
        let dt = run.times[i + 1] - run.times[i];
        let step = if euler { net(i) * dt } else { 0.5 * (net(i) + net(i + 1)) * dt };
        moved += step;
        scale = scale.max(step.abs());
    }
    let change = total(run.times.len() - 1) - total(0);
    let tol = c.tolerance.unwrap_or(if euler { 1e-6 } else { 2e-2 });
    let allowed = if c.relative || c.tolerance.is_none() { tol * scale.max(1e-9) } else { tol };
    ((change - moved).abs() > allowed || !(change - moved).is_finite()).then(|| Failure {
        var: m.by_name(&expr::normalize(&c.stocks[0])).map(|v| v.id),
        message: format!("Check “{label}”: {} changed by {} but the boundary flows add up to {}", c.stocks.join(" + "), num(change), num(moved)),
    })
}

/// Every stock should change by exactly what flowed in minus what flowed out. Averaged stocks and
/// stocks filled by flows of a different set are left out, since the roll-up deliberately breaks the
/// balance there. (A "never negative" stock is not exempt: the engine reduces its outflows instead
/// of clamping the level, so it conserves too.)
pub fn conservation(m: &Model, run: &Run) -> Vec<Failure> {
    // Euler steps are exact; the higher-order methods use mid-step values the run does not keep
    let tol = if m.settings.method == Method::Euler { 1e-6 } else { 2e-2 };
    stock_balance(m, run, tol, |_| true)
}

/// The balance of each selected stock: change over the run against the integral of its flows.
fn stock_balance(m: &Model, run: &Run, tol: f64, select: impl Fn(&crate::model::Var) -> bool) -> Vec<Failure> {
    let mut out = Vec::new();
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        if !select(s) || s.rollup == Rollup::Average {
            continue;
        }
        let flows: Vec<_> = m.vars.iter().filter(|f| f.kind == Kind::Flow && (f.from == Some(s.id) || f.to == Some(s.id))).collect();
        if flows.is_empty() || flows.iter().any(|f| f.set != s.set || f.rollup == Rollup::Average) {
            continue;
        }
        let Some(level) = run.get(&s.name) else { continue };
        let series: Vec<(&crate::model::Var, &Vec<f64>)> = flows.iter().filter_map(|f| run.get(&f.name).map(|x| (*f, x))).collect();
        if series.len() != flows.len() || level.len() != run.times.len() || level.len() < 2 {
            continue;
        }
        let net = |i: usize| -> f64 { series.iter().map(|(f, x)| if f.to == Some(s.id) && f.from != Some(s.id) { x[i] } else if f.from == Some(s.id) && f.to != Some(s.id) { -x[i] } else { 0.0 }).sum() };
        let mut moved = 0.0;
        let mut scale: f64 = level.iter().fold(0.0, |a, x| a.max(x.abs()));
        for i in 0..level.len() - 1 {
            let dt = run.times[i + 1] - run.times[i];
            let step = if m.settings.method == Method::Euler { net(i) * dt } else { 0.5 * (net(i) + net(i + 1)) * dt };
            moved += step;
            scale = scale.max(step.abs());
        }
        let change = level[level.len() - 1] - level[0];
        let gap = (change - moved).abs();
        if gap > tol * scale.max(1e-9) && gap.is_finite() {
            out.push(Failure {
                var: Some(s.id),
                message: format!("Stock “{}” changed by {} but its flows add up to {}: something other than its flows moves it", s.name, num(change), num(moved)),
            });
        }
    }
    out
}

/// Everything that failed, limits first.
pub fn all(m: &Model, run: &Run) -> Vec<Failure> {
    let mut v = limits(m, run);
    v.extend(conservation(m, run));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Var;

    fn tank() -> Model {
        let mut m = Model::default();
        m.settings.final_time = 10.0;
        m.settings.time_step = 0.5;
        let mut s = Var::new(1, "Tank", Kind::Stock, [0.0, 0.0]);
        s.equation = "100".into();
        let mut f = Var::new(2, "Burn", Kind::Flow, [0.0, 0.0]);
        f.equation = "8".into();
        f.from = Some(1);
        let mut g = Var::new(3, "Fill", Kind::Flow, [0.0, 0.0]);
        g.equation = "TIME".into();
        g.to = Some(1);
        m.vars = vec![s, f, g];
        m.next_id = 10;
        m
    }

    fn run(m: &Model) -> Run {
        let c = crate::sim::compile(m).unwrap();
        crate::sim::simulate(&c, m, "x").unwrap()
    }

    /// S = 1 (never negative) asks to drain 2 per step into T; only what S holds may leave.
    fn drain_model() -> Model {
        let mut m = Model::default();
        m.settings.initial_time = 0.0;
        m.settings.final_time = 3.0;
        m.settings.time_step = 1.0;
        let mut s = Var::new(1, "S", Kind::Stock, [0.0, 0.0]);
        s.equation = "1".into();
        s.non_negative = true;
        let mut t = Var::new(2, "T", Kind::Stock, [0.0, 0.0]);
        t.equation = "0".into();
        let mut f = Var::new(3, "Drain", Kind::Flow, [0.0, 0.0]);
        f.equation = "2".into();
        f.from = Some(1);
        f.to = Some(2);
        m.vars = vec![s, t, f];
        m.next_id = 10;
        m
    }

    #[test]
    fn nonneg_conserves() {
        let m = drain_model();
        let r = run(&m);
        let (s, t, d) = (r.get("S").unwrap(), r.get("T").unwrap(), r.get("Drain").unwrap());
        for i in 0..s.len() {
            assert!((s[i] + t[i] - 1.0).abs() < 1e-12, "S+T at step {i} = {}", s[i] + t[i]);
        }
        assert_eq!((s[1], t[1]), (0.0, 1.0));
        assert_eq!((d[0], d[1], d[2]), (1.0, 0.0, 0.0), "flows are reported as constrained: {d:?}");
        assert_eq!(r.constraints.len(), 1, "{:?}", r.constraints);
        let c = &r.constraints[0];
        assert_eq!((c.time, c.stock.as_str(), c.requested, c.allowed), (0.0, "S", 2.0, 1.0));
    }

    #[test]
    fn conservation_check() {
        let mut m = drain_model();
        m.checks = vec![Check { name: "Material".into(), kind: CheckKind::Conservation, stocks: vec!["S".into(), "T".into()], ..Default::default() }];
        assert!(all(&m, &run(&m)).is_empty(), "{:?}", all(&m, &run(&m)));
        // a run in which something other than the flows moved T is caught, with the check's name
        let mut r = run(&m);
        let n = r.series["t"].len();
        r.series.get_mut("t").unwrap()[n - 1] += 0.5;
        let f = limits(&m, &r);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("Material") && f[0].message.contains("T"), "{}", f[0].message);
    }

    #[test]
    fn a_plain_stock_conserves_with_every_method() {
        for method in [Method::Euler, Method::RungeKutta2, Method::RungeKutta4] {
            let mut m = tank();
            m.settings.method = method;
            let r = run(&m);
            assert!(conservation(&m, &r).is_empty(), "{method:?}: {:?}", conservation(&m, &r));
        }
    }

    #[test]
    fn a_stock_moved_by_something_else_is_caught() {
        let m = tank();
        let mut r = run(&m);
        let level = r.series.get_mut("tank").unwrap();
        let n = level.len();
        level[n - 1] += 25.0; // as if something outside its flows had added to it
        let f = conservation(&m, &r);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("Tank"));
    }

    #[test]
    fn limits_say_what_broke_and_when() {
        let mut m = tank();
        m.checks = vec![
            Check { name: "Stays above 80".into(), var: "Tank".into(), min: Some(80.0), max: None, when: CheckWhen::Always, ..Default::default() },
            Check { name: "Ends high".into(), var: "Tank".into(), min: Some(500.0), max: None, when: CheckWhen::End, ..Default::default() },
            Check { name: "Fine".into(), var: "Tank".into(), min: Some(-1e6), max: Some(1e6), when: CheckWhen::Always, ..Default::default() },
            Check { name: "Typo".into(), var: "Tnak".into(), min: Some(0.0), max: None, when: CheckWhen::End, ..Default::default() },
        ];
        let f = limits(&m, &run(&m));
        assert_eq!(f.len(), 3, "{f:?}");
        assert!(f[0].message.contains("at least 80") && f[0].message.contains(" at "), "{}", f[0].message);
        assert!(f[1].message.contains("at the end"), "{}", f[1].message);
        assert!(f[2].message.contains("no variable"), "{}", f[2].message);
    }

    #[test]
    fn a_nonneg_test_fails_where_a_clamp_would_hide_it() {
        let mut m = tank(); // Tank starts at 100, burns 8 a day, fills TIME a day over 10 days: it dips but stays above 0
        m.checks = vec![Check { name: "Never dry".into(), var: "Tank".into(), kind: CheckKind::NonNeg, ..Default::default() }];
        assert!(limits(&m, &run(&m)).is_empty());
        m.vars[1].equation = "20".into(); // burns 20 a day: runs below zero
        let f = limits(&m, &run(&m));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("never go below zero"), "{}", f[0].message);
    }

    #[test]
    fn a_reference_test_holds_a_series_to_a_band_and_a_single_point_means_flat() {
        let mut m = tank();
        m.vars[2].equation = "8".into(); // fill = burn: the tank stays flat at 100
        m.checks = vec![
            Check { name: "Flat".into(), var: "Tank".into(), kind: CheckKind::Reference, reference: vec![[0.0, 100.0]], tolerance: Some(1e-9), ..Default::default() },
            Check { name: "Follows".into(), var: "Tank".into(), kind: CheckKind::Reference, reference: vec![[0.0, 100.0], [10.0, 100.0]], tolerance: Some(0.01), relative: true, ..Default::default() },
        ];
        assert!(limits(&m, &run(&m)).is_empty());
        m.vars[2].equation = "9".into(); // now it climbs one a day
        let f = limits(&m, &run(&m));
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f[0].message.contains("reference"), "{}", f[0].message);
    }

    #[test]
    fn a_balance_test_covers_a_whole_chain_of_stocks() {
        let mut m = Model::default();
        m.settings.final_time = 10.0;
        m.settings.time_step = 1.0;
        let mk = |id: u64, name: &str, kind, eq: &str| { let mut v = Var::new(id, name, kind, [0.0, 0.0]); v.equation = eq.into(); v };
        let (mut a, mut b) = (mk(1, "Upstream", Kind::Stock, "50"), mk(2, "Downstream", Kind::Stock, "10"));
        a.non_negative = false;
        let mut produce = mk(3, "Production", Kind::Flow, "5");
        produce.to = Some(1);
        let mut ship = mk(4, "Shipping", Kind::Flow, "3");
        ship.from = Some(1);
        ship.to = Some(2);
        let mut sell = mk(5, "Sales", Kind::Flow, "2");
        sell.from = Some(2);
        b.non_negative = false;
        m.vars = vec![a, b, produce, ship, sell];
        m.next_id = 10;
        m.checks = vec![Check { name: "Chain balance".into(), kind: CheckKind::Balance, stocks: vec!["Upstream".into(), "Downstream".into()], inflows: vec!["Production".into()], outflows: vec!["Sales".into()], ..Default::default() }];
        assert!(limits(&m, &run(&m)).is_empty(), "{:?}", limits(&m, &run(&m)));
        // forget that sales leave the chain: the balance no longer closes
        m.checks[0].outflows.clear();
        let f = limits(&m, &run(&m));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("boundary flows"), "{}", f[0].message);
        // and an unknown name is explained
        m.checks[0].inflows = vec!["Nope".into()];
        assert!(limits(&m, &run(&m))[0].message.contains("no results"));
    }
}
