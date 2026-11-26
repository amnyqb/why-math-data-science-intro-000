//! The content of `dyn export --xlsx`: five sheets of plain cells that the command line writes as an Excel workbook.
//!
//! Built from the specification of the task, not from a reference script. Cover (purpose, validity, check status, how to
//! re-create the numbers), Inputs (levers with units, tested ranges, sources), Scenarios (outputs at the validity horizon,
//! and at the end of the run, labelled as beyond the horizon when it is later), Series (the base run, a row per saved time)
//! and Checks (the model's own checks and the sanity sheet). Values only: every number is the engine's.

use std::collections::BTreeMap;

use crate::expr;
use crate::manifest::{self, Goal};
use crate::model::{Kind, Model};
use crate::sanity;

#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Text(String),
    Num(f64),
    Empty,
}

#[derive(Clone, Debug)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
    /// the first row is a heading
    pub heading: bool,
}

fn t(s: impl Into<String>) -> Cell {
    Cell::Text(s.into())
}

fn n(x: f64) -> Cell {
    if x.is_finite() { Cell::Num(x) } else { Cell::Text("not a number".into()) }
}

/// The variables to show: the manifest's outputs, else the goals' outputs, else the stocks.
pub fn outputs_of(m: &Model) -> Vec<String> {
    let mut v: Vec<String> = m.manifest.outputs.iter().map(|o| o.variable.clone()).collect();
    if v.is_empty() {
        v = m.manifest.goals.iter().map(|g| g.output.clone()).collect();
        v.dedup();
    }
    if v.is_empty() {
        v = m.vars.iter().filter(|x| x.kind == Kind::Stock).map(|x| x.name.clone()).collect();
    }
    v
}

fn at(run: &crate::sim::Run, name: &str, time: f64) -> Option<f64> {
    manifest::goal_value_raw(run, &Goal { name: name.into(), output: name.into(), measure: format!("at:{time}"), ..Default::default() })
}

