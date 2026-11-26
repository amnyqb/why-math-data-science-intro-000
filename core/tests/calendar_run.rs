//! Phase 1, section 3: calendar time in a run.

use dynamist_core::model::{Kind, Model, TimeConvention, Var};
use dynamist_core::sim;

fn mk(id: u64, name: &str, kind: Kind, eq: &str, units: &str) -> Var {
    let mut v = Var::new(id, name, kind, [0.0, 0.0]);
    v.equation = eq.into();
    v.units = units.into();
    v
}

fn monthly(convention: TimeConvention) -> Model {
    let mut m = Model::default();
    m.settings.initial_time = 0.0;
    m.settings.final_time = 14.0;
    m.settings.time_step = 1.0;
    m.settings.time_units = "Month".into();
    m.settings.start = Some("2027-01-01".into());
    m.settings.time_convention = convention;
    let s = mk(1, "Stock", Kind::Stock, "0", "widget");
    let mut f = mk(2, "Rate", Kind::Flow, "10", "widget/Day");
    f.to = Some(1);
    m.vars = vec![s, f, mk(3, "Days", Kind::Aux, "DAYS IN STEP()", "Day"), mk(4, "Hours", Kind::Aux, "HOURS IN STEP()", "hour")];
    m.next_id = 20;
    m
}

#[test]
fn ten_per_day_over_january_and_february_2027_gives_310_and_280() {
    let m = monthly(TimeConvention::Calendar);
    let c = sim::compile(&m).unwrap();
    let r = sim::simulate(&c, &m, "x").unwrap();
    let s = r.get("Stock").unwrap();
    assert_eq!(s[1] - s[0], 310.0);
    assert_eq!(s[2] - s[1], 280.0);
    // February 2028 has 29 days: step 13
    assert_eq!(r.get("Days").unwrap()[13], 29.0);
    assert_eq!(r.get("Hours").unwrap()[13], 29.0 * 24.0);
    assert_eq!(r.get("Days").unwrap()[0], 31.0);
}

#[test]
fn year_fraction_in_a_run() {
    let mut m = monthly(TimeConvention::Calendar);
    m.vars.push(mk(5, "Frac", Kind::Aux, "YEAR FRACTION(0, 6, 3)", "dmnl"));
    let c = sim::compile(&m).unwrap();
    let r = sim::simulate(&c, &m, "x").unwrap();
    // 2027-01-01 to 2027-07-01 is 181 days, actual/365
    assert!((r.get("Frac").unwrap()[0] - 181.0 / 365.0).abs() < 1e-12);
}

#[test]
fn calendar_functions_need_a_start_date() {
    let mut m = monthly(TimeConvention::Abstract);
    m.settings.start = None;
    let e = sim::compile(&m).err().expect("must refuse");
    assert!(e.iter().any(|p| p.message.contains("start date")), "{e:?}");
}
