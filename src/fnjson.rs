//! `functions.json`: one entry per function of the equation language, generated from the function
//! table (`expr::FUNCTIONS`, `expr::ALIASES`), the reference (`docs`), the stateful list of the
//! engine (`core/src/sim.rs`) and a probe of the exporters. Nothing here is typed per function except
//! the unit rules, which mirror the match in `doctor::units_rules::Engine::infer`.
//!
//! Run it with `dynamist --write-functions functions.json`, or `DYN_WRITE_FUNCTIONS=functions.json
//! cargo test -p dynamist --lib functions_json`.

use dynamist_core::expr::{self, FnInfo, ALIASES, FUNCTIONS};
use dynamist_core::io::{self, Format};
use dynamist_core::loss::{LossItem, LossKind};
use dynamist_core::model::{Kind, Model, Var};
use serde_json::{json, Value};

/// The engine source: the names compiled to a stateful node (`"NAME" => Some(stateful(SKind::…`).
const SIM_SRC: &str = include_str!("../core/src/sim.rs");

/// Functions that expand to stateful ones before the engine sees them (the verbs and the delays of
/// a given order), or that keep state in another way.
const RUNNING_EXTRA: &[&str] = &[
    "DELAY", "VALUE", "LEVELISED", "LOAN", "TAX", "QUEUE", "WAIT", "AGE", "EXPECT", "DELAYINF", "DELAYMTR",
    "QUEUE AGE AVERAGE", "QUEUE AGE OLDEST", "QUEUE AGE IN RANGE", "SMOOTH N", "DELAYN",
];

/// Names the engine compiles to a state node, read from `sim.rs`.
fn stateful_names() -> Vec<String> {
    let mut out = Vec::new();
    for line in SIM_SRC.lines() {
        let t = line.trim();
        if t.starts_with('"') && t.contains("=> Some(stateful(") {
            if let Some(end) = t[1..].find('"') {
                out.push(t[1..1 + end].to_string());
            }
        }
    }
    out
}

/// "Result unit" in one sentence, as `Engine::infer` (core/src/doctor/units_rules.rs) decides it.
pub fn unit_rule(name: &str) -> &'static str {
    match name {
        "MIN" | "MAX" => "unit of the arguments, which must agree",
        "IF THEN ELSE" => "unit of the two branches, which must agree",
        "SMOOTH" | "SMOOTHI" | "SMOOTH3" | "SMOOTH3I" | "DELAY1" | "DELAY1I" | "DELAY3" | "DELAY3I" | "DELAY FIXED" | "DELAYINF" | "DELAYMTR" | "TREND" | "ABS" | "ROUND" | "INTEGER" | "CEILING" | "CLAMP" | "TOTAL" | "AVERAGE" | "MEAN" | "MEDIAN" | "STEP" | "DELAY" => "unit of the first argument",
        "ZIDZ" | "XIDZ" => "numerator over denominator: unit of the first argument divided by unit of the second",
        "SQRT" => "square root of the unit of the argument (every exponent must be even)",
        "PMT" | "FV" | "PV" => "unit of the third argument (the present value)",
        "IPMT" | "PPMT" => "unit of the fourth argument (the present value)",
        "REAL RATE" | "NOMINAL RATE" | "EFFECTIVE RATE" | "WACC" | "ANNUITY FACTOR" | "NPER" | "RATE" | "DSCR" => "dimensionless",
        "LN" | "LOG10" | "EXP" | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ATAN" | "SINH" | "COSH" | "TANH" => "dimensionless",
        "RAMP" => "unit of the slope times the time unit",
        _ => "not checked",
    }
}

/// The names written "(alias X)" or "(aliases X, Y)" in the reference of a function.
fn doc_aliases(summary: &str) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["alias ", "aliases "] {
        if let Some(i) = summary.find(key) {
            let rest = &summary[i + key.len()..];
            let end = rest.find([')', '.', ';']).unwrap_or(rest.len());
            for a in rest[..end].split([',', ' ']).map(str::trim).filter(|a| !a.is_empty() && *a != "and") {
                if a.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
                    out.push(a.to_string());
                }
            }
        }
    }
    out
}

fn aliases_of(name: &str, summary: &str) -> Vec<String> {
    let mut v: Vec<String> = ALIASES.iter().filter(|(_, t)| *t == name).map(|(a, _)| a.to_string()).collect();
    if name == "XIDZ" || name == "ZIDZ" {
        v.push("SAFEDIV".into());
    }
    for a in doc_aliases(summary) {
        if !v.contains(&a) {
            v.push(a);
        }
    }
    v
}

/// The variable names and lookup names an expression refers to.
fn refs(e: &expr::Expr, out: &mut Vec<String>) {
    use expr::Expr::*;
    match e {
        Num(_) => {}
        Var(n) => out.push(n.clone()),
        Neg(a) | Not(a) => refs(a, out),
        Bin(_, a, b) => {
            refs(a, out);
            refs(b, out);
        }
        Call(_, args) => args.iter().for_each(|a| refs(a, out)),
        LookupCall(n, a) => {
            out.push(n.clone());
            refs(a, out);
        }
        WithLookup(a, _) => refs(a, out),
    }
}

