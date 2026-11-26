//! Module definitions and instances. A *definition* is a reusable piece of model — a warehouse,
//! a hiring pipeline, a tank — written once with named, typed ports. An *instance* uses it with
//! its own parameter values and its own inputs. Before a model runs, every instance is expanded
//! into ordinary variables named `Instance.Variable`; the engine never sees modules.
//!
//! Ports are declared. An *in* port is a variable inside the definition whose formula the
//! instance replaces with an expression from the rest of the model; an *out* port is a variable
//! the rest of the model may read, written `"Instance.Port variable"`. Each port has a kind —
//! `material` (a stock or flow, something that is conserved) or `information` (anything else) —
//! and units, which the connection must match.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::expr;
use crate::model::{Kind, Model, Var};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortDirection {
    #[serde(alias = "in")]
    In,
    #[serde(alias = "out")]
    Out,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortKind {
    #[serde(alias = "material")]
    Material,
    #[serde(alias = "information")]
    Information,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortDef {
    pub name: String,
    pub direction: PortDirection,
    pub kind: PortKind,
    #[serde(default)]
    pub units: String,
    /// The variable inside the definition the port stands for.
    pub var: String,
}

/// A variable of a definition, by local name (flow ends are local names too).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DefVar {
    pub name: String,
    pub kind: Kind,
    #[serde(default)]
    pub equation: String,
    #[serde(default)]
    pub units: String,
    #[serde(default)]
    pub doc: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub non_negative: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModuleDef {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub vars: Vec<DefVar>,
    #[serde(default)]
    pub ports: Vec<PortDef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub def: String,
    /// Constants of the definition given a different value (local name → value or formula).
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    /// In ports connected to a formula of the rest of the model (port name → formula).
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
}

pub fn flat(instance: &str, var: &str) -> String {
    format!("{instance}.{var}")
}

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "'"))
}

/// Problems with the definitions themselves (ports naming nothing, material ports on non-stocks).
pub fn check_defs(m: &Model) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for d in &m.defs {
        for p in &d.ports {
            match d.vars.iter().find(|v| expr::normalize(&v.name) == expr::normalize(&p.var)) {
                None => out.push((d.name.clone(), format!("port “{}” stands for “{}”, which is not in the module", p.name, p.var))),
                Some(v) => {
                    let material_kind = matches!(v.kind, Kind::Stock | Kind::Flow);
                    if p.kind == PortKind::Material && !material_kind {
                        out.push((d.name.clone(), format!("material port “{}” must be a stock or a flow, but “{}” is a {}", p.name, p.var, v.kind.label())));
                    }
                    if p.kind == PortKind::Information && material_kind {
                        out.push((d.name.clone(), format!("information port “{}” stands for a {}: mark it material", p.name, v.kind.label())));
                    }
                    if p.direction == PortDirection::In && matches!(v.kind, Kind::Stock) {
                        out.push((d.name.clone(), format!("in port “{}” would replace a stock’s start value; give it an auxiliary or a flow", p.name)));
                    }
                }
            }
        }
    }
    out
}

/// Problems with instances: unknown definitions, parameters and ports, missing inputs, units.
pub fn check_instances(m: &Model) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for i in &m.instances {
        let Some(d) = m.defs.iter().find(|d| expr::normalize(&d.name) == expr::normalize(&i.def)) else {
            out.push((i.name.clone(), format!("there is no module called “{}”", i.def)));
            continue;
        };
        for k in i.params.keys() {
            if !d.vars.iter().any(|v| v.kind == Kind::Constant && expr::normalize(&v.name) == expr::normalize(k)) {
                out.push((i.name.clone(), format!("“{k}” is not a constant of module “{}”, so it cannot be a parameter", d.name)));
            }
        }
        for (k, f) in &i.inputs {
            match d.ports.iter().find(|p| p.direction == PortDirection::In && expr::normalize(&p.name) == expr::normalize(k)) {
                None => out.push((i.name.clone(), format!("“{k}” is not an in port of module “{}”", d.name))),
                Some(_) => {}
            }
        }
        for p in d.ports.iter().filter(|p| p.direction == PortDirection::In) {
            let has_default = d.vars.iter().any(|v| expr::normalize(&v.name) == expr::normalize(&p.var) && !v.equation.trim().is_empty());
            if !i.inputs.contains_key(&p.name) && !has_default {
                out.push((i.name.clone(), format!("in port “{}” is not connected", p.name)));
            }
        }
    }
    out
}

