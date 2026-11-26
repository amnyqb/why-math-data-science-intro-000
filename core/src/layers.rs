//! Layers: a stack of models, each with its own question, horizon and state of completion.
//!
//! A layer is a named tier, ordered from the outermost (slow, uncertain, context) to the
//! innermost (fast, detailed, operational). Variables, submodels and sheets belong to a layer.
//! Layers talk to each other only through declared *interfaces* (contracts that are written down
//! by `freeze`), and a model with no layers behaves exactly as before: every function here
//! returns "nothing to report" when `Model::layers` is empty.
//!
//! This is metadata only: it never changes an equation, so a layered model gives the same
//! results as the same model without layers.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::expr;
use crate::model::{Kind, Model, Var};
use crate::runner;
use crate::sim::Run;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerStatus {
    /// Only an idea: nothing is modelled yet.
    #[default]
    Planned,
    /// Standing in as given series or a scenario.
    Stub,
    /// Modelled, not yet shown to hold.
    Built,
    /// Checked: runs alone, passes its checks and the decoupling test.
    Tested,
    /// Tested and compared with data.
    Calibrated,
}

impl LayerStatus {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "planned" => Ok(Self::Planned),
            "stub" => Ok(Self::Stub),
            "built" => Ok(Self::Built),
            "tested" => Ok(Self::Tested),
            "calibrated" => Ok(Self::Calibrated),
            o => Err(format!("unknown status “{o}” (planned, stub, built, tested, calibrated)")),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Stub => "stub",
            Self::Built => "built",
            Self::Tested => "tested",
            Self::Calibrated => "calibrated",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    /// 0 = outermost; larger numbers are further in.
    #[serde(default)]
    pub order: i32,
    /// The one question this layer exists to answer.
    #[serde(default)]
    pub question: String,
    /// How far ahead (from the start of the run) this layer's numbers are trusted.
    #[serde(default)]
    pub horizon: Option<f64>,
    /// Beyond this, structure only: illustrative, not a forecast.
    #[serde(default)]
    pub structural_limit: Option<f64>,
    #[serde(default)]
    pub status: LayerStatus,
    #[serde(default)]
    pub colour: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub owner: String,
    /// Where this layer's exogenous inputs come from (nature, society, market…).
    #[serde(default)]
    pub spheres: Vec<String>,
}

/// A variable that crosses from one layer to another: the contract between them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    pub var: String,
    pub from: u64,
    pub to: u64,
    #[serde(default)]
    pub units: String,
    /// "information" or "material".
    #[serde(default)]
    pub kind: String,
    /// "as_is", "average", "sum" or "sampled".
    #[serde(default)]
    pub time: String,
}

/// A layer standing in as given formulas (or series) for its outgoing variables.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stub {
    pub layer: u64,
    pub name: String,
    /// variable → formula
    #[serde(default)]
    pub series: BTreeMap<String, String>,
}

fn key(s: &str) -> String {
    expr::normalize(s)
}

impl Model {
    pub fn layer_by_name(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| key(&l.name) == key(name))
    }
    pub fn layer_name(&self, id: u64) -> String {
        self.layers.iter().find(|l| l.id == id).map(|l| l.name.clone()).unwrap_or_else(|| format!("layer {id}"))
    }
}

