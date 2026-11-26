//! Models in and out of spreadsheets, using the template described in
//! `wiki/Spreadsheet-Template.md`: one sheet each for Settings, Parameters, Stocks, Flows,
//! Variables, Lookups, Links, Scenarios and TimeSeries. Column names are matched loosely
//! (case, spaces and common synonyms don't matter), so a hand-made workbook works too.
#[cfg(not(feature = "gui"))]
use crate::rfd;

use std::collections::HashMap;
use std::path::Path;

use crate::expr;
use crate::model::{Change, ItemSet, ItemValue, Kind, Method, Model, Rollup, Scenario, Sheet, SheetKind, TableMode, Var};


/// A workbook as plain text: (sheet name, rows of cells).
#[derive(Clone, Debug, Default)]
pub struct Book {
    pub sheets: Vec<(String, Vec<Vec<String>>)>,
}

/// The sheets of the template, in order, with their columns.
pub const TEMPLATE: &[(&str, &[&str])] = &[
    ("Settings", &["Setting", "Value"]),
    ("Parameters", &["Name", "Value", "Units", "Min", "Max", "Description"]),
    ("Stocks", &["Name", "Initial value", "Units", "Never negative", "Description"]),
    ("Flows", &["Name", "Equation", "From stock", "To stock", "Units", "Description"]),
    ("Variables", &["Name", "Equation", "Units", "Description"]),
    ("Lookups", &["Name", "Input", "Points", "Units", "Description"]),
    ("Links", &["Cause", "Effect", "Polarity", "Delay", "Notes"]),
    ("Scenarios", &["Scenario", "Variable", "Value"]),
    ("TimeSeries", &["Time"]),
];

fn norm(s: &str) -> String {
    s.trim().to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Sheet names people use for each template sheet.
fn sheet_aliases(name: &str) -> &'static [&'static str] {
    match name {
        "Settings" => &["settings", "setting", "simulation", "control", "timesettings"],
        "Parameters" => &["parameters", "parameter", "constants", "constant", "inputs", "assumptions"],
        "Stocks" => &["stocks", "stock", "levels", "level", "accumulations"],
        "Flows" => &["flows", "flow", "rates", "rate"],
        "Variables" => &["variables", "variable", "auxiliaries", "auxiliary", "aux", "calculations", "formulas"],
        "Lookups" => &["lookups", "lookup", "tables", "graphicalfunctions", "curves"],
        "Links" => &["links", "link", "relationships", "causallinks", "arrows", "cld", "causalloop"],
        "Scenarios" => &["scenarios", "scenario", "whatifs", "cases"],
        "TimeSeries" => &["timeseries", "data", "historicaldata", "series"],
        "Sets" => &["sets", "set", "itemsets", "lists", "families", "dimensions", "subscripts", "entities"],
        "Values" => &["values", "byitem", "itemvalues", "peritem", "valuesbyitem"],
        "ItemSeries" => &["itemseries", "itemdata", "dataperitem", "seriesbyitem", "timeseriesbyitem"],
        "Modules" => &["modules", "subsystems", "parts", "areas"],
        "Functions" => &["functions", "customfunctions", "userfunctions", "macros", "formulas2"],
        "Checks" => &["checks", "check", "tests", "limits", "assertions", "expectations", "referencebehaviour", "referencebehavior"],
        _ => &[],
    }
}

/// Column names people use for each field.
fn col_aliases(field: &str) -> &'static [&'static str] {
    match field {
        "name" => &["name", "check", "parameter", "variable", "stock", "flow", "item", "lookup"],
        "when" => &["when", "applies", "appliesat", "at", "timing", "checkat"],
        "checkkind" => &["kind", "type", "checkkind", "checktype"],
        "tolerance" => &["tolerance", "tol", "band", "allowed"],
        "reference" => &["reference", "referencepoints", "expected", "referenceseries"],
        "stocks" => &["stocks", "stocklist", "balanceof"],
        "inflows" => &["inflows", "in", "entering", "boundaryin"],
        "outflows" => &["outflows", "out", "leaving", "boundaryout"],
        "value" => &["value", "default", "base", "basevalue", "amount", "number"],
        "units" => &["units", "unit", "uom"],
        "min" => &["min", "minimum", "low", "lower"],
        "max" => &["max", "maximum", "high", "upper"],
        "description" => &["description", "notes", "note", "comment", "comments", "definition", "meaning"],
        "equation" => &["equation", "formula", "expression", "definition"],
        "initial" => &["initialvalue", "initial", "start", "startvalue", "value"],
        "nonneg" => &["nevernegative", "nonnegative", "positiveonly"],
        "from" => &["fromstock", "from", "source", "drains"],
        "to" => &["tostock", "to", "destination", "fills"],
        "input" => &["input", "inputvariable", "x"],
        "points" => &["points", "table", "pairs", "data"],
        "cause" => &["cause", "from", "source"],
        "effect" => &["effect", "to", "target"],
        "polarity" => &["polarity", "sign", "direction"],
        "delay" => &["delay", "delayed"],
        "scenario" => &["scenario", "case", "scenarioname"],
        "var" => &["variable", "parameter", "lever", "name", "changes"],
        "setting" => &["setting", "name", "parameter"],
        "per" => &["per", "set", "foreach", "appliesto", "perset", "for"],
        "combine" => &["combine", "combineas", "rollup", "showas", "summary"],
        "setname" => &["set", "setname", "list", "family", "dimension", "kind", "type"],
        "item" => &["item", "member", "element", "items"],
        "belongs" => &["belongsto", "group", "parent", "within", "partof"],
        "time" => &["time", "t", "day", "x"],
        "module" => &["module", "area", "subsystem", "level", "section", "part"],
        "lever" => &["lever", "keylever", "control", "keycontrol"],
        "params" => &["parameters", "params", "arguments", "args"],
        "body" => &["body", "formula", "equation", "expression", "definition"],
        _ => &[],
    }
}

impl Book {
    /// Rows of a template sheet (by name or alias), without its header row, plus a column finder.
    fn sheet(&self, name: &str) -> Option<Table<'_>> {
        let aliases = sheet_aliases(name);
        let (_, rows) = self.sheets.iter().find(|(n, _)| aliases.contains(&norm(n).as_str()) || norm(n) == norm(name))?;
        // the header is the first row with at least one non-empty cell
        let h = rows.iter().position(|r| r.iter().any(|c| !c.trim().is_empty()))?;
        Some(Table { header: rows[h].iter().map(|c| norm(c)).collect(), rows: rows[h + 1..].iter().filter(|r| r.iter().any(|c| !c.trim().is_empty())).collect() })
    }
}

struct Table<'a> {
    header: Vec<String>,
    rows: Vec<&'a Vec<String>>,
}

impl Table<'_> {
    /// Index of the column for `field`, trying its aliases in order.
    fn col(&self, field: &str) -> Option<usize> {
        col_aliases(field).iter().find_map(|a| self.header.iter().position(|h| h == a))
    }
    fn get<'r>(&self, row: &'r [String], field: &str) -> &'r str {
        self.col(field).and_then(|i| row.get(i)).map(|s| s.trim()).unwrap_or("")
    }
}

// ------------------------------------------------------------------ reading files

fn cell_text(d: &calamine::Data) -> String {
    use calamine::Data;
    match d {
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Bool(b) => if *b { "yes".into() } else { "no".into() },
        Data::DateTime(x) => format!("{}", x.as_f64()),
        Data::Error(_) | Data::Empty => String::new(),
    }
}

/// Read an Excel (.xlsx/.xlsm/.xls), OpenDocument (.ods) or CSV file.
pub fn read_book(path: &Path) -> Result<Book, String> {
    // a folder of CSV files: every file is a sheet named after the file
    if path.is_dir() {
        let mut files: Vec<_> = std::fs::read_dir(path).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("csv"))).collect();
        files.sort();
        let mut sheets = Vec::new();
        for f in files {
            let text = std::fs::read_to_string(&f).map_err(|e| e.to_string())?;
            sheets.push((f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(), parse_delimited(&text)));
        }
        if sheets.is_empty() {
            return Err("The folder has no .csv files".into());
        }
        return Ok(Book { sheets });
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "csv" || ext == "tsv" || ext == "txt" {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        return Ok(Book { sheets: vec![(name, parse_delimited(&text))] });
    }
    use calamine::Reader;
    let mut wb = calamine::open_workbook_auto(path).map_err(|e| e.to_string())?;
    let mut sheets = Vec::new();
    for name in wb.sheet_names() {
        let range = wb.worksheet_range(&name).map_err(|e| e.to_string())?;
        let rows: Vec<Vec<String>> = range.rows().map(|r| r.iter().map(cell_text).collect()).collect();
        sheets.push((name, rows));
    }
    Ok(Book { sheets })
}

/// Comma-, semicolon- or tab-separated text, with "quoted, cells".
pub fn parse_delimited(text: &str) -> Vec<Vec<String>> {
    let first = text.lines().next().unwrap_or("");
    let sep = [',', ';', '\t'].into_iter().max_by_key(|c| first.matches(*c).count()).unwrap_or(',');
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut row = Vec::new();
        let mut cur = String::new();
        let mut quoted = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    cur.push('"');
                    chars.next();
                }
                '"' => quoted = !quoted,
                c if c == sep && !quoted => row.push(std::mem::take(&mut cur)),
                c => cur.push(c),
            }
        }
        row.push(cur);
        rows.push(row);
    }
    rows
}

// ------------------------------------------------------------------ spreadsheet → model

fn yes(s: &str) -> bool {
    matches!(norm(s).as_str(), "yes" | "y" | "true" | "1" | "x")
}

fn num(s: &str) -> Option<f64> {
    s.trim().replace(',', "").parse::<f64>().ok()
}

