//! Differences between two models (what they mean, not where they are drawn) and between two
//! runs (which series moved, and by how much).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::expr;
use crate::model::{Method, Model, Var};
use crate::sim::Run;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    Added,
    Removed,
    Changed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Difference {
    pub change: Change,
    /// "variable", "set", "check", "function", "setting"
    pub what: String,
    pub name: String,
    /// For a change: what differs, old → new.
    pub detail: String,
}

fn units_of(m: &Model, id: Option<u64>) -> String {
    id.and_then(|i| m.var(i)).map(|v| v.name.clone()).unwrap_or_else(|| "outside".into())
}

fn describe_var_diff(a: &Model, b: &Model, x: &Var, y: &Var) -> Vec<String> {
    let mut d = Vec::new();
    let mut f = |what: &str, p: String, q: String| {
        if p != q {
            d.push(format!("{what}: {p} → {q}"));
        }
    };
    f("kind", x.kind.label().into(), y.kind.label().into());
    f("equation", x.equation.trim().into(), y.equation.trim().into());
    f("units", x.units.trim().into(), y.units.trim().into());
    f("doc", x.comment.trim().into(), y.comment.trim().into());
    f("drains", units_of(a, x.from), units_of(b, y.from));
    f("fills", units_of(a, x.to), units_of(b, y.to));
    f("per", format!("{:?}", x.set), format!("{:?}", y.set));
    f("never negative", x.non_negative.to_string(), y.non_negative.to_string());
    f("lever", x.lever.to_string(), y.lever.to_string());
    f("curve", format!("{:?}", x.table), format!("{:?}", y.table));
    let items = |v: &Var| v.by_item.iter().map(|o| format!("{}={}", o.item, o.equation)).collect::<BTreeSet<_>>();
    f("item values", format!("{:?}", items(x)), format!("{:?}", items(y)));
    d
}

