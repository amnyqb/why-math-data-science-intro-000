//! Everything that can be said about a model without a person looking at it: equations that do
//! not read, arrows that disagree with equations, units that do not fit, limits the model states
//! about itself, stocks that do not balance. One call, structured answers with severity, the
//! variable involved and a short code, so a script, a CI job or an AI agent can act on them.

use serde::{Deserialize, Serialize};

use crate::expr;
use crate::model::{Kind, Model};
use crate::sim;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// The model cannot run, or a stated limit is broken.
    Error,
    /// Probably a mistake, but the model runs.
    Warning,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    /// "equation", "arrow-unused", "arrow-missing", "stock-no-flow", "units", "check", "conservation".
    pub code: String,
    pub var: Option<String>,
    pub var_id: Option<u64>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub issues: Vec<Issue>,
    /// Did the model compile and run (so that checks could be tested)?
    pub ran: bool,
}

impl Report {
    pub fn errors(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }
    pub fn ok(&self) -> bool {
        self.errors().next().is_none()
    }
    pub fn summary(&self) -> String {
        if self.issues.is_empty() {
            return "No problems.".into();
        }
        self.issues.iter().map(|i| format!("{:?} [{}] {}", i.severity, i.code, i.message)).collect::<Vec<_>>().join("\n")
    }
}

/// Advice that does not stop the run: arrows not used, equations using what has no arrow,
/// stocks with no flows, and unit mismatches. (id of the variable, message)
pub fn suggestions(m: &Model) -> Vec<(u64, String)> {
    suggestions_coded(m, false).into_iter().map(|(id, _, msg)| (id, msg)).collect()
}

/// How strictly to check.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// A rate may not meet its amount: a flow's units times the time unit must be its stock's.
    pub strict_units: bool,
}

/// The unit problems of a model, from the same exact engine as `dyn check` (`doctor::units_rules`): validate, the doctor
/// and the gate of the operations all say the same. The default mode is lenient about the time unit (a rate may meet its
/// amount, and a flow whose time unit is not the model's is not reported); strict mode is the doctor's own reading.
/// Notes that only inform (a percent read as a fraction, an inferred unit) are left to the doctor.
fn unit_findings(m: &Model, strict_units: bool) -> Vec<(u64, String)> {
    let opts = crate::doctor::Options { lenient_time: !strict_units, ..Default::default() };
    crate::doctor::units_rules::check(m, &opts)
        .into_iter()
        .filter(|f| f.severity != crate::doctor::Severity::Info)
        .filter(|f| strict_units || !f.message.starts_with("Flow “"))
        .map(|f| (f.var.as_deref().and_then(|n| m.by_name(&expr::normalize(n))).map(|v| v.id).unwrap_or(0), f.message))
        .collect()
}

