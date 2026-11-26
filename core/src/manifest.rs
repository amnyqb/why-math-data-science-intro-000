//! The product manifest: what a model offers to someone who is not its author. Which numbers
//! may be changed and within what range, which results to look at, named scenarios, where its
//! inputs come from, where it is trusted, and which of its checks vouch for it. A model carries
//! one; an end-user screen, a service or a benchmark reads it instead of guessing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::expr;
use crate::model::{Kind, Model};

/// A number a user may change.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ManifestLever {
    /// A constant of the model.
    pub name: String,
    #[serde(default)]
    pub units: String,
    pub min: f64,
    pub max: f64,
    /// The value the model is delivered with; empty means "as the model has it".
    #[serde(default)]
    pub default: Option<f64>,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub group: String,
}

/// A result worth showing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ManifestOutput {
    pub variable: String,
    /// "line", "area", "bar" or "table".
    #[serde(default)]
    pub chart: String,
    #[serde(default)]
    pub units: String,
    #[serde(default)]
    pub reference_lines: Vec<f64>,
}

/// A named set of lever values.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ManifestScenario {
    pub name: String,
    /// lever name → value
    #[serde(default)]
    pub values: BTreeMap<String, f64>,
    /// The direction this scenario is expected to move a goal, relative to the first scenario: goal name → "up", "down",
    /// "better" or "worse" (better and worse follow the goal's direction). The sanity sheet tests it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expect: BTreeMap<String, String>,
}

/// An input that comes from a file or a forecaster instead of being typed in.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    /// The variable of the model that receives it.
    pub input: String,
    /// A file name or an identifier of the source.
    pub source: String,
    #[serde(default)]
    pub column: String,
    /// "data" or "forecast".
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub note: String,
}

/// What a study of the model is trying to achieve: one number out of a run, and which way is better.
/// Declared before any analysis is run, so that "what matters" is always relative to a stated goal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub name: String,
    /// The variable the number is read from.
    pub output: String,
    /// "final", "peak", "lowest", "total", "average", "mean:FROM-TO", "at:TIME" (default "final").
    #[serde(default)]
    pub measure: String,
    /// "min", "max" or "target:VALUE" (default "max").
    #[serde(default)]
    pub direction: String,
    /// Names of manifest scenarios whose ranking must be kept by any simplified model.
    #[serde(default)]
    pub policies: Vec<String>,
    #[serde(default)]
    pub protected_ranking: bool,
    /// Relative importance of this goal (positive). Optional: when every goal carries one, `dyn recommend`
    /// also returns a single ranked list; without weights it returns only the Pareto set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// How the number is transformed before any analysis reads it: "" (not declared, same as "none") or
    /// "none" leave it alone; "log" takes the natural logarithm (only for strictly positive values; a
    /// value that is zero or negative counts as a failed run). Declare "log" for goals that grow
    /// exponentially and so have a heavy-tailed distribution over the parameter ranges: variance-based
    /// importance is meaningless on them. `reduce` tries the goal untransformed first and applies the
    /// declared transform only when its convergence gates fail.
    #[serde(default)]
    pub transform: String,
}

/// Something taken out of the model to see what the goals owe to it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ablation {
    pub name: String,
    /// Stocks held at their initial value (their flows are detached).
    #[serde(default)]
    pub freeze: Vec<String>,
    /// Links cut: [from, to]. The equation of `to` reads the starting value of `from` from then on.
    #[serde(default)]
    pub cut: Vec<[String; 2]>,
}

/// A check that only a person can make, recorded in the manifest.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HumanCheck {
    /// "expected-value", "actuals-reconciled", "cross-check", "benchmark" or "reviewer".
    pub kind: String,
    /// "pass", "fail" or "open" (default: pass once it is signed).
    #[serde(default)]
    pub status: String,
    /// What was done and what it showed.
    #[serde(default)]
    pub note: String,
    /// The number written down (the expected value, before the run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Who made the check.
    #[serde(default)]
    pub by: String,
    /// When (ISO date). For the expected value: the date it was written, which must be before the run.
    #[serde(default)]
    pub date: String,
}

/// The kinds of human check the sanity sheet asks for, in order.
pub const HUMAN_CHECKS: [(&str, &str); 5] = [
    ("expected-value", "Expected value written down before the run"),
    ("actuals-reconciled", "Actuals reconciled with the model's starting values"),
    ("cross-check", "Cross-check against an independent calculation"),
    ("benchmark", "Benchmark against a published or measured number"),
    ("reviewer", "Reviewed by a second person, with the date"),
];

