//! Phase 3, section 2: scalar finance functions in equations, against the independent golden values.
use dynamist_core::model::{Kind, Model, Var};
use dynamist_core::sim;

fn eval(eq: &str) -> f64 {
    let mut m = Model::default();
    m.settings.final_time = 1.0;
    m.settings.time_step = 1.0;
    let mut v = Var::new(1, "X", Kind::Aux, [0.0, 0.0]);
    v.equation = eq.into();
    m.vars = vec![v];
    m.next_id = 5;
    let c = sim::compile(&m).unwrap_or_else(|e| panic!("{eq}: {:?}", e.iter().map(|p| &p.message).collect::<Vec<_>>()));
    sim::simulate(&c, &m, "t").unwrap().get("X").unwrap()[0]
}

fn close(eq: &str, want: f64) {
    let got = eval(eq);
    assert!((got - want).abs() <= 1e-9 * want.abs().max(1e-12), "{eq}: {got} vs {want}");
}

#[test]
fn golden_scalar_finance() {
    close("PMT(0.05/12, 360, 200000)", -1073.6432460242795);
    close("IPMT(0.05/12, 1, 360, 200000)", -833.3333333333334);
    close("PPMT(0.05/12, 1, 360, 200000)", -240.30991269094613);
    close("FV(0.06/12, 120, -100, -1000)", 18207.33141467809);
    close("PV(0.08, 20, -500, 1000)", 4694.52549632059);
    close("NPER(0.07/12, -200, 5000)", 27.100842790202556);
    close("RATE(48, -200, 8000)", 0.007701472488201888);
    close("ANNUITY FACTOR(0.08, 20)", 0.10185220882315058);
    close("REAL RATE(0.07, 0.025)", 0.043902439024390505);
    close("EFFECTIVE RATE(0.12, 12)", 0.12682503013196977);
    close("WACC(0.7, 0.07, 0.085, 0.257)", 0.061907);
    close("DSCR(10, 0)", 1e9);
}

/// The user-supplied reference table (tolerance 1e-9 relative unless the value is printed to fewer digits).
#[test]
fn golden_json_reference_values() {
    let g: serde_json::Value = serde_json::from_str(include_str!("data/golden.json")).unwrap();
    let want = |k: &str| g[k].as_f64().unwrap();
    let cases: &[(&str, &str)] = &[
        ("PMT(0.05/12, 360, 200000)", "PMT(0.05/12, 360, 200000)"),
        ("IPMT(0.05/12, 1, 360, 200000)", "IPMT(0.05/12, 1, 360, 200000)"),
        ("PPMT(0.05/12, 1, 360, 200000)", "PPMT(0.05/12, 1, 360, 200000)"),
        ("FV(0.06, 10, -1000, 0)", "FV(0.06, 10, -1000, 0)"),
        ("PV(0.07, 20, -10000)", "PV(0.07, 20, -10000)"),
        ("NPER(0.08/12, -500, 20000)", "NPER(0.08/12, -500, 20000)"),
        ("RATE(60, -400, 20000)", "RATE(60, -400, 20000)"),
        ("ANNUITY FACTOR / CRF(0.036, 30)", "ANNUITY FACTOR(0.036, 30)"),
        ("REAL RATE(0.056, 0.02)", "REAL RATE(0.056, 0.02)"),
        ("EFFECTIVE RATE(0.12, 12)", "EFFECTIVE RATE(0.12, 12)"),
        ("WACC(0.8, 0.05, 0.08, 0.2) nominal", "WACC(0.8, 0.05, 0.08, 0.2)"),
        ("NORMAL INV(0.95, 0, 1)", "NORMAL INV(0.95, 0, 1)"),
        ("NORMAL CDF(1.96, 0, 1)", "NORMAL CDF(1.96, 0, 1)"),
        ("LOGNORMAL INV(0.5, mu=0, sigma=1)", "LOGNORMAL INV(0.5, 0, 1)"),
    ];
    for (key, eq) in cases {
        let (got, w) = (eval(eq), want(key));
        // the table is printed to 10 digits (or 9 for the statistics)
        assert!((got - w).abs() <= 2e-9 * w.abs().max(1.0), "{key}: {got} vs {w}");
    }
}