/// "(0,1) (0.5,0.7) (1,0.2)" or "0,1; 0.5,0.7" → points.
pub fn parse_points(s: &str) -> Vec<[f64; 2]> {
    // "(0.5, 0.7)" → "(0.5,0.7)"
    let mut s = s.to_string();
    while s.contains(", ") || s.contains(" ,") {
        s = s.replace(", ", ",").replace(" ,", ",");
    }
    let cleaned: String = s.chars().map(|c| if c == '(' || c == ')' || c == ';' || c == '[' || c == ']' { ' ' } else { c }).collect();
    let mut pts = Vec::new();
    for tok in cleaned.split_whitespace() {
        let parts: Vec<&str> = tok.split(',').filter(|p| !p.is_empty()).collect();
        if parts.len() == 2 {
            if let (Ok(x), Ok(y)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                pts.push([x, y]);
            }
        }
    }
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    pts
}

pub fn format_points(t: &[[f64; 2]]) -> String {
    t.iter().map(|p| format!("({},{})", p[0], p[1])).collect::<Vec<_>>().join(" ")
}

/// Build a model from a workbook in the template. Returns the model and notes on anything
/// that was skipped or guessed.
pub fn model_from_book(book: &Book) -> Result<(Model, Vec<String>), String> {
    let mut m = Model { title: "Imported model".into(), ..Default::default() };
    let mut notes: Vec<String> = Vec::new();
    let known = ["Settings", "Parameters", "Stocks", "Flows", "Variables", "Lookups", "Links", "Scenarios", "TimeSeries", "Sets", "Values", "ItemSeries", "Modules", "Functions", "Checks"];
    if !known.iter().any(|k| book.sheet(k).is_some()) {
        return Err("No sheet matches the template (Parameters, Stocks, Flows, Variables, Lookups, Links…). See the wiki page “Spreadsheet template”.".into());
    }
    for (name, _) in &book.sheets {
        if !known.iter().any(|k| sheet_aliases(k).contains(&norm(name).as_str()) || norm(k) == norm(name)) && norm(name) != "readme" {
            notes.push(format!("Sheet “{name}” isn't part of the template, so it was left out."));
        }
    }

    // settings
    if let Some(t) = book.sheet("Settings") {
        for r in &t.rows {
            let (k, v) = (norm(t.get(r, "setting")), t.get(r, "value"));
            match k.as_str() {
                "title" | "name" | "model" => m.title = v.to_string(),
                "initialtime" | "starttime" | "start" => m.settings.initial_time = num(v).unwrap_or(0.0),
                "finaltime" | "endtime" | "end" | "stoptime" => m.settings.final_time = num(v).unwrap_or(100.0),
                "timestep" | "dt" | "step" => m.settings.time_step = num(v).unwrap_or(1.0),
                "timeunits" | "units" | "timeunit" => m.settings.time_units = v.to_string(),
                "method" | "integration" => {
                    let n = norm(v);
                    m.settings.method = if n.contains("rk2") || n.contains("runge2") || n.contains("heun") { Method::RungeKutta2 } else if n.contains("rk3") || n.contains("runge3") { Method::RungeKutta3 } else if n.contains("rk") || n.contains("runge") { Method::RungeKutta4 } else { Method::Euler };
                }
                "" => {}
                other => notes.push(format!("Unknown setting “{other}” was ignored.")),
            }
        }
    }

    // the person's own functions: Name, Parameters, Body, Description
    if let Some(t) = book.sheet("Functions") {
        for r in &t.rows {
            let (name, params, body) = (t.get(r, "name"), t.get(r, "params"), t.get(r, "body"));
            if name.is_empty() || body.is_empty() {
                continue;
            }
            m.functions.push(crate::userfn::UserFn {
                name: name.to_string(),
                params: params.split(|c| c == ',' || c == ';').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
                body: body.to_string(),
                description: t.get(r, "description").to_string(),
            });
        }
        crate::userfn::set(&m.functions);
    }
    // what the results must satisfy: Name, Variable, Min, Max, When (always / end), Kind (bounds / nonneg /
    // reference / balance), Tolerance (a number, or 5% for relative), Reference (t:y; t:y), Stocks, Inflows, Outflows
    if let Some(t) = book.sheet("Checks") {
        let list = |x: &str| -> Vec<String> { x.split(';').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect() };
        for r in &t.rows {
            use crate::checks::{Check, CheckKind, CheckWhen};
            let (var, label) = (t.get(r, "var"), t.get(r, "name"));
            let kind = match t.get(r, "checkkind").to_lowercase().as_str() {
                "" | "bounds" | "bound" | "limit" => CheckKind::Bounds,
                "nonneg" | "non-negative" | "nonnegative" => CheckKind::NonNeg,
                "reference" | "band" => CheckKind::Reference,
                "balance" | "conservation" => CheckKind::Balance,
                "material" | "conserve" => CheckKind::Conservation,
                other => {
                    notes.push(format!("The check “{label}” has an unknown kind “{other}”, so it was ignored."));
                    continue;
                }
            };
            let w = t.get(r, "when").to_lowercase();
            let when = if w.contains("end") || w.contains("final") || w.contains("last") { CheckWhen::End } else { CheckWhen::Always };
            let name = if label.eq_ignore_ascii_case(var) { String::new() } else { label.to_string() };
            let tol_text = t.get(r, "tolerance").replace(',', "");
            let (tolerance, relative) = match tol_text.strip_suffix('%') {
                Some(p) => (p.trim().parse::<f64>().ok().map(|x| x / 100.0), true),
                None => (tol_text.parse::<f64>().ok(), false),
            };
            let mut c = Check { name, var: var.to_string(), when, kind, tolerance, relative, ..Default::default() };
            match kind {
                CheckKind::Bounds => {
                    let (lo, hi) = (t.get(r, "min").replace(',', ""), t.get(r, "max").replace(',', ""));
                    let (min, max) = (lo.parse::<f64>().ok(), hi.parse::<f64>().ok());
                    if var.is_empty() {
                        continue;
                    }
                    if (!lo.is_empty() && min.is_none()) || (!hi.is_empty() && max.is_none()) {
                        notes.push(format!("The check on “{var}” has a minimum or maximum that is not a number, so it was ignored."));
                        continue;
                    }
                    if min.is_none() && max.is_none() {
                        notes.push(format!("The check on “{var}” has neither a minimum nor a maximum, so it was ignored."));
                        continue;
                    }
                    c.min = min;
                    c.max = max;
                }
                CheckKind::NonNeg => {
                    if var.is_empty() {
                        continue;
                    }
                }
                CheckKind::Reference => {
                    let pts: Vec<[f64; 2]> = list(t.get(r, "reference").replace(',', "").as_str())
                        .iter()
                        .filter_map(|p| match p.split_once(':') {
                            Some((a, b)) => Some([a.trim().parse().ok()?, b.trim().parse().ok()?]),
                            None => Some([0.0, p.parse().ok()?]),
                        })
                        .collect();
                    if var.is_empty() || pts.is_empty() {
                        notes.push(format!("The reference check “{label}” needs a variable and a Reference such as 0:100; 10:120 (or one number for a constant), so it was ignored."));
                        continue;
                    }
                    c.reference = pts;
                }
                CheckKind::Conservation => {
                    c.stocks = list(t.get(r, "stocks"));
                }
                CheckKind::Balance => {
                    c.stocks = list(t.get(r, "stocks"));
                    c.inflows = list(t.get(r, "inflows"));
                    c.outflows = list(t.get(r, "outflows"));
                    if c.stocks.is_empty() {
                        notes.push(format!("The balance check “{label}” lists no stocks, so it was ignored."));
                        continue;
                    }
                }
            }
            m.checks.push(c);
        }
    }
    // sets of similar things: Set, Item, Belongs to
    if let Some(t) = book.sheet("Sets") {
        let mut order: Vec<String> = Vec::new();
        let mut rows_of: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for r in &t.rows {
            let (set, item, up) = (t.get(r, "setname"), t.get(r, "item"), t.get(r, "belongs"));
            if set.is_empty() || item.is_empty() {
                continue;
            }
            if !order.contains(&set.to_string()) {
                order.push(set.to_string());
            }
            let v = rows_of.entry(set.to_string()).or_default();
            if v.iter().any(|(i, _)| expr::normalize(i) == expr::normalize(item)) {
                notes.push(format!("The set “{set}” lists “{item}” twice; the second was ignored."));
            } else {
                v.push((item.to_string(), up.to_string()));
            }
        }
        for name in &order {
            let id = m.new_id();
            let rows = &rows_of[name];
            m.sets.push(ItemSet { id, name: name.clone(), items: rows.iter().map(|r| r.0.clone()).collect(), parent: None, parent_of: rows.iter().map(|r| r.1.clone()).collect(), note: String::new() });
        }
        // which set does each set belong to? the one that holds the names in its "Belongs to" column
        let snapshot = m.sets.clone();
        for s in m.sets.iter_mut() {
            let Some(first) = s.parent_of.iter().find(|p| !p.is_empty()).cloned() else { continue };
            match snapshot.iter().find(|o| o.id != s.id && o.items.iter().any(|i| expr::normalize(i) == expr::normalize(&first))) {
                Some(p) => {
                    s.parent = Some(p.name.clone());
                    for x in s.parent_of.iter_mut() {
                        match p.items.iter().find(|i| expr::normalize(i) == expr::normalize(x)) {
                            Some(real) => *x = real.clone(),
                            None if !x.is_empty() => {
                                notes.push(format!("“{x}” (in {} → Belongs to) is not an item of {}.", s.name, p.name));
                                x.clear();
                            }
                            None => {}
                        }
                    }
                }
                None => notes.push(format!("“Belongs to” in {} names “{first}”, which isn't an item of any set.", s.name)),
            }
        }
    }
    let set_names: Vec<String> = m.sets.iter().map(|s| s.name.clone()).collect();
    // the "Per" and "Combine" columns of a variable sheet
    let per_of = |t: &Table, r: &[String], what: &str, notes: &mut Vec<String>| -> Option<String> {
        let p = t.get(r, "per");
        if p.is_empty() || matches!(norm(p).as_str(), "none" | "all" | "one" | "single" | "-") {
            return None;
        }
        match set_names.iter().find(|s| expr::normalize(s) == expr::normalize(p)) {
            Some(s) => Some(s.clone()),
            None => {
                notes.push(format!("“{what}” is per “{p}”, which isn't listed in the Sets sheet; it is treated as a single value."));
                None
            }
        }
    };
    let combine_of = |t: &Table, r: &[String]| -> Rollup {
        if matches!(norm(t.get(r, "combine")).as_str(), "average" | "avg" | "mean") { Rollup::Average } else { Rollup::Total }
    };

    let add = |m: &mut Model, name: &str, kind: Kind, eq: &str, units: &str, desc: &str, notes: &mut Vec<String>| -> Option<u64> {
        if name.is_empty() {
            return None;
        }
        if m.by_name(&expr::normalize(name)).is_some() {
            notes.push(format!("“{name}” appears twice; only the first was kept."));
            return None;
        }
        let id = m.new_id();
        let mut v = Var::new(id, name, kind, [0.0, 0.0]);
        v.equation = eq.to_string();
        v.units = units.to_string();
        v.comment = desc.to_string();
        m.vars.push(v);
        Some(id)
    };

    if let Some(t) = book.sheet("Parameters") {
        for r in &t.rows {
            let name = t.get(r, "name");
            let value = t.get(r, "value");
            if let Some(id) = add(&mut m, name, Kind::Constant, if value.is_empty() { "0" } else { value }, t.get(r, "units"), t.get(r, "description"), &mut notes) {
                let (per, roll) = (per_of(&t, r, name, &mut notes), combine_of(&t, r));
                let v = m.var_mut(id).unwrap();
                v.set = per;
                v.rollup = roll;
                v.lever = yes(t.get(r, "lever"));
                let x = num(&v.equation);
                let lo = num(t.get(r, "min")).or(x.map(|x| x.min(0.0)));
                let hi = num(t.get(r, "max")).or(x.map(|x| if x == 0.0 { 1.0 } else { x.abs() * 2.0 }));
                if let (Some(a), Some(b)) = (lo, hi) {
                    v.slider = Some([a.min(b), a.max(b)]);
                }
                if value.is_empty() {
                    notes.push(format!("Parameter “{name}” had no value; it was set to 0."));
                }
            }
        }
    }
    if let Some(t) = book.sheet("Stocks") {
        for r in &t.rows {
            let init = t.get(r, "initial");
            if let Some(id) = add(&mut m, t.get(r, "name"), Kind::Stock, if init.is_empty() { "0" } else { init }, t.get(r, "units"), t.get(r, "description"), &mut notes) {
                let (per, roll) = (per_of(&t, r, t.get(r, "name"), &mut notes), combine_of(&t, r));
                let v = m.var_mut(id).unwrap();
                v.non_negative = yes(t.get(r, "nonneg"));
                v.set = per;
                v.rollup = roll;
            }
        }
    }
    let mut flow_ends: Vec<(u64, String, String)> = Vec::new();
    if let Some(t) = book.sheet("Flows") {
        for r in &t.rows {
            if let Some(id) = add(&mut m, t.get(r, "name"), Kind::Flow, t.get(r, "equation"), t.get(r, "units"), t.get(r, "description"), &mut notes) {
                flow_ends.push((id, t.get(r, "from").to_string(), t.get(r, "to").to_string()));
                let (per, roll) = (per_of(&t, r, t.get(r, "name"), &mut notes), combine_of(&t, r));
                let v = m.var_mut(id).unwrap();
                v.set = per;
                v.rollup = roll;
            }
        }
    }
    if let Some(t) = book.sheet("Variables") {
        for r in &t.rows {
            if let Some(id) = add(&mut m, t.get(r, "name"), Kind::Aux, t.get(r, "equation"), t.get(r, "units"), t.get(r, "description"), &mut notes) {
                let (per, roll) = (per_of(&t, r, t.get(r, "name"), &mut notes), combine_of(&t, r));
                let v = m.var_mut(id).unwrap();
                v.set = per;
                v.rollup = roll;
            }
        }
    }
    if let Some(t) = book.sheet("Lookups") {
        for r in &t.rows {
            let pts = parse_points(t.get(r, "points"));
            let name = t.get(r, "name").to_string();
            if let Some(id) = add(&mut m, &name, Kind::Lookup, t.get(r, "input"), t.get(r, "units"), t.get(r, "description"), &mut notes) {
                let per = per_of(&t, r, &name, &mut notes);
                // a lookup per item gets its points from the ItemSeries sheet
                if pts.len() < 2 && per.is_none() {
                    notes.push(format!("Lookup “{name}” needs at least two points like (0,1) (1,2)."));
                }
                let v = m.var_mut(id).unwrap();
                v.table = pts;
                v.set = per;
            }
        }
    }
    if let Some(t) = book.sheet("TimeSeries") {
        // first column is time, each other column becomes a lookup over TIME
        let ncols = t.header.len();
        let raw_header: Vec<String> = book.sheets.iter().find(|(n, _)| sheet_aliases("TimeSeries").contains(&norm(n).as_str())).and_then(|(_, rows)| rows.iter().find(|r| r.iter().any(|c| !c.trim().is_empty())).cloned()).unwrap_or_default();
        for c in 1..ncols {
            let name = raw_header.get(c).cloned().unwrap_or_default();
            let pts: Vec<[f64; 2]> = t.rows.iter().filter_map(|r| Some([num(r.first()?)?, num(r.get(c)?)?])).collect();
            if pts.len() >= 2 {
                if let Some(id) = add(&mut m, name.trim(), Kind::Lookup, "Time", "", "Data series (from the TimeSeries sheet)", &mut notes) {
                    m.var_mut(id).unwrap().table = pts;
                }
            }
        }
    }

    // values that differ by item
    if let Some(t) = book.sheet("Values") {
        for r in &t.rows {
            let (name, item, value) = (t.get(r, "name"), t.get(r, "item"), t.get(r, "value"));
            if name.is_empty() || item.is_empty() || value.is_empty() {
                continue;
            }
            let Some(v) = m.vars.iter_mut().find(|v| expr::normalize(&v.name) == expr::normalize(name)) else {
                notes.push(format!("Values: no variable called “{name}”."));
                continue;
            };
            let Some(set) = v.set.clone().and_then(|s| m.sets.iter().find(|x| x.name == s)) else {
                notes.push(format!("Values: “{name}” isn't per a set (add its set in the Per column), so “{item}” was skipped."));
                continue;
            };
            let Some(real) = set.items.iter().find(|i| expr::normalize(i) == expr::normalize(item)).cloned() else {
                notes.push(format!("Values: “{item}” isn't an item of {} (for “{name}”).", set.name));
                continue;
            };
            match v.by_item.iter_mut().find(|o| o.item == real) {
                Some(o) => o.equation = value.to_string(),
                None => v.by_item.push(ItemValue { item: real, equation: value.to_string(), table: Vec::new() }),
            }
        }
    }
    if let Some(t) = book.sheet("ItemSeries") {
        let mut pts: HashMap<(String, String), Vec<[f64; 2]>> = HashMap::new();
        let mut order: Vec<(String, String)> = Vec::new();
        for r in &t.rows {
            let (name, item) = (t.get(r, "name"), t.get(r, "item"));
            let (Some(x), Some(y)) = (num(t.get(r, "time")), num(t.get(r, "value"))) else { continue };
            let key = (name.to_string(), item.to_string());
            if !pts.contains_key(&key) {
                order.push(key.clone());
            }
            pts.entry(key).or_default().push([x, y]);
        }
        let mut bad: Vec<String> = Vec::new();
        for key in order {
            let mut p = pts.remove(&key).unwrap_or_default();
            p.sort_by(|a, b| a[0].total_cmp(&b[0]));
            let (name, item) = &key;
            let Some(v) = m.vars.iter_mut().find(|v| expr::normalize(&v.name) == expr::normalize(name)) else {
                if !bad.contains(name) {
                    bad.push(name.clone());
                    notes.push(format!("ItemSeries: no variable called “{name}”."));
                }
                continue;
            };
            let real = v.set.clone().and_then(|s| m.sets.iter().find(|x| x.name == s)).and_then(|set| set.items.iter().find(|i| expr::normalize(i) == expr::normalize(item)).cloned());
            let Some(real) = real else {
                if !bad.contains(name) {
                    bad.push(name.clone());
                    notes.push(format!("ItemSeries: “{name}” must be per a set, and “{item}” an item of it (first problem shown only)."));
                }
                continue;
            };
            match v.by_item.iter_mut().find(|o| o.item == real) {
                Some(o) => o.table = p,
                None => v.by_item.push(ItemValue { item: real, equation: String::new(), table: p }),
            }
        }
    }

    // flows: tie them to their stocks
    for (id, from, to) in flow_ends {
        let find = |m: &Model, n: &str, notes: &mut Vec<String>, fname: &str| -> Option<u64> {
            if n.is_empty() || matches!(norm(n).as_str(), "none" | "outside" | "cloud" | "source" | "sink") {
                return None;
            }
            match m.by_name(&expr::normalize(n)) {
                Some(v) if v.kind == Kind::Stock => Some(v.id),
                _ => {
                    notes.push(format!("Flow “{fname}” refers to stock “{n}”, which isn't in the Stocks sheet; that end was left open."));
                    None
                }
            }
        };
        let fname = m.var(id).map(|v| v.name.clone()).unwrap_or_default();
        let a = find(&m, &from, &mut notes, &fname);
        let b = find(&m, &to, &mut notes, &fname);
        let v = m.var_mut(id).unwrap();
        v.from = a;
        v.to = b;
    }

    // the optional "Module" column says which part of the system a row belongs to
    let mut module_labels: Vec<(String, Vec<u64>)> = Vec::new();
    for sheet in ["Parameters", "Stocks", "Flows", "Variables", "Lookups"] {
        let Some(t) = book.sheet(sheet) else { continue };
        for r in &t.rows {
            let (name, label) = (t.get(r, "name"), t.get(r, "module"));
            if label.is_empty() {
                continue;
            }
            let Some(id) = m.by_name(&expr::normalize(name)).map(|v| v.id) else { continue };
            match module_labels.iter_mut().find(|(l, _)| l == label) {
                Some((_, ids)) => ids.push(id),
                None => module_labels.push((label.to_string(), vec![id])),
            }
        }
    }
    let block_groups = layout(&mut m, false);

    // links: arrows on the diagram, and anything about concepts outside the model goes to the CLD
    let mut cld_extra: Vec<(String, String, String, bool)> = Vec::new();
    if let Some(t) = book.sheet("Links") {
        for r in &t.rows {
            let (c, e) = (t.get(r, "cause"), t.get(r, "effect"));
            if c.is_empty() || e.is_empty() {
                continue;
            }
            let pol = match t.get(r, "polarity").trim() {
                "+" | "positive" | "pos" | "same" | "s" => "+",
                "-" | "−" | "negative" | "neg" | "opposite" | "o" => "-",
                _ => "",
            };
            let delay = yes(t.get(r, "delay"));
            match (m.by_name(&expr::normalize(c)).map(|v| v.id), m.by_name(&expr::normalize(e)).map(|v| (v.id, v.kind))) {
                (Some(a), Some((b, k))) if k != Kind::Stock => {
                    if m.add_link(a, b) {
                        let l = m.links.last_mut().unwrap();
                        l.polarity = pol.into();
                        l.delay_mark = delay;
                        l.bend = 30.0;
                    } else if let Some(l) = m.links.iter_mut().find(|l| l.from == a && l.to == b) {
                        l.polarity = pol.into();
                        l.delay_mark = delay;
                    }
                }
                _ => cld_extra.push((c.to_string(), e.to_string(), pol.to_string(), delay)),
            }
        }
    }
    // arrows for every equation input that has none yet
    let added = crate::canvas::sync_links(&mut m);
    if added > 0 {
        notes.push(format!("Added {added} arrow(s) for equation inputs that weren't in the Links sheet."));
    }

    // scenarios
    if let Some(t) = book.sheet("Scenarios") {
        for r in &t.rows {
            let (s, var, x) = (t.get(r, "scenario"), t.get(r, "var").to_string(), t.get(r, "value"));
            if s.is_empty() || var.is_empty() {
                continue;
            }
            let base = var.split('[').next().unwrap_or(&var).trim().to_string();
            if m.by_name(&expr::normalize(&base)).is_none() {
                notes.push(format!("Scenario “{s}” changes “{var}”, which isn't in the model."));
            }
            let idx = match m.scenarios.iter().position(|x| x.name == s) {
                Some(i) => i,
                None => {
                    let id = m.new_id();
                    m.scenarios.push(Scenario { id, name: s.to_string(), color: None, enabled: true, changes: Vec::new(), notes: String::new() });
                    m.scenarios.len() - 1
                }
            };
            m.scenarios[idx].changes.push(Change { var: var.trim().to_string(), value: x.to_string() });
        }
    }

    // sheets: the diagram, its equations, results, the causal loop picture and scenarios
    let d = m.new_id();
    m.sheets.push(Sheet::new(d, "Model", SheetKind::Diagram));
    for v in m.vars.iter_mut() {
        v.view = d;
    }
    // with modules, the blocks are the modules: no frames needed
    let block_groups = if module_labels.is_empty() { block_groups } else { Vec::new() };
    for (name, members) in block_groups {
        if members.len() > 1 {
            let id = m.new_id();
            m.groups.push(crate::model::Group { id, name, members, view: d, color: None });
        }
    }
    let t = m.new_id();
    m.sheets.push(Sheet::new(t, "Equations", SheetKind::Table));
    let r = m.new_id();
    let mut rs = Sheet::new(r, "Results", SheetKind::Table);
    rs.table_mode = TableMode::Results;
    rs.columns = m.vars.iter().filter(|v| v.kind == Kind::Stock).map(|v| v.name.clone()).collect();
    m.sheets.push(rs);
    let c = m.new_id();
    let mut cs = Sheet::new(c, "Causal loops", SheetKind::CausalLoop);
    cs.cld = crate::cld::from_model(&mut m);
    // relationships between concepts that aren't model variables
    let mut extra_n = 0;
    for (a, b, pol, delay) in cld_extra {
        let node = |m: &mut Model, cs: &mut Sheet, n: &str| -> u64 {
            if let Some(x) = cs.cld.nodes.iter().find(|x| expr::normalize(&x.text) == expr::normalize(n)) {
                return x.id;
            }
            let id = m.new_id();
            let k = cs.cld.nodes.len() as f32;
            cs.cld.nodes.push(crate::model::CldNode { id, text: n.to_string(), pos: [120.0 + (k % 5.0) * 230.0, 820.0 + (k / 5.0).floor() * 120.0] });
            id
        };
        let (x, y) = (node(&mut m, &mut cs, &a), node(&mut m, &mut cs, &b));
        let id = m.new_id();
        cs.cld.links.push(crate::model::CldLink { id, from: x, to: y, bend: 25.0, polarity: pol, delay });
        extra_n += 1;
    }
    if extra_n > 0 {
        notes.push(format!("{extra_n} relationship(s) between concepts outside the stock-and-flow model went to the causal loop diagram only."));
    }
    m.sheets.push(cs);
    if !module_labels.is_empty() {
        modularize(&mut m, d, &module_labels);
        // modules show their inputs and outputs (View → Modules → Compact hides them)
        m.module_style = crate::model::ModuleStyle::Detailed;
        // what each module is for, from the optional Modules sheet
        if let Some(t) = book.sheet("Modules") {
            for r in &t.rows {
                let (n, desc) = (t.get(r, "module"), t.get(r, "description"));
                let key = |p: &str| p.split('/').map(|x| expr::normalize(x)).collect::<Vec<_>>().join("/");
                let ids: Vec<u64> = m.modules.iter().map(|x| x.id).collect();
                for id in ids {
                    if key(&module_path(&m, id)) == key(n) {
                        if let Some(x) = m.modules.iter_mut().find(|x| x.id == id) {
                            x.description = desc.to_string();
                        }
                    }
                }
            }
        }
        notes.push(format!("{} module(s) built from the Module column (a path like “Plans / Optimal plan” nests them): the Model sheet is the overview, each module opens on its own sheet.", m.modules.len()));
    }
    if !m.scenarios.is_empty() {
        let s = m.new_id();
        let mut ss = Sheet::new(s, "Scenarios", SheetKind::Scenarios);
        let mut levers: Vec<String> = Vec::new();
        for sc in &m.scenarios {
            for ch in &sc.changes {
                if !levers.contains(&ch.var) {
                    levers.push(ch.var.clone());
                }
            }
        }
        ss.columns = levers;
        ss.compare_vars = m.vars.iter().filter(|v| v.kind == Kind::Stock).take(2).map(|v| v.name.clone()).collect();
        m.sheets.push(ss);
    }
    m.ensure_sheets();
    if m.links.len() > 25 {
        // the textbook look: plain labels and thin dark curved arrows
        m.diagram_style = crate::model::DiagramStyle::Classic;
        m.arrow_shape = crate::model::ArrowShape::Curved;
    }
    if m.vars.is_empty() {
        notes.push("No variables were found — check the sheet and column names against the template.".into());
    }
    Ok((m, notes))
}

