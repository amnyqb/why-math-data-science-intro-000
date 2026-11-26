//! The sanity sheet of `dyn check --gate deliver`: what has to be true of a model before it is handed to someone else.
//!
//! Built from the specification of the task and the book's reply (its section 7.6 has ten checks; here they are the rows S1 to S14), not from a reference script. Seven rows are automated (strict units, the
//! model's checks and conservation, a rerun at DT/2, scenarios that differ in one lever only, goals read inside the
//! validity horizon, scenarios inside the validity range, two clean runs identical) and five are recorded by a person in
//! the manifest (`Manifest::sanity`): expected value written before the run, actuals reconciled, cross-check, benchmark,
//! reviewer and date. Each row is `pass`, `fail` or `open` (not enough declared or recorded to say).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::expr;
use crate::manifest::{self, Goal, GoalMeasure};
use crate::model::Model;
use crate::sim::Run;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
    Open,
    /// the stakes declared in the manifest do not ask for this row
    #[serde(rename = "not required")]
    NotRequired,
}

impl Status {
    pub fn word(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Open => "open",
            Status::NotRequired => "not required",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub id: String,
    pub check: String,
    pub status: Status,
    pub detail: String,
    /// made by the program (true) or recorded by a person in the manifest (false)
    pub automated: bool,
}

/// The relative change at DT/2 above which a goal depends on the time step.
pub const DT_TOLERANCE: f64 = 0.01;

fn row(id: &str, check: &str, status: Status, detail: impl Into<String>, automated: bool) -> Row {
    Row { id: id.into(), check: check.into(), status, detail: detail.into(), automated }
}

/// The goals to read: the manifest's, or one per declared output (final value) when it has none.
fn goals_of(m: &Model) -> Vec<Goal> {
    if !m.manifest.goals.is_empty() {
        return m.manifest.goals.clone();
    }
    m.manifest.outputs.iter().map(|o| Goal { name: o.variable.clone(), output: o.variable.clone(), measure: "final".into(), ..Default::default() }).collect()
}

/// The shortest horizon any validity statement declares.
pub fn horizon(m: &Model) -> Option<f64> {
    m.manifest.validity.iter().filter_map(|v| v.horizon).fold(None, |a: Option<f64>, h| Some(a.map_or(h, |x| x.min(h))))
}

/// The model with the values of a scenario (lever name → value) put in.
pub fn with_values(m: &Model, values: &BTreeMap<String, f64>) -> Result<Model, String> {
    let mut c = m.clone();
    for (k, v) in values {
        if !c.apply_change(k, &format!("{v}")) {
            return Err(format!("there is nothing called “{k}” to set"));
        }
    }
    Ok(c)
}

fn run(m: &Model) -> Result<Run, String> {
    let c = crate::sim::compile(m).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("; "))?;
    crate::sim::simulate(&c, m, "sanity")
}

/// Where in time a goal reads: (from, to).
fn read_window(g: &Goal, m: &Model) -> (f64, f64) {
    let (t0, t1) = (m.settings.initial_time, m.settings.final_time);
    match manifest::parse_goal_measure(&g.measure) {
        Ok(GoalMeasure::At(t)) => (t, t),
        Ok(GoalMeasure::Mean(a, b)) => (a, b),
        _ => (t0, t1),
    }
}

/// The rows a stake asks for. A workshop sketch needs the order of magnitude (S8), units and conservation (S1, S2) and the
/// time step (S3); a board paper or a loan file (and a manifest that says nothing) needs every row.
pub fn required(stakes: &str, id: &str) -> bool {
    match stakes.trim().to_lowercase().as_str() {
        "workshop" | "sketch" => ["S1", "S2", "S3", "S8"].contains(&id),
        _ => true,
    }
}

/// A row that the stakes do not ask for, made without running anything.
fn skipped(id: &str, check: &str, stakes: &str) -> Row {
    row(id, check, Status::NotRequired, format!("not required for stakes “{}”", stakes.trim()), true)
}

