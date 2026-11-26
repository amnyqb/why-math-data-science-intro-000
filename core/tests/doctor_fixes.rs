//! Fixes to the doctor after the book-side review: one small broken model per fix.

use dynamist_core::doctor::{self, Options, Severity, Stage};
use dynamist_core::model::{Kind, Model, Var};

fn var(id: u64, name: &str, kind: Kind, eq: &str, units: &str) -> Var {
    let mut v = Var::new(id, name, kind, [0.0, 0.0]);
    v.equation = eq.into();
    v.units = units.into();
    v
}

fn model(time_units: &str, vars: Vec<Var>) -> Model {
    let mut m = Model::default();
    m.settings.time_units = time_units.into();
    m.settings.final_time = 10.0;
    m.settings.units_scale_values = true;
    m.next_id = 100;
    m.vars = vars;
    dynamist_core::ops::sync_links_for_tests(&mut m);
    m
}

fn units_only() -> Options {
    Options { units_only: true, ..Default::default() }
}

fn run_last(m: &Model, name: &str) -> f64 {
    let c = dynamist_core::sim::compile(m).unwrap();
    *dynamist_core::sim::simulate(&c, m, "t").unwrap().get(name).unwrap().last().unwrap()
}

#[test]
fn an_exponent_with_units_is_reported_and_divided_by_one_unit_of_itself() {
    let m = model("Year", vec![var(1, "Yield", Kind::Constant, "100", "kWh"), var(2, "Degradation", Kind::Constant, "0.01", "dmnl"), var(3, "Output", Kind::Aux, "Yield * (1 - Degradation)^Time", "kWh")]);
    let r = doctor::check(&m, &units_only());
    let f = r.findings.iter().find(|f| f.rule == "U1" && f.var.as_deref() == Some("Output")).expect("an exponent finding");
    assert!(f.message.contains("an exponent must be a number without units"), "{}", f.message);
    let fix = &f.fixes[0];
    assert!(fix.keeps_numbers, "{fix:?}");
    let mut fixed = m.clone();
    doctor::apply_fix(&mut fixed, fix).unwrap();
    assert_eq!(fixed.by_name("output").unwrap().equation, "Yield * (1 - Degradation)^(Time / Year Length)");
    let yl = fixed.by_name("year length").unwrap();
    assert_eq!((yl.equation.as_str(), yl.units.as_str()), ("1", "Year"));
    assert!(doctor::check(&fixed, &units_only()).findings.iter().all(|f| f.rule != "U1"), "{:?}", doctor::check(&fixed, &units_only()).findings);
    assert_eq!(run_last(&m, "Output"), run_last(&fixed, "Output"));
}

#[test]
fn a_dimensionless_exponent_is_fine() {
    let m = model("Year", vec![var(1, "Yield", Kind::Constant, "100", "kWh"), var(2, "Degradation", Kind::Constant, "0.01", "dmnl"), var(3, "Age", Kind::Constant, "3", "dmnl"), var(4, "Output", Kind::Aux, "Yield * (1 - Degradation)^Age", "kWh")]);
    assert!(doctor::check(&m, &units_only()).findings.is_empty());
}

#[test]
fn trend_is_per_time_not_the_unit_of_its_input() {
    let m = model(
        "Week",
        vec![var(1, "Demand", Kind::Constant, "100", "cases/Week"), var(2, "Averaging Time", Kind::Constant, "4", "Week"), var(3, "Perceived Trend", Kind::Aux, "TREND(Demand, Averaging Time, 0)", "1/Week")],
    );
    let r = doctor::check(&m, &units_only());
    assert!(r.findings.iter().all(|f| f.severity == Severity::Info), "{:?}", r.findings);
    let mut wrong = m.clone();
    wrong.vars[2].units = "cases/Week".into();
    assert!(doctor::check(&wrong, &units_only()).findings.iter().any(|f| f.var.as_deref() == Some("Perceived Trend") && f.severity == Severity::Error));
    let _ = Stage::C3;
}

