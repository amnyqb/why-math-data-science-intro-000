//! Stage C7, rules P: purpose and architecture.
//!
//! P1: no modelling brief (the question, the KPIs). P2: a variable owned by two layers, or a link across layers with no
//! declared contract (the layer checks of `layers.rs`). P3: a benchmark whose published range the model misses.

use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::ops::Op;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "P1", stage: Stage::C4, about: "No modelling brief: the question, the kind of model and the behaviour wanted.", f: p1 }),
        Box::new(FnRule { id: "P2", stage: Stage::C4, about: "A variable owned by two layers, or a link across layers with no declared contract.", f: p2 }),
        Box::new(FnRule { id: "P3", stage: Stage::C4, about: "A benchmark with a published source that the base run misses.", f: p3 }),
    ]
}

fn p1(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if m.purpose.as_ref().is_some_and(|p| !p.problem.trim().is_empty()) {
        return Vec::new();
    }
    let mut f = Finding::new("P1", Severity::Info, None, "The model has no brief: write the question it answers, the key results (KPIs) and the behaviour it should show");
    f.source = "The book, chapter 18".into();
    f.fixes.push(Fix { title: "Add a brief to fill in".into(), ops: vec![Op::SetPurpose { problem: "TODO: the question this model answers, in one sentence".into(), kind: "explain a behaviour".into(), out_of_scope: String::new() }], keeps_numbers: true, needs_decision: false });
    vec![f]
}

fn p2(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if m.layers.is_empty() {
        return Vec::new();
    }
    crate::layers::check(m)
        .issues
        .into_iter()
        .filter(|i| matches!(i.code.as_str(), "layer-contract" | "layer-crossing" | "layer-no-contracts" | "layer-cycle"))
        .map(|i| {
            let mut f = Finding::new("P2", Severity::Warning, i.var.as_deref(), i.message);
            f.source = "Dynamist architecture specification".into();
            f.fixes.push(Fix { title: "Write the interfaces down as contracts".into(), ops: vec![Op::FreezeInterfaces {}], keeps_numbers: true, needs_decision: false });
            f
        })
        .collect()
}

fn p3(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let Some(base) = ctx.base() else { return Vec::new() };
    let mut out = Vec::new();
    for c in m.checks.iter().filter(|c| !c.source.trim().is_empty()) {
        let Some(s) = base.get(&c.var) else { continue };
        let x = match c.at {
            Some(t) => base.times.iter().position(|x| (*x - t).abs() < 1e-9).and_then(|i| s.get(i)).copied(),
            None => s.last().copied(),
        };
        if let Some(x) = x {
            if c.min.is_some_and(|lo| x < lo) || c.max.is_some_and(|hi| x > hi) {
                let mut f = Finding::new("P3", Severity::Error, Some(&c.var), format!("“{}” is {} but the benchmark “{}” says {} to {} ({})", c.var, super::num(x), c.name, c.min.map(crate::fmt::fmt_num).unwrap_or_default(), c.max.map(crate::fmt::fmt_num).unwrap_or_default(), c.source));
                f.source = c.source.clone();
                out.push(f);
            }
        }
    }
    out
}
