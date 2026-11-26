//! The model doctor: findings about a model, each with exact fixes.
//!
//! A `Finding` has a code (`unit-scale-mismatch`, `magic-number`…), a severity, the variable it is about, a message and
//! a list of `Fix`es: each fix is a list of `ops::Op` (the same operations as everything else that edits a model), and
//! says whether it keeps the numbers of the run. `check` runs the stages in order and returns a `Report`.
//!
//! Phase 1 holds the units rules (`units_rules`) and the other base checks (`base`); phase 2 adds the full rule table.

pub mod base;
pub mod experiments;
pub mod fixes;
pub mod interval;
pub mod rules_base;
pub mod rules_c;
pub mod rules_f;
pub mod rules_g;
pub mod rules_i;
pub mod rules_n;
pub mod rules_p;
pub mod rules_r;
pub mod rules_s;
pub mod rules_u;
pub mod rules_x;
pub mod units_rules;

use serde::Serialize;

use crate::model::Model;
use crate::ops::Op;

/// A number for a message of the doctor: as everywhere, but "not a number" where `fmt_num` writes a dash.
pub fn num(v: f64) -> String {
    if v.is_nan() { "not a number".into() } else { crate::fmt::fmt_num(v) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fix {
    pub title: String,
    pub ops: Vec<Op>,
    /// Applying it leaves every number of the run as it is.
    pub keeps_numbers: bool,
    /// The fix is a domain choice (7 calendar days or 5 working days to a week): it is never applied automatically.
    #[serde(default)]
    pub needs_decision: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub var: Option<String>,
    pub message: String,
    pub fixes: Vec<Fix>,
    /// The citation of the rule (empty when the rule is Dynamist's own).
    pub source: String,
    /// The id of the rule in the catalogue (S1, G1, F3, N2, U1…).
    pub rule: String,
    /// A "can" finding of the interval analysis is false until the base run or an experiment confirms it; every other
    /// finding is confirmed.
    pub confirmed: bool,
}

impl Finding {
    pub fn new(code: &str, severity: Severity, var: Option<&str>, message: impl Into<String>) -> Finding {
        Finding { code: code.into(), severity, var: var.map(|v| v.to_string()), message: message.into(), fixes: Vec::new(), source: String::new(), rule: rule_of(code).into(), confirmed: true }
    }
    pub fn with_fixes(mut self, fixes: Vec<Fix>) -> Finding {
        self.fixes = fixes;
        self
    }
}

/// The catalogue id of a finding code: the rule ids of the new rules are their own codes; the codes of the phase 1 checks
/// map to the rule that covers them.
pub fn rule_of(code: &str) -> &str {
    match code {
        "unit-dimension-mismatch" | "unit-declared" => "U1",
        "unit-scale-mismatch" => "U2",
        "unit-percent" | "unit-scale-change" => "U3",
        "unit-flow" => "U4",
        "money-real-nominal" => "U5",
        "unit-ambiguous" => "U6",
        "unit-parse" => "U7",
        "units-inferred" => "U8",
        "magic-number" => "F4",
        "zero-division" => "F3",
        "lookup-outside" => "F8",
        "ratio-outside" => "F14",
        "dt-sensitivity" => "N3",
        "dt-too-large" => "N2",
        "time-equality" => "N6",
        "equilibrium" => "I2",
        "check" => "X6",
        other => other,
    }
}

/// The seven stages of the doctor, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Stage {
    /// C1: parse
    C1,
    /// C2: graphs
    C2,
    /// C3: units, signs and intervals
    C3,
    /// C4: base run and numerical health
    C4,
    /// C5: automatic experiments
    C5,
    /// C6: data and calibration
    C6,
    /// C7: report and gate
    C7,
}

impl Stage {
    pub fn parse(s: &str) -> Option<Stage> {
        Some(match s.trim().to_uppercase().trim_start_matches('C') {
            "1" => Stage::C1,
            "2" => Stage::C2,
            "3" => Stage::C3,
            "4" => Stage::C4,
            "5" => Stage::C5,
            "6" => Stage::C6,
            "7" => Stage::C7,
            _ => return None,
        })
    }
}

/// The gate a model passes: Draft (errors), Review (no errors), Publish (no errors and no warnings).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Gate {
    Draft,
    Review,
    Publish,
}

