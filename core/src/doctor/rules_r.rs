//! Stage C7, rules R: reproducibility and documentation.
//!
//! R1: variables with no description (documentation coverage). R3: no run manifest (levers, outputs): nobody else can run
//! the model without reading it. R2: random functions with no recorded seed. R4: a model that loses equations when exported.

use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::ops::Op;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "R1", stage: Stage::C4, about: "Variables with no description.", f: r1 }),
        Box::new(FnRule { id: "R2", stage: Stage::C4, about: "Random functions with no recorded seed.", f: r2 }),
        Box::new(FnRule { id: "R4", stage: Stage::C4, about: "A model that cannot be exported with its equations.", f: r4 }),
        Box::new(FnRule { id: "R3", stage: Stage::C4, about: "No run manifest: levers with ranges, outputs.", f: r3 }),
    ]
}

fn r1(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let bare: Vec<&crate::model::Var> = m.vars.iter().filter(|v| (v.comment.trim().is_empty() || (v.units.trim().is_empty() && v.kind != crate::model::Kind::Lookup)) && !v.name.starts_with("zz ")).collect();
    if bare.is_empty() {
        return Vec::new();
    }
    let mut f = Finding::new("R1", Severity::Warning, Some(&bare[0].name), format!("{} of {} variables have no description or no units, starting with “{}”", bare.len(), m.vars.len(), bare[0].name));
    f.source = "SDM-Doc (Martinez-Moyano 2012): every variable is documented".into();
    f.fixes.push(Fix {
        title: format!("Add a prompt to describe the {} undocumented variables", bare.len()),
        ops: bare.iter().map(|v| Op::SetDoc { name: v.name.clone(), doc: format!("TODO: what is {} and where does its number come from?", v.name) }).collect(),
        keeps_numbers: true, needs_decision: false,
    });
    vec![f]
}

fn r3(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if !m.manifest.levers.is_empty() || !m.manifest.outputs.is_empty() || m.vars.iter().filter(|v| v.kind == crate::model::Kind::Constant).count() == 0 {
        return Vec::new();
    }
    let mut f = Finding::new("R3", Severity::Warning, None, "The model has no run manifest (levers with ranges, outputs), so a reader cannot tell what may be changed or what to look at");
    f.source = "Reproducibility practice: state the inputs, their ranges and the results that matter".into();
    f.fixes.push(Fix { title: "Write a draft manifest from the constants and the stocks".into(), ops: vec![Op::SetManifest { manifest: crate::manifest::draft(m) }], keeps_numbers: true, needs_decision: false });
    vec![f]
}

const RANDOMS: &[&str] = &["RANDOM", "NORMAL(", "UNIFORM(", "POISSON(", "EXPRND(", "TRIANGULAR(", "LOGNORMAL(", "WEIBULL("];

fn r2(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    if m.settings.seed_recorded {
        return Vec::new();
    }
    let user = m.vars.iter().find(|v| { let e = v.equation.to_uppercase(); RANDOMS.iter().any(|r| e.contains(r)) });
    let Some(v) = user else { return Vec::new() };
    let mut f = Finding::new("R2", Severity::Warning, Some(&v.name), format!("“{}” draws random numbers but the model does not record its seed: nobody can reproduce the run", v.name));
    f.source = "Dynamist function specification; Rahmandad and Sterman (2012)".into();
    f.fixes.push(Fix { title: format!("Record the model seed ({})", m.settings.seed), ops: vec![Op::SetSetting { key: "seed".into(), value: m.settings.seed.to_string() }], keeps_numbers: true, needs_decision: false });
    vec![f]
}

fn r4(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let Ok((_, loss)) = crate::io::write_str(m, crate::io::Format::Xmile) else {
        let mut f = Finding::new("R4", Severity::Warning, None, "The model cannot be exported to XMILE at all");
        f.source = "Rahmandad and Sterman (2012)".into();
        return vec![f];
    };
    let dropped: Vec<String> = loss.items.iter().filter(|i| i.construct.to_lowercase().starts_with("function")).map(|i| format!("{} ({})", i.construct, i.detail)).collect();
    if dropped.is_empty() {
        return Vec::new();
    }
    let mut f = Finding::new("R4", Severity::Warning, None, format!("The XMILE export loses equations: {}", dropped.join("; ")));
    f.source = "Rahmandad and Sterman (2012)".into();
    vec![f]
}
