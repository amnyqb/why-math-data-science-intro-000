//! F1 `screen`: which of the model's parameters matter for the declared goals?
//!
//! Morris elementary effects (Morris 1991, Campolongo et al. 2007 for |effect|). Each parameter is
//! scaled to 0..1 over the range its manifest lever declares; `r` trajectories of `k + 1` points
//! each move one parameter at a time by `delta = p / (2 (p - 1))`. Per parameter and goal:
//! `mu` (mean effect), `mu_star` (mean absolute effect, the importance) and `sigma` (sample standard
//! deviation, non-zero when the effect depends on the value of the parameter or of others).
//!
//! The same run serves every goal, so the cost is `r (k + 1)` model runs however many goals there are.
//! A second design of twice the size shares the first one's trajectories, so the ranking of the
//! small design can be compared with the ranking of the large one (convergence).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde::Serialize;

use crate::manifest::{goal_value, Goal};
use crate::model::Model;

/// A parameter with the range it is varied over.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Param {
    pub name: String,
    pub min: f64,
    pub max: f64,
}

/// The parameters of a study: the manifest's levers, or the named subset of them.
pub fn params_of(m: &Model, only: &[String]) -> Result<Vec<Param>, String> {
    let all: Vec<Param> = m.manifest.levers.iter().map(|l| Param { name: l.name.clone(), min: l.min, max: l.max }).collect();
    if all.is_empty() {
        return Err("the manifest declares no levers, so there is no range to vary any parameter over; declare levers with a min and max (set_manifest or dyn manifest --draft)".into());
    }
    if only.is_empty() {
        return Ok(all);
    }
    let mut out = Vec::new();
    for n in only {
        let k = crate::expr::normalize(n);
        match all.iter().find(|p| crate::expr::normalize(&p.name) == k) {
            Some(p) => out.push(p.clone()),
            None => return Err(format!("“{n}” is not a lever of the manifest, so it has no declared range")),
        }
    }
    Ok(out)
}

/// The goals of a study: the manifest's goals, or the named subset.
pub fn goals_of(m: &Model, only: &[String]) -> Result<Vec<Goal>, String> {
    if m.manifest.goals.is_empty() {
        return Err("the manifest declares no goals; declare what the study is after (set_manifest, goals)".into());
    }
    if only.is_empty() {
        return Ok(m.manifest.goals.clone());
    }
    only.iter().map(|n| m.manifest.goals.iter().find(|g| g.name == *n).cloned().ok_or_else(|| format!("“{n}” is not a goal of the manifest"))).collect()
}

/// Seeded generator (xorshift), shared by all designs.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f64) as usize).min(n - 1)
    }
}

/// One trajectory: `k + 1` points in 0..1, and for each step `(parameter, signed step)`.
#[derive(Clone, Debug)]
pub struct Trajectory {
    pub points: Vec<Vec<f64>>,
    pub steps: Vec<(usize, f64)>,
}

/// `r` trajectories over `k` parameters with `p` (even) levels.
pub fn morris_design(k: usize, r: usize, p: usize, seed: u64) -> Vec<Trajectory> {
    let delta = p as f64 / (2.0 * (p as f64 - 1.0));
    let mut rng = Rng(seed | 1);
    (0..r)
        .map(|_| {
            // a base point on the grid from which a step of `delta` stays inside 0..1 in at least one direction
            let levels: Vec<f64> = (0..p).map(|i| i as f64 / (p as f64 - 1.0)).collect();
            let mut x: Vec<f64> = (0..k).map(|_| levels[rng.below(p)]).collect();
            let mut order: Vec<usize> = (0..k).collect();
            for i in (1..k).rev() {
                order.swap(i, rng.below(i + 1));
            }
            let mut points = vec![x.clone()];
            let mut steps = Vec::new();
            for &i in &order {
                let (up, down) = (x[i] + delta <= 1.0 + 1e-12, x[i] - delta >= -1e-12);
                let dir = match (up, down) {
                    (true, true) => if rng.next() < 0.5 { 1.0 } else { -1.0 },
                    (true, false) => 1.0,
                    (false, true) => -1.0,
                    // delta larger than any gap: step to the far end
                    (false, false) => if x[i] < 0.5 { 1.0 } else { -1.0 },
                };
                let nx = (x[i] + dir * delta).clamp(0.0, 1.0);
                steps.push((i, nx - x[i]));
                x[i] = nx;
                points.push(x.clone());
            }
            Trajectory { points, steps }
        })
        .collect()
}

