//! A Model Context Protocol server over stdio (newline-delimited JSON-RPC). An agent edits and
//! runs models only through these tools: every change is an `apply_ops` batch that is validated
//! before it is accepted, so a generated model cannot be saved in a broken state.

use std::io::{BufRead, Write};

use dynamist_core::model::Model;
use dynamist_core::{diff, ops, runner, validate};
use serde_json::{Value, json};

use crate::files;

struct State {
    session: ops::Session,
    path: Option<String>,
}

fn tool_list() -> Value {
    let t = |name: &str, desc: &str, props: Value, required: &[&str]| json!({"name": name, "description": desc, "inputSchema": {"type": "object", "properties": props, "required": required}});
    let s = |d: &str| json!({"type": "string", "description": d});
    json!([
        t("new_model", "Start a new empty model (replaces the one in memory).", json!({"title": s("Model title")}), &[]),
        t("open_model", "Open a model file (.sdm, .stmx/.xmile or .mdl). Returns what the format could not carry.", json!({"path": s("File path")}), &["path"]),
        t("save_model", "Save the model. The format follows the extension, or `format` (sdm, xmile, mdl). Returns the loss report.", json!({"path": s("File path"), "format": s("sdm, xmile or mdl")}), &["path"]),
        t("describe_api", "The full reference for the operations language: every operation with its fields, an example and when it is refused; the validation codes and queries. Read this before building a model.", json!({}), &[]),
        t("info", "Counts of stocks, flows, variables, sets, checks and the simulation settings.", json!({}), &[]),
        t("list_vars", "List variables with equation, units, flow ends. Optionally only one kind.", json!({"kind": s("stock, flow, aux, constant or lookup")}), &[]),
        t("get_var", "One variable in full.", json!({"name": s("Variable name")}), &["name"]),
        t("dependencies", "What a variable depends on and what uses it.", json!({"name": s("Variable name")}), &["name"]),
        t("loops", "Feedback loops of the model (reinforcing or balancing) with the variables around each.", json!({}), &[]),
        t("layers", "The stack of layers: each one's question, status, horizon, counts, interfaces; unassigned variables and cross-layer loops. Layers are changed with add_layer, set_layer, remove_layer, assign_layer, freeze_interfaces, add_stub, remove_stub (see describe_api).", json!({}), &[]),
        t("layer_check", "Check the layering: unassigned variables, undeclared crossings between layers, contracts that no longer match, horizons that contradict the order of feeding.", json!({}), &[]),
        t("layer_decouple", "The decoupling test for one layer: run it alone on inputs recorded from the full run (must match), and measure how much the other layers react when its own constants change.", json!({"name": s("Layer name"), "max_nrmse": {"type": "number"}}), &["name"]),
        t("structure_notes", "Stocks without flows and variables nothing uses.", json!({}), &[]),
        t("apply_ops", "Apply a batch of operations atomically. The batch is validated and refused as a whole if the result has errors. Operations (field `op`): add_var{name,kind,equation,units,doc,from,to,non_negative}, remove_var{name}, rename_var{name,to}, set_equation{name,equation}, set_units{name,units}, set_doc{name,doc}, set_non_negative{name,on}, set_flow{name,from,to}, set_table{name,points}, connect{from,to,polarity}, disconnect{from,to}, set_setting{key,value} (initial_time, final_time, time_step, time_units, method, title), add_set{name,items,parent,parent_of}, set_per{var,set}, set_item_value{var,item,equation}, set_lever{var,on}, add_check{name,var,min,max,when}, remove_check{name}, add_function{name,params,body,doc}, remove_function{name}, add_layer{name,order,question,horizon,structural_limit,status}, set_layer{name,key,value}, remove_layer{name}, assign_layer{layer,vars,sheet,module}, freeze_interfaces{}, add_stub{layer,name,series}, remove_stub{name} (add_var also takes layer). Kinds: stock, flow, aux, constant, lookup. A flow's from/to name the stocks it drains and fills.", json!({"ops": {"type": "array", "items": {"type": "object"}, "description": "The operations"}, "actor": s("Who is making the change"), "reason": s("Why: one sentence, recorded in the log")}), &["ops"]),
        t("draft_manifest", "Draft a manifest from the model's key levers, stocks and checks (not applied: review it and submit it with apply_ops set_manifest).", json!({}), &[]),
        t("explain_problems", "Validate and return every issue with a plain-language hint on how to fix it. Use after a refused batch.", json!({"strict_units": {"type": "boolean"}}), &[]),
        t("validate", "Check the whole model: equations, units (dimensional algebra), arrows, the model's own checks and stock conservation. Returns structured issues. strict_units: a rate may not meet its amount.", json!({"strict_units": {"type": "boolean"}}), &[]),
        t("run", "Simulate, optionally with overrides (name → value or formula). Returns a summary per series, or the values every `every`th step.", json!({"overrides": {"type": "object", "description": "name → value"}, "series": {"type": "array", "items": {"type": "string"}}, "every": {"type": "integer"}}), &[]),
        t("optimize", "Search for lever values that minimise (or maximise) a result, within limits (random sampling then Nelder-Mead). Does not change the model: apply the best values with set_equation.", json!({"goal": s("Variable to optimise"), "measure": s("final, peak, lowest, total or average"), "maximize": {"type": "boolean"}, "levers": {"type": "array", "items": {"type": "object"}, "description": "[{name, min, max}]"}, "limits": {"type": "array", "items": {"type": "string"}, "description": "like Stock>=0:always or Shortage<=1000:final"}, "effort": {"type": "integer"}}), &["goal", "levers"]),
        t("sensitivity", "Monte Carlo: vary the key levers (or every constant) at random by ± spread, run many times, return the band of outcomes (5/25/50/75/95 percent) for one variable.", json!({"var": s("Variable to follow"), "runs": {"type": "integer"}, "spread": {"type": "number", "description": "0.2 = ±20%"}, "seed": {"type": "integer"}}), &["var"]),
        t("screen", "Morris screening: which levers of the manifest matter for its goals? Moves one lever at a time over its declared range (r trajectories), returns per goal and lever mu, mu_star (importance), sigma (non-linearity or interaction), a class (negligible, linear, nonlinear_or_interacting) and a convergence check (ranking with r against 2r trajectories). Needs manifest levers (with ranges) and goals.", json!({"goals": {"type": "array", "items": {"type": "string"}, "description": "goal names (default: all)"}, "params": {"type": "array", "items": {"type": "string"}, "description": "lever names (default: all)"}, "trajectories": {"type": "integer"}, "levels": {"type": "integer"}, "seed": {"type": "integer"}, "tau": {"type": "number", "description": "negligible below tau × the largest mu_star"}}), &[]),
        t("importance", "Variance-based importance (Sobol first-order and total indices with bootstrap intervals and a convergence check), which feedback loop dominates (base run and an ensemble of parameter draws), eigenvalue elasticities at a time (optional), and the variables that cannot influence any goal. Needs manifest levers and goals. Cost: samples × (levers + 2) × 2 runs.", json!({"goals": {"type": "array", "items": {"type": "string"}}, "params": {"type": "array", "items": {"type": "string"}, "description": "lever names (default: all; pass those that survived `screen`)"}, "samples": {"type": "integer"}, "seed": {"type": "integer"}, "ensemble": {"type": "integer"}, "eigen_at": {"type": "number"}}), &[]),
        t("dominance", "Loops That Matter: which feedback loop dominates the behaviour at each time (base run). Euler only; loops through DELAY, TREND or queues are listed but not scored.", json!({}), &[]),
        t("diff_with_file", "What differs in meaning between the model in memory and a file.", json!({"path": s("File path")}), &["path"]),
        t("undo", "Take back the last committed batch.", json!({}), &[]),
        t("redo", "Redo an undone batch.", json!({}), &[]),
        t("log", "The log of committed batches: who, when, what.", json!({}), &[]),
    ])
}

