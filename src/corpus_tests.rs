//! Tests that need both the engine and the spreadsheet adapter: the fuel model as the corpus.

use crate::checks::{CheckWhen, Check};
use crate::model::{Kind, Model, Var};

fn fuel() -> Option<Model> {
    let dir = std::path::Path::new(crate::spreadsheet::WIKI_DIR).join("inbox").join("fuel-supply");
    if !dir.join("Checks.csv").exists() {
        return None;
    }
    Some(crate::spreadsheet::model_from_book(&crate::spreadsheet::read_book(&dir).unwrap()).unwrap().0)
}

fn run(m: &Model) -> crate::sim::Run {
    let c = crate::sim::compile(m).unwrap();
    crate::sim::simulate(&c, m, "x").unwrap()
}

#[test]
fn the_fuel_model_has_no_unit_warnings() {
    let Some(m) = fuel() else { return };
    let w = crate::units::warnings(&m);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn the_fuel_model_conserves_every_stock() {
    let Some(m) = fuel() else { return };
    let f = crate::checks::all(&m, &run(&m));
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn the_fuel_model_states_its_own_limits_and_they_hold() {
    let Some(m) = fuel() else { return };
    assert_eq!(m.checks.len(), 4);
    assert_eq!(m.checks[3].when, CheckWhen::End);
    let f = crate::checks::limits(&m, &run(&m));
    assert!(f.is_empty(), "{f:?}");
    let mut tight = m.clone();
    tight.checks[3].max = Some(100.0);
    assert_eq!(crate::checks::limits(&tight, &run(&tight)).len(), 1);
}

#[test]
fn checks_survive_a_trip_through_the_spreadsheet() {
    let mut m = Model::default();
    m.settings.final_time = 10.0;
    let mut s = Var::new(1, "Tank", Kind::Stock, [0.0, 0.0]);
    s.equation = "100".into();
    m.vars = vec![s];
    m.next_id = 10;
    m.checks = vec![
        Check { name: "Stays above 80".into(), var: "Tank".into(), min: Some(80.0), max: None, when: CheckWhen::Always, ..Default::default() },
        Check { name: String::new(), var: "Tank".into(), min: None, max: Some(1000.5), when: CheckWhen::End, ..Default::default() },
        Check { name: "Never dry".into(), var: "Tank".into(), kind: dynamist_core::checks::CheckKind::NonNeg, ..Default::default() },
        Check { name: "Flat".into(), var: "Tank".into(), kind: dynamist_core::checks::CheckKind::Reference, reference: vec![[0.0, 100.0]], tolerance: Some(0.05), relative: true, ..Default::default() },
        Check { name: "Follows".into(), var: "Tank".into(), kind: dynamist_core::checks::CheckKind::Reference, reference: vec![[0.0, 100.0], [10.0, 120.0]], tolerance: Some(2.5), ..Default::default() },
        Check { name: "Chain".into(), kind: dynamist_core::checks::CheckKind::Balance, stocks: vec!["Tank".into(), "Other".into()], inflows: vec!["In".into()], outflows: vec!["Out".into(), "Lost".into()], ..Default::default() },
    ];
    let book = crate::spreadsheet::book_from_model(&m);
    let (back, notes) = crate::spreadsheet::model_from_book(&book).unwrap();
    assert_eq!(back.checks, m.checks, "{notes:?}");
}

#[test]
fn the_wiki_operations_reference_is_the_generated_one() {
    let path = std::path::Path::new(crate::spreadsheet::WIKI_DIR).join("API-Reference.md");
    let Ok(on_disk) = std::fs::read_to_string(&path) else { return };
    assert_eq!(on_disk, dynamist_core::apidoc::markdown(), "run `dyn docs > wiki/API-Reference.md` to refresh it");
}
