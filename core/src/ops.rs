//! The operations API. Every change to a model, by a person's script or by an AI agent, is a
//! batch of `Op`s. A batch is applied to a copy, the copy is validated, and only a batch that
//! passes becomes the model: all or nothing. Each committed batch is logged with who made it,
//! when, and exactly what it did, so changes can be audited and undone.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::checks::{Check, CheckWhen};
use crate::expr;
use crate::model::{ItemSet, Kind, Link, Method, Model, Var};
use crate::validate::{self, Issue, Severity};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    AddVar {
        name: String,
        /// stock, flow, aux, constant or lookup
        kind: String,
        #[serde(default)]
        equation: String,
        #[serde(default)]
        units: String,
        #[serde(default)]
        doc: String,
        /// Flows: the stock they drain and fill (names; leave out for outside the model).
        #[serde(default)]
        from: Option<String>,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        non_negative: bool,
        /// The layer it belongs to (leave out for the layer of its sheet).
        #[serde(default)]
        layer: Option<String>,
    },
    RemoveVar { name: String },
    RenameVar { name: String, to: String },
    SetEquation { name: String, equation: String },
    SetUnits { name: String, units: String },
    SetDoc { name: String, doc: String },
    SetNonNegative { name: String, on: bool },
    SetFlow { name: String, from: Option<String>, to: Option<String> },
    /// A lookup's curve, as (x, y) points.
    SetTable { name: String, points: Vec<[f64; 2]> },
    Connect { from: String, to: String, #[serde(default)] polarity: String },
    Disconnect { from: String, to: String },
    /// initial_time, final_time, time_step, time_units, method (euler, rk2, rk3, rk4), title
    SetSetting { key: String, value: String },
    AddSet { name: String, items: Vec<String>, #[serde(default)] parent: Option<String>, #[serde(default)] parent_of: Vec<String> },
    /// Make a variable exist once per item of a set (None = back to a single one).
    SetPer { var: String, set: Option<String> },
    SetItemValue { var: String, item: String, equation: String },
    SetLever { var: String, on: bool },
    /// Give a constant a slider range.
    SetSlider { var: String, min: f64, max: f64 },
    AddCheck {
        name: String,
        #[serde(default)]
        var: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
        #[serde(default)]
        when: Option<String>,
        /// bounds (default), nonneg, reference or balance
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        tolerance: Option<f64>,
        #[serde(default)]
        relative: bool,
        #[serde(default)]
        reference: Vec<[f64; 2]>,
        #[serde(default)]
        stocks: Vec<String>,
        #[serde(default)]
        inflows: Vec<String>,
        #[serde(default)]
        outflows: Vec<String>,
        /// A benchmark: test at this time only, and cite where the number comes from.
        #[serde(default)]
        at: Option<f64>,
        #[serde(default)]
        source: String,
        /// "validity" (the range the model is trusted in) or "promise" (meant to break outside it); empty for an invariant.
        #[serde(default)]
        role: String,
    },
    RemoveCheck { name: String },
    AddFunction { name: String, params: Vec<String>, body: String, #[serde(default)] doc: String },
    RemoveFunction { name: String },
    /// A reusable module: its variables (local names) and its typed ports.
    DefineModule { name: String, #[serde(default)] description: String, vars: Vec<crate::modules::DefVar>, #[serde(default)] ports: Vec<crate::modules::PortDef> },
    RemoveModule { name: String },
    /// A use of a module, with parameter values (local constant → value) and in-port formulas.
    Instantiate { def: String, name: String, #[serde(default)] params: std::collections::BTreeMap<String, String>, #[serde(default)] inputs: std::collections::BTreeMap<String, String> },
    SetInstanceParam { instance: String, param: String, value: String },
    SetInstanceInput { instance: String, port: String, formula: String },
    RemoveInstance { name: String },
    /// Replace the manifest: levers with ranges, outputs, scenarios, bindings, validity, tests.
    SetManifest { manifest: crate::manifest::Manifest },
    /// A new tier in the stack of layers.
    AddLayer {
        name: String,
        #[serde(default)]
        order: Option<i32>,
        #[serde(default)]
        question: String,
        #[serde(default)]
        horizon: Option<f64>,
        #[serde(default)]
        structural_limit: Option<f64>,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        colour: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        spheres: Vec<String>,
    },
    /// Change one field of a layer: name, order, question, horizon, structural_limit, status, colour, description, owner, spheres.
    SetLayer { name: String, key: String, value: String },
    /// Remove a layer; what it held becomes unassigned, and its interfaces and stubs go.
    RemoveLayer { name: String },
    /// Write the purpose of the model: the problem, the kind of model, what is out of scope.
    SetPurpose {
        problem: String,
        #[serde(default)]
        kind: String,
        #[serde(default)]
        out_of_scope: String,
    },
    /// Add an objective with a test, and the variable that measures it.
    AddObjective {
        title: String,
        #[serde(default)]
        kind: String,
        #[serde(default)]
        test: String,
        #[serde(default)]
        var: String,
    },
    /// Remove an objective by its title.
    RemoveObjective { title: String },
    /// Group variables into a subsystem (shown as a frame in the architecture view).
    AddGroup { name: String, vars: Vec<String> },
    /// Put variables in a group, making the group if it does not exist yet (the doctor's "Conversions" group).
    AddToGroup { name: String, vars: Vec<String> },
    /// Put variables, a sheet or a submodel in a layer.
    AssignLayer {
        layer: String,
        #[serde(default)]
        vars: Vec<String>,
        #[serde(default)]
        sheet: Option<String>,
        #[serde(default)]
        module: Option<String>,
    },
    /// The sign of an arrow of a causal loop sheet (or of a diagram arrow when the sheet is not named).
    SetCldPolarity { from: String, to: String, polarity: String, #[serde(default)] sheet: Option<String> },
    /// Record the result of a calibration (parameters, fits, correlations).
    SetCalibration { calibration: crate::model::Calibration },
    /// Write the variables that cross between layers down as the interface contracts.
    FreezeInterfaces {},
    /// A layer standing in as given formulas for its outgoing variables.
    AddStub { layer: String, name: String, series: std::collections::BTreeMap<String, String> },
    RemoveStub { name: String },
}

fn committed() -> String {
    "committed".into()
}

/// One batch that was submitted: committed, or rejected with what was wrong.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// Order of submission, counting rejected batches too.
    pub seq: u64,
    /// How many batches had been committed once this one was handled (the model's revision).
    #[serde(default)]
    pub revision: u64,
    /// "committed" or "rejected".
    #[serde(default = "committed")]
    pub result: String,
    /// Why the batch was made, in the actor's words (optional).
    #[serde(default)]
    pub reason: Option<String>,
    /// For a rejected batch: the reason and every validation error.
    #[serde(default)]
    pub problems: Vec<String>,
    /// Who made it: "user", "agent:claude", a script name …
    pub actor: String,
    /// Seconds since 1970.
    pub at: u64,
    pub ops: Vec<Op>,
    /// How many warnings the model had after this batch.
    pub warnings: usize,
}

/// A batch that was refused, and why. Nothing changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rejected {
    /// The operation that failed (index into the batch), when one did.
    pub op_index: Option<usize>,
    pub reason: String,
    /// What validation found, when the batch applied but the result did not pass.
    pub issues: Vec<Issue>,
}

impl Rejected {
    fn at(i: usize, reason: impl Into<String>) -> Self {
        Rejected { op_index: Some(i), reason: reason.into(), issues: Vec::new() }
    }
}

fn kind_of(s: &str) -> Result<Kind, String> {
    Ok(match s.trim().to_lowercase().as_str() {
        "stock" | "level" => Kind::Stock,
        "flow" | "rate" => Kind::Flow,
        "aux" | "auxiliary" | "variable" => Kind::Aux,
        "constant" | "parameter" => Kind::Constant,
        "lookup" | "graphical function" => Kind::Lookup,
        other => return Err(format!("unknown kind “{other}” (stock, flow, aux, constant, lookup)")),
    })
}

fn layer_id(m: &Model, name: &str) -> Result<u64, String> {
    m.layer_by_name(name).map(|l| l.id).ok_or_else(|| format!("there is no layer called “{name}”"))
}

fn find(m: &Model, name: &str) -> Result<u64, String> {
    m.by_name(&expr::normalize(name)).map(|v| v.id).ok_or_else(|| format!("there is no variable called “{name}”"))
}

fn var_mut<'a>(m: &'a mut Model, name: &str) -> Result<&'a mut Var, String> {
    let id = find(m, name)?;
    Ok(m.var_mut(id).unwrap())
}

/// Where a new variable goes: a column per kind, a row per variable already of that kind.
fn place(m: &Model, kind: Kind) -> [f32; 2] {
    let col = match kind {
        Kind::Constant | Kind::Lookup => 0,
        Kind::Aux => 1,
        Kind::Flow => 2,
        Kind::Stock => 3,
    };
    let row = m.vars.iter().filter(|v| v.kind == kind).count();
    [140.0 + col as f32 * 200.0, 100.0 + row as f32 * 90.0]
}

/// Make every variable an equation uses show as an arrow into it.
pub fn sync_links_for_tests(m: &mut Model) {
    sync_links(m)
}

pub(crate) fn sync_links(m: &mut Model) {
    let mut pairs = Vec::new();
    for v in &m.vars {
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut refs = Vec::new();
        expr::references(&e, &mut refs);
        for r in refs {
            if let Some(src) = m.by_name(&r) {
                if src.id != v.id && !m.links.iter().any(|l| l.from == src.id && l.to == v.id) {
                    pairs.push((src.id, v.id));
                }
            }
        }
    }
    for (a, b) in pairs {
        m.add_link(a, b);
    }
}

fn parse_method(s: &str) -> Result<Method, String> {
    let n = s.trim().to_lowercase();
    Ok(if n.contains("rk2") || n.contains("heun") { Method::RungeKutta2 } else if n.contains("rk3") { Method::RungeKutta3 } else if n.contains("rk4") || n.contains("runge") { Method::RungeKutta4 } else if n.contains("euler") { Method::Euler } else { return Err(format!("unknown method “{s}” (euler, rk2, rk3, rk4)")) })
}

fn parse_bool(key: &str, s: &str) -> Result<bool, String> {
    match s.trim().to_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        o => Err(format!("{key} is true or false, not “{o}”")),
    }
}