fn arg_str(a: &Value, k: &str) -> Result<String, String> {
    a.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| format!("missing “{k}”"))
}

fn call(st: &mut State, name: &str, a: &Value) -> Result<Value, String> {
    Ok(match name {
        "new_model" => {
            let mut m = Model::default();
            m.title = a.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled model").to_string();
            m.ensure_sheets();
            st.session = ops::Session::new(m);
            st.path = None;
            json!({"ok": true})
        }
        "open_model" => {
            let p = arg_str(a, "path")?;
            let (m, loss) = files::open(&p)?;
            st.session = ops::Session::new(m);
            st.path = Some(p);
            json!({"ok": true, "lossless": loss.is_lossless(), "loss": loss.items, "info": ops::info(&st.session.model)})
        }
        "save_model" => {
            let p = arg_str(a, "path")?;
            let loss = files::save(&st.session.model, &p, a.get("format").and_then(|v| v.as_str()))?;
            st.path = Some(p);
            json!({"ok": true, "lossless": loss.is_lossless(), "loss": loss.items})
        }
        "describe_api" => dynamist_core::apidoc::json(),
        "info" => ops::info(&st.session.model),
        "list_vars" => ops::list(&st.session.model, a.get("kind").and_then(|v| v.as_str()))?,
        "get_var" => ops::get(&st.session.model, &arg_str(a, "name")?)?,
        "dependencies" => ops::dependencies(&st.session.model, &arg_str(a, "name")?)?,
        "loops" => ops::loops(&st.session.model),
        "structure_notes" => ops::structure_notes(&st.session.model),
        "layers" => dynamist_core::layers::summary(&st.session.model),
        "layer_check" => {
            let r = dynamist_core::layers::check(&st.session.model);
            json!({"ok": r.errors() == 0, "errors": r.errors(), "crossings": r.crossings, "contracts": r.contracts, "cross_layer_loops": r.cross_layer_loops, "issues": r.issues})
        }
        "layer_decouple" => {
            let m = &st.session.model;
            let name = arg_str(a, "name")?;
            let l = m.layer_by_name(&name).cloned().ok_or_else(|| format!("there is no layer called “{name}”"))?;
            dynamist_core::layers::decouple(m, &l, a.get("max_nrmse").and_then(|v| v.as_f64()).unwrap_or(0.05))?
        }
        "apply_ops" => {
            let batch: Vec<ops::Op> = serde_json::from_value(a.get("ops").cloned().ok_or("missing “ops”")?).map_err(|e| format!("not a list of operations: {e}"))?;
            let actor = a.get("actor").and_then(|v| v.as_str()).unwrap_or("agent:mcp").to_string();
            let reason = a.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string());
            match st.session.apply_with_reason(&actor, reason.as_deref(), &batch) {
                Ok(entry) => {
                    let entry = entry.clone();
                    if let Some(p) = &st.path {
                        files::save(&st.session.model, p, None)?;
                        files::append_log(p, &entry)?;
                    }
                    json!({"ok": true, "seq": entry.seq, "warnings": entry.warnings})
                }
                Err(r) => {
                    if let (Some(p), Some(e)) = (&st.path, st.session.log.last()) {
                        let _ = files::append_log(p, e);
                    }
                    json!({"ok": false, "rejected": r})
                }
            }
        }
        "draft_manifest" => json!(dynamist_core::manifest::draft(&st.session.model)),
        "explain_problems" => {
            let r = validate::validate_with(&st.session.model, &validate::Options { strict_units: a.get("strict_units").and_then(|v| v.as_bool()).unwrap_or(false) });
            json!({"ok": r.ok(), "issues": validate::explain(&r)})
        }
        "validate" => {
            let r = validate::validate_with(&st.session.model, &validate::Options { strict_units: a.get("strict_units").and_then(|v| v.as_bool()).unwrap_or(false) });
            json!({"ok": r.ok(), "ran": r.ran, "issues": r.issues})
        }
        "run" => {
            let over: Vec<(String, String)> = a.get("overrides").and_then(|v| v.as_object()).map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string()))).collect()).unwrap_or_default();
            let series: Vec<String> = a.get("series").and_then(|v| v.as_array()).map(|x| x.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
            let run = runner::run_with(&st.session.model, &over)?;
            let outside = dynamist_core::manifest::range_warnings(&st.session.model, &over);
            if !outside.is_empty() {
                let mut v = runner::summary_json(&run, &series);
                v["outside_declared_range"] = json!(outside);
                return Ok(v);
            }
            match a.get("every").and_then(|v| v.as_u64()) {
                Some(n) if n > 0 => runner::series_json(&run, &series, n as usize),
                _ => runner::summary_json(&run, &series),
            }
        }
        "optimize" => {
            use dynamist_core::model::{Lever, OptSpec};
            let levers: Vec<Lever> = a.get("levers").and_then(|v| v.as_array()).ok_or("missing “levers”")?.iter().map(|l| Ok(Lever { name: l.get("name").and_then(|v| v.as_str()).ok_or("a lever needs a name")?.to_string(), on: true, min: l.get("min").and_then(|v| v.as_f64()).ok_or("a lever needs a min")?, max: l.get("max").and_then(|v| v.as_f64()).ok_or("a lever needs a max")? })).collect::<Result<_, String>>()?;
            let limits = a.get("limits").and_then(|v| v.as_array()).map(|x| x.iter().filter_map(|s| s.as_str()).map(dynamist_core::optimize::parse_limit).collect::<Result<Vec<_>, _>>()).transpose()?.unwrap_or_default();
            let spec = OptSpec { maximize: a.get("maximize").and_then(|v| v.as_bool()).unwrap_or(false), goal: arg_str(a, "goal")?, measure: dynamist_core::optimize::parse_measure(a.get("measure").and_then(|v| v.as_str()).unwrap_or("final"))?, levers, limits, effort: a.get("effort").and_then(|v| v.as_u64()).unwrap_or(2) as u8 };
            let p = dynamist_core::optimize::run_blocking(st.session.model.clone(), spec.clone())?;
            dynamist_core::optimize::result_json(&spec, &p)
        }
        "sensitivity" => runner::sensitivity_json(&st.session.model, &arg_str(a, "var")?, a.get("runs").and_then(|v| v.as_u64()).unwrap_or(100) as usize, a.get("spread").and_then(|v| v.as_f64()).unwrap_or(0.2), a.get("seed").and_then(|v| v.as_u64()).unwrap_or(1))?,
        "screen" => {
            let list = |k: &str| a.get(k).and_then(|v| v.as_array()).map(|x| x.iter().filter_map(|s| s.as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default();
            let mut spec = dynamist_core::screen::ScreenSpec::default();
            if let Some(v) = a.get("trajectories").and_then(|v| v.as_u64()) { spec.trajectories = v as usize; }
            if let Some(v) = a.get("levels").and_then(|v| v.as_u64()) { spec.levels = v as usize; }
            if let Some(v) = a.get("seed").and_then(|v| v.as_u64()) { spec.seed = v; }
            if let Some(v) = a.get("tau").and_then(|v| v.as_f64()) { spec.tau = v; }
            dynamist_core::screen::screen_json(&st.session.model, &list("params"), &list("goals"), &spec)?
        }
        "importance" => {
            let list = |k: &str| a.get(k).and_then(|v| v.as_array()).map(|x| x.iter().filter_map(|s| s.as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default();
            let mut spec = dynamist_core::importance::ImportanceSpec::default();
            if let Some(v) = a.get("samples").and_then(|v| v.as_u64()) { spec.sobol.samples = v as usize; }
            if let Some(v) = a.get("seed").and_then(|v| v.as_u64()) { spec.sobol.seed = v; }
            if let Some(v) = a.get("ensemble").and_then(|v| v.as_u64()) { spec.ensemble = v as usize; }
            spec.eigen_at = a.get("eigen_at").and_then(|v| v.as_f64());
            let m = &st.session.model;
            dynamist_core::importance::importance_json(m, &dynamist_core::screen::params_of(m, &list("params"))?, &dynamist_core::screen::goals_of(m, &list("goals"))?, &spec)?
        }
        "dominance" => dynamist_core::importance::dominance_json(&st.session.model)?,
        "diff_with_file" => {
            let (other, _) = files::open(&arg_str(a, "path")?)?;
            let d = diff::diff_models(&st.session.model, &other);
            json!({"same": d.is_empty(), "differences": d})
        }
        "undo" => json!({"ok": st.session.undo()}),
        "redo" => json!({"ok": st.session.redo()}),
        "log" => json!(st.session.log),
        other => return Err(format!("unknown tool “{other}”")),
    })
}

fn reply(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Handle one JSON-RPC message; None for notifications.
fn handle(st: &mut State, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else { return None }; // a notification: no answer
    Some(match method {
        "initialize" => reply(&id, json!({
            "protocolVersion": params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or("2024-11-05"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "dynamist", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Edit models only with apply_ops batches; validate after changes; a batch that does not validate is refused whole."
        })),
        "ping" => reply(&id, json!({})),
        "tools/list" => reply(&id, json!({"tools": tool_list()})),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call(st, name, &args) {
                Ok(v) => reply(&id, json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&v).unwrap()}], "isError": false})),
                Err(e) => reply(&id, json!({"content": [{"type": "text", "text": e}], "isError": true})),
            }
        }
        _ => error(&id, -32601, "method not found"),
    })
}

