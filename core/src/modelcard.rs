//! The model card of `dyn pack`: filled only from what the model itself states (the manifest and the brief).
//! A field the model does not state is written "not stated", never guessed. Layout after Mitchell et al. (2019).

use crate::model::Model;

pub const NOT_STATED: &str = "not stated";

fn or_not(s: &str) -> String {
    if s.trim().is_empty() { NOT_STATED.to_string() } else { s.trim().to_string() }
}

/// The card as Markdown. `source` is the file name shown under the title.
pub fn card(m: &Model, source: &str) -> String {
    let mf = &m.manifest;
    let (problem, kind, out_of_scope) = match &m.purpose {
        Some(p) => (p.problem.clone(), p.kind.clone(), p.out_of_scope.clone()),
        None => (String::new(), String::new(), String::new()),
    };
    let mut s = format!("# Model card: {}\n\nFile: {source}\n\n", or_not(&mf.title));
    s.push_str(&format!("## Description\n\n{}\n\n", or_not(&mf.description)));
    s.push_str(&format!("## Intended use\n\nProblem: {}\n\nKind of model: {}\n\n", or_not(&problem), or_not(&kind)));
    s.push_str(&format!("## Out of scope\n\n{}\n\n", or_not(&out_of_scope)));
    s.push_str("## Validity\n\n");
    if mf.validity.is_empty() {
        s.push_str(&format!("{NOT_STATED}\n\n"));
    } else {
        for v in &mf.validity {
            let mut extra = Vec::new();
            if let Some(x) = &v.variable {
                extra.push(format!("variable {x}"));
            }
            if let (Some(a), Some(b)) = (v.min, v.max) {
                extra.push(format!("tried from {a} to {b}"));
            }
            if let Some(h) = v.horizon {
                extra.push(format!("trusted up to a horizon of {h} {}", m.settings.time_units.trim()));
            }
            s.push_str(&format!("- {}{}\n", v.statement.trim(), if extra.is_empty() { String::new() } else { format!(" ({})", extra.join("; ")) }));
        }
        s.push('\n');
    }
    s.push_str("## Goals\n\n");
    if mf.goals.is_empty() {
        s.push_str(&format!("{NOT_STATED}\n\n"));
    } else {
        for g in &mf.goals {
            s.push_str(&format!("- {}: {} of {}, direction {}\n", g.name, if g.measure.is_empty() { "final" } else { &g.measure }, g.output, if g.direction.is_empty() { "max" } else { &g.direction }));
        }
        s.push('\n');
    }
    s.push_str("## Objectives of the brief\n\n");
    match m.purpose.as_ref().filter(|p| !p.objectives.is_empty()) {
        None => s.push_str(&format!("{NOT_STATED}\n\n")),
        Some(p) => {
            for o in &p.objectives {
                s.push_str(&format!("- {}: {} (test: {}; measured by {})\n", o.title, or_not(&o.kind), or_not(&o.test), or_not(&o.var)));
            }
            s.push('\n');
        }
    }
    s.push_str("## Tests that vouch for the model\n\n");
    if mf.tests.is_empty() {
        s.push_str(&format!("{NOT_STATED}\n\n"));
    } else {
        for t in &mf.tests {
            s.push_str(&format!("- {t}\n"));
        }
        s.push('\n');
    }
    s.push_str("## Levers (what may be changed, and within what range)\n\n");
    if mf.levers.is_empty() {
        s.push_str(&format!("{NOT_STATED}\n"));
    } else {
        for l in &mf.levers {
            s.push_str(&format!("- {}: {} to {}{}\n", l.name, l.min, l.max, if l.units.is_empty() { String::new() } else { format!(" {}", l.units) }));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Validity;

    #[test]
    fn what_the_model_does_not_state_is_not_stated() {
        let mut m = Model::default();
        let empty = card(&m, "m.sdm");
        assert!(empty.matches(NOT_STATED).count() >= 8, "{empty}");
        m.manifest.title = "Tank".into();
        m.manifest.description = "A bathtub.".into();
        m.manifest.validity.push(Validity { statement: "Short runs".into(), horizon: Some(30.0), ..Default::default() });
        let c = card(&m, "m.sdm");
        assert!(c.contains("# Model card: Tank") && c.contains("A bathtub.") && c.contains("Short runs (trusted up to a horizon of 30"));
        // delete the description: it is "not stated" again, and nothing is made up
        m.manifest.description.clear();
        assert!(card(&m, "m.sdm").contains("## Description\n\nnot stated"));
    }
}