/// Where the model is trusted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Validity {
    pub statement: String,
    /// A variable and a range it was tried over (optional).
    #[serde(default)]
    pub variable: Option<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    /// The longest horizon it is trusted for, in the model's time unit.
    #[serde(default)]
    pub horizon: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub levers: Vec<ManifestLever>,
    #[serde(default)]
    pub outputs: Vec<ManifestOutput>,
    #[serde(default)]
    pub scenarios: Vec<ManifestScenario>,
    #[serde(default)]
    pub bindings: Vec<Binding>,
    #[serde(default)]
    pub validity: Vec<Validity>,
    /// Names of the model's checks that vouch for it.
    #[serde(default)]
    pub tests: Vec<String>,
    /// What a study of the model is after (see [`Goal`]).
    #[serde(default)]
    pub goals: Vec<Goal>,
    /// How much rests on the model: "workshop" (a sketch: the order-of-magnitude, units and conservation, and time-step checks
    /// are required), "board" or "loan" (all checks are required). Empty means all.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stakes: String,
    /// Named ablations for `dyn ablate` (without them it tries each loop and each stock).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ablations: Vec<Ablation>,
    /// The human checks of the sanity sheet (`dyn check --gate deliver`): expected value written before the run, actuals
    /// reconciled, cross-check, benchmark, reviewer and date. Each is recorded by a person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sanity: Vec<HumanCheck>,
    /// Set on a model made by `reduce`: the title or file of the model it was reduced from.
    #[serde(default)]
    pub reduced_from: String,
}

impl Manifest {
    pub fn is_empty(&self) -> bool {
        *self == Manifest::default()
    }
}

fn has_var(m: &Model, name: &str) -> bool {
    m.by_name(&expr::normalize(name)).is_some()
}

/// Mistakes in the manifest: names the model does not have, ranges the wrong way round.
pub fn check(m: &Model) -> Vec<(Option<String>, String)> {
    let mf = &m.manifest;
    let mut out: Vec<(Option<String>, String)> = Vec::new();
    let mut bad = |who: &str, msg: String| out.push((Some(who.to_string()), format!("manifest: {msg}")));
    for l in &mf.levers {
        match m.by_name(&expr::normalize(&l.name)) {
            None => bad(&l.name, format!("lever “{}” is not a variable of the model", l.name)),
            Some(v) if v.kind != Kind::Constant => bad(&l.name, format!("lever “{}” is a {}, not a constant", l.name, v.kind.label())),
            Some(_) => {}
        }
        if !(l.min < l.max) {
            bad(&l.name, format!("lever “{}” has min {} not below max {}", l.name, l.min, l.max));
        }
        if let Some(d) = l.default {
            if d < l.min || d > l.max {
                bad(&l.name, format!("lever “{}” has a default {} outside {} to {}", l.name, d, l.min, l.max));
            }
        }
    }
    for o in &mf.outputs {
        if !has_var(m, &o.variable) {
            bad(&o.variable, format!("output “{}” is not a variable of the model", o.variable));
        }
        if !o.chart.is_empty() && !["line", "area", "bar", "table"].contains(&o.chart.as_str()) {
            bad(&o.variable, format!("output “{}” has an unknown chart “{}” (line, area, bar, table)", o.variable, o.chart));
        }
    }
    for s in &mf.scenarios {
        for k in s.values.keys() {
            match mf.levers.iter().find(|l| expr::normalize(&l.name) == expr::normalize(k)) {
                None => bad(&s.name, format!("scenario “{}” sets “{k}”, which is not a lever of the manifest", s.name)),
                Some(l) => {
                    let v = s.values[k];
                    if v < l.min || v > l.max {
                        bad(&s.name, format!("scenario “{}” sets “{k}” to {v}, outside its range {} to {}", s.name, l.min, l.max));
                    }
                }
            }
        }
    }
    for b in &mf.bindings {
        if !has_var(m, &b.input) {
            bad(&b.input, format!("binding for “{}” names no variable of the model", b.input));
        }
        if b.source.trim().is_empty() {
            bad(&b.input, format!("binding for “{}” has no source", b.input));
        }
        if !b.kind.is_empty() && b.kind != "data" && b.kind != "forecast" {
            bad(&b.input, format!("binding for “{}” has kind “{}” (data or forecast)", b.input, b.kind));
        }
    }
    for v in &mf.validity {
        if let Some(n) = &v.variable {
            if !has_var(m, n) {
                bad(n, format!("validity statement “{}” names “{n}”, which is not a variable of the model", v.statement));
            }
        }
    }
    for g in &mf.goals {
        if g.name.trim().is_empty() {
            bad(&g.output, "a goal has no name".to_string());
        }
        if !has_var(m, &g.output) {
            bad(&g.name, format!("goal “{}” reads “{}”, which is not a variable of the model", g.name, g.output));
        }
        if let Err(e) = parse_goal_measure(&g.measure) {
            bad(&g.name, format!("goal “{}”: {e}", g.name));
        }
        if let Err(e) = parse_direction(&g.direction) {
            bad(&g.name, format!("goal “{}”: {e}", g.name));
        }
        if !valid_transform(&g.transform) {
            bad(&g.name, format!("goal “{}” has an unknown transform “{}” (none, log)", g.name, g.transform));
        }
        for p in &g.policies {
            if !mf.scenarios.iter().any(|s| s.name == *p) {
                bad(&g.name, format!("goal “{}” ranks policy “{p}”, which is not a scenario of the manifest", g.name));
            }
        }
        if g.weight.is_some_and(|w| !(w > 0.0 && w.is_finite())) {
            bad(&g.name, format!("goal “{}”: the weight must be a positive number", g.name));
        }
        if g.protected_ranking && g.policies.len() < 2 {
            bad(&g.name, format!("goal “{}” protects a ranking but names fewer than two policies", g.name));
        }
    }
    for t in &mf.tests {
        if !m.checks.iter().any(|c| expr::normalize(&c.name) == expr::normalize(t) || (c.name.trim().is_empty() && expr::normalize(&c.var) == expr::normalize(t))) {
            bad(t, format!("test “{t}” is not one of the model's checks"));
        }
    }
    out
}