pub fn serve() {
    let mut st = State { session: ops::Session::new(Model::default()), path: None };
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&mut st, &msg),
            Err(e) => Some(error(&Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(a) = answer {
            let _ = writeln!(out, "{a}");
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpc(st: &mut State, method: &str, params: Value) -> Value {
        handle(st, &json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params})).unwrap()
    }

    fn tool(st: &mut State, name: &str, args: Value) -> Value {
        let r = rpc(st, "tools/call", json!({"name": name, "arguments": args}));
        assert_eq!(r["result"]["isError"], json!(false), "{r}");
        serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    #[test]
    fn an_agent_builds_validates_runs_and_saves_a_model_through_tools_alone() {
        let mut st = State { session: ops::Session::new(Model::default()), path: None };
        let init = rpc(&mut st, "initialize", json!({"protocolVersion": "2025-03-26"}));
        assert_eq!(init["result"]["serverInfo"]["name"], json!("dynamist"));
        let tools = rpc(&mut st, "tools/list", json!({}));
        assert!(tools["result"]["tools"].as_array().unwrap().len() >= 20);
        assert!(handle(&mut st, &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());

        tool(&mut st, "new_model", json!({"title": "Inventory"}));
        let r = tool(&mut st, "apply_ops", json!({"actor": "agent:test", "ops": [
            {"op": "set_setting", "key": "final_time", "value": "30"},
            {"op": "set_setting", "key": "time_units", "value": "Day"},
            {"op": "add_var", "name": "Stock", "kind": "stock", "equation": "100", "units": "Unit"},
            {"op": "add_var", "name": "Target", "kind": "constant", "equation": "120", "units": "Unit"},
            {"op": "add_var", "name": "Adjustment time", "kind": "constant", "equation": "5", "units": "Day"},
            {"op": "add_var", "name": "Orders", "kind": "flow", "equation": "MAX(0, (Target - Stock) / Adjustment time)", "units": "Unit/Day", "to": "Stock"},
            {"op": "add_check", "name": "Reaches the target", "var": "Stock", "min": 115, "when": "end"}
        ]}));
        assert_eq!(r["ok"], json!(true), "{r}");
        assert_eq!(tool(&mut st, "validate", json!({}))["ok"], json!(true));
        let run = tool(&mut st, "run", json!({"series": ["Stock"]}));
        let last = run["series"][0]["last"].as_f64().unwrap();
        assert!(last > 115.0 && last <= 120.0, "{last}");
        // a bad batch is refused whole, and the model is untouched
        let bad = tool(&mut st, "apply_ops", json!({"ops": [{"op": "set_equation", "name": "Orders", "equation": "Target -"}]}));
        assert_eq!(bad["ok"], json!(false));
        assert_eq!(tool(&mut st, "get_var", json!({"name": "Orders"}))["equation"], json!("MAX(0, (Target - Stock) / Adjustment time)"));
        // what if the adjustment is very slow: the stated limit breaks the gate
        let slow = tool(&mut st, "apply_ops", json!({"ops": [{"op": "set_equation", "name": "Adjustment time", "equation": "500"}]}));
        assert_eq!(slow["ok"], json!(false));
        assert_eq!(tool(&mut st, "loops", json!({})).as_array().unwrap().len(), 1);
        let band = tool(&mut st, "sensitivity", json!({"var": "Stock", "runs": 20, "spread": 0.1}));
        assert_eq!(band["runs"], json!(20));
        assert!(band["end"]["p5"].as_f64().unwrap() <= band["end"]["p95"].as_f64().unwrap());
        let dir = std::env::temp_dir().join(format!("dyn-mcp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("inventory.sdm");
        let saved = tool(&mut st, "save_model", json!({"path": p.to_str().unwrap()}));
        assert_eq!(saved["ok"], json!(true));
        let again = tool(&mut st, "open_model", json!({"path": p.to_str().unwrap()}));
        assert_eq!(again["info"]["stocks"], json!(1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