/// What a rule sees: the model, the options, and the things several rules need (computed once, when first asked for).
pub struct Ctx<'a> {
    pub model: &'a Model,
    pub opts: &'a Options,
    base: std::cell::OnceCell<Option<crate::sim::Run>>,
    facts: std::cell::OnceCell<interval::Facts>,
    problems: std::cell::OnceCell<Vec<crate::sim::Problem>>,
}

impl<'a> Ctx<'a> {
    pub fn new(model: &'a Model, opts: &'a Options) -> Ctx<'a> {
        Ctx { model, opts, base: Default::default(), facts: Default::default(), problems: Default::default() }
    }
    /// The base run, if the model compiles and runs.
    pub fn base(&self) -> Option<&crate::sim::Run> {
        self.base.get_or_init(|| crate::sim::compile(self.model).ok().and_then(|c| crate::sim::simulate(&c, self.model, "doctor").ok())).as_ref()
    }
    /// The base run, only when the stages that need it (C4 and later) are asked for: C1 to C3 must stay fast.
    pub fn base_if_asked(&self) -> Option<&crate::sim::Run> {
        if self.opts.last_stage() >= Stage::C4 { self.base() } else { None }
    }
    pub fn facts(&self) -> &interval::Facts {
        self.facts.get_or_init(|| interval::analyse(self.model))
    }
    /// What the compiler complains about.
    pub fn problems(&self) -> &[crate::sim::Problem] {
        self.problems.get_or_init(|| crate::sim::compile(self.model).err().unwrap_or_default())
    }
}

/// One rule of the catalogue.
pub trait Rule {
    fn id(&self) -> &'static str;
    fn stage(&self) -> Stage;
    fn check(&self, ctx: &Ctx) -> Vec<Finding>;
    /// A line for the generated documentation: what it catches.
    fn about(&self) -> &'static str {
        ""
    }
}

/// A rule made of a function.
pub struct FnRule {
    pub id: &'static str,
    pub stage: Stage,
    pub about: &'static str,
    pub f: fn(&Ctx) -> Vec<Finding>,
}

impl Rule for FnRule {
    fn id(&self) -> &'static str {
        self.id
    }
    fn stage(&self) -> Stage {
        self.stage
    }
    fn check(&self, ctx: &Ctx) -> Vec<Finding> {
        (self.f)(ctx)
    }
    fn about(&self) -> &'static str {
        self.about
    }
}

/// The table of rules: every rule is registered here, and the documentation is generated from it.
pub fn rules() -> Vec<Box<dyn Rule>> {
    let mut v: Vec<Box<dyn Rule>> = Vec::new();
    v.extend(rules_s::rules());
    v.extend(rules_base::rules());
    v.extend(rules_u::rules());
    v.extend(rules_g::rules());
    v.extend(rules_f::rules());
    v.extend(rules_n::rules());
    v.extend(rules_i::rules());
    v.extend(rules_x::rules());
    v.extend(rules_r::rules());
    v.extend(rules_c::rules());
    v.extend(rules_p::rules());
    v
}

/// What `check` is asked to do.
#[derive(Clone, Debug)]
pub struct Options {
    /// Warnings count as errors (the publish gate).
    pub publish: bool,
    /// Old models: a rate may meet its amount (the time unit is ignored when comparing).
    pub lenient_time: bool,
    /// Only the units rules.
    pub units_only: bool,
    /// Run the stages up to this one. None: C1 to C4 (C5 only with `experiments`, and everything under the publish gate).
    pub stage: Option<Stage>,
    /// Run the automatic experiments of stage C5.
    pub experiments: bool,
    /// Threads for the experiments.
    pub jobs: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { publish: false, lenient_time: false, units_only: false, stage: None, experiments: false, jobs: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(4) }
    }
}

