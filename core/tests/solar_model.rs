//! Phase 3, sections 7 and 8: the solar LCOE model rewritten with REAL RATE, WACC and LEVELISED, and its benchmark.
use dynamist_core::{checks, doctor, io, sim};

fn model() -> dynamist_core::model::Model {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../models/solar/solar_lcoe.sdm");
    let text = io::read_text(&path).unwrap();
    io::read_str(&text, io::Format::Sdm).unwrap().0
}

fn lcoe(m: &dynamist_core::model::Model) -> f64 {
    let c = sim::compile(m).unwrap();
    *sim::simulate(&c, m, "x").unwrap().get("LCOE").unwrap().last().unwrap()
}

#[test]
fn at_most_eight_inputs_and_six_equations_and_the_publish_gate() {
    let m = model();
    let inputs = m.vars.iter().filter(|v| v.kind == dynamist_core::model::Kind::Constant && !doctor::base::is_conversion(&m, v)).count();
    let eqs = m.vars.iter().filter(|v| v.kind != dynamist_core::model::Kind::Constant).count();
    assert!(inputs <= 8 && eqs <= 6, "{inputs} inputs, {eqs} equations, groups {:?}", m.groups);
    let r = doctor::check(&m, &doctor::Options { experiments: true, stage: Some(doctor::Stage::C7), jobs: 2, ..Default::default() });
    assert_eq!(r.gate, Some(doctor::Gate::Publish), "{:#?}", r.findings.iter().map(|f| (&f.rule, f.severity, &f.var, &f.message)).collect::<Vec<_>>());
}

#[test]
fn the_benchmark_holds_for_fraunhofer_and_the_default_inputs_give_31_19() {
    let mut m = model();
    assert!((lcoe(&m) - 41.10).abs() < 0.01);
    assert!(checks::all(&m, &{ let c = sim::compile(&m).unwrap(); sim::simulate(&c, &m, "x").unwrap() }).is_empty());
    // the default inputs of the programme text
    for (n, v) in [("Capex", "750"), ("Yield", "1800"), ("Degradation", "0.005"), ("OM", "12"), ("Debt Share", "0.8"), ("Cost of Debt", "0.05"), ("Cost of Equity", "0.08"), ("Inflation", "0.02")] {
        m.var_mut(m.by_name(&dynamist_core::expr::normalize(n)).unwrap().id).unwrap().equation = v.into();
    }
    assert!((lcoe(&m) - 31.19).abs() < 0.01, "{}", lcoe(&m));
    // and now the benchmark fails, naming its source
    let run = { let c = sim::compile(&m).unwrap(); sim::simulate(&c, &m, "x").unwrap() };
    let f = checks::all(&m, &run);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].message.contains("Fraunhofer ISE 2024"), "{}", f[0].message);
}

#[test]
fn the_three_cases_are_scenarios_and_the_constant_is_called_kwh_per_mwh() {
    let m = model();
    assert!(m.by_name("kwh per mwh").is_some_and(|v| v.units == "kWh/MWh" && v.equation == "1000"));
    assert!(m.by_name("eur per mwh per eur per kwh").is_none());
    assert_eq!(m.scenarios.len(), 3);
    for (name, want) in [("Fraunhofer ISE 2024", 41.10), ("Programme defaults", 31.19), ("NREL ATB", 41.4332)] {
        let s = m.scenarios.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("scenario {name}"));
        let mut c = m.clone();
        for ch in &s.changes {
            c.var_mut(c.by_name(&dynamist_core::expr::normalize(&ch.var)).unwrap().id).unwrap().equation = ch.value.clone();
        }
        assert!((lcoe(&c) - want).abs() < 0.01, "{name}: {}", lcoe(&c));
    }
}

#[test]
fn the_exponent_is_divided_by_one_year_so_it_has_no_units() {
    let m = model();
    assert_eq!(m.by_name("output").unwrap().equation, "Yield * (1 - Degradation)^(Time / Year Length)");
    let r = doctor::check(&m, &doctor::Options { units_only: true, ..Default::default() });
    assert!(r.findings.iter().all(|f| f.severity == doctor::Severity::Info), "{:?}", r.findings);
}