/// Place things in readable rows: parameters on top, stocks and flows in the middle,
/// variables and lookups below.
fn layout(m: &mut Model, layered: bool) -> Vec<(String, Vec<u64>)> {
    let ids_of = |m: &Model, k: Kind| m.vars.iter().filter(|v| v.kind == k).map(|v| v.id).collect::<Vec<_>>();
    let stocks = ids_of(m, Kind::Stock);
    for (i, id) in stocks.iter().enumerate() {
        m.var_mut(*id).unwrap().pos = [160.0 + i as f32 * 380.0, 320.0];
    }
    let pos_of = |m: &Model, id: Option<u64>| id.and_then(|i| m.var(i)).map(|v| v.pos);
    let flows = ids_of(m, Kind::Flow);
    let mut loose = 0.0;
    for id in flows {
        let (a, b) = {
            let v = m.var(id).unwrap();
            (pos_of(m, v.from), pos_of(m, v.to))
        };
        let v = m.var_mut(id).unwrap();
        match (a, b) {
            (Some(a), Some(b)) => {
                v.pos = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
                // unused cloud spots stay near the flow so zoom-to-fit isn't thrown off
                v.from_pt = a;
                v.to_pt = b;
            }
            (Some(a), None) => {
                v.to_pt = [a[0] + 260.0, a[1] + 130.0];
                v.pos = [a[0] + 130.0, a[1] + 65.0];
            }
            (None, Some(b)) => {
                v.from_pt = [b[0] - 260.0, b[1] - 130.0];
                v.pos = [b[0] - 130.0, b[1] - 65.0];
            }
            (None, None) => {
                v.from_pt = [160.0 + loose, 470.0];
                v.to_pt = [360.0 + loose, 470.0];
                v.pos = [260.0 + loose, 470.0];
                loose += 320.0;
            }
        }
    }
    if layered || crate::sets::uses_sets(m) {
        return layout_blocks(m);
    }
    for (row, kind, y) in [(0, Kind::Constant, 110.0), (1, Kind::Aux, 560.0), (2, Kind::Lookup, 700.0)] {
        let _ = row;
        for (i, id) in ids_of(m, kind).iter().enumerate() {
            let per = 6;
            m.var_mut(*id).unwrap().pos = [140.0 + (i % per) as f32 * 230.0, y + (i / per) as f32 * 70.0];
        }
    }
    Vec::new()
}

