//! The polarity of causal links and loops from the numbers, not from the syntax of the equations.
//!
//! A link's sign is the sign of the partial derivative of the output with respect to the input at the operating point
//! (`sim::link_signs`): at every saved time of the base run the output is evaluated from the same state with the input
//! raised by a small relative step. `MIN`, `MAX`, `IF THEN ELSE`, lookups and `1/(1+x)` shapes therefore get the sign they
//! really have where the model is. A loop's sign at a time is the product of its links' signs at that time; where it
//! changes during the run the loop is labelled "switches" and the times are reported.

use std::collections::HashMap;

use crate::cld::FoundLoop;
use crate::model::{CldData, Kind, Model};
use crate::sim;

#[derive(Clone, Debug, PartialEq)]
pub struct LoopPolarity {
    /// "reinforcing", "balancing", "switches", or "inactive" (a link of the loop has zero gain at every time of the run)
    pub label: &'static str,
    /// the times (first saved time with the new sign) at which a "switches" loop changes sign
    pub switches_at: Vec<f64>,
    /// sign at the start: +1 reinforcing, -1 balancing
    pub first_sign: i8,
}

/// The polarity of each of `found` (loops of the diagram `d`, built from `m`). Falls back to the drawn signs when the
/// model does not run.
pub fn loop_polarities(m: &Model, d: &CldData, found: &[FoundLoop]) -> Vec<LoopPolarity> {
    let text = |id: u64| d.nodes.iter().find(|n| n.id == id).map(|n| n.text.clone()).unwrap_or_default();
    // the distinct links used by the loops
    let mut order: Vec<u64> = Vec::new();
    for l in found {
        for k in &l.links {
            if !order.contains(k) {
                order.push(*k);
            }
        }
    }
    let link = |id: u64| d.links.iter().find(|l| l.id == id).unwrap();
    let pairs: Vec<(String, String)> = order.iter().map(|k| (text(link(*k).from), text(link(*k).to))).collect();
    let drawn = |k: u64| -> i8 { if link(k).polarity == "-" { -1 } else { 1 } };
    let numeric = sim::link_signs(m, &pairs).ok();
    let mut series: HashMap<u64, Vec<i8>> = HashMap::new();
    // links whose measured effect is zero at every saved time (a weight of 0 or 1 that switches the term off)
    let mut dead: Vec<u64> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    if let Some((t, all)) = numeric {
        times = t;
        for (k, s) in order.iter().zip(all) {
            let to_is_stock = m.by_name(&crate::expr::normalize(&text(link(*k).to))).is_some_and(|v| v.kind == Kind::Stock);
            // a flow's push on its stock is structural (+ into, - out of); everything else is measured
            let filled = if to_is_stock { None } else { fill(&s) };
            // a function with a hidden state (SMOOTH, DELAY, TREND…) shows no instantaneous effect of its input: that is delay, not zero gain
            let stateful = m.by_name(&crate::expr::normalize(&text(link(*k).to))).is_some_and(|v| {
                let eq = v.equation.to_uppercase();
                ["SMOOTH", "DELAY", "TREND", "FORECAST", "NPV", "SAMPLE", "INTEG", "LEVELISED", "PAYBACK", "LOAN", "TAX"].iter().any(|w| eq.contains(w))
            });
            if !to_is_stock && filled.is_none() && !s.is_empty() && !stateful {
                dead.push(*k);
            }
            series.insert(*k, filled.unwrap_or_else(|| vec![drawn(*k); times.len().max(1)]));
        }
    }
    found
        .iter()
        .map(|l| {
            let n = times.len().max(1);
            if l.links.iter().any(|k| dead.contains(k)) {
                return LoopPolarity { label: "inactive", switches_at: Vec::new(), first_sign: 0 };
            }
            let sign_at = |i: usize| -> i8 { l.links.iter().map(|k| series.get(k).map(|s| s[i.min(s.len() - 1)]).unwrap_or_else(|| drawn(*k))).product() };
            let first = sign_at(0);
            let mut switches = Vec::new();
            let mut cur = first;
            for i in 1..n {
                let s = sign_at(i);
                if s != cur {
                    switches.push(times.get(i).copied().unwrap_or(0.0));
                    cur = s;
                }
            }
            let label = if !switches.is_empty() { "switches" } else if first > 0 { "reinforcing" } else { "balancing" };
            LoopPolarity { label, switches_at: switches, first_sign: first }
        })
        .collect()
}

