//! Sets of similar things.
//!
//! A *set* is a list of things that share one structure — fifty tanks, four regions, three
//! products. A variable that is "per Tank" is written once; before the model is simulated it is
//! expanded into one copy per tank (`Stock[Hail 2 — Crude]`, `Stock[Layla PP — Diesel]`, …).
//! The diagram, equations and arrows stay short; the simulator sees the plain copies.
//!
//! Rules, in plain words:
//! - Inside a per-Tank formula, another per-Tank variable means *the same tank's* value.
//! - A per-Tank formula may read a per-Region variable: it gets the value of the tank's own region
//!   (when the Tank set says which region each tank belongs to).
//! - `TOTAL(X)` adds up all the items of X; inside a per-Region formula it adds up only the
//!   tanks of that region. `TOTAL(X, COA)` adds up the items that belong to COA. `AVERAGE` likewise.
//! - `X[item]` picks one item.

use std::collections::HashMap;

use crate::expr::{self, Tok};
use crate::model::{ItemSet, Kind, Model, Rollup as RollKind, Var};

/// How the items of a set variable are combined into one number for the diagram and charts.
pub struct Rollup {
    pub name: String,
    pub parts: Vec<String>,
    pub average: bool,
}

pub struct Expanded {
    pub model: Model,
    pub rollups: Vec<Rollup>,
}

pub type Problems = Vec<(Option<String>, String)>;

/// The name of the copy of `var` for `item`.
pub fn flat_name(var: &str, item: &str) -> String {
    format!("{var}[{item}]")
}

fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "'"))
}

#[derive(Clone, Copy)]
enum Ctx<'a> {
    Scalar,
    Item(&'a ItemSet, usize),
}

struct Env<'a> {
    sets: HashMap<String, &'a ItemSet>,
    vars: HashMap<String, &'a Var>,
}

impl Env<'_> {
    fn set_of(&self, v: &Var) -> Option<&ItemSet> {
        v.set.as_ref().and_then(|s| self.sets.get(&expr::normalize(s)).copied())
    }
}

/// Items of `set` an aggregate covers.
fn covered(set: &ItemSet, label: Option<&str>, ctx: Ctx) -> Result<Vec<usize>, String> {
    if let Some(l) = label {
        let ln = expr::normalize(l);
        let v: Vec<usize> = (0..set.items.len())
            .filter(|&i| {
                expr::normalize(&set.items[i]) == ln
                    || set.parent_item(i).is_some_and(|p| expr::normalize(p) == ln)
            })
            .collect();
        if v.is_empty() {
            return Err(format!("'{l}' is not an item of {} or of the group it belongs to", set.name));
        }
        return Ok(v);
    }
    if let (Ctx::Item(g, gi), Some(parent)) = (ctx, set.parent.as_ref()) {
        if expr::normalize(parent) == expr::normalize(&g.name) && expr::normalize(&g.name) != expr::normalize(&set.name) {
            let me = &g.items[gi];
            return Ok((0..set.items.len()).filter(|&i| set.parent_item(i) == Some(me.as_str())).collect());
        }
    }
    Ok((0..set.items.len()).collect())
}