/// Models with sets: inputs on top, stocks and flows in the middle, and the calculations below in
/// columns by how far they are from the inputs, in blocks: the main chain first, then one block
/// per smaller set (for example "Per Region") and a block of network-wide totals. The blocks are
/// returned as groups so the diagram shows each as one labelled frame.
fn layout_blocks(m: &mut Model) -> Vec<(String, Vec<u64>)> {
    let ids_of = |m: &Model, k: Kind| m.vars.iter().filter(|v| v.kind == k).map(|v| v.id).collect::<Vec<_>>();
    for (i, id) in ids_of(m, Kind::Constant).iter().enumerate() {
        m.var_mut(*id).unwrap().pos = [140.0 + (i % 9) as f32 * 230.0, 110.0 + (i / 9) as f32 * 70.0];
    }
    // distance from the inputs: stocks, constants, lookups and flows count as 0
    let n = m.vars.len();
    let refs: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let mut r = Vec::new();
            if let Ok(e) = expr::parse(&m.vars[i].equation) {
                expr::references(&e, &mut r);
            }
            r.iter().filter_map(|x| m.vars.iter().position(|v| expr::normalize(&v.name) == *x)).collect()
        })
        .collect();
    let mut depth = vec![0usize; n];
    for _ in 0..30 {
        for i in 0..n {
            if matches!(m.vars[i].kind, Kind::Aux) {
                let d = refs[i].iter().filter(|&&j| m.vars[j].kind == Kind::Aux).map(|&j| depth[j] + 1).max().unwrap_or(0);
                depth[i] = d.min(12);
            }
        }
    }
    let main_set = {
        let mut c: HashMap<String, usize> = HashMap::new();
        for v in &m.vars {
            if let Some(s) = &v.set {
                *c.entry(s.clone()).or_default() += 1;
            }
        }
        c.into_iter().max_by_key(|(_, k)| *k).map(|(s, _)| s)
    };
    // block of each calculated variable / lookup
    let block_of = |v: &Var| -> String {
        if v.kind == Kind::Lookup {
            return "main".into();
        }
        match &v.set {
            Some(s) if Some(s) != main_set.as_ref() => format!("Per {s}"),
            Some(_) => "main".into(),
            None if v.equation.to_uppercase().contains("TOTAL(") || v.equation.to_uppercase().contains("AVERAGE(") => "Totals".into(),
            None => "main".into(),
        }
    };
    let mut blocks: Vec<String> = vec!["main".into()];
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Lookup)) {
        let b = block_of(v);
        if !blocks.contains(&b) {
            blocks.push(b);
        }
    }
    let x0 = 140.0f32;
    let mut y0 = 640.0f32;
    let mut groups = Vec::new();
    for b in &blocks {
        let members: Vec<usize> = (0..n).filter(|&i| matches!(m.vars[i].kind, Kind::Aux | Kind::Lookup) && block_of(&m.vars[i]) == *b).collect();
        let max_d = members.iter().map(|&i| depth[i] + if m.vars[i].kind == Kind::Lookup { 0 } else { 1 }).max().unwrap_or(0);
        let mut next_row: HashMap<usize, usize> = HashMap::new();
        for &i in &members {
            let col = if m.vars[i].kind == Kind::Lookup { 0 } else { depth[i] + 1 };
            let row = next_row.entry(col).or_default();
            m.vars[i].pos = [x0 + col as f32 * 205.0, y0 + *row as f32 * 62.0];
            *row += 1;
        }
        let rows = next_row.values().copied().max().unwrap_or(0);
        if *b != "main" {
            groups.push((if b == "Totals" { "Network totals".to_string() } else { format!("{b} totals") }, members.iter().map(|&i| m.vars[i].id).collect::<Vec<_>>()));
        }
        let _ = max_d;
        // blocks stack downwards so the diagram stays about as wide as it is tall
        y0 += rows as f32 * 62.0 + 90.0;
    }
    groups
}