fn suggestions_coded(m: &Model, strict_units: bool) -> Vec<(u64, &'static str, String)> {
    let mut out: Vec<(u64, &'static str, String)> = unit_findings(m, strict_units).into_iter().map(|(id, msg)| (id, "units", msg)).collect();
    for v in &m.vars {
        let mut refs = Vec::new();
        if let Ok(e) = expr::parse(&v.equation) {
            expr::references(&e, &mut refs);
        }
        for l in m.links.iter().filter(|l| l.to == v.id) {
            if let Some(s) = m.var(l.from) {
                if !refs.contains(&expr::normalize(&s.name)) && !(v.kind == Kind::Lookup && v.equation.trim().is_empty()) {
                    out.push((v.id, "arrow-unused", format!("Arrow from '{}' to '{}', but the equation doesn't use it", s.name, v.name)));
                }
            }
        }
        for r in &refs {
            if let Some(s) = m.by_name(r) {
                if s.id != v.id && !m.links.iter().any(|l| l.from == s.id && l.to == v.id) {
                    out.push((v.id, "arrow-missing", format!("'{}' uses '{}' but there's no arrow between them", v.name, s.name)));
                }
            }
        }
        if v.kind == Kind::Stock && !m.vars.iter().any(|f| f.kind == Kind::Flow && (f.from == Some(v.id) || f.to == Some(v.id))) {
            out.push((v.id, "stock-no-flow", format!("Stock '{}' has no flows, so it never changes", v.name)));
        }
    }
    for (inst, msg) in crate::modules::unit_warnings(m) {
        out.push((0, "units", format!("Instance '{inst}': {msg}")));
    }
    out
}

/// Check a model completely: compile it, run it, and test every check against the run.
pub fn validate(m: &Model) -> Report {
    validate_with(m, &Options::default())
}

/// As `validate`, with options.
pub fn validate_with(m: &Model, opts: &Options) -> Report {
    let mut r = Report::default();
    let id_of = |name: &str| m.by_name(&expr::normalize(name)).map(|v| v.id);
    match sim::compile(m) {
        Err(problems) => {
            for p in problems {
                r.issues.push(Issue { severity: Severity::Error, code: "equation".into(), var_id: p.var.as_deref().and_then(id_of), var: p.var, message: p.message });
            }
        }
        Ok(c) => match sim::simulate(&c, m, "validate") {
            Err(e) => r.issues.push(Issue { severity: Severity::Error, code: "run".into(), var: None, var_id: None, message: e }),
            Ok(run) => {
                r.ran = true;
                for f in crate::checks::all(m, &run) {
                    let code = if f.message.starts_with("Stock ") { "conservation" } else { "check" };
                    r.issues.push(Issue { severity: Severity::Error, code: code.into(), var: f.var.and_then(|id| m.var(id)).map(|v| v.name.clone()), var_id: f.var, message: f.message });
                }
            }
        },
    }
    for (var, message) in crate::manifest::check(m) {
        r.issues.push(Issue { severity: Severity::Error, code: "manifest".into(), var_id: var.as_deref().and_then(id_of), var, message });
    }
    for i in crate::layers::check(m).issues {
        let severity = if i.severity == crate::layers::Sev::Error && i.code != "layer-unassigned" { Severity::Error } else { Severity::Warning };
        r.issues.push(Issue { severity, code: i.code, var_id: i.var.as_deref().and_then(id_of), var: i.var, message: i.message });
    }
    for (id, code, message) in suggestions_coded(m, opts.strict_units) {
        r.issues.push(Issue { severity: Severity::Warning, code: code.into(), var: m.var(id).map(|v| v.name.clone()), var_id: Some(id), message });
    }
    r
}

/// An issue with what to do about it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Explained {
    pub code: String,
    pub severity: Severity,
    pub var: Option<String>,
    pub message: String,
    pub hint: String,
}

fn hint_for(code: &str) -> &'static str {
    match code {
        "equation" => "Check the spelling of every name in the equation (case and spaces do not matter), balance the brackets, and give every stock a start value and every flow an equation. Fix it with set_equation. For a module instance, check the parameter and port names against the module.",
        "run" => "The equations read but the run failed, for example a loop of variables with no stock in it. Put a stock or a delay function in the loop.",
        "check" => "A limit the model states about itself is broken in this run. Either change the parameters or the policy so it holds, or, if the limit is wrong, replace it with remove_check and add_check.",
        "conservation" => "A stock changed by something other than the flows attached to it. Look for flows attached to a different set than the stock, or a stock that is also written to by an equation; stocks held non-negative are not checked.",
        "arrow-unused" => "An arrow joins two variables, but the equation does not use the source. Remove the arrow with disconnect, or use the variable in the equation.",
        "arrow-missing" => "An equation uses a variable that has no arrow to it. Add it with connect (arrows are made automatically when you set an equation through the operations).",
        "stock-no-flow" => "Nothing fills or drains this stock, so it never changes. Add a flow with add_var and from or to, or remove the stock.",
        "units" => "Two things of different kinds are added, compared or declared equal. Make both sides the same kind of thing, or correct the declared units (set_units). A rate and its amount (MB and MB/Day) only clash when strict units is on.",
        "manifest" => "The manifest names something the model does not have. Correct the name, or remove the entry (set_manifest).",
        _ => "No advice for this code.",
    }
}

