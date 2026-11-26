//! `dyn attribute OLD NEW`: why the goals changed between two versions of a model.
//!
//! The edits between the versions are grouped, one group per changed variable. For every goal of the manifest the change
//! (new value minus old value) is split between the groups by the Shapley value (Shorrocks 2013): a group's share is its
//! average effect over every order in which the groups could have been applied to the old model. The shares add up to the
//! whole change exactly. Up to 10 groups every one of the 2^k combinations is run once (cached). Above that, permutations are
//! sampled with a fixed seed; each permutation adds up to the whole change by itself, so the sampled shares still add up
//! exactly, and the standard error of every share is reported.
//!
//! Only edits that can change a run are players: a changed equation, constant, lookup, flow end, kind, set value, an added or
//! removed variable, and the run settings. A changed comment, unit label, position, check or manifest entry cannot move the
//! base run; they are listed as "no effect on the goals" and take no part in the split.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::diff::{self, Change};
use crate::expr;
use crate::manifest::{self, Direction, Goal};
use crate::model::{Kind, Model, Var};
use crate::stats::Rng;

/// One thing that was done to the old model to make the new one.
#[derive(Clone, Debug)]
enum Edit {
    /// A variable that exists in both, taken from the new model.
    Changed(String),
    Added(String),
    Removed(String),
    Setting(&'static str),
    /// Sets, functions, modules and their instances.
    Pieces,
}

#[derive(Clone, Debug, Serialize)]
pub struct GroupInfo {
    pub name: String,
    pub edits: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Part {
    pub group: String,
    /// the group's Shapley share of the change in the goal
    pub share: f64,
    /// share of the whole change, in percent (None when the goal did not move)
    pub share_pct: Option<f64>,
    /// what applying this group alone to the old model does
    pub alone: f64,
    /// share minus alone: what the group adds or loses through combining with the others
    pub interaction: f64,
    /// standard error of the share (0 when exact)
    pub se: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct GoalResult {
    pub goal: String,
    pub output: String,
    pub measure: String,
    pub direction: String,
    pub old: Option<f64>,
    pub new: Option<f64>,
    pub change: Option<f64>,
    pub change_pct: Option<f64>,
    pub parts: Vec<Part>,
    /// new (read from the new model) minus the value the groups reach together; 0 when the groups explain every edit
    pub unexplained: Option<f64>,
    /// the largest gap between the parts' sum and the change, relative to the change
    pub sum_gap: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Attribution {
    pub old: String,
    pub new: String,
    pub groups: Vec<GroupInfo>,
    pub no_effect: Vec<String>,
    pub goals: Vec<GoalResult>,
    /// "exact" or "sampled"
    pub method: String,
    pub permutations: usize,
    pub seed: u64,
    pub runs: usize,
    /// what was done when the edits could not be run one by one
    pub notes: Vec<String>,
}

struct Group {
    name: String,
    edits: Vec<Edit>,
    text: Vec<String>,
}

fn key(v: &Var) -> String {
    expr::normalize(&v.name)
}

/// What a run can notice of a variable: everything except its comment, units text and place on the page.
fn essence(m: &Model, v: &Var) -> String {
    let end = |id: Option<u64>| id.and_then(|i| m.var(i)).map(|x| expr::normalize(&x.name));
    format!("{:?}|{}|{:?}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}", v.kind, v.equation.trim(), v.table, v.non_negative, end(v.from), end(v.to), v.set, v.by_item, v.rollup, v.lever_scale)
}

fn new_equation_reads(v: &Var, names: &[String]) -> bool {
    let Ok(e) = expr::parse(&v.equation) else { return false };
    let mut r = Vec::new();
    expr::references(&e, &mut r);
    r.iter().any(|x| names.contains(x))
}

fn make_groups(a: &Model, b: &Model, lump: bool) -> (Vec<Group>, Vec<String>) {
    let d = diff::diff_models(a, b);
    let described = |name: &str| -> Vec<String> { d.iter().filter(|x| x.what == "variable" && expr::normalize(&x.name) == expr::normalize(name)).map(|x| if x.detail.is_empty() { format!("{:?}", x.change).to_lowercase() } else { x.detail.clone() }).collect() };
    let (am, bm): (BTreeMap<String, &Var>, BTreeMap<String, &Var>) = (a.vars.iter().map(|v| (key(v), v)).collect(), b.vars.iter().map(|v| (key(v), v)).collect());
    let mut groups: Vec<Group> = Vec::new();
    let mut no_effect = Vec::new();
    let mut structural: Vec<Edit> = Vec::new();
    let mut structural_text: Vec<String> = Vec::new();
    let added: Vec<String> = bm.keys().filter(|k| !am.contains_key(*k)).cloned().collect();
    let removed: Vec<String> = am.keys().filter(|k| !bm.contains_key(*k)).cloned().collect();
    for (k, y) in &bm {
        match am.get(k) {
            Some(x) => {
                if essence(a, x) == essence(b, y) {
                    if x.name != y.name || x.units != y.units || x.comment != y.comment {
                        no_effect.push(format!("{} (name, units or comment only)", y.name));
                    }
                    continue;
                }
                let reads_new = lump && (new_equation_reads(y, &added) || new_equation_reads(x, &removed));
                if reads_new {
                    structural.push(Edit::Changed(k.clone()));
                    structural_text.extend(described(&y.name).into_iter().map(|t| format!("{}: {t}", y.name)));
                } else {
                    groups.push(Group { name: y.name.clone(), edits: vec![Edit::Changed(k.clone())], text: described(&y.name) });
                }
            }
            None => {
                let t = format!("{}: added {} {}", y.name, y.kind.label(), y.equation.trim());
                if lump {
                    structural.push(Edit::Added(k.clone()));
                    structural_text.push(t);
                } else {
                    groups.push(Group { name: format!("{} (added)", y.name), edits: vec![Edit::Added(k.clone())], text: vec![t] });
                }
            }
        }
    }
    for (k, x) in &am {
        if !bm.contains_key(k) {
            let t = format!("{}: removed", x.name);
            if lump {
                structural.push(Edit::Removed(k.clone()));
                structural_text.push(t);
            } else {
                groups.push(Group { name: format!("{} (removed)", x.name), edits: vec![Edit::Removed(k.clone())], text: vec![t] });
            }
        }
    }
    let (s, t) = (&a.settings, &b.settings);
    for (name, differs) in [("initial_time", s.initial_time != t.initial_time), ("final_time", s.final_time != t.final_time), ("time_step", s.time_step != t.time_step), ("method", s.method != t.method)] {
        if differs {
            let text = d.iter().filter(|x| x.what == "setting" && x.name == name).map(|x| x.detail.clone()).collect();
            groups.push(Group { name: format!("Setting {name}"), edits: vec![Edit::Setting(name)], text });
        }
    }
    if format!("{:?}", a.sets) != format!("{:?}", b.sets) || a.functions != b.functions || a.defs != b.defs || a.instances != b.instances {
        let text: Vec<String> = d.iter().filter(|x| matches!(x.what.as_str(), "set" | "function" | "module" | "instance")).map(|x| format!("{} {}: {}", x.what, x.name, format!("{:?}", x.change).to_lowercase())).collect();
        groups.push(Group { name: "Sets, functions and modules".into(), edits: vec![Edit::Pieces], text });
    }
    if !structural.is_empty() {
        groups.push(Group { name: "Added, removed or renamed variables, and what reads them".into(), edits: structural, text: structural_text });
    }
    // the manifest, checks and the like cannot move the base run
    for x in d.iter().filter(|x| x.what.starts_with("manifest") || x.what == "check") {
        no_effect.push(format!("{} {}", x.what, x.name));
    }
    let _ = Change::Added;
    (groups, no_effect)
}

fn id_by_name(m: &Model, name: &str) -> Option<u64> {
    m.by_name(name).map(|v| v.id)
}

/// The old model with these edits made, taking what is needed from the new one.
fn apply(old: &Model, new: &Model, edits: &[Edit]) -> Model {
    let mut m = old.clone();
    for e in edits {
        match e {
            Edit::Changed(k) | Edit::Added(k) => {
                let Some(src) = new.by_name(k) else { continue };
                let mut nv = src.clone();
                nv.from = nv.from.and_then(|i| new.var(i)).and_then(|s| id_by_name(&m, &expr::normalize(&s.name)));
                nv.to = nv.to.and_then(|i| new.var(i)).and_then(|s| id_by_name(&m, &expr::normalize(&s.name)));
                match m.vars.iter().position(|v| key(v) == *k) {
                    Some(i) => {
                        let o = &m.vars[i];
                        nv.id = o.id;
                        nv.pos = o.pos;
                        nv.from_pt = o.from_pt;
                        nv.to_pt = o.to_pt;
                        nv.view = o.view;
                        nv.layer = o.layer;
                        m.vars[i] = nv;
                    }
                    None => {
                        nv.id = m.new_id();
                        m.vars.push(nv);
                    }
                }
            }
            Edit::Removed(k) => {
                let Some(id) = m.by_name(k).map(|v| v.id) else { continue };
                m.vars.retain(|v| v.id != id);
                m.links.retain(|l| l.from != id && l.to != id);
                for v in m.vars.iter_mut().filter(|v| v.kind == Kind::Flow) {
                    if v.from == Some(id) {
                        v.from = None;
                    }
                    if v.to == Some(id) {
                        v.to = None;
                    }
                }
            }
            Edit::Setting(n) => match *n {
                "initial_time" => m.settings.initial_time = new.settings.initial_time,
                "final_time" => m.settings.final_time = new.settings.final_time,
                "time_step" => m.settings.time_step = new.settings.time_step,
                _ => m.settings.method = new.settings.method,
            },
            Edit::Pieces => {
                m.sets = new.sets.clone();
                m.functions = new.functions.clone();
                m.defs = new.defs.clone();
                m.instances = new.instances.clone();
            }
        }
    }
    m
}

fn values(m: &Model, goals: &[Goal]) -> Result<Vec<f64>, String> {
    let run = crate::runner::run_with(m, &[])?;
    goals.iter().map(|g| manifest::goal_value_raw(&run, g).filter(|v| v.is_finite()).ok_or_else(|| format!("goal “{}” cannot be read from the run", g.name))).collect()
}

struct Game<'a> {
    old: &'a Model,
    new: &'a Model,
    groups: &'a [Group],
    goals: &'a [Goal],
    cache: HashMap<u64, Result<Vec<f64>, String>>,
}

impl Game<'_> {
    fn v(&mut self, mask: u64) -> Result<Vec<f64>, String> {
        if let Some(r) = self.cache.get(&mask) {
            return r.clone();
        }
        let edits: Vec<Edit> = self.groups.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).flat_map(|(_, g)| g.edits.clone()).collect();
        let r = values(&apply(self.old, self.new, &edits), self.goals);
        self.cache.insert(mask, r.clone());
        r
    }
}

fn factorial(n: usize) -> f64 {
    (1..=n).map(|x| x as f64).product()
}

/// Split the change in every goal of `new`'s manifest between the edit groups.
pub fn run(old: &Model, new: &Model, permutations: usize, seed: u64, names: (&str, &str)) -> Result<Attribution, String> {
    let goals = crate::ablate::goals_of(new);
    if goals.is_empty() {
        return Err("the new model has no goals, outputs or stocks to read".into());
    }
    let old_vals = values(old, &goals).map_err(|e| format!("the old model: {e}"))?;
    let new_vals = values(new, &goals).map_err(|e| format!("the new model: {e}"))?;
    let mut notes = Vec::new();
    let (mut groups, mut no_effect) = make_groups(old, new, false);
    // the coalitions must all run; when edits depend on each other (a variable that reads an added one) they are one group
    let probe_ok = |groups: &[Group]| -> bool {
        let k = groups.len().min(10);
        // every group alone and all together (the others are mixtures of these; the full game checks the rest as it goes)
        (0..k).all(|i| values(&apply(old, new, &groups[i].edits), &goals).is_ok()) && values(&apply(old, new, &groups.iter().flat_map(|g| g.edits.clone()).collect::<Vec<_>>()), &goals).is_ok()
    };
    if !groups.is_empty() && !probe_ok(&groups) {
        let (g2, n2) = make_groups(old, new, true);
        notes.push("some edits cannot be run one at a time (a variable was added, removed or renamed, and others read it), so those edits form one group".to_string());
        groups = g2;
        no_effect = n2;
    }
    if groups.len() > 62 {
        return Err(format!("{} groups of edits: too many to split (the limit is 62); compare smaller steps", groups.len()));
    }
    let k = groups.len();
    let full: u64 = if k == 0 { 0 } else { (1u64 << k) - 1 };
    let mut game = Game { old, new, groups: &groups, goals: &goals, cache: HashMap::new() };
    let base = game.v(0)?;
    let all = game.v(full).map_err(|e| format!("the old model with every edit made does not run: {e}"))?;
    let exact = k <= 10;
    let n_perm = if exact { 0 } else { permutations.max(2) };
    // shares[goal][group] and the squared sums for the standard error
    let ng = goals.len();
    let mut shares = vec![vec![0.0f64; k]; ng];
    let mut ses = vec![vec![0.0f64; k]; ng];
    if exact {
        let weights: Vec<f64> = (0..k).map(|s| factorial(s) * factorial(k - s - 1) / factorial(k)).collect();
        for i in 0..k {
            for mask in 0..(1u64 << k) {
                if mask >> i & 1 == 1 {
                    continue;
                }
                let (without, with) = (game.v(mask)?, game.v(mask | 1 << i)?);
                let w = weights[mask.count_ones() as usize];
                for g in 0..ng {
                    shares[g][i] += w * (with[g] - without[g]);
                }
            }
        }
    } else {
        let mut rng = Rng::new(seed);
        let mut sum = vec![vec![0.0f64; k]; ng];
        let mut sq = vec![vec![0.0f64; k]; ng];
        for _ in 0..n_perm {
            let mut order: Vec<usize> = (0..k).collect();
            for i in (1..k).rev() {
                let j = (rng.next_u64() % (i as u64 + 1)) as usize;
                order.swap(i, j);
            }
            let (mut mask, mut prev) = (0u64, base.clone());
            for &i in &order {
                mask |= 1 << i;
                let now = game.v(mask)?;
                for g in 0..ng {
                    let d = now[g] - prev[g];
                    sum[g][i] += d;
                    sq[g][i] += d * d;
                }
                prev = now;
            }
        }
        let n = n_perm as f64;
        for g in 0..ng {
            for i in 0..k {
                let mean = sum[g][i] / n;
                shares[g][i] = mean;
                let var = ((sq[g][i] - n * mean * mean) / (n - 1.0)).max(0.0);
                ses[g][i] = (var / n).sqrt();
            }
        }
    }
    let mut alone = vec![vec![0.0f64; k]; ng];
    for i in 0..k {
        let one = game.v(1 << i)?;
        for g in 0..ng {
            alone[g][i] = one[g] - base[g];
        }
    }
    let runs = game.cache.len() + 2;
    let mut results = Vec::new();
    for (g, goal) in goals.iter().enumerate() {
        let total = all[g] - base[g];
        let parts: Vec<Part> = (0..k).map(|i| Part { group: groups[i].name.clone(), share: shares[g][i], share_pct: if total.abs() > 1e-12 { Some(100.0 * shares[g][i] / total) } else { None }, alone: alone[g][i], interaction: shares[g][i] - alone[g][i], se: ses[g][i] }).collect();
        let sum: f64 = parts.iter().map(|p| p.share).sum();
        let change = new_vals[g] - old_vals[g];
        results.push(GoalResult {
            goal: goal.name.clone(),
            output: goal.output.clone(),
            measure: if goal.measure.is_empty() { "final".into() } else { goal.measure.clone() },
            direction: if goal.direction.is_empty() { "max".into() } else { goal.direction.clone() },
            old: Some(old_vals[g]),
            new: Some(new_vals[g]),
            change: Some(change),
            change_pct: if old_vals[g].abs() > 1e-12 { Some(100.0 * change / old_vals[g].abs()) } else { None },
            parts,
            unexplained: Some(new_vals[g] - all[g]),
            sum_gap: Some((sum - total).abs() / total.abs().max(1e-12).max(if total == 0.0 { 1.0 } else { 0.0 })),
        });
    }
    let _ = Direction::Max;
    Ok(Attribution { old: names.0.into(), new: names.1.into(), groups: groups.iter().map(|g| GroupInfo { name: g.name.clone(), edits: g.text.clone() }).collect(), no_effect, goals: results, method: if exact { "exact".into() } else { "sampled".into() }, permutations: n_perm, seed, runs, notes })
}

fn says(g: &GoalResult) -> String {
    let n = crate::doctor::num;
    match (g.old, g.new, g.change) {
        (Some(o), Some(nw), Some(c)) => {
            let head = match g.change_pct {
                Some(p) => format!("{} went from {} to {} ({:+.1}%)", g.goal, n(o), n(nw), p),
                None => format!("{} went from {} to {} ({:+})", g.goal, n(o), n(nw), n(c)),
            };
            let mut parts: Vec<&Part> = g.parts.iter().filter(|p| p.share.abs() > 1e-12).collect();
            parts.sort_by(|a, b| b.share.abs().partial_cmp(&a.share.abs()).unwrap_or(std::cmp::Ordering::Equal));
            match parts.first() {
                Some(p) if c.abs() > 1e-12 => format!("{head}. The largest single effect is {} ({}, {} of the whole change; a share above 100% means other edits pulled the other way).", p.group, n(p.share), p.share_pct.map_or("n/a".into(), |x| format!("{x:.0}%"))),
                _ => format!("{head}."),
            }
        }
        _ => format!("{} cannot be read.", g.goal),
    }
}

/// The result as Markdown: one short sentence and one table per goal.
pub fn markdown(a: &Attribution) -> String {
    let n = crate::doctor::num;
    let mut s = format!("# Why the goals changed\n\nOld: {}. New: {}.\n\n", a.old, a.new);
    if a.groups.is_empty() {
        s.push_str("No edit between the two versions can change a run, so no goal moved.\n");
    } else {
        s.push_str(&format!("{} groups of edits, split by the Shapley value ({}{}). {} runs.\n\n", a.groups.len(), a.method, if a.method == "sampled" { format!(", {} permutations, seed {}", a.permutations, a.seed) } else { String::new() }, a.runs));
    }
    for t in &a.notes {
        s.push_str(&format!("Note: {t}.\n\n"));
    }
    for g in &a.goals {
        s.push_str(&format!("## {}\n\n{}\n\n", g.goal, says(g)));
        if g.parts.is_empty() {
            continue;
        }
        s.push_str("| Edit | Share of the change | Percent | Alone | Through others |");
        if a.method == "sampled" {
            s.push_str(" Standard error |");
        }
        s.push_str("\n|---|---|---|---|---|");
        if a.method == "sampled" {
            s.push_str("---|");
        }
        s.push('\n');
        let mut parts: Vec<&Part> = g.parts.iter().collect();
        parts.sort_by(|x, y| y.share.abs().partial_cmp(&x.share.abs()).unwrap_or(std::cmp::Ordering::Equal));
        for p in parts {
            s.push_str(&format!("| {} | {} | {} | {} | {} |", p.group.replace('|', "/"), n(p.share), p.share_pct.map_or("-".into(), |x| format!("{x:.1}%")), n(p.alone), n(p.interaction)));
            if a.method == "sampled" {
                s.push_str(&format!(" {} |", n(p.se)));
            }
            s.push('\n');
        }
        let sum: f64 = g.parts.iter().map(|p| p.share).sum();
        s.push_str(&format!("\nThe shares add up to {} (the change is {}).", n(sum), g.change.map_or("n/a".into(), n)));
        if let Some(u) = g.unexplained.filter(|u| u.abs() > 1e-9 * g.new.unwrap_or(1.0).abs().max(1.0)) {
            s.push_str(&format!(" The groups together reach {}, {} short of the new model's own value: something in the new model is not in any group.", n(g.new.unwrap_or(0.0) - u), n(u)));
        }
        s.push_str("\n\n");
    }
    if !a.groups.is_empty() {
        s.push_str("## The edits in each group\n\n");
        for g in &a.groups {
            s.push_str(&format!("- **{}**: {}\n", g.name, if g.edits.is_empty() { "see the difference list".to_string() } else { g.edits.join("; ") }));
        }
        s.push('\n');
    }
    if !a.no_effect.is_empty() {
        s.push_str("## No effect on the goals\n\nThese differ between the versions but cannot change a run: ");
        s.push_str(&a.no_effect.join("; "));
        s.push_str(".\n");
    }
    s
}

/// A waterfall for one goal, as SVG.
pub fn waterfall_svg(a: &Attribution, goal: &GoalResult) -> String {
    let mut steps: Vec<(String, f64)> = goal.parts.iter().filter(|p| p.share.abs() > 1e-12).map(|p| (p.group.clone(), p.share)).collect();
    steps.sort_by(|x, y| y.1.abs().partial_cmp(&x.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
    let (o, nw) = (goal.old.unwrap_or(f64::NAN), goal.new.unwrap_or(f64::NAN));
    let (old_label, new_label) = (format!("Old: {}", short(&a.old)), format!("New: {}", short(&a.new)));
    let unexplained = goal.unexplained.unwrap_or(0.0);
    if unexplained.abs() > 1e-9 * nw.abs().max(1.0) {
        steps.push(("not in any group".into(), unexplained));
    }
    crate::plotsvg::waterfall(&format!("Why {} changed ({})", goal.goal, goal.measure), &old_label, o, &steps, &new_label, nw)
}

fn short(p: &str) -> String {
    std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Goal;
    use crate::ops::{Op, Session};

    fn base(extra: &str) -> Model {
        let mut s = Session::new(Model::default());
        let ops: Vec<Op> = serde_json::from_str(&format!(
            r#"[{{"op":"add_var","name":"Tank","kind":"stock","equation":"0","units":"MB"}},
            {{"op":"add_var","name":"A","kind":"constant","equation":"1"}},
            {{"op":"add_var","name":"B","kind":"constant","equation":"1"}},
            {{"op":"add_var","name":"C","kind":"constant","equation":"0"}},
            {{"op":"add_var","name":"Fill","kind":"flow","equation":"{extra}","to":"Tank","units":"MB/Day"}}]"#
        ))
        .unwrap();
        s.apply("t", &ops).unwrap();
        let mut m = s.model;
        m.manifest.goals.push(Goal { name: "Tank at the end".into(), output: "Tank".into(), measure: "final".into(), direction: "max".into(), ..Default::default() });
        m
    }

    fn set(m: &Model, name: &str, eq: &str) -> Model {
        let mut c = m.clone();
        assert!(c.apply_change(name, eq));
        c
    }

    fn horizon(m: &Model) -> f64 {
        m.settings.final_time - m.settings.initial_time
    }

    #[test]
    fn on_an_additive_model_the_shares_equal_the_one_at_a_time_changes() {
        let old = base("A + B");
        let new = set(&set(&old, "A", "3"), "B", "5");
        let r = run(&old, &new, 100, 1, ("old", "new")).unwrap();
        let g = &r.goals[0];
        let t = horizon(&old);
        assert_eq!(r.groups.len(), 2);
        for p in &g.parts {
            assert!((p.share - p.alone).abs() < 1e-9 * t, "{p:?}");
            assert!(p.interaction.abs() < 1e-9 * t);
        }
        let a = g.parts.iter().find(|p| p.group == "A").unwrap();
        assert!((a.share - 2.0 * t).abs() < 1e-9 * t, "{a:?}");
    }

    #[test]
    fn an_interaction_is_shared_as_the_hand_calculation_says() {
        // Fill = A * B, old A = 1, B = 1; new A = 2, B = 3. Final = T * A * B: 1T old, 2T with A alone, 3T with B alone, 6T with both.
        // Shapley A = (1T + (6T - 3T)) / 2 = 2T, B = (2T + (6T - 2T)) / 2 = 3T; together 5T = the whole change.
        let old = base("A * B");
        let new = set(&set(&old, "A", "2"), "B", "3");
        let r = run(&old, &new, 100, 1, ("old", "new")).unwrap();
        let t = horizon(&old);
        let g = &r.goals[0];
        let share = |n: &str| g.parts.iter().find(|p| p.group == n).unwrap().share;
        assert!((share("A") - 2.0 * t).abs() < 1e-9 * t, "{g:?}");
        assert!((share("B") - 3.0 * t).abs() < 1e-9 * t, "{g:?}");
        assert!((g.change.unwrap() - 5.0 * t).abs() < 1e-9 * t);
    }

    #[test]
    fn the_parts_add_up_and_repeated_runs_are_identical() {
        let old = base("A * B + C");
        let new = set(&set(&set(&old, "A", "2"), "B", "3"), "C", "0.5");
        let one = run(&old, &new, 100, 1, ("o", "n")).unwrap();
        let two = run(&old, &new, 100, 1, ("o", "n")).unwrap();
        assert_eq!(serde_json::to_string(&one).unwrap(), serde_json::to_string(&two).unwrap());
        let g = &one.goals[0];
        let sum: f64 = g.parts.iter().map(|p| p.share).sum();
        assert!((sum - g.change.unwrap()).abs() <= 1e-9 * g.change.unwrap().abs(), "{sum} {g:?}");
        assert!(g.unexplained.unwrap().abs() < 1e-9);
    }

    #[test]
    fn above_ten_groups_the_shares_are_sampled_with_a_fixed_seed_and_still_add_up() {
        let names: Vec<String> = (0..12).map(|i| format!("K{i}")).collect();
        let ops: Vec<String> = names.iter().map(|n| format!(r#"{{"op":"add_var","name":"{n}","kind":"constant","equation":"1"}}"#)).collect();
        let eq = names.join(" + ");
        let mut s = Session::new(Model::default());
        let mut all = vec![r#"{"op":"add_var","name":"Tank","kind":"stock","equation":"0"}"#.to_string()];
        all.extend(ops);
        all.push(format!(r#"{{"op":"add_var","name":"Fill","kind":"flow","equation":"{eq}","to":"Tank"}}"#));
        s.apply("t", &serde_json::from_str::<Vec<Op>>(&format!("[{}]", all.join(","))).unwrap()).unwrap();
        let mut old = s.model;
        old.manifest.goals.push(Goal { name: "T".into(), output: "Tank".into(), measure: "final".into(), ..Default::default() });
        let mut new = old.clone();
        for (i, n) in names.iter().enumerate() {
            new.apply_change(n, &format!("{}", 2 + i));
        }
        let a = run(&old, &new, 40, 7, ("o", "n")).unwrap();
        let b = run(&old, &new, 40, 7, ("o", "n")).unwrap();
        assert_eq!(a.method, "sampled");
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
        let g = &a.goals[0];
        let sum: f64 = g.parts.iter().map(|p| p.share).sum();
        assert!((sum - g.change.unwrap()).abs() <= 1e-9 * g.change.unwrap().abs());
        // an additive game: every permutation gives the same marginals, so the standard error is zero
        assert!(g.parts.iter().all(|p| p.se < 1e-9 * g.change.unwrap().abs()));
    }

    #[test]
    fn edits_that_cannot_move_a_run_are_listed_not_split() {
        let old = base("A");
        let mut new = old.clone();
        new.vars.iter_mut().find(|v| v.name == "Tank").unwrap().comment = "words".into();
        new.manifest.title = "A new title".into();
        let r = run(&old, &new, 100, 1, ("o", "n")).unwrap();
        assert!(r.groups.is_empty() && !r.no_effect.is_empty(), "{r:?}");
        assert!(markdown(&r).contains("No edit between the two versions can change a run"));
    }

    #[test]
    fn an_added_variable_that_another_reads_is_one_group() {
        let old = base("A");
        let mut new = old.clone();
        let mut s = Session::new(new.clone());
        s.apply("t", &serde_json::from_str::<Vec<Op>>(r#"[{"op":"add_var","name":"Boost","kind":"constant","equation":"4"}]"#).unwrap()).unwrap();
        new = s.model;
        new.apply_change("Fill", "A + Boost");
        let r = run(&old, &new, 100, 1, ("o", "n")).unwrap();
        let g = &r.goals[0];
        let sum: f64 = g.parts.iter().map(|p| p.share).sum();
        assert!((sum - g.change.unwrap()).abs() <= 1e-9 * g.change.unwrap().abs(), "{r:?}");
        assert!(r.notes.iter().any(|n| n.contains("one group")), "{:?}", r.notes);
    }
}