/// Wrap labelled parts of the model into modules, to any depth. A label is a path —
/// `Plans / Optimal plan / Trucks` — so a module can hold modules that hold modules. Each module
/// gets its own diagram sheet (laid out tidily) and shows on the sheet above it as one box;
/// boxes are placed left to right in the order information flows between them. Variables
/// without a label stay on `parent`.
pub fn modularize(m: &mut Model, parent: u64, labels: &[(String, Vec<u64>)]) {
    struct Node {
        name: String,
        vars: Vec<u64>,
        kids: Vec<Node>,
    }
    fn insert(nodes: &mut Vec<Node>, path: &[&str], ids: &[u64]) {
        let Some((first, rest)) = path.split_first() else { return };
        let i = match nodes.iter().position(|n| n.name == *first) {
            Some(i) => i,
            None => {
                nodes.push(Node { name: first.to_string(), vars: Vec::new(), kids: Vec::new() });
                nodes.len() - 1
            }
        };
        if rest.is_empty() {
            nodes[i].vars.extend_from_slice(ids);
        } else {
            insert(&mut nodes[i].kids, rest, ids);
        }
    }
    fn build(m: &mut Model, parent: u64, nodes: &[Node], at: &mut usize) {
        for n in nodes {
            let sid = m.new_id();
            let mid = m.new_id();
            let mut sheet = Sheet::new(sid, &n.name, SheetKind::Diagram);
            sheet.module = Some(mid);
            m.sheets.insert((*at).min(m.sheets.len()), sheet);
            *at += 1;
            for id in &n.vars {
                if let Some(v) = m.var_mut(*id) {
                    v.view = sid;
                }
            }
            m.modules.push(crate::model::Module { id: mid, name: n.name.clone(), view: parent, inner: sid, pos: [0.0, 0.0], description: String::new(), collapsed: false, layer: None });
            build(m, sid, &n.kids, at);
        }
    }
    let mut roots: Vec<Node> = Vec::new();
    for (label, ids) in labels {
        let path: Vec<&str> = label.split('/').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
        insert(&mut roots, &path, ids);
    }
    let mut at = m.sheets.iter().position(|s| s.id == parent).map(|i| i + 1).unwrap_or(m.sheets.len());
    build(m, parent, &roots, &mut at);
    layout_level(m, parent);
}

/// Lay out a sheet and, first, every module sheet below it.
fn layout_level(m: &mut Model, sheet: u64) {
    let kids: Vec<u64> = m.modules.iter().filter(|x| x.view == sheet).map(|x| x.inner).collect();
    for k in kids {
        layout_level(m, k);
    }
    if m.vars.iter().any(|v| v.view == sheet) {
        tidy(m, sheet);
    }
    place_modules(m, sheet);
}

/// Put the module boxes of `sheet` left to right in the order information flows between them.
fn place_modules(m: &mut Model, sheet: u64) {
    let mods: Vec<u64> = m.modules.iter().filter(|x| x.view == sheet).map(|x| x.id).collect();
    if mods.is_empty() {
        return;
    }
    // which box on this sheet holds a variable, however many levels down it sits
    let box_of = |m: &Model, var: u64| -> Option<usize> {
        let mut s = m.var(var)?.view;
        for _ in 0..32 {
            if s == sheet {
                return None;
            }
            let md = m.modules.iter().find(|x| x.inner == s)?;
            if md.view == sheet {
                return mods.iter().position(|x| *x == md.id);
            }
            s = md.view;
        }
        None
    };
    let mut feeds: Vec<(usize, usize)> = Vec::new();
    for l in &m.links {
        if let (Some(a), Some(b)) = (box_of(m, l.from), box_of(m, l.to)) {
            if a != b && !feeds.contains(&(a, b)) {
                feeds.push((a, b));
            }
        }
    }
    let n = mods.len();
    let mut rank = vec![0usize; n];
    for _ in 0..n {
        for &(a, b) in &feeds {
            // a cycle between modules (feedback) must not run the ranks away
            if rank[b] <= rank[a] && rank[a] + 1 < n {
                rank[b] = rank[a] + 1;
            }
        }
    }
    // modules go below any variables that share the sheet
    let base = m.vars.iter().filter(|v| v.view == sheet).map(|v| v.pos[1]).fold(f32::NEG_INFINITY, f32::max);
    let y0 = if base.is_finite() { base + 280.0 } else { 220.0 };
    let mut row_in_rank: HashMap<usize, usize> = HashMap::new();
    for (i, mid) in mods.iter().enumerate() {
        let r = row_in_rank.entry(rank[i]).or_default();
        if let Some(x) = m.modules.iter_mut().find(|x| x.id == *mid) {
            x.pos = [320.0 + rank[i] as f32 * 520.0, y0 + *r as f32 * 240.0];
        }
        *r += 1;
    }
}

/// `Plans / Optimal plan / Trucks`: a module's name with the modules it sits inside.
pub fn module_path(m: &Model, mid: u64) -> String {
    let mut parts = Vec::new();
    let mut cur = m.module(mid);
    for _ in 0..32 {
        let Some(x) = cur else { break };
        parts.push(x.name.clone());
        cur = m.modules.iter().find(|p| p.inner == x.view);
    }
    parts.reverse();
    parts.join(" / ")
}