/// The five sheets. `file` is the name of the model file, for the instructions to re-create the numbers.
pub fn build(m: &Model, file: &str) -> Vec<Sheet> {
    let end = m.settings.final_time;
    let horizon = sanity::horizon(m);
    let read_at = horizon.map_or(end, |h| h.min(end));
    let outs = outputs_of(m);
    let base = crate::runner::run_with(m, &[]);
    let title = if m.manifest.title.trim().is_empty() { m.title.clone() } else { m.manifest.title.clone() };

    // ---- Cover
    let mut cover: Vec<Vec<Cell>> = vec![vec![t(if title.trim().is_empty() { "Model" } else { &title })]];
    let mut line = |a: &str, b: String| cover.push(vec![t(a), t(b)]);
    line("Purpose", m.purpose.as_ref().map(|p| p.problem.clone()).filter(|p| !p.trim().is_empty()).unwrap_or_else(|| m.manifest.description.clone()));
    if let Some(p) = &m.purpose {
        if !p.out_of_scope.trim().is_empty() {
            line("Left out on purpose", p.out_of_scope.clone());
        }
    }
    for v in &m.manifest.validity {
        let range = match (&v.variable, v.min, v.max) {
            (Some(x), lo, hi) => format!(" ({x} between {} and {})", lo.map_or("-".into(), crate::doctor::num), hi.map_or("-".into(), crate::doctor::num)),
            _ => String::new(),
        };
        line("Valid", format!("{}{}{}", v.statement, range, v.horizon.map_or(String::new(), |h| format!("; horizon {} {}", crate::doctor::num(h), m.settings.time_units))));
    }
    if m.manifest.validity.is_empty() {
        line("Valid", "no validity statement is declared".into());
    }
    let rows = sanity::sheet(m, 1);
    let count = |s: sanity::Status| rows.iter().filter(|r| r.status == s).count();
    line("Check status", format!("{} pass, {} fail, {} open of {} rows of the sanity sheet; see the sheet Checks", count(sanity::Status::Pass), count(sanity::Status::Fail), count(sanity::Status::Open), rows.len()));
    line("Model file", file.to_string());
    line("Time", format!("{} to {} {} in steps of {} ({:?}); seed {}", crate::doctor::num(m.settings.initial_time), crate::doctor::num(end), m.settings.time_units, crate::doctor::num(m.settings.time_step), m.settings.method, m.settings.seed));
    line("Re-create the numbers", format!("dyn run {file} runs the base case; dyn run {file} --set NAME=VALUE changes a lever; dyn check {file} --gate deliver prints the sanity sheet. The sheet Series is that base run, the sheet Scenarios runs each scenario of the manifest."));
    line("Reading the results", match horizon {
        Some(h) => format!("outputs are read at the validity horizon, time {}; the end of the run (time {}) is shown as beyond the horizon where it is later", crate::doctor::num(h), crate::doctor::num(end)),
        None => format!("no horizon is declared: outputs are read at the end of the run, time {}", crate::doctor::num(end)),
    });

    // ---- Inputs
    let mut inputs: Vec<Vec<Cell>> = vec![["Lever", "Units", "Minimum tested", "Maximum tested", "Default", "Value in the model", "Explanation", "Group", "Source"].iter().map(|s| t(*s)).collect()];
    for l in &m.manifest.levers {
        let source = m.manifest.bindings.iter().find(|b| expr::normalize(&b.input) == expr::normalize(&l.name)).map(|b| format!("{} {}", b.source, b.note).trim().to_string()).or_else(|| m.by_name(&expr::normalize(&l.name)).map(|v| v.comment.clone())).unwrap_or_default();
        let value = m.by_name(&expr::normalize(&l.name)).and_then(|v| v.equation.trim().parse::<f64>().ok());
        let units = if l.units.trim().is_empty() { m.by_name(&expr::normalize(&l.name)).map(|v| v.units.clone()).unwrap_or_default() } else { l.units.clone() };
        inputs.push(vec![t(&l.name), t(units), n(l.min), n(l.max), l.default.map_or(Cell::Empty, n), value.map_or(Cell::Empty, n), t(&l.explanation), t(&l.group), t(source)]);
    }

    // ---- Scenarios
    let mut head: Vec<Cell> = vec![t("Scenario")];
    head.extend(m.manifest.levers.iter().map(|l| t(format!("{} (lever)", l.name))));
    for o in &outs {
        head.push(t(format!("{o} at time {}", crate::doctor::num(read_at))));
        if end > read_at + 1e-9 {
            head.push(t(format!("{o} at time {} (beyond the horizon)", crate::doctor::num(end))));
        }
    }
    let mut scen: Vec<Vec<Cell>> = vec![head];
    let mut cases: Vec<(String, BTreeMap<String, f64>)> = vec![("Base case".into(), BTreeMap::new())];
    cases.extend(m.manifest.scenarios.iter().map(|s| (s.name.clone(), s.values.clone())));
    for (name, values) in &cases {
        let mut row = vec![t(name)];
        for l in &m.manifest.levers {
            let v = values.iter().find(|(k, _)| expr::normalize(k) == expr::normalize(&l.name)).map(|(_, v)| *v).or_else(|| m.by_name(&expr::normalize(&l.name)).and_then(|v| v.equation.trim().parse().ok()));
            row.push(v.map_or(Cell::Empty, n));
        }
        match sanity::with_values(m, values).and_then(|c| crate::runner::run_with(&c, &[])) {
            Ok(r) => {
                for o in &outs {
                    row.push(at(&r, o, read_at).map_or(t("n/a"), n));
                    if end > read_at + 1e-9 {
                        row.push(at(&r, o, end).map_or(t("n/a"), n));
                    }
                }
            }
            Err(e) => row.push(t(format!("does not run: {e}"))),
        }
        scen.push(row);
    }

    // ---- Series
    let mut series: Vec<Vec<Cell>> = vec![std::iter::once(t(format!("Time ({})", m.settings.time_units))).chain(outs.iter().map(|o| t(o.clone()))).collect()];
    if let Ok(r) = &base {
        for (i, time) in r.times.iter().enumerate() {
            let mut row = vec![n(*time)];
            for o in &outs {
                row.push(r.get(o).and_then(|s| s.get(i)).map_or(Cell::Empty, |x| n(*x)));
            }
            series.push(row);
        }
    }

    // ---- Checks
    let mut checks: Vec<Vec<Cell>> = vec![["Check", "Variable", "Kind", "Status", "Detail"].iter().map(|s| t(*s)).collect()];
    for c in &m.checks {
        let failed = base.as_ref().ok().map(|r| crate::checks::limits(&Model { checks: vec![c.clone()], ..m.clone() }, r));
        let (status, detail) = match failed {
            None => ("fail".to_string(), "the model does not run".to_string()),
            Some(f) if f.is_empty() => ("pass".to_string(), String::new()),
            Some(f) => ("fail".to_string(), f.into_iter().next().map(|x| x.message).unwrap_or_default()),
        };
        let vouched = m.manifest.tests.iter().any(|x| expr::normalize(x) == expr::normalize(&c.name));
        checks.push(vec![t(&c.name), t(&c.var), t(format!("{:?}{}", c.kind, if vouched { ", vouches for the model" } else { "" })), t(status), t(detail)]);
    }
    checks.push(vec![Cell::Empty]);
    checks.push(["Sanity row", "Check", "Made by", "Status", "Detail"].iter().map(|s| t(*s)).collect());
    for r in &rows {
        checks.push(vec![t(&r.id), t(&r.check), t(if r.automated { "program" } else { "person" }), t(r.status.word()), t(&r.detail)]);
    }

    vec![
        Sheet { name: "Cover".into(), rows: cover, heading: false },
        Sheet { name: "Inputs".into(), rows: inputs, heading: true },
        Sheet { name: "Scenarios".into(), rows: scen, heading: true },
        Sheet { name: "Series".into(), rows: series, heading: true },
        Sheet { name: "Checks".into(), rows: checks, heading: true },
    ]
}
