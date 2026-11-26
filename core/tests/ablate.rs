//! `dyn ablate`: defaults and named ablations, the table and the flag.

use dynamist_core::ablate;
use dynamist_core::model::Model;
use dynamist_core::ops::{self, Op};

fn model(direction: &str, ablations: &str) -> Model {
    let mut m = Model::default();
    m.settings.final_time = 20.0;
    m.settings.time_step = 0.25;
    m.settings.time_units = "Day".into();
    for j in [
        r#"{"op":"add_var","name":"Tank","kind":"stock","equation":"100","units":"widget"}"#,
        r#"{"op":"add_var","name":"Tau","kind":"constant","equation":"5","units":"Day"}"#,
        r#"{"op":"add_var","name":"Pressure","kind":"aux","equation":"Tank / 100","units":"dmnl"}"#,
        r#"{"op":"add_var","name":"Drain","kind":"flow","equation":"Tank * Pressure / Tau","units":"widget/Day","from":"Tank"}"#,
        r#"{"op":"add_var","name":"Waste","kind":"stock","equation":"0","units":"widget"}"#,
        r#"{"op":"add_var","name":"Trash","kind":"flow","equation":"Drain","units":"widget/Day","to":"Waste"}"#,
    ] {
        ops::apply(&mut m, &serde_json::from_str::<Op>(j).unwrap()).unwrap();
    }
    let mf = format!(r#"{{"goals":[{{"name":"Left","output":"Tank","measure":"final","direction":"{direction}"}},{{"name":"Waste","output":"Waste","measure":"final","direction":"min"}}]{ablations}}}"#);
    let op: Op = serde_json::from_str(&format!(r#"{{"op":"set_manifest","manifest":{mf}}}"#)).unwrap();
    ops::apply(&mut m, &op).unwrap();
    m
}

#[test]
fn the_defaults_cut_each_loop_and_freeze_each_stock_and_flag_what_helps_a_goal() {
    let m = model("max", "");
    let t = ablate::run(&m, None).unwrap();
    let names: Vec<&str> = t.results.iter().map(|r| r.ablation.as_str()).collect();
    assert!(names.iter().any(|n| n.starts_with("Loop ")), "{names:?}");
    assert!(names.contains(&"Freeze Tank") && names.contains(&"Freeze Waste"), "{names:?}");
    assert_eq!(t.goals, vec!["Left", "Waste"]);
    // freezing the Tank keeps 100 in it: more than the run leaves, and the goal wants more
    let f = t.results.iter().find(|r| r.ablation == "Freeze Tank").unwrap();
    assert!(f.error.is_none(), "{:?}", f.error);
    assert!(f.cells[0].improves && f.flag == "check the goal", "{f:?}");
    let md = ablate::markdown(&t);
    assert!(md.contains("check the goal") && md.contains("| Freeze Tank |"), "{md}");
    // the same removal does not improve a goal that wants less
    let t = ablate::run(&model("min", ""), None).unwrap();
    let f = t.results.iter().find(|r| r.ablation == "Freeze Tank").unwrap();
    assert!(!f.cells[0].improves);
}

#[test]
fn a_named_ablation_cuts_a_link_and_a_bad_name_is_reported_not_fatal() {
    let m = model("min", r#","ablations":[{"name":"No pressure","cut":[["Pressure","Drain"]]},{"name":"Nothing there","freeze":["Nope"]}]"#);
    let t = ablate::run(&m, None).unwrap();
    assert_eq!(t.results.len(), 2);
    assert!(t.results[0].error.is_none() && t.results[0].what.contains("Pressure to Drain"), "{:?}", t.results[0]);
    // Pressure is held at its starting value 1: the drain no longer slows as the tank empties
    assert!(t.results[0].cells[0].change.unwrap() < 0.0);
    assert!(t.results[1].error.as_deref().is_some_and(|e| e.contains("Nope")));
    assert_eq!(ablate::run(&m, Some("No pressure")).unwrap().results.len(), 1);
    assert!(ablate::run(&m, Some("zzz")).is_err());
}
