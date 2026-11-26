//! `dyn check` (the doctor) on every model of the book source runs without crashing. The report is saved when the
//! environment variable `DYN_HEALTH_OUT` names a file (the build step writes `build/model_health.json`).

use dynamist_core::doctor::{self, Options};
use dynamist_core::io;

mod par;

fn models() -> Vec<(String, dynamist_core::model::Model)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book_models");
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
    files.sort();
    let mut out = Vec::new();
    for p in files {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let Ok(text) = io::read_text(&p) else { continue };
        let Some(fmt) = io::detect(&name, &text) else { continue };
        if let Ok((m, _)) = io::read_str(&text, fmt) {
            out.push((name, m));
        }
    }
    out
}

#[test]
fn the_doctor_runs_on_every_book_model() {
    let ms = models();
    assert!(ms.len() > 150, "only {} models read", ms.len());
    // the largest models first, so that the slowest one does not start last
    let mut order: Vec<usize> = (0..ms.len()).collect();
    order.sort_by_key(|i| std::cmp::Reverse(ms[*i].1.vars.len()));
    let mut report = par_map_ordered(&ms, &order);
    if let Ok(out) = std::env::var("DYN_HEALTH_OUT") {
        if let Some(parent) = std::path::Path::new(&out).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
}

fn par_map_ordered(ms: &[(String, dynamist_core::model::Model)], order: &[usize]) -> Vec<serde_json::Value> {
    let items: Vec<&(String, dynamist_core::model::Model)> = order.iter().map(|i| &ms[*i]).collect();
    let mut rows = par::par_map(&items, |(name, m)| {
        // the fast stages alone: they run on every edit in the app
        let t3 = std::time::Instant::now();
        let _ = doctor::check(m, &Options { stage: Some(doctor::Stage::C3), ..Default::default() });
        let c3_ms = t3.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        let r = doctor::check(m, &Options::default());
        if std::env::var("DYN_TIMING").is_ok() {
            eprintln!("{:>8.0} ms  {name}", t.elapsed().as_secs_f64() * 1000.0);
        }
        serde_json::json!({
            "model": name,
            "c1_to_c3_ms": c3_ms,
            "errors": r.count(doctor::Severity::Error),
            "warnings": r.count(doctor::Severity::Warning),
            "info": r.count(doctor::Severity::Info),
            "findings": r.findings.iter().map(|f| serde_json::json!({"code": f.code, "severity": f.severity, "var": f.var, "message": f.message})).collect::<Vec<_>>(),
        })
    });
    rows.sort_by(|a, b| a["model"].as_str().cmp(&b["model"].as_str()));
    let mut by: Vec<(f64, String)> = rows.iter().map(|r| (r["c1_to_c3_ms"].as_f64().unwrap(), r["model"].as_str().unwrap().to_string())).collect();
    by.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!("slowest C1 to C3: {:?}", &by[..3.min(by.len())]);
    let worst = by[0].0;
    if !cfg!(debug_assertions) {
        assert!(worst < 500.0, "stages C1 to C3 took {worst:.0} ms on the slowest book model");
    }
    rows
}