#[test]
fn irr_of_the_ch11_plant_is_found_per_month() {
    // the book's 1.284 percent a month (a continuous rate): the cash flow of the plant, 120 months at a step of 1/16 month
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book_models/ch11__ch11-plant.mdl")).unwrap();
    let m = dynamist_core::mdl::import(&text).unwrap();
    let c = dynamist_core::sim::compile(&m).unwrap();
    let run = dynamist_core::sim::simulate(&c, &m, "t").unwrap();
    let dt = m.settings.time_step;
    let flows: Vec<f64> = run.get("Cash Flow").unwrap().iter().map(|x| x * dt).collect();
    let r = dynamist_core::finance::stream_irr(&flows, dt, 1.0).expect("an IRR");
    // 1.2924% a month compounded monthly is the book's 1.284% a month as a continuous rate: ln(1 + r)
    assert!((r - 0.012924).abs() < 2e-6 && ((1.0 + r).ln() - 0.01284).abs() < 1e-5, "{r}");
    // the NPV crosses zero there, and the rate does not depend on the timing convention
    assert!(dynamist_core::finance::stream_pv(&flows, r, dt, 1.0).abs() < 1e-3 * flows.iter().map(|x| x.abs()).sum::<f64>());
    assert!((dynamist_core::finance::stream_irr(&flows, dt, 0.0).unwrap() - r).abs() < 1e-9);
}

#[test]
fn a_start_value_that_reads_a_flow_is_information_and_a_stock_reading_itself_an_error() {
    let mut m = model("Year", vec![var(1, "Accounts", Kind::Stock, "Acquisitions / Churn Fraction", "accounts"), var(2, "Acquisitions", Kind::Flow, "100", "accounts/Year"), var(3, "Churn Fraction", Kind::Constant, "0.1", "1/Year")]);
    m.vars[1].to = Some(1);
    let r = doctor::check(&m, &Options { stage: Some(Stage::C2), ..Default::default() });
    let g2: Vec<_> = r.findings.iter().filter(|f| f.rule == "G2").collect();
    assert_eq!(g2.len(), 1, "{:?}", r.findings);
    assert_eq!(g2[0].severity, Severity::Info);
    assert!(g2[0].fixes.is_empty());
    m.vars[0].equation = "Accounts + 1".into();
    let r = doctor::check(&m, &Options { stage: Some(Stage::C2), ..Default::default() });
    assert!(r.findings.iter().any(|f| f.rule == "G2" && f.severity == Severity::Error));
}

fn dt_model(input: &str) -> Model {
    let mut m = model(
        "Day",
        vec![var(1, "Level", Kind::Stock, "0", "widget"), var(2, "Feed", Kind::Flow, "Fast", "widget/Day"), var(3, "Fast", Kind::Aux, &format!("SMOOTH({input}, 0.1)"), "widget/Day")],
    );
    m.vars[1].to = Some(1);
    m.settings.time_step = 0.5;
    m.settings.final_time = 10.0;
    m
}

fn dt_findings(m: &Model) -> Vec<doctor::Finding> {
    doctor::check(m, &Options { stage: Some(Stage::C4), ..Default::default() }).findings.into_iter().filter(|f| matches!(f.rule.as_str(), "N2" | "N3" | "N5")).collect()
}

#[test]
fn a_long_step_is_reported_only_when_the_reruns_move_a_stock_by_one_percent_of_its_range() {
    // a smooth that starts at its input does not move: the step is longer than its time constant but nothing depends on it
    assert!(dt_findings(&dt_model("10")).is_empty(), "{:?}", dt_findings(&dt_model("10")));
    // a step in the input does: the smooth reacts within 0.1 days, the stock feels how it is integrated
    let f = dt_findings(&dt_model("STEP(10, 1)"));
    assert!(f.iter().any(|x| x.rule == "N5" && x.confirmed && x.message.contains("of its range")), "{f:?}");
}