/// A first manifest drawn from what the model already says: its key levers (with their slider
/// ranges, or ±50%), its stocks as outputs, its checks as tests. Nothing is applied.
pub fn draft(m: &Model) -> Manifest {
    let mut mf = Manifest { title: m.title.clone(), ..Default::default() };
    for v in m.vars.iter().filter(|v| v.lever && v.kind == Kind::Constant) {
        let value = v.equation.trim().parse::<f64>().ok();
        let (min, max) = v.slider.map(|s| (s[0], s[1])).unwrap_or_else(|| match value {
            Some(x) if x != 0.0 => (x.min(x * 0.5), x.max(x * 1.5)),
            _ => (0.0, 1.0),
        });
        mf.levers.push(ManifestLever { name: v.name.clone(), units: v.units.clone(), min, max, default: value.filter(|x| *x >= min && *x <= max), explanation: v.comment.clone(), group: String::new() });
    }
    for v in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        mf.outputs.push(ManifestOutput { variable: v.name.clone(), chart: "line".into(), units: v.units.clone(), reference_lines: Vec::new() });
    }
    mf.tests = m.checks.iter().map(|c| if c.name.trim().is_empty() { c.var.clone() } else { c.name.clone() }).collect();
    mf
}

/// Overrides that go outside a lever's declared range, as sentences.
pub fn range_warnings(m: &Model, overrides: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (name, value) in overrides {
        let Some(l) = m.manifest.levers.iter().find(|l| expr::normalize(&l.name) == expr::normalize(name)) else { continue };
        if let Ok(x) = value.trim().parse::<f64>() {
            if x < l.min || x > l.max {
                out.push(format!("“{}” = {x} is outside the range {} to {} the manifest declares; results there are not vouched for", l.name, l.min, l.max));
            }
        }
    }
    out
}

/// How a goal reads its number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GoalMeasure {
    Basic(crate::model::Measure),
    /// Average over a time window.
    Mean(f64, f64),
    /// Value at a time (linear interpolation between saved times).
    At(f64),
}

pub fn parse_goal_measure(s: &str) -> Result<GoalMeasure, String> {
    let t = s.trim().to_lowercase();
    if t.is_empty() {
        return Ok(GoalMeasure::Basic(crate::model::Measure::Final));
    }
    if let Some(r) = t.strip_prefix("mean:") {
        let (a, b) = r.split_once('-').ok_or_else(|| format!("“{s}”: write mean:FROM-TO"))?;
        let (a, b): (f64, f64) = (a.trim().parse().map_err(|_| format!("“{s}”: FROM is not a number"))?, b.trim().parse().map_err(|_| format!("“{s}”: TO is not a number"))?);
        if !(a < b) {
            return Err(format!("“{s}”: FROM must be below TO"));
        }
        return Ok(GoalMeasure::Mean(a, b));
    }
    if let Some(r) = t.strip_prefix("at:") {
        return Ok(GoalMeasure::At(r.trim().parse().map_err(|_| format!("“{s}”: write at:TIME"))?));
    }
    crate::optimize::parse_measure(&t).map(GoalMeasure::Basic).map_err(|e| format!("{e}; also mean:FROM-TO and at:TIME"))
}