/// Rewrite one formula: per-set names become the plain copy that applies, totals become sums.
fn rewrite(src: &str, ctx: Ctx, env: &Env) -> Result<String, String> {
    // the person's own functions first, so what they use (set variables, TOTAL…) is resolved too
    let src = &crate::userfn::expand_text(src)?;
    let toks = expr::tokenize(src)?;
    let mut out = String::new();
    let mut last = 0;
    let mut i = 0;
    while i < toks.len() {
        let (t, span) = &toks[i];
        if let Tok::Ident(s) = t {
            let n = expr::normalize(s);
            let paren = matches!(toks.get(i + 1), Some((Tok::LParen, _)));
            if paren && (n == "total" || n == "average") {
                // TOTAL ( name [, label] )
                let Some((Tok::Ident(arg), _)) = toks.get(i + 2) else {
                    return Err(format!("{} needs the name of a variable that is per a set", n.to_uppercase()));
                };
                let (label, close) = match (toks.get(i + 3), toks.get(i + 4), toks.get(i + 5)) {
                    (Some((Tok::RParen, _)), _, _) => (None, i + 3),
                    (Some((Tok::Comma, _)), Some((Tok::Ident(l), _)), Some((Tok::RParen, _))) => (Some(l.clone()), i + 5),
                    _ => return Err(format!("Write {0}(name) or {0}(name, group)", n.to_uppercase())),
                };
                let Some(v) = env.vars.get(&expr::normalize(arg)).copied() else {
                    return Err(format!("{}: no variable called '{arg}'", n.to_uppercase()));
                };
                let Some(set) = env.set_of(v) else {
                    return Err(format!("{}: '{}' is not per a set, so there is nothing to add up", n.to_uppercase(), v.name));
                };
                let idx = covered(set, label.as_deref(), ctx)?;
                let text = if idx.is_empty() {
                    "0".to_string()
                } else {
                    let terms: Vec<String> = idx.iter().map(|&k| quoted(&flat_name(&v.name, &set.items[k]))).collect();
                    let sum = terms.join(" + ");
                    if n == "average" { format!("(({sum}) / {})", idx.len()) } else { format!("({sum})") }
                };
                out.push_str(&src[last..span.start]);
                out.push_str(&text);
                last = toks[close].1.end;
                i = close + 1;
                continue;
            }
            if let Some(v) = env.vars.get(&n).copied().filter(|v| v.set.is_some()) {
                let Some(set) = env.set_of(v) else {
                    return Err(format!("'{}' is per '{}', which is not a set of this model", v.name, v.set.clone().unwrap_or_default()));
                };
                // explicit item: Name[item]
                if let (Some((Tok::LBracket, _)), Some((Tok::Ident(item), _)), Some((Tok::RBracket, e))) = (toks.get(i + 1), toks.get(i + 2), toks.get(i + 3)) {
                    let Some(k) = set.items.iter().position(|x| expr::normalize(x) == expr::normalize(item)) else {
                        return Err(format!("'{item}' is not an item of {}", set.name));
                    };
                    out.push_str(&src[last..span.start]);
                    out.push_str(&quoted(&flat_name(&v.name, &set.items[k])));
                    last = e.end;
                    i += 4;
                    continue;
                }
                let item = match ctx {
                    Ctx::Scalar => {
                        return Err(format!(
                            "'{0}' has one value per {1}: use TOTAL({0}), AVERAGE({0}) or {0}[item]",
                            v.name, set.name
                        ));
                    }
                    Ctx::Item(g, gi) => {
                        if expr::normalize(&g.name) == expr::normalize(&set.name) {
                            set.items[gi].clone()
                        } else if g.parent.as_ref().is_some_and(|p| expr::normalize(p) == expr::normalize(&set.name)) {
                            match g.parent_item(gi).filter(|p| set.items.iter().any(|x| x == p)) {
                                Some(p) => p.to_string(),
                                None => {
                                    return Err(format!("'{}' has no {} to read '{}' from", g.items[gi], set.name, v.name));
                                }
                            }
                        } else {
                            return Err(format!(
                                "This formula is per {} but '{}' is per {}; use TOTAL({}) or {}[item]",
                                g.name, v.name, set.name, v.name, v.name
                            ));
                        }
                    }
                };
                out.push_str(&src[last..span.start]);
                out.push_str(&quoted(&flat_name(&v.name, &item)));
                last = span.end;
            }
        }
        i += 1;
    }
    out.push_str(&src[last..]);
    Ok(out)
}

/// Does the model use sets at all?
pub fn uses_sets(m: &Model) -> bool {
    !m.sets.is_empty() || m.vars.iter().any(|v| v.set.is_some())
}