fn x_model() -> Model {
    let mut m = model("Day", vec![var(1, "Cash", Kind::Stock, "10", "dollar"), var(2, "Rate", Kind::Constant, "0.5", "1/Day"), var(3, "Spend", Kind::Flow, "Cash * Rate", "dollar/Day")]);
    m.vars[2].from = Some(1);
    m.settings.final_time = 10.0;
    m
}

fn x_findings(m: &Model, rule: &str) -> Vec<doctor::Finding> {
    doctor::check(m, &Options { experiments: true, jobs: 2, stage: Some(Stage::C7), ..Default::default() }).findings.into_iter().filter(|f| f.rule == rule).collect()
}

#[test]
fn x1_lets_cash_go_negative_but_not_a_stock_declared_never_negative_or_a_fraction() {
    let mut m = x_model();
    // Cash may go negative: ten times the rate overshoots it
    assert!(x_findings(&m, "X1").iter().all(|f| f.severity == Severity::Info), "{:?}", x_findings(&m, "X1"));
    // a fraction that leaves 0 to 1 is reported, one with units of its own is not
    m.vars.push(var(4, "Use Fraction", Kind::Aux, "Rate * 1", "dmnl"));
    m.vars[1].units = "dmnl".into();
    m.vars[2].equation = "Cash * Use Fraction".into();
    dynamist_core::ops::sync_links_for_tests(&mut m);
    assert!(x_findings(&m, "X1").iter().any(|f| f.severity == Severity::Warning && f.message.contains("leaves the range 0 to 1")), "{:?}", x_findings(&m, "X1"));
    m.vars[3].units = "dollar".into();
    assert!(x_findings(&m, "X1").iter().all(|f| f.severity == Severity::Info), "{:?}", x_findings(&m, "X1"));
}

#[test]
fn x6_holds_the_extreme_cases_to_invariants_only() {
    let mut m = x_model();
    m.checks.push(dynamist_core::checks::Check { name: "Near five".into(), var: "Cash".into(), kind: dynamist_core::checks::CheckKind::Reference, reference: vec![[0.0, 5.0]], tolerance: Some(5.0), ..Default::default() });
    assert!(x_findings(&m, "X6").is_empty(), "a reference is a scenario of the base case: {:?}", x_findings(&m, "X6"));
    m.checks.push(dynamist_core::checks::Check { name: "Never negative".into(), var: "Cash".into(), kind: dynamist_core::checks::CheckKind::NonNeg, ..Default::default() });
    assert_eq!(x_findings(&m, "X6").len(), 1);
}

#[test]
fn doctor_messages_say_not_a_number_where_the_dash_would_be() {
    assert_eq!(dynamist_core::fmt::fmt_num(f64::NAN), "–");
}

#[test]
fn g7_is_information_without_declared_outputs_and_never_offers_to_remove_a_terminal_variable() {
    let mut m = x_model();
    m.vars.push(var(4, "LCOE", Kind::Aux, "Cash / 2", "dollar"));
    dynamist_core::ops::sync_links_for_tests(&mut m);
    let g7 = |m: &Model| doctor::check(m, &Options { stage: Some(Stage::C2), ..Default::default() }).findings.into_iter().filter(|f| f.rule == "G7").collect::<Vec<_>>();
    let f = g7(&m);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!((f[0].severity, f[0].fixes.len()), (Severity::Info, 0));
    // with a declared output elsewhere the unused variable is a warning, still with no fix to delete the end of a chain
    m.checks.push(dynamist_core::checks::Check { name: "c".into(), var: "Cash".into(), kind: dynamist_core::checks::CheckKind::NonNeg, ..Default::default() });
    let f = g7(&m);
    assert_eq!((f[0].severity, f[0].fixes.len()), (Severity::Warning, 0), "{f:?}");
}

fn f3(m: &Model) -> Vec<doctor::Finding> {
    doctor::check(m, &Options { stage: Some(Stage::C3), ..Default::default() }).findings.into_iter().filter(|f| f.rule == "F3").collect()
}