/// What changed in the manifest, which is what a user of the model reads: levers and their ranges, scenarios, goals,
/// validity, tests, outputs and bindings. Items are matched by name (validity by its statement).
fn diff_manifest(a: &Model, b: &Model) -> Vec<Difference> {
    let (x, y) = (&a.manifest, &b.manifest);
    let mut out = Vec::new();
    fn items<T: PartialEq + std::fmt::Debug>(out: &mut Vec<Difference>, what: &str, old: &[T], new: &[T], name: impl Fn(&T) -> String, detail: impl Fn(&T, &T) -> String) {
        let key = |t: &T| expr::normalize(&name(t));
        for o in old {
            match new.iter().find(|n| key(n) == key(o)) {
                None => out.push(Difference { change: Change::Removed, what: what.into(), name: name(o), detail: String::new() }),
                Some(n) if n != o => out.push(Difference { change: Change::Changed, what: what.into(), name: name(o), detail: detail(o, n) }),
                _ => {}
            }
        }
        for n in new.iter().filter(|n| !old.iter().any(|o| key(o) == key(n))) {
            out.push(Difference { change: Change::Added, what: what.into(), name: name(n), detail: String::new() });
        }
    }
    let diffs = |pairs: Vec<(&str, String, String)>| pairs.into_iter().filter(|(_, p, q)| p != q).map(|(k, p, q)| format!("{k}: {p} → {q}")).collect::<Vec<_>>().join("; ");
    if x.stakes != y.stakes {
        out.push(Difference { change: Change::Changed, what: "manifest".into(), name: "stakes".into(), detail: format!("{:?} → {:?}", x.stakes, y.stakes) });
    }
    if x.title != y.title || x.description != y.description {
        out.push(Difference { change: Change::Changed, what: "manifest".into(), name: "title and description".into(), detail: diffs(vec![("title", x.title.clone(), y.title.clone()), ("description", x.description.clone(), y.description.clone())]) });
    }
    items(&mut out, "manifest lever", &x.levers, &y.levers, |l| l.name.clone(), |p, q| diffs(vec![("range", format!("{} to {}", p.min, p.max), format!("{} to {}", q.min, q.max)), ("default", format!("{:?}", p.default), format!("{:?}", q.default)), ("units", p.units.clone(), q.units.clone()), ("explanation", p.explanation.clone(), q.explanation.clone()), ("group", p.group.clone(), q.group.clone())]));
    items(&mut out, "manifest scenario", &x.scenarios, &y.scenarios, |s| s.name.clone(), |p, q| {
        let mut parts = Vec::new();
        for k in p.values.keys().chain(q.values.keys()).collect::<BTreeSet<_>>() {
            let (u, v) = (p.values.get(k), q.values.get(k));
            if u != v {
                parts.push(format!("{k}: {} → {}", u.map_or("not set".to_string(), |x| x.to_string()), v.map_or("not set".to_string(), |x| x.to_string())));
            }
        }
        for k in p.expect.keys().chain(q.expect.keys()).collect::<BTreeSet<_>>() {
            let (u, v) = (p.expect.get(k), q.expect.get(k));
            if u != v {
                parts.push(format!("expected direction of {k}: {} → {}", u.map_or("none", |x| x.as_str()), v.map_or("none", |x| x.as_str())));
            }
        }
        parts.join("; ")
    });
    items(&mut out, "manifest goal", &x.goals, &y.goals, |g| g.name.clone(), |p, q| diffs(vec![("output", p.output.clone(), q.output.clone()), ("measure", p.measure.clone(), q.measure.clone()), ("direction", p.direction.clone(), q.direction.clone()), ("policies", format!("{:?}", p.policies), format!("{:?}", q.policies)), ("protected ranking", p.protected_ranking.to_string(), q.protected_ranking.to_string()), ("weight", format!("{:?}", p.weight), format!("{:?}", q.weight)), ("transform", p.transform.clone(), q.transform.clone())]));
    items(&mut out, "manifest validity", &x.validity, &y.validity, |v| v.statement.clone(), |p, q| diffs(vec![("variable", format!("{:?}", p.variable), format!("{:?}", q.variable)), ("min", format!("{:?}", p.min), format!("{:?}", q.min)), ("max", format!("{:?}", p.max), format!("{:?}", q.max)), ("horizon", format!("{:?}", p.horizon), format!("{:?}", q.horizon))]));
    items(&mut out, "manifest test", &x.tests, &y.tests, |t| t.clone(), |_, _| String::new());
    items(&mut out, "manifest output", &x.outputs, &y.outputs, |o| o.variable.clone(), |p, q| diffs(vec![("chart", p.chart.clone(), q.chart.clone()), ("units", p.units.clone(), q.units.clone()), ("reference lines", format!("{:?}", p.reference_lines), format!("{:?}", q.reference_lines))]));
    items(&mut out, "manifest binding", &x.bindings, &y.bindings, |b| b.input.clone(), |p, q| diffs(vec![("source", p.source.clone(), q.source.clone()), ("column", p.column.clone(), q.column.clone()), ("kind", p.kind.clone(), q.kind.clone()), ("note", p.note.clone(), q.note.clone())]));
    out
}

