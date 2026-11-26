//! `dyn layers` and `dyn layer ...`: the stack of layers (see core/src/layers.rs).

use dynamist_core::layers::{self, Input};
use dynamist_core::{ops, runner};
use serde_json::json;
use std::collections::HashMap;

use crate::{die, files, has_flag, print, take_flag};

/// Apply a batch through the operations language, save and log it, like `dyn ops`.
fn apply(file: &str, actor: &str, ops_: Vec<ops::Op>) {
    let (model, _) = files::open(file).unwrap_or_else(|e| die(e));
    let mut s = ops::Session::new(model);
    match s.apply(actor, &ops_) {
        Ok(entry) => {
            let entry = entry.clone();
            files::save(&s.model, file, None).unwrap_or_else(|e| die(e));
            files::append_log(file, &entry).unwrap_or_else(|e| die(e));
            print(json!({"ok": true, "seq": entry.seq, "layers": layers::summary(&s.model)}));
        }
        Err(r) => {
            if let Some(e) = s.log.last() {
                let _ = files::append_log(file, e);
            }
            print(json!({"ok": false, "rejected": r}));
            std::process::exit(1);
        }
    }
}

fn op(v: serde_json::Value) -> ops::Op {
    serde_json::from_value(v).unwrap_or_else(|e| die(e))
}

fn num(a: &mut Vec<String>, flag: &str) -> Option<f64> {
    take_flag(a, flag).map(|s| s.trim().parse().unwrap_or_else(|_| die(format!("{flag} needs a number"))))
}

pub fn layers_cmd(mut args: Vec<String>) {
    let file = if args.is_empty() { die("a model file is needed") } else { args.remove(0) };
    let (m, _) = files::open(&file).unwrap_or_else(|e| die(e));
    print(layers::summary(&m));
}

