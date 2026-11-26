//! Loop polarity from the partial derivatives along the base run (Phase 0, fix 2).

use dynamist_core::{mdl, ops};

#[test]
fn ch02_growth_market_limit_loop_is_balancing() {
    let m = mdl::import(include_str!("data/ch02-growth.mdl")).unwrap();
    let loops = ops::loops(&m);
    let loops = loops.as_array().unwrap();
    let has = |l: &serde_json::Value, v: &str| l["variables"].as_array().unwrap().iter().any(|x| x == v);
    let market: Vec<_> = loops.iter().filter(|l| has(l, "Utilisation")).collect();
    assert_eq!(market.len(), 1, "{loops:?}");
    assert_eq!(market[0]["polarity"], "balancing", "{:?}", market[0]);
    assert_eq!(market[0]["reinforcing"], false);
    // the hiring loop without the market limit is reinforcing, and departures are balancing
    let hiring = loops.iter().find(|l| has(l, "Crew Additions") && !has(l, "Utilisation")).unwrap();
    assert_eq!(hiring["polarity"], "reinforcing");
    let departures = loops.iter().find(|l| has(l, "Crew Departures")).unwrap();
    assert_eq!(departures["polarity"], "balancing");
}