/// Lay out the diagram sheet `view` tidily (inputs on top, stocks and flows in the middle,
/// calculations below in columns by distance from the inputs).
pub fn tidy(m: &mut Model, view: u64) {
    let mut sub = m.clone();
    sub.vars.retain(|v| v.view == view);
    layout(&mut sub, true);
    for v in sub.vars {
        if let Some(o) = m.var_mut(v.id) {
            o.pos = v.pos;
            o.from_pt = v.from_pt;
            o.to_pt = v.to_pt;
        }
    }
}

// ------------------------------------------------------------------ model → spreadsheet

/// The model in the template, ready to write out (and to read back in).
pub fn book_from_model(m: &Model) -> Book {
    let name = |id: Option<u64>| id.and_then(|i| m.var(i)).map(|v| v.name.clone()).unwrap_or_default();
    let head = |s: &str| TEMPLATE.iter().find(|t| t.0 == s).map(|t| t.1.iter().map(|x| x.to_string()).collect::<Vec<_>>()).unwrap_or_default();
    let fmtn = |x: f64| if x.fract() == 0.0 && x.abs() < 1e15 { format!("{}", x as i64) } else { format!("{x}") };
    let mut sheets = Vec::new();
    sheets.push((
        "Settings".to_string(),
        vec![
            head("Settings"),
            vec!["Title".into(), m.title.clone()],
            vec!["Initial time".into(), fmtn(m.settings.initial_time)],
            vec!["Final time".into(), fmtn(m.settings.final_time)],
            vec!["Time step".into(), fmtn(m.settings.time_step)],
            vec!["Time units".into(), m.settings.time_units.clone()],
            vec!["Method".into(), match m.settings.method { Method::RungeKutta2 => "RK2", Method::RungeKutta3 => "RK3", Method::RungeKutta4 => "RK4", Method::Euler => "Euler" }.into()],
        ],
    ));
    let rows = |k: Kind, f: &dyn Fn(&Var) -> Vec<String>| m.vars.iter().filter(|v| v.kind == k).map(f).collect::<Vec<_>>();
    let mut p = vec![head("Parameters")];
    p.extend(rows(Kind::Constant, &|v| vec![v.name.clone(), v.equation.clone(), v.units.clone(), v.slider.map(|s| fmtn(s[0])).unwrap_or_default(), v.slider.map(|s| fmtn(s[1])).unwrap_or_default(), v.comment.clone()]));
    sheets.push(("Parameters".into(), p));
    let mut s = vec![head("Stocks")];
    s.extend(rows(Kind::Stock, &|v| vec![v.name.clone(), v.equation.clone(), v.units.clone(), if v.non_negative { "yes".into() } else { String::new() }, v.comment.clone()]));
    sheets.push(("Stocks".into(), s));
    let mut f = vec![head("Flows")];
    f.extend(rows(Kind::Flow, &|v| vec![v.name.clone(), v.equation.clone(), name(v.from), name(v.to), v.units.clone(), v.comment.clone()]));
    sheets.push(("Flows".into(), f));
    let mut a = vec![head("Variables")];
    a.extend(rows(Kind::Aux, &|v| vec![v.name.clone(), v.equation.clone(), v.units.clone(), v.comment.clone()]));
    sheets.push(("Variables".into(), a));
    let mut l = vec![head("Lookups")];
    l.extend(rows(Kind::Lookup, &|v| vec![v.name.clone(), v.equation.clone(), format_points(&v.table), v.units.clone(), v.comment.clone()]));
    sheets.push(("Lookups".into(), l));
    let mut k = vec![head("Links")];
    for x in &m.links {
        k.push(vec![name(Some(x.from)), name(Some(x.to)), x.polarity.clone(), if x.delay_mark { "yes".into() } else { String::new() }, String::new()]);
    }
    sheets.push(("Links".into(), k));
    let mut sc = vec![head("Scenarios")];
    for x in &m.scenarios {
        for c in &x.changes {
            sc.push(vec![x.name.clone(), c.var.clone(), c.value.clone()]);
        }
    }
    sheets.push(("Scenarios".into(), sc));
    if !m.checks.is_empty() {
        let mut cs = vec![["Name", "Variable", "Min", "Max", "When", "Kind", "Tolerance", "Reference", "Stocks", "Inflows", "Outflows"].iter().map(|s| s.to_string()).collect::<Vec<_>>()];
        for c in &m.checks {
            use crate::checks::CheckKind;
            let f = |x: Option<f64>| x.map(|v| format!("{v}")).unwrap_or_default();
            let kind = match c.kind {
                CheckKind::Bounds => "",
                CheckKind::NonNeg => "nonneg",
                CheckKind::Reference => "reference",
                CheckKind::Balance => "balance",
                CheckKind::Conservation => "material",
            };
            let tol = match (c.tolerance, c.relative) {
                (Some(t), true) => format!("{}%", t * 100.0),
                (Some(t), false) => format!("{t}"),
                (None, _) => String::new(),
            };
            let reference = c.reference.iter().map(|p| format!("{}:{}", p[0], p[1])).collect::<Vec<_>>().join("; ");
            cs.push(vec![
                c.name.clone(), c.var.clone(), f(c.min), f(c.max),
                if c.when == crate::checks::CheckWhen::End { "end".into() } else { "always".into() },
                kind.into(), tol, reference, c.stocks.join("; "), c.inflows.join("; "), c.outflows.join("; "),
            ]);
        }
        sheets.push(("Checks".into(), cs));
    }
    if !m.functions.is_empty() {
        let mut fs = vec![vec!["Name".to_string(), "Parameters".into(), "Body".into(), "Description".into()]];
        for f in &m.functions {
            fs.push(vec![f.name.clone(), f.params.join(", "), f.body.clone(), f.description.clone()]);
        }
        sheets.push(("Functions".into(), fs));
    }
    if m.vars.iter().any(|v| v.lever) {
        for (sn, rows) in sheets.iter_mut() {
            if sn != "Parameters" {
                continue;
            }
            for (i, r) in rows.iter_mut().enumerate() {
                if i == 0 {
                    r.push("Lever".into());
                } else {
                    let on = m.by_name(&expr::normalize(&r[0])).is_some_and(|v| v.lever);
                    r.push(if on { "yes".into() } else { String::new() });
                }
            }
        }
    }
    if !m.modules.is_empty() {
        let mut ms = vec![vec!["Module".to_string(), "Description".into()]];
        for x in &m.modules {
            ms.push(vec![module_path(m, x.id), x.description.clone()]);
        }
        sheets.push(("Modules".into(), ms));
        // the part of the system each variable belongs to
        for (sn, rows) in sheets.iter_mut() {
            if !matches!(sn.as_str(), "Parameters" | "Stocks" | "Flows" | "Variables" | "Lookups") {
                continue;
            }
            for (i, r) in rows.iter_mut().enumerate() {
                if i == 0 {
                    r.push("Module".into());
                    continue;
                }
                let label = m.by_name(&expr::normalize(&r[0])).and_then(|v| m.modules.iter().find(|x| x.inner == v.view)).map(|x| module_path(m, x.id)).unwrap_or_default();
                r.push(label);
            }
        }
    }
    if crate::sets::uses_sets(m) {
        // which set each variable is per, and how its items are combined
        for (sn, rows) in sheets.iter_mut() {
            if !matches!(sn.as_str(), "Parameters" | "Stocks" | "Flows" | "Variables" | "Lookups") {
                continue;
            }
            for (i, r) in rows.iter_mut().enumerate() {
                if i == 0 {
                    r.push("Per".into());
                    r.push("Combine".into());
                    continue;
                }
                let v = m.by_name(&expr::normalize(&r[0]));
                r.push(v.and_then(|v| v.set.clone()).unwrap_or_default());
                r.push(if v.is_some_and(|v| v.rollup == Rollup::Average) { "Average".into() } else { String::new() });
            }
        }
        let _ = 0;
        let mut st = vec![vec!["Set".to_string(), "Item".into(), "Belongs to".into()]];
        for set in &m.sets {
            for (i, it) in set.items.iter().enumerate() {
                st.push(vec![set.name.clone(), it.clone(), set.parent_item(i).unwrap_or("").to_string()]);
            }
        }
        sheets.push(("Sets".into(), st));
        let mut vals = vec![vec!["Name".to_string(), "Item".into(), "Value".into()]];
        let mut ser = vec![vec!["Name".to_string(), "Item".into(), "Time".into(), "Value".into()]];
        for v in &m.vars {
            for o in &v.by_item {
                if !o.equation.trim().is_empty() {
                    vals.push(vec![v.name.clone(), o.item.clone(), o.equation.clone()]);
                }
                for p in &o.table {
                    ser.push(vec![v.name.clone(), o.item.clone(), fmtn(p[0]), fmtn(p[1])]);
                }
            }
        }
        sheets.push(("Values".into(), vals));
        if ser.len() > 1 {
            sheets.push(("ItemSeries".into(), ser));
        }
    }
    Book { sheets }
}

