//! F5 `recommend_structures`: proposals for structure that would improve the goals.
//!
//! A **library** of structures, each a parameterised batch of operations tied to a real decision
//! (versioned: [`LIBRARY_VERSION`]). The roles a structure needs (which flow is the decision, which
//! variable is the arrival of what was ordered, ...) are named by the caller. For each structure
//! the weight is searched on a grid over an ensemble of draws of the manifest's levers, and the
//! results are reported as proposals with their effect on every goal, how often they beat the
//! model as it is, and which loops they add: nothing is ever applied to the model.
//!
//! Roles not named by the caller are detected from the structure of the model where possible (see
//! [`detect_roles`]); only roles that cannot be detected are asked for. With several goals the answer is
//! the Pareto set (proposals not dominated on the per-goal improvements); a single ranked list is given
//! only when the manifest declares a `weight` on every goal. Improvement is scale-free: each goal's
//! change is divided by the spread of its base-run scores over the ensemble (by |target| for a target).
//!
//! Library 1: `supply_line` (count what is already ordered), `forecast` (act on an expected
//! value), `information_link` (act on the actual value instead of the perceived one), `buffer`
//! (aim above the target), `delay_change` (a process faster or slower).

use std::collections::HashMap;

use serde::Serialize;
use serde_json::{json, Value};

use crate::expr::{normalize, parse, references, rename_in, BinOp, Expr};
use crate::manifest::{goal_score, goal_value, parse_direction, Direction, Goal};
use crate::model::{Kind, Model};
use crate::ops::{Op, Session};
use crate::screen::Rng;

pub const LIBRARY_VERSION: &str = "2";

pub struct Template {
    pub name: &'static str,
    pub decision: &'static str,
    pub roles: &'static [&'static str],
    /// weights tried: (low, high)
    pub range: (f64, f64),
    /// beyond this the structure has no plausible real-world counterpart
    pub plausible: (f64, f64),
    pub adds: &'static str,
    /// Where the structure comes from (textbook or paper). "to be recorded" when not known.
    pub source: &'static str,
    /// true when no source has been recorded: flagged in the output
    pub unsourced: bool,
}

const TBR: &str = "to be recorded";
const STERMAN_SL: &str = "Sterman (2000) Business Dynamics, ch. 17: stock management structure with the supply line";
const STERMAN_FC: &str = "Sterman (2000) Business Dynamics, expectation formation and forecasting (chapter number to be verified: the task brief says 18)";

pub const LIBRARY: &[Template] = &[
    Template { name: "supply_line", decision: "Count what is already on order or in transit when deciding how much more to order", roles: &["flow", "arrival", "delay", "time"], range: (0.0, 2.0), plausible: (0.0, 1.5), adds: "a balancing loop through the supply line", source: STERMAN_SL, unsourced: false },
    Template { name: "forecast", decision: "Act on an expected value (a smoothed trend projected ahead) instead of the latest observation", roles: &["flow", "input", "time"], range: (0.0, 8.0), plausible: (0.0, 6.0), adds: "an anticipating information link", source: STERMAN_FC, unsourced: false },
    Template { name: "information_link", decision: "Give the decision-maker the actual value instead of the delayed or smoothed one", roles: &["flow", "perceived", "actual"], range: (0.0, 1.0), plausible: (0.0, 1.0), adds: "a shorter information path in the existing loop", source: TBR, unsourced: true },
    Template { name: "buffer", decision: "Aim above the target (safety stock, spare capacity)", roles: &["flow", "target"], range: (0.0, 1.0), plausible: (0.0, 0.5), adds: "no new loop: a higher target", source: TBR, unsourced: true },
    Template { name: "delay_change", decision: "Make a process faster or slower (a shorter lead time, a quicker decision)", roles: &["delay"], range: (0.25, 2.0), plausible: (0.5, 2.0), adds: "no new loop: a different delay", source: TBR, unsourced: true },
];