/// The layer a variable belongs to: its own setting, else its sheet's, else its submodel's.
pub fn layer_of(m: &Model, v: &Var) -> Option<u64> {
    if v.layer.is_some() {
        return v.layer;
    }
    let sheet = m.sheets.iter().find(|s| s.id == v.view).or_else(|| m.sheets.first());
    if let Some(s) = sheet {
        if s.layer.is_some() {
            return s.layer;
        }
        if let Some(mid) = s.module {
            if let Some(md) = m.modules.iter().find(|x| x.id == mid) {
                if md.layer.is_some() {
                    return md.layer;
                }
            }
        }
    }
    None
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Crossing {
    pub var: String,
    pub from: u64,
    pub to: u64,
    /// "information" or "material"
    pub kind: String,
    /// The variables on the receiving side that use it.
    pub used_by: Vec<String>,
}

/// Every place where something passes from one layer to another, derived from the equations and
/// from where flows start and end.
pub fn crossings(m: &Model) -> Vec<Crossing> {
    if m.layers.is_empty() {
        return Vec::new();
    }
    let by_key: HashMap<String, &Var> = m.vars.iter().map(|v| (key(&v.name), v)).collect();
    let lay: HashMap<u64, Option<u64>> = m.vars.iter().map(|v| (v.id, layer_of(m, v))).collect();
    let mut found: BTreeMap<(String, u64, u64), Crossing> = BTreeMap::new();
    let mut add = |var: &Var, a: Option<u64>, b: Option<u64>, kind: &str, user: &str| {
        if let (Some(a), Some(b)) = (a, b) {
            if a != b {
                let e = found.entry((var.name.clone(), a, b)).or_insert_with(|| Crossing { var: var.name.clone(), from: a, to: b, kind: kind.into(), used_by: Vec::new() });
                if !e.used_by.contains(&user.to_string()) {
                    e.used_by.push(user.to_string());
                }
            }
        }
    };
    for v in &m.vars {
        let lv = lay[&v.id];
        let mut refs = Vec::new();
        let mut eqs: Vec<&str> = vec![&v.equation];
        eqs.extend(v.by_item.iter().map(|i| i.equation.as_str()));
        for eq in eqs {
            if let Ok(e) = expr::parse(eq) {
                expr::references(&e, &mut refs);
            }
        }
        for r in refs {
            if let Some(src) = by_key.get(&r) {
                add(src, lay[&src.id], lv, "information", &v.name);
            }
        }
        if v.kind == Kind::Flow {
            if let Some(s) = v.to.and_then(|i| m.var(i)) {
                add(v, lv, lay[&s.id], "material", &s.name);
            }
            if let Some(s) = v.from.and_then(|i| m.var(i)) {
                add(v, lay[&s.id], lv, "material", &v.name);
            }
        }
    }
    found.into_values().collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sev {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LayerIssue {
    pub severity: Sev,
    pub code: String,
    pub message: String,
    pub var: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LayerReport {
    pub issues: Vec<LayerIssue>,
    pub crossings: usize,
    pub contracts: usize,
    /// Groups of layers that feed each other (declared or not).
    pub cross_layer_loops: Vec<Vec<String>>,
}

impl LayerReport {
    pub fn errors(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Sev::Error).count()
    }
}

fn issue(sev: Sev, code: &str, var: Option<&str>, message: String) -> LayerIssue {
    LayerIssue { severity: sev, code: code.into(), message, var: var.map(|s| s.to_string()) }
}

/// Layers that feed each other, as groups (strongly connected components of the layer graph).
fn layer_cycles(m: &Model, edges: &BTreeSet<(u64, u64)>) -> Vec<Vec<String>> {
    let ids: Vec<u64> = m.layers.iter().map(|l| l.id).collect();
    let reach = |a: u64| -> BTreeSet<u64> {
        let mut seen = BTreeSet::new();
        let mut stack = vec![a];
        while let Some(x) = stack.pop() {
            for (f, t) in edges {
                if *f == x && seen.insert(*t) {
                    stack.push(*t);
                }
            }
        }
        seen
    };
    let mut done: BTreeSet<u64> = BTreeSet::new();
    let mut out = Vec::new();
    for &a in &ids {
        if done.contains(&a) {
            continue;
        }
        let ra = reach(a);
        let group: Vec<u64> = ids.iter().copied().filter(|&b| b == a || (ra.contains(&b) && reach(b).contains(&a))).collect();
        if group.len() > 1 {
            done.extend(group.iter().copied());
            out.push(group.iter().map(|g| m.layer_name(*g)).collect());
        }
    }
    out
}

/// What is wrong with the layering: unassigned items, crossings nobody declared, contracts that
/// no longer match, horizons that contradict the order of feeding.
pub fn check(m: &Model) -> LayerReport {
    let mut rep = LayerReport::default();
    if m.layers.is_empty() {
        return rep;
    }
    let ids: BTreeSet<u64> = m.layers.iter().map(|l| l.id).collect();
    let mut names = BTreeSet::new();
    for l in &m.layers {
        if !names.insert(key(&l.name)) {
            rep.issues.push(issue(Sev::Error, "layer-name", None, format!("two layers are called “{}”", l.name)));
        }
        if let (Some(h), Some(s)) = (l.horizon, l.structural_limit) {
            if s < h {
                rep.issues.push(issue(Sev::Error, "layer-limit", None, format!("layer “{}”: the structural limit ({s}) is shorter than the horizon ({h})", l.name)));
            }
        }
    }
    for v in &m.vars {
        match layer_of(m, v) {
            None => rep.issues.push(issue(Sev::Error, "layer-unassigned", Some(&v.name), format!("“{}” is not in any layer", v.name))),
            Some(id) if !ids.contains(&id) => rep.issues.push(issue(Sev::Error, "layer-unknown", Some(&v.name), format!("“{}” is in layer {id}, which does not exist", v.name))),
            _ => {}
        }
    }
    let cr = crossings(m);
    rep.crossings = cr.len();
    rep.contracts = m.interfaces.len();
    let declared: BTreeSet<(String, u64, u64)> = m.interfaces.iter().map(|i| (key(&i.var), i.from, i.to)).collect();
    let present: BTreeSet<(String, u64, u64)> = cr.iter().map(|c| (key(&c.var), c.from, c.to)).collect();
    if m.interfaces.is_empty() && !cr.is_empty() {
        rep.issues.push(issue(Sev::Warning, "layer-no-contracts", None, format!("{} variable(s) cross between layers and no interface is declared yet (run freeze to write them down)", cr.len())));
    }
    if !m.interfaces.is_empty() {
        for c in &cr {
            if !declared.contains(&(key(&c.var), c.from, c.to)) {
                rep.issues.push(issue(
                    Sev::Error,
                    "layer-crossing",
                    Some(&c.var),
                    format!("“{}” passes from layer {} to layer {} (used by {}) and no interface declares it", c.var, m.layer_name(c.from), m.layer_name(c.to), c.used_by.join(", ")),
                ));
            }
        }
    }
    for i in &m.interfaces {
        let v = m.vars.iter().find(|v| key(&v.name) == key(&i.var));
        let (a, b) = (m.layer_name(i.from), m.layer_name(i.to));
        if !ids.contains(&i.from) || !ids.contains(&i.to) {
            rep.issues.push(issue(Sev::Error, "layer-contract", Some(&i.var), format!("the interface “{}” names a layer that does not exist", i.var)));
            continue;
        }
        match v {
            None => rep.issues.push(issue(Sev::Error, "layer-contract", Some(&i.var), format!("the interface “{}” ({a} to {b}) names a variable that no longer exists", i.var))),
            Some(v) => {
                if !present.contains(&(key(&i.var), i.from, i.to)) {
                    rep.issues.push(issue(Sev::Warning, "layer-contract-unused", Some(&i.var), format!("the interface “{}” from {a} to {b} is no longer used by any equation", i.var)));
                }
                let (u1, u2) = (v.units.trim().to_lowercase(), i.units.trim().to_lowercase());
                if !u1.is_empty() && !u2.is_empty() && u1 != u2 {
                    rep.issues.push(issue(Sev::Error, "layer-units", Some(&i.var), format!("“{}” is in “{}” but its interface says “{}”", i.var, v.units, i.units)));
                }
            }
        }
    }
    let edges: BTreeSet<(u64, u64)> = cr.iter().map(|c| (c.from, c.to)).chain(m.interfaces.iter().map(|i| (i.from, i.to))).collect();
    rep.cross_layer_loops = layer_cycles(m, &edges);
    let in_loop: BTreeSet<String> = rep.cross_layer_loops.iter().flatten().map(|n| key(n)).collect();
    // a layer's horizon may not exceed that of a built layer that feeds it (a stub has none)
    for (f, t) in &edges {
        let (Some(lf), Some(lt)) = (m.layers.iter().find(|l| l.id == *f), m.layers.iter().find(|l| l.id == *t)) else { continue };
        if lf.status <= LayerStatus::Stub || lt.status <= LayerStatus::Stub {
            continue;
        }
        if let (Some(hf), Some(ht)) = (lf.horizon, lt.horizon) {
            if ht > hf {
                let both = in_loop.contains(&key(&lf.name)) && in_loop.contains(&key(&lt.name));
                let sev = if both { Sev::Warning } else { Sev::Error };
                rep.issues.push(issue(sev, "layer-horizon", None, format!("layer “{}” (horizon {ht}) is fed by “{}” (horizon {hf}): its horizon cannot be longer", lt.name, lf.name)));
            }
        }
    }
    for g in &rep.cross_layer_loops {
        rep.issues.push(issue(Sev::Warning, "layer-cycle", None, format!("layers feed each other: {}", g.join(", "))));
    }
    rep
}

/// The interfaces that match the model as it is now; time treatment and units already written
/// down are kept.
pub fn frozen(m: &Model) -> Vec<Interface> {
    crossings(m)
        .into_iter()
        .map(|c| {
            let old = m.interfaces.iter().find(|i| key(&i.var) == key(&c.var) && i.from == c.from && i.to == c.to);
            let units = m.vars.iter().find(|v| key(&v.name) == key(&c.var)).map(|v| v.units.clone()).unwrap_or_default();
            Interface {
                var: c.var,
                from: c.from,
                to: c.to,
                units: old.map(|o| o.units.clone()).filter(|u| !u.is_empty()).unwrap_or(units),
                kind: c.kind,
                time: old.map(|o| o.time.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| "as_is".into()),
            }
        })
        .collect()
}

// ------------------------------------------------------------------ horizons

/// The time up to which a layer's numbers are trusted, and the time beyond which only structure.
pub fn horizon_times(m: &Model, layer: &Layer) -> (Option<f64>, Option<f64>) {
    let t0 = m.settings.initial_time;
    (layer.horizon.map(|h| t0 + h), layer.structural_limit.map(|h| t0 + h))
}

/// Layers whose horizon ends before the run does, with the variables they own.
pub fn horizon_report(m: &Model) -> Vec<Value> {
    let end = m.settings.final_time;
    m.layers
        .iter()
        .filter_map(|l| {
            let (trusted, structural) = horizon_times(m, l);
            let t = trusted?;
            if t >= end {
                return None;
            }
            let vars: Vec<String> = m.vars.iter().filter(|v| layer_of(m, v) == Some(l.id)).map(|v| v.name.clone()).collect();
            Some(json!({
                "layer": l.name,
                "trusted_until": t,
                "beyond": "ranges only",
                "structural_limit_at": structural,
                "beyond_structural_limit": structural.map(|s| s < end).unwrap_or(false).then_some("Illustrative structure, not a forecast"),
                "variables": vars,
            }))
        })
        .collect()
}

/// Why a goal on `var` measured at `at` (None = over or at the end of the run) cannot be trusted,
/// when it falls beyond the horizon of the layer that owns it.
pub fn goal_beyond_horizon(m: &Model, var: &str, at: Option<f64>) -> Option<String> {
    let v = m.vars.iter().find(|v| key(&v.name) == key(var))?;
    let l = m.layers.iter().find(|l| Some(l.id) == layer_of(m, v))?;
    let (trusted, _) = horizon_times(m, l);
    let t = at.unwrap_or(m.settings.final_time);
    let limit = trusted?;
    (t > limit).then(|| format!("the goal on “{}” is measured at time {t}, beyond the horizon of layer “{}” (trusted until {limit}); pass --allow-beyond-horizon to go on", v.name, l.name))
}

/// The last moment a manifest goal's measure reads: `at:T` reads T, `mean:A-B` reads up to B,
/// everything else (final, peak, total…) reads to the end of the run.
pub fn measure_end(measure: &str) -> Option<f64> {
    let m = measure.trim().to_lowercase();
    if let Some(t) = m.strip_prefix("at:") {
        return t.trim().parse().ok();
    }
    if let Some(r) = m.strip_prefix("mean:") {
        return r.rsplit_once('-').and_then(|(_, b)| b.trim().parse().ok());
    }
    None
}

// ------------------------------------------------------------------ isolated runs

/// What stands in for a variable that comes from another layer.
#[derive(Clone, Debug)]
pub enum Input {
    Series(Vec<(f64, f64)>),
    Formula(String),
}

/// The variables other layers supply to this one (lookups are shared functions, not inputs).
pub fn incoming(m: &Model, layer: u64) -> Vec<(String, u64)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for c in crossings(m) {
        if c.to != layer {
            continue;
        }
        let kind = m.vars.iter().find(|v| key(&v.name) == key(&c.var)).map(|v| v.kind);
        if kind == Some(Kind::Lookup) {
            continue;
        }
        if seen.insert(key(&c.var)) {
            out.push((c.var, c.from));
        }
    }
    out
}

const SYNTH: &str = "__in";

/// A copy of the model holding only one layer, its incoming variables replaced by `inputs`.
pub fn isolate(m: &Model, layer: &Layer, inputs: &HashMap<String, Input>) -> Result<Model, String> {
    let inc = incoming(m, layer.id);
    let missing: Vec<&str> = inc.iter().filter(|(n, _)| !inputs.contains_key(&key(n))).map(|(n, _)| n.as_str()).collect();
    if !missing.is_empty() {
        return Err(format!("layer “{}” needs these from other layers and nothing supplies them: {}", layer.name, missing.join(", ")));
    }
    let mut sub = m.clone();
    let inc_keys: BTreeSet<String> = inc.iter().map(|(n, _)| key(n)).collect();
    let own: BTreeSet<u64> = m.vars.iter().filter(|v| layer_of(m, v) == Some(layer.id)).map(|v| v.id).collect();
    // lookups used by the layer from elsewhere stay, as shared functions
    let mut keep: BTreeSet<u64> = own.clone();
    for c in crossings(m).into_iter().filter(|c| c.to == layer.id) {
        if let Some(v) = m.vars.iter().find(|v| key(&v.name) == key(&c.var)) {
            if v.kind == Kind::Lookup || inc_keys.contains(&key(&v.name)) {
                keep.insert(v.id);
            }
        }
    }
    sub.vars.retain(|v| keep.contains(&v.id));
    let alive: BTreeSet<u64> = sub.vars.iter().map(|v| v.id).collect();
    sub.links.retain(|l| alive.contains(&l.from) && alive.contains(&l.to));
    let mut synth: Vec<Var> = Vec::new();
    let mut next = sub.next_id.max(m.next_id) + 1;
    for v in &mut sub.vars {
        if v.kind == Kind::Flow {
            if v.from.map(|i| !alive.contains(&i)).unwrap_or(false) {
                v.from = None;
            }
            if v.to.map(|i| !alive.contains(&i)).unwrap_or(false) {
                v.to = None;
            }
        }
        if !inc_keys.contains(&key(&v.name)) {
            continue;
        }
        let inp = &inputs[&key(&v.name)];
        v.by_item.clear();
        v.set = None;
        if v.kind == Kind::Stock || v.kind == Kind::Constant {
            v.kind = Kind::Aux;
        }
        if v.kind == Kind::Flow {
            v.from = None;
            v.to = v.to.filter(|i| alive.contains(i));
        }
        match inp {
            Input::Formula(f) => v.equation = f.clone(),
            Input::Series(pts) => {
                if pts.is_empty() {
                    return Err(format!("the series for “{}” is empty", v.name));
                }
                let lname = format!("{}{SYNTH}", v.name);
                let mut lv = Var::new(next, &lname, Kind::Lookup, v.pos);
                next += 1;
                lv.table = pts.iter().map(|(t, x)| [*t, *x]).collect();
                lv.view = v.view;
                lv.layer = Some(layer.id);
                v.equation = format!("{lname}(TIME)");
                synth.push(lv);
            }
        }
    }
    sub.vars.extend(synth);
    sub.next_id = next + 1;
    sub.checks.clear();
    sub.scenarios.clear();
    sub.manifest = Default::default();
    sub.layers.clear();
    sub.interfaces.clear();
    sub.stubs.clear();
    // a variable's layer is meaningless in a model without layers
    for v in &mut sub.vars {
        v.layer = None;
    }
    Ok(sub)
}

fn series_of(run: &Run, name: &str) -> Option<Vec<(f64, f64)>> {
    run.get(name).map(|s| run.times.iter().copied().zip(s.iter().copied()).collect())
}

/// What the full model gave to this layer's incoming variables.
pub fn recorded_inputs(m: &Model, layer: u64, full: &Run) -> Result<HashMap<String, Input>, String> {
    let mut out = HashMap::new();
    for (n, _) in incoming(m, layer) {
        let s = series_of(full, &n).ok_or_else(|| format!("the full run has no series for “{n}”"))?;
        out.insert(key(&n), Input::Series(s));
    }
    Ok(out)
}

/// Inputs from a table of recorded series: a header row `time,Var,Var…` then one row per time.
pub fn inputs_from_csv(text: &str) -> Result<HashMap<String, Input>, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let head: Vec<String> = lines.next().ok_or("the CSV is empty")?.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
    if head.len() < 2 || key(&head[0]) != "time" {
        return Err("the first column of the CSV must be called time".into());
    }
    let mut cols: Vec<Vec<(f64, f64)>> = vec![Vec::new(); head.len() - 1];
    for (r, line) in lines.enumerate() {
        let cells: Vec<&str> = line.split(',').collect();
        if cells.len() != head.len() {
            return Err(format!("row {} has {} cells, the header has {}", r + 2, cells.len(), head.len()));
        }
        let t: f64 = cells[0].trim().parse().map_err(|_| format!("row {}: “{}” is not a number", r + 2, cells[0]))?;
        for (c, cell) in cells.iter().enumerate().skip(1) {
            let x: f64 = cell.trim().parse().map_err(|_| format!("row {}: “{}” is not a number", r + 2, cell))?;
            cols[c - 1].push((t, x));
        }
    }
    Ok(head.iter().skip(1).zip(cols).map(|(h, c)| (key(h), Input::Series(c))).collect())
}

pub fn inputs_from_stub(m: &Model, layer: u64, stub: &Stub) -> Result<HashMap<String, Input>, String> {
    let mut out = HashMap::new();
    for (n, from) in incoming(m, layer) {
        if from != stub.layer {
            continue;
        }
        let f = stub.series.iter().find(|(k, _)| key(k) == key(&n)).map(|(_, f)| f.clone()).ok_or_else(|| format!("the stub “{}” gives no formula for “{n}”", stub.name))?;
        out.insert(key(&n), Input::Formula(f));
    }
    Ok(out)
}

/// The variables of a layer that show behaviour (not constants or lookups).
pub fn behaviour_vars(m: &Model, layer: u64) -> Vec<String> {
    m.vars.iter().filter(|v| layer_of(m, v) == Some(layer) && v.kind != Kind::Constant && v.kind != Kind::Lookup).map(|v| v.name.clone()).collect()
}

/// Root-mean-square gap between two runs, as a fraction of each variable's range; the worst
/// variable is reported.
pub fn worst_nrmse(a: &Run, b: &Run, vars: &[String]) -> (f64, Option<String>) {
    let mut worst = (0.0, None);
    for n in vars {
        let (Some(x), Some(y)) = (a.get(n), b.get(n)) else { continue };
        let k = x.len().min(y.len());
        if k == 0 {
            continue;
        }
        let rmse = (x.iter().zip(y).map(|(p, q)| (p - q) * (p - q)).sum::<f64>() / k as f64).sqrt();
        let (lo, hi) = x.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), v| (l.min(*v), h.max(*v)));
        let scale = (hi - lo).max(hi.abs().max(lo.abs()) * 1e-9).max(1e-12);
        let e = rmse / scale;
        if e > worst.0 {
            worst = (e, Some(n.clone()));
        }
    }
    worst
}

