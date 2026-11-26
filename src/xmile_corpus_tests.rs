//! The fuel-supply model through XMILE: written, read back, and run. Its sets have parents, so
//! the file is written flattened (one variable per tank) and every flat series must come back
//! the same to within 1e-9.

use crate::model::Model;

fn fuel() -> Option<Model> {
    let dir = std::path::Path::new(crate::spreadsheet::WIKI_DIR).join("inbox").join("fuel-supply");
    if !dir.join("Checks.csv").exists() {
        return None;
    }
    Some(crate::spreadsheet::model_from_book(&crate::spreadsheet::read_book(&dir).unwrap()).unwrap().0)
}

fn run(m: &Model) -> crate::sim::Run {
    crate::userfn::set(&m.functions);
    let c = crate::sim::compile(m).unwrap_or_else(|p| panic!("{:?}", p.iter().take(3).map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>()));
    crate::sim::simulate(&c, m, "x").unwrap()
}

#[test]
fn the_fuel_model_survives_a_trip_through_xmile_flattened() {
    let Some(m) = fuel() else { return };
    let (text, wrep) = dynamist_core::xmile::write(&m);
    assert!(wrep.items.iter().any(|i| i.construct == "sets"), "{}", wrep.summary());
    assert!(!text.contains("<dimensions"), "a model with parent sets is written flattened");
    let (back, rrep) = dynamist_core::xmile::read(&text).unwrap_or_else(|e| panic!("cannot read it back: {e}"));
    assert!(back.sets.is_empty());
    let (a, b) = (run(&m), run(&back));
    assert_eq!(a.times, b.times);
    let mut compared = 0;
    for (k, sa) in &a.series {
        let Some(sb) = b.series.get(k) else { continue };
        for (i, (x, y)) in sa.iter().zip(sb).enumerate() {
            assert!((x - y).abs() <= 1e-9 * (1.0 + x.abs()), "{k} differs at step {i}: {x} vs {y}");
        }
        compared += 1;
    }
    assert!(compared > 500, "only {compared} series were compared");
    // the checks and levers come back from the comment the file carries
    assert_eq!(back.checks.len(), m.checks.len());
    assert!(back.vars.iter().any(|v| v.lever), "levers were not restored");
    println!("fuel model: {} variables written, {} series compared; write report: {} item(s), read report: {} item(s)", back.vars.len(), compared, wrep.items.len(), rrep.items.len());
}
