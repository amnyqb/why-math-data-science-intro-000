//! The architecture of a model, worked out from the model itself: the layers as lanes, the
//! subsystems (groups) inside each lane, the contracts between lanes, and what each variable *is*
//! (exogenous, lever, constant, lookup, switch, member of a feedback loop, KPI of an objective).
//!
//! Nothing here changes the model. The only things a person writes are the layers, the groups and
//! the purpose; every role below is derived, so it is always current.

use std::collections::{HashMap, HashSet};

use crate::expr;
use crate::layers::{self, Layer};
use crate::model::{Kind, Model, Var};

/// What one variable is, in the vocabulary of the architecture view.
#[derive(Clone, Debug, Default)]
pub struct VarInfo {
    /// Depends on no stock, directly or through other variables: it comes from outside the feedback structure.
    pub exogenous: bool,
    pub lever: bool,
    /// A 0 or 1 constant used inside an IF.
    pub switch: bool,
    /// Labels of the feedback loops it lies on ("R1", "B2").
    pub loops: Vec<String>,
    /// The objective it measures (1-based).
    pub kpi: Option<usize>,
}

/// A group of variables inside a layer; `id` is None for the variables nobody grouped.
#[derive(Clone, Debug)]
pub struct Subsystem {
    pub id: Option<u64>,
    pub name: String,
    pub vars: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct Lane {
    pub layer: Layer,
    /// 1 for the outermost layer.
    pub number: usize,
    pub subsystems: Vec<Subsystem>,
}

impl Lane {
    /// "Layer 2: Capital", or "Layer 2" when the layer has no title of its own.
    pub fn heading(&self) -> String {
        let n = self.layer.name.trim();
        if n.is_empty() || n.eq_ignore_ascii_case(&format!("layer {}", self.number)) {
            format!("Layer {}", self.number)
        } else {
            format!("Layer {}: {}", self.number, n)
        }
    }
    pub fn var_count(&self) -> usize {
        self.subsystems.iter().map(|s| s.vars.len()).sum()
    }
}

#[derive(Clone, Debug)]
pub struct Contract {
    pub var: String,
    pub var_id: Option<u64>,
    /// Lane indexes.
    pub from: usize,
    pub to: usize,
    pub kind: String,
    pub used_by: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct LoopInfo {
    pub label: String,
    pub reinforcing: bool,
    pub vars: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Architecture {
    pub lanes: Vec<Lane>,
    pub contracts: Vec<Contract>,
    pub loops: Vec<LoopInfo>,
    pub info: HashMap<u64, VarInfo>,
    /// Variables in no layer (only listed when the model has layers).
    pub unassigned: Vec<u64>,
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

/// Variables that depend on the state of the model: stocks, and anything that reads one, however indirectly.
pub fn state_dependent(m: &Model) -> HashSet<u64> {
    let by_key: HashMap<String, u64> = m.vars.iter().map(|v| (expr::normalize(&v.name), v.id)).collect();
    let refs: HashMap<u64, Vec<u64>> = m
        .vars
        .iter()
        .map(|v| {
            let mut r: Vec<u64> = refs_of(v).iter().filter_map(|n| by_key.get(&expr::normalize(n)).copied()).collect();
            r.extend(v.from.into_iter().chain(v.to));
            (v.id, r)
        })
        .collect();
    let mut dep: HashSet<u64> = m.vars.iter().filter(|v| v.kind == Kind::Stock).map(|v| v.id).collect();
    loop {
        let before = dep.len();
        for v in &m.vars {
            if !dep.contains(&v.id) && refs[&v.id].iter().any(|r| dep.contains(r)) {
                dep.insert(v.id);
            }
        }
        if dep.len() == before {
            break;
        }
    }
    dep
}

pub fn build(m: &Model) -> Architecture {
    let mut a = Architecture::default();
    // roles
    let dep = state_dependent(m);
    let loops = crate::ops::loops(m);
    let by_key: HashMap<String, u64> = m.vars.iter().map(|v| (expr::normalize(&v.name), v.id)).collect();
    let (mut r, mut b) = (0, 0);
    for l in loops.as_array().into_iter().flatten() {
        let rein = l["reinforcing"].as_bool().unwrap_or(false);
        let label = if rein {
            r += 1;
            format!("R{r}")
        } else {
            b += 1;
            format!("B{b}")
        };
        let vars: Vec<String> = l["variables"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
        for n in &vars {
            if let Some(id) = by_key.get(&expr::normalize(n)) {
                a.info.entry(*id).or_default().loops.push(label.clone());
            }
        }
        a.loops.push(LoopInfo { label, reinforcing: rein, vars });
    }
    let if_text: Vec<String> = m.vars.iter().filter(|v| v.equation.to_uppercase().contains("IF ")).map(|v| expr::normalize(&v.equation)).collect();
    for v in &m.vars {
        let i = a.info.entry(v.id).or_default();
        i.lever = v.lever;
        i.exogenous = !dep.contains(&v.id) && matches!(v.kind, Kind::Aux | Kind::Constant | Kind::Lookup);
        let is_flag = matches!(v.equation.trim(), "0" | "1");
        i.switch = v.kind == Kind::Constant && is_flag && if_text.iter().any(|t| t.contains(&expr::normalize(&v.name)));
    }
    if let Some(p) = &m.purpose {
        for (n, o) in p.objectives.iter().enumerate() {
            if let Some(id) = by_key.get(&expr::normalize(&o.var)) {
                a.info.entry(*id).or_default().kpi = Some(n + 1);
            }
        }
    }
    // lanes
    let mut order: Vec<&Layer> = m.layers.iter().collect();
    order.sort_by_key(|l| (l.order, l.id));
    let lane_of: HashMap<u64, usize> = order.iter().enumerate().map(|(i, l)| (l.id, i)).collect();
    let mut lanes: Vec<Lane> = order.iter().enumerate().map(|(i, l)| Lane { layer: (*l).clone(), number: i + 1, subsystems: Vec::new() }).collect();
    let mut var_lane: HashMap<u64, usize> = HashMap::new();
    for v in &m.vars {
        match layers::layer_of(m, v).and_then(|id| lane_of.get(&id).copied()) {
            Some(i) => {
                var_lane.insert(v.id, i);
            }
            None => {
                if !m.layers.is_empty() {
                    a.unassigned.push(v.id);
                }
            }
        }
    }
    // a model without layers is one implicit lane
    if lanes.is_empty() && !m.vars.is_empty() {
        lanes.push(Lane { layer: Layer { id: 0, name: m.title.clone(), order: 0, question: String::new(), horizon: None, structural_limit: None, status: layers::LayerStatus::default(), colour: String::new(), description: String::new(), owner: String::new(), spheres: Vec::new() }, number: 1, subsystems: Vec::new() });
        for v in &m.vars {
            var_lane.insert(v.id, 0);
        }
    }
    // subsystems: each group goes to the lane most of its members are in
    let mut grouped: HashSet<u64> = HashSet::new();
    for g in &m.groups {
        let mut count: HashMap<usize, usize> = HashMap::new();
        for id in &g.members {
            if let Some(l) = var_lane.get(id) {
                *count.entry(*l).or_default() += 1;
            }
        }
        let Some((&lane, _)) = count.iter().max_by_key(|(l, c)| (**c, std::cmp::Reverse(**l))) else { continue };
        let vars: Vec<u64> = g.members.iter().copied().filter(|id| var_lane.get(id) == Some(&lane)).collect();
        grouped.extend(vars.iter().copied());
        lanes[lane].subsystems.push(Subsystem { id: Some(g.id), name: g.name.clone(), vars });
    }
    for (i, lane) in lanes.iter_mut().enumerate() {
        let rest: Vec<u64> = m.vars.iter().filter(|v| var_lane.get(&v.id) == Some(&i) && !grouped.contains(&v.id)).map(|v| v.id).collect();
        if !rest.is_empty() {
            let name = if lane.subsystems.is_empty() { "Variables".to_string() } else { "Other".to_string() };
            lane.subsystems.push(Subsystem { id: None, name, vars: rest });
        }
    }
    a.lanes = lanes;
    for c in layers::crossings(m) {
        if let (Some(f), Some(t)) = (lane_of.get(&c.from), lane_of.get(&c.to)) {
            let var_id = by_key.get(&expr::normalize(&c.var)).copied();
            a.contracts.push(Contract { var: c.var.clone(), var_id, from: *f, to: *t, kind: c.kind.clone(), used_by: c.used_by.clone() });
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{apply, Op};

    fn model() -> Model {
        let mut m = Model::default();
        m.settings.final_time = 20.0;
        m.ensure_sheets();
        let ops: Vec<Op> = serde_json::from_str(
            r#"[{"op":"add_var","name":"Firm","kind":"stock","equation":"10"},
                {"op":"add_var","name":"Climate","kind":"aux","equation":"1 + 0.02 * TIME"},
                {"op":"add_var","name":"Rate","kind":"constant","equation":"0.05"},
                {"op":"add_var","name":"Boost","kind":"constant","equation":"1"},
                {"op":"add_var","name":"Growth","kind":"flow","equation":"Firm * Rate * Climate * IF THEN ELSE(Boost > 0, 2, 1)","to":"Firm"},
                {"op":"add_layer","name":"World","order":0},
                {"op":"add_layer","name":"Firm","order":1},
                {"op":"assign_layer","layer":"World","vars":["Climate"]},
                {"op":"assign_layer","layer":"Firm","vars":["Firm","Rate","Boost","Growth"]},
                {"op":"add_group","name":"Money","vars":["Firm","Growth"]},
                {"op":"set_lever","var":"Rate","on":true},
                {"op":"set_purpose","problem":"Does it grow?","kind":"explain a behaviour"},
                {"op":"add_objective","title":"Grow","kind":"grow","test":"Firm rises","var":"Firm"}]"#,
        )
        .unwrap();
        for o in &ops {
            apply(&mut m, o).unwrap();
        }
        m
    }

    #[test]
    fn roles_lanes_groups_and_contracts_are_derived() {
        let m = model();
        let a = build(&m);
        let id = |n: &str| m.vars.iter().find(|v| v.name == n).unwrap().id;
        assert!(a.info[&id("Climate")].exogenous && a.info[&id("Rate")].exogenous);
        assert!(!a.info[&id("Growth")].exogenous && !a.info[&id("Firm")].exogenous);
        assert!(a.info[&id("Rate")].lever && a.info[&id("Boost")].switch && !a.info[&id("Rate")].switch);
        assert_eq!(a.info[&id("Firm")].kpi, Some(1));
        assert!(a.info[&id("Firm")].loops.is_empty() || a.info[&id("Firm")].loops[0].starts_with(['R', 'B']));
        assert_eq!(a.lanes.len(), 2);
        assert_eq!(a.lanes[1].heading(), "Layer 2: Firm");
        let names: Vec<&str> = a.lanes[1].subsystems.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Money", "Other"]);
        assert_eq!(a.contracts.len(), 1);
        assert_eq!((a.contracts[0].var.as_str(), a.contracts[0].from, a.contracts[0].to), ("Climate", 0, 1));
        // a variable moves to a new group and leaves the old one
        let mut m2 = m.clone();
        apply(&mut m2, &serde_json::from_str::<Op>(r#"{"op":"add_group","name":"Pipe","vars":["Growth"]}"#).unwrap()).unwrap();
        assert_eq!(m2.groups.iter().find(|g| g.name == "Money").unwrap().members.len(), 1);
    }

    #[test]
    fn a_model_without_layers_is_one_lane() {
        let mut m = model();
        m.layers.clear();
        m.interfaces.clear();
        let a = build(&m);
        assert_eq!(a.lanes.len(), 1);
        assert_eq!(a.lanes[0].var_count(), 5 - 0);
        assert!(a.unassigned.is_empty());
    }
}