/// Run the model at each unit-cube point (scaled to the parameters' ranges) and read every goal.
/// Result: one row per point, one column per goal; NaN where a run failed or a goal could not be read.
pub fn evaluate(model: &Model, params: &[Param], goals: &[Goal], points: &[Vec<f64>], workers: usize) -> Vec<Vec<f64>> {
    let keys: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
    let out: Mutex<Vec<(usize, Vec<f64>)>> = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..workers.max(1) {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= points.len() {
                    break;
                }
                let over: Vec<(String, String)> = keys.iter().zip(params).zip(&points[i]).map(|((n, p), u)| (n.clone(), format!("{}", p.min + u * (p.max - p.min)))).collect();
                let row = match crate::runner::run_with(model, &over) {
                    Ok(run) => goals.iter().map(|g| goal_value(&run, g).filter(|v| v.is_finite()).unwrap_or(f64::NAN)).collect(),
                    Err(_) => vec![f64::NAN; goals.len()],
                };
                out.lock().unwrap().push((i, row));
            });
        }
    });
    let mut v = out.into_inner().unwrap();
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, r)| r).collect()
}

/// How a parameter acts on a goal.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Negligible,
    Linear,
    NonlinearOrInteracting,
}

#[derive(Clone, Debug, Serialize)]
pub struct Effect {
    pub param: String,
    pub mu: f64,
    pub mu_star: f64,
    pub sigma: f64,
    pub class: Class,
    /// elementary effects that could be computed
    pub n: usize,
}

/// `mu`, `mu*`, `sigma` per parameter from trajectories and the outputs of one goal.
pub fn effects(trajs: &[Trajectory], y: &[f64], params: &[Param], delta_tol: f64) -> Vec<Effect> {
    let k = params.len();
    let mut ees: Vec<Vec<f64>> = vec![Vec::new(); k];
    let mut at = 0;
    for t in trajs {
        for (s, (i, d)) in t.steps.iter().enumerate() {
            let (a, b) = (y[at + s], y[at + s + 1]);
            if a.is_finite() && b.is_finite() && d.abs() > delta_tol {
                ees[*i].push((b - a) / d);
            }
        }
        at += t.points.len();
    }
    params
        .iter()
        .zip(ees)
        .map(|(p, e)| {
            let n = e.len();
            let mu = if n > 0 { e.iter().sum::<f64>() / n as f64 } else { f64::NAN };
            let mu_star = if n > 0 { e.iter().map(|x| x.abs()).sum::<f64>() / n as f64 } else { f64::NAN };
            let sigma = if n > 1 { (e.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt() } else { f64::NAN };
            Effect { param: p.name.clone(), mu, mu_star, sigma, class: Class::Linear, n }
        })
        .collect()
}

/// Classify: negligible if `mu_star < tau * max(mu_star)`; else linear if `sigma / mu_star < linear_cut`.
pub fn classify(e: &mut [Effect], tau: f64, linear_cut: f64) {
    let top = e.iter().map(|x| x.mu_star).filter(|x| x.is_finite()).fold(0.0, f64::max);
    for x in e.iter_mut() {
        x.class = if !x.mu_star.is_finite() || x.mu_star < tau * top || top == 0.0 {
            Class::Negligible
        } else if x.sigma / x.mu_star < linear_cut {
            Class::Linear
        } else {
            Class::NonlinearOrInteracting
        };
    }
}

/// Kendall's tau-b between two score lists (ties handled; NaN if undefined).
pub fn kendall_tau(a: &[f64], b: &[f64]) -> f64 {
    let zero = vec![0.0; a.len().min(b.len())];
    kendall_tau_tied(a, b, &zero, &zero)
}

/// Kendall's tau-b where two scores of a list count as tied when they differ by no more than the sum of
/// their tolerances `ea[i] + ea[j]` (for `a`) or `eb[i] + eb[j]` (for `b`): parameters whose effects are
/// indistinguishable have no ranking to keep stable.
pub fn kendall_tau_tied(a: &[f64], b: &[f64], ea: &[f64], eb: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    let (mut c, mut d, mut ta, mut tb) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        for j in i + 1..n {
            let (tia, tib) = ((a[i] - a[j]).abs() <= ea[i] + ea[j] && (ea[i] + ea[j] > 0.0 || a[i] == a[j]), (b[i] - b[j]).abs() <= eb[i] + eb[j] && (eb[i] + eb[j] > 0.0 || b[i] == b[j]));
            let (x, y) = ((a[i] - a[j]).signum() * f64::from(!tia), (b[i] - b[j]).signum() * f64::from(!tib));
            if x == 0.0 && y == 0.0 {
                continue;
            }
            if x == 0.0 {
                ta += 1.0;
            } else if y == 0.0 {
                tb += 1.0;
            } else if x == y {
                c += 1.0;
            } else {
                d += 1.0;
            }
        }
    }
    let den = ((c + d + ta) * (c + d + tb)).sqrt();
    if den == 0.0 { f64::NAN } else { (c - d) / den }
}