/// The blank template with an explanation sheet, filled with the example model so every
/// column shows what it expects.
pub fn template_book() -> Book {
    let mut b = book_from_model(&Model::example());
    let readme: Vec<Vec<String>> = [
        "DYNAMIST MODEL TEMPLATE",
        "Fill in the sheets below and use File → Import model from spreadsheet… in Dynamist.",
        "Every sheet is optional; leave out what you don't need. Column order doesn't matter; names are matched loosely.",
        "",
        "Settings    — Title, Initial time, Final time, Time step, Time units, Method (Euler or RK4)",
        "Parameters  — constants: Name, Value, Units, Min, Max (slider range), Description",
        "Stocks      — accumulations: Name, Initial value (number or formula), Units, Never negative (yes/no), Description",
        "Flows       — rates: Name, Equation, From stock (drains), To stock (fills); leave a stock empty for outside the model",
        "Variables   — calculated values: Name, Equation, Units, Description",
        "Lookups     — curves: Name, Input (empty = use as a function: Name(x)), Points like (0,1) (0.5,0.7) (1,0.2)",
        "Links       — cause → effect arrows with Polarity (+ or -) and Delay (yes/no); names not in the model build the causal loop diagram",
        "Scenarios   — one row per change: Scenario, Variable, Value",
        "TimeSeries  — first column Time, then one column per data series (each becomes a lookup over time)",
        "",
        "SETS — many things with the same structure (tanks, regions, products) are written once:",
        "Sets        — Set, Item, Belongs to (e.g. Tank / Hail 2 - Crude / COA, so each tank knows its region)",
        "Per         — an extra column on Parameters, Stocks, Flows, Variables and Lookups: name the set the row exists once for (e.g. Tank)",
        "Combine     — optional, same sheets: Total (default) or Average — how the items show as one number on the diagram and in charts",
        "Values      — Name, Item, Value: an item whose number or formula differs from the row's default",
        "ItemSeries  — Name, Item, Time, Value: a curve or data series for each item (the variable is a Lookup per the set, with input Time)",
        "In formulas: another per-Tank variable means the same tank's value; TOTAL(Stock) adds up all tanks (inside a per-Region variable: that region's tanks); TOTAL(Stock, COA) one group; Stock[Hail 2 - Crude] one item.",
        "",
        "Equations use names with spaces, + - * / ^, IF THEN ELSE(c, a, b), SMOOTH(x, t), DELAY3(x, t), STEP(h, t) and more — see the wiki's Function reference.",
    ]
    .iter()
    .map(|s| vec![s.to_string()])
    .collect();
    b.sheets.insert(0, ("README".into(), readme));
    b.sheets.push(("TimeSeries".into(), vec![vec!["Time".into(), "Example data".into()], vec!["0".into(), "10".into()], vec!["30".into(), "14".into()], vec!["60".into(), "12".into()]]));
    b
}

/// Write a workbook as .xlsx with a bold, shaded header row and sensible column widths.
pub fn write_xlsx(book: &Book, path: &Path) -> Result<(), String> {
    use rust_xlsxwriter::{Color, Format, Workbook};
    let mut wb = Workbook::new();
    let head = Format::new().set_bold().set_background_color(Color::RGB(0xEEF0F7));
    let title = Format::new().set_bold().set_font_color(Color::RGB(0x4F46E5));
    for (name, rows) in &book.sheets {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(|e| e.to_string())?;
        let mut widths: HashMap<u16, usize> = HashMap::new();
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let (r, c) = (r as u32, c as u16);
                let w = widths.entry(c).or_insert(8);
                *w = (*w).max(cell.chars().count().min(70));
                if r == 0 && name != "README" {
                    ws.write_string_with_format(r, c, cell, &head).map_err(|e| e.to_string())?;
                } else if r == 0 {
                    ws.write_string_with_format(r, c, cell, &title).map_err(|e| e.to_string())?;
                } else if let Some(x) = cell.trim().parse::<f64>().ok().filter(|_| !cell.trim().is_empty()) {
                    ws.write_number(r, c, x).map_err(|e| e.to_string())?;
                } else if !cell.is_empty() {
                    ws.write_string(r, c, cell).map_err(|e| e.to_string())?;
                }
            }
        }
        if name != "README" {
            for (c, w) in widths {
                ws.set_column_width(c, (w as f64 + 2.0).min(60.0)).map_err(|e| e.to_string())?;
            }
            ws.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
        }
    }
    wb.save(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_survives_a_trip_through_excel() {
        let original = Model::example();
        let path = std::env::temp_dir().join("fc_roundtrip.xlsx");
        write_xlsx(&book_from_model(&original), &path).unwrap();
        let book = read_book(&path).unwrap();
        let (m, notes) = model_from_book(&book).unwrap();
        assert_eq!(m.title, original.title);
        assert_eq!(m.vars.len(), original.vars.len(), "{notes:?}");
        assert_eq!(m.scenarios.len(), 3);
        assert_eq!(m.by_name("effect of awareness").unwrap().table.len(), 7);
        let run = |m: &Model| {
            let c = crate::sim::compile(m).map_err(|p| format!("{:?}", p.iter().map(|x| x.message.clone()).collect::<Vec<_>>())).unwrap();
            crate::sim::simulate(&c, m, "x").unwrap()
        };
        let (a, b) = (run(&original), run(&m));
        let (fa, fb) = (a.get("Infected").unwrap(), b.get("Infected").unwrap());
        assert!((fa.last().unwrap() - fb.last().unwrap()).abs() < 1e-9, "same results after the round trip");
        assert!(m.sheets.iter().any(|s| s.kind == SheetKind::CausalLoop && !s.cld.links.is_empty()));
    }

    /// The fuel-supply workbook, restructured by wiki/inbox/restructure_fuel_workbook.py, must give
    /// the answers the original workbook gives for every tank that has a plan.
    #[test]
    fn fuel_supply_model_matches_the_workbook() {
        let dir = Path::new(WIKI_DIR).join("inbox").join("fuel-supply");
        let Ok(expected) = std::fs::read_to_string(Path::new(WIKI_DIR).join("inbox").join("fuel-supply-expected.csv")) else { return };
        let (m, notes) = model_from_book(&read_book(&dir).unwrap()).unwrap();
        let problems: Vec<_> = notes.iter().filter(|n| !n.starts_with("Added ") && !n.contains("went to the causal loop diagram only") && !n.contains("module(s) built")).collect();
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(m.sets.len(), 2);
        assert_eq!(m.sets[1].items.len(), 57);
        let cld = m.sheets.iter().find(|s| s.kind == SheetKind::CausalLoop).unwrap();
        assert!(cld.cld.nodes.len() >= 20 && cld.cld.links.len() >= 20, "causal loop diagram: {} words, {} arrows", cld.cld.nodes.len(), cld.cld.links.len());
        assert!(m.modules.iter().all(|x| !x.description.is_empty()), "every module says what it is for");
        // three levels: the overview holds Plans, which holds Optimal plan, which holds three more
        let top: Vec<&str> = m.modules.iter().filter(|x| x.view == m.first_diagram()).map(|x| x.name.as_str()).collect();
        assert_eq!(top, ["Inputs & limits", "Plans", "Totals"]);
        let paths: Vec<String> = m.modules.iter().map(|x| module_path(&m, x.id)).collect();
        assert!(paths.contains(&"Plans / Optimal plan / Trucks & skids".to_string()), "{paths:?}");
        assert_eq!(m.modules.len(), 10);
        // the overview shows only the five modules: no loose variables on it
        assert!(m.vars.iter().all(|v| v.view != m.first_diagram()), "variables left on the overview");
        let found = crate::cld::find_loops(&cld.cld);
        assert!(!found.is_empty(), "the causal loop diagram should contain the stock-filling feedback loops");
        println!("CLD: {} words, {} arrows, {} loops found", cld.cld.nodes.len(), cld.cld.links.len(), found.len());
        let c = crate::sim::compile(&m).map_err(|p| p.iter().map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>()).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        let at = |name: &str, t: f64| {
            let i = r.times.iter().position(|x| (*x - t).abs() < 1e-9).unwrap();
            r.get(name).unwrap_or_else(|| panic!("no series {name}"))[i]
        };
        let (mut checked, mut bad) = (0, Vec::new());
        for row in parse_delimited(&expected).iter().skip(1) {
            let f = |i: usize| row[i].parse::<f64>().ok();
            if f(1) != Some(1.0) {
                continue;
            }
            let tank = &row[0];
            checked += 1;
            let trucks: f64 = (15..=30).map(|d| at(&format!("Trucks planned[{tank}]"), d as f64)).sum();
            let got = [trucks, at(&format!("Closing fill optimal[{tank}]"), 25.0), at(&format!("Closing fill current[{tank}]"), 25.0), at(&format!("Shortage vs target[{tank}]"), 25.0)];
            let want = [f(2).unwrap(), f(3).unwrap(), f(4).unwrap(), f(5).unwrap()];
            for k in 0..4 {
                let tol = if k == 0 { 0.5 } else if k == 3 { 1e-4 } else { 1e-6 };
                if (got[k] - want[k]).abs() > tol {
                    bad.push(format!("{tank} #{k}: model {} vs workbook {}", got[k], want[k]));
                }
            }
        }
        assert!(checked >= 45, "only {checked} tanks checked");
        assert!(bad.is_empty(), "{} differences, e.g. {:?}", bad.len(), &bad[..bad.len().min(8)]);
    }

    #[test]
    fn tidy_up_gives_every_variable_its_own_place() {
        let dir = Path::new(WIKI_DIR).join("inbox").join("fuel-supply");
        if !dir.exists() {
            return;
        }
        let (mut m, _) = model_from_book(&read_book(&dir).unwrap()).unwrap();
        for v in m.vars.iter_mut() {
            v.pos = [0.0, 0.0];
        }
        let sheets: Vec<u64> = m.sheets.iter().filter(|s| s.kind == SheetKind::Diagram).map(|s| s.id).collect();
        for view in sheets {
            tidy(&mut m, view);
            let calc: Vec<_> = m.vars.iter().filter(|v| v.view == view && matches!(v.kind, Kind::Aux | Kind::Lookup)).collect();
            for (i, a) in calc.iter().enumerate() {
                for b in &calc[i + 1..] {
                    assert!((a.pos[0] - b.pos[0]).abs() > 100.0 || (a.pos[1] - b.pos[1]).abs() > 40.0, "{} and {} overlap", a.name, b.name);
                }
            }
            let (w, h) = calc.iter().fold((0f32, 0f32), |(w, h), v| (w.max(v.pos[0]), h.max(v.pos[1])));
            assert!(w < 2600.0 && h < 2400.0, "diagram too spread out: {w} x {h}");
        }
    }

    #[test]
    fn sets_survive_a_trip_through_excel() {
        let dir = Path::new(WIKI_DIR).join("inbox").join("fuel-supply");
        if !dir.exists() {
            return;
        }
        let (m, _) = model_from_book(&read_book(&dir).unwrap()).unwrap();
        let path = std::env::temp_dir().join("fc_sets_roundtrip.xlsx");
        write_xlsx(&book_from_model(&m), &path).unwrap();
        let (m2, notes) = model_from_book(&read_book(&path).unwrap()).unwrap();
        assert_eq!(m2.sets.len(), 2, "{notes:?}");
        assert_eq!(m2.vars.len(), m.vars.len());
        let run = |m: &Model| crate::sim::simulate(&crate::sim::compile(m).ok().unwrap(), m, "x").unwrap();
        let (a, b) = (run(&m), run(&m2));
        assert_eq!(a.get("Network stock optimal").unwrap().last(), b.get("Network stock optimal").unwrap().last());
        assert_eq!(a.get("Network stock current").unwrap().last(), b.get("Network stock current").unwrap().last());
    }

    #[test]
    fn my_functions_survive_a_trip_through_excel() {
        let mut m = Model::example();
        m.functions.push(crate::userfn::UserFn { name: "Half".into(), params: vec!["x".into()], body: "x / 2".into(), description: "half of it".into() });
        m.by_name("contact rate").map(|v| v.id).map(|id| m.var_mut(id).unwrap().equation = "Half(12)".into());
        let path = std::env::temp_dir().join("fc_functions.xlsx");
        write_xlsx(&book_from_model(&m), &path).unwrap();
        let (back, notes) = model_from_book(&read_book(&path).unwrap()).unwrap();
        assert_eq!(back.functions.len(), 1, "{notes:?}");
        assert_eq!(back.functions[0].params, vec!["x".to_string()]);
        let run = |m: &Model| crate::sim::simulate(&crate::sim::compile(m).ok().unwrap(), m, "x").unwrap();
        assert_eq!(run(&m).get("Contact Rate").unwrap()[0], 6.0);
        assert_eq!(run(&back).get("Contact Rate").unwrap()[0], 6.0);
    }

    #[test]
    fn loose_hand_made_csv_imports() {
        let text = "Parameter,Value,Unit\nBirth rate,0.03,1/Year\n\"Average lifetime\",70,Year\n";
        let book = Book { sheets: vec![("Parameters".into(), parse_delimited(text))] };
        let (m, _) = model_from_book(&book).unwrap();
        assert_eq!(m.by_name("birth rate").unwrap().equation, "0.03");
        assert_eq!(m.by_name("average lifetime").unwrap().units, "Year");
        assert_eq!(parse_points("(0,1) (0.5, 0.7); (1,0.2)").len(), 3);
        assert_eq!(parse_points("(0,1) (0.5,0.7) (1,0.2)").len(), 3);
    }
}