impl Options {
    /// The last stage that runs.
    pub fn last_stage(&self) -> Stage {
        match self.stage {
            Some(s) => s,
            None if self.experiments || self.publish => Stage::C5,
            None => Stage::C4,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// The summary of stage C7.
    pub summary: Summary,
    /// The gate the model passes.
    pub gate: Option<Gate>,
}

/// The SDM-Doc style summary: what the model holds and how well it is described.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub stocks: usize,
    pub flows: usize,
    pub auxiliaries: usize,
    pub constants: usize,
    pub lookups: usize,
    pub loops: usize,
    pub units_coverage_pct: f64,
    pub documentation_coverage_pct: f64,
}

impl Report {
    pub fn count(&self, s: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == s).count()
    }
    pub fn fixes_available(&self) -> usize {
        self.findings.iter().filter(|f| !f.fixes.is_empty()).count()
    }
    pub fn by_code(&self, code: &str) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.code == code).collect()
    }
    /// The one-line summary printed by `dyn check`.
    pub fn line(&self, applied: usize) -> String {
        format!("{} errors, {} warnings, {} fixes available, {} fixes applied", self.count(Severity::Error), self.count(Severity::Warning), self.fixes_available(), applied)
    }
}

/// Run the checks.
pub fn check(m: &Model, opts: &Options) -> Report {
    let ctx = Ctx::new(m, opts);
    let last = opts.last_stage();
    let mut findings = units_rules::check(m, opts);
    // a figure model folds constants into equations on purpose: what the units rules find is information
    if m.settings.figure_model {
        for f in findings.iter_mut() {
            f.severity = Severity::Info;
            f.message.push_str(" (figure model: reported as information)");
        }
    }
    if !opts.units_only {
        let timing = std::env::var("DYN_TIMING").is_ok();
        if last >= Stage::C4 {
            let t = std::time::Instant::now();
            findings.extend(base::check(m, opts, ctx.base()));
            if timing {
                eprintln!("    {:>8.0} ms  base checks", t.elapsed().as_secs_f64() * 1000.0);
            }
        }
        for r in rules() {
            if r.stage() <= last {
                let t = std::time::Instant::now();
                findings.extend(r.check(&ctx));
                if timing {
                    eprintln!("    {:>8.0} ms  {}", t.elapsed().as_secs_f64() * 1000.0, r.id());
                }
            }
        }
    }
    // one finding per rule and variable, the confirmed one first
    findings.sort_by_key(|f| (std::cmp::Reverse(f.confirmed), std::cmp::Reverse(f.severity)));
    let mut seen: std::collections::HashSet<(String, Option<String>)> = Default::default();
    findings.retain(|f| seen.insert((f.rule.clone(), f.var.clone())));
    findings.sort_by_key(|f| (family_order(&f.rule), std::cmp::Reverse(f.severity)));
    // a teaching draft shows a weakness on purpose: what the extreme condition experiments find is information
    if m.settings.teaching_draft {
        for f in findings.iter_mut().filter(|f| f.rule.starts_with('X') && f.severity != Severity::Info) {
            f.severity = Severity::Info;
            f.message.push_str(" (teaching draft: reported as information)");
        }
    }
    // a finding that no run has confirmed is only a possibility: the text says so
    for f in findings.iter_mut().filter(|f| !f.confirmed && !f.message.starts_with("Possible")) {
        f.message = format!("Possible: {} (not confirmed by a run)", f.message);
    }
    let gate = grade(&findings, last >= Stage::C5);
    if opts.publish {
        for f in findings.iter_mut() {
            if f.severity == Severity::Warning {
                f.severity = Severity::Error;
            }
        }
    }
    Report { summary: summarise(m), findings, gate: Some(gate) }
}

fn family_order(rule: &str) -> usize {
    "SGUFNIXCRP".find(rule.chars().next().unwrap_or('Z')).unwrap_or(99)
}

/// The grade of stage C7 (Part C7 of the specification). Draft: an error in the families S, G or U (or, with the
/// experiments not run, anywhere); Review: no error anywhere (a "can" finding of the interval analysis that no run has
/// confirmed counts as a warning); Publish: Review, the experiments run and X1 to X5, N3, R1 to R4 and the benchmarks
/// (P3) all clear.
pub fn grade(findings: &[Finding], experiments_ran: bool) -> Gate {
    let real_error = |f: &&Finding| f.severity == Severity::Error && f.confirmed;
    if findings.iter().filter(real_error).any(|f| matches!(f.rule.chars().next(), Some('S' | 'G' | 'U'))) {
        return Gate::Draft;
    }
    if findings.iter().filter(real_error).next().is_some() {
        return Gate::Draft;
    }
    let blocks_publish = |r: &str| matches!(r, "X1" | "X2" | "X3" | "X4" | "X5" | "N3" | "R1" | "R2" | "R3" | "R4" | "P3");
    if experiments_ran && !findings.iter().any(|f| blocks_publish(&f.rule) && f.severity != Severity::Info) {
        Gate::Publish
    } else {
        Gate::Review
    }
}