/// Semantic differences of `a` → `b`, ignoring every position, colour and arrow shape. The manifest comes first.
pub fn diff_models(a: &Model, b: &Model) -> Vec<Difference> {
    let mut out = diff_manifest(a, b);
    let key = |v: &Var| expr::normalize(&v.name);
    let (am, bm): (BTreeMap<String, &Var>, BTreeMap<String, &Var>) = (a.vars.iter().map(|v| (key(v), v)).collect(), b.vars.iter().map(|v| (key(v), v)).collect());
    for (k, x) in &am {
        match bm.get(k) {
            None => out.push(Difference { change: Change::Removed, what: "variable".into(), name: x.name.clone(), detail: format!("{} {}", x.kind.label(), x.equation) }),
            Some(y) => {
                let d = describe_var_diff(a, b, x, y);
                if !d.is_empty() {
                    out.push(Difference { change: Change::Changed, what: "variable".into(), name: y.name.clone(), detail: d.join("; ") });
                }
            }
        }
    }
    for (k, y) in &bm {
        if !am.contains_key(k) {
            out.push(Difference { change: Change::Added, what: "variable".into(), name: y.name.clone(), detail: format!("{} {}", y.kind.label(), y.equation) });
        }
    }
    // a variable removed and another added with the same kind, equation, units and flow ends is a rename
    {
        let key = |m: &Model, v: &Var| format!("{}|{}|{}|{}|{}|{:?}", v.kind.label(), v.equation.trim(), v.units.trim(), units_of(m, v.from), units_of(m, v.to), v.set);
        let removed: Vec<(usize, String)> = out.iter().enumerate().filter(|(_, d)| d.what == "variable" && d.change == Change::Removed).filter_map(|(i, d)| am.get(&expr::normalize(&d.name)).map(|v| (i, key(a, v)))).collect();
        let added: Vec<(usize, String)> = out.iter().enumerate().filter(|(_, d)| d.what == "variable" && d.change == Change::Added).filter_map(|(i, d)| bm.get(&expr::normalize(&d.name)).map(|v| (i, key(b, v)))).collect();
        let mut drop = Vec::new();
        for (ri, rk) in &removed {
            let twins: Vec<&(usize, String)> = added.iter().filter(|(_, ak)| ak == rk).collect();
            let same_removed = removed.iter().filter(|(_, k)| k == rk).count();
            if twins.len() == 1 && same_removed == 1 {
                let (ai, _) = twins[0];
                out[*ri] = Difference { change: Change::Changed, what: "variable".into(), name: format!("{} → {}", out[*ri].name, out[*ai].name), detail: "renamed".into() };
                drop.push(*ai);
            }
        }
        drop.sort_unstable_by(|x, y| y.cmp(x));
        for i in drop {
            out.remove(i);
        }
    }
    // sets
    for s in &a.sets {
        match b.sets.iter().find(|t| t.name == s.name) {
            None => out.push(Difference { change: Change::Removed, what: "set".into(), name: s.name.clone(), detail: format!("{} items", s.items.len()) }),
            Some(t) if t.items != s.items || t.parent != s.parent || t.parent_of != s.parent_of => out.push(Difference { change: Change::Changed, what: "set".into(), name: s.name.clone(), detail: format!("{} items → {} items", s.items.len(), t.items.len()) }),
            _ => {}
        }
    }
    for t in b.sets.iter().filter(|t| !a.sets.iter().any(|s| s.name == t.name)) {
        out.push(Difference { change: Change::Added, what: "set".into(), name: t.name.clone(), detail: format!("{} items", t.items.len()) });
    }
    // checks and functions, by name
    let cn = |c: &crate::checks::Check| if c.name.trim().is_empty() { c.var.clone() } else { c.name.clone() };
    for c in &a.checks {
        match b.checks.iter().find(|d| cn(d) == cn(c)) {
            None => out.push(Difference { change: Change::Removed, what: "check".into(), name: cn(c), detail: String::new() }),
            Some(d) if d != c => out.push(Difference { change: Change::Changed, what: "check".into(), name: cn(c), detail: format!("{:?} → {:?}", (c.min, c.max, c.when), (d.min, d.max, d.when)) }),
            _ => {}
        }
    }
    for d in b.checks.iter().filter(|d| !a.checks.iter().any(|c| cn(c) == cn(d))) {
        out.push(Difference { change: Change::Added, what: "check".into(), name: cn(d), detail: String::new() });
    }
    for f in &a.functions {
        match b.functions.iter().find(|g| expr::normalize(&g.name) == expr::normalize(&f.name)) {
            None => out.push(Difference { change: Change::Removed, what: "function".into(), name: f.name.clone(), detail: String::new() }),
            Some(g) if g.body != f.body || g.params != f.params => out.push(Difference { change: Change::Changed, what: "function".into(), name: f.name.clone(), detail: format!("{} → {}", f.body, g.body) }),
            _ => {}
        }
    }
    for g in b.functions.iter().filter(|g| !a.functions.iter().any(|f| expr::normalize(&f.name) == expr::normalize(&g.name))) {
        out.push(Difference { change: Change::Added, what: "function".into(), name: g.name.clone(), detail: String::new() });
    }
    // modules and their instances
    for d in &a.defs {
        match b.defs.iter().find(|e| e.name == d.name) {
            None => out.push(Difference { change: Change::Removed, what: "module".into(), name: d.name.clone(), detail: String::new() }),
            Some(e) if e != d => out.push(Difference { change: Change::Changed, what: "module".into(), name: d.name.clone(), detail: format!("{} → {} variables", d.vars.len(), e.vars.len()) }),
            _ => {}
        }
    }
    for e in b.defs.iter().filter(|e| !a.defs.iter().any(|d| d.name == e.name)) {
        out.push(Difference { change: Change::Added, what: "module".into(), name: e.name.clone(), detail: String::new() });
    }
    for i in &a.instances {
        match b.instances.iter().find(|j| j.name == i.name) {
            None => out.push(Difference { change: Change::Removed, what: "instance".into(), name: i.name.clone(), detail: i.def.clone() }),
            Some(j) if j != i => out.push(Difference { change: Change::Changed, what: "instance".into(), name: i.name.clone(), detail: format!("params {:?} → {:?}; inputs {:?} → {:?}", i.params, j.params, i.inputs, j.inputs) }),
            _ => {}
        }
    }
    for j in b.instances.iter().filter(|j| !a.instances.iter().any(|i| i.name == j.name)) {
        out.push(Difference { change: Change::Added, what: "instance".into(), name: j.name.clone(), detail: j.def.clone() });
    }
    // settings
    let (s, t) = (&a.settings, &b.settings);
    let mut setting = |name: &str, p: String, q: String| {
        if p != q {
            out.push(Difference { change: Change::Changed, what: "setting".into(), name: name.into(), detail: format!("{p} → {q}") });
        }
    };
    setting("initial_time", s.initial_time.to_string(), t.initial_time.to_string());
    setting("final_time", s.final_time.to_string(), t.final_time.to_string());
    setting("time_step", s.time_step.to_string(), t.time_step.to_string());
    setting("time_units", s.time_units.clone(), t.time_units.clone());
    let method = |m: Method| format!("{m:?}");
    setting("method", method(s.method), method(t.method));
    out
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeriesDiff {
    pub name: String,
    pub max_abs: f64,
    /// Largest difference relative to the series' size.
    pub max_rel: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RunDiff {
    pub only_in_a: Vec<String>,
    pub only_in_b: Vec<String>,
    pub differing: Vec<SeriesDiff>,
    pub compared: usize,
    pub same_times: bool,
}

impl RunDiff {
    pub fn is_same(&self) -> bool {
        self.same_times && self.only_in_a.is_empty() && self.only_in_b.is_empty() && self.differing.is_empty()
    }
}

/// Series that differ by more than `tol` (relative to their own size, absolute near zero).
pub fn diff_runs(a: &Run, b: &Run, tol: f64) -> RunDiff {
    let mut d = RunDiff { same_times: a.times.len() == b.times.len() && a.times.iter().zip(&b.times).all(|(x, y)| (x - y).abs() < 1e-9), ..Default::default() };
    let mut names: Vec<&String> = a.series.keys().chain(b.series.keys()).collect();
    names.sort();
    names.dedup();
    for n in names {
        match (a.series.get(n), b.series.get(n)) {
            (Some(_), None) => d.only_in_a.push(n.clone()),
            (None, Some(_)) => d.only_in_b.push(n.clone()),
            (Some(x), Some(y)) => {
                d.compared += 1;
                let scale = x.iter().chain(y.iter()).fold(0.0f64, |m, v| m.max(v.abs())).max(1.0);
                let max_abs = x.iter().zip(y).map(|(p, q)| (p - q).abs()).fold(0.0, f64::max);
                if max_abs / scale > tol || x.len() != y.len() {
                    d.differing.push(SeriesDiff { name: n.clone(), max_abs, max_rel: max_abs / scale });
                }
            }
            _ => {}
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{Op, Session};

    fn base() -> Session {
        let mut s = Session::new(Model::default());
        let ops: Vec<Op> = serde_json::from_str(r#"[
            {"op":"add_var","name":"Tank","kind":"stock","equation":"100","units":"MB"},
            {"op":"add_var","name":"Burn","kind":"flow","equation":"4","units":"MB/Day","from":"Tank"}]"#).unwrap();
        s.apply("t", &ops).unwrap();
        s
    }

    #[test]
    fn positions_are_not_differences_but_meaning_is() {
        let s = base();
        let mut moved = s.model.clone();
        moved.vars[0].pos = [999.0, 5.0];
        assert!(diff_models(&s.model, &moved).is_empty());
        let mut t = s.clone();
        let more: Vec<Op> = serde_json::from_str(r#"[{"op":"set_equation","name":"Burn","equation":"5"},{"op":"add_var","name":"Fill","kind":"flow","equation":"1","to":"Tank"}]"#).unwrap();
        t.apply("t", &more).unwrap();
        let d = diff_models(&s.model, &t.model);
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(d.iter().any(|x| x.change == Change::Changed && x.name == "Burn" && x.detail.contains("4 → 5")));
        assert!(d.iter().any(|x| x.change == Change::Added && x.name == "Fill"));
    }

    #[test]
    fn a_rename_is_one_change_not_a_removal_and_an_addition() {
        let s = base();
        let mut t = s.clone();
        t.apply("t", &serde_json::from_str::<Vec<Op>>(r#"[{"op":"rename_var","name":"Burn","to":"Daily burn"}]"#).unwrap()).unwrap();
        let d = diff_models(&s.model, &t.model);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!((d[0].change, d[0].name.as_str(), d[0].detail.as_str()), (Change::Changed, "Burn → Daily burn", "renamed"));
    }

    #[test]
    fn runs_differ_only_where_they_move() {
        let s = base();
        let c = crate::sim::compile(&s.model).unwrap();
        let a = crate::sim::simulate(&c, &s.model, "a").unwrap();
        assert!(diff_runs(&a, &a, 1e-9).is_same());
        let mut t = s.clone();
        t.apply("t", &serde_json::from_str::<Vec<Op>>(r#"[{"op":"set_equation","name":"Burn","equation":"5"}]"#).unwrap()).unwrap();
        let b = crate::sim::simulate(&crate::sim::compile(&t.model).unwrap(), &t.model, "b").unwrap();
        let d = diff_runs(&a, &b, 1e-9);
        assert!(!d.is_same() && d.differing.iter().any(|x| x.name == "tank"), "{d:?}");
    }

    #[test]
    fn a_changed_manifest_is_a_difference() {
        use crate::manifest::{Goal, ManifestLever, ManifestScenario, Validity};
        let mut a = base().model;
        a.manifest.levers.push(ManifestLever { name: "Burn".into(), min: 1.0, max: 9.0, ..Default::default() });
        a.manifest.scenarios.push(ManifestScenario { name: "Fast".into(), values: [("Burn".to_string(), 8.0)].into(), ..Default::default() });
        a.manifest.scenarios.push(ManifestScenario { name: "Slow".into(), values: [("Burn".to_string(), 2.0)].into(), ..Default::default() });
        a.manifest.goals.push(Goal { name: "Keep".into(), output: "Tank".into(), measure: "final".into(), direction: "max".into(), ..Default::default() });
        let mut b = a.clone();
        b.manifest.scenarios[0].values.insert("Burn".into(), 7.0);
        b.manifest.scenarios[1].values.insert("Burn".into(), 3.0);
        b.manifest.goals[0].direction = "min".into();
        b.manifest.levers[0].max = 12.0;
        b.manifest.validity.push(Validity { statement: "Short runs".into(), horizon: Some(30.0), ..Default::default() });
        b.manifest.tests.push("Never negative".into());
        let d = diff_models(&a, &b);
        let whats: Vec<&str> = d.iter().map(|x| x.what.as_str()).collect();
        for w in ["manifest scenario", "manifest goal", "manifest lever", "manifest validity", "manifest test"] {
            assert!(whats.contains(&w), "{w}: {d:?}");
        }
        assert_eq!(whats.iter().filter(|w| **w == "manifest scenario").count(), 2);
        assert!(d.iter().any(|x| x.name == "Fast" && x.detail.contains("8 → 7")), "{d:?}");
        assert!(diff_models(&a, &a).is_empty());
    }
}