fn num(key: &str, s: &str) -> Result<f64, String> {
    s.trim().parse().map_err(|_| format!("{key} must be a number, not “{s}”"))
}

/// Apply one operation to a model (no validation: that is the batch's job).
pub fn apply(m: &mut Model, op: &Op) -> Result<(), String> {
    match op {
        Op::AddVar { name, kind, equation, units, doc, from, to, non_negative, layer } => {
            if name.trim().is_empty() {
                return Err("a variable needs a name".into());
            }
            if m.by_name(&expr::normalize(name)).is_some() {
                return Err(format!("“{name}” already exists"));
            }
            let kind = kind_of(kind)?;
            let id = m.new_id();
            let mut v = Var::new(id, name.trim(), kind, place(m, kind));
            v.equation = equation.clone();
            v.units = units.clone();
            v.comment = doc.clone();
            v.non_negative = *non_negative;
            if let Some(l) = layer {
                v.layer = Some(layer_id(m, l)?);
            }
            if kind == Kind::Flow {
                v.from = from.as_deref().map(|n| find(m, n)).transpose()?;
                v.to = to.as_deref().map(|n| find(m, n)).transpose()?;
            } else if from.is_some() || to.is_some() {
                return Err("only a flow has a from and a to".into());
            }
            m.vars.push(v);
            sync_links(m);
        }
        Op::RemoveVar { name } => {
            let id = find(m, name)?;
            m.remove_var(id);
        }
        Op::RenameVar { name, to } => {
            let id = find(m, name)?;
            if m.by_name(&expr::normalize(to)).is_some_and(|v| v.id != id) {
                return Err(format!("“{to}” already exists"));
            }
            let old = m.var(id).unwrap().name.clone();
            m.var_mut(id).unwrap().name = to.trim().to_string();
            for v in m.vars.iter_mut() {
                v.equation = expr::rename_in(&v.equation, &expr::normalize(&old), to.trim());
                for o in v.by_item.iter_mut() {
                    o.equation = expr::rename_in(&o.equation, &expr::normalize(&old), to.trim());
                }
            }
            for c in m.checks.iter_mut().filter(|c| expr::normalize(&c.var) == expr::normalize(&old)) {
                c.var = to.trim().to_string();
            }
        }
        Op::SetEquation { name, equation } => {
            var_mut(m, name)?.equation = equation.clone();
            sync_links(m);
        }
        Op::SetUnits { name, units } => var_mut(m, name)?.units = units.clone(),
        Op::SetDoc { name, doc } => var_mut(m, name)?.comment = doc.clone(),
        Op::SetNonNegative { name, on } => var_mut(m, name)?.non_negative = *on,
        Op::SetFlow { name, from, to } => {
            let f = from.as_deref().map(|n| find(m, n)).transpose()?;
            let t = to.as_deref().map(|n| find(m, n)).transpose()?;
            let v = var_mut(m, name)?;
            if v.kind != Kind::Flow {
                return Err(format!("“{name}” is not a flow"));
            }
            v.from = f;
            v.to = t;
        }
        Op::SetTable { name, points } => {
            let v = var_mut(m, name)?;
            if v.kind != Kind::Lookup {
                return Err(format!("“{name}” is not a lookup"));
            }
            if points.len() < 2 || points.windows(2).any(|w| w[1][0] <= w[0][0]) {
                return Err("a curve needs at least two points with increasing x".into());
            }
            v.table = points.clone();
        }
        Op::Connect { from, to, polarity } => {
            let (a, b) = (find(m, from)?, find(m, to)?);
            m.add_link(a, b);
            if let Some(l) = m.links.iter_mut().find(|l| l.from == a && l.to == b) {
                l.polarity = polarity.clone();
            }
        }
        Op::Disconnect { from, to } => {
            let (a, b) = (find(m, from)?, find(m, to)?);
            m.links.retain(|l| !(l.from == a && l.to == b));
        }
        Op::SetSetting { key, value } => {
            let s = &mut m.settings;
            match key.trim().to_lowercase().replace(' ', "_").as_str() {
                "initial_time" | "start" => s.initial_time = num(key, value)?,
                "final_time" | "stop" => s.final_time = num(key, value)?,
                "time_step" | "dt" => {
                    let dt = num(key, value)?;
                    if dt <= 0.0 {
                        return Err("the time step must be above zero".into());
                    }
                    s.time_step = dt;
                }
                "time_units" => s.time_units = value.clone(),
                "method" => s.method = parse_method(value)?,
                "title" => m.title = value.clone(),
                "units_scale_values" => s.units_scale_values = parse_bool(key, value)?,
                "figure_model" => s.figure_model = parse_bool(key, value)?,
                "teaching_draft" => s.teaching_draft = parse_bool(key, value)?,
                "starts_in_equilibrium" => s.starts_in_equilibrium = parse_bool(key, value)?,
                // "MB=thousand barrels": the meaning chosen for an ambiguous unit symbol (empty meaning removes the choice)
                "unit_choice" | "unit_choices" => match value.split_once('=') {
                    Some((sym, meaning)) if meaning.trim().is_empty() => {
                        s.unit_choices.remove(sym.trim());
                    }
                    Some((sym, meaning)) => {
                        s.unit_choices.insert(sym.trim().to_string(), meaning.trim().to_string());
                    }
                    None => return Err("unit_choice is written SYMBOL=meaning, for example MB=thousand barrels".into()),
                },
                "start_date" => s.start = if value.trim().is_empty() { None } else { Some(value.trim().to_string()) },
                "time_convention" => {
                    s.time_convention = match value.trim().to_lowercase().as_str() {
                        "abstract" => crate::model::TimeConvention::Abstract,
                        "calendar" => crate::model::TimeConvention::Calendar,
                        o => return Err(format!("time_convention is abstract or calendar, not “{o}”")),
                    }
                }
                "year_days_abstract" => s.year_days_abstract = num(key, value)?,
                "weekend" => match value.trim() {
                    "sat-sun" | "fri-sat" => s.weekend = value.trim().to_string(),
                    o => return Err(format!("weekend is sat-sun or fri-sat, not “{o}”")),
                },
                "holidays" => s.holidays = value.split([',', ';']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
                "seed" => {
                    s.seed = num(key, value)? as u64;
                    s.seed_recorded = true;
                }
                other => return Err(format!("unknown setting “{other}”")),
            }
        }
        Op::AddSet { name, items, parent, parent_of } => {
            if m.sets.iter().any(|s| expr::normalize(&s.name) == expr::normalize(name)) {
                return Err(format!("the set “{name}” already exists"));
            }
            if items.is_empty() {
                return Err("a set needs items".into());
            }
            if let Some(p) = parent {
                let ps = m.sets.iter().find(|s| &s.name == p).ok_or_else(|| format!("there is no set called “{p}”"))?;
                if parent_of.len() != items.len() || parent_of.iter().any(|x| !ps.items.contains(x)) {
                    return Err(format!("parent_of must name one item of “{p}” for each item"));
                }
            }
            let id = m.new_id();
            m.sets.push(ItemSet { id, name: name.clone(), items: items.clone(), parent: parent.clone(), parent_of: parent_of.clone(), note: String::new() });
        }
        Op::SetPer { var, set } => {
            if let Some(s) = set {
                if !m.sets.iter().any(|x| &x.name == s) {
                    return Err(format!("there is no set called “{s}”"));
                }
            }
            let v = var_mut(m, var)?;
            v.set = set.clone();
            if set.is_none() {
                v.by_item.clear();
            }
        }
        Op::SetItemValue { var, item, equation } => {
            let v = var_mut(m, var)?;
            let Some(set) = v.set.clone() else { return Err(format!("“{var}” is not per a set")) };
            let _ = set;
            match v.by_item.iter_mut().find(|o| expr::normalize(&o.item) == expr::normalize(item)) {
                Some(o) => o.equation = equation.clone(),
                None => v.by_item.push(crate::model::ItemValue { item: item.clone(), equation: equation.clone(), table: Vec::new() }),
            }
        }
        Op::SetSlider { var, min, max } => {
            if !(min < max) {
                return Err("the slider's minimum must be below its maximum".into());
            }
            let v = var_mut(m, var)?;
            if v.kind != Kind::Constant {
                return Err(format!("only a constant has a slider; “{var}” is a {}", v.kind.label()));
            }
            v.slider = Some([*min, *max]);
        }
        Op::SetLever { var, on } => {
            let v = var_mut(m, var)?;
            if *on && v.kind != Kind::Constant {
                return Err(format!("only a constant can be a key lever; “{var}” is a {}", v.kind.label()));
            }
            v.lever = *on;
        }
        Op::AddCheck { name, var, min, max, when, kind, tolerance, relative, reference, stocks, inflows, outflows, at, source, role } => {
            use crate::checks::CheckKind;
            let kind = match kind.as_deref().map(|k| k.to_lowercase()).as_deref() {
                None | Some("bounds") => CheckKind::Bounds,
                Some("benchmark") => {
                    if source.trim().is_empty() {
                        return Err("a benchmark needs a source: where does the number come from?".into());
                    }
                    CheckKind::Bounds
                }
                Some("nonneg") => CheckKind::NonNeg,
                Some("reference") => CheckKind::Reference,
                Some("balance") => CheckKind::Balance,
                Some("conservation") => CheckKind::Conservation,
                Some(k) => return Err(format!("kind must be bounds, nonneg, reference, balance or conservation, not “{k}”")),
            };
            let when = match when.as_deref().map(|w| w.to_lowercase()).as_deref() {
                None | Some("always") => CheckWhen::Always,
                Some("end") => CheckWhen::End,
                Some(w) => return Err(format!("when must be “always” or “end”, not “{w}”")),
            };
            if !["", "validity", "promise"].contains(&role.trim().to_lowercase().as_str()) {
                return Err(format!("role must be empty, “validity” or “promise”, not “{role}”"));
            }
            match kind {
                CheckKind::Bounds => {
                    if min.is_none() && max.is_none() {
                        return Err("a bounds check needs a min, a max or both".into());
                    }
                    find(m, var)?;
                }
                CheckKind::NonNeg => {
                    find(m, var)?;
                }
                CheckKind::Reference => {
                    find(m, var)?;
                    if reference.is_empty() {
                        return Err("a reference check needs at least one (time, value) point; one point means a constant".into());
                    }
                    if tolerance.is_some_and(|t| t < 0.0) {
                        return Err("the tolerance cannot be negative".into());
                    }
                }
                CheckKind::Conservation => {
                    for n in stocks {
                        find(m, n)?;
                    }
                }
                CheckKind::Balance => {
                    if stocks.is_empty() {
                        return Err("a balance check needs at least one stock".into());
                    }
                    for n in stocks.iter().chain(inflows).chain(outflows) {
                        find(m, n)?;
                    }
                }
            }
            m.checks.push(Check { name: name.clone(), var: var.clone(), min: *min, max: *max, when, kind, tolerance: *tolerance, relative: *relative, reference: reference.clone(), stocks: stocks.clone(), inflows: inflows.clone(), outflows: outflows.clone(), at: *at, source: source.clone(), role: role.trim().to_lowercase() });
        }
        Op::RemoveCheck { name } => {
            let n = m.checks.len();
            m.checks.retain(|c| expr::normalize(&c.name) != expr::normalize(name));
            if m.checks.len() == n {
                return Err(format!("there is no check called “{name}”"));
            }
        }
        Op::AddFunction { name, params, body, doc } => {
            if m.functions.iter().any(|f| expr::normalize(&f.name) == expr::normalize(name)) {
                return Err(format!("the function “{name}” already exists"));
            }
            m.functions.push(crate::userfn::UserFn { name: name.clone(), params: params.clone(), body: body.clone(), description: doc.clone() });
        }
        Op::DefineModule { name, description, vars, ports } => {
            if m.defs.iter().any(|d| expr::normalize(&d.name) == expr::normalize(name)) {
                return Err(format!("the module “{name}” already exists"));
            }
            m.defs.push(crate::modules::ModuleDef { name: name.clone(), description: description.clone(), vars: vars.clone(), ports: ports.clone() });
        }
        Op::AddLayer { name, order, question, horizon, structural_limit, status, colour, description, spheres } => {
            if name.trim().is_empty() {
                return Err("a layer needs a name".into());
            }
            if m.layer_by_name(name).is_some() {
                return Err(format!("the layer “{name}” already exists"));
            }
            let id = m.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
            let order = order.unwrap_or_else(|| m.layers.iter().map(|l| l.order + 1).max().unwrap_or(0));
            let status = status.as_deref().map(crate::layers::LayerStatus::parse).transpose()?.unwrap_or_default();
            m.layers.push(crate::layers::Layer { id, name: name.trim().into(), order, question: question.clone(), horizon: *horizon, structural_limit: *structural_limit, status, colour: colour.clone(), description: description.clone(), owner: String::new(), spheres: spheres.clone() });
        }
        Op::SetLayer { name, key, value } => {
            let id = layer_id(m, name)?;
            if key.as_str() == "name" && m.layers.iter().any(|l| l.id != id && expr::normalize(&l.name) == expr::normalize(value)) {
                return Err(format!("the layer “{value}” already exists"));
            }
            let num = |v: &str| -> Result<Option<f64>, String> { if v.trim().is_empty() { Ok(None) } else { v.trim().parse::<f64>().map(Some).map_err(|_| format!("“{v}” is not a number")) } };
            let l = m.layers.iter_mut().find(|l| l.id == id).unwrap();
            match key.as_str() {
                "name" => l.name = value.trim().into(),
                "order" => l.order = value.trim().parse().map_err(|_| format!("“{value}” is not a whole number"))?,
                "question" => l.question = value.clone(),
                "horizon" => l.horizon = num(value)?,
                "structural_limit" => l.structural_limit = num(value)?,
                "status" => l.status = crate::layers::LayerStatus::parse(value)?,
                "colour" => l.colour = value.clone(),
                "description" => l.description = value.clone(),
                "owner" => l.owner = value.clone(),
                "spheres" => l.spheres = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                other => return Err(format!("a layer has no field “{other}”")),
            }
        }
        Op::SetPurpose { problem, kind, out_of_scope } => {
            let p = m.purpose.get_or_insert_with(Default::default);
            p.problem = problem.clone();
            p.kind = kind.clone();
            p.out_of_scope = out_of_scope.clone();
        }
        Op::AddObjective { title, kind, test, var } => {
            if title.trim().is_empty() {
                return Err("an objective needs a title".into());
            }
            if !var.trim().is_empty() {
                find(m, var)?;
            }
            let p = m.purpose.get_or_insert_with(Default::default);
            if p.objectives.iter().any(|o| expr::normalize(&o.title) == expr::normalize(title)) {
                return Err(format!("there is already an objective called “{title}”"));
            }
            p.objectives.push(crate::model::Objective { title: title.trim().into(), kind: kind.clone(), test: test.clone(), var: var.trim().into() });
        }
        Op::RemoveObjective { title } => {
            let p = m.purpose.as_mut().ok_or("the model has no purpose yet")?;
            let n = p.objectives.len();
            p.objectives.retain(|o| expr::normalize(&o.title) != expr::normalize(title));
            if p.objectives.len() == n {
                return Err(format!("there is no objective called “{title}”"));
            }
        }
        Op::AddGroup { name, vars } => {
            if name.trim().is_empty() || vars.is_empty() {
                return Err("a group needs a name and variables".into());
            }
            if m.groups.iter().any(|g| expr::normalize(&g.name) == expr::normalize(name)) {
                return Err(format!("the group “{name}” already exists"));
            }
            let mut members = Vec::new();
            let mut view = 0;
            for n in vars {
                let vid = find(m, n)?;
                if let Some(v) = m.vars.iter().find(|v| v.id == vid) {
                    view = v.view;
                }
                members.push(vid);
            }
            // a variable is in one group only
            for g in m.groups.iter_mut() {
                g.members.retain(|x| !members.contains(x));
            }
            m.groups.retain(|g| !g.members.is_empty());
            let id = m.new_id();
            m.groups.push(crate::model::Group { id, name: name.trim().into(), members, view, color: None });
        }
        Op::SetCldPolarity { from, to, polarity, sheet } => {
            crate::figure::set_polarity(m, sheet.as_deref(), from, to, polarity)?;
        }
        Op::SetCalibration { calibration } => {
            m.calibration = Some(calibration.clone());
        }
        Op::AddToGroup { name, vars } => {
            let key = expr::normalize(name);
            if let Some(g) = m.groups.iter().find(|g| expr::normalize(&g.name) == key) {
                let mut ids: Vec<u64> = g.members.clone();
                let gname = g.name.clone();
                for n in vars {
                    let vid = find(m, n)?;
                    if !ids.contains(&vid) {
                        ids.push(vid);
                    }
                }
                // re-make the group with all of its members (a variable is in one group only)
                m.groups.retain(|g| expr::normalize(&g.name) != key);
                let names: Vec<String> = ids.iter().filter_map(|id| m.var(*id).map(|v| v.name.clone())).collect();
                return apply(m, &Op::AddGroup { name: gname, vars: names });
            }
            return apply(m, &Op::AddGroup { name: name.clone(), vars: vars.clone() });
        }
        Op::RemoveLayer { name } => {
            let id = layer_id(m, name)?;
            m.layers.retain(|l| l.id != id);
            m.interfaces.retain(|i| i.from != id && i.to != id);
            m.stubs.retain(|s| s.layer != id);
            for v in m.vars.iter_mut().filter(|v| v.layer == Some(id)) {
                v.layer = None;
            }
            for s in m.sheets.iter_mut().filter(|s| s.layer == Some(id)) {
                s.layer = None;
            }
            for x in m.modules.iter_mut().filter(|x| x.layer == Some(id)) {
                x.layer = None;
            }
        }
        Op::AssignLayer { layer, vars, sheet, module } => {
            let id = layer_id(m, layer)?;
            if vars.is_empty() && sheet.is_none() && module.is_none() {
                return Err("name variables, a sheet or a module to assign".into());
            }
            for n in vars {
                let vid = find(m, n)?;
                m.vars.iter_mut().find(|v| v.id == vid).unwrap().layer = Some(id);
            }
            if let Some(n) = sheet {
                let s = m.sheets.iter_mut().find(|s| expr::normalize(&s.name) == expr::normalize(n)).ok_or_else(|| format!("there is no sheet called “{n}”"))?;
                s.layer = Some(id);
            }
            if let Some(n) = module {
                let x = m.modules.iter_mut().find(|x| expr::normalize(&x.name) == expr::normalize(n)).ok_or_else(|| format!("there is no submodel called “{n}”"))?;
                x.layer = Some(id);
            }
        }
        Op::FreezeInterfaces {} => {
            if m.layers.is_empty() {
                return Err("there are no layers".into());
            }
            m.interfaces = crate::layers::frozen(m);
        }
        Op::AddStub { layer, name, series } => {
            let id = layer_id(m, layer)?;
            if m.stubs.iter().any(|s| expr::normalize(&s.name) == expr::normalize(name)) {
                return Err(format!("the stub “{name}” already exists"));
            }
            m.stubs.push(crate::layers::Stub { layer: id, name: name.clone(), series: series.clone() });
        }
        Op::RemoveStub { name } => {
            let n = m.stubs.len();
            m.stubs.retain(|s| expr::normalize(&s.name) != expr::normalize(name));
            if m.stubs.len() == n {
                return Err(format!("there is no stub called “{name}”"));
            }
        }
        Op::RemoveModule { name } => {
            if m.instances.iter().any(|i| expr::normalize(&i.def) == expr::normalize(name)) {
                return Err(format!("the module “{name}” still has instances; remove them first"));
            }
            let n = m.defs.len();
            m.defs.retain(|d| expr::normalize(&d.name) != expr::normalize(name));
            if m.defs.len() == n {
                return Err(format!("there is no module called “{name}”"));
            }
        }
        Op::Instantiate { def, name, params, inputs } => {
            if m.instances.iter().any(|i| expr::normalize(&i.name) == expr::normalize(name)) {
                return Err(format!("the instance “{name}” already exists"));
            }
            if !m.defs.iter().any(|d| expr::normalize(&d.name) == expr::normalize(def)) {
                return Err(format!("there is no module called “{def}”"));
            }
            m.instances.push(crate::modules::Instance { name: name.clone(), def: def.clone(), params: params.clone(), inputs: inputs.clone() });
        }
        Op::SetInstanceParam { instance, param, value } => {
            let i = m.instances.iter_mut().find(|i| expr::normalize(&i.name) == expr::normalize(instance)).ok_or_else(|| format!("there is no instance called “{instance}”"))?;
            i.params.insert(param.clone(), value.clone());
        }
        Op::SetInstanceInput { instance, port, formula } => {
            let i = m.instances.iter_mut().find(|i| expr::normalize(&i.name) == expr::normalize(instance)).ok_or_else(|| format!("there is no instance called “{instance}”"))?;
            i.inputs.insert(port.clone(), formula.clone());
        }
        Op::RemoveInstance { name } => {
            let n = m.instances.len();
            m.instances.retain(|i| expr::normalize(&i.name) != expr::normalize(name));
            if m.instances.len() == n {
                return Err(format!("there is no instance called “{name}”"));
            }
        }
        Op::SetManifest { manifest } => {
            m.manifest = manifest.clone();
        }
        Op::RemoveFunction { name } => {
            let n = m.functions.len();
            m.functions.retain(|f| expr::normalize(&f.name) != expr::normalize(name));
            if m.functions.len() == n {
                return Err(format!("there is no function called “{name}”"));
            }
        }
    }
    Ok(())
}

/// A model with its history: the log of committed batches and the states to go back to.
#[derive(Clone, Debug, Default)]
pub struct Session {
    pub model: Model,
    pub log: Vec<LogEntry>,
    undo: Vec<Model>,
    redo: Vec<Model>,
    /// Refuse any batch whose result does not pass validation (errors, not warnings).
    pub gate: bool,
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Session {
    pub fn new(model: Model) -> Self {
        Session { model, gate: true, ..Default::default() }
    }

    /// Apply a batch atomically. On success the model is replaced and the batch logged.
    pub fn apply(&mut self, actor: &str, ops: &[Op]) -> Result<&LogEntry, Rejected> {
        self.apply_with_reason(actor, None, ops)
    }

    /// As `apply`, recording why the batch was made. A rejected batch is logged too, with its problems.
    pub fn apply_with_reason(&mut self, actor: &str, reason: Option<&str>, ops: &[Op]) -> Result<&LogEntry, Rejected> {
        let seq = self.log.last().map(|e| e.seq + 1).unwrap_or(1);
        let committed_so_far = self.log.iter().filter(|e| e.result == "committed").count() as u64;
        let attempt = (|| -> Result<(Model, usize), Rejected> {
            let mut next = self.model.clone();
            for (i, op) in ops.iter().enumerate() {
                apply(&mut next, op).map_err(|e| Rejected::at(i, e))?;
            }
            let report = validate::validate(&next);
            if self.gate && !report.ok() {
                let issues: Vec<Issue> = report.issues.into_iter().filter(|i| i.severity == Severity::Error).collect();
                return Err(Rejected { op_index: None, reason: format!("the result does not pass validation ({} error(s)); nothing was changed", issues.len()), issues });
            }
            let warnings = report.issues.iter().filter(|i| i.severity == Severity::Warning).count();
            Ok((next, warnings))
        })();
        match attempt {
            Ok((next, warnings)) => {
                self.undo.push(std::mem::replace(&mut self.model, next));
                self.redo.clear();
                self.log.push(LogEntry { seq, revision: committed_so_far + 1, result: committed(), reason: reason.map(|r| r.to_string()), problems: Vec::new(), actor: actor.to_string(), at: now(), ops: ops.to_vec(), warnings });
                Ok(self.log.last().unwrap())
            }
            Err(r) => {
                let mut problems = vec![match r.op_index {
                    Some(i) => format!("operation {} refused: {}", i + 1, r.reason),
                    None => r.reason.clone(),
                }];
                problems.extend(r.issues.iter().map(|i| format!("{}{}", i.var.as_ref().map(|v| format!("{v}: ")).unwrap_or_default(), i.message)));
                self.log.push(LogEntry { seq, revision: committed_so_far, result: "rejected".into(), reason: reason.map(|r| r.to_string()), problems, actor: actor.to_string(), at: now(), ops: ops.to_vec(), warnings: 0 });
                Err(r)
            }
        }
    }

    /// Take back the last committed batch (it stays in the log, marked by an undo entry's absence
    /// from the model: the log is the history of what was done, undo only moves the model).
    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.model, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.model, next));
                true
            }
            None => false,
        }
    }
}

