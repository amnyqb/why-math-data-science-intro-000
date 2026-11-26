//! Layered modelling, Phase 1 acceptance tests (tests 1 to 4 and 8 of the specification), driven
//! through the `dyn` binary on a four-layer start-up model: World, Capital, Firm, Operations.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fc(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dyn")).args(args).output().unwrap()
}

fn json(o: &Output) -> serde_json::Value {
    serde_json::from_slice(&o.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(&o.stdout)))
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("dyn-layers-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// (name, kind, equation, from, to, layer)
const VARS: &[(&str, &str, &str, &str, &str, &str)] = &[
    ("Capital Climate", "aux", "0.6 + 0.005 * TIME", "", "", "World"),
    ("Capital Pool", "stock", "100", "", "", "Capital"),
    ("Investment", "flow", "Capital Climate * Traction * 5", "", "Capital Pool", "Capital"),
    ("Funds Available", "aux", "Capital Pool * 0.02", "", "", "Capital"),
    ("Cash", "stock", "50", "", "", "Firm"),
    ("Funding", "flow", "Funds Available", "", "Cash", "Firm"),
    ("Revenue", "flow", "Units Sold * Price", "", "Cash", "Firm"),
    ("Spend", "flow", "Fixed Cost", "Cash", "", "Firm"),
    ("Price", "constant", "2", "", "", "Firm"),
    ("Fixed Cost", "constant", "10", "", "", "Firm"),
    ("Traction", "aux", "Cash / (Cash + 100)", "", "", "Firm"),
    ("Capacity", "stock", "10", "", "", "Operations"),
    ("Capacity Growth", "flow", "Cash * 0.001", "", "Capacity", "Operations"),
    ("Capacity Decay", "flow", "Capacity * 0.05", "Capacity", "", "Operations"),
    ("Units Sold", "aux", "Capacity * 0.8", "", "", "Operations"),
];

const LAYERS: &[(&str, i32, f64)] = &[("World", 0, 60.0), ("Capital", 1, 60.0), ("Firm", 2, 60.0), ("Operations", 3, 60.0)];

fn batch(layered: bool) -> String {
    let mut ops: Vec<serde_json::Value> = vec![
        serde_json::json!({"op": "set_setting", "key": "final_time", "value": "60"}),
        serde_json::json!({"op": "set_setting", "key": "time_step", "value": "1"}),
        serde_json::json!({"op": "set_setting", "key": "time_units", "value": "Month"}),
    ];
    if layered {
        for (n, o, h) in LAYERS {
            ops.push(serde_json::json!({"op": "add_layer", "name": n, "order": o, "horizon": h, "status": "built"}));
        }
    }
    // stocks first so flows can name them
    for pass in ["stock", "other"] {
        for (n, k, e, from, to, layer) in VARS {
            if (*k == "stock") != (pass == "stock") {
                continue;
            }
            let mut o = serde_json::json!({"op": "add_var", "name": n, "kind": k, "equation": e});
            if !from.is_empty() {
                o["from"] = (*from).into();
            }
            if !to.is_empty() {
                o["to"] = (*to).into();
            }
            if layered {
                o["layer"] = (*layer).into();
            }
            ops.push(o);
        }
    }
    serde_json::to_string(&ops).unwrap()
}

fn build(dir: &PathBuf, name: &str, layered: bool) -> String {
    let (file, ops) = (dir.join(format!("{name}.sdm")), dir.join(format!("{name}.json")));
    std::fs::write(&ops, batch(layered)).unwrap();
    let o = fc(&["ops", file.to_str().unwrap(), ops.to_str().unwrap(), "--no-gate"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stdout));
    file.to_str().unwrap().to_string()
}

fn run_series(file: &str) -> String {
    let o = fc(&["run", file, "--every", "1"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stdout));
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn test1_four_layers_check_clean_and_results_do_not_change() {
    let d = scratch("t1");
    let plain = build(&d, "plain", false);
    let layered = build(&d, "layered", true);
    let before = run_series(&plain);
    let o = fc(&["layer", "freeze", &layered]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stdout));
    let c = fc(&["layer", "check", &layered]);
    let v = json(&c);
    assert!(c.status.success(), "{v}");
    assert_eq!(v["errors"], 0);
    assert!(v["contracts"].as_u64().unwrap() >= 5, "{v}");
    assert_eq!(run_series(&layered), before, "results must be bit-identical");
    // dyn layers lists the four layers with their interfaces
    let l = json(&fc(&["layers", &layered]));
    assert_eq!(l["layers"].as_array().unwrap().len(), 4);
    assert_eq!(l["unassigned"], 0);
    assert!(!l["cross_layer_loops"].as_array().unwrap().is_empty(), "Capital, Firm and Operations feed each other");
}

#[test]
fn test2_firm_alone_on_recorded_inputs_matches_its_embedded_run() {
    let d = scratch("t2");
    let f = build(&d, "m", true);
    fc(&["layer", "freeze", &f]);
    let o = fc(&["layer", "decouple", &f, "Firm", "--max-nrmse", "0.01"]);
    let v = json(&o);
    assert!(o.status.success(), "{v}");
    assert!(v["plumbing_nrmse"].as_f64().unwrap() < 0.01, "{v}");
    // the Firm reacts back on its neighbours, so its own constants do move the other layers
    assert_eq!(v["decoupled"], false, "{v}");
    // a layer run alone on its recorded inputs, to a CSV
    let csv = d.join("firm.csv");
    let r = fc(&["layer", "run", &f, "Firm", "--from-full-run", "--csv", csv.to_str().unwrap()]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stdout));
    assert!(std::fs::read_to_string(csv).unwrap().starts_with("time,"));
}