/// Every issue of a report with a plain-language hint on how to fix it.
pub fn explain(r: &Report) -> Vec<Explained> {
    r.issues.iter().map(|i| Explained { code: i.code.clone(), severity: i.severity, var: i.var.clone(), message: i.message.clone(), hint: hint_for(&i.code).into() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Var;

    fn tank() -> Model {
        let mut m = Model::default();
        m.settings.final_time = 5.0;
        let mut s = Var::new(1, "Tank", Kind::Stock, [0.0, 0.0]);
        s.equation = "10".into();
        let mut f = Var::new(2, "Burn", Kind::Flow, [0.0, 0.0]);
        f.equation = "2".into();
        f.from = Some(1);
        m.vars = vec![s, f];
        m.next_id = 10;
        m
    }

    #[test]
    fn a_sound_model_has_no_errors() {
        let r = validate(&tank());
        assert!(r.ok() && r.ran, "{}", r.summary());
    }

    #[test]
    fn a_broken_equation_is_an_error_with_its_variable() {
        let mut m = tank();
        m.vars[1].equation = "2 +".into();
        let r = validate(&m);
        assert!(!r.ok() && !r.ran);
        let e = r.errors().next().unwrap();
        assert_eq!(e.code, "equation");
        assert_eq!(e.var.as_deref(), Some("Burn"));
        assert_eq!(e.var_id, Some(2));
    }

    #[test]
    fn strict_units_and_unreadable_units_are_reported_and_every_issue_has_advice() {
        let mut m = tank();
        m.settings.time_units = "Day".into();
        m.vars[0].units = "Unit".into();
        m.vars[1].units = "Unit/Month".into(); // a flow in other time units than the model's
        assert!(validate(&m).issues.iter().all(|i| i.code != "units"), "lenient by default");
        let strict = validate_with(&m, &Options { strict_units: true });
        assert!(strict.issues.iter().any(|i| i.code == "units"), "{}", strict.summary());
        m.vars[1].units = "Unit (approx)/Day".into();
        let v = validate(&m);
        assert!(v.issues.iter().any(|i| i.code == "units" && i.message.contains("cannot be read")), "{}", v.summary());
        for e in explain(&v) {
            assert!(!e.hint.starts_with("No advice"), "{}", e.code);
        }
    }

    #[test]
    fn strict_validate_and_the_doctor_say_the_same_about_units() {
        let mut m = tank();
        m.settings.time_units = "Day".into();
        m.vars[0].units = "kg".into();
        m.vars[1].units = "g/Day".into(); // a factor 1000 apart from its stock
        let strict = validate_with(&m, &Options { strict_units: true });
        let doctor = crate::doctor::check(&m, &crate::doctor::Options { units_only: true, ..Default::default() });
        let n = |code: &str| doctor.findings.iter().filter(|f| f.code == code).count();
        assert!(n("unit-scale-mismatch") >= 1);
        assert_eq!(strict.issues.iter().filter(|i| i.code == "units").count(), doctor.findings.iter().filter(|f| f.severity != crate::doctor::Severity::Info).count(), "{}", strict.summary());
    }

    #[test]
    fn a_broken_limit_is_an_error_and_a_stock_without_flows_a_warning() {
        let mut m = tank();
        m.checks.push(crate::checks::Check { name: "Never empty".into(), var: "Tank".into(), min: Some(5.0), max: None, when: crate::checks::CheckWhen::Always, ..Default::default() });
        let r = validate(&m);
        assert!(r.errors().any(|i| i.code == "check"), "{}", r.summary());
        let mut lone = tank();
        lone.vars.remove(1);
        let r = validate(&lone);
        assert!(r.ok() && r.issues.iter().any(|i| i.code == "stock-no-flow"), "{}", r.summary());
    }
}