pub fn run(mut args: Vec<String>) {
    if args.is_empty() {
        die("dyn layer add|set|assign|check|freeze|run|decouple FILE ...");
    }
    let sub = args.remove(0);
    let actor = take_flag(&mut args, "--actor").unwrap_or_else(|| "cli".into());
    match sub.as_str() {
        "add" => {
            let name = take_flag(&mut args, "--name").unwrap_or_else(|| die("--name is needed"));
            let mut o = json!({"op": "add_layer", "name": name});
            if let Some(k) = num(&mut args, "--order") {
                o["order"] = json!(k as i32);
            }
            if let Some(q) = take_flag(&mut args, "--question") {
                o["question"] = json!(q);
            }
            if let Some(h) = num(&mut args, "--horizon") {
                o["horizon"] = json!(h);
            }
            if let Some(h) = num(&mut args, "--limit") {
                o["structural_limit"] = json!(h);
            }
            if let Some(s) = take_flag(&mut args, "--status") {
                o["status"] = json!(s);
            }
            let file = if args.is_empty() { die("a model file is needed") } else { args.remove(0) };
            apply(&file, &actor, vec![op(o)]);
        }
        "set" => {
            let mut ops_ = Vec::new();
            for (flag, key) in [("--status", "status"), ("--horizon", "horizon"), ("--question", "question"), ("--order", "order"), ("--limit", "structural_limit"), ("--colour", "colour"), ("--description", "description"), ("--owner", "owner"), ("--rename", "name")] {
                while let Some(v) = take_flag(&mut args, flag) {
                    ops_.push((key, v));
                }
            }
            if args.len() < 2 {
                die("dyn layer set FILE NAME --status built | --horizon H | --question Q | --order K | --limit L");
            }
            if ops_.is_empty() {
                die("nothing to set (--status, --horizon, --question, --order, --limit, --colour, --description, --owner, --rename)");
            }
            let (file, name) = (args.remove(0), args.remove(0));
            apply(&file, &actor, ops_.into_iter().map(|(k, v)| op(json!({"op": "set_layer", "name": name, "key": k, "value": v}))).collect());
        }
        "assign" => {
            let vars: Vec<String> = take_flag(&mut args, "--vars").map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default();
            let sheet = take_flag(&mut args, "--sheet");
            let module = take_flag(&mut args, "--module");
            if args.len() < 2 {
                die("dyn layer assign FILE NAME --vars A,B | --sheet S | --module M");
            }
            let (file, name) = (args.remove(0), args.remove(0));
            apply(&file, &actor, vec![op(json!({"op": "assign_layer", "layer": name, "vars": vars, "sheet": sheet, "module": module}))]);
        }
        "freeze" => {
            let file = if args.is_empty() { die("a model file is needed") } else { args.remove(0) };
            apply(&file, &actor, vec![op(json!({"op": "freeze_interfaces"}))]);
        }
        "check" => {
            let file = if args.is_empty() { die("a model file is needed") } else { args.remove(0) };
            let (m, _) = files::open(&file).unwrap_or_else(|e| die(e));
            let r = layers::check(&m);
            let ok = r.errors() == 0;
            print(json!({"ok": ok, "errors": r.errors(), "crossings": r.crossings, "contracts": r.contracts, "cross_layer_loops": r.cross_layer_loops, "issues": r.issues}));
            if !ok {
                std::process::exit(1);
            }
        }
        "run" => {
            let inputs_csv = take_flag(&mut args, "--inputs");
            let stub = take_flag(&mut args, "--stub");
            let from_full = has_flag(&mut args, "--from-full-run");
            let csv = take_flag(&mut args, "--csv");
            if args.len() < 2 {
                die("dyn layer run FILE NAME [--inputs CSV | --stub S | --from-full-run] [--csv OUT]");
            }
            let (file, name) = (args.remove(0), args.remove(0));
            let (m, _) = files::open(&file).unwrap_or_else(|e| die(e));
            let layer = m.layer_by_name(&name).cloned().unwrap_or_else(|| die(format!("there is no layer called “{name}”")));
            let chosen = [inputs_csv.is_some(), stub.is_some(), from_full].iter().filter(|b| **b).count();
            if chosen > 1 {
                die("choose one of --inputs, --stub, --from-full-run");
            }
            let (inputs, source): (HashMap<String, Input>, String) = if let Some(p) = inputs_csv {
                let text = std::fs::read_to_string(&p).unwrap_or_else(|e| die(format!("{p}: {e}")));
                (layers::inputs_from_csv(&text).unwrap_or_else(|e| die(e)), format!("csv {p}"))
            } else if let Some(sn) = stub {
                let st = m.stubs.iter().find(|s| dynamist_core::expr::normalize(&s.name) == dynamist_core::expr::normalize(&sn)).unwrap_or_else(|| die(format!("there is no stub called “{sn}”")));
                (layers::inputs_from_stub(&m, layer.id, st).unwrap_or_else(|e| die(e)), format!("stub {sn}"))
            } else {
                let full = runner::run_with(&m, &[]).unwrap_or_else(|e| die(e));
                (layers::recorded_inputs(&m, layer.id, &full).unwrap_or_else(|e| die(e)), "the full run".to_string())
            };
            let run = layers::run_layer(&m, &layer, &inputs, &[]).unwrap_or_else(|e| die(e));
            let own = layers::behaviour_vars(&m, layer.id);
            if let Some(out) = csv {
                std::fs::write(&out, runner::to_csv(&run, &own)).unwrap_or_else(|e| die(e));
            }
            let mut v = runner::summary_json(&run, &own);
            v["layer"] = json!(layer.name);
            v["inputs_from"] = json!(source);
            print(v);
        }
        "decouple" => {
            let max = num(&mut args, "--max-nrmse").unwrap_or(0.05);
            if args.len() < 2 {
                die("dyn layer decouple FILE NAME [--max-nrmse 0.05]");
            }
            let (file, name) = (args.remove(0), args.remove(0));
            let (m, _) = files::open(&file).unwrap_or_else(|e| die(e));
            let layer = m.layer_by_name(&name).cloned().unwrap_or_else(|| die(format!("there is no layer called “{name}”")));
            let v = layers::decouple(&m, &layer, max).unwrap_or_else(|e| die(e));
            let ok = v["ok"].as_bool().unwrap_or(false);
            print(v);
            if !ok {
                std::process::exit(1);
            }
        }
        other => die(format!("unknown layer command “{other}” (add, set, assign, check, freeze, run, decouple)")),
    }
}
