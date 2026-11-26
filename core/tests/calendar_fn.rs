//! Phase 3, sections 5 and 6: calendar and seasonality functions, logic functions and aliases.
use dynamist_core::model::{Kind, Model, Var};
use dynamist_core::sim;

fn run(unit: &str, start: Option<&str>, weekend: &str, dt: f64, final_t: f64, eqs: &[(&str, &str)]) -> sim::Run {
    let mut m = Model::default();
    m.settings.final_time = final_t;
    m.settings.time_step = dt;
    m.settings.time_units = unit.into();
    m.settings.start = start.map(|s| s.to_string());
    m.settings.weekend = weekend.into();
    for (i, (n, e)) in eqs.iter().enumerate() {
        let mut v = Var::new(i as u64 + 1, n, Kind::Aux, [0.0, 0.0]);
        v.equation = (*e).into();
        m.vars.push(v);
    }
    m.next_id = 50;
    let c = sim::compile(&m).unwrap_or_else(|e| panic!("{:?}", e.iter().map(|p| &p.message).collect::<Vec<_>>()));
    sim::simulate(&c, &m, "t").unwrap()
}

#[test]
fn sunday_start_weekdays() {
    let r = run("Day", Some("2026-11-22"), "sat-sun", 1.0, 7.0, &[("DOW", "DAY OF WEEK()"), ("WE", "IS WEEKEND()"), ("WD", "IS WORKDAY()"), ("D", "DATE()"), ("Y", "YEAR()"), ("WN", "WEEK NUMBER()")]);
    assert_eq!(r.get("DOW").unwrap()[0], 7.0);
    assert_eq!(r.get("WE").unwrap()[0], 1.0);
    assert_eq!(r.get("WE").unwrap()[5], 0.0); // Friday
    assert_eq!(r.get("D").unwrap()[0], 46_348.0);
    assert_eq!(r.get("Y").unwrap()[0], 2026.0);
    let r = run("Day", Some("2026-11-22"), "fri-sat", 1.0, 7.0, &[("WE", "IS WEEKEND()")]);
    assert_eq!(r.get("WE").unwrap()[0], 0.0);
    assert_eq!(r.get("WE").unwrap()[5], 1.0);
}

#[test]
fn profile_season_daylight_attime() {
    let r = run("Day", Some("2026-11-22"), "sat-sun", 0.5, 20.0, &[("P", "PROFILE STEP(7, 1,1,1,1,1,1.3,1.3)"), ("S", "SEASON(2, 5, 3, 10)"), ("A", "AT TIME(13)"), ("E", "EVERY(5)")]);
    let i = r.times.iter().position(|t| (*t - 5.5).abs() < 1e-9).unwrap();
    assert_eq!(r.get("P").unwrap()[i], 1.3);
    assert!(r.get("S").unwrap().iter().all(|x| *x >= 2.0 - 1e-12 && *x <= 5.0 + 1e-12));
    assert_eq!(r.get("A").unwrap().iter().sum::<f64>(), 1.0);
    assert_eq!(r.get("E").unwrap().iter().sum::<f64>(), 5.0); // 0, 5, 10, 15, 20
    let r = run("Hour", Some("2026-11-22"), "sat-sun", 1.0, 24.0, &[("L", "DAYLIGHT(6, 18)")]);
    let l = r.get("L").unwrap();
    assert_eq!((l[3], l[12]), (0.0, 1.0));
    assert!((l[8] - 0.5).abs() < 1e-12);
    // a profile interpolates between slot centres and wraps
    let r = run("Day", None, "sat-sun", 0.25, 4.0, &[("P", "PROFILE(4, 0, 4, 8, 4)")]);
    assert!(r.get("P").unwrap()[2].abs() < 1e-9 && (r.get("P").unwrap()[6] - 4.0).abs() < 1e-9);
}

#[test]
fn calendar_unit_must_suit() {
    let mut m = Model::default();
    m.settings.time_units = "Minute".into();
    m.settings.start = Some("2026-01-01".into());
    let mut v = Var::new(1, "X", Kind::Aux, [0.0, 0.0]);
    v.equation = "DAY OF WEEK()".into();
    m.vars = vec![v];
    let e = sim::compile(&m).err().unwrap();
    assert!(e[0].message.contains("DAY OF WEEK"), "{e:?}");
}

#[test]
fn logic_and_aliases() {
    let r = run("Day", None, "sat-sun", 1.0, 1.0, &[("A", "AND(1, 2, 3)"), ("B", "OR(0, 0)"), ("C", "NOT(0)"), ("D", "BETWEEN(5, 1, 5)"), ("E", "SAFEDIV(6, 3, 9)"), ("F", "SAFEDIV(6, 0)"), ("G", "MOD(7, 3)"), ("H", "INT(2.7)"), ("I", "CRF(0.08, 20)"), ("J", "EFFECT(0.12, 12)"), ("K", "SMTH1(5, 2)")]);
    let g = |n: &str| r.get(n).unwrap()[0];
    assert_eq!((g("A"), g("B"), g("C"), g("D")), (1.0, 0.0, 1.0, 1.0));
    assert_eq!((g("E"), g("F"), g("G"), g("H")), (2.0, 0.0, 1.0, 2.0));
    assert!((g("I") - 0.10185220882315058).abs() < 1e-12 && (g("J") - 0.12682503013196977).abs() < 1e-12);
}