fn summarise(m: &Model) -> Summary {
    use crate::model::Kind;
    let n = |k: Kind| m.vars.iter().filter(|v| v.kind == k).count();
    let total = m.vars.len().max(1) as f64;
    let mut copy = m.clone();
    let loops = crate::cld::find_loops(&crate::cld::from_model(&mut copy)).len();
    Summary {
        stocks: n(Kind::Stock),
        flows: n(Kind::Flow),
        auxiliaries: n(Kind::Aux),
        constants: n(Kind::Constant),
        lookups: n(Kind::Lookup),
        loops,
        units_coverage_pct: 100.0 * m.vars.iter().filter(|v| !v.units.trim().is_empty()).count() as f64 / total,
        documentation_coverage_pct: 100.0 * m.vars.iter().filter(|v| !v.comment.trim().is_empty()).count() as f64 / total,
    }
}

/// Apply the first-ranked fix of each finding that keeps the numbers (`--fix-all-safe`). Returns the new model and how
/// many fixes were applied.
pub fn apply_safe_fixes(m: &Model, report: &Report) -> (Model, usize) {
    let mut out = m.clone();
    let mut n = 0;
    for f in &report.findings {
        if let Some(fix) = f.fixes.first() {
            if fix.keeps_numbers && !fix.needs_decision && apply_fix(&mut out, fix).is_ok() {
                n += 1;
            }
        }
    }
    (out, n)
}

/// Apply one fix atomically: on any failure the model is left as it was.
pub fn apply_fix(m: &mut Model, fix: &Fix) -> Result<(), String> {
    let mut copy = m.clone();
    for op in &fix.ops {
        crate::ops::apply(&mut copy, op)?;
    }
    *m = copy;
    Ok(())
}

/// The Model Doctor reference page, generated from the rule table so that it cannot drift.
pub fn catalogue_markdown() -> String {
    let mut s = String::from("# Model doctor\n\n> Generated from the rule table in `core/src/doctor/` (`cargo test -p dynamist-core --test doctor_catalogue` rewrites it when `DYN_WRITE_WIKI=1`). Do not edit by hand.\n\n");
    s.push_str("`dyn check FILE` runs the stages C1 to C4 and prints findings, each with exact fixes (`--fix 1,3` applies the first fix of findings 1 and 3, `--fix-all-safe` the first fixes that keep the numbers). `--experiments` adds stage C5, `--gate publish` runs everything and turns warnings into errors, `--stage C3` stops early, `--json` prints the report, `--lenient-time` lets a rate meet its amount.\n\n");
    s.push_str("| Stage | Content |\n|---|---|\n| C1 | parse: names |\n| C2 | graphs: loops, double counting, initial values |\n| C3 | units, signs and intervals |\n| C4 | the base run and numerical health |\n| C5 | automatic experiments (`--experiments`) |\n| C6 | data and calibration (not implemented) |\n| C7 | the report and the gate: Draft (errors), Review (no errors), Publish (no errors and no warnings) |\n\n");
    s.push_str("## Rules\n\nEach finding names its rule and carries the source it comes from.\n\n| Rule | Stage | What it catches |\n|---|---|---|\n");
    let mut rows: Vec<(String, String, String)> = rules().iter().map(|r| (r.id().to_string(), format!("{:?}", r.stage()), r.about().to_string())).collect();
    rows.sort_by_key(|r| (family_order(&r.0), r.0[1..].parse::<u32>().unwrap_or(0)));
    for (id, st, about) in rows {
        s.push_str(&format!("| {id} | {st} | {about} |\n"));
    }
    s.push_str("\nThe catalogue is `wiki/specs/Dynamist_Model_Doctor_Research_and_Rules.md` (Part B); all 72 rules are in the table above. Findings carry the rule id in `rule`, and the older codes of the unit and base checks are named in each rule's text.\n");
    s
}
