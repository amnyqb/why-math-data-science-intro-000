//! Phase 3, section 3: running finance (VALUE timings, LEVELISED, PAYBACK, LOAN, TAX), against values computed
//! independently in Python (core/tests/data/golden.py).
use dynamist_core::model::{Kind, Model, Var};
use dynamist_core::sim;

fn model(unit: &str, dt: f64, final_t: f64, vars: &[(&str, Kind, &str, &str)]) -> Model {
    let mut m = Model::default();
    m.settings.time_units = unit.into();
    m.settings.time_step = dt;
    m.settings.final_time = final_t;
    for (i, (n, k, e, u)) in vars.iter().enumerate() {
        let mut v = Var::new(i as u64 + 1, n, *k, [0.0, 0.0]);
        v.equation = (*e).into();
        v.units = (*u).into();
        m.vars.push(v);
    }
    m.next_id = 100;
    m
}

fn run(m: &Model) -> sim::Run {
    let c = sim::compile(m).unwrap_or_else(|e| panic!("{:?}", e.iter().map(|p| &p.message).collect::<Vec<_>>()));
    sim::simulate(&c, m, "t").unwrap()
}

fn solar(capex: &str, om: &str, yld: &str, degr: &str, d: &str, kd: &str, ke: &str, t: &str, infl: &str, factor: &str) -> f64 {
    let m = model("Year", 1.0, 29.0, &[
        ("Capex", Kind::Constant, capex, ""),
        ("OM", Kind::Constant, om, ""),
        ("Yield", Kind::Constant, yld, ""),
        ("Degradation", Kind::Constant, degr, ""),
        ("Real WACC", Kind::Aux, &format!("REAL RATE(WACC({d}, {kd}, {ke}, {t}), {infl})"), ""),
        ("Output", Kind::Aux, "Yield * (1 - Degradation)^Time", ""),
        ("LCOE", Kind::Aux, &format!("({factor}) * LEVELISED(OM, Output, Real WACC, Capex)"), ""),
    ]);
    *run(&m).get("LCOE").unwrap().last().unwrap()
}

#[test]
fn levelised_default_inputs_give_31_19() {
    // Capex 750, yield 1800, degradation 0.005, O&M 12, 30 years, D 0.8, kd 0.05, ke 0.08, T 0, inflation 0.02
    let v = solar("750", "12", "1800", "0.005", "0.8", "0.05", "0.08", "0", "0.02", "1000");
    assert!((v - 31.19199094983617).abs() < 1e-6, "{v}");
    assert!((v - 31.19).abs() < 0.01);
}

#[test]
fn levelised_fraunhofer_gives_41_10() {
    let v = solar("700", "13.3", "1280", "0.0025", "0.8", "0.05", "0.065", "0", "0.018", "1000");
    assert!((v - 41.10324775439558).abs() < 1e-6, "{v}");
    assert!((v - 41.10).abs() < 0.01);
}

#[test]
fn levelised_nrel() {
    // NREL inputs with the combined tax rate T = 0.21 + 0.06 * (1 - 0.21) = 0.2574 (golden.json: 41.4332)
    let v = solar("1430", "24", "0.283 * 8760", "0", "0.70", "0.07", "0.085", "0.2574", "0.025", "1000");
    assert!((v - 41.4332).abs() < 1e-4, "{v}");
    // with the rounded T = 0.257 the same inputs give 41.4407 (an independent Python computation)
    let w = solar("1430", "24", "0.283 * 8760", "0", "0.70", "0.07", "0.085", "0.257", "0.025", "1000");
    assert!((w - 41.44072764932272).abs() < 1e-6, "{w}");
}

