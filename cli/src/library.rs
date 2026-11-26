//! `dyn library`: search and filter the model library (see core/src/catalog.rs).

use crate::{die, files, has_flag, print, take_all, take_flag};
use dynamist_core::catalog::{self, Filter};
use dynamist_core::{ops, runner, validate};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn find_catalog(given: Option<String>) -> PathBuf {
    if let Some(p) = given.or_else(|| std::env::var("DYNAMIST_LIBRARY").ok()) {
        return PathBuf::from(p);
    }
    let mut d = std::env::current_dir().unwrap_or_default();
    loop {
        let c = d.join("library").join("catalog.json");
        if c.exists() {
            return c;
        }
        if !d.pop() {
            die("no library/catalog.json found here or above; pass --catalog FILE or set DYNAMIST_LIBRARY");
        }
    }
}

/// Size facts read from the model file itself, so they cannot go stale.
fn stats(root: &Path, e: &Value) -> Option<Value> {
    let file = e.get("file")?.as_str()?;
    let (m, _) = files::open(&root.join(file).to_string_lossy()).ok()?;
    let i = ops::info(&m);
    Some(json!({"stocks": i["stocks"], "flows": i["flows"], "auxiliaries": i["auxiliaries"], "constants": i["constants"], "lookups": i["lookups"],
        "levers": m.vars.iter().filter(|v| v.lever).count(), "layers": m.layers.len(), "checks": i["checks"]}))
}


/// Every model file under `dir` (recursively) with a format we import.
fn model_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut items: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            model_files(&p, out);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_lowercase().as_str(), "mdl" | "stmx" | "xmile" | "itmx" | "sdm" | "flowmodel") {
                out.push(p);
            }
        }
    }
}

fn slugify(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    t.split('-').filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-")
}

/// `dyn library --ingest FOLDER --out DIR --source TEXT --licence TEXT [--json OUT]`: import every model file found,
/// check it (strict units), run it, save it as .sdm under DIR, and print one catalogue entry per file. Nothing is
/// invented: tags stay empty and the entry says "needs tags".
fn ingest(folder: &str, out_dir: &str, source: String, licence: String, json_out: Option<String>) {
    let root = PathBuf::from(folder);
    let mut found = vec![];
    model_files(&root, &mut found);
    std::fs::create_dir_all(out_dir).unwrap_or_else(|e| die(format!("{out_dir}: {e}")));
    let mut entries = vec![];
    let (mut ok_import, mut ok_run) = (0, 0);
    for f in &found {
        let rel = f.strip_prefix(&root).unwrap_or(f).to_string_lossy().to_string();
        let slug = slugify(&rel.rsplit_once('.').map(|x| x.0).unwrap_or(&rel).to_string());
        let mut e = json!({"slug": slug, "title": f.file_stem().and_then(|s| s.to_str()).unwrap_or("").replace(['_', '-'], " "), "status": "planned", "needs": ["tags", "question", "authors", "year"],
            "source_file": rel, "source": source, "licence": licence, "format": f.extension().and_then(|e| e.to_str()).unwrap_or("")});
        // a file that panics inside the importer must not stop the sweep
        let path = f.to_string_lossy().to_string();
        let res = std::panic::catch_unwind(|| files::open(&path));
        match res {
            Ok(Ok((m, loss))) if !m.vars.is_empty() => {
                ok_import += 1;
                let i = ops::info(&m);
                e["stats"] = json!({"stocks": i["stocks"], "flows": i["flows"], "auxiliaries": i["auxiliaries"], "constants": i["constants"], "lookups": i["lookups"]});
                e["loss_items"] = json!(loss.items.len());
                let v = validate::validate_with(&m, &validate::Options { strict_units: false });
                e["validate_errors"] = json!(v.errors().count());
                match std::panic::catch_unwind(|| runner::run_with(&m, &[])) {
                    Ok(Ok(_)) => {
                        ok_run += 1;
                        e["runs"] = json!(true);
                    }
                    Ok(Err(err)) => {
                        e["runs"] = json!(false);
                        e["run_error"] = json!(err.chars().take(160).collect::<String>());
                    }
                    Err(_) => e["runs"] = json!(false),
                }
                let target = Path::new(out_dir).join(format!("{slug}.sdm"));
                if files::save(&m, &target.to_string_lossy(), Some("sdm")).is_ok() {
                    e["file"] = json!(target.to_string_lossy());
                    if e["runs"] == json!(true) {
                        e["status"] = json!("built");
                    }
                }
            }
            Ok(Ok(_)) => e["import_error"] = json!("the file was read but holds no variables"),
            Ok(Err(err)) => e["import_error"] = json!(err.chars().take(160).collect::<String>()),
            Err(_) => e["import_error"] = json!("the importer stopped on this file"),
        }
        entries.push(e);
    }
    let summary = json!({"files": found.len(), "imported": ok_import, "run": ok_run});
    if let Some(p) = json_out {
        std::fs::write(&p, serde_json::to_string_pretty(&json!({"summary": summary, "entries": entries})).unwrap()).unwrap_or_else(|e| die(format!("{p}: {e}")));
    }
    print(json!({"summary": summary, "failed": entries.iter().filter(|e| e.get("import_error").is_some()).map(|e| json!({"file": e["source_file"], "why": e["import_error"]})).collect::<Vec<_>>()}));
}