/// The value of every goal in the base case and in each scenario, with the model as given.
fn scenario_values(m: &Model, goals: &[Goal], extra: &BTreeMap<String, f64>) -> Option<Vec<Vec<Option<f64>>>> {
    let mut out = Vec::new();
    let mut cases: Vec<BTreeMap<String, f64>> = Vec::new();
    cases.extend(m.manifest.scenarios.iter().map(|s| s.values.clone()));
    if cases.is_empty() {
        cases.push(BTreeMap::new());
    }
    for c in cases {
        let mut v = extra.clone();
        v.extend(c);
        let r = with_values(m, &v).and_then(|x| run(&x)).ok()?;
        out.push(goals.iter().map(|g| manifest::goal_value_raw(&r, g)).collect());
    }
    Some(out)
}

/// The index of the best scenario for a goal.
fn best(vals: &[Vec<Option<f64>>], gi: usize, dir: manifest::Direction) -> Option<usize> {
    let key = |x: f64| match dir {
        manifest::Direction::Max => -x,
        manifest::Direction::Min => x,
        manifest::Direction::Target(t) => (x - t).abs(),
    };
    vals.iter().enumerate().filter_map(|(i, v)| v[gi].filter(|x| x.is_finite()).map(|x| (i, key(x)))).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
}