/// Which way is better, and for "target" the value aimed at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Min,
    Max,
    Target(f64),
}

pub fn parse_direction(s: &str) -> Result<Direction, String> {
    let t = s.trim().to_lowercase();
    match t.as_str() {
        "" | "max" => Ok(Direction::Max),
        "min" => Ok(Direction::Min),
        _ => match t.strip_prefix("target:") {
            Some(v) => v.trim().parse().map(Direction::Target).map_err(|_| format!("“{s}”: write target:VALUE")),
            None => Err(format!("direction “{s}” is not min, max or target:VALUE")),
        },
    }
}

/// The number a goal reads out of a run, through the goal's transform; `None` when the output is missing, the
/// window is empty, or the transform is undefined for the value (log of zero or less).
pub fn goal_value(run: &crate::sim::Run, g: &Goal) -> Option<f64> {
    let v = goal_value_raw(run, g)?;
    match g.transform.trim().to_lowercase().as_str() {
        "" | "none" => Some(v),
        "log" => (v > 0.0).then(|| v.ln()),
        _ => None,
    }
}

/// Transforms a goal may declare.
pub fn valid_transform(t: &str) -> bool {
    matches!(t.trim().to_lowercase().as_str(), "" | "none" | "log")
}

/// The goal's number before the transform.
pub fn goal_value_raw(run: &crate::sim::Run, g: &Goal) -> Option<f64> {
    let s = run.get(&g.output)?;
    if s.is_empty() {
        return None;
    }
    match parse_goal_measure(&g.measure).ok()? {
        GoalMeasure::Basic(m) => crate::optimize::measure(run, &g.output, m, false),
        GoalMeasure::Mean(a, b) => {
            let v: Vec<f64> = run.times.iter().zip(s.iter()).filter(|(t, _)| **t >= a - 1e-9 && **t <= b + 1e-9).map(|(_, x)| *x).collect();
            if v.is_empty() { None } else { Some(v.iter().sum::<f64>() / v.len() as f64) }
        }
        GoalMeasure::At(t) => {
            let ts = &run.times;
            if t < ts[0] - 1e-9 || t > ts[ts.len() - 1] + 1e-9 {
                return None;
            }
            let i = ts.partition_point(|x| *x <= t + 1e-9).saturating_sub(1).min(ts.len() - 1);
            if i + 1 >= ts.len() || (ts[i] - t).abs() < 1e-9 {
                return Some(s[i]);
            }
            let f = (t - ts[i]) / (ts[i + 1] - ts[i]);
            Some(s[i] + f * (s[i + 1] - s[i]))
        }
    }
}