// ---------------------------------------------------------------- queries (read-only)

fn var_json(m: &Model, v: &Var) -> Value {
    json!({
        "name": v.name, "kind": v.kind.label(), "equation": v.equation, "units": v.units, "doc": v.comment,
        "from": v.from.and_then(|i| m.var(i)).map(|s| s.name.clone()),
        "to": v.to.and_then(|i| m.var(i)).map(|s| s.name.clone()),
        "per": v.set, "lever": v.lever, "non_negative": v.non_negative,
        "table": if v.table.is_empty() { Value::Null } else { json!(v.table) },
    })
}

/// A short description of the whole model.
/// What the model has: counts by kind, and `order`, the number of state variables (stocks plus the hidden states inside
/// SMOOTH, DELAY and the like; null when the model does not compile).
pub fn info(m: &Model) -> Value {
    let count = |k: Kind| m.vars.iter().filter(|v| v.kind == k).count();
    json!({
        "title": m.title,
        "stocks": count(Kind::Stock), "order": crate::sim::order(m), "flows": count(Kind::Flow), "auxiliaries": count(Kind::Aux),
        "constants": count(Kind::Constant), "lookups": count(Kind::Lookup),
        "sets": m.sets.iter().map(|s| json!({"name": s.name, "items": s.items.len(), "parent": s.parent})).collect::<Vec<_>>(),
        "checks": m.checks.len(), "functions": m.functions.len(), "manifest": !m.manifest.is_empty(), "modules": m.defs.iter().map(|d| d.name.clone()).collect::<Vec<_>>(), "instances": m.instances.iter().map(|i| json!({"name": i.name, "module": i.def})).collect::<Vec<_>>(),
        "settings": {"initial_time": m.settings.initial_time, "final_time": m.settings.final_time, "time_step": m.settings.time_step, "time_units": m.settings.time_units, "method": format!("{:?}", m.settings.method)},
    })
}

