//! Phase 1, section 0: the units trap model must be caught, with exact fixes.

use dynamist_core::doctor::{self, Options, Severity};
use dynamist_core::{mdl, sim};

fn trap() -> dynamist_core::model::Model {
    let mut m = mdl::import(include_str!("data/units_trap.mdl")).unwrap();
    // the model's own comment says what MB means ("thousand barrels"), and a new-style model reads %-units as values
    m.settings.unit_choices.insert("MB".into(), "thousand barrels".into());
    m.settings.units_scale_values = true;
    m
}

fn unit_findings(m: &dynamist_core::model::Model) -> Vec<doctor::Finding> {
    let r = doctor::check(m, &Options { units_only: true, ..Default::default() });
    r.findings
}

#[test]
fn units_trap_is_caught() {
    let m = trap();
    let f = unit_findings(&m);
    assert_eq!(f.len(), 3, "{f:#?}");
    // 1. a scale mismatch between Delivery (barrel/Day) and Tank (MB)
    let scale = f.iter().find(|x| x.code == "unit-scale-mismatch").expect("scale mismatch");
    assert_eq!(scale.var.as_deref(), Some("Delivery"));
    assert_eq!(scale.severity, Severity::Error);
    assert!(scale.message.contains("barrel/Day") && scale.message.contains("Tank"), "{}", scale.message);
    let first = &scale.fixes[0];
    assert!(!first.keeps_numbers);
    assert!(format!("{:?}", first.ops).contains("MB per barrel") && format!("{:?}", first.ops).contains("0.001"), "{first:#?}");
    // 2. a percent unit on Use Fraction: informational, confirms the reading 0.05/Day
    let pct = f.iter().find(|x| x.code == "unit-percent").expect("percent");
    assert_eq!(pct.severity, Severity::Info);
    assert_eq!(pct.var.as_deref(), Some("Use Fraction"));
    assert!(pct.message.contains("0.05"), "{}", pct.message);
    // 3. Power (MW) into Energy (kWh) with time unit Day: factor 24,000
    let mw = f.iter().find(|x| x.code == "unit-scale-mismatch" && x.var.as_deref() == Some("Power")).expect("power");
    assert!(mw.message.contains("24000"), "{}", mw.message);
    assert!(format!("{:?}", mw.fixes[0].ops).contains("24000"), "{:#?}", mw.fixes[0]);
}

#[test]
fn the_first_ranked_fixes_make_the_trap_pass_the_strict_check() {
    let mut m = trap();
    for f in unit_findings(&m).iter().filter(|f| f.code == "unit-scale-mismatch") {
        doctor::apply_fix(&mut m, &f.fixes[0]).unwrap();
    }
    let after = unit_findings(&m);
    assert!(after.iter().all(|f| f.severity != Severity::Error), "{after:#?}");
    // and the tank stays above zero for the ten days
    let c = sim::compile(&m).unwrap();
    let run = sim::simulate(&c, &m, "x").unwrap();
    assert!(run.get("Tank").unwrap().iter().all(|x| *x > 0.0), "{:?}", run.get("Tank"));
    assert!((run.get("Energy").unwrap().last().unwrap() - 100.0 * 24_000.0 * 10.0).abs() < 1e-6);
}

#[test]
fn an_unchosen_ambiguous_symbol_is_asked_about() {
    let mut m = trap();
    m.settings.unit_choices.clear();
    let r = doctor::check(&m, &Options { units_only: true, ..Default::default() });
    let a = r.by_code("unit-ambiguous");
    assert_eq!(a.len(), 1, "{a:#?}");
    assert_eq!(a[0].fixes.len(), 2, "one fix per meaning");
}

#[test]
fn with_scale_values_off_the_doctor_lists_what_would_change() {
    let mut m = trap();
    m.settings.units_scale_values = false;
    let r = doctor::check(&m, &Options { units_only: true, ..Default::default() });
    let c = r.by_code("unit-scale-change");
    assert_eq!(c.len(), 1, "{:#?}", r.findings);
    assert!(c[0].message.contains("5") && c[0].message.contains("0.05"));
}