#[test]
fn f3_knows_constant_expressions_and_positive_powers() {
    let m = model(
        "Year",
        vec![
            var(1, "Degradation", Kind::Constant, "0.0025", ""),
            var(2, "Yield", Kind::Constant, "1280", ""),
            var(3, "Output", Kind::Aux, "Yield / (1 - Degradation)^Time", ""),
            var(4, "Scaled", Kind::Aux, "Yield / (2 - 1)", ""),
            var(5, "Power", Kind::Aux, "Yield / POWER(1.5, Time)", ""),
            var(6, "Unknown", Kind::Aux, "Yield / (Degradation - 0.0025)", ""),
            var(7, "Gap", Kind::Aux, "Yield / (Output - Scaled)", ""),
        ],
    );
    let f = f3(&m);
    assert!(f.iter().all(|x| x.var.as_deref() != Some("Output") && x.var.as_deref() != Some("Scaled") && x.var.as_deref() != Some("Power")), "{f:?}");
    // a constant that cancels exactly is zero: a divisor made of numbers only is evaluated, and an unknown difference is possible
    let gap = f.iter().find(|x| x.var.as_deref() == Some("Gap")).expect("a difference of two variables can be zero");
    assert!(!gap.confirmed && gap.message.starts_with("Possible: "), "{}", gap.message);
}

fn rule(m: &Model, id: &str) -> Vec<doctor::Finding> {
    doctor::check(m, &Options { stage: Some(Stage::C4), ..Default::default() }).findings.into_iter().filter(|f| f.rule == id).collect()
}

#[test]
fn f9_lets_period_indices_and_tests_of_time_against_a_constant_through() {
    let mut m = model(
        "Month",
        vec![
            var(1, "Start Month", Kind::Constant, "6", "Month"),
            var(2, "Stock", Kind::Stock, "0", "widget"),
            var(3, "Month Index", Kind::Aux, "MODULO(Time, 12)", "Month"),
            var(4, "Launched", Kind::Aux, "IF THEN ELSE(Time >= Start Month, 1, 0)", "dmnl"),
            var(5, "Inflow", Kind::Flow, "Month Index * Launched", "widget/Month"),
            var(6, "Drift", Kind::Aux, "Time * 2", "widget"),
        ],
    );
    m.vars[4].to = Some(2);
    let f = rule(&m, "F9");
    assert_eq!(f.iter().map(|x| x.var.as_deref().unwrap()).collect::<Vec<_>>(), vec!["Drift"], "{f:?}");
}

#[test]
fn g4_is_information_for_a_stock_over_a_flow() {
    let mut m = model("Day", vec![var(1, "S", Kind::Stock, "10", "widget"), var(2, "Out", Kind::Flow, "S / 5", "widget/Day"), var(3, "Time in Stock", Kind::Aux, "S / Out", "Day"), var(4, "Pressure", Kind::Aux, "Out + 1", "widget/Day"), var(5, "Drain", Kind::Flow, "Pressure", "widget/Day")]);
    m.vars[1].from = Some(1);
    m.vars[4].from = Some(1);
    let f = rule(&m, "G4");
    let little = f.iter().find(|x| x.var.as_deref() == Some("Time in Stock")).expect("Little's law note");
    assert_eq!(little.severity, Severity::Info);
    assert!(f.iter().any(|x| x.var.as_deref() == Some("Pressure") && x.severity == Severity::Warning), "{f:?}");
}