pub fn list(m: &Model, kind: Option<&str>) -> Result<Value, String> {
    let k = kind.map(kind_of).transpose()?;
    Ok(json!(m.vars.iter().filter(|v| k.map_or(true, |k| v.kind == k)).map(|v| var_json(m, v)).collect::<Vec<_>>()))
}

pub fn get(m: &Model, name: &str) -> Result<Value, String> {
    let id = find(m, name)?;
    Ok(var_json(m, m.var(id).unwrap()))
}

fn refs_of(v: &Var) -> Vec<String> {
    let mut refs = Vec::new();
    for eq in std::iter::once(&v.equation).chain(v.by_item.iter().map(|o| &o.equation)) {
        if let Ok(e) = expr::parse(eq) {
            expr::references(&e, &mut refs);
        }
    }
    refs.sort();
    refs.dedup();
    refs
}

/// What `name` depends on (its causes) and what depends on it (its uses), by name.
pub fn dependencies(m: &Model, name: &str) -> Result<Value, String> {
    let id = find(m, name)?;
    let v = m.var(id).unwrap();
    let mut causes: Vec<String> = refs_of(v).iter().filter_map(|r| m.by_name(r)).map(|x| x.name.clone()).collect();
    let mut uses: Vec<String> = m.vars.iter().filter(|x| x.id != id && refs_of(x).iter().any(|r| *r == expr::normalize(&v.name))).map(|x| x.name.clone()).collect();
    match v.kind {
        // a stock is moved by the flows that drain and fill it
        Kind::Stock => causes.extend(m.vars.iter().filter(|f| f.kind == Kind::Flow && (f.from == Some(id) || f.to == Some(id))).map(|f| f.name.clone())),
        // a flow moves the stocks at its ends (and a draining flow also depends on its source)
        Kind::Flow => {
            for sid in [v.from, v.to].into_iter().flatten() {
                if let Some(st) = m.var(sid) {
                    uses.push(st.name.clone());
                }
            }
            if let Some(src) = v.from.and_then(|i| m.var(i)) {
                causes.push(src.name.clone());
            }
        }
        _ => {}
    }
    uses.sort();
    uses.dedup();
    causes.sort();
    causes.dedup();
    Ok(json!({"name": v.name, "causes": causes, "uses": uses}))
}

