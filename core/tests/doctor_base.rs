//! Phase 1, section 2: the base checks of the doctor, each against a deliberately broken model.

use dynamist_core::doctor::{self, Options};
use dynamist_core::model::{Kind, Model, Var};

fn mk(id: u64, name: &str, kind: Kind, eq: &str, units: &str) -> Var {
    let mut v = Var::new(id, name, kind, [0.0, 0.0]);
    v.equation = eq.into();
    v.units = units.into();
    v
}

fn orders_model() -> Model {
    let mut m = Model::default();
    m.settings.final_time = 10.0;
    m.settings.time_step = 0.25;
    m.settings.time_units = "Day".into();
    m.vars = vec![
        mk(1, "Orders", Kind::Constant, "100", "widget/Day"),
        mk(2, "Shipments", Kind::Aux, "Orders * 0.3", "widget/Day"),
        mk(3, "Stock", Kind::Stock, "0", "widget"),
        { let mut f = mk(4, "Inflow", Kind::Flow, "Shipments", "widget/Day"); f.to = Some(3); f },
    ];
    m.next_id = 20;
    dynamist_core::ops::sync_links_for_tests(&mut m);
    m
}

#[test]
fn magic_number_flags_a_literal_and_proposes_a_named_constant() {
    let m = orders_model();
    let r = doctor::check(&m, &Options::default());
    let f = r.by_code("magic-number");
    assert_eq!(f.len(), 1, "{:#?}", r.findings);
    assert_eq!(f[0].var.as_deref(), Some("Shipments"));
    let fix = &f[0].fixes[0];
    assert!(fix.keeps_numbers);
    assert!(format!("{:?}", fix.ops).contains("Orders fraction"), "{fix:#?}");
    // applying it removes the finding and keeps the numbers
    let mut m2 = m.clone();
    let before = {
        let c = dynamist_core::sim::compile(&m2).unwrap();
        dynamist_core::sim::simulate(&c, &m2, "a").unwrap().get("Stock").unwrap().clone()
    };
    doctor::apply_fix(&mut m2, fix).unwrap();
    assert_eq!(m2.by_name("shipments").unwrap().equation, "Orders * Orders fraction");
    let after = {
        let c = dynamist_core::sim::compile(&m2).unwrap();
        dynamist_core::sim::simulate(&c, &m2, "b").unwrap().get("Stock").unwrap().clone()
    };
    assert_eq!(before, after);
    assert!(doctor::check(&m2, &Options::default()).by_code("magic-number").is_empty());
}

#[test]
fn dt_too_large_flags_a_smooth_with_a_time_of_one_day_at_dt_one_day() {
    let mut m = Model::default();
    m.settings.final_time = 20.0;
    m.settings.time_step = 1.0;
    m.settings.time_units = "Day".into();
    m.vars = vec![
        mk(1, "Input", Kind::Constant, "10", "widget"),
        mk(2, "Perceived", Kind::Aux, "SMOOTH(Input, 1)", "widget"),
    ];
    m.next_id = 20;
    dynamist_core::ops::sync_links_for_tests(&mut m);
    let r = doctor::check(&m, &Options::default());
    let f = r.by_code("dt-too-large");
    assert_eq!(f.len(), 1, "{:#?}", r.findings);
    assert!(f[0].message.contains("time step"));
    assert!(format!("{:?}", f[0].fixes[0].ops).contains("0.25"), "{:?}", f[0].fixes);
    // at DT 0.25 it is fine
    m.settings.time_step = 0.25;
    assert!(doctor::check(&m, &Options::default()).by_code("dt-too-large").is_empty());
}

#[test]
fn time_equality_and_zero_division_and_ratio_outside() {
    let mut m = Model::default();
    m.settings.final_time = 10.0;
    m.settings.time_step = 0.3;
    m.settings.time_units = "Day".into();
    m.vars = vec![
        mk(1, "Pulse", Kind::Aux, "IF THEN ELSE(Time = 3, 1, 0)", "dmnl"),
        mk(2, "Count", Kind::Aux, "Time - 5", "dmnl"),
        mk(3, "Ratio of things", Kind::Aux, "10 / Count", "dmnl"),
    ];
    m.next_id = 20;
    dynamist_core::ops::sync_links_for_tests(&mut m);
    let r = doctor::check(&m, &Options::default());
    assert_eq!(r.by_code("time-equality").len(), 1, "{:#?}", r.findings);
    m.settings.time_step = 0.25;
    let r = doctor::check(&m, &Options::default());
    assert!(r.by_code("time-equality").is_empty(), "a binary time step is fine");
    let z = r.by_code("zero-division");
    assert_eq!(z.len(), 1, "{:#?}", r.findings);
    assert!(!r.by_code("ratio-outside").is_empty(), "{:#?}", r.findings);
}

#[test]
fn units_are_inferred_for_a_variable_with_none() {
    let mut m = orders_model();
    m.vars.push(mk(5, "Twice", Kind::Aux, "Orders * Orders", ""));
    let r = doctor::check(&m, &Options::default());
    let f = r.by_code("units-inferred");
    assert_eq!(f.len(), 1, "{:#?}", r.findings);
    assert!(format!("{:?}", f[0].fixes[0].ops).contains("widget/Day"));
}
