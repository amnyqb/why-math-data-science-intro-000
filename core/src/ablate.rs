//! `dyn ablate`: take one thing out of the model at a time and see what each goal owes to it.
//!
//! Built from the specification of the task, not from a reference script. The ablations are the manifest's own (`ablations`)
//! or, when it has none, the defaults: each feedback loop knocked out by cutting one of its links (the equation that
//! reads the link's source reads the source's starting value instead), and each stock frozen at its initial value (its
//! flows are detached from it). Each runs on a copy of the model. The table gives the change in every goal of the
//! manifest (or, with no goals, in every declared output at the end of the run); a removal that improves a goal is
//! flagged "check the goal": something that is meant to help the goal is working against it, or the goal is wrong.

use serde::Serialize;

use crate::expr;
use crate::manifest::{self, Ablation, Direction, Goal};
use crate::model::{Kind, Model};

#[derive(Clone, Debug, Serialize)]
pub struct Cell {
    pub goal: String,
    pub base: Option<f64>,
    pub value: Option<f64>,
    pub change: Option<f64>,
    /// relative to the size of the base value
    pub change_pct: Option<f64>,
    /// the removal moves the goal the way the goal wants
    pub improves: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Result {
    pub ablation: String,
    pub what: String,
    /// None when the ablated model ran
    pub error: Option<String>,
    pub cells: Vec<Cell>,
    /// "check the goal" when any goal improves
    pub flag: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Table {
    pub goals: Vec<String>,
    pub results: Vec<Result>,
}

pub fn goals_of(m: &Model) -> Vec<Goal> {
    if !m.manifest.goals.is_empty() {
        return m.manifest.goals.clone();
    }
    let outs: Vec<String> = if m.manifest.outputs.is_empty() { m.vars.iter().filter(|v| v.kind == Kind::Stock).map(|v| v.name.clone()).collect() } else { m.manifest.outputs.iter().map(|o| o.variable.clone()).collect() };
    outs.into_iter().map(|o| Goal { name: o.clone(), output: o, measure: "final".into(), ..Default::default() }).collect()
}

/// The defaults: each loop (one cut each) and each stock.
pub fn defaults(m: &Model) -> Vec<Ablation> {
    let mut out: Vec<Ablation> = Vec::new();
    let mut copy = m.clone();
    let d = crate::cld::from_model(&mut copy);
    let text = |id: u64| d.nodes.iter().find(|n| n.id == id).map(|n| n.text.clone()).unwrap_or_default();
    for l in crate::cld::find_loops(&d) {
        // cut where the target is not a stock (a stock only reads its flows), and not into a flow's own stock
        let pick = l.links.iter().filter_map(|k| d.links.iter().find(|x| x.id == *k)).find(|x| m.by_name(&expr::normalize(&text(x.to))).is_some_and(|v| v.kind != Kind::Stock && v.kind != Kind::Constant) && m.by_name(&expr::normalize(&text(x.from))).is_some_and(|v| v.kind != Kind::Flow || v.from.is_none() && v.to.is_none()));
        let pick = pick.or_else(|| l.links.iter().filter_map(|k| d.links.iter().find(|x| x.id == *k)).find(|x| m.by_name(&expr::normalize(&text(x.to))).is_some_and(|v| v.kind != Kind::Stock)));
        let Some(x) = pick else { continue };
        let names: Vec<String> = l.nodes.iter().map(|n| text(*n)).collect();
        let name = format!("Loop {}", names.join(" > "));
        if !out.iter().any(|o| o.cut == vec![[text(x.from), text(x.to)]]) {
            out.push(Ablation { name, freeze: Vec::new(), cut: vec![[text(x.from), text(x.to)]] });
        }
    }
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        out.push(Ablation { name: format!("Freeze {}", s.name), freeze: vec![s.name.clone()], cut: Vec::new() });
    }
    out
}

/// The model with the ablation made; an error names what could not be done.
pub fn apply(m: &Model, a: &Ablation) -> std::result::Result<Model, String> {
    let mut c = m.clone();
    for name in &a.freeze {
        let id = c.by_name(&expr::normalize(name)).filter(|v| v.kind == Kind::Stock).map(|v| v.id).ok_or_else(|| format!("“{name}” is not a stock to freeze"))?;
        for v in c.vars.iter_mut().filter(|v| v.kind == Kind::Flow) {
            if v.from == Some(id) {
                v.from = None;
            }
            if v.to == Some(id) {
                v.to = None;
            }
        }
    }
    for [from, to] in &a.cut {
        let key = expr::normalize(from);
        let id = c.by_name(&expr::normalize(to)).map(|v| v.id).ok_or_else(|| format!("there is no “{to}” to cut a link into"))?;
        c.by_name(&key).ok_or_else(|| format!("there is no “{from}” to cut a link from"))?;
        let v = c.var_mut(id).unwrap();
        let new = expr::rename_in(&v.equation, &key, &format!("INITIAL({from})"));
        if new == v.equation {
            return Err(format!("the equation of “{to}” does not read “{from}”"));
        }
        v.equation = new;
    }
    Ok(c)
}

fn describe(a: &Ablation) -> String {
    let mut p: Vec<String> = a.freeze.iter().map(|s| format!("{s} frozen at its initial value")).collect();
    p.extend(a.cut.iter().map(|[f, t]| format!("link {f} to {t} cut")));
    p.join("; ")
}

fn better(dir: Direction, base: f64, now: f64) -> bool {
    let eps = 1e-9 * base.abs().max(now.abs()).max(1e-12);
    match dir {
        Direction::Max => now > base + eps,
        Direction::Min => now < base - eps,
        Direction::Target(t) => (now - t).abs() < (base - t).abs() - eps,
    }
}

/// Run every ablation (the manifest's, or the defaults) and read the goals.
pub fn run(m: &Model, only: Option<&str>) -> std::result::Result<Table, String> {
    let goals = goals_of(m);
    if goals.is_empty() {
        return Err("the model has no goals, outputs or stocks to read".into());
    }
    let base = crate::runner::run_with(m, &[]).map_err(|e| format!("the model does not run: {e}"))?;
    let base_vals: Vec<Option<f64>> = goals.iter().map(|g| manifest::goal_value_raw(&base, g)).collect();
    let list = if m.manifest.ablations.is_empty() { defaults(m) } else { m.manifest.ablations.clone() };
    let list: Vec<Ablation> = list.into_iter().filter(|a| only.is_none_or(|o| expr::normalize(o) == expr::normalize(&a.name))).collect();
    if list.is_empty() {
        return Err(match only {
            Some(o) => format!("there is no ablation called “{o}”"),
            None => "there is nothing to ablate: no loops and no stocks".into(),
        });
    }
    let mut results = Vec::new();
    for a in &list {
        let what = describe(a);
        let run = apply(m, a).and_then(|c| crate::runner::run_with(&c, &[]));
        match run {
            Err(e) => results.push(Result { ablation: a.name.clone(), what, error: Some(e), cells: Vec::new(), flag: String::new() }),
            Ok(r) => {
                let cells: Vec<Cell> = goals
                    .iter()
                    .zip(&base_vals)
                    .map(|(g, b)| {
                        let v = manifest::goal_value_raw(&r, g);
                        let dir = manifest::parse_direction(&g.direction).unwrap_or(Direction::Max);
                        let change = match (b, v) {
                            (Some(b), Some(v)) => Some(v - b),
                            _ => None,
                        };
                        let pct = match (b, change) {
                            (Some(b), Some(c)) if b.abs() > 1e-12 => Some(100.0 * c / b.abs()),
                            _ => None,
                        };
                        let improves = matches!((b, v), (Some(b), Some(v)) if better(dir, *b, v));
                        Cell { goal: g.name.clone(), base: *b, value: v, change, change_pct: pct, improves }
                    })
                    .collect();
                let flag = if cells.iter().any(|c| c.improves) { "check the goal".to_string() } else { String::new() };
                results.push(Result { ablation: a.name.clone(), what, error: None, cells, flag });
            }
        }
    }
    Ok(Table { goals: goals.iter().map(|g| g.name.clone()).collect(), results })
}

/// The table as Markdown.
pub fn markdown(t: &Table) -> String {
    let n = crate::doctor::num;
    let mut s = String::from("| Removed |");
    for g in &t.goals {
        s.push_str(&format!(" {g} |"));
    }
    s.push_str(" Flag |\n|---|");
    for _ in &t.goals {
        s.push_str("---|");
    }
    s.push_str("---|\n");
    for r in &t.results {
        s.push_str(&format!("| {} |", r.ablation.replace('|', "/")));
        if let Some(e) = &r.error {
            s.push_str(&format!(" {} |", e.replace('|', "/")));
            for _ in 1..t.goals.len() {
                s.push_str(" |");
            }
            s.push_str(" |\n");
            continue;
        }
        for c in &r.cells {
            let cell = match (c.value, c.change_pct, c.change) {
                (Some(v), Some(p), _) => format!("{} ({:+.1}%)", n(v), p),
                (Some(v), None, Some(d)) => format!("{} ({:+})", n(v), n(d)),
                (Some(v), _, _) => n(v),
                _ => "cannot be read".to_string(),
            };
            s.push_str(&format!(" {cell}{} |", if c.improves { " *" } else { "" }));
        }
        s.push_str(&format!(" {} |\n", r.flag));
    }
    s.push_str("\nBase values: ");
    s.push_str(&t.results.iter().find(|r| r.error.is_none()).map(|r| r.cells.iter().map(|c| format!("{} = {}", c.goal, c.base.map_or("n/a".to_string(), n))).collect::<Vec<_>>().join(", ")).unwrap_or_default());
    s.push_str("\n\n`*` marks a goal that improves when the thing is removed: check the goal.\n");
    s
}