/// Run one layer alone.
pub fn run_layer(m: &Model, layer: &Layer, inputs: &HashMap<String, Input>, overrides: &[(String, String)]) -> Result<Run, String> {
    let sub = isolate(m, layer, inputs)?;
    runner::run_with(&sub, overrides)
}

/// The decoupling test. Two measures:
///
/// * `plumbing_nrmse`: the layer run alone on inputs recorded from the full run against its
///   behaviour inside the full run. It must be near zero or the isolation is broken.
/// * `feedback_nrmse`: change one of the layer's own constants by 10%; run the full model, and
///   run the layer alone with the inputs recorded from the *unchanged* run. The gap is what the
///   other layers do in reply: the strength of feedback across the boundary.
pub fn decouple(m: &Model, layer: &Layer, max_nrmse: f64) -> Result<Value, String> {
    let full = runner::run_with(m, &[])?;
    let inputs = recorded_inputs(m, layer.id, &full)?;
    let vars = behaviour_vars(m, layer.id);
    let alone = run_layer(m, layer, &inputs, &[])?;
    let (plumbing, worst) = worst_nrmse(&full, &alone, &vars);
    let mut rows = Vec::new();
    let mut feedback = 0.0f64;
    for v in m.vars.iter().filter(|v| layer_of(m, v) == Some(layer.id) && v.kind == Kind::Constant) {
        let Ok(x) = v.equation.trim().parse::<f64>() else { continue };
        if x == 0.0 {
            continue;
        }
        let ov = vec![(v.name.clone(), format!("{}", x * 1.1))];
        let (Ok(full2), Ok(alone2)) = (runner::run_with(m, &ov), run_layer(m, layer, &inputs, &ov)) else { continue };
        let (e, w) = worst_nrmse(&full2, &alone2, &vars);
        feedback = feedback.max(e);
        rows.push(json!({"changed": v.name, "nrmse": e, "most_affected": w}));
    }
    let n_in = incoming(m, layer.id).len();
    Ok(json!({
        "layer": layer.name,
        "inputs_from_other_layers": n_in,
        "max_nrmse": max_nrmse,
        "plumbing_nrmse": plumbing,
        "plumbing_worst": worst,
        "ok": plumbing <= max_nrmse,
        "feedback_nrmse": feedback,
        "decoupled": feedback <= max_nrmse,
        "feedback_by_constant": rows,
        "note": "ok: the isolated layer reproduces its behaviour inside the full model. decoupled: the other layers do not react when this layer's constants change (false = feedback across the boundary).",
    }))
}