// ------------------------------------------------------------------ in the app

use crate::app::App;

/// Where the documentation lives (next to the app's source).
pub const WIKI_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wiki");

impl App {
    pub fn tidy_diagram(&mut self) {
        let view = self.sheet;
        if !self.model.vars.iter().any(|v| v.view == view) {
            self.status = "Open a diagram sheet to tidy it up.".into();
            return;
        }
        self.checkpoint();
        tidy(&mut self.model, view);
        self.changed();
        self.fit_pending = true;
        self.status = "Tidied up the diagram — ⌘Z undoes it.".into();
    }

    pub fn import_spreadsheet(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Spreadsheet", &["xlsx", "xlsm", "xls", "ods", "csv"]).pick_file() else { return };
        self.import_spreadsheet_path(&path);
    }

    pub fn import_spreadsheet_path(&mut self, path: &Path) {
        match read_book(path).and_then(|b| model_from_book(&b)) {
            Ok((mut m, notes)) => {
                if m.title == "Imported model" {
                    m.title = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                }
                self.checkpoint();
                self.sheet = m.first_diagram();
                self.plotted = m.vars.iter().filter(|v| v.kind == Kind::Stock).take(4).map(|v| v.name.clone()).collect();
                self.model = m;
                self.path = None;
                self.saved_model_json = String::new();
                self.saved_runs.clear();
                self.select_one(None);
                self.fit_pending = true;
                self.changed();
                self.simulate();
                let mut report = vec![format!("Built “{}”: {} variables, {} arrows, {} scenarios.", self.model.title, self.model.vars.len(), self.model.links.len(), self.model.scenarios.len())];
                if !self.problems.is_empty() {
                    report.push(format!("{} equation problem(s) — see the Problems tab.", self.problems.len()));
                }
                report.extend(notes);
                self.status = report[0].clone();
                self.import_notes = Some(report);
            }
            Err(e) => self.status = format!("Couldn't import the spreadsheet: {e}"),
        }
    }

    pub fn export_spreadsheet(&mut self) {
        let name = format!("{}.xlsx", if self.model.title.is_empty() { "model" } else { &self.model.title });
        let Some(path) = rfd::FileDialog::new().add_filter("Excel workbook", &["xlsx"]).set_file_name(name).save_file() else { return };
        self.status = match write_xlsx(&book_from_model(&self.model), &path) {
            Ok(()) => format!("Saved the model as a spreadsheet: {}", path.display()),
            Err(e) => format!("Couldn't save the spreadsheet: {e}"),
        };
    }

    pub fn save_template(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Excel workbook", &["xlsx"]).set_file_name("Dynamist model template.xlsx").save_file() else { return };
        self.status = match write_xlsx(&template_book(), &path) {
            Ok(()) => format!("Saved the template: {}", path.display()),
            Err(e) => format!("Couldn't save the template: {e}"),
        };
    }

    pub fn open_wiki(&mut self) {
        let path = Path::new(WIKI_DIR);
        let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
        self.status = match std::process::Command::new(opener).arg(path).spawn() {
            Ok(_) => format!("Opened the documentation folder: {}", path.display()),
            Err(e) => format!("Couldn't open {}: {e}", path.display()),
        };
    }

    /// What happened during a spreadsheet import.
    pub fn import_report_window(&mut self, ctx: &egui::Context) {
        let Some(notes) = self.import_notes.clone() else { return };
        let pal = self.pal;
        let mut open = true;
        egui::Window::new("Spreadsheet import").open(&mut open).collapsible(false).default_width(560.0).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
            ui.label(egui::RichText::new(&notes[0]).size(15.0).strong());
            ui.add_space(6.0);
            if notes.len() > 1 {
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    for n in &notes[1..] {
                        ui.label(egui::RichText::new(format!("•  {n}")).color(pal.text));
                    }
                });
            } else {
                ui.label(egui::RichText::new("Everything in the workbook was used.").color(pal.ok));
            }
            ui.add_space(8.0);
            ui.label(egui::RichText::new("The model, its equations, results, a causal loop diagram and any scenarios are on their own sheets. ⌘Z undoes the import.").small().color(pal.muted));
        });
        if !open {
            self.import_notes = None;
        }
    }
}

/// Write the generated parts of the wiki: the function reference and the template workbook.
pub fn write_generated_docs(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut md = String::from("# Function reference\n\n> Generated from the app (`src/docs.rs`) — edit there, then regenerate with `dynamist --write-docs wiki`. Every entry here is also in the app: **Help → Function reference** or **F1**.\n\n");
    for g in crate::docs::GROUPS {
        md.push_str(&format!("## {g}\n\n"));
        for d in crate::docs::DOCS.iter().filter(|d| d.group == *g) {
            md.push_str(&format!("### {}\n\n{}\n\n```\n{}\n```\n\n", d.name, d.summary, d.signature));
            if let Some((_, v, st, note)) = crate::docs::equivalents(d.name) {
                md.push_str(&format!("**In Vensim:** `{v}` · **In Stella:** `{st}`{}\n\n", if note.is_empty() { String::new() } else { format!(" — {note}") }));
            }
            if !d.params.is_empty() {
                md.push_str("| Parameter | Meaning |\n|---|---|\n");
                for (n, x) in d.params {
                    md.push_str(&format!("| `{n}` | {x} |\n"));
                }
                md.push('\n');
            }
            md.push_str(&format!("**Result:** {}\n\n", d.returns));
            if !d.details.is_empty() {
                md.push_str(&format!("{}\n\n", d.details));
            }
            if !d.example.is_empty() {
                md.push_str(&format!("**Example:** `{}`\n\n", d.example));
            }
            for t in d.tips {
                md.push_str(&format!("- Tip: {t}\n"));
            }
            if !d.see.is_empty() {
                md.push_str(&format!("\nSee also: {}\n", d.see.join(", ")));
            }
            md.push_str("\n---\n\n");
        }
    }
    std::fs::write(dir.join("Function-Reference.md"), md).map_err(|e| e.to_string())?;
    write_xlsx(&template_book(), &dir.join("templates").join("Dynamist-Model-Template.xlsx")).or_else(|_| {
        std::fs::create_dir_all(dir.join("templates")).map_err(|e| e.to_string())?;
        write_xlsx(&template_book(), &dir.join("templates").join("Dynamist-Model-Template.xlsx"))
    })?;
    // the example model in the template, as CSVs too, for tools that can't read Excel
    for (name, rows) in template_book().sheets.iter().filter(|(n, _)| n != "README") {
        let csv: String = rows
            .iter()
            .map(|r| r.iter().map(|c| if c.contains(',') || c.contains('"') { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.clone() }).collect::<Vec<_>>().join(","))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.join("templates").join(format!("{name}.csv")), csv + "\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}
