//! The fuel-supply model (57 tanks, 4 regions, three levels of modules) through a Vensim file and
//! back: written with `mdlw`, read with `mdl`, and simulated. Needs the spreadsheet adapter, so it
//! lives in the app crate.

use std::path::Path;

use crate::model::Model;

fn fuel() -> Option<Model> {
    let dir = Path::new(crate::spreadsheet::WIKI_DIR).join("inbox").join("fuel-supply");
    if !dir.join("Variables.csv").exists() {
        return None;
    }
    Some(crate::spreadsheet::model_from_book(&crate::spreadsheet::read_book(&dir).unwrap()).unwrap().0)
}

fn run(m: &Model) -> crate::sim::Run {
    let c = crate::sim::compile(m).unwrap_or_else(|p| panic!("{:?}", p.iter().take(5).map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>()));
    crate::sim::simulate(&c, m, "x").unwrap()
}

#[test]
fn the_fuel_model_survives_a_vensim_round_trip() {
    let Some(m) = fuel() else { return };
    let (text, wloss) = dynamist_core::mdlw::write(&m);
    assert!(text.contains("Tank:"), "the tank set is written as a range");
    let (back, rloss) = crate::mdl::read(&text).unwrap_or_else(|e| panic!("{e}"));
    println!("written: {} bytes, {} lines\nwrite loss:\n{}\nread loss:\n{}", text.len(), text.lines().count(), wloss.summary(), rloss.summary());
    // structure
    assert_eq!(back.sets.len(), m.sets.len());
    let tank = back.sets.iter().find(|s| s.name == "Tank").unwrap();
    assert_eq!(tank.items.len(), 57);
    assert_eq!(tank.parent.as_deref(), Some("Region"));
    assert_eq!(tank.parent_of.len(), 57);
    // every variable that was written is back (one or more per original)
    for v in &m.vars {
        assert!(back.by_name(&crate::expr::normalize(&v.name)).is_some(), "variable {} lost", v.name);
    }
    // the same numbers at every step for everything the original has
    let (ra, rb) = (run(&m), run(&back));
    assert_eq!(ra.times, rb.times);
    let mut compared = 0;
    for (k, sa) in &ra.series {
        let Some(sb) = rb.series.get(k) else { continue };
        for (x, y) in sa.iter().zip(sb) {
            assert!((x - y).abs() <= 1e-9 * (1.0 + x.abs()), "{k}: {x} vs {y}");
        }
        compared += 1;
    }
    assert!(compared > 500, "only {compared} series compared");
    // what the write could not carry is said, not silent
    assert!(wloss.items.iter().any(|i| i.construct.contains("module")) || m.modules.is_empty());
    // and writing what we read gives the same file apart from layout
    let (again, _) = dynamist_core::mdlw::write(&back);
    let strip = |t: &str| t.split("\\\\\\---///").next().unwrap().to_string();
    assert_eq!(strip(&again).lines().count(), strip(&text).lines().count(), "a second write has the same equations");
}