/// Carry the last known sign over the times where the link had no effect (zero), and back-fill the start.
/// None if the link never had an effect.
fn fill(s: &[i8]) -> Option<Vec<i8>> {
    let first = s.iter().copied().find(|x| *x != 0)?;
    let mut last = first;
    Some(s.iter().map(|x| { if *x != 0 { last = *x; } last }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Var;

    fn mk(id: u64, name: &str, kind: Kind, eq: &str) -> Var {
        let mut v = Var::new(id, name, kind, [0.0, 0.0]);
        v.equation = eq.into();
        v
    }

    /// Stock grows by Gain = Stock * MIN(Rate, Limit - Stock)/Limit-style cap: with MIN the cap loop is balancing.
    fn capped() -> Model {
        let mut m = Model::default();
        m.settings.final_time = 40.0;
        m.settings.time_step = 0.25;
        let s = mk(1, "Pop", Kind::Stock, "1");
        let mut g = mk(2, "Growth", Kind::Flow, "Pop * Rate");
        g.to = Some(1);
        let r = mk(3, "Rate", Kind::Aux, "MIN(0.2, 0.01 * (Limit - Pop))");
        let l = mk(4, "Limit", Kind::Constant, "100");
        m.vars = vec![s, g, r, l];
        m.next_id = 50;
        crate::ops::sync_links(&mut m);
        m
    }

    #[test]
    fn a_min_limit_loop_is_balancing_when_the_limit_binds_and_switches_before() {
        let mut m = capped();
        let d = crate::cld::from_model(&mut m);
        let found = crate::cld::find_loops(&d);
        let pol = loop_polarities(&m, &d, &found);
        // the loop through Rate: at the start MIN picks the constant 0.2 (no effect of Pop), later the limit term binds
        let through_rate = found.iter().position(|f| f.nodes.iter().any(|n| d.nodes.iter().any(|x| x.id == *n && x.text == "Rate"))).unwrap();
        let p = &pol[through_rate];
        assert_eq!(p.label, "balancing", "{p:?}");
        // the direct loop Growth -> Pop -> Growth is reinforcing
        let direct = found.iter().position(|f| f.nodes.len() == 2).unwrap();
        assert_eq!(pol[direct].label, "reinforcing");
    }

    /// Gap = Target - S closed through `(1 - w) * MIN(Gap, cap)`: with w = 1 the link has zero gain, so the loop is inactive.
    fn weighted(w: &str) -> Model {
        let mut m = Model::default();
        m.settings.final_time = 10.0;
        m.settings.time_step = 0.25;
        let s = mk(1, "Level", Kind::Stock, "10");
        let mut f = mk(2, "Close", Kind::Flow, "(1 - Weight) * MIN(Gap, 50) / 5");
        f.to = Some(1);
        let g = mk(3, "Gap", Kind::Aux, "Target - Level");
        let t = mk(4, "Target", Kind::Constant, "100");
        let wv = mk(5, "Weight", Kind::Constant, w);
        m.vars = vec![s, f, g, t, wv];
        m.next_id = 50;
        crate::ops::sync_links(&mut m);
        m
    }

    fn weighted_labels(w: &str) -> Vec<&'static str> {
        let mut m = weighted(w);
        let d = crate::cld::from_model(&mut m);
        let found = crate::cld::find_loops(&d);
        assert!(!found.is_empty());
        loop_polarities(&m, &d, &found).iter().map(|p| p.label).collect()
    }

    #[test]
    fn a_link_with_zero_gain_makes_its_loop_inactive_and_a_real_weight_keeps_it_balancing() {
        assert!(weighted_labels("1").iter().all(|l| *l == "inactive"), "{:?}", weighted_labels("1"));
        assert!(weighted_labels("0.5").iter().all(|l| *l == "balancing"), "{:?}", weighted_labels("0.5"));
    }

    #[test]
    fn f12_ignores_an_inactive_loop() {
        for w in ["1", "0.5"] {
            let m = weighted(w);
            let r = crate::doctor::check(&m, &crate::doctor::Options { stage: Some(crate::doctor::Stage::C3), ..Default::default() });
            assert!(!r.findings.iter().any(|f| f.rule == "F12"), "w={w}: {:?}", r.findings);
        }
    }

    #[test]
    fn a_loop_whose_sign_changes_along_the_run_says_when() {
        // Gap = Target - Level, Push = Level * (Level - 5): Level raises Push when Level > 5 and lowers it below 5
        let mut m = Model::default();
        m.settings.final_time = 10.0;
        m.settings.time_step = 0.125;
        let s = mk(1, "Level", Kind::Stock, "1");
        let mut f = mk(2, "Push", Kind::Flow, "0.3 * Level * (Level - 5) * -1 + 1.5 * Level");
        f.to = Some(1);
        m.vars = vec![s, f];
        m.next_id = 50;
        crate::ops::sync_links(&mut m);
        let d = crate::cld::from_model(&mut m);
        let found = crate::cld::find_loops(&d);
        let pol = loop_polarities(&m, &d, &found);
        assert_eq!(pol.len(), 1);
        assert_eq!(pol[0].label, "switches", "{:?}", pol[0]);
        assert!(!pol[0].switches_at.is_empty());
    }
}
