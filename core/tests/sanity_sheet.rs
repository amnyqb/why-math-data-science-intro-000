//! The sanity sheet of `dyn check --gate deliver`: each row, with a small model.

use dynamist_core::model::Model;
use dynamist_core::ops::{self, Op};
use dynamist_core::sanity::{self, Status};

fn model(manifest: &str) -> Model {
    let mut m = Model::default();
    m.settings.final_time = 20.0;
    m.settings.time_step = 0.25;
    m.settings.time_units = "Day".into();
    for j in [
        r#"{"op":"add_var","name":"Tank","kind":"stock","equation":"100","units":"widget"}"#,
        r#"{"op":"add_var","name":"Tau","kind":"constant","equation":"5","units":"Day"}"#,
        r#"{"op":"add_var","name":"Extra","kind":"constant","equation":"1","units":"widget/Day"}"#,
        r#"{"op":"add_var","name":"Drain","kind":"flow","equation":"Tank / Tau","units":"widget/Day","from":"Tank"}"#,
        r#"{"op":"add_var","name":"Supply","kind":"flow","equation":"Extra","units":"widget/Day","to":"Tank"}"#,
    ] {
        ops::apply(&mut m, &serde_json::from_str::<Op>(j).unwrap()).unwrap();
    }
    let op: Op = serde_json::from_str(&format!(r#"{{"op":"set_manifest","manifest":{manifest}}}"#)).unwrap();
    ops::apply(&mut m, &op).unwrap();
    m
}

fn status(rows: &[sanity::Row], id: &str) -> Status {
    rows.iter().find(|r| r.id == id).unwrap_or_else(|| panic!("row {id}")).status
}

const LEVERS: &str = r#""levers":[{"name":"Tau","min":2,"max":10},{"name":"Extra","min":0,"max":5}]"#;

#[test]
fn the_automated_rows_pass_on_a_sound_model_and_the_human_rows_are_open_until_recorded() {
    let m = model(&format!(r#"{{{LEVERS},"goals":[{{"name":"Stock","output":"Tank","measure":"at:10","direction":"max"}}],"scenarios":[{{"name":"Base","values":{{"Tau":5}}}},{{"name":"Fast","values":{{"Tau":3}}}}],"validity":[{{"statement":"short runs","variable":"Tank","min":0,"max":500,"horizon":15}}]}}"#));
    let rows = sanity::sheet(&m, 1);
    assert_eq!(rows.len(), 14);
    for id in ["S1", "S2", "S4", "S5", "S6", "S7"] {
        assert_eq!(status(&rows, id), Status::Pass, "{id}: {:?}", rows.iter().find(|r| r.id == id));
    }
    // a step of 0.25 against a time constant of 5 moves the stock a little, by far less than 1%?
    assert!(matches!(status(&rows, "S3"), Status::Pass | Status::Fail));
    for id in ["S8", "S9", "S10", "S11", "S12"] {
        assert_eq!(status(&rows, id), Status::Open);
    }
    // the conclusion (Base is best in every draw over the ranges) holds; no expected directions are declared
    assert_eq!(status(&rows, "S14"), Status::Pass, "{:?}", rows.iter().find(|r| r.id == "S14"));
    assert_eq!(status(&rows, "S13"), Status::Open, "{:?}", rows.iter().find(|r| r.id == "S13"));
    assert!(!sanity::delivered(&rows));
    let md = sanity::markdown(&m, &rows);
    assert!(md.contains("| S4 |") && md.contains("Not ready to deliver"));
}

#[test]
fn scenarios_that_differ_in_two_levers_or_goals_read_beyond_the_horizon_fail() {
    let m = model(&format!(r#"{{{LEVERS},"goals":[{{"name":"End","output":"Tank","measure":"final"}}],"scenarios":[{{"name":"Base","values":{{"Tau":5,"Extra":1}}}},{{"name":"Both","values":{{"Tau":3,"Extra":2}}}}],"validity":[{{"statement":"short","variable":"Tank","min":0,"max":120,"horizon":10}}]}}"#));
    let rows = sanity::sheet(&m, 1);
    assert_eq!(status(&rows, "S4"), Status::Fail, "{:?}", rows[3]);
    // the goal reads the end of a run of 20 days, the horizon is 10
    assert_eq!(status(&rows, "S5"), Status::Fail);
    // Tank is above 120 at some time before day 10 in "Both"? it falls from 100 towards 10 here, so it stays inside
    assert_eq!(status(&rows, "S6"), Status::Pass);
    // a range that the base run leaves before the horizon
    let m = model(&format!(r#"{{{LEVERS},"validity":[{{"statement":"narrow","variable":"Tank","min":90,"max":120,"horizon":10}}]}}"#));
    assert_eq!(status(&sanity::sheet(&m, 1), "S6"), Status::Fail);
}

#[test]
fn a_person_signs_the_human_rows_in_the_manifest() {
    let m = model(&format!(
        r#"{{{LEVERS},"sanity":[{{"kind":"expected-value","value":18.2,"note":"written on paper","by":"Ada","date":"2026-11-01"}},{{"kind":"reviewer","by":"Bo","date":"2026-11-20"}},{{"kind":"cross-check","note":"spreadsheet","by":"Ada"}},{{"kind":"benchmark","status":"fail","note":"15 percent off","by":"Bo","date":"2026-11-20"}}]}}"#
    ));
    let rows = sanity::sheet(&m, 1);
    assert_eq!(status(&rows, "S8"), Status::Pass);
    assert_eq!(status(&rows, "S12"), Status::Pass);
    assert_eq!(status(&rows, "S10"), Status::Open, "no date: not signed");
    assert_eq!(status(&rows, "S11"), Status::Fail);
    assert_eq!(status(&rows, "S9"), Status::Open);
}

#[test]
fn the_workbook_has_five_sheets_and_labels_what_lies_beyond_the_horizon() {
    use dynamist_core::workbook::{self, Cell};
    let m = model(&format!(r#"{{"title":"Tank","description":"How fast a tank empties",{LEVERS},"outputs":[{{"variable":"Tank"}}],"scenarios":[{{"name":"Base","values":{{"Tau":5}}}},{{"name":"Fast","values":{{"Tau":3}}}}],"validity":[{{"statement":"short runs","horizon":10}}]}}"#));
    let sheets = workbook::build(&m, "tank.sdm");
    assert_eq!(sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Cover", "Inputs", "Scenarios", "Series", "Checks"]);
    let text = |c: &Cell| if let Cell::Text(t) = c { t.clone() } else { String::new() };
    let head: Vec<String> = sheets[2].rows[0].iter().map(text).collect();
    assert!(head.iter().any(|h| h.contains("at time 10")) && head.iter().any(|h| h.contains("at time 20 (beyond the horizon)")), "{head:?}");
    // base case plus two scenarios; a faster drain leaves less in the tank at day 10
    assert_eq!(sheets[2].rows.len(), 4);
    let val = |r: usize| if let Cell::Num(x) = sheets[2].rows[r][3] { x } else { f64::NAN };
    assert!(val(3) < val(1), "{} {}", val(3), val(1));
    assert_eq!(sheets[1].rows.len(), 3);
    assert!(sheets[3].rows.len() > 50);
    assert!(sheets[0].rows.iter().any(|r| r.first().map(text).as_deref() == Some("Re-create the numbers")));
}

#[test]
fn expected_directions_are_tested_and_the_stakes_say_which_rows_are_required() {
    let mk = |expect: &str, stakes: &str| model(&format!(r#"{{{LEVERS},"stakes":"{stakes}","goals":[{{"name":"Stock","output":"Tank","measure":"at:10","direction":"max"}}],"scenarios":[{{"name":"Base","values":{{"Tau":5}}}},{{"name":"Fast","values":{{"Tau":3}},"expect":{{"Stock":"{expect}"}}}}]}}"#));
    // a faster drain leaves less in the tank: down holds, up does not
    assert_eq!(status(&sanity::sheet(&mk("down", ""), 1), "S13"), Status::Pass);
    let rows = sanity::sheet(&mk("up", ""), 1);
    assert_eq!(status(&rows, "S13"), Status::Fail);
    assert!(rows.iter().find(|r| r.id == "S13").unwrap().detail.contains("should move"));
    assert_eq!(status(&sanity::sheet(&mk("worse", ""), 1), "S13"), Status::Pass);
    // a workshop sketch needs the order of magnitude, units, conservation and the time step only
    let rows = sanity::sheet(&mk("down", "workshop"), 1);
    for id in ["S4", "S5", "S6", "S7", "S9", "S10", "S11", "S12", "S13", "S14"] {
        assert_eq!(status(&rows, id), Status::NotRequired, "{id}");
    }
    for id in ["S1", "S2", "S3"] {
        assert_ne!(status(&rows, id), Status::NotRequired, "{id}");
    }
    assert_eq!(status(&rows, "S8"), Status::Open);
    let md = sanity::markdown(&mk("down", "workshop"), &rows);
    assert!(md.contains("Stakes: workshop") && md.contains("not required"), "{md}");
    // a ranking that changes with the step fails the time step row: none here, so it passes with a note
    assert!(sanity::sheet(&mk("down", ""), 1).iter().find(|r| r.id == "S3").unwrap().detail.contains("ranking"));
}