#[test]
fn test3_an_undeclared_crossing_is_an_error_naming_both_ends() {
    let d = scratch("t3");
    let f = build(&d, "m", true);
    fc(&["layer", "freeze", &f]);
    // Capital now reads Capacity, which lives in Operations
    let batch = d.join("cross.json");
    std::fs::write(&batch, r#"[{"op":"set_equation","name":"Funds Available","equation":"Capital Pool * 0.02 + Capacity * 0"}]"#).unwrap();
    // the gate refuses it
    let refused = fc(&["ops", &f, batch.to_str().unwrap()]);
    assert!(!refused.status.success());
    let text = String::from_utf8_lossy(&refused.stdout).to_string();
    assert!(text.contains("Capacity") && text.contains("Operations") && text.contains("Capital"), "{text}");
    // without the gate it is saved, and `layer check` exits 1 naming both ends
    assert!(fc(&["ops", &f, batch.to_str().unwrap(), "--no-gate"]).status.success());
    let c = fc(&["layer", "check", &f]);
    assert_eq!(c.status.code(), Some(1));
    let t = String::from_utf8_lossy(&c.stdout).to_string();
    assert!(t.contains("layer-crossing") && t.contains("Capacity") && t.contains("Operations") && t.contains("Capital"), "{t}");
}

#[test]
fn test4_horizon_is_shaded_on_charts_and_goals_beyond_it_are_refused() {
    let d = scratch("t4");
    let f = build(&d, "m", true);
    fc(&["layer", "freeze", &f]);
    assert!(fc(&["layer", "set", &f, "Capital", "--horizon", "36"]).status.success());
    let r = json(&fc(&["run", &f]));
    let b = r["beyond_horizon"].as_array().unwrap();
    assert!(b.iter().any(|x| x["layer"] == "Capital" && x["trusted_until"] == 36.0), "{r}");
    // the chart shades months 36 to 60 and says "ranges only"
    let svg = d.join("c.svg");
    let c = fc(&["chart", &f, "--vars", "Capital Pool", "--out", svg.to_str().unwrap()]);
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stdout));
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.contains("ranges only"), "the chart must label the region");
    // a goal on a Capital variable at month 60 is beyond the horizon
    let refused = fc(&["optimize", &f, "--goal", "Capital Pool", "--lever", "Price:1:3"]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stdout).contains("horizon"));
    let ok = fc(&["optimize", &f, "--goal", "Capital Pool", "--lever", "Price:1:3", "--allow-beyond-horizon"]);
    assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stdout));
    // a goal in a layer whose horizon covers the run is fine
    let fine = fc(&["optimize", &f, "--goal", "Cash", "--lever", "Price:1:3"]);
    assert!(fine.status.success(), "{}", String::from_utf8_lossy(&fine.stdout));
}

#[test]
fn test8_a_model_without_layers_is_written_as_before() {
    let d = scratch("t8");
    let plain = build(&d, "plain", false);
    let text = std::fs::read_to_string(&plain).unwrap();
    for key in ["\"layers\"", "\"interfaces\"", "\"stubs\"", "\"layer\""] {
        assert!(!text.contains(key), "{key} must not appear in a model without layers");
    }
    // layer commands on it say nothing is layered and change nothing
    let l = json(&fc(&["layers", &plain]));
    assert_eq!(l["layers"].as_array().unwrap().len(), 0);
    assert!(fc(&["layer", "check", &plain]).status.success());
    // and a layerless model is validated as before
    assert!(fc(&["validate", &plain]).status.success());
}