#[test]
fn f4_does_not_flag_pi_and_calls_calendar_numbers_a_note() {
    let m = model("Day", vec![var(1, "Radius", Kind::Constant, "2", "m"), var(2, "Area", Kind::Aux, "3.14159265 * Radius * Radius", "m*m"), var(3, "Weekly", Kind::Aux, "Radius * 7", "m"), var(4, "Odd", Kind::Aux, "Radius * 0.37", "m")]);
    let f = doctor::check(&m, &Options { stage: Some(Stage::C4), ..Default::default() }).findings.into_iter().filter(|f| f.rule == "F4").collect::<Vec<_>>();
    assert!(f.iter().all(|x| x.var.as_deref() != Some("Area")), "{f:?}");
    assert_eq!(f.iter().find(|x| x.var.as_deref() == Some("Weekly")).map(|x| (x.severity, x.fixes.len())), Some((Severity::Info, 1)));
    assert_eq!(f.iter().find(|x| x.var.as_deref() == Some("Odd")).map(|x| x.severity), Some(Severity::Warning));
}

#[test]
fn x5_skips_a_pure_accumulator_and_names_the_stock_of_the_model() {
    let mut m = model(
        "Day",
        vec![
            var(1, "Level", Kind::Stock, "100", "widget"),
            var(2, "Input", Kind::Constant, "10", "widget/Day"),
            var(3, "Tau", Kind::Constant, "5", "Day"),
            var(4, "Fill", Kind::Flow, "Input", "widget/Day"),
            var(5, "Drain", Kind::Flow, "Level / Tau", "widget/Day"),
            var(6, "Counter", Kind::Stock, "0", "widget"),
            var(7, "Count", Kind::Flow, "Input", "widget/Day"),
        ],
    );
    m.vars[3].to = Some(1);
    m.vars[4].from = Some(1);
    m.vars[6].to = Some(6);
    m.settings.final_time = 60.0;
    m.settings.time_step = 0.25;
    dynamist_core::ops::sync_links_for_tests(&mut m);
    assert!(x_findings(&m, "X5").is_empty(), "{:?}", x_findings(&m, "X5"));
    // without the skip a counter that grows for ever is not what a settling test is about; with a leak it is a loop and settles
}

#[test]
fn info_reports_the_order_next_to_the_stocks() {
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/ch02-growth.mdl")).unwrap();
    let m = dynamist_core::mdl::import(&text).unwrap();
    let v = dynamist_core::ops::info(&m);
    // one stock (Crews) and a SMOOTH3 with three states of its own
    assert_eq!((v["stocks"].as_u64(), v["order"].as_u64()), (Some(1), Some(4)), "{v}");
}

#[test]
fn a_figure_model_has_its_unit_findings_as_information() {
    let mut m = model("Day", vec![var(1, "Stock", Kind::Stock, "0", "widget"), var(2, "Rate", Kind::Flow, "Stock * 0.5 + 3", "widget/Day")]);
    m.vars[1].to = Some(1);
    let errors = |m: &Model| doctor::check(m, &units_only()).findings.iter().filter(|f| f.severity == Severity::Error).count();
    assert!(errors(&m) > 0, "a stock added to a rate is an error");
    let mut ops_m = m.clone();
    dynamist_core::ops::apply(&mut ops_m, &serde_json::from_str(r#"{"op":"set_setting","key":"figure_model","value":"true"}"#).unwrap()).unwrap();
    assert!(ops_m.settings.figure_model);
    assert_eq!(errors(&ops_m), 0);
    assert!(doctor::check(&ops_m, &units_only()).findings.iter().any(|f| f.severity == Severity::Info && f.message.contains("figure model")));
}

#[test]
fn x6_leaves_a_value_that_is_not_a_number_to_x1() {
    // a check on a quantity that divides by a constant: at 0 times the constant the value is not a number, which X1 reports
    let mut m = model("Day", vec![var(1, "Pool", Kind::Stock, "10", "widget"), var(2, "Cost", Kind::Constant, "2", "dmnl"), var(3, "Share", Kind::Aux, "Pool / Cost", "widget"), var(4, "Out", Kind::Flow, "Share / 10 + Cost", "widget/Day")]);
    m.vars[3].from = Some(1);
    m.checks.push(dynamist_core::checks::Check { name: "Share stays bounded".into(), var: "Share".into(), min: Some(-1e9), max: Some(1e9), kind: dynamist_core::checks::CheckKind::Bounds, ..Default::default() });
    dynamist_core::ops::sync_links_for_tests(&mut m);
    assert!(x_findings(&m, "X6").is_empty(), "{:?}", x_findings(&m, "X6"));
    assert!(x_findings(&m, "X1").iter().any(|f| f.var.as_deref() == Some("Cost") && f.message.contains("not a number")), "{:?}", x_findings(&m, "X1"));
}

#[test]
fn x6_says_a_run_diverges_and_reports_the_worst_broken_check_not_the_first() {
    use dynamist_core::checks::{Check, CheckKind};
    let mut m = model("Day", vec![var(1, "Stock", Kind::Stock, "10", "widget"), var(2, "Gain", Kind::Constant, "0.5", "1/Day"), var(3, "Out", Kind::Flow, "Stock * Gain", "widget/Day"), var(4, "Warning Light", Kind::Aux, "IF THEN ELSE(Stock < -1, 2, 1)", "dmnl")]);
    m.vars[2].from = Some(1);
    m.settings.final_time = 40.0;
    m.settings.time_step = 1.0;
    // the small breach is listed first
    m.checks.push(Check { name: "Light stays at one".into(), var: "Warning Light".into(), max: Some(1.0), kind: CheckKind::Bounds, ..Default::default() });
    m.checks.push(Check { name: "Stock never negative".into(), var: "Stock".into(), kind: CheckKind::NonNeg, ..Default::default() });
    dynamist_core::ops::sync_links_for_tests(&mut m);
    let f = x_findings(&m, "X6");
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].message.contains("diverges") && f[0].message.contains("Stock never negative"), "{}", f[0].message);
}