/// One line per layer for `dyn layers`.
pub fn summary(m: &Model) -> Value {
    let cr = crossings(m);
    let rep = check(m);
    let layers: Vec<Value> = {
        let mut ls: Vec<&Layer> = m.layers.iter().collect();
        ls.sort_by_key(|l| (l.order, l.id));
        ls.into_iter()
            .map(|l| {
                let count = |k: Option<Kind>| m.vars.iter().filter(|v| layer_of(m, v) == Some(l.id) && k.map(|k| v.kind == k).unwrap_or(true)).count();
                json!({
                    "name": l.name, "order": l.order, "status": l.status.label(), "question": l.question,
                    "horizon": l.horizon, "structural_limit": l.structural_limit,
                    "variables": count(None), "stocks": count(Some(Kind::Stock)),
                    "modules": m.modules.iter().filter(|x| x.layer == Some(l.id)).count(),
                    "sheets": m.sheets.iter().filter(|x| x.layer == Some(l.id)).count(),
                    "interfaces_in": m.interfaces.iter().filter(|i| i.to == l.id).count(),
                    "interfaces_out": m.interfaces.iter().filter(|i| i.from == l.id).count(),
                    "crossings_in": cr.iter().filter(|c| c.to == l.id).count(),
                    "crossings_out": cr.iter().filter(|c| c.from == l.id).count(),
                    "stub": m.stubs.iter().filter(|s| s.layer == l.id).map(|s| s.name.clone()).collect::<Vec<_>>(),
                })
            })
            .collect()
    };
    json!({
        "layers": layers,
        "unassigned": m.vars.iter().filter(|v| layer_of(m, v).is_none()).count(),
        "interfaces": m.interfaces.len(),
        "crossings": cr.len(),
        "cross_layer_loops": rep.cross_layer_loops,
        "errors": rep.errors(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Model, Var};

    /// World (a growing climate) feeds Firm (a stock that grows with it); Firm's size feeds Ops
    /// only as a report. Optionally Ops reacts back on the world (a cross-layer loop).
    pub fn layered(feedback: bool) -> Model {
        let mut m = Model::default();
        m.title = "layered".into();
        m.settings.final_time = 40.0;
        m.settings.time_step = 0.5;
        m.settings.initial_time = 0.0;
        let mut add = |m: &mut Model, name: &str, kind: Kind, eq: &str, layer: u64| -> u64 {
            let id = m.new_id();
            let mut v = Var::new(id, name, kind, [0.0, 0.0]);
            v.equation = eq.into();
            v.layer = Some(layer);
            m.vars.push(v);
            id
        };
        m.layers = vec![
            Layer { id: 1, name: "World".into(), order: 0, status: LayerStatus::Built, horizon: Some(60.0), ..Default::default() },
            Layer { id: 2, name: "Firm".into(), order: 1, status: LayerStatus::Built, horizon: Some(36.0), ..Default::default() },
        ];
        add(&mut m, "Climate", Kind::Aux, if feedback { "1 + 0.02 * TIME - 0.001 * Firm" } else { "1 + 0.02 * TIME" }, 1);
        let firm = add(&mut m, "Firm", Kind::Stock, "10", 2);
        add(&mut m, "Growth rate", Kind::Constant, "0.05", 2);
        let g = add(&mut m, "Growth", Kind::Flow, "Firm * Growth rate * Climate", 2);
        m.vars.iter_mut().find(|v| v.id == g).unwrap().to = Some(firm);
        m
    }

    #[test]
    fn crossings_found_and_frozen_and_checked() {
        let mut m = layered(false);
        let c = crossings(&m);
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].var.as_str(), c[0].from, c[0].to), ("Climate", 1, 2));
        assert_eq!(check(&m).errors(), 0);
        m.interfaces = frozen(&m);
        assert_eq!(m.interfaces.len(), 1);
        assert_eq!(check(&m).errors(), 0);
        // an undeclared crossing: World now reads Firm
        let w = m.vars.iter_mut().find(|v| v.name == "Climate").unwrap();
        w.equation = "1 + 0.001 * Firm".into();
        let r = check(&m);
        assert!(r.issues.iter().any(|i| i.code == "layer-crossing" && i.message.contains("Firm") && i.message.contains("World")));
        assert_eq!(r.cross_layer_loops.len(), 1);
    }

    #[test]
    fn horizon_order_is_checked_and_goals_beyond_refused() {
        let mut m = layered(false);
        m.layers[1].horizon = Some(80.0); // Firm longer than World, which feeds it
        assert!(check(&m).issues.iter().any(|i| i.code == "layer-horizon"));
        m.layers[1].horizon = Some(30.0);
        assert!(goal_beyond_horizon(&m, "Firm", Some(35.0)).is_some());
        assert!(goal_beyond_horizon(&m, "Firm", Some(25.0)).is_none());
        assert_eq!(horizon_report(&m).len(), 1);
    }

    #[test]
    fn layer_alone_matches_full_run_and_feedback_is_measured() {
        for feedback in [false, true] {
            let m = layered(feedback);
            let firm = m.layers[1].clone();
            let d = decouple(&m, &firm, 0.01).unwrap();
            assert!(d["ok"].as_bool().unwrap(), "plumbing {}", d["plumbing_nrmse"]);
            assert!(d["plumbing_nrmse"].as_f64().unwrap() < 1e-9);
            // the Firm's own constants never change the world unless the world reads Firm
            assert_eq!(d["decoupled"].as_bool().unwrap(), !feedback, "{d}");
        }
    }

    #[test]
    fn stub_replaces_the_outer_layer() {
        let m = layered(false);
        let stub = Stub { layer: 1, name: "flat".into(), series: [("Climate".to_string(), "1".to_string())].into() };
        let inputs = inputs_from_stub(&m, 2, &stub).unwrap();
        let run = run_layer(&m, &m.layers[1], &inputs, &[]).unwrap();
        let end = *run.get("Firm").unwrap().last().unwrap();
        assert!(end > 11.0, "growth happens: {end}");
        let bad = Stub { layer: 1, name: "none".into(), series: Default::default() };
        assert!(inputs_from_stub(&m, 2, &bad).is_err());
    }
}
