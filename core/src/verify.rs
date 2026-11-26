//! F4 `verify_reduction`: does the reduced model still answer the questions the full model answers?
//!
//! Compared on the declared goals, for the base run and across an ensemble of parameter draws (the
//! levers both models share, drawn uniformly over their ranges, seeded):
//! * error of each goal's output series: normalised RMSE and Theil's decomposition (bias `um`,
//!   variance `us`, covariance `uc`, summing to 1) with the full model as the reference;
//! * behaviour mode of the output in each run (see [`mode_of`]), compared run by run;
//! * agreement of the goal values across the ensemble (Kendall tau) and of the ranking of the
//!   protected policies (manifest scenarios named by a goal with `protected_ranking`);
//! * the reduced model's own checks.
//!
//! Pass or fail against thresholds that are part of the frozen configuration.

use serde::Serialize;

use crate::manifest::{goal_value, Goal};
use crate::model::Model;
use crate::screen::{kendall_tau, Param, Rng};

#[derive(Clone, Debug)]
pub struct VerifySpec {
    pub ensemble: usize,
    pub seed: u64,
    /// largest acceptable normalised RMSE of an output series (base run and ensemble mean)
    pub max_nrmse: f64,
    /// smallest acceptable share of runs with the same behaviour mode
    pub min_mode_agreement: f64,
    /// smallest acceptable Kendall tau of goal values across the ensemble
    pub min_goal_tau: f64,
}

impl Default for VerifySpec {
    fn default() -> Self {
        VerifySpec { ensemble: 50, seed: 1, max_nrmse: 0.05, min_mode_agreement: 0.9, min_goal_tau: 0.9 }
    }
}