/// Connections whose formula does not carry the units the port declares (a warning, not an error).
pub fn unit_warnings(m: &Model) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let t = crate::units::time_base(m);
    for i in &m.instances {
        let Some(d) = m.defs.iter().find(|d| expr::normalize(&d.name) == expr::normalize(&i.def)) else { continue };
        for (k, f) in &i.inputs {
            let Some(p) = d.ports.iter().find(|p| p.direction == PortDirection::In && expr::normalize(&p.name) == expr::normalize(k)) else { continue };
            if p.units.trim().is_empty() {
                continue;
            }
            if let (Some(want), Some(got)) = (crate::units::parse_unit(&p.units), crate::units::expression_dim(m, f)) {
                let strip = |d: &crate::units::Dim| crate::units::without_time_pub(d, &t);
                if strip(&want) != strip(&got) {
                    out.push((i.name.clone(), format!("input “{k}” expects {} but the connected formula gives {}", crate::units::show(&want), crate::units::show(&got))));
                }
            }
        }
    }
    out
}

/// The model with every instance written out as ordinary variables. None when it has none.
pub fn flatten(m: &Model) -> Result<Option<Model>, Vec<(Option<String>, String)>> {
    if m.instances.is_empty() {
        return Ok(None);
    }
    let mut problems: Vec<(Option<String>, String)> = check_defs(m).into_iter().chain(check_instances(m)).map(|(v, msg)| (Some(v), msg)).collect();
    if !problems.is_empty() {
        return Err(problems);
    }
    let mut out = m.clone();
    out.instances.clear();
    let pos = |k: usize| [60.0 + (k % 6) as f32 * 160.0, 700.0 + (k / 6) as f32 * 80.0];
    let mut k = 0;
    for i in &m.instances {
        let d = m.defs.iter().find(|d| expr::normalize(&d.name) == expr::normalize(&i.def)).unwrap();
        // local name → flat name, and → new id
        let names: HashMap<String, String> = d.vars.iter().map(|v| (expr::normalize(&v.name), flat(&i.name, &v.name))).collect();
        let mut ids: HashMap<String, u64> = HashMap::new();
        for v in &d.vars {
            ids.insert(expr::normalize(&v.name), out.new_id());
        }
        let rewrite = |text: &str| -> String {
            let mut t = text.to_string();
            // longest names first so "Stock level" is not clipped by "Stock"
            let mut order: Vec<&String> = names.keys().collect();
            order.sort_by_key(|n| std::cmp::Reverse(n.len()));
            for n in order {
                t = expr::rename_in(&t, n, &quoted(&names[n]));
            }
            t
        };
        for v in &d.vars {
            let key = expr::normalize(&v.name);
            let mut nv = Var::new(ids[&key], &names[&key], v.kind, pos(k));
            k += 1;
            nv.units = v.units.clone();
            nv.comment = v.doc.clone();
            nv.non_negative = v.non_negative;
            nv.equation = v.equation.clone();
            // an in port takes the connected formula (written in the outer model's names)
            if let Some(p) = d.ports.iter().find(|p| p.direction == PortDirection::In && expr::normalize(&p.var) == key) {
                if let Some(f) = i.inputs.get(&p.name) {
                    nv.equation = f.clone();
                } else {
                    nv.equation = rewrite(&nv.equation);
                }
            } else if let Some(over) = i.params.iter().find(|(n, _)| expr::normalize(n) == key).map(|(_, f)| f) {
                nv.equation = over.clone();
            } else {
                nv.equation = rewrite(&nv.equation);
            }
            nv.from = v.from.as_ref().and_then(|n| ids.get(&expr::normalize(n)).copied());
            nv.to = v.to.as_ref().and_then(|n| ids.get(&expr::normalize(n)).copied());
            out.vars.push(nv);
        }
    }
    problems.clear();
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warehouse() -> ModuleDef {
        let v = |name: &str, kind, eq: &str, units: &str| DefVar { name: name.into(), kind, equation: eq.into(), units: units.into(), doc: String::new(), from: None, to: None, non_negative: false };
        let mut stock = v("Stock", Kind::Stock, "Start", "Unit");
        stock.non_negative = true;
        let mut ship = v("Shipments", Kind::Flow, "MIN(Stock / DT, Demand)", "Unit/Day");
        ship.from = Some("Stock".into());
        let mut receive = v("Receipts", Kind::Flow, "Supply", "Unit/Day");
        receive.to = Some("Stock".into());
        ModuleDef {
            name: "Warehouse".into(),
            description: "Holds goods and ships what is asked".into(),
            vars: vec![stock, ship, receive, v("Start", Kind::Constant, "100", "Unit"), v("Demand", Kind::Aux, "0", "Unit/Day"), v("Supply", Kind::Aux, "0", "Unit/Day")],
            ports: vec![
                PortDef { name: "Demand in".into(), direction: PortDirection::In, kind: PortKind::Information, units: "Unit/Day".into(), var: "Demand".into() },
                PortDef { name: "Supply in".into(), direction: PortDirection::In, kind: PortKind::Information, units: "Unit/Day".into(), var: "Supply".into() },
                PortDef { name: "Stock out".into(), direction: PortDirection::Out, kind: PortKind::Material, units: "Unit".into(), var: "Stock".into() },
            ],
        }
    }

    fn model_with_two() -> Model {
        let mut m = Model::default();
        m.settings.final_time = 10.0;
        m.settings.time_step = 1.0;
        m.settings.time_units = "Day".into();
        m.defs.push(warehouse());
        m.instances.push(Instance { name: "North".into(), def: "Warehouse".into(), params: BTreeMap::from([("Start".into(), "50".into())]), inputs: BTreeMap::from([("Demand in".into(), "10".into()), ("Supply in".into(), "4".into())]) });
        m.instances.push(Instance { name: "South".into(), def: "Warehouse".into(), params: BTreeMap::new(), inputs: BTreeMap::from([("Demand in".into(), "\"North.Stock\" / 20".into()), ("Supply in".into(), "0".into())]) });
        m
    }

    #[test]
    fn one_definition_gives_independent_instances() {
        let m = model_with_two();
        assert!(check_defs(&m).is_empty() && check_instances(&m).is_empty());
        let c = crate::sim::compile(&m).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        let n = r.get("North.Stock").unwrap();
        // 50 start, receives 4 a day, ships up to 10: down 6 a day, then ships what arrives (4 left)
        assert_eq!(n[0], 50.0);
        assert!((n.last().unwrap() - 4.0).abs() < 1e-9, "{:?}", n);
        // South starts from the definition's 100 and follows North through its own demand
        assert_eq!(r.get("South.Stock").unwrap()[0], 100.0);
        assert!(r.get("South.Stock").unwrap().last().unwrap() < &100.0);
    }

    #[test]
    fn a_wrongly_wired_instance_is_explained() {
        let mut m = model_with_two();
        m.instances[0].inputs.remove("Supply in");
        // "Supply" has a default of 0 in the definition, so that is fine; but a bad port and param are not
        m.instances[0].inputs.insert("Nope".into(), "1".into());
        m.instances[1].params.insert("Demand".into(), "5".into());
        let p: Vec<String> = check_instances(&m).into_iter().map(|x| x.1).collect();
        assert!(p.iter().any(|x| x.contains("not an in port")), "{p:?}");
        assert!(p.iter().any(|x| x.contains("not a constant")), "{p:?}");
        assert!(crate::sim::compile(&m).is_err());
    }

    #[test]
    fn ports_have_kinds_and_units_that_connections_must_match() {
        let mut m = model_with_two();
        m.defs[0].ports[0].kind = PortKind::Material; // Demand is an aux, not material
        assert!(check_defs(&m).iter().any(|x| x.1.contains("must be a stock or a flow")));
        let mut m = model_with_two();
        m.instances[0].inputs.insert("Demand in".into(), "Mass".into());
        m.vars.push({ let mut v = Var::new(1, "Mass", Kind::Constant, [0.0, 0.0]); v.equation = "3".into(); v.units = "Kilogram".into(); v });
        assert!(unit_warnings(&m).iter().any(|x| x.1.contains("expects")), "{:?}", unit_warnings(&m));
    }
}
