//! Phase 3, section 4 (and the carry function of section 6): random draws with seeds, distributions, PREVIOUS, HISTORY.
use dynamist_core::model::{Kind, Model, Var};
use dynamist_core::sim;

fn run(final_t: f64, seed: u64, eqs: &[(&str, &str)]) -> sim::Run {
    let mut m = Model::default();
    m.settings.final_time = final_t;
    m.settings.time_step = 1.0;
    m.settings.seed = seed;
    for (i, (n, e)) in eqs.iter().enumerate() {
        let mut v = Var::new(i as u64 + 1, n, Kind::Aux, [0.0, 0.0]);
        v.equation = (*e).into();
        m.vars.push(v);
    }
    m.next_id = 500;
    let c = sim::compile(&m).unwrap_or_else(|e| panic!("{:?}", e.iter().map(|p| &p.message).collect::<Vec<_>>()));
    sim::simulate(&c, &m, "t").unwrap()
}

fn mean_within_4se(eq: &str, mean: f64, sd: f64) {
    let n = 200_000.0;
    let r = run(n - 1.0, 1, &[("X", eq)]);
    let xs = r.get("X").unwrap();
    let m = xs.iter().sum::<f64>() / xs.len() as f64;
    let se = sd / (xs.len() as f64).sqrt();
    assert!((m - mean).abs() < 4.0 * se, "{eq}: mean {m} vs {mean} (4 se {})", 4.0 * se);
}

#[test]
fn every_distribution_has_the_right_mean_over_200000_draws() {
    mean_within_4se("RANDOM TRIANGULAR(-1e9, 1e9, 2)".replace("-1e9", "0").replace("1e9", "6").as_str(), 8.0 / 3.0, (28.0f64 / 18.0).sqrt());
    mean_within_4se("RANDOM LOGNORMAL(0, 1e9, 10, 3)", 10.0, 3.0);
    mean_within_4se("RANDOM GAMMA(0, 1e9, 3, 2)", 6.0, 12.0f64.sqrt());
    mean_within_4se("RANDOM BETA(0, 1, 2, 3)", 0.4, (0.4f64 * 0.6 / 6.0).sqrt());
    mean_within_4se("RANDOM WEIBULL(0, 1e9, 2, 3)", 3.0 * 0.886_226_925_452_758, 3.0 * (1.0 - 0.886_226_925_452_758f64.powi(2)).sqrt());
    mean_within_4se("RANDOM POISSON(0, 1e9, 4)", 4.0, 2.0);
    mean_within_4se("RANDOM BINOMIAL(0, 1e9, 10, 0.3)", 3.0, 2.1f64.sqrt());
    mean_within_4se("RANDOM NEGATIVE BINOMIAL(0, 1e9, 3, 0.4)", 4.5, 11.25f64.sqrt());
    mean_within_4se("RANDOM GEOMETRIC(0, 1e9, 0.25)", 3.0, 12.0f64.sqrt());
    mean_within_4se("RANDOM PARETO(0, 1e9, 5, 2)", 2.5, (4.0f64 * 5.0 / (16.0 * 3.0)).sqrt());
    mean_within_4se("RANDOM PERT(0, 10, 3, 4)", (0.0 + 4.0 * 3.0 + 10.0) / 6.0, (((10.0 - 0.0) / 6.0f64).powi(2)) * 0.0 + 1.7);
    mean_within_4se("RANDOM 0 1()", 0.5, (1.0f64 / 12.0).sqrt());
    mean_within_4se("TRIANGULAR(0, 2, 6)", 8.0 / 3.0, (28.0f64 / 18.0).sqrt());
    mean_within_4se("LOGNORMAL(10, 3)", 10.0, 3.0);
    mean_within_4se("WEIBULL(2, 3)", 3.0 * 0.886_226_925_452_758, 3.0 * (1.0 - 0.886_226_925_452_758f64.powi(2)).sqrt());
    mean_within_4se("POISSON(4)", 4.0, 2.0);
    mean_within_4se("NORMAL(5, 2)", 5.0, 2.0);
    mean_within_4se("UNIFORM(2, 4)", 3.0, (4.0f64 / 12.0).sqrt());
    mean_within_4se("EXPRND(3)", 3.0, 3.0);
}

#[test]
fn seed_stability_adding_an_unrelated_random_variable_changes_nothing() {
    let a = run(50.0, 1, &[("Demand", "RANDOM NORMAL(-100, 100, 0, 1, 0) * 0 + NORMAL(0, 1)")]);
    let b = run(50.0, 1, &[("Noise", "UNIFORM(0, 1)"), ("Demand", "RANDOM NORMAL(-100, 100, 0, 1, 0) * 0 + NORMAL(0, 1)"), ("More", "POISSON(3)")]);
    assert_eq!(a.get("Demand"), b.get("Demand"));
    // a different model seed gives different draws; the same seed the same
    let c = run(50.0, 2, &[("Demand", "NORMAL(0, 1)")]);
    let d = run(50.0, 1, &[("Demand", "NORMAL(0, 1)")]);
    assert_ne!(c.get("Demand"), d.get("Demand"));
    assert_eq!(d.get("Demand"), run(50.0, 1, &[("Demand", "NORMAL(0, 1)")]).get("Demand"));
    // a seed of the call's own
    let e = run(20.0, 1, &[("X", "NORMAL(0, 1, seed=7)")]);
    let f = run(20.0, 9, &[("X", "NORMAL(0, 1, 7)")]);
    assert_eq!(e.get("X"), f.get("X"));
}

#[test]
fn golden_normal_cdf_and_inverse_in_equations() {
    let r = run(1.0, 1, &[("A", "NORMAL CDF(1, 0, 1)"), ("B", "NORMAL INV(0.975, 0, 1)"), ("C", "LOGNORMAL CDF(3, 0.5, 0.4)"), ("D", "LOGNORMAL INV(0.8, 0.5, 0.4)"), ("E", "NORMAL INV(0.9, 10, 2)")]);
    let g = |n: &str| r.get(n).unwrap()[0];
    assert!((g("A") - 0.8413447460685429).abs() < 1e-9);
    assert!((g("B") - 1.9599639845400534).abs() < 1e-9);
    assert!((g("C") - 0.9327422958725134).abs() < 1e-9);
    assert!((g("D") - 2.3086166525239706).abs() < 1e-9);
    assert!((g("E") - 12.563103131089202).abs() < 1e-9);
}

#[test]
fn previous_history_and_round_with_carry() {
    let r = run(10.0, 1, &[("T", "Time * 10"), ("P", "PREVIOUS(T, -1)"), ("H", "HISTORY(T, 3)"), ("C", "ROUND WITH CARRY(0.4)")]);
    assert_eq!(r.get("P").unwrap()[..4], [-1.0, 0.0, 10.0, 20.0]);
    assert_eq!(r.get("H").unwrap()[5], 30.0);
    assert_eq!(r.get("C").unwrap()[..5], [0.0, 0.0, 1.0, 0.0, 1.0]);
    let r = run(999.0, 1, &[("C", "ROUND WITH CARRY(0.4)")]);
    assert_eq!(r.get("C").unwrap().iter().sum::<f64>(), 400.0);
}