/// Behaviour mode of a series, from features of its shape.
pub fn mode_of(s: &[f64]) -> &'static str {
    let n = s.len();
    if n < 5 {
        return "flat";
    }
    let (lo, hi) = s.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
    let range = hi - lo;
    let scale = s.iter().map(|x| x.abs()).fold(0.0, f64::max).max(1e-12);
    if range <= 1e-6 * scale {
        return "flat";
    }
    // turning points that matter: swings of at least 5% of the range
    let thr = 0.05 * range;
    let mut ext: Vec<f64> = vec![s[0]];
    let mut dir = 0i8;
    for &v in &s[1..] {
        let last = *ext.last().unwrap();
        if dir == 0 {
            if (v - last).abs() >= thr {
                ext.push(v);
                dir = if v > last { 1 } else { -1 };
            }
        } else if (v - last) * f64::from(dir) > 0.0 {
            *ext.last_mut().unwrap() = v;
        } else if (v - last).abs() >= thr {
            ext.push(v);
            dir = -dir;
        }
    }
    let turns = ext.len().saturating_sub(2); // interior extrema
    let (first, last) = (s[0], s[n - 1]);
    match turns {
        0 => {
            if (last - first).abs() < 0.1 * range {
                return "flat";
            }
            // monotone: curvature tells exponential growth, goal seeking, or S-shape
            let q = n / 4;
            let (d1, d2, d3) = (s[q] - s[0], s[2 * q] - s[q], s[n - 1] - s[3 * q]);
            let sign = if last > first { 1.0 } else { -1.0 };
            let (a, b, c) = (d1 * sign, d2 * sign, d3 * sign);
            if a < b && b < c {
                if sign > 0.0 { "exponential growth" } else { "accelerating decline" }
            } else if a > b && b > c {
                if sign > 0.0 { "goal seeking (rise)" } else { "goal seeking (fall)" }
            } else if a < b && b > c {
                if sign > 0.0 { "s-shaped growth" } else { "s-shaped decline" }
            } else if sign > 0.0 {
                "growth"
            } else {
                "decline"
            }
        }
        1 => {
            let peak = ext[1];
            let base = if (peak - first).abs() > 0.0 { first } else { last };
            if (last - base).abs() < 0.3 * (peak - base).abs() { "overshoot and settle" } else { "overshoot and collapse" }
        }
        _ => {
            let amps: Vec<f64> = ext.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
            // the first and last swings are partial: compare the first and last full ones when there are enough
            let inner: &[f64] = if amps.len() >= 4 { &amps[1..amps.len() - 1] } else { &amps };
            let (a, b) = (inner[0], *inner.last().unwrap());
            if b < 0.7 * a {
                "damped oscillation"
            } else if b > 1.4 * a {
                "growing oscillation"
            } else {
                "sustained oscillation"
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Theil {
    pub nrmse: f64,
    pub um: f64,
    pub us: f64,
    pub uc: f64,
}

/// Error of `sim` against `actual` (same length): NRMSE by the actual's range, and Theil's decomposition.
pub fn theil(actual: &[f64], sim: &[f64]) -> Theil {
    let n = actual.len().min(sim.len()) as f64;
    let (ma, ms) = (actual.iter().sum::<f64>() / n, sim.iter().sum::<f64>() / n);
    let mse = actual.iter().zip(sim).map(|(a, s)| (s - a).powi(2)).sum::<f64>() / n;
    let (sa, ss) = ((actual.iter().map(|a| (a - ma).powi(2)).sum::<f64>() / n).sqrt(), (sim.iter().map(|s| (s - ms).powi(2)).sum::<f64>() / n).sqrt());
    let cov = actual.iter().zip(sim).map(|(a, s)| (a - ma) * (s - ms)).sum::<f64>() / n;
    let r = if sa > 0.0 && ss > 0.0 { cov / (sa * ss) } else { 1.0 };
    let (lo, hi) = actual.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
    let range = if hi - lo > 0.0 { hi - lo } else { ma.abs().max(1.0) };
    if mse == 0.0 {
        return Theil { nrmse: 0.0, um: 0.0, us: 0.0, uc: 0.0 };
    }
    Theil { nrmse: mse.sqrt() / range, um: (ms - ma).powi(2) / mse, us: (ss - sa).powi(2) / mse, uc: 2.0 * (1.0 - r) * sa * ss / mse }
}

#[derive(Clone, Debug, Serialize)]
pub struct GoalVerdict {
    pub goal: String,
    pub base: Theil,
    pub ensemble_nrmse_mean: f64,
    pub ensemble_nrmse_max: f64,
    pub mode_agreement: f64,
    pub goal_value_tau: f64,
    pub failed_runs: usize,
    pub pass_error: bool,
    pub pass_modes: bool,
    pub pass_tau: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyVerdict {
    pub goal: String,
    pub policies: Vec<String>,
    pub full: Vec<f64>,
    pub reduced: Vec<f64>,
    pub tau: f64,
    pub pass: bool,
    pub skipped: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Verdict {
    pub pass: bool,
    pub goals: Vec<GoalVerdict>,
    pub policies: Vec<PolicyVerdict>,
    pub checks_failed: usize,
    pub shared_parameters: Vec<String>,
    pub runs: usize,
}

fn run_with_values(m: &Model, over: &[(String, String)]) -> Option<crate::sim::Run> {
    crate::runner::run_with(m, over).ok()
}

pub fn verify(full: &Model, reduced: &Model, goals: &[Goal], spec: &VerifySpec) -> Result<Verdict, String> {
    let have = |m: &Model, n: &str| m.by_name(&crate::expr::normalize(n)).is_some();
    // every lever of the full model is drawn; the reduced model gets the values of those it still has, so that a
    // parameter fixed at its nominal value shows an error wherever it matters
    let all: Vec<Param> = full.manifest.levers.iter().map(|l| Param { name: l.name.clone(), min: l.min, max: l.max }).collect();
    let params: Vec<Param> = all.iter().filter(|p| have(reduced, &p.name)).cloned().collect();
    for g in goals {
        if !have(reduced, &g.output) {
            return Err(format!("the reduced model has no “{}”, the output of goal “{}”", g.output, g.name));
        }
    }
    let mut rng = Rng(spec.seed | 1);
    let mut draws: Vec<Vec<(String, String)>> = vec![Vec::new()]; // the first is the base run
    for _ in 0..spec.ensemble {
        draws.push(all.iter().map(|p| (p.name.clone(), format!("{}", p.min + rng.next() * (p.max - p.min)))).collect());
    }
    let mut per_goal: Vec<(Vec<f64>, Vec<f64>, Vec<f64>, usize, usize)> = goals.iter().map(|_| (Vec::new(), Vec::new(), Vec::new(), 0, 0)).collect(); // fullvals, redvals, nrmse, modes equal, runs
    let mut base: Vec<Option<Theil>> = vec![None; goals.len()];
    let mut failed = vec![0usize; goals.len()];
    for (di, d) in draws.iter().enumerate() {
        let dr: Vec<(String, String)> = d.iter().filter(|(k, _)| have(reduced, k)).cloned().collect();
        let (a, b) = (run_with_values(full, d), run_with_values(reduced, &dr));
        for (gi, g) in goals.iter().enumerate() {
            let (Some(a), Some(b)) = (&a, &b) else {
                failed[gi] += 1;
                continue;
            };
            let (Some(sa), Some(sb)) = (a.get(&g.output), b.get(&g.output)) else {
                failed[gi] += 1;
                continue;
            };
            let t = theil(sa, sb);
            if di == 0 {
                base[gi] = Some(t.clone());
            }
            let (va, vb) = (goal_value(a, g), goal_value(b, g));
            let e = &mut per_goal[gi];
            if let (Some(va), Some(vb)) = (va, vb) {
                e.0.push(va);
                e.1.push(vb);
            }
            e.2.push(t.nrmse);
            e.4 += 1;
            if mode_of(sa) == mode_of(sb) {
                e.3 += 1;
            }
        }
    }
    let mut gv = Vec::new();
    for (gi, g) in goals.iter().enumerate() {
        let (fv, rv, nr, same, runs) = &per_goal[gi];
        let tau = kendall_tau(fv, rv);
        let base_t = base[gi].clone().ok_or_else(|| format!("goal “{}”: the base run failed in one of the models", g.name))?;
        let (mean, max) = (nr.iter().sum::<f64>() / nr.len().max(1) as f64, nr.iter().cloned().fold(0.0, f64::max));
        let agree = *same as f64 / (*runs).max(1) as f64;
        gv.push(GoalVerdict {
            goal: g.name.clone(),
            pass_error: base_t.nrmse <= spec.max_nrmse && mean <= spec.max_nrmse,
            base: base_t,
            ensemble_nrmse_mean: mean,
            ensemble_nrmse_max: max,
            mode_agreement: agree,
            goal_value_tau: tau,
            failed_runs: failed[gi],
            pass_modes: agree >= spec.min_mode_agreement,
            pass_tau: tau.is_nan() || tau >= spec.min_goal_tau,
        });
    }
    // protected policy rankings
    let mut pv = Vec::new();
    for g in goals.iter().filter(|g| g.protected_ranking) {
        let mut vals_f = Vec::new();
        let mut vals_r = Vec::new();
        let mut skipped = None;
        for p in &g.policies {
            let Some(sc) = full.manifest.scenarios.iter().find(|s| s.name == *p) else {
                skipped = Some(format!("policy “{p}” is not a scenario of the full model"));
                break;
            };
            let over: Vec<(String, String)> = sc.values.iter().map(|(k, v)| (k.clone(), format!("{v}"))).collect();
            if over.iter().any(|(k, _)| !have(reduced, k)) {
                skipped = Some(format!("policy “{p}” sets a lever the reduced model no longer has"));
                break;
            }
            match (run_with_values(full, &over), run_with_values(reduced, &over)) {
                (Some(a), Some(b)) => match (goal_value(&a, g), goal_value(&b, g)) {
                    (Some(x), Some(y)) => {
                        vals_f.push(crate::manifest::goal_score(g, x));
                        vals_r.push(crate::manifest::goal_score(g, y));
                    }
                    _ => {
                        skipped = Some("a goal value could not be read".into());
                        break;
                    }
                },
                _ => {
                    skipped = Some("a policy run failed".into());
                    break;
                }
            }
        }
        let tau = if skipped.is_none() { kendall_tau(&vals_f, &vals_r) } else { f64::NAN };
        pv.push(PolicyVerdict { goal: g.name.clone(), policies: g.policies.clone(), full: vals_f, reduced: vals_r, tau, pass: skipped.is_none() && tau == 1.0, skipped });
    }
    let checks_failed = match crate::runner::run_with(reduced, &[]) {
        Ok(r) => crate::checks::all(reduced, &r).len(),
        Err(_) => 1,
    };
    let pass = gv.iter().all(|g| g.pass_error && g.pass_modes && g.pass_tau) && pv.iter().all(|p| p.pass) && checks_failed == 0;
    Ok(Verdict { pass, goals: gv, policies: pv, checks_failed, shared_parameters: params.iter().map(|p| p.name.clone()).collect(), runs: draws.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_are_recognised() {
        let t: Vec<f64> = (0..200).map(|i| i as f64 * 0.1).collect();
        let m = |f: &dyn Fn(f64) -> f64| mode_of(&t.iter().map(|x| f(*x)).collect::<Vec<_>>());
        assert_eq!(m(&|_| 3.0), "flat");
        assert_eq!(m(&|x| (0.2 * x).exp()), "exponential growth");
        assert_eq!(m(&|x| 100.0 * (1.0 - (-x / 4.0).exp())), "goal seeking (rise)");
        assert_eq!(m(&|x| 1000.0 / (1.0 + 99.0 * (-0.5 * (x - 0.0)).exp())), "s-shaped growth");
        assert_eq!(m(&|x| (x * 0.8).sin() * (-x / 8.0).exp()), "damped oscillation");
        assert_eq!(m(&|x| (x * 0.8).sin()), "sustained oscillation");
        assert_eq!(m(&|x| (x * 0.8).sin() * (x / 8.0).exp()), "growing oscillation");
        assert_eq!(m(&|x| x * (-x / 6.0).exp()), "overshoot and collapse");
    }

    #[test]
    fn theil_parts_sum_to_one_and_identify_bias() {
        let a: Vec<f64> = (0..100).map(|i| (i as f64 * 0.2).sin()).collect();
        let shifted: Vec<f64> = a.iter().map(|x| x + 0.3).collect();
        let t = theil(&a, &shifted);
        assert!((t.um + t.us + t.uc - 1.0).abs() < 1e-9 && t.um > 0.99, "{t:?}");
        assert_eq!(theil(&a, &a).nrmse, 0.0);
    }
}