#[test]
fn a_check_marked_as_a_validity_range_or_a_promise_breaks_as_information_under_an_extreme_input() {
    use dynamist_core::checks::{Check, CheckKind};
    let build = |role: &str| {
        let mut m = model("Day", vec![var(1, "Crews", Kind::Stock, "10", "crew"), var(2, "Demand", Kind::Constant, "5", "crew/Day"), var(3, "Hire", Kind::Flow, "Demand", "crew/Day")]);
        m.vars[2].to = Some(1);
        m.checks.push(Check { name: "Region carries 60 crews".into(), var: "Crews".into(), max: Some(60.0), kind: CheckKind::Bounds, role: role.into(), ..Default::default() });
        dynamist_core::ops::sync_links_for_tests(&mut m);
        m
    };
    let plain = x_findings(&build(""), "X6");
    assert_eq!((plain.len(), plain[0].severity), (1, Severity::Error), "{plain:?}");
    for role in ["validity", "promise"] {
        let f = x_findings(&build(role), "X6");
        assert_eq!((f.len(), f[0].severity), (1, Severity::Info), "{f:?}");
        assert!(f[0].message.contains("outside the declared range"), "{}", f[0].message);
    }
    // the op takes the role, and refuses another word
    let mut m = build("");
    let op = |j: &str| serde_json::from_str::<dynamist_core::ops::Op>(j).unwrap();
    dynamist_core::ops::apply(&mut m, &op(r#"{"op":"add_check","name":"Cap","var":"Crews","max":70,"role":"validity"}"#)).unwrap();
    assert_eq!(m.checks.last().unwrap().role, "validity");
    assert!(dynamist_core::ops::apply(&mut m, &op(r#"{"op":"add_check","name":"Cap2","var":"Crews","max":70,"role":"wish"}"#)).is_err());
}

#[test]
fn a_constant_with_a_declared_range_is_tried_at_the_ends_of_that_range_only() {
    use dynamist_core::checks::{Check, CheckKind};
    let build = |lever: bool| {
        let mut m = model("Month", vec![var(1, "Staff", Kind::Stock, "0", "person"), var(2, "Hiring Delay", Kind::Constant, "12", "Month"), var(3, "Hires", Kind::Flow, "DELAY3((100 - Staff) * 0.02, Hiring Delay)", "person/Month")]);
        m.vars[2].to = Some(1);
        m.settings.final_time = 60.0;
        m.settings.time_step = 1.0;
        m.checks.push(Check { name: "Staff never negative".into(), var: "Staff".into(), kind: CheckKind::NonNeg, ..Default::default() });
        if lever {
            let op: dynamist_core::ops::Op = serde_json::from_str(r#"{"op":"set_manifest","manifest":{"levers":[{"name":"Hiring Delay","min":6,"max":18}]}}"#).unwrap();
            dynamist_core::ops::apply(&mut m, &op).unwrap();
        }
        dynamist_core::ops::sync_links_for_tests(&mut m);
        m
    };
    // one time step is inside the instability of three Euler stages of a third of a step each
    let f = x_findings(&build(false), "X6");
    assert!(f.len() == 1 && f[0].message.contains("one time step"), "{f:?}");
    assert!(x_findings(&build(true), "X6").is_empty(), "{:?}", x_findings(&build(true), "X6"));
    assert!(x_findings(&build(true), "X1").iter().all(|f| f.severity == Severity::Info));
}

#[test]
fn a_factor_of_seven_between_days_and_weeks_says_it_assumes_a_seven_day_week() {
    let mut m = model("Week", vec![var(1, "Backlog", Kind::Stock, "100", "Ticket"), var(2, "Rate", Kind::Constant, "4", "Ticket/Day"), var(3, "Resolutions", Kind::Flow, "Rate", "")]);
    m.vars[2].from = Some(1);
    let r = doctor::check(&m, &units_only());
    let f = r.findings.iter().find(|f| f.code == "unit-scale-mismatch").expect("a scale mismatch");
    assert!(f.message.contains("a factor 7 apart (assuming a 7-day week)"), "{}", f.message);
    // other factors carry no such note
    let mut n = model("Month", vec![var(1, "Pile", Kind::Stock, "100", "kg"), var(2, "Rate", Kind::Constant, "4", "g/Month"), var(3, "Take", Kind::Flow, "Rate", "")]);
    n.vars[2].from = Some(1);
    let r = doctor::check(&n, &units_only());
    assert!(r.findings.iter().filter(|f| f.code == "unit-scale-mismatch").all(|f| !f.message.contains("7-day")));
}

#[test]
fn a_teaching_draft_has_its_extreme_condition_findings_as_information() {
    use dynamist_core::checks::{Check, CheckKind};
    let mut m = model("Day", vec![var(1, "Backlog", Kind::Stock, "5", "job"), var(2, "Arrivals", Kind::Constant, "2", "job/Day"), var(3, "Done", Kind::Flow, "5 - Arrivals", "job/Day")]);
    m.vars[2].from = Some(1);
    m.checks.push(Check { name: "Backlog never negative".into(), var: "Backlog".into(), kind: CheckKind::NonNeg, ..Default::default() });
    dynamist_core::ops::sync_links_for_tests(&mut m);
    let x = |m: &Model| doctor::check(m, &Options { experiments: true, jobs: 2, stage: Some(Stage::C7), ..Default::default() }).findings.into_iter().filter(|f| f.rule.starts_with('X')).collect::<Vec<_>>();
    assert!(x(&m).iter().any(|f| f.severity == Severity::Error && f.rule == "X6"), "{:?}", x(&m));
    dynamist_core::ops::apply(&mut m, &serde_json::from_str(r#"{"op":"set_setting","key":"teaching_draft","value":"true"}"#).unwrap()).unwrap();
    let f = x(&m);
    assert!(!f.is_empty() && f.iter().all(|f| f.severity == Severity::Info), "{f:?}");
    assert!(f.iter().any(|f| f.message.contains("teaching draft")));
    // the mark is a loss in a Vensim file
    assert!(dynamist_core::mdlw::write(&m).1.items.iter().any(|i| i.construct.contains("teaching_draft")));
}