pub fn run(mut args: Vec<String>) {
    if let Some(folder) = take_flag(&mut args, "--ingest") {
        let out = take_flag(&mut args, "--out").unwrap_or_else(|| die("--ingest FOLDER needs --out DIR (where the .sdm files go)"));
        let source = take_flag(&mut args, "--source").unwrap_or_else(|| die("--ingest needs --source TEXT (where these files come from)"));
        let licence = take_flag(&mut args, "--licence").unwrap_or_else(|| die("--ingest needs --licence TEXT (the terms the files are under; write \"unverified\" if you do not know)"));
        ingest(&folder, &out, source, licence, take_flag(&mut args, "--json"));
        return;
    }
    let cat_path = find_catalog(take_flag(&mut args, "--catalog"));
    let text = std::fs::read_to_string(&cat_path).unwrap_or_else(|e| die(format!("{}: {e}", cat_path.display())));
    let catalog: Value = serde_json::from_str(&text).unwrap_or_else(|e| die(format!("{}: {e}", cat_path.display())));
    let root = cat_path.parent().and_then(|p| p.parent()).unwrap_or(Path::new(".")).to_path_buf();

    if has_flag(&mut args, "--check") {
        let mut problems = catalog::check(&catalog);
        for e in catalog::entries(&catalog) {
            for k in ["file", "tutorial"] {
                if let Some(f) = e.get(k).and_then(|f| f.as_str()) {
                    if !root.join(f).exists() {
                        problems.push(format!("{}: {k} {f} does not exist", e["slug"].as_str().unwrap_or("?")));
                    }
                }
            }
        }
        print(json!({"ok": problems.is_empty(), "entries": catalog::entries(&catalog).len(), "problems": problems}));
        std::process::exit(if problems.is_empty() { 0 } else { 1 });
    }

    let num = |s: Option<String>, what: &str| s.map(|v| v.parse::<i64>().unwrap_or_else(|_| die(format!("{what} needs a whole number"))));
    let f = Filter {
        text: take_all(&mut args, "--text"),
        category: take_all(&mut args, "--category"),
        domain: take_all(&mut args, "--domain"),
        skills: take_all(&mut args, "--skill"),
        features: take_all(&mut args, "--feature"),
        tag: take_all(&mut args, "--tag"),
        author: take_flag(&mut args, "--author"),
        level: take_flag(&mut args, "--level"),
        status: take_flag(&mut args, "--status"),
        year_from: num(take_flag(&mut args, "--year-from"), "--year-from"),
        year_to: num(take_flag(&mut args, "--year-to"), "--year-to"),
        max_simplicity: num(take_flag(&mut args, "--max-simplicity"), "--max-simplicity"),
        max_difficulty: num(take_flag(&mut args, "--max-difficulty"), "--max-difficulty"),
    };
    let want_facets = has_flag(&mut args, "--facets");
    let want_stats = has_flag(&mut args, "--stats");
    let slug = take_flag(&mut args, "--slug");
    if let Some(a) = args.first() {
        die(format!("unexpected “{a}” (dyn library [--text W]… [--category C] [--domain D] [--skill S] [--feature F] [--tag T] [--author A] [--level L] [--status built|planned] [--year-from Y] [--year-to Y] [--max-simplicity N] [--max-difficulty N] [--facets] [--stats] [--check])"));
    }
    let mut found = catalog::query(&catalog, &f);
    if let Some(s) = slug {
        found.retain(|e| e["slug"] == s.as_str());
    }
    if want_stats {
        for e in found.iter_mut() {
            if let Some(s) = stats(&root, e) {
                e["stats"] = s;
            }
        }
    }
    let mut out = json!({"count": found.len(), "models": found});
    if want_facets {
        out["facets"] = catalog::facets(&catalog, &found);
        out["vocab"] = catalog["vocab"].clone();
    }
    print(out);
}
