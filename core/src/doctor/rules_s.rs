//! Stage C1, rules S: names and parsing.
//!
//! S1: a name that no variable has. S2: an empty equation (or a stock with no initial value). S3: one name defined twice.
//! S4: an equation that cannot be read (the parser's message says where and what was expected).
//!
//! S1: a name that no variable has. Suggests the closest defined name by Levenshtein distance after normalising case and
//! spaces, if the distance is 3 or less. Fixes: rename to the suggestion; create the variable.

use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::expr;
use crate::model::Kind;
use crate::ops::Op;
use crate::unit_table::edit_distance;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "S1", stage: Stage::C1, about: "A name in an equation that no variable has (with the closest defined name as a suggestion).", f: s1 }),
        Box::new(FnRule { id: "S2", stage: Stage::C1, about: "A variable with an empty equation, or a stock with no initial value.", f: s2 }),
        Box::new(FnRule { id: "S3", stage: Stage::C1, about: "The same name defined twice.", f: s3 }),
        Box::new(FnRule { id: "S4", stage: Stage::C1, about: "An equation with a syntax error: where, and what was expected.", f: s4 }),
    ]
}

fn squash(s: &str) -> String {
    expr::normalize(s).replace(' ', "")
}

fn s1(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for p in ctx.problems() {
        let Some(rest) = p.message.strip_prefix("Unknown name '") else { continue };
        let Some(name) = rest.strip_suffix('\'') else { continue };
        let Some(owner) = p.var.as_deref() else { continue };
        let want = squash(name);
        let best = m.vars.iter().map(|v| (edit_distance(&want, &squash(&v.name)), v)).filter(|(d, v)| *d <= 3 && squash(&v.name) != want && v.name != owner).min_by_key(|(d, _)| *d);
        let mut f = Finding::new("S1", Severity::Error, Some(owner), format!("“{owner}” uses the name “{name}”, which no variable has{}", best.as_ref().map(|(_, v)| format!(" (did you mean “{}”?)", v.name)).unwrap_or_default()));
        f.source = "Vensim and Stella manuals: every name in an equation must be defined".into();
        if let (Some((_, v)), Some(ov)) = (&best, m.by_name(&expr::normalize(owner))) {
            f.fixes.push(Fix { title: format!("Rename “{name}” to “{}” in {owner}", v.name), ops: vec![Op::SetEquation { name: owner.into(), equation: expr::rename_in(&ov.equation, &expr::normalize(name), &v.name) }], keeps_numbers: false, needs_decision: false });
        }
        f.fixes.push(Fix {
            title: format!("Create the constant “{name}” (value 0) so that {owner} has what it asks for"),
            ops: vec![Op::AddVar { name: name.into(), kind: "constant".into(), equation: "0".into(), units: String::new(), doc: "Created by the doctor: set its value".into(), from: None, to: None, non_negative: false, layer: None }],
            keeps_numbers: false, needs_decision: false,
        });
        let _ = Kind::Aux;
        out.push(f);
    }
    out
}

fn s2(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in &m.vars {
        if !v.equation.trim().is_empty() || (v.kind == Kind::Lookup && !v.table.is_empty()) {
            continue;
        }
        let what = if v.kind == Kind::Stock { "has no initial value" } else { "has no equation" };
        let mut f = Finding::new("S2", Severity::Error, Some(&v.name), format!("“{}” {what}", v.name));
        f.source = "MIT D-4851 checklist, item 11: every variable has an equation".into();
        // a stock is started where its flows balance (rule I2), when that can be found
        let start = if v.kind == Kind::Stock {
            let mut c = m.clone();
            if let Some(x) = c.var_mut(v.id) {
                x.equation = "0".into();
            }
            super::rules_i::solve_equilibrium(&c).and_then(|sol| sol.into_iter().find(|(n, _)| expr::normalize(n) == expr::normalize(&v.name)).map(|(_, x)| super::fixes::clean_factor(x)))
        } else {
            None
        };
        let eq = start.unwrap_or_else(|| "0".into());
        f.fixes.push(Fix { title: format!("Give “{}” the equation {eq} (a template: set the real one)", v.name), ops: vec![Op::SetEquation { name: v.name.clone(), equation: eq }], keeps_numbers: false, needs_decision: false });
        out.push(f);
    }
    out
}

fn s3(ctx: &Ctx) -> Vec<Finding> {
    let mut seen: std::collections::HashMap<String, usize> = Default::default();
    let mut out = Vec::new();
    for v in &ctx.model.vars {
        let n = expr::normalize(&v.name);
        let c = seen.entry(n).or_insert(0);
        *c += 1;
        if *c == 2 {
            let mut f = Finding::new("S3", Severity::Error, Some(&v.name), format!("The name “{}” is defined twice", v.name));
            f.source = "Vensim help: Check Model, each name is defined once".into();
            f.fixes.push(Fix { title: format!("Rename the first “{}” to “{} 2”", v.name, v.name), ops: vec![Op::RenameVar { name: v.name.clone(), to: format!("{} 2", v.name) }], keeps_numbers: false, needs_decision: false });
            out.push(f);
        }
    }
    out
}

fn s4(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for v in &ctx.model.vars {
        if v.equation.trim().is_empty() || (v.kind == Kind::Lookup && v.equation.trim().is_empty()) {
            continue;
        }
        if let Err(e) = expr::parse(&v.equation) {
            let mut f = Finding::new("S4", Severity::Error, Some(&v.name), format!("The equation of “{}” cannot be read: {e}", v.name));
            f.source = "Vensim help: Check Model, syntax".into();
            out.push(f);
        }
    }
    out
}
