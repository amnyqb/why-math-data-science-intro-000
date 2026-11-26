//! Sobol variance-based importance: how much of the variance of a goal does each parameter explain?
//!
//! Saltelli's scheme with two random matrices A and B over the unit cube (scaled to each lever's
//! declared range) and, for each parameter j, the hybrid AB_j (A with column j taken from B).
//! First-order index `S1_j = mean(B (f(AB_j) - f(A))) / Var` (Saltelli et al. 2010) and total index
//! `ST_j = mean((f(A) - f(AB_j))^2) / (2 Var)` (Jansen 1999). Cost: `N (k + 2)` runs. Confidence
//! intervals by bootstrap over the base samples. Convergence: the indices from `N` base samples are
//! compared with those from `2N`.

use serde::Serialize;

use crate::manifest::Goal;
use crate::model::Model;
use crate::screen::{evaluate, Param, Rng};

#[derive(Clone, Debug)]
pub struct SobolSpec {
    /// base samples N (the run uses 2N, the first N shared with the smaller design)
    pub samples: usize,
    pub seed: u64,
    pub bootstrap: usize,
    /// largest difference of any index between N and 2N that counts as converged
    pub converge_tol: f64,
    pub workers: usize,
    pub keep_design: bool,
}

impl Default for SobolSpec {
    fn default() -> Self {
        SobolSpec { samples: 512, seed: 1, bootstrap: 100, converge_tol: 0.05, workers: 4, keep_design: false }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SobolIndex {
    pub param: String,
    pub s1: f64,
    pub s1_conf: f64,
    pub st: f64,
    pub st_conf: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct GoalSobol {
    pub goal: String,
    pub variance: f64,
    pub indices: Vec<SobolIndex>,
    /// largest change of an index between N and 2N base samples
    pub max_change_n_to_2n: f64,
    pub converged: bool,
    /// Kendall tau between the ranking of the total indices from N and from 2N base samples, over the
    /// parameters whose total index is at least 0.01 in either (1 when fewer than two such parameters):
    /// does the ranking hold when N doubles?
    pub rank_tau_n_to_2n: f64,
    /// sum of S1: well below 1 means interactions, above 1 means estimator noise
    pub sum_s1: f64,
    pub failed_runs: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sobol {
    pub params: Vec<Param>,
    pub samples: usize,
    pub runs: usize,
    pub goals: Vec<GoalSobol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<serde_json::Value>,
}

/// Rows per base sample: A, AB_1..AB_k, B.
fn design(k: usize, n: usize, seed: u64) -> Vec<Vec<f64>> {
    let mut rng = Rng(seed | 1);
    let mut rows = Vec::with_capacity(n * (k + 2));
    for _ in 0..n {
        let a: Vec<f64> = (0..k).map(|_| rng.next()).collect();
        let b: Vec<f64> = (0..k).map(|_| rng.next()).collect();
        rows.push(a.clone());
        for j in 0..k {
            let mut ab = a.clone();
            ab[j] = b[j];
            rows.push(ab);
        }
        rows.push(b);
    }
    rows
}

fn variance(v: &[f64]) -> f64 {
    let m = v.iter().sum::<f64>() / v.len() as f64;
    v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64
}

/// S1 and ST for every parameter from the groups `idx` (indices of base samples) of outputs `y` laid out as in `design`.
fn indices(y: &[f64], k: usize, idx: &[usize]) -> (f64, Vec<f64>, Vec<f64>) {
    let g = k + 2;
    // centre the outputs (as SALib does): the estimator of S1 is exact for any shift, but its noise is not
    let all: Vec<f64> = idx.iter().flat_map(|&i| y[i * g..(i + 1) * g].iter().cloned()).collect();
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    let y: Vec<f64> = y.iter().map(|v| v - mean).collect();
    let y = &y[..];
    let both: Vec<f64> = idx.iter().flat_map(|&i| [y[i * g], y[i * g + k + 1]]).collect();
    let var = variance(&both);
    let n = idx.len() as f64;
    let (mut s1, mut st) = (Vec::new(), Vec::new());
    for j in 0..k {
        let s: f64 = idx.iter().map(|&i| y[i * g + k + 1] * (y[i * g + 1 + j] - y[i * g])).sum::<f64>() / n;
        let t: f64 = idx.iter().map(|&i| (y[i * g] - y[i * g + 1 + j]).powi(2)).sum::<f64>() / (2.0 * n);
        s1.push(if var > 0.0 { s / var } else { 0.0 });
        st.push(if var > 0.0 { t / var } else { 0.0 });
    }
    (var, s1, st)
}

pub fn sobol(model: &Model, params: &[Param], goals: &[Goal], spec: &SobolSpec) -> Result<Sobol, String> {
    let k = params.len();
    if k == 0 {
        return Err("no parameters".into());
    }
    if let Some(p) = params.iter().find(|p| !(p.min < p.max)) {
        return Err(format!("“{}” has no range (min {} not below max {})", p.name, p.min, p.max));
    }
    let n = spec.samples.max(8);
    let pts = design(k, 2 * n, spec.seed);
    let ys = evaluate(model, params, goals, &pts, spec.workers);
    let mut out = Vec::new();
    for (gi, g) in goals.iter().enumerate() {
        let y: Vec<f64> = ys.iter().map(|r| r[gi]).collect();
        // a base sample with any failed run is dropped as a whole
        let gsz = k + 2;
        let ok: Vec<usize> = (0..2 * n).filter(|&i| y[i * gsz..(i + 1) * gsz].iter().all(|v| v.is_finite())).collect();
        let failed = y.iter().filter(|v| !v.is_finite()).count();
        if ok.len() < 8 {
            return Err(format!("goal “{}”: too few complete base samples ({}) — the model fails over much of the parameter space", g.name, ok.len()));
        }
        let (var, s1, st) = indices(&y, k, &ok);
        let half: Vec<usize> = ok.iter().cloned().filter(|&i| i < n).collect();
        let (_, s1h, sth) = indices(&y, k, &half);
        let change = (0..k).map(|j| (s1[j] - s1h[j]).abs().max((st[j] - sth[j]).abs())).fold(0.0, f64::max);
        // bootstrap
        let mut rng = Rng(spec.seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let (mut bs1, mut bst): (Vec<Vec<f64>>, Vec<Vec<f64>>) = (vec![Vec::new(); k], vec![Vec::new(); k]);
        for _ in 0..spec.bootstrap {
            let pick: Vec<usize> = (0..ok.len()).map(|_| ok[rng.below(ok.len())]).collect();
            let (_, a, b) = indices(&y, k, &pick);
            for j in 0..k {
                bs1[j].push(a[j]);
                bst[j].push(b[j]);
            }
        }
        let conf = |v: &Vec<f64>| if v.len() > 1 { 1.96 * variance(v).sqrt() * (v.len() as f64 / (v.len() as f64 - 1.0)).sqrt() } else { f64::NAN };
        let ix: Vec<SobolIndex> = (0..k).map(|j| SobolIndex { param: params[j].name.clone(), s1: s1[j], s1_conf: conf(&bs1[j]), st: st[j], st_conf: conf(&bst[j]) }).collect();
        // ranking stability: two indices closer than their confidence half-widths are tied (no ranking to keep)
        let mattering: Vec<usize> = (0..k).filter(|&j| st[j].max(sth[j]) >= 0.01).collect();
        let rank_tau = if mattering.len() < 2 {
            1.0
        } else {
            let e: Vec<f64> = mattering.iter().map(|&j| ix[j].st_conf.max(0.0)).collect();
            let t = crate::screen::kendall_tau_tied(&mattering.iter().map(|&j| sth[j]).collect::<Vec<_>>(), &mattering.iter().map(|&j| st[j]).collect::<Vec<_>>(), &e, &e);
            if t.is_nan() { 1.0 } else { t }
        };
        out.push(GoalSobol { goal: g.name.clone(), variance: var, sum_s1: s1.iter().sum(), indices: ix, max_change_n_to_2n: change, converged: change <= spec.converge_tol, rank_tau_n_to_2n: rank_tau, failed_runs: failed });
    }
    let design = spec.keep_design.then(|| serde_json::json!({"x": pts, "y": ys, "group_size": k + 2, "goals": goals.iter().map(|g| g.name.clone()).collect::<Vec<_>>()}));
    Ok(Sobol { params: params.to_vec(), samples: n, runs: pts.len(), goals: out, design })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::{goals_of, params_of};

    fn formula(f: &str, xs: &[&str], lo: f64, hi: f64) -> Model {
        let mut m = crate::screen::tests_formula_model(f, xs);
        for l in &mut m.manifest.levers {
            l.min = lo;
            l.max = hi;
        }
        m
    }

    #[test]
    fn ishigami_indices_match_the_analytic_values() {
        let m = formula("SIN(X1) + 7 * SIN(X2) ^ 2 + 0.1 * X3 ^ 4 * SIN(X1)", &["X1", "X2", "X3"], -std::f64::consts::PI, std::f64::consts::PI);
        let (p, g) = (params_of(&m, &[]).unwrap(), goals_of(&m, &[]).unwrap());
        let s = sobol(&m, &p, &g, &SobolSpec { samples: 4096, ..Default::default() }).unwrap();
        let (s1, st) = ([0.3139, 0.4424, 0.0], [0.5576, 0.4424, 0.2437]);
        for (j, i) in s.goals[0].indices.iter().enumerate() {
            assert!((i.s1 - s1[j]).abs() < i.s1_conf.max(0.03), "S1[{j}] = {} ± {}", i.s1, i.s1_conf);
            assert!((i.st - st[j]).abs() < i.st_conf.max(0.03), "ST[{j}] = {} ± {}", i.st, i.st_conf);
        }
        assert!(s.goals[0].converged, "{}", s.goals[0].max_change_n_to_2n);
        assert_eq!(s.runs, 2 * 4096 * 5);
    }

    #[test]
    fn an_additive_model_has_s1_equal_st_and_shares_by_variance() {
        // Y = 3 X1 + X2, X uniform 0..1: variances 9/12 and 1/12, so S1 = 0.9 and 0.1
        let m = formula("3 * X1 + X2", &["X1", "X2"], 0.0, 1.0);
        let (p, g) = (params_of(&m, &[]).unwrap(), goals_of(&m, &[]).unwrap());
        let s = sobol(&m, &p, &g, &SobolSpec { samples: 2048, ..Default::default() }).unwrap();
        let i = &s.goals[0].indices;
        assert!((i[0].s1 - 0.9).abs() < 0.1 && (i[1].s1 - 0.1).abs() < 0.05, "{i:?}");
        assert!((i[0].st - i[0].s1).abs() < 0.06, "{i:?}");
    }
}