#[test]
fn loan_annuity_monthly() {
    let vars = |measure: &str| {
        let eq = format!("LOAN(Drawdown, Loan Rate, Term, type=annuity, measure={measure})");
        model("Month", 1.0, 360.0, &[
            ("Drawdown", Kind::Aux, "IF THEN ELSE(Time = 0, 200000, 0)", ""),
            ("Loan Rate", Kind::Constant, "0.05", "1/Year"),
            ("Term", Kind::Constant, "30", "Year"),
            ("X", Kind::Aux, &eq, ""),
        ])
    };
    let pay = run(&vars("payment"));
    let p = pay.get("X").unwrap();
    assert!((p[0] - 1073.643246).abs() < 1e-6 && (p[359] - 1073.643246).abs() < 1e-6, "{} {}", p[0], p[359]);
    assert_eq!(p[360], 0.0);
    let int = run(&vars("interest"));
    assert!((int.get("X").unwrap()[0] - 833.333333).abs() < 1e-6);
    let bal = run(&vars("balance"));
    let b = bal.get("X").unwrap();
    assert_eq!(b[0], 200000.0);
    assert!(b[360].abs() < 1e-6, "{}", b[360]);
    let pri = run(&vars("principal"));
    assert!((pri.get("X").unwrap()[0] - 240.309913).abs() < 1e-6);
}

#[test]
fn payback_simple() {
    let m = model("Year", 1.0, 8.0, &[
        ("Cash", Kind::Aux, "IF THEN ELSE(Time < 1, -1000, 300)", ""),
        ("PB", Kind::Aux, "PAYBACK(Cash)", ""),
    ]);
    let r = run(&m);
    let pb = r.get("PB").unwrap();
    assert!(pb[3].is_nan());
    assert!((pb[4] - (4.0 + 100.0 / 300.0)).abs() < 1e-9, "{}", pb[4]);
    assert!((pb[8] - (4.0 + 100.0 / 300.0)).abs() < 1e-9);
}

#[test]
fn tax_carryforward() {
    let m = model("Year", 1.0, 2.0, &[
        ("Income", Kind::Aux, "IF THEN ELSE(Time = 0, -100, 60)", ""),
        ("Tax", Kind::Aux, "TAX(Income, 0.2, carryforward=yes)", ""),
        ("No Carry", Kind::Aux, "TAX(Income, 0.2, carryforward=no)", ""),
    ]);
    let r = run(&m);
    let t = r.get("Tax").unwrap();
    assert!((t[0].abs() + t[1].abs()) < 1e-12 && (t[2] - 4.0).abs() < 1e-12, "{t:?}");
    let n = r.get("No Carry").unwrap();
    assert!((n[1] - 12.0).abs() < 1e-12);
}

#[test]
fn value_timings_with_dt_equal_to_the_rate_period() {
    let mk = |timing: &str| {
        let m = model("Year", 1.0, 2.0, &[("Cash", Kind::Constant, "100", ""), ("PV", Kind::Aux, &format!("VALUE(Cash, 0.1, timing={timing})"), "")]);
        *run(&m).get("PV").unwrap().last().unwrap()
    };
    let (s, e, mid) = (mk("period_start"), mk("period_end"), mk("mid_period"));
    assert!((s - 100.0 * (1.0 + 1.0 / 1.1 + 1.0 / 1.21)).abs() < 1e-9, "{s}");
    assert!((e - 100.0 * (1.0 / 1.1 + 1.0 / 1.21 + 1.0 / 1.331)).abs() < 1e-9, "{e}");
    assert!((mid - 100.0 * (1.1f64.powf(-0.5) + 1.1f64.powf(-1.5) + 1.1f64.powf(-2.5))).abs() < 1e-9, "{mid}");
}

#[test]
fn escalate_uses_the_period_of_its_rate() {
    let m = model("Year", 1.0, 3.0, &[("Price", Kind::Aux, "ESCALATE(100, 0.03)", "")]);
    let p = run(&m).get("Price").unwrap().clone();
    assert!((p[3] - 100.0 * 1.03f64.powi(3)).abs() < 1e-9);
}