/// The sanity sheet of a model.
pub fn sheet(m: &Model, jobs: usize) -> Vec<Row> {
    let stakes = m.manifest.stakes.clone();
    let mut rows = Vec::new();
    // 1. strict units
    let units = crate::doctor::units_rules::check(m, &crate::doctor::Options::default());
    let bad: Vec<String> = units.iter().filter(|f| f.severity != crate::doctor::Severity::Info).map(|f| f.message.clone()).collect();
    rows.push(if bad.is_empty() { row("S1", "Strict units: a rate never meets an amount, every scale agrees", Status::Pass, "no unit finding above information", true) } else { row("S1", "Strict units: a rate never meets an amount, every scale agrees", Status::Fail, format!("{} unit finding(s), the first: {}", bad.len(), bad[0]), true) });
    // the base run, for the rows that need it
    let base = run(m);
    // 2. the model's own checks and conservation
    rows.push(match &base {
        Err(e) => row("S2", "The model's checks and conservation hold", Status::Fail, format!("the model does not run: {e}"), true),
        Ok(r) => {
            let f = crate::checks::all(m, r);
            if f.is_empty() { row("S2", "The model's checks and conservation hold", Status::Pass, format!("{} check(s) and the stock balances hold in the base run", m.checks.len()), true) } else { row("S2", "The model's checks and conservation hold", Status::Fail, f.iter().take(3).map(|x| x.message.clone()).collect::<Vec<_>>().join(" | "), true) }
        }
    });
    // 3. rerun at DT/2
    let goals = goals_of(m);
    rows.push(match &base {
        Err(_) => row("S3", "A rerun at DT/2 moves no goal by more than 1% and keeps the ranking of the scenarios", Status::Fail, "the model does not run", true),
        Ok(r) if goals.is_empty() => row("S3", "A rerun at DT/2 moves no goal by more than 1% and keeps the ranking of the scenarios", Status::Open, "no goals or outputs are declared in the manifest, so there is nothing to read", true),
        Ok(r) => match crate::doctor::base::rerun_at(m, 2) {
            None => row("S3", "A rerun at DT/2 moves no goal by more than 1% and keeps the ranking of the scenarios", Status::Open, "the rerun could not be made (too many steps)", true),
            Some(half) => {
                let mut parts = Vec::new();
                let mut worst: f64 = 0.0;
                for g in &goals {
                    let (a, b) = (manifest::goal_value_raw(r, g), manifest::goal_value_raw(&half, g));
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            let rel = if a == b { 0.0 } else { (a - b).abs() / a.abs().max(b.abs()).max(1e-300) };
                            worst = worst.max(rel);
                            parts.push(format!("{}: {:.3}%", g.name, rel * 100.0));
                        }
                        _ => parts.push(format!("{}: cannot be read", g.name)),
                    }
                }
                let mut st = if worst > DT_TOLERANCE || parts.iter().any(|p| p.ends_with("cannot be read")) { Status::Fail } else { Status::Pass };
                // the conclusion: the ranking of the scenarios by each goal must not change with the step
                let mut ranking = String::new();
                if m.manifest.scenarios.len() >= 2 {
                    let mut hm = m.clone();
                    hm.settings.time_step /= 2.0;
                    if let (Some(a), Some(b)) = (scenario_values(m, &goals, &BTreeMap::new()), scenario_values(&hm, &goals, &BTreeMap::new())) {
                        let order = |v: &[Vec<Option<f64>>], gi: usize| -> Vec<usize> {
                            let mut idx: Vec<usize> = (0..v.len()).collect();
                            idx.sort_by(|x, y| v[*x][gi].unwrap_or(f64::NAN).total_cmp(&v[*y][gi].unwrap_or(f64::NAN)));
                            idx
                        };
                        let changed: Vec<&str> = goals.iter().enumerate().filter(|(gi, _)| order(&a, *gi) != order(&b, *gi)).map(|(_, g)| g.name.as_str()).collect();
                        if changed.is_empty() {
                            ranking = "; the ranking of the scenarios is the same at DT/2".into();
                        } else {
                            st = Status::Fail;
                            ranking = format!("; the ranking of the scenarios changes at DT/2 for {}", changed.join(", "));
                        }
                    }
                }
                row("S3", "A rerun at DT/2 moves no goal by more than 1% and keeps the ranking of the scenarios", st, format!("largest relative change per goal: {}{}", parts.join("; "), ranking), true)
            }
        },
    });
    // 4. paired comparison
    let sc = &m.manifest.scenarios;
    rows.push(if sc.len() < 2 {
        row("S4", "Every scenario differs from the first in one lever only", Status::Open, "fewer than two scenarios are declared", true)
    } else {
        let value_of = |s: &manifest::ManifestScenario, lever: &manifest::ManifestLever| -> f64 {
            s.values.iter().find(|(k, _)| expr::normalize(k) == expr::normalize(&lever.name)).map(|(_, v)| *v).or(lever.default).or_else(|| m.by_name(&expr::normalize(&lever.name)).and_then(|v| v.equation.trim().parse().ok())).unwrap_or(f64::NAN)
        };
        let mut problems = Vec::new();
        let names: Vec<String> = sc.iter().flat_map(|s| s.values.keys().cloned()).collect();
        for s in &sc[1..] {
            let mut differing = Vec::new();
            for k in names.iter().collect::<std::collections::BTreeSet<_>>() {
                let lever = m.manifest.levers.iter().find(|l| expr::normalize(&l.name) == expr::normalize(k)).cloned().unwrap_or(manifest::ManifestLever { name: k.clone(), ..Default::default() });
                let (a, b) = (value_of(&sc[0], &lever), value_of(s, &lever));
                if a.is_nan() || b.is_nan() {
                    if a.is_nan() != b.is_nan() {
                        differing.push(k.clone());
                    }
                } else if a != b {
                    differing.push(k.clone());
                }
            }
            if differing.len() != 1 {
                problems.push(format!("“{}” differs from “{}” in {} lever(s){}", s.name, sc[0].name, differing.len(), if differing.is_empty() { String::new() } else { format!(" ({})", differing.join(", ")) }));
            }
        }
        if problems.is_empty() { row("S4", "Every scenario differs from the first in one lever only", Status::Pass, format!("{} scenarios, each against “{}”", sc.len() - 1, sc[0].name), true) } else { row("S4", "Every scenario differs from the first in one lever only", Status::Fail, problems.join("; "), true) }
    });
    // 5. goals inside the validity horizon
    let h = horizon(m);
    rows.push(match (h, goals.is_empty()) {
        (None, _) => row("S5", "Every goal is read inside the validity horizon", Status::Open, "no validity horizon is declared", true),
        (_, true) => row("S5", "Every goal is read inside the validity horizon", Status::Open, "no goals or outputs are declared", true),
        (Some(h), false) => {
            let beyond: Vec<String> = goals.iter().filter(|g| read_window(g, m).1 > h + 1e-9).map(|g| format!("“{}” reads up to {}", g.name, crate::doctor::num(read_window(g, m).1))).collect();
            if beyond.is_empty() { row("S5", "Every goal is read inside the validity horizon", Status::Pass, format!("{} goal(s), all read by time {}", goals.len(), crate::doctor::num(h)), true) } else { row("S5", "Every goal is read inside the validity horizon", Status::Fail, format!("horizon {}: {}", crate::doctor::num(h), beyond.join("; ")), true) }
        }
    });
    // 6. scenarios inside the validity range up to the horizon
    let ranges: Vec<&manifest::Validity> = m.manifest.validity.iter().filter(|v| v.variable.is_some() && (v.min.is_some() || v.max.is_some())).collect();
    rows.push(if ranges.is_empty() {
        row("S6", "Every scenario stays inside the validity range up to the horizon", Status::Open, "no validity range (a variable with a minimum or maximum) is declared", true)
    } else {
        let mut cases: Vec<(String, BTreeMap<String, f64>)> = vec![("the base run".into(), BTreeMap::new())];
        cases.extend(sc.iter().map(|s| (format!("scenario “{}”", s.name), s.values.clone())));
        let mut problems = Vec::new();
        for (label, values) in &cases {
            let r = with_values(m, values).and_then(|c| run(&c));
            match r {
                Err(e) => problems.push(format!("{label} does not run: {e}")),
                Ok(r) => {
                    for v in &ranges {
                        let name = v.variable.as_deref().unwrap_or_default();
                        let Some(s) = r.get(name) else { continue };
                        let last = h.unwrap_or(f64::INFINITY);
                        if let Some((t, x)) = r.times.iter().zip(s).find(|(t, x)| **t <= last + 1e-9 && (v.min.is_some_and(|lo| **x < lo - 1e-9) || v.max.is_some_and(|hi| **x > hi + 1e-9))) {
                            problems.push(format!("{label}: “{name}” is {} at time {}, outside {} to {}", crate::doctor::num(*x), crate::doctor::num(*t), v.min.map_or("-".into(), crate::doctor::num), v.max.map_or("-".into(), crate::doctor::num)));
                        }
                    }
                }
            }
        }
        if problems.is_empty() { row("S6", "Every scenario stays inside the validity range up to the horizon", Status::Pass, format!("{} run(s) checked against {} range(s){}", cases.len(), ranges.len(), h.map_or(String::new(), |h| format!(" up to time {}", crate::doctor::num(h)))), true) } else { row("S6", "Every scenario stays inside the validity range up to the horizon", Status::Fail, problems.join("; "), true) }
    });
    // 7. two clean runs identical
    rows.push(match (run(m), run(m)) {
        (Ok(a), Ok(b)) => {
            let same = a.times == b.times && a.series.len() == b.series.len() && a.series.iter().all(|(k, x)| b.series.get(k).is_some_and(|y| x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits())));
            if same { row("S7", "Two clean runs give identical numbers", Status::Pass, format!("{} series, bit for bit", a.series.len()), true) } else { row("S7", "Two clean runs give identical numbers", Status::Fail, "the two runs differ", true) }
        }
        (Err(e), _) | (_, Err(e)) => row("S7", "Two clean runs give identical numbers", Status::Fail, format!("the model does not run: {e}"), true),
    });
    // 13. extreme runs and signs
    rows.push(if !required(&stakes, "S13") { skipped("S13", "Extreme runs hold, and each scenario moves the goals in the expected direction", &stakes) } else {
        let opts = crate::doctor::Options { experiments: true, jobs: jobs.max(1), stage: Some(crate::doctor::Stage::C5), ..Default::default() };
        let rep = crate::doctor::check(m, &opts);
        let hits: Vec<&crate::doctor::Finding> = rep.findings.iter().filter(|f| matches!(f.rule.as_str(), "X1" | "X2" | "X3" | "X6") && f.severity != crate::doctor::Severity::Info && f.confirmed).collect();
        // each scenario moves each goal it declares an expectation for in that direction, against the first scenario
        let mut signs: Vec<String> = Vec::new();
        let mut declared = 0;
        let sc = &m.manifest.scenarios;
        if sc.len() >= 2 {
            if let Some(vals) = scenario_values(m, &goals, &BTreeMap::new()) {
                for (si, s) in sc.iter().enumerate().skip(1) {
                    for (gname, dir) in &s.expect {
                        let Some(gi) = goals.iter().position(|g| expr::normalize(&g.name) == expr::normalize(gname)) else {
                            signs.push(format!("“{}” expects a direction for “{gname}”, which is not a goal", s.name));
                            continue;
                        };
                        declared += 1;
                        let (a, b) = (vals[0][gi], vals[si][gi]);
                        let (Some(a), Some(b)) = (a, b) else {
                            signs.push(format!("“{}”: “{gname}” cannot be read", s.name));
                            continue;
                        };
                        let gdir = manifest::parse_direction(&goals[gi].direction).unwrap_or(manifest::Direction::Max);
                        let better = match gdir {
                            manifest::Direction::Max => b > a,
                            manifest::Direction::Min => b < a,
                            manifest::Direction::Target(t) => (b - t).abs() < (a - t).abs(),
                        };
                        let ok = match dir.trim().to_lowercase().as_str() {
                            "up" => b > a,
                            "down" => b < a,
                            "better" => better,
                            "worse" => !better && b != a,
                            other => {
                                signs.push(format!("“{}”: “{other}” is not up, down, better or worse", s.name));
                                continue;
                            }
                        };
                        if !ok {
                            signs.push(format!("“{}” should move “{gname}” {dir} but it goes from {} to {}", s.name, crate::doctor::num(a), crate::doctor::num(b)));
                        }
                    }
                }
            }
        }
        let extremes = if hits.is_empty() { "no extreme run breaks an invariant (X1, X2, X3, X6)".to_string() } else { format!("{} finding(s) above information in the extreme runs, the first: {}", hits.len(), hits[0].message) };
        let signs_text = if declared == 0 && signs.is_empty() { "no expected direction is declared for any scenario (manifest scenarios, field expect)".to_string() } else if signs.is_empty() { format!("{declared} expected direction(s) hold") } else { signs.join("; ") };
        let st = if !hits.is_empty() || !signs.is_empty() { Status::Fail } else if declared == 0 { Status::Open } else { Status::Pass };
        row("S13", "Extreme runs hold, and each scenario moves the goals in the expected direction", st, format!("{extremes}; {signs_text}"), true)
    });
    // 14. the conclusion across plausible values of the parameters
    rows.push(if !required(&stakes, "S14") { skipped("S14", "The conclusion holds across plausible values of the parameters", &stakes) } else if m.manifest.levers.is_empty() {
        row("S14", "The conclusion holds across plausible values of the parameters", Status::Open, "no levers with ranges are declared, so there are no plausible values to sweep", true)
    } else if goals.is_empty() {
        row("S14", "The conclusion holds across plausible values of the parameters", Status::Open, "no goals or outputs are declared", true)
    } else {
        let draws = if stakes.trim().eq_ignore_ascii_case("loan") { 64 } else { 16 };
        // stratified draws over every lever's range, the same every time
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut perms: Vec<Vec<usize>> = Vec::new();
        for _ in &m.manifest.levers {
            let mut p: Vec<usize> = (0..draws).collect();
            for i in (1..draws).rev() {
                p.swap(i, (next() * (i + 1) as f64) as usize % (i + 1));
            }
            perms.push(p);
        }
        let dirs: Vec<manifest::Direction> = goals.iter().map(|g| manifest::parse_direction(&g.direction).unwrap_or(manifest::Direction::Max)).collect();
        let reference = scenario_values(m, &goals, &BTreeMap::new());
        let ref_best: Vec<Option<usize>> = reference.as_ref().map(|v| (0..goals.len()).map(|gi| best(v, gi, dirs[gi])).collect()).unwrap_or_default();
        let (mut failed, mut changed) = (0usize, 0usize);
        let mut lo = vec![f64::INFINITY; goals.len()];
        let mut hi = vec![f64::NEG_INFINITY; goals.len()];
        for d in 0..draws {
            let mut x: BTreeMap<String, f64> = BTreeMap::new();
            for (li, l) in m.manifest.levers.iter().enumerate() {
                x.insert(l.name.clone(), l.min + (perms[li][d] as f64 + next()) / draws as f64 * (l.max - l.min));
            }
            match scenario_values(m, &goals, &x) {
                None => failed += 1,
                Some(v) => {
                    if v.iter().flatten().flatten().any(|y| !y.is_finite()) {
                        failed += 1;
                        continue;
                    }
                    for gi in 0..goals.len() {
                        for row_ in &v {
                            if let Some(y) = row_[gi] {
                                lo[gi] = lo[gi].min(y);
                                hi[gi] = hi[gi].max(y);
                            }
                        }
                    }
                    if m.manifest.scenarios.len() >= 2 && (0..goals.len()).any(|gi| ref_best.get(gi).copied().flatten() != best(&v, gi, dirs[gi])) {
                        changed += 1;
                    }
                }
            }
        }
        let ranges = goals.iter().enumerate().filter(|(i, _)| lo[*i].is_finite()).map(|(i, g)| format!("{}: {} to {}", g.name, crate::doctor::num(lo[i]), crate::doctor::num(hi[i]))).collect::<Vec<_>>().join("; ");
        let st = if failed > 0 || changed > 0 { Status::Fail } else if m.manifest.scenarios.len() < 2 { Status::Open } else { Status::Pass };
        let conclusion = if m.manifest.scenarios.len() < 2 { "fewer than two scenarios, so only the ranges of the goals are shown".to_string() } else if changed == 0 { "the best scenario of every goal is the same in every draw".to_string() } else { format!("the best scenario of some goal changes in {changed} of {draws} draws") };
        row("S14", "The conclusion holds across plausible values of the parameters", st, format!("{draws} draws over the ranges of {} lever(s); {} run(s) failed or gave no number; {conclusion}; ranges of the goals: {ranges}", m.manifest.levers.len(), failed), true)
    });
    // a teaching draft is not for delivery
    if m.settings.teaching_draft {
        rows.push(row("S15", "Not marked as a teaching draft", Status::Open, "the model is marked as a teaching draft (setting teaching_draft): its extreme-run findings are information, and it is not for delivery", false));
    }
    // 8 to 12: recorded by a person
    for (i, (kind, what)) in manifest::HUMAN_CHECKS.iter().enumerate() {
        let id = format!("S{}", 8 + i);
        let rec = m.manifest.sanity.iter().find(|h| h.kind == *kind);
        rows.push(match rec {
            None => row(&id, what, Status::Open, format!("not recorded: add a “{kind}” entry to the manifest (sanity)"), false),
            Some(h) => {
                let signed = !h.by.trim().is_empty() && !h.date.trim().is_empty();
                let complete = match *kind {
                    "expected-value" => h.value.is_some(),
                    "reviewer" => true,
                    _ => !h.note.trim().is_empty(),
                };
                let st = match h.status.trim().to_lowercase().as_str() {
                    "fail" => Status::Fail,
                    "open" => Status::Open,
                    _ if signed && complete => Status::Pass,
                    _ => Status::Open,
                };
                let mut detail = format!("{}{}{}", if h.note.trim().is_empty() { String::new() } else { format!("{} ", h.note.trim()) }, h.value.map_or(String::new(), |v| format!("(value {}) ", crate::doctor::num(v))), if signed { format!("by {} on {}", h.by.trim(), h.date.trim()) } else { "not signed: by and date are needed".into() });
                detail = detail.trim().to_string();
                row(&id, what, st, detail, false)
            }
        });
    }
    for r in rows.iter_mut() {
        if !required(&stakes, &r.id) && r.status != Status::NotRequired {
            *r = skipped(&r.id, &r.check, &stakes);
        }
    }
    rows.sort_by_key(|r| r.id[1..].parse::<u32>().unwrap_or(99));
    rows
}