#[derive(Clone, Debug, Serialize)]
pub struct Proposal {
    pub structure: String,
    pub decision: String,
    pub weight: f64,
    pub plausible: bool,
    pub improvement: Vec<(String, f64)>,
    /// mean over goals of the relative improvement
    pub mean_improvement: f64,
    /// share of the ensemble draws in which it is better than the model as it is
    pub robustness: f64,
    pub loops_before: (usize, usize),
    pub loops_after: (usize, usize),
    pub adds: String,
    pub source: String,
    /// true when the structure has no recorded source
    pub unsourced: bool,
    /// not dominated by another proposal (or by doing nothing) on the per-goal mean improvements
    pub pareto: bool,
    /// weighted mean improvement, present only when every goal declares a weight
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weighted_improvement: Option<f64>,
    pub operations: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoleUse {
    pub role: String,
    pub name: String,
    /// "given" or "detected: ..." with the reason
    pub how: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Skipped {
    pub structure: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Recommendation {
    pub library_version: String,
    pub proposals: Vec<Proposal>,
    /// structure names, best first; present only when every goal declares a weight
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking: Option<Vec<String>>,
    pub roles: Vec<RoleUse>,
    pub skipped: Vec<Skipped>,
    pub ensemble: usize,
    pub note: String,
}

fn eq_of(m: &Model, name: &str) -> Result<String, String> {
    m.by_name(&normalize(name)).map(|v| v.equation.clone()).ok_or_else(|| format!("“{name}” is not a variable of the model"))
}

/// The batch of operations that puts `t` into the model with weight `w`.
fn operations(t: &Template, roles: &HashMap<String, String>, m: &Model, w: f64) -> Result<Value, String> {
    let r = |k: &str| roles.get(k).cloned().ok_or_else(|| format!("needs the role “{k}”"));
    Ok(match t.name {
        "supply_line" => {
            let (flow, arrival, _delay, time) = (r("flow")?, r("arrival")?, r("delay")?, r("time")?);
            let sl = format!("{flow} supply line");
            let orig = eq_of(m, &flow)?;
            json!([
                {"op": "add_var", "name": sl, "kind": "stock", "equation": "0", "doc": "Added by recommend_structures: orders minus arrivals since the start, i.e. the supply line's deviation from its starting level (the model is assumed to start in balance)."},
                {"op": "add_var", "name": format!("{sl} in"), "kind": "flow", "equation": flow, "to": sl},
                {"op": "add_var", "name": format!("{sl} out"), "kind": "flow", "equation": arrival, "from": sl},
                {"op": "set_equation", "name": flow, "equation": format!("({orig}) - {w} * {sl} / {time}")},
            ])
        }
        "forecast" => {
            let (flow, input, time) = (r("flow")?, r("input")?, r("time")?);
            let orig = eq_of(m, &flow)?;
            let new = rename_in(&orig, &normalize(&input), &format!("EXPECT({input}, {time}, ahead={w}, measure=forecast)"));
            json!([{"op": "set_equation", "name": flow, "equation": new}])
        }
        "information_link" => {
            let (flow, perceived, actual) = (r("flow")?, r("perceived")?, r("actual")?);
            let orig = eq_of(m, &flow)?;
            let new = rename_in(&orig, &normalize(&perceived), &format!("((1 - {w}) * {perceived} + {w} * {actual})"));
            json!([{"op": "set_equation", "name": flow, "equation": new}])
        }
        "buffer" => {
            let (flow, target) = (r("flow")?, r("target")?);
            let orig = eq_of(m, &flow)?;
            let new = rename_in(&orig, &normalize(&target), &format!("({target} * (1 + {w}))"));
            json!([{"op": "set_equation", "name": flow, "equation": new}])
        }
        "delay_change" => {
            let delay = r("delay")?;
            let orig = eq_of(m, &delay)?;
            let x: f64 = orig.trim().parse().map_err(|_| format!("“{delay}” is not a plain number"))?;
            json!([{"op": "set_equation", "name": delay, "equation": format!("{}", x * w)}])
        }
        _ => return Err("unknown structure".into()),
    })
}

fn apply(m: &Model, ops: &Value) -> Result<Model, String> {
    let batch: Vec<Op> = serde_json::from_value(ops.clone()).map_err(|e| e.to_string())?;
    let mut s = Session::new(m.clone());
    s.apply("recommend_structures", &batch).map_err(|e| format!("{} {:?}", e.reason, e.issues.iter().map(|i| i.message.clone()).collect::<Vec<_>>()))?;
    Ok(s.model)
}

fn loop_counts(m: &Model) -> (usize, usize) {
    match crate::loopscore::loop_scores(m) {
        Ok(s) => (s.loops.iter().filter(|l| l.polarity == "R").count(), s.loops.iter().filter(|l| l.polarity == "B").count()),
        Err(_) => (0, 0),
    }
}

/// goal values (as scores: larger is better) for each ensemble draw
fn scores(m: &Model, goals: &[Goal], draws: &[Vec<(String, String)>]) -> Vec<Vec<f64>> {
    draws.iter().map(|d| match crate::runner::run_with(m, d) {
        Ok(run) => goals.iter().map(|g| goal_value(&run, g).map(|v| goal_score(g, v)).unwrap_or(f64::NAN)).collect(),
        Err(_) => vec![f64::NAN; goals.len()],
    }).collect()
}

pub struct RecSpec {
    pub grid: usize,
    pub ensemble: usize,
    pub seed: u64,
}

impl Default for RecSpec {
    fn default() -> Self {
        RecSpec { grid: 17, ensemble: 20, seed: 1 }
    }
}

// ---------------------------------------------------------------------------------------------
// role detection

fn is_delay_fn(n: &str) -> bool {
    n.starts_with("DELAY") || n.starts_with("SMOOTH")
}

fn walk<'a>(e: &'a Expr, f: &mut dyn FnMut(&'a Expr)) {
    f(e);
    match e {
        Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => walk(a, f),
        Expr::Bin(_, a, b) => {
            walk(a, f);
            walk(b, f);
        }
        Expr::Call(_, args) => args.iter().for_each(|a| walk(a, f)),
        _ => {}
    }
}

fn refs_of(e: &Expr) -> Vec<String> {
    let mut o = Vec::new();
    references(e, &mut o);
    o
}

fn var_name(m: &Model, norm: &str) -> String {
    m.by_name(norm).map(|v| v.name.clone()).unwrap_or_else(|| norm.to_string())
}

fn is_constant(m: &Model, norm: &str) -> bool {
    m.by_name(norm).is_some_and(|v| v.kind == Kind::Constant || (v.kind != Kind::Stock && v.equation.trim().parse::<f64>().is_ok()))
}

fn found_role(found: &mut Vec<RoleUse>, role: &str, name: String, how: String) {
    found.push(RoleUse { role: role.into(), name, how: format!("detected: {how}") });
}

/// Roles read off the structure of the model from the decision flow alone. Returns the roles found with the
/// reason each was found, and for every role that could not be found the reason why not.
/// Detected: `stock` (the stock the flow ultimately feeds and reads back; informational), `arrival` and
/// `delay` (a DELAY/SMOOTH function or a stage `stock / time` on the path from the flow to that stock),
/// `time` (the constant in a `(gap) / time` pattern), `target` (the constant in `target - stock`),
/// `perceived` and `actual` (a variable read by the flow that is a SMOOTH/DELAY of another variable).
pub fn detect_roles(m: &Model, flow: &str) -> (Vec<RoleUse>, HashMap<String, String>) {
    let mut found: Vec<RoleUse> = Vec::new();
    let mut why: HashMap<String, String> = HashMap::new();
    let fnorm = normalize(flow);
    let Some(fv) = m.by_name(&fnorm) else {
        return (found, why);
    };
    let fexpr = parse(&fv.equation).ok();
    let parsed: HashMap<String, Option<Expr>> = m.vars.iter().map(|v| (normalize(&v.name), parse(&v.equation).ok())).collect();

    // downstream graph: a variable's dependents, and the stock a flow fills
    let mut succ: HashMap<String, Vec<String>> = HashMap::new();
    for v in &m.vars {
        let n = normalize(&v.name);
        if v.kind != Kind::Stock {
            if let Some(Some(e)) = parsed.get(&n) {
                for u in refs_of(e) {
                    succ.entry(u).or_default().push(n.clone());
                }
            }
        }
        if v.kind == Kind::Flow {
            if let Some(sid) = v.to {
                if let Some(st) = m.vars.iter().find(|x| x.id == sid) {
                    succ.entry(n.clone()).or_default().push(normalize(&st.name));
                }
            }
        }
    }
    // variables the flow reads (through auxiliaries; stocks end the walk)
    let mut upstream: Vec<String> = Vec::new();
    let mut stack = fexpr.as_ref().map(refs_of).unwrap_or_default();
    while let Some(n) = stack.pop() {
        if upstream.contains(&n) {
            continue;
        }
        upstream.push(n.clone());
        if m.by_name(&n).is_some_and(|v| v.kind != Kind::Stock) {
            if let Some(Some(e)) = parsed.get(&n) {
                stack.extend(refs_of(e));
            }
        }
    }
    // breadth-first search downstream from the flow
    let mut parent: HashMap<String, String> = HashMap::new();
    let mut order = vec![fnorm.clone()];
    let mut qi = 0;
    while qi < order.len() {
        let u = order[qi].clone();
        qi += 1;
        for w in succ.get(&u).cloned().unwrap_or_default() {
            if w != fnorm && !parent.contains_key(&w) {
                parent.insert(w.clone(), u.clone());
                order.push(w);
            }
        }
    }
    let reach_stocks: Vec<&String> = order.iter().filter(|n| m.by_name(n).is_some_and(|v| v.kind == Kind::Stock)).collect();
    let stock = reach_stocks.iter().find(|n| upstream.contains(**n)).or(reach_stocks.first()).map(|s| (*s).clone());
    let Some(stock) = stock else {
        let r = format!("no stock is fed by “{flow}”, directly or through other variables");
        for k in ["arrival", "delay", "time", "target"] {
            why.insert(k.into(), r.clone());
        }
        detect_perceived(m, &fexpr, &parsed, &mut found, &mut why);
        return (found, why);
    };
    let sname = var_name(m, &stock);
    // the path from the flow to that stock
    let mut path = vec![stock.clone()];
    while let Some(p) = parent.get(path.last().unwrap()) {
        path.push(p.clone());
    }
    path.reverse();
    // arrival and delay on the path
    let mut ad: Option<(String, Option<String>, String)> = None;
    'p: for (i, n) in path.iter().enumerate().skip(1) {
        let Some(v) = m.by_name(n) else { continue };
        let Some(Some(e)) = parsed.get(n) else { continue };
        if v.kind == Kind::Stock {
            continue;
        }
        let mut hit: Option<(Option<String>, String)> = None;
        walk(e, &mut |x| {
            if hit.is_none() {
                if let Expr::Call(f, a) = x {
                    if is_delay_fn(f) && a.len() >= 2 && refs_of(&a[0]).iter().any(|r| path[..i].contains(r)) {
                        hit = Some(match &a[1] {
                            Expr::Var(t) => (Some(var_name(m, t)), format!("{f} on the path from “{flow}” to “{sname}”")),
                            _ => (None, format!("{f} with a literal delay time")),
                        });
                    }
                }
            }
        });
        if let Some((d, how)) = hit {
            ad = Some((v.name.clone(), d, how));
            break 'p;
        }
        // a stage: an outflow of a stock on the path, equal to stock / time
        if let Expr::Bin(BinOp::Div, a, b) = e {
            if let (Expr::Var(sv), Expr::Var(t)) = (&**a, &**b) {
                if v.kind == Kind::Flow && path[..i].contains(sv) && m.by_name(sv).is_some_and(|s| s.kind == Kind::Stock) {
                    ad = Some((v.name.clone(), Some(var_name(m, t)), format!("a stage ({} = {} / {}) on the path from “{flow}” to “{sname}”", v.name, var_name(m, sv), var_name(m, t))));
                    break 'p;
                }
            }
        }
    }
    match ad {
        Some((arr, d, how)) => {
            found_role(&mut found, "arrival", arr, how.clone());
            match d {
                Some(d) => found_role(&mut found, "delay", d, how),
                None => {
                    why.insert("delay".into(), "the delay time is a literal number in the equation, not a named variable".into());
                }
            }
        }
        None => {
            let r = format!("no DELAY or SMOOTH function and no stage on the path from “{flow}” to “{sname}”");
            why.insert("arrival".into(), r.clone());
            why.insert("delay".into(), r);
        }
    }
    // (gap) / time and target - stock in the flow's own equation
    let mut time: Option<String> = None;
    let mut target: Option<String> = None;
    if let Some(e) = &fexpr {
        walk(e, &mut |x| {
            if let Expr::Bin(BinOp::Div, num, den) = x {
                if let Expr::Var(t) = &**den {
                    if time.is_none() && is_constant(m, t) && refs_of(num).contains(&stock) {
                        time = Some(var_name(m, t));
                        walk(num, &mut |y| {
                            if let Expr::Bin(BinOp::Sub, a, b) = y {
                                if let (Expr::Var(p), Expr::Var(q)) = (&**a, &**b) {
                                    for (g, s) in [(p, q), (q, p)] {
                                        if *s == stock && is_constant(m, g) && target.is_none() {
                                            target = Some(var_name(m, g));
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }
        });
    }
    match time {
        Some(t) => found_role(&mut found, "time", t, format!("the constant in the (gap) / time pattern of “{flow}”")),
        None => {
            why.insert("time".into(), format!("“{flow}” has no (gap involving “{sname}”) / constant pattern"));
        }
    }
    match target {
        Some(t) => found_role(&mut found, "target", t, format!("the constant in (target - {sname}) of “{flow}”")),
        None => {
            why.insert("target".into(), format!("“{flow}” has no gap of the form constant - “{sname}”"));
        }
    }
    detect_perceived(m, &fexpr, &parsed, &mut found, &mut why);
    found.insert(0, RoleUse { role: "stock".into(), name: sname, how: "detected: the stock the flow feeds and reads back".into() });
    (found, why)
}

fn detect_perceived(m: &Model, fexpr: &Option<Expr>, parsed: &HashMap<String, Option<Expr>>, found: &mut Vec<RoleUse>, why: &mut HashMap<String, String>) {
    let mut res: Option<(String, String, String)> = None;
    if let Some(e) = fexpr {
        for r in refs_of(e) {
            if let Some(Some(Expr::Call(f, a))) = parsed.get(&r) {
                if is_delay_fn(f) && a.len() >= 2 {
                    if let Expr::Var(x) = &a[0] {
                        res = Some((var_name(m, &r), var_name(m, x), f.clone()));
                        break;
                    }
                }
            }
        }
    }
    match res {
        Some((p, x, f)) => {
            found_role(found, "perceived", p.clone(), format!("the flow reads {p}, a {f} of another variable"));
            found_role(found, "actual", x, format!("the variable that {p} is a {f} of"));
        }
        None => {
            let r = "the flow reads no variable that is a SMOOTH or DELAY of another variable".to_string();
            why.insert("perceived".into(), r.clone());
            why.insert("actual".into(), r);
        }
    }
}

// ---------------------------------------------------------------------------------------------

fn pareto_flags(v: &[Vec<f64>]) -> Vec<bool> {
    const E: f64 = 1e-12;
    let dom = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| *x >= y - E) && a.iter().zip(b).any(|(x, y)| *x > y + E);
    (0..v.len())
        .map(|i| {
            let zero = vec![0.0; v[i].len()];
            !dom(&zero, &v[i]) && !(0..v.len()).any(|j| j != i && dom(&v[j], &v[i]))
        })
        .collect()
}

pub fn recommend(model: &Model, goals: &[Goal], given: &HashMap<String, String>, only: &[String], spec: &RecSpec) -> Result<Recommendation, String> {
    // roles: the caller's override what is detected
    let mut roles = given.clone();
    let mut role_use: Vec<RoleUse> = given.iter().map(|(k, v)| RoleUse { role: k.clone(), name: v.clone(), how: "given".into() }).collect();
    role_use.sort_by(|a, b| a.role.cmp(&b.role));
    let mut undetected: HashMap<String, String> = HashMap::new();
    match given.get("flow") {
        Some(flow) => {
            let (found, why) = detect_roles(model, flow);
            for r in found {
                if r.role == "stock" {
                    role_use.push(r);
                } else if !roles.contains_key(&r.role) {
                    roles.insert(r.role.clone(), r.name.clone());
                    role_use.push(r);
                }
            }
            undetected = why;
        }
        None => {
            undetected.insert("flow".into(), "the decision flow cannot be detected: name it with --role flow=NAME".into());
        }
    }
    for k in ["input"] {
        undetected.entry(k.into()).or_insert_with(|| "this role is not detected from structure".into());
    }

    let mut rng = Rng(spec.seed | 1);
    let mut draws: Vec<Vec<(String, String)>> = vec![Vec::new()];
    for _ in 0..spec.ensemble {
        draws.push(model.manifest.levers.iter().map(|l| (l.name.clone(), format!("{}", l.min + rng.next() * (l.max - l.min)))).collect());
    }
    let base = scores(model, goals, &draws);
    if base.iter().any(|r| r.iter().any(|v| v.is_nan())) {
        return Err("the model as it is fails for some draws; no comparison is possible".into());
    }
    // scale of each goal: |target| for a target, else the spread of the base scores over the ensemble
    let n_e = spec.ensemble.max(1) as f64;
    let scale: Vec<f64> = goals
        .iter()
        .enumerate()
        .map(|(g, goal)| {
            let col: Vec<f64> = (1..draws.len()).map(|d| base[d][g]).collect();
            let mean = col.iter().sum::<f64>() / n_e;
            let sd = (col.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n_e).sqrt();
            match parse_direction(&goal.direction) {
                Ok(Direction::Target(t)) if t.abs() > 1e-12 => t.abs(),
                _ if sd > 1e-12 => sd,
                _ if mean.abs() > 1e-12 => mean.abs(),
                _ => 1.0,
            }
        })
        .collect();
    let weights: Option<Vec<f64>> = if !goals.is_empty() && goals.iter().all(|g| g.weight.is_some()) { Some(goals.iter().map(|g| g.weight.unwrap()).collect()) } else { None };
    let wsum: f64 = weights.as_ref().map_or(goals.len() as f64, |w| w.iter().sum());
    let imp = |sc: &Vec<Vec<f64>>, d: usize, g: usize| (sc[d][g] - base[d][g]) / scale[g];
    // per goal mean improvement over the ensemble draws
    let per_goal = |sc: &Vec<Vec<f64>>| -> Vec<f64> { (0..goals.len()).map(|g| (1..draws.len()).map(|d| imp(sc, d, g)).filter(|x| x.is_finite()).sum::<f64>() / n_e).collect() };
    let combine = |v: &[f64]| -> f64 { v.iter().enumerate().map(|(g, x)| x * weights.as_ref().map_or(1.0, |w| w[g])).sum::<f64>() / wsum };

    let lb = loop_counts(model);
    let (mut proposals, mut skipped) = (Vec::new(), Vec::new());
    let mut vectors: Vec<Vec<f64>> = Vec::new();
    for t in LIBRARY.iter().filter(|t| only.is_empty() || only.iter().any(|o| o == t.name)) {
        if let Some(miss) = t.roles.iter().find(|k| !roles.contains_key(**k)) {
            let how = undetected.get(*miss).map(|r| format!("; not detected: {r}; pass --role {miss}=NAME")).unwrap_or_default();
            skipped.push(Skipped { structure: t.name.into(), reason: format!("needs the role “{miss}”{how}") });
            continue;
        }
        let mut best: Option<(f64, f64, Vec<Vec<f64>>)> = None;
        for gi in 0..spec.grid {
            let w = t.range.0 + (t.range.1 - t.range.0) * gi as f64 / (spec.grid.max(2) - 1) as f64;
            let ops = match operations(t, &roles, model, w) {
                Ok(o) => o,
                Err(e) => {
                    skipped.push(Skipped { structure: t.name.into(), reason: e });
                    best = None;
                    break;
                }
            };
            let m2 = match apply(model, &ops) {
                Ok(m2) => m2,
                Err(e) => {
                    skipped.push(Skipped { structure: t.name.into(), reason: format!("the operations are refused by the model: {e}") });
                    best = None;
                    break;
                }
            };
            let sc = scores(&m2, goals, &draws);
            let score = combine(&per_goal(&sc));
            if best.as_ref().map_or(true, |b| score > b.1) {
                best = Some((w, score, sc));
            }
        }
        let Some((w, _, sc)) = best else { continue };
        let ops = operations(t, &roles, model, w)?;
        let m2 = apply(model, &ops)?;
        let pg = per_goal(&sc);
        let improvement: Vec<(String, f64)> = goals.iter().zip(&pg).map(|(g, x)| (g.name.clone(), *x)).collect();
        let robust = (1..draws.len()).filter(|&d| (0..goals.len()).map(|g| imp(&sc, d, g)).sum::<f64>() > 1e-9).count() as f64 / n_e;
        let mean = pg.iter().sum::<f64>() / goals.len().max(1) as f64;
        vectors.push(pg.clone());
        proposals.push(Proposal {
            structure: t.name.into(),
            decision: t.decision.into(),
            weight: w,
            plausible: w >= t.plausible.0 - 1e-12 && w <= t.plausible.1 + 1e-12,
            improvement,
            mean_improvement: mean,
            robustness: robust,
            loops_before: lb,
            loops_after: loop_counts(&m2),
            adds: t.adds.into(),
            source: t.source.into(),
            unsourced: t.unsourced,
            pareto: false,
            weighted_improvement: weights.as_ref().map(|_| combine(&pg)),
            operations: ops,
        });
    }
    for (p, f) in proposals.iter_mut().zip(pareto_flags(&vectors)) {
        p.pareto = f;
    }
    // Pareto set first; within each group by the equal-weight mean (a reading order, not a ranking)
    proposals.sort_by(|a, b| b.pareto.cmp(&a.pareto).then(b.mean_improvement.total_cmp(&a.mean_improvement)));
    let ranking = weights.as_ref().map(|_| {
        let mut r: Vec<&Proposal> = proposals.iter().collect();
        r.sort_by(|a, b| b.weighted_improvement.unwrap_or(f64::MIN).total_cmp(&a.weighted_improvement.unwrap_or(f64::MIN)));
        r.iter().map(|p| p.structure.clone()).collect()
    });
    let note = if weights.is_some() {
        "Proposals only: nothing was applied. Check each against a real decision before using it. `pareto` marks proposals no other proposal beats on every goal; `ranking` uses the goal weights declared in the manifest."
    } else {
        "Proposals only: nothing was applied. Check each against a real decision before using it. `pareto` marks proposals no other proposal beats on every goal; no ranking is given because the manifest declares no goal weights."
    };
    Ok(Recommendation { library_version: LIBRARY_VERSION.into(), proposals, ranking, roles: role_use, skipped, ensemble: spec.ensemble, note: note.into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pareto_set_of_a_known_tradeoff() {
        // A and B trade off; C is beaten by A on both; D is worse than doing nothing; E equals A (ties stay)
        let v = vec![vec![1.0, -0.2], vec![0.2, 0.6], vec![0.1, -0.3], vec![-0.1, -0.1], vec![1.0, -0.2]];
        assert_eq!(pareto_flags(&v), vec![true, true, false, false, true]);
    }

    #[test]
    fn every_template_has_a_source_or_is_flagged() {
        for t in LIBRARY {
            assert!(!t.source.is_empty() && (t.source != "to be recorded" || t.unsourced), "{}", t.name);
        }
    }
}
