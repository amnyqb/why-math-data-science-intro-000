//! F2 `importance`: what matters, and why? Sobol indices of the goals, which feedback loop
//! dominates (over the base run and over an ensemble of parameter draws), optionally the
//! eigenvalue elasticities at a time, and the variables that cannot matter at all.

use serde_json::{json, Value};

use crate::loopscore::{dominant, loop_scores, unreachable_from_goals, LoopScores};
use crate::manifest::Goal;
use crate::model::Model;
use crate::screen::{Param, Rng};
use crate::sobol::{sobol, SobolSpec};

#[derive(Clone, Debug)]
pub struct ImportanceSpec {
    pub sobol: SobolSpec,
    /// parameter draws over which loop dominance is tallied (0 = base run only)
    pub ensemble: usize,
    pub eigen_at: Option<f64>,
}

impl Default for ImportanceSpec {
    fn default() -> Self {
        ImportanceSpec { sobol: SobolSpec::default(), ensemble: 30, eigen_at: None }
    }
}

/// Per loop: how much of the time it is the strongest, and its mean relative score.
fn tally(s: &LoopScores, share: &mut [f64], mean: &mut [f64], steps: &mut f64) {
    let d = dominant(s);
    for (t, who) in d.iter().enumerate() {
        if let Some(i) = who {
            share[*i] += 1.0;
        }
        for i in 0..s.loops.len() {
            mean[i] += s.relative[i][t];
        }
    }
    *steps += d.len() as f64;
}

pub fn dominance_json(model: &Model) -> Result<Value, String> {
    dominance_json_with(model, "auto")
}

/// `method`: "auto" (exhaustive, or the published strongest path when the cap would be reached), "exhaustive",
/// "published" (Eberlein and Schoenberg 2020) or "bestfirst" (our earlier best-first search, for comparison).
pub fn dominance_json_with(model: &Model, method: &str) -> Result<Value, String> {
    let s = match method {
        "auto" => loop_scores(model)?,
        "exhaustive" => crate::loopscore::loop_scores_exhaustive(model)?,
        "published" => crate::loopscore::loop_scores_published(model)?,
        "bestfirst" => crate::loopscore::loop_scores_strongest(model)?,
        o => return Err(format!("unknown method “{o}” (auto, exhaustive, published, bestfirst)")),
    };
    Ok(loops_value(&s, &run_tally(&s), None))
}

fn run_tally(s: &LoopScores) -> (Vec<f64>, Vec<f64>, f64) {
    let (mut share, mut mean, mut steps) = (vec![0.0; s.loops.len()], vec![0.0; s.loops.len()], 0.0);
    tally(s, &mut share, &mut mean, &mut steps);
    (share, mean, steps)
}

fn loops_value(s: &LoopScores, t: &(Vec<f64>, Vec<f64>, f64), runs: Option<usize>) -> Value {
    let (share, mean, steps) = t;
    let mut rows: Vec<Value> = (0..s.loops.len())
        .map(|i| {
            let l = &s.loops[i];
            json!({"id": l.id, "variables": l.variables, "polarity": l.polarity, "scored": l.scored,
                   "share_of_time_dominant": if *steps > 0.0 { share[i] / steps } else { 0.0 },
                   "mean_relative_score": if *steps > 0.0 { mean[i] / steps } else { 0.0 }})
        })
        .collect();
    rows.sort_by(|a, b| b["share_of_time_dominant"].as_f64().unwrap().total_cmp(&a["share_of_time_dominant"].as_f64().unwrap()));
    json!({"loops": rows, "loop_count": s.loops.len(), "unscored_loops": s.unscored_loops, "method": s.method, "integration": s.integration, "cap_reached": s.cap_reached, "truncated": s.truncated, "runs": runs.unwrap_or(1), "times": s.times.len()})
}

/// Dominance tallied over `n` runs with parameters drawn uniformly over their ranges (seeded).
fn ensemble(model: &Model, params: &[Param], n: usize, seed: u64) -> Result<Value, String> {
    let mut rng = Rng(seed.wrapping_add(77) | 1);
    let mut acc: Option<(LoopScores, Vec<f64>, Vec<f64>, f64)> = None;
    let mut used = 0;
    for _ in 0..n {
        let mut m = model.clone();
        for p in params {
            m.apply_change(&p.name, &format!("{}", p.min + rng.next() * (p.max - p.min)));
        }
        let Ok(s) = loop_scores(&m) else { continue };
        // strongest-path runs find different loops each time: the tally across runs is only defined for the same loop set
        if s.method != "exhaustive" {
            return Err("the loop set is found by a strongest-path search and differs from run to run: no ensemble tally".into());
        }
        match &mut acc {
            None => {
                let (a, b, c) = run_tally(&s);
                acc = Some((s, a, b, c));
            }
            Some((first, share, mean, steps)) => {
                if s.loops.len() != first.loops.len() {
                    continue;
                }
                tally(&s, share, mean, steps);
            }
        }
        used += 1;
    }
    let (s, share, mean, steps) = acc.ok_or("no ensemble run could be scored")?;
    Ok(loops_value(&s, &(share, mean, steps), Some(used)))
}

pub fn importance_json(model: &Model, params: &[Param], goals: &[Goal], spec: &ImportanceSpec) -> Result<Value, String> {
    let sob = sobol(model, params, goals, &spec.sobol)?;
    let outputs: Vec<String> = goals.iter().map(|g| g.output.clone()).collect();
    let prune = unreachable_from_goals(model, &outputs)?;
    let base = loop_scores(model);
    let dom_base = base.as_ref().ok().map(|s| loops_value(s, &run_tally(s), None));
    let dom_ens = if spec.ensemble > 0 { ensemble(model, params, spec.ensemble, spec.sobol.seed).ok() } else { None };
    let eig = match spec.eigen_at {
        Some(t) => Some(crate::eigen::eigen_analysis(model, t, &params.iter().map(|p| p.name.clone()).collect::<Vec<_>>())?),
        None => None,
    };
    let header = match &base {
        Ok(_) => dom_base.as_ref().map(|d| json!({"method": d["method"], "unscored_loops": d["unscored_loops"], "integration": d["integration"]})),
        Err(e) => Some(json!({"method": null, "unscored_loops": null, "integration": "euler", "error": e})),
    };
    Ok(json!({
        "sobol": serde_json::to_value(&sob).map_err(|e| e.to_string())?,
        "loop_scores": header,
        "dominance_base_run": dom_base,
        "dominance_ensemble": dom_ens,
        "eigenvalues": eig,
        "removable": prune,
    }))
}