pub fn expand(m: &Model) -> Result<Expanded, Problems> {
    let mut problems: Problems = Vec::new();
    let mut sets = HashMap::new();
    for s in &m.sets {
        let n = expr::normalize(&s.name);
        if n.is_empty() {
            problems.push((None, "A set has no name".into()));
        } else if sets.insert(n, s).is_some() {
            problems.push((None, format!("Two sets are called '{}'", s.name)));
        }
        let mut seen: Vec<String> = Vec::new();
        for it in &s.items {
            let k = expr::normalize(it);
            if seen.contains(&k) {
                problems.push((None, format!("The set {} lists '{it}' twice", s.name)));
            }
            seen.push(k);
        }
    }
    let vars: HashMap<String, &Var> = m.vars.iter().map(|v| (expr::normalize(&v.name), v)).collect();
    let env = Env { sets, vars };
    for v in &m.vars {
        if let Some(s) = &v.set {
            if !env.sets.contains_key(&expr::normalize(s)) {
                problems.push((Some(v.name.clone()), format!("is per '{s}', but there is no set with that name")));
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    // ids of the copies
    let mut next = m.vars.iter().map(|v| v.id).max().unwrap_or(0) + 1;
    let mut copy_id: HashMap<(u64, usize), u64> = HashMap::new();
    for v in &m.vars {
        if let Some(set) = env.set_of(v) {
            for k in 0..set.items.len() {
                copy_id.insert((v.id, k), next);
                next += 1;
            }
        }
    }
    let by_id: HashMap<u64, &Var> = m.vars.iter().map(|v| (v.id, v)).collect();
    // which end of a flow a copy connects to
    let endpoint = |flow: &Var, end: Option<u64>, g: &ItemSet, gi: usize| -> Result<Option<u64>, String> {
        let Some(id) = end else { return Ok(None) };
        let Some(sv) = by_id.get(&id) else { return Ok(None) };
        let Some(sset) = env.set_of(sv) else { return Ok(Some(id)) };
        if expr::normalize(&sset.name) == expr::normalize(&g.name) {
            return Ok(copy_id.get(&(id, gi)).copied());
        }
        if g.parent.as_ref().is_some_and(|p| expr::normalize(p) == expr::normalize(&sset.name)) {
            if let Some(k) = g.parent_item(gi).and_then(|p| sset.items.iter().position(|x| x == p)) {
                return Ok(copy_id.get(&(id, k)).copied());
            }
        }
        Err(format!("{} is per {} but the stock '{}' it connects to is per {}", flow.name, g.name, sv.name, sset.name))
    };
    let mut flat = m.clone();
    flat.vars.clear();
    let mut rollups = Vec::new();
    for v in &m.vars {
        match env.set_of(v) {
            None => {
                let mut c = v.clone();
                c.by_item.clear();
                if !c.equation.trim().is_empty() {
                    match rewrite(&c.equation, Ctx::Scalar, &env) {
                        Ok(e) => c.equation = e,
                        Err(e) => problems.push((Some(v.name.clone()), e)),
                    }
                }
                flat.vars.push(c);
            }
            Some(set) => {
                let mut parts = Vec::new();
                for (k, item) in set.items.iter().enumerate() {
                    let ov = v.by_item.iter().find(|o| expr::normalize(&o.item) == expr::normalize(item));
                    let mut c = v.clone();
                    c.id = copy_id[&(v.id, k)];
                    c.name = flat_name(&v.name, item);
                    c.set = None;
                    c.by_item.clear();
                    if let Some(o) = ov {
                        if !o.equation.trim().is_empty() {
                            c.equation = o.equation.clone();
                        }
                        if !o.table.is_empty() {
                            c.table = o.table.clone();
                        }
                    }
                    // the "all items" lever scale
                    if v.kind == Kind::Constant && (v.lever_scale - 1.0).abs() > 1e-12 && !c.equation.trim().is_empty() {
                        c.equation = format!("({}) * {}", c.equation, v.lever_scale);
                    }
                    if !c.equation.trim().is_empty() {
                        match rewrite(&c.equation, Ctx::Item(set, k), &env) {
                            Ok(e) => c.equation = e,
                            Err(e) => problems.push((Some(c.name.clone()), e)),
                        }
                    }
                    if v.kind == Kind::Flow {
                        match endpoint(v, v.from, set, k) {
                            Ok(x) => c.from = x,
                            Err(e) => problems.push((Some(v.name.clone()), e)),
                        }
                        match endpoint(v, v.to, set, k) {
                            Ok(x) => c.to = x,
                            Err(e) => problems.push((Some(v.name.clone()), e)),
                        }
                    }
                    parts.push(c.name.clone());
                    flat.vars.push(c);
                }
                rollups.push(Rollup { name: v.name.clone(), parts, average: v.rollup == RollKind::Average });
            }
        }
    }
    if !problems.is_empty() {
        // the same message for every item is noise: keep the first of each
        let mut seen: Vec<String> = Vec::new();
        problems.retain(|(_, m)| {
            if seen.contains(m) {
                false
            } else {
                seen.push(m.clone());
                true
            }
        });
        return Err(problems);
    }
    flat.sets.clear();
    Ok(Expanded { model: flat, rollups })
}

/// Which item of each set the person is looking at: set name → item (or group) name.
pub type Focus = HashMap<String, String>;

/// The item (or group) a set variable is being shown for, if any.
pub fn focus_label(m: &Model, v: &Var, focus: &Focus) -> Option<String> {
    let sn = v.set.as_ref()?;
    let set = m.sets.iter().find(|s| &s.name == sn)?;
    if let Some(i) = focus.get(sn).filter(|i| set.items.contains(i)) {
        return Some(i.clone());
    }
    let parent = set.parent.as_ref()?;
    focus.get(parent).filter(|g| set.parent_of.contains(g)).cloned()
}

/// Make a run show what the person asked to look at: a set variable's usual series becomes the
/// chosen item's series, or the total of a chosen group's items. The per-item series stay too.
pub fn apply_focus(run: &mut crate::sim::Run, m: &Model, focus: &Focus) {
    if focus.is_empty() {
        return;
    }
    for v in m.vars.iter().filter(|v| v.set.is_some()) {
        let Some(set) = m.sets.iter().find(|s| Some(&s.name) == v.set.as_ref()) else { continue };
        let key = expr::normalize(&v.name);
        if let Some(item) = focus.get(&set.name).filter(|i| set.items.contains(i)) {
            if let Some(x) = run.series.get(&expr::normalize(&flat_name(&v.name, item))).cloned() {
                run.series.insert(key, x);
            }
            continue;
        }
        let Some(group) = set.parent.as_ref().and_then(|p| focus.get(p)).filter(|g| set.parent_of.contains(g)) else { continue };
        let parts: Vec<&Vec<f64>> = (0..set.items.len())
            .filter(|&i| set.parent_item(i) == Some(group.as_str()))
            .filter_map(|i| run.series.get(&expr::normalize(&flat_name(&v.name, &set.items[i]))))
            .collect();
        if parts.is_empty() {
            continue;
        }
        let mut sum = vec![0.0; run.times.len()];
        for p in &parts {
            for (a, b) in sum.iter_mut().zip(p.iter()) {
                *a += *b;
            }
        }
        if v.rollup == RollKind::Average {
            let n = parts.len() as f64;
            sum.iter_mut().for_each(|a| *a /= n);
        }
        run.series.insert(key, sum);
    }
}

use crate::app::App;

impl App {
    /// The "Showing" bar: one dropdown per set. Pick a tank and the whole model — numbers on the
    /// diagram, graphs, sliders — is about that tank; pick a region and it is about its tanks.
    pub fn focus_bar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let sets: Vec<ItemSet> = self.model.sets.clone();
        let mut changed = false;
        ui.horizontal_centered(|ui| {
            ui.label(egui::RichText::new("Showing").small().color(pal.muted));
            // groups first (Region), then the sets that belong to them (Tank)
            let mut order: Vec<&ItemSet> = sets.iter().filter(|s| s.parent.is_none()).collect();
            order.extend(sets.iter().filter(|s| s.parent.is_some()));
            for set in order {
                let cur = self.focus.get(&set.name).cloned();
                // only the items of the chosen group when the set belongs to one
                let allowed: Vec<&String> = (0..set.items.len())
                    .filter(|&i| match (&set.parent, set.parent.as_ref().and_then(|p| self.focus.get(p))) {
                        (Some(_), Some(g)) => set.parent_item(i) == Some(g.as_str()),
                        _ => true,
                    })
                    .map(|i| &set.items[i])
                    .collect();
                ui.label(egui::RichText::new(&set.name).color(pal.text));
                egui::ComboBox::from_id_salt(("focus", set.id))
                    .width(190.0)
                    .selected_text(cur.clone().unwrap_or_else(|| format!("All ({})", allowed.len())))
                    .show_ui(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                            if ui.selectable_label(cur.is_none(), "All (totals)").clicked() {
                                self.focus.remove(&set.name);
                                changed = true;
                            }
                            for it in allowed {
                                if ui.selectable_label(cur.as_ref() == Some(it), it).clicked() {
                                    self.focus.insert(set.name.clone(), it.clone());
                                    // choosing a tank also shows its region
                                    if let Some(i) = set.items.iter().position(|x| x == it) {
                                        if let (Some(p), Some(g)) = (&set.parent, set.parent_item(i)) {
                                            self.focus.insert(p.clone(), g.to_string());
                                        }
                                    }
                                    changed = true;
                                }
                            }
                        });
                    });
            }
            if !self.focus.is_empty() && crate::icons::tool(ui, crate::icons::Ic::Close, "Back to totals", pal.muted, pal.accent).clicked() {
                self.focus.clear();
                changed = true;
            }
            ui.label(egui::RichText::new(if self.focus.is_empty() { "— each box shows the total of its items" } else { "— numbers, graphs and sliders are for this selection" }).small().color(pal.muted));
        });
        if changed {
            // a region change can leave a tank from another region selected
            let sets = self.model.sets.clone();
            for set in &sets {
                if let (Some(item), Some(p)) = (self.focus.get(&set.name).cloned(), &set.parent) {
                    if let (Some(g), Some(i)) = (self.focus.get(p).cloned(), set.items.iter().position(|x| *x == item)) {
                        if set.parent_item(i) != Some(g.as_str()) {
                            self.focus.remove(&set.name);
                        }
                    }
                }
            }
            self.simulate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Settings, Var};

    fn add(m: &mut Model, name: &str, kind: Kind, eq: &str, set: Option<&str>) -> u64 {
        let id = m.new_id();
        let mut v = Var::new(id, name, kind, [0.0, 0.0]);
        v.equation = eq.into();
        v.set = set.map(|s| s.to_string());
        m.vars.push(v);
        id
    }

    /// Two regions, three tanks: a stock per tank filled at a per-tank rate, a total per region.
    fn model() -> Model {
        let mut m = Model::default();
        m.settings = Settings { initial_time: 0.0, final_time: 4.0, time_step: 1.0, time_units: "Day".into(), method: crate::model::Method::Euler };
        m.sets.push(ItemSet { id: 1, name: "Region".into(), items: vec!["North".into(), "South".into()], ..Default::default() });
        m.sets.push(ItemSet {
            id: 2,
            name: "Tank".into(),
            items: vec!["A".into(), "B".into(), "C".into()],
            parent: Some("Region".into()),
            parent_of: vec!["North".into(), "North".into(), "South".into()],
            note: String::new(),
        });
        let stock = add(&mut m, "Level", Kind::Stock, "10", Some("Tank"));
        let flow = add(&mut m, "Fill", Kind::Flow, "Rate * Bonus", Some("Tank"));
        m.var_mut(flow).unwrap().to = Some(stock);
        let rate = add(&mut m, "Rate", Kind::Constant, "1", Some("Tank"));
        m.var_mut(rate).unwrap().by_item.push(crate::model::ItemValue { item: "B".into(), equation: "2".into(), table: vec![] });
        add(&mut m, "Bonus", Kind::Constant, "1", Some("Region"));
        m.vars.iter_mut().find(|v| v.name == "Bonus").unwrap().by_item.push(crate::model::ItemValue { item: "South".into(), equation: "3".into(), table: vec![] });
        add(&mut m, "Region level", Kind::Aux, "TOTAL(Level)", Some("Region"));
        add(&mut m, "All level", Kind::Aux, "TOTAL(Level)", None);
        add(&mut m, "North only", Kind::Aux, "TOTAL(Level, North)", None);
        add(&mut m, "B level", Kind::Aux, "Level[B]", None);
        add(&mut m, "Mean level", Kind::Aux, "AVERAGE(Level)", None);
        m
    }

    #[test]
    fn expands_and_totals() {
        let m = model();
        let c = crate::sim::compile(&m).map_err(|p| p.iter().map(|p| format!("{:?}: {}", p.var, p.message)).collect::<Vec<_>>()).unwrap();
        let r = crate::sim::simulate(&c, &m, "t").unwrap();
        let last = |n: &str| *r.get(n).unwrap_or_else(|| panic!("no series {n}")).last().unwrap();
        // after 4 days: A +1/day, B +2/day, C +3/day (South bonus 3)
        assert_eq!(last("Level[A]"), 14.0);
        assert_eq!(last("Level[B]"), 18.0);
        assert_eq!(last("Level[C]"), 22.0);
        assert_eq!(last("Level"), 54.0, "the set variable shows the total");
        assert_eq!(last("All level"), 54.0);
        assert_eq!(last("North only"), 32.0);
        assert_eq!(last("B level"), 18.0);
        assert_eq!(last("Region level[North]"), 32.0);
        assert_eq!(last("Region level[South]"), 22.0);
        assert_eq!(last("Mean level"), 18.0);
    }

    #[test]
    fn one_item_can_be_plotted_by_name() {
        let m = model();
        assert_eq!(m.by_name("level[b]").map(|v| v.name.as_str()), Some("Level"));
        assert!(m.by_name("rate[nowhere]").is_some(), "only the variable is looked up, not the item");
        assert!(m.by_name("nothing[b]").is_none());
        assert_eq!(m.items_of(m.by_name("level").unwrap()), vec!["A", "B", "C"]);
        let c = crate::sim::compile(&m).ok().unwrap();
        let r = crate::sim::simulate(&c, &m, "t").unwrap();
        assert!(r.get("Level[B]").is_some() && r.get("level[b]").is_some());
    }

    #[test]
    fn focus_shows_one_item_or_one_group() {
        let m = model();
        let c = crate::sim::compile(&m).ok().unwrap();
        let base = crate::sim::simulate(&c, &m, "t").unwrap();
        let last = |r: &crate::sim::Run, n: &str| *r.get(n).unwrap().last().unwrap();
        let mut one = base.clone();
        apply_focus(&mut one, &m, &[("Tank".to_string(), "B".to_string())].into_iter().collect());
        assert_eq!(last(&one, "Level"), 18.0, "one tank");
        let mut group = base.clone();
        apply_focus(&mut group, &m, &[("Region".to_string(), "North".to_string())].into_iter().collect());
        assert_eq!(last(&group, "Level"), 32.0, "the region's tanks together");
        assert_eq!(last(&group, "Region level"), 32.0, "a per-Region variable shows the chosen region");
        assert_eq!(last(&base, "Level"), 54.0);
        let v = m.by_name("level").unwrap();
        assert_eq!(focus_label(&m, v, &[("Tank".to_string(), "C".to_string())].into_iter().collect()), Some("C".into()));
    }

    #[test]
    fn a_lever_scale_moves_every_item() {
        let mut m = model();
        let base = {
            let c = crate::sim::compile(&m).ok().unwrap();
            *crate::sim::simulate(&c, &m, "t").unwrap().get("Level").unwrap().last().unwrap()
        };
        let rate = m.vars.iter_mut().find(|v| v.name == "Rate").unwrap();
        rate.lever = true;
        rate.lever_scale = 2.0;
        let c = crate::sim::compile(&m).ok().unwrap();
        let more = *crate::sim::simulate(&c, &m, "t").unwrap().get("Level").unwrap().last().unwrap();
        // start 3 x 10; filled 4 days at rates 1, 2, 1 (x bonus 1,1,3) -> doubling Rate doubles what is added
        assert_eq!(base, 54.0);
        assert_eq!(more, 30.0 + (54.0 - 30.0) * 2.0);
    }

    #[test]
    fn explains_mistakes() {
        let mut m = model();
        add(&mut m, "Bad", Kind::Aux, "Level + 1", None);
        let e = crate::sim::compile(&m).err().unwrap();
        assert!(e.iter().any(|p| p.message.contains("TOTAL(Level)")), "{:?}", e.iter().map(|p| &p.message).collect::<Vec<_>>());
        let mut m = model();
        add(&mut m, "Bad2", Kind::Aux, "TOTAL(Level, Nowhere)", None);
        assert!(crate::sim::compile(&m).is_err());
    }
}