/// Feedback loops of the model: each as the list of variable names around the loop and its polarity.
/// The polarity comes from the partial derivatives along the base run (see `polarity.rs`): "reinforcing", "balancing",
/// "switches" with the times at which it does, or "inactive" when a link has zero gain all along (`reinforcing` is then null).
pub fn loops(m: &Model) -> Value {
    let mut copy = m.clone();
    let d = crate::cld::from_model(&mut copy);
    let found = crate::cld::find_loops(&d);
    let pol = crate::polarity::loop_polarities(m, &d, &found);
    json!(found.iter().zip(&pol).map(|(l, p)| json!({
        "reinforcing": match p.label { "reinforcing" => json!(true), "balancing" => json!(false), _ => Value::Null },
        "polarity": p.label,
        "switches_at": p.switches_at,
        "variables": l.nodes.iter().filter_map(|n| d.nodes.iter().find(|x| x.id == *n)).map(|n| n.text.clone()).collect::<Vec<_>>()
    })).collect::<Vec<_>>())
}

/// Stocks that no flow touches, and variables nothing uses: structure worth a second look.
pub fn structure_notes(m: &Model) -> Value {
    let lone: Vec<&str> = m.vars.iter().filter(|v| v.kind == Kind::Stock && !m.vars.iter().any(|f| f.kind == Kind::Flow && (f.from == Some(v.id) || f.to == Some(v.id)))).map(|v| v.name.as_str()).collect();
    let unused: Vec<&str> = m.vars.iter().filter(|v| !matches!(v.kind, Kind::Stock) && !m.vars.iter().any(|x| x.id != v.id && refs_of(x).iter().any(|r| *r == expr::normalize(&v.name)))).map(|v| v.name.as_str()).collect();
    json!({"stocks_without_flows": lone, "variables_nothing_uses": unused})
}