/// True when every row that is required passes.
pub fn delivered(rows: &[Row]) -> bool {
    rows.iter().all(|r| matches!(r.status, Status::Pass | Status::NotRequired))
}

/// The sheet as Markdown.
pub fn markdown(m: &Model, rows: &[Row]) -> String {
    let title = if m.manifest.title.trim().is_empty() { m.title.clone() } else { m.manifest.title.clone() };
    let cell = |s: &str| s.replace('|', "/").replace('\n', " ");
    let stakes = if m.manifest.stakes.trim().is_empty() { "not declared (every row is required)".to_string() } else { m.manifest.stakes.trim().to_string() };
    let mut s = format!("# Sanity sheet: {}\n\nStakes: {stakes}.\n\n| # | Check | Made by | Status | Detail |\n|---|---|---|---|---|\n", cell(&title));
    for r in rows {
        s.push_str(&format!("| {} | {} | {} | {} | {} |\n", r.id, cell(&r.check), if r.automated { "program" } else { "person" }, r.status.word(), cell(&r.detail)));
    }
    let n = |st: Status| rows.iter().filter(|r| r.status == st).count();
    s.push_str(&format!("\n{} pass, {} fail, {} open, {} not required. {}\n", n(Status::Pass), n(Status::Fail), n(Status::Open), n(Status::NotRequired), if delivered(rows) { "Ready to deliver." } else { "Not ready to deliver." }));
    s
}
