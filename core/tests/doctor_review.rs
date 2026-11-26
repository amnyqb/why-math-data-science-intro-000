//! Phase 2, section 3: `book_models_pass_review` (no false alarms on the book's models) and the speed test.

use dynamist_core::doctor::{self, Options, Severity, Stage};
use dynamist_core::io;

mod par;
use dynamist_core::model::{Kind, Model, Var};

/// The reviewed allow-list: lines of `model | rule | variable | reason`, `#` for comments.
fn allow() -> Vec<(String, String, String)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book_models/allow.toml");
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut out = Vec::new();
    let (mut model, mut rule, mut var) = (String::new(), String::new(), String::new());
    for line in text.lines() {
        let line = line.trim();
        if line == "[[allow]]" {
            (model, rule, var) = Default::default();
        } else if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').to_string();
            match k.trim() {
                "model" => model = v,
                "rule" => rule = v,
                "var" => var = v,
                "reason" => {
                    assert!(!v.trim().is_empty(), "an allow-list entry needs a reason");
                    out.push((model.clone(), rule.clone(), var.clone()));
                }
                _ => {}
            }
        }
    }
    out
}

#[test]
fn book_models_pass_review() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book_models");
    let allowed = allow();
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
    files.sort();
    let models: Vec<(String, dynamist_core::model::Model)> = files
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy().to_string();
            let text = io::read_text(p).ok()?;
            let fmt = io::detect(&name, &text)?;
            io::read_str(&text, fmt).ok().map(|(m, _)| (name, m))
        })
        .collect();
    let results = par::par_map(&models, |(name, m)| (name.clone(), doctor::check(m, &Options::default())));
    let mut bad = Vec::new();
    let mut unused: Vec<bool> = vec![false; allowed.len()];
    for (name, r) in results {
        // an interval "can" finding that no run confirmed is not an error for the Review gate (Part C3 of the specification)
        for f in r.findings.iter().filter(|f| f.severity == Severity::Error && f.confirmed) {
            let hit = allowed.iter().position(|(a, rule, var)| *a == name && (*rule == f.rule || *rule == f.code) && (var.is_empty() || Some(var) == f.var.as_ref()));
            match hit {
                Some(i) => unused[i] = true,
                None => bad.push(format!("{name}: {} {:?}: {}", f.rule, f.var, f.message)),
            }
        }
    }
    assert!(bad.is_empty(), "{} findings not in allow.toml:\n{}", bad.len(), bad.join("\n"));
    let stale: Vec<_> = allowed.iter().zip(&unused).filter(|(_, u)| !**u).map(|(a, _)| format!("{a:?}")).collect();
    assert!(stale.is_empty(), "allow.toml entries that no longer match anything: {}", stale.join(", "));
}

/// A chain of 500 variables: stages C1 to C3 take under 200 ms (in a release build; a debug build is only checked to finish).
#[test]
fn the_early_stages_are_fast_on_500_variables() {
    let mut m = Model::default();
    m.settings.final_time = 20.0;
    m.settings.time_step = 0.25;
    m.settings.time_units = "Day".into();
    let mut id = 1;
    let mut add = |m: &mut Model, name: String, kind: Kind, eq: String, units: &str| {
        let mut v = Var::new(id, &name, kind, [0.0, 0.0]);
        v.equation = eq;
        v.units = units.into();
        v.comment = "test".into();
        m.vars.push(v);
        id += 1;
    };
    add(&mut m, "Base".into(), Kind::Constant, "2".into(), "widget/Day");
    add(&mut m, "Level".into(), Kind::Stock, "10".into(), "widget");
    for i in 0..497 {
        let prev = if i == 0 { "Base".to_string() } else { format!("Step {}", i - 1) };
        add(&mut m, format!("Step {i}"), Kind::Aux, format!("{prev} + Base"), "widget/Day");
    }
    add(&mut m, "Fill".into(), Kind::Flow, "Step 496 / 1000".into(), "widget/Day");
    m.vars.last_mut().unwrap().to = Some(2);
    m.next_id = 10_000;
    dynamist_core::ops::sync_links_for_tests(&mut m);
    assert!(m.vars.len() >= 500);
    let start = std::time::Instant::now();
    let r = doctor::check(&m, &Options { stage: Some(Stage::C3), ..Default::default() });
    let took = start.elapsed();
    assert!(r.count(Severity::Error) == 0, "{:?}", r.findings.iter().filter(|f| f.severity == Severity::Error).take(3).collect::<Vec<_>>());
    if !cfg!(debug_assertions) {
        assert!(took.as_millis() < 200, "stages C1 to C3 took {took:?}");
    }
}