#[allow(dead_code)]
fn _link_is_used(_: &Link) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(j: &str) -> Op {
        serde_json::from_str(j).unwrap()
    }

    fn stock_model() -> Vec<Op> {
        vec![
            op(r#"{"op":"set_setting","key":"final_time","value":"20"}"#),
            op(r#"{"op":"set_setting","key":"time_units","value":"Day"}"#),
            op(r#"{"op":"add_var","name":"Tank","kind":"stock","equation":"100","units":"MB"}"#),
            op(r#"{"op":"add_var","name":"Burn rate","kind":"constant","equation":"4","units":"MB/Day"}"#),
            op(r#"{"op":"add_var","name":"Burn","kind":"flow","equation":"Burn rate","units":"MB/Day","from":"Tank"}"#),
        ]
    }

    #[test]
    fn a_batch_builds_a_model_and_is_logged() {
        let mut s = Session::new(Model::default());
        let e = s.apply("agent:test", &stock_model()).unwrap();
        assert_eq!((e.seq, e.actor.as_str(), e.ops.len()), (1, "agent:test", 5));
        let v = validate::validate(&s.model);
        assert!(v.ok() && v.ran, "{}", v.summary());
        // arrows were made from the equation
        assert!(s.model.links.iter().any(|l| l.from == s.model.by_name("burn rate").unwrap().id && l.to == s.model.by_name("burn").unwrap().id));
    }

    #[test]
    fn a_failing_batch_changes_nothing() {
        let mut s = Session::new(Model::default());
        s.apply("t", &stock_model()).unwrap();
        let before = s.model.vars.len();
        let bad = vec![op(r#"{"op":"add_var","name":"Extra","kind":"aux","equation":"1"}"#), op(r#"{"op":"set_equation","name":"Nope","equation":"1"}"#)];
        let r = s.apply("t", &bad).unwrap_err();
        assert_eq!(r.op_index, Some(1));
        assert_eq!(s.model.vars.len(), before);
        // the refusal is in the log, but the model has one committed revision only
        assert_eq!(s.log.len(), 2);
        assert_eq!(s.log.iter().filter(|e| e.result == "committed").count(), 1);
    }

    #[test]
    fn a_rejected_batch_is_logged_with_its_reason_and_problems() {
        let mut s = Session::new(Model::default());
        s.apply_with_reason("agent:x", Some("add the supply line"), &stock_model()).unwrap();
        let bad = [op(r#"{"op":"set_equation","name":"Burn","equation":"Burn rate *"}"#)];
        assert!(s.apply_with_reason("agent:x", Some("try a different rule"), &bad).is_err());
        assert_eq!(s.log.len(), 2);
        let (ok, no) = (&s.log[0], &s.log[1]);
        assert_eq!((ok.result.as_str(), ok.revision, ok.reason.as_deref()), ("committed", 1, Some("add the supply line")));
        assert_eq!((no.result.as_str(), no.revision, no.seq), ("rejected", 1, 2));
        assert!(no.problems.iter().any(|p| p.contains("validation")) && no.problems.len() >= 2, "{:?}", no.problems);
        // an old log line without the new fields still reads
        let old: LogEntry = serde_json::from_str(r#"{"seq":1,"actor":"a","at":0,"ops":[],"warnings":0}"#).unwrap();
        assert_eq!((old.result.as_str(), old.revision), ("committed", 0));
    }

    #[test]
    fn the_gate_refuses_a_model_that_does_not_validate() {
        let mut s = Session::new(Model::default());
        s.apply("t", &stock_model()).unwrap();
        let r = s.apply("agent", &[op(r#"{"op":"set_equation","name":"Burn","equation":"Burn rate *"}"#)]).unwrap_err();
        assert!(r.reason.contains("validation") && r.issues.iter().any(|i| i.code == "equation"), "{r:?}");
        assert!(s.model.by_name("burn").unwrap().equation == "Burn rate");
        s.gate = false;
        assert!(s.apply("script", &[op(r#"{"op":"set_equation","name":"Burn","equation":"Burn rate *"}"#)]).is_ok());
    }

    #[test]
    fn limits_can_be_stated_and_break_the_gate() {
        let mut s = Session::new(Model::default());
        s.apply("t", &stock_model()).unwrap();
        s.apply("t", &[op(r#"{"op":"add_check","name":"Never empty","var":"Tank","min":0}"#)]).unwrap();
        // burning 12 a day empties 100 in 9 days
        let r = s.apply("t", &[op(r#"{"op":"set_equation","name":"Burn rate","equation":"12"}"#)]).unwrap_err();
        assert!(r.issues.iter().any(|i| i.code == "check"), "{r:?}");
    }

    #[test]
    fn rename_follows_equations_and_checks_and_undo_redo_work() {
        let mut s = Session::new(Model::default());
        s.apply("t", &stock_model()).unwrap();
        s.apply("t", &[op(r#"{"op":"add_check","name":"c","var":"Burn rate","min":0}"#), op(r#"{"op":"rename_var","name":"Burn rate","to":"Daily burn"}"#)]).unwrap();
        assert_eq!(s.model.by_name("burn").unwrap().equation, "Daily burn");
        assert_eq!(s.model.checks[0].var, "Daily burn");
        assert!(s.undo());
        assert!(s.model.by_name("burn rate").is_some());
        assert!(s.redo());
        assert!(s.model.by_name("daily burn").is_some());
    }

    #[test]
    fn modules_are_defined_once_and_used_many_times() {
        let mut s = Session::new(Model::default());
        let batch: Vec<Op> = serde_json::from_str(r#"[
          {"op":"set_setting","key":"final_time","value":"10"},
          {"op":"set_setting","key":"time_units","value":"Day"},
          {"op":"define_module","name":"Tank","description":"A tank that drains","vars":[
             {"name":"Level","kind":"Stock","equation":"Start","units":"MB"},
             {"name":"Start","kind":"Constant","equation":"100","units":"MB"},
             {"name":"Draw","kind":"Flow","equation":"Rate","units":"MB/Day","from":"Level"},
             {"name":"Rate","kind":"Aux","equation":"1","units":"MB/Day"}],
           "ports":[{"name":"Rate in","direction":"In","kind":"Information","units":"MB/Day","var":"Rate"},
                    {"name":"Level out","direction":"Out","kind":"Material","units":"MB","var":"Level"}]},
          {"op":"instantiate","def":"Tank","name":"A","params":{"Start":"50"},"inputs":{"Rate in":"2"}},
          {"op":"instantiate","def":"Tank","name":"B","inputs":{"Rate in":"3"}}]"#).unwrap();
        s.apply("t", &batch).unwrap();
        let run = crate::runner::run_with(&s.model, &[]).unwrap();
        assert_eq!(run.get("A.Level").unwrap().last().copied(), Some(30.0));
        assert_eq!(run.get("B.Level").unwrap().last().copied(), Some(70.0));
        // a typo in a port name is refused, and so is removing a module that is in use
        let bad: Vec<Op> = serde_json::from_str(r#"[{"op":"set_instance_input","instance":"A","port":"Rate on","formula":"1"}]"#).unwrap();
        assert!(s.apply("t", &bad).is_err());
        assert!(s.apply("t", &[op(r#"{"op":"remove_module","name":"Tank"}"#)]).is_err());
        assert_eq!(info(&s.model)["instances"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_manifest_is_set_through_operations_and_a_bad_one_is_refused() {
        let mut s = Session::new(Model::default());
        s.apply("t", &stock_model()).unwrap();
        s.apply("t", &[op(r#"{"op":"set_lever","var":"Burn rate","on":true}"#)]).unwrap();
        let draft = crate::manifest::draft(&s.model);
        let good = vec![Op::SetManifest { manifest: draft }];
        s.apply("t", &good).unwrap();
        assert_eq!(info(&s.model)["manifest"], json!(true));
        let bad = op(r#"{"op":"set_manifest","manifest":{"levers":[{"name":"Nope","min":0,"max":1}]}}"#);
        let r = s.apply("agent", &[bad]).unwrap_err();
        assert!(r.issues.iter().any(|i| i.code == "manifest"), "{r:?}");
    }

    #[test]
    fn sets_levers_and_queries_work() {
        let mut s = Session::new(Model::default());
        s.apply("t", &stock_model()).unwrap();
        s.apply("t", &[op(r#"{"op":"set_lever","var":"Burn rate","on":true}"#), op(r#"{"op":"add_set","name":"Region","items":["North","South"]}"#)]).unwrap();
        assert!(s.model.by_name("burn rate").unwrap().lever);
        assert!(s.apply("t", &[op(r#"{"op":"set_lever","var":"Tank","on":true}"#)]).is_err());
        let d = dependencies(&s.model, "Burn").unwrap();
        assert_eq!(d["causes"], json!(["Burn rate", "Tank"]));
        assert_eq!(d["uses"], json!(["Tank"]));
        assert_eq!(dependencies(&s.model, "Tank").unwrap()["causes"], json!(["Burn"]));
        assert_eq!(info(&s.model)["stocks"], json!(1));
        assert_eq!(list(&s.model, Some("flow")).unwrap().as_array().unwrap().len(), 1);
    }
}