/// A one-variable model that uses `f`: the reference's example when it parses and calls `f` (its
/// variables are added as constants), else a call with as many 1s as the function needs.
fn probe(f: &FnInfo, probe_var: &str) -> Model {
    let mut m = Model::default();
    let mut equation = format!("{}({})", f.name, vec!["1"; f.min.max(1)].join(", "));
    let mut names: Vec<String> = Vec::new();
    if let Some(d) = crate::docs::find(f.name) {
        if let Ok(e) = expr::parse(d.example) {
            if matches!(&e, expr::Expr::Call(n, _) if n == f.name) {
                equation = d.example.to_string();
                refs(&e, &mut names);
            }
        }
    }
    names.sort();
    names.dedup();
    let mut id = 1;
    for n in names {
        let mut v = Var::new(id, &n, Kind::Constant, [0.0, 0.0]);
        v.equation = "1".into();
        m.vars.push(v);
        id += 1;
    }
    let mut v = Var::new(id, probe_var, Kind::Aux, [0.0, 0.0]);
    v.equation = equation;
    m.vars.push(v);
    m
}

/// What an export does with a call of `f`: ("native" | "raw formula" | "dropped", the loss note). The probe
/// is a one-variable model; the loss items the same model without the call also has are left out.
fn export_of(f: &FnInfo, format: Format) -> (&'static str, String) {
    let m = probe(f, "Probe");
    let mut base = m.clone();
    if let Some(v) = base.vars.last_mut() {
        v.equation = "1".into();
    }
    let Ok((_, loss)) = io::write_str(&m, format) else { return ("dropped", "the writer refused the model".into()) };
    let base_loss = io::write_str(&base, format).map(|x| x.1.items).unwrap_or_default();
    let mine: Vec<&LossItem> = loss.items.iter().filter(|i| !base_loss.contains(i)).collect();
    let Some(worst) = mine.iter().max_by_key(|i| rank(i.kind)) else { return ("native", String::new()) };
    let word = match worst.kind {
        LossKind::Dropped => "dropped",
        _ => "raw formula",
    };
    (word, worst.detail.clone())
}

fn rank(k: LossKind) -> u8 {
    match k {
        LossKind::Dropped => 3,
        LossKind::PreservedNotSimulable => 2,
        _ => 1,
    }
}

/// The whole file: a JSON array, one object per function, in table order.
pub fn functions_json() -> String {
    let stateful = stateful_names();
    let entries: Vec<Value> = FUNCTIONS
        .iter()
        .map(|f| {
            let doc = crate::docs::find(f.name);
            let summary = doc.map(|d| d.summary).unwrap_or("");
            let eq = crate::docs::equivalents(f.name);
            let running = stateful.iter().any(|s| s == f.name) || RUNNING_EXTRA.contains(&f.name);
            let (ev, ev_note) = export_of(f, Format::Mdl);
            let (ex, ex_note) = export_of(f, Format::Xmile);
            json!({
                "name": f.name,
                "aliases": aliases_of(f.name, summary),
                "signature": f.signature,
                "group": doc.map(|d| d.group).unwrap_or(f.group),
                "kind": if running { "running" } else { "pure" },
                "min_args": f.min,
                "max_args": f.max,
                "result_unit_rule": unit_rule(f.name),
                "help": f.help,
                "excel": crate::docs::excel_equivalent(f.name).unwrap_or(""),
                "vensim": eq.map(|e| e.1).unwrap_or(""),
                "stella": eq.map(|e| e.2).unwrap_or(""),
                "export_vensim": ev,
                "export_vensim_note": ev_note,
                "export_xmile": ex,
                "export_xmile_note": ex_note,
            })
        })
        .collect();
    let _ = expr::function_info("MIN");
    serde_json::to_string_pretty(&entries).expect("json") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn functions_json_covers_the_table_and_the_new_functions() {
        let text = functions_json();
        let v: Vec<Value> = serde_json::from_str(&text).unwrap();
        assert_eq!(v.len(), FUNCTIONS.len());
        let get = |n: &str| v.iter().find(|e| e["name"] == n).unwrap_or_else(|| panic!("{n} missing"));
        for n in ["PMT", "LEVELISED", "LOAN", "RANDOM TRIANGULAR", "DAYS IN STEP", "YEAR FRACTION", "NORMAL INV"] {
            get(n);
        }
        assert_eq!(get("PMT")["kind"], "pure");
        assert_eq!(get("LOAN")["kind"], "running");
        assert_eq!(get("SMOOTH")["kind"], "running");
        assert_eq!(get("LEVELISED")["aliases"], json!(["LCOE"]));
        assert_eq!(get("PMT")["result_unit_rule"], "unit of the third argument (the present value)");
        assert_eq!(get("PMT")["excel"].as_str().unwrap().starts_with("PMT("), true);
        assert_eq!(get("MIN")["export_vensim"], "native");
        assert!(get("GAMMA")["help"].as_str().unwrap().len() > 3);
        for e in &v {
            assert!(matches!(e["export_vensim"].as_str(), Some("native" | "raw formula" | "dropped")));
            assert!(matches!(e["export_xmile"].as_str(), Some("native" | "raw formula" | "dropped")));
        }
        if let Ok(path) = std::env::var("DYN_WRITE_FUNCTIONS") {
            std::fs::write(path, text).unwrap();
        }
    }
}