/// A score where larger is always better: direction applied (min flips the sign, a target is minus the distance).
pub fn goal_score(g: &Goal, value: f64) -> f64 {
    match parse_direction(&g.direction).unwrap_or(Direction::Max) {
        Direction::Max => value,
        Direction::Min => -value,
        Direction::Target(t) => -(value - t).abs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Var;

    fn model() -> Model {
        let mut m = Model::default();
        let mut k = Var::new(1, "Rate", Kind::Constant, [0.0, 0.0]);
        k.equation = "4".into();
        k.lever = true;
        k.units = "MB/Day".into();
        let mut s = Var::new(2, "Tank", Kind::Stock, [0.0, 0.0]);
        s.equation = "100".into();
        m.vars = vec![k, s];
        m.checks.push(crate::checks::Check { name: "Never dry".into(), var: "Tank".into(), kind: crate::checks::CheckKind::NonNeg, ..Default::default() });
        m
    }

    #[test]
    fn a_draft_comes_from_the_levers_stocks_and_checks_and_is_consistent() {
        let mut m = model();
        m.manifest = draft(&m);
        assert_eq!((m.manifest.levers.len(), m.manifest.outputs.len(), m.manifest.tests.len()), (1, 1, 1));
        assert_eq!((m.manifest.levers[0].min, m.manifest.levers[0].max), (2.0, 6.0));
        assert!(check(&m).is_empty(), "{:?}", check(&m));
    }

    #[test]
    fn dangling_names_and_bad_ranges_are_found() {
        let mut m = model();
        m.manifest.levers.push(ManifestLever { name: "Tank".into(), min: 0.0, max: 1.0, ..Default::default() }); // a stock
        m.manifest.levers.push(ManifestLever { name: "Rate".into(), min: 5.0, max: 1.0, ..Default::default() }); // wrong way round
        m.manifest.outputs.push(ManifestOutput { variable: "Nope".into(), chart: "pie".into(), ..Default::default() });
        m.manifest.scenarios.push(ManifestScenario { name: "Fast".into(), values: BTreeMap::from([("Rate".into(), 99.0)]), ..Default::default() });
        m.manifest.bindings.push(Binding { input: "Rate".into(), source: String::new(), ..Default::default() });
        m.manifest.tests.push("Ghost check".into());
        let msgs: Vec<String> = check(&m).into_iter().map(|x| x.1).collect();
        for needle in ["not a constant", "not below max", "not a variable", "unknown chart", "outside its range", "no source", "not one of the model's checks"] {
            assert!(msgs.iter().any(|x| x.contains(needle)), "missing “{needle}” in {msgs:?}");
        }
    }

    #[test]
    fn running_outside_a_declared_range_is_called_out() {
        let mut m = model();
        m.manifest = draft(&m);
        assert!(range_warnings(&m, &[("Rate".into(), "5".into())]).is_empty());
        assert_eq!(range_warnings(&m, &[("rate".into(), "50".into())]).len(), 1);
    }

    #[test]
    fn the_manifest_survives_native_and_xmile_files_and_is_reported_lost_in_vensim() {
        let mut m = model();
        m.settings.final_time = 5.0;
        m.manifest = draft(&m);
        let native: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(native.manifest, m.manifest);
        let (xml, _) = crate::xmile::write(&m);
        let (back, _) = crate::xmile::read(&xml).unwrap();
        assert_eq!(back.manifest, m.manifest);
        let (_, loss) = crate::mdlw::write(&m);
        assert!(loss.items.iter().any(|i| i.construct == "manifest"), "{}", loss.summary());
    }

    fn run_of(vals: &[f64]) -> crate::sim::Run {
        crate::sim::Run { name: String::new(), times: (0..vals.len()).map(|i| i as f64).collect(), series: std::collections::HashMap::from([("tank".to_string(), vals.to_vec())]), time_units: String::new(), ..Default::default() }
    }

    #[test]
    fn goals_read_the_declared_number_and_bad_goals_are_refused() {
        let r = run_of(&[1.0, 4.0, 9.0, 6.0, 5.0]);
        let g = |measure: &str| Goal { name: "G".into(), output: "Tank".into(), measure: measure.into(), ..Default::default() };
        assert_eq!(goal_value(&r, &g("")), Some(5.0));
        let mut lg = g("peak");
        lg.transform = "log".into();
        assert!((goal_value(&r, &lg).unwrap() - 9.0f64.ln()).abs() < 1e-12);
        let neg = run_of(&[-1.0, -2.0]);
        assert_eq!(goal_value(&neg, &lg), None); // log of a non-positive value is a failed run
        assert_eq!(goal_value(&r, &g("peak")), Some(9.0));
        assert_eq!(goal_value(&r, &g("lowest")), Some(1.0));
        assert_eq!(goal_value(&r, &g("mean:1-3")), Some(19.0 / 3.0));
        assert_eq!(goal_value(&r, &g("at:2")), Some(9.0));
        assert_eq!(goal_value(&r, &g("at:2.5")), Some(7.5));
        assert_eq!(goal_value(&r, &g("at:9")), None);
        let mut gg = g("final");
        gg.direction = "min".into();
        assert_eq!(goal_score(&gg, 5.0), -5.0);
        gg.direction = "target:4".into();
        assert_eq!(goal_score(&gg, 5.0), -1.0);

        let mut m = model();
        m.manifest = draft(&m);
        m.manifest.goals.push(Goal { name: "ok".into(), output: "Tank".into(), measure: "mean:0-3".into(), direction: "target:50".into(), ..Default::default() });
        assert!(check(&m).is_empty(), "{:?}", check(&m));
        let native: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(native.manifest.goals, m.manifest.goals);
        let (xml, _) = crate::xmile::write(&m);
        let (back, _) = crate::xmile::read(&xml).unwrap();
        assert_eq!(back.manifest.goals, m.manifest.goals);
        m.manifest.goals.push(Goal { name: "bad".into(), output: "Nope".into(), measure: "median".into(), direction: "up".into(), policies: vec!["Ghost".into()], protected_ranking: true, weight: None, transform: "sqrt".into() });
        let msgs: Vec<String> = check(&m).into_iter().map(|x| x.1).collect();
        for needle in ["not a variable", "unknown measure", "not min, max", "not a scenario", "fewer than two", "unknown transform"] {
            assert!(msgs.iter().any(|x| x.contains(needle)), "missing “{needle}” in {msgs:?}");
        }
    }
}
