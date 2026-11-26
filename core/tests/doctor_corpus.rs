//! Phase 2: one deliberately broken model per rule (`core/tests/doctor/<ID>.json`, a batch of operations that builds the model;
//! its title says which rule it breaks). For each: the doctor reports the rule, applying the first fix removes it, and the fix
//! adds no new finding of equal or higher severity. A rule that only informs is tested for the finding alone.

use dynamist_core::doctor::{self, Finding, Options, Severity};
use dynamist_core::model::Model;
use dynamist_core::ops::{self, Op};

/// Build the model from the batch. Besides operations a batch may hold `{"raw": {...}}` (a variable put in as it is, for
/// the mistakes that `add_var` refuses: a name twice, an empty equation, an equation that does not parse) and
/// `{"special": "cld_from_model"}` (a causal loop sheet called CLD made from the model).
fn build(path: &std::path::Path) -> Result<Model, String> {
    let items: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).map_err(|e| format!("{path:?}: {e}"))?;
    let mut m = Model::default();
    m.settings.units_scale_values = true;
    for it in items {
        if let Some(raw) = it.get("raw") {
            let id = m.new_id();
            let kind = match raw["kind"].as_str().unwrap() { "stock" => dynamist_core::model::Kind::Stock, "flow" => dynamist_core::model::Kind::Flow, "constant" => dynamist_core::model::Kind::Constant, "lookup" => dynamist_core::model::Kind::Lookup, _ => dynamist_core::model::Kind::Aux };
            let mut v = dynamist_core::model::Var::new(id, raw["name"].as_str().unwrap(), kind, [0.0, 0.0]);
            v.equation = raw["equation"].as_str().unwrap().into();
            v.units = raw["units"].as_str().unwrap_or("").into();
            v.comment = raw["comment"].as_str().unwrap_or("").into();
            if let Some(t) = raw.get("table") {
                v.table = serde_json::from_value(t.clone()).unwrap();
            }
            m.vars.push(v);
        } else if it.get("special").is_some() {
            dynamist_core::ops::sync_links_for_tests(&mut m);
            dynamist_core::figure::cld_from_model(&mut m, &[], "CLD").map_err(|e| format!("{path:?}: {e}"))?;
        } else {
            let op: Op = serde_json::from_value(it.clone()).map_err(|e| format!("{path:?}: {it}: {e}"))?;
            ops::apply(&mut m, &op).map_err(|e| format!("{path:?}: {op:?}: {e}"))?;
        }
    }
    Ok(m)
}

fn opts() -> Options {
    Options { experiments: true, jobs: 2, stage: Some(dynamist_core::doctor::Stage::C7), ..Default::default() }
}

fn key(f: &Finding) -> (String, Option<String>) {
    (f.rule.clone(), f.var.clone())
}

#[test]
fn every_rule_has_its_broken_model() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/doctor");
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    let mut table: Vec<String> = doctor::rules().iter().map(|r| r.id().to_string()).collect();
    table.sort();
    let mut stems: Vec<String> = files.iter().map(|p| p.file_stem().unwrap().to_string_lossy().to_string()).collect();
    stems.sort();
    assert_eq!(table, stems, "the rule table and the corpus must list the same rules");
    assert_eq!(files.len(), 72, "one model for each of the 72 rules of the catalogue");
    let mut problems = Vec::new();
    for p in &files {
        let id = p.file_stem().unwrap().to_string_lossy().to_string();
        let m = match build(p) {
            Ok(m) => m,
            Err(e) => {
                problems.push(format!("{id}: cannot build: {e}"));
                continue;
            }
        };
        let before = doctor::check(&m, &opts());
        let Some(hit) = before.findings.iter().find(|f| f.rule == id || f.code == id) else {
            problems.push(format!("{id}: the doctor did not report it. Findings: {:?}", before.findings.iter().map(|f| (&f.rule, &f.var)).collect::<Vec<_>>()));
            continue;
        };
        let Some(fix) = hit.fixes.first() else { continue };
        let mut fixed = m.clone();
        if let Err(e) = doctor::apply_fix(&mut fixed, fix) {
            problems.push(format!("{id}: the first fix does not apply: {e}"));
            continue;
        }
        let after = doctor::check(&fixed, &opts());
        if after.findings.iter().any(|f| (f.rule == id || f.code == id) && f.var == hit.var) {
            problems.push(format!("{id}: the first fix did not remove the finding"));
        }
        let old: Vec<_> = before.findings.iter().map(key).collect();
        for f in &after.findings {
            // the control the specification asks F1 and F2 to insert has a time constant of 2 DT, which N2 and N3 then flag:
            // a consequence of the specified fix, not a mistake of the fix
            let expected = matches!(id.as_str(), "F1" | "F2") && matches!(f.rule.as_str(), "N2" | "N3");
            if !expected && !old.contains(&key(f)) && f.severity >= hit.severity && f.severity != Severity::Info {
                problems.push(format!("{id}: the fix added {} ({:?}) on {:?}: {}", f.rule, f.severity, f.var, f.message));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