/// Settings of a screening.
#[derive(Clone, Debug)]
pub struct ScreenSpec {
    pub trajectories: usize,
    pub levels: usize,
    pub seed: u64,
    pub tau: f64,
    pub linear_cut: f64,
    /// ranking agreement (Kendall) between r and 2r trajectories that counts as converged
    pub converge_tau: f64,
    pub workers: usize,
    /// keep the design points and every output in the result (for cross-checks with other tools)
    pub keep_design: bool,
}

impl Default for ScreenSpec {
    fn default() -> Self {
        ScreenSpec { trajectories: 20, levels: 4, seed: 1, tau: 0.1, linear_cut: 0.1, converge_tau: 0.9, workers: 4, keep_design: false }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GoalScreen {
    pub goal: String,
    pub effects: Vec<Effect>,
    /// Kendall tau between the mu* of the first r trajectories and of all 2r
    pub kendall_r_vs_2r: f64,
    /// share of the parameters that matter (not negligible) at 2r that also matter at r
    pub top_overlap: f64,
    pub converged: bool,
    pub failed_runs: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Screening {
    pub params: Vec<Param>,
    pub trajectories: usize,
    pub levels: usize,
    pub seed: u64,
    pub runs: usize,
    pub goals: Vec<GoalScreen>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<serde_json::Value>,
}

pub fn screen(model: &Model, params: &[Param], goals: &[Goal], spec: &ScreenSpec) -> Result<Screening, String> {
    if params.is_empty() {
        return Err("no parameters to screen".into());
    }
    if spec.levels < 2 || spec.levels % 2 != 0 {
        return Err("levels must be an even number, 4 or more".into());
    }
    if let Some(p) = params.iter().find(|p| !(p.min < p.max)) {
        return Err(format!("“{}” has no range (min {} not below max {})", p.name, p.min, p.max));
    }
    let r = spec.trajectories.max(2);
    let trajs = morris_design(params.len(), 2 * r, spec.levels, spec.seed);
    let points: Vec<Vec<f64>> = trajs.iter().flat_map(|t| t.points.clone()).collect();
    let ys = evaluate(model, params, goals, &points, spec.workers);
    let half_len: usize = trajs[..r].iter().map(|t| t.points.len()).sum();
    let mut out = Vec::new();
    for (gi, g) in goals.iter().enumerate() {
        let y: Vec<f64> = ys.iter().map(|row| row[gi]).collect();
        let mut full = effects(&trajs, &y, params, 1e-12);
        let mut small = effects(&trajs[..r], &y[..half_len], params, 1e-12);
        classify(&mut full, spec.tau, spec.linear_cut);
        classify(&mut small, spec.tau, spec.linear_cut);
        let (a, b): (Vec<f64>, Vec<f64>) = (small.iter().map(|e| e.mu_star).collect(), full.iter().map(|e| e.mu_star).collect());
        // effects within 2% of the largest are ties: parameters with equal effects have no ranking to keep stable
        let scale = a.iter().chain(b.iter()).map(|x| x.abs()).fold(0.0, f64::max);
        let tol = vec![0.01 * scale; a.len()];
        let tau = kendall_tau_tied(&a, &b, &tol, &tol);
        let matter: Vec<usize> = (0..full.len()).filter(|&i| full[i].class != Class::Negligible).collect();
        let overlap = if matter.is_empty() { 1.0 } else { matter.iter().filter(|&&i| small[i].class != Class::Negligible).count() as f64 / matter.len() as f64 };
        out.push(GoalScreen {
            goal: g.name.clone(),
            effects: full,
            kendall_r_vs_2r: tau,
            top_overlap: overlap,
            converged: tau.is_nan() && overlap == 1.0 || tau >= spec.converge_tau,
            failed_runs: y.iter().filter(|v| v.is_nan()).count(),
        });
    }
    let design = spec.keep_design.then(|| serde_json::json!({"x": points, "y": ys, "trajectory_length": params.len() + 1, "goals": goals.iter().map(|g| g.name.clone()).collect::<Vec<_>>()}));
    Ok(Screening { params: params.to_vec(), trajectories: r, levels: spec.levels, seed: spec.seed, runs: points.len(), goals: out, design })
}

/// The screening as JSON: for the command line, the agent tools and Python.
pub fn screen_json(model: &Model, params: &[String], goals: &[String], spec: &ScreenSpec) -> Result<serde_json::Value, String> {
    let (p, g) = (params_of(model, params)?, goals_of(model, goals)?);
    Ok(serde_json::to_value(screen(model, &p, &g, spec)?).map_err(|e| e.to_string())?)
}

/// One row per goal and parameter, for a spreadsheet.
pub fn to_csv(s: &Screening) -> String {
    let mut o = String::from("goal,param,mu,mu_star,sigma,class,n,converged\n");
    for g in &s.goals {
        for e in &g.effects {
            o += &format!("{},{},{},{},{},{:?},{},{}\n", g.goal, e.param, e.mu, e.mu_star, e.sigma, e.class, e.n, g.converged);
        }
    }
    o
}

/// For the tests of the other analysis functions.
#[cfg(test)]
pub fn tests_formula_model(formula: &str, xs: &[&str]) -> Model {
    tests::formula_model(formula, xs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ManifestLever;
    use crate::model::{Kind, Var};

    /// A model whose goal `Y` is `formula` of constants X1.. over 0..1 (no dynamics: one time point is enough).
    pub fn formula_model(formula: &str, xs: &[&str]) -> Model {
        let mut m = Model::default();
        m.settings.initial_time = 0.0;
        m.settings.final_time = 1.0;
        m.settings.time_step = 1.0;
        for (i, n) in xs.iter().enumerate() {
            let mut v = Var::new(i as u64 + 1, n, Kind::Constant, [0.0, 0.0]);
            v.equation = "0.5".into();
            m.vars.push(v);
            m.manifest.levers.push(ManifestLever { name: n.to_string(), min: 0.0, max: 1.0, ..Default::default() });
        }
        let mut y = Var::new(100, "Y", Kind::Aux, [0.0, 0.0]);
        y.equation = formula.into();
        m.vars.push(y);
        for n in xs {
            let from = m.by_name(&crate::expr::normalize(n)).unwrap().id;
            m.add_link(from, 100);
        }
        m.manifest.goals.push(Goal { name: "y".into(), output: "Y".into(), ..Default::default() });
        m
    }

    #[test]
    fn trajectories_move_one_parameter_by_delta_inside_the_cube() {
        let d = morris_design(5, 30, 4, 7);
        let delta = 2.0 / 3.0;
        for t in &d {
            assert_eq!(t.points.len(), 6);
            let mut seen = [false; 5];
            for (s, (i, step)) in t.steps.iter().enumerate() {
                assert!(!seen[*i]);
                seen[*i] = true;
                assert!((step.abs() - delta).abs() < 1e-9, "{step}");
                for (j, (a, b)) in t.points[s].iter().zip(&t.points[s + 1]).enumerate() {
                    assert!(if j == *i { (b - a - step).abs() < 1e-12 } else { a == b });
                    assert!((0.0..=1.0).contains(b));
                }
            }
        }
    }

    #[test]
    fn an_additive_model_gives_exact_effects_and_an_inert_parameter_is_negligible() {
        let m = formula_model("10 * X1 + 2 * X2 + 0 * X3", &["X1", "X2", "X3"]);
        let (p, g) = (params_of(&m, &[]).unwrap(), goals_of(&m, &[]).unwrap());
        let s = screen(&m, &p, &g, &ScreenSpec { trajectories: 10, ..Default::default() }).unwrap();
        let e = &s.goals[0].effects;
        assert!((e[0].mu_star - 10.0).abs() < 1e-9 && (e[1].mu_star - 2.0).abs() < 1e-9 && e[2].mu_star.abs() < 1e-9, "{e:?}");
        assert!(e[0].sigma < 1e-9);
        assert_eq!((e[0].class, e[1].class, e[2].class), (Class::Linear, Class::Linear, Class::Negligible));
        assert_eq!(s.runs, 20 * 4);
        assert!(s.goals[0].converged);
    }

    #[test]
    fn interaction_and_curvature_show_as_sigma() {
        let m = formula_model("X1 * X2 * 10 + X3", &["X1", "X2", "X3"]);
        let (p, g) = (params_of(&m, &[]).unwrap(), goals_of(&m, &[]).unwrap());
        let s = screen(&m, &p, &g, &ScreenSpec { trajectories: 40, ..Default::default() }).unwrap();
        let e = &s.goals[0].effects;
        assert_eq!(e[0].class, Class::NonlinearOrInteracting, "{e:?}");
        assert_eq!(e[2].class, Class::Linear);
    }

    #[test]
    fn a_parameter_without_a_declared_range_is_refused() {
        let mut m = formula_model("X1", &["X1"]);
        m.manifest.levers.clear();
        assert!(params_of(&m, &[]).unwrap_err().contains("declares no levers"));
        let m2 = formula_model("X1", &["X1"]);
        assert!(params_of(&m2, &["Nope".into()]).unwrap_err().contains("no declared range"));
    }

    #[test]
    fn kendall_tau_counts_ties_the_b_way() {
        assert_eq!(kendall_tau(&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0]), 1.0);
        assert_eq!(kendall_tau(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]), -1.0);
        let t = kendall_tau(&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 3.0]);
        assert!((t - 5.0 / (30.0f64).sqrt()).abs() < 1e-12, "{t}");
    }
}
