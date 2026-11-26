//! `dyn pack FILE --out DIR`: the audit pack, one folder holding what a reviewer needs and an index that links it.

use std::path::Path;

use serde_json::json;

use crate::{files, xlsx};
use dynamist_core::model::Model;

struct Part {
    file: String,
    what: String,
    status: String,
    ok: bool,
}

fn write(dir: &Path, name: &str, text: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::write(dir.join(name), text).map_err(|e| format!("{name}: {e}"))
}

fn add(parts: &mut Vec<Part>, file: &str, what: &str, r: Result<String, String>) {
    match r {
        Ok(note) => parts.push(Part { file: file.into(), what: what.into(), status: if note.is_empty() { "made".into() } else { note }, ok: true }),
        Err(e) => parts.push(Part { file: file.into(), what: what.into(), status: format!("FAILED: {e}"), ok: false }),
    }
}

/// Returns the parts that failed (empty = the whole pack was made).
pub fn run(file: &str, out: &str) -> Result<Vec<String>, String> {
    let dir = Path::new(out);
    std::fs::create_dir_all(dir).map_err(|e| format!("{out}: {e}"))?;
    let (m, loss) = files::open(file)?;
    let name = Path::new(file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| file.to_string());
    let mut parts: Vec<Part> = Vec::new();

    // the doctor report and its gate
    add(&mut parts, "doctor-report.md", "The model doctor: findings with their fixes, and the gate reached", doctor_part(&m, dir));
    // the sanity sheet (the deliver gate)
    add(&mut parts, "sanity-sheet.md", "The sanity sheet of the deliver gate", (|| {
        let rows = dynamist_core::sanity::sheet(&m, 4);
        let delivered = dynamist_core::sanity::delivered(&rows);
        write(dir, "sanity-sheet.md", format!("# Sanity sheet\n\nResult: {}\n\n{}\n", if delivered { "delivered" } else { "not delivered" }, dynamist_core::sanity::markdown(&m, &rows)))?;
        write(dir, "sanity-sheet.json", serde_json::to_string_pretty(&json!({"delivered": delivered, "rows": rows})).unwrap())?;
        Ok(if delivered { String::new() } else { "made; the model is not delivered (see the sheet)".into() })
    })());
    // the model's own checks
    add(&mut parts, "checks.md", "The model's own checks and conservation", (|| {
        let r = dynamist_core::validate::validate_with(&m, &dynamist_core::validate::Options { strict_units: false });
        let issues: Vec<_> = r.issues.iter().filter(|i| matches!(i.code.as_str(), "check" | "conservation" | "equation" | "run")).collect();
        let mut s = format!("# The model's checks\n\n{} check(s) declared. {}\n\n", m.checks.len(), if r.ran { "The base run completed." } else { "The base run did not complete." });
        if issues.is_empty() {
            s.push_str("No check failed and no stock balance was broken.\n");
        } else {
            for i in &issues {
                s.push_str(&format!("- [{}] {}\n", i.code, serde_json::to_value(i).ok().and_then(|v| v.get("message").and_then(|x| x.as_str().map(String::from))).unwrap_or_default()));
            }
        }
        write(dir, "checks.md", s)?;
        Ok(String::new())
    })());
    // the conversion loss report, when the model was imported
    let imported = !loss.items.is_empty() || loss.source.as_deref().is_some_and(|s| s != "sdm");
    if imported {
        add(&mut parts, "conversion-loss.md", "What the import from the original format kept and dropped", (|| {
            let mut s = format!("# Conversion loss report\n\nRead from: {}.\n\n", loss.source.clone().unwrap_or_else(|| "another format".into()));
            s.push_str(&if loss.items.is_empty() { "Nothing was dropped.\n".to_string() } else { format!("{}\n", loss.summary().lines().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n")) });
            write(dir, "conversion-loss.md", s)?;
            Ok(String::new())
        })());
    } else {
        parts.push(Part { file: String::new(), what: "Conversion loss report".into(), status: "not made: the model was not imported".into(), ok: true });
    }
    // the operation log
    let log = format!("{file}.oplog.jsonl");
    if Path::new(&log).exists() {
        add(&mut parts, "operation-log.jsonl", "Every batch of edits made through dyn ops, with actor and reason", std::fs::copy(&log, dir.join("operation-log.jsonl")).map(|_| String::new()).map_err(|e| e.to_string()));
    } else {
        parts.push(Part { file: String::new(), what: "Operation log".into(), status: format!("not made: {name}.oplog.jsonl does not exist (no edit was made through dyn ops)"), ok: true });
    }
    // the workbook
    add(&mut parts, "workbook.xlsx", "The model as an Excel workbook (Cover, Inputs, Scenarios, Series, Checks)", (|| {
        let sheets = dynamist_core::workbook::build(&m, &name);
        write(dir, "workbook.xlsx", xlsx::workbook(&sheets))?;
        Ok(String::new())
    })());
    // the model card
    add(&mut parts, "model-card.md", "Intended use, out of scope, validity, goals and tests, from what the model states", write(dir, "model-card.md", dynamist_core::modelcard::card(&m, &name)).map(|_| String::new()));
    // the reporting checklist (Rahmandad and Sterman 2012): shipped only when the paper's items are in the repository
    parts.push(Part { file: String::new(), what: "Reporting checklist (Rahmandad and Sterman 2012)".into(), status: "not made: the paper's item list is not in the repository yet (papers/TO_FETCH.md)".into(), ok: true });

    let failed: Vec<String> = parts.iter().filter(|p| !p.ok).map(|p| format!("{}: {}", p.what, p.status)).collect();
    let mut idx = format!("# Audit pack: {name}\n\n{}\n\n| Part | What it is | Status |\n|---|---|---|\n", if failed.is_empty() { "Every part was made, or is marked below as not made, with the reason." } else { "Some parts FAILED to generate. They are marked below." });
    for p in &parts {
        let link = if p.file.is_empty() || !p.ok { p.what.clone() } else { format!("[{}]({})", p.file, p.file) };
        idx.push_str(&format!("| {link} | {} | {} |\n", if p.file.is_empty() || !p.ok { String::new() } else { p.what.clone() }, p.status.replace('|', "/")));
    }
    write(dir, "index.md", idx).map_err(|e| format!("index.md: {e}"))?;
    Ok(failed)
}

fn doctor_part(m: &Model, dir: &Path) -> Result<String, String> {
    use dynamist_core::doctor;
    let opts = doctor::Options { experiments: true, jobs: 4, ..Default::default() };
    let report = doctor::check(m, &opts);
    let mut s = format!("# Model doctor\n\nGate reached: {:?}. {}\n\n", report.gate.ok_or("the doctor gave no gate")?, report.line(0));
    for (i, f) in report.findings.iter().enumerate() {
        let word = if f.confirmed { format!("{:?}", f.severity) } else { "possible".to_string() };
        s.push_str(&format!("{}. [{word}] {} {}: {}\n", i + 1, f.code, f.var.as_deref().unwrap_or("-"), f.message));
        for fix in &f.fixes {
            s.push_str(&format!("   - fix: {}{}\n", fix.title, if fix.keeps_numbers { " (keeps the numbers)" } else { "" }));
        }
    }
    if report.findings.is_empty() {
        s.push_str("No findings.\n");
    }
    write(dir, "doctor-report.md", s)?;
    let findings: Vec<_> = report.findings.iter().enumerate().map(|(i, f)| { let mut v = serde_json::to_value(f).unwrap(); v["id"] = json!(i + 1); v }).collect();
    write(dir, "doctor-report.json", serde_json::to_string_pretty(&json!({"summary": report.line(0), "gate": report.gate, "model": report.summary, "findings": findings})).unwrap())?;
    Ok(String::new())
}
