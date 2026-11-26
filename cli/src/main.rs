//! `dyn`: the Dynamist model tool for scripts, CI and agents. Every command prints JSON.
//!
//!   dyn info FILE                      what the model contains
//!   dyn export FILE --xlsx OUT   an Excel workbook: Cover, Inputs, Scenarios, Series, Checks
//!   dyn serve FILE [--port 8080] [--bind 127.0.0.1] [--token T | --token-file F] [--log FILE]   the model as a web service (manifest, run, batch, OpenAPI)
//!   dyn attribute OLD NEW [--json] [--out PREFIX] [--permutations 200] [--seed 1]   why the goals changed: the change in every goal split between the edits (Shapley), parts adding up to the whole
//!   dyn hindcast FILE --data CSV --cut DATE [--var NAME] [--fit [--lever Name:min:max]…] [--runs 200] [--spread 0.2] [--seed 1] [--season S] [--json] [--out PREFIX]   blind hindcast: fit before the cut, score after it, coverage of the 50% and 90% bands
//!   dyn pack FILE --out DIR   the audit pack: doctor report and gate, sanity sheet, checks, conversion loss, operation log, workbook, model card, index
//!   dyn ablate FILE [--only NAME] [--json]   take one thing out at a time (the manifest's ablations, or each loop and each stock) and read the goals
//!   dyn check FILE [--json] [--publish] [--gate review|publish|deliver] [--units] [--lenient-time] [--fix ID[,ID]] [--fix-all-safe] [--out OUT]   the model doctor: findings (units, magic numbers, time step…) each with exact fixes
//!   dyn finance FILE --cash VAR [--quantity VAR] --rate R (per model time unit) [--timing period_end] [--finance-rate R --reinvest-rate R]   NPV, IRR, MIRR, paybacks, levelised cost
//!   dyn validate FILE [--strict-units]  equations, units, checks, balance (exit 1 on errors)
//!   dyn explain FILE                   the same issues with advice on how to fix each
//!   dyn test FILE                      only the model's own checks and conservation
//!   dyn run FILE [--seed N] [--set N=V]… [--series a,b] [--every N] [--csv OUT]
//!   dyn optimize FILE --goal Var [--measure peak] [--maximize] [--allow-beyond-horizon] --lever Name:min:max… [--limit "Var<=1000:final"]… [--effort 2] [--apply OUT]
//!   dyn sens FILE --var NAME [--runs 100] [--spread 0.2] [--seed 1]   Monte Carlo band of outcomes
//!   dyn screen FILE [--goal G]… [--param P]… [--trajectories 20] [--levels 4] [--seed 1] [--tau 0.1] [--csv OUT] [--design]   Morris screening: which levers matter for the manifest's goals
//!   dyn importance FILE [--goal G]… [--param P]… [--samples 512] [--seed 1] [--ensemble 30] [--eigen-at T] [--design]   Sobol indices, loop dominance, variables that cannot matter
//!   dyn dominance FILE [--csv OUT] [--method auto|exhaustive|published|bestfirst]      which loop dominates when (Loops That Matter), base run
//!   dyn reduce FILE --out OUT [--log CSV] [--goal G]… [--samples 512] [--trajectories 20] [--ensemble 30] [--batch 4] [--st-threshold 0.01]   a smaller linked model that keeps the goals (gates, then cut, verify, add back); exit 1 and no file when the gates refuse; the source is untouched
//!   dyn verify FULL REDUCED [--goal G]… [--ensemble 50] [--seed 1] [--max-nrmse 0.05]   does the reduced model still answer the same? (error, modes, ranking, checks)
//!   dyn recommend FILE --role flow=Orders --role arrival=Arrivals … [--structure supply_line]… [--goal G]… [--ensemble 20]   proposals for structure that would improve the goals (nothing is applied)
//!   dyn convert FILE OUT [--format sdm|xmile|mdl]   (prints the loss report)
//!   dyn diff A B [--runs]   dyn compare A B [--same-draws]              what differs in meaning (or in results)
//!   dyn list FILE [--kind stock]       get FILE NAME   deps FILE NAME   loops FILE
//!   dyn ops FILE BATCH.json [--actor NAME] [--reason TEXT] [--no-gate]   apply a batch atomically, save, log (rejections too)
//!   dyn manifest FILE [--draft]        the manifest (levers with ranges, outputs, scenarios…), or a draft of one
//!   dyn render FILE --out fig.svg [--sheet NAME] [--png [--scale 3]] [--with-caption] [--style FILE]   draw a sheet exactly as the app does (headless; --out x.png writes only a PNG)
//!   dyn chart FILE --vars A,B --out chart.svg [--from T0] [--to T1] [--kind line|area|step|stacked|bars] [--title T] [--scenarios A,B|all] [--set N=V]… [--png]   variables over time
//!   dyn layout FILE [--sheet NAME] [--mode tidy|loops] [--loops R1,B2] [--out OUT]   arrange a sheet; positions are written back to the .sdm
//!   dyn cld FILE --from-model [--loops R1,B2] --out cld.sdm [--layout keep|loops] | dyn cld FILE --list   a causal loop sheet that keeps only the named loops
//!   dyn set FILE [--sheet NAME] --pos VAR X Y | --polarity A B + | --delay A B [off] | --loop-name B1 "name" | --caption TEXT | --source TEXT
//!   dyn style FILE --apply book.style.json | --show   font, sizes, colours, arrow style and line widths for figures
//!   dyn layers FILE                    the stack of layers: status, horizon, counts, interfaces
//!   dyn layer add|set|assign|check|freeze|run|decouple FILE ...   build and test a model layer by layer (see the Layers wiki page)
//!   dyn library --ingest FOLDER --out DIR --source TEXT --licence TEXT [--json OUT]   import every model file in a folder, check, run, save as .sdm
//!   dyn library [--text W] [--domain D] [--skill S] [--feature F] [--tag T] [--author A] [--level L] [--year-from Y] [--max-simplicity N] [--facets] [--stats] [--check]   search and filter the model library
//!   dyn docs [--json]                  the operations language reference (Markdown, or JSON)
//!   dyn mcp                            serve the same operations to an AI agent over stdio

mod files;
mod hindcast;
mod layer;
mod library;
mod mcp;
mod pack;
mod render;
mod serve;
mod xlsx;

use dynamist_core::{diff, ops, runner, validate};
use serde_json::{Value, json};

fn die(msg: impl std::fmt::Display) -> ! {
    println!("{}", json!({"ok": false, "error": msg.to_string()}));
    std::process::exit(2);
}

fn take_flag(args: &mut Vec<String>, name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.remove(i);
    if i < args.len() { Some(args.remove(i)) } else { die(format!("{name} needs a value")) }
}

fn take_all(args: &mut Vec<String>, name: &str) -> Vec<String> {
    let mut v = Vec::new();
    while let Some(x) = take_flag(args, name) {
        v.push(x);
    }
    v
}

fn has_flag(args: &mut Vec<String>, name: &str) -> bool {
    match args.iter().position(|a| a == name) {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    }
}

fn print(v: Value) {
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("{}", include_str!("main.rs").lines().take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!").trim_start_matches(' ')).collect::<Vec<_>>().join("\n"));
        return;
    }
    let cmd = args.remove(0);
    if cmd == "library" {
        library::run(args);
        return;
    }
    if cmd == "mcp" {
        return mcp::serve();
    }
    if cmd == "docs" {
        if has_flag(&mut args, "--json") {
            print(dynamist_core::apidoc::json());
        } else {
            print!("{}", dynamist_core::apidoc::markdown());
        }
        return;
    }
    let path = |args: &mut Vec<String>| -> String { if args.is_empty() { die("a model file is needed") } else { args.remove(0) } };
    match cmd.as_str() {
        "info" => {
            let (m, loss) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let mut v = ops::info(&m);
            v["loss"] = json!(loss.items);
            print(v);
        }
        "manifest" => {
            let draft = has_flag(&mut args, "--draft");
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let mf = if draft { dynamist_core::manifest::draft(&m) } else { m.manifest.clone() };
            print(json!({"draft": draft, "manifest": mf, "problems": dynamist_core::manifest::check(&m).into_iter().map(|x| x.1).collect::<Vec<_>>()}));
        }
        "explain" => {
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let r = validate::validate(&m);
            print(json!({"ok": r.ok(), "issues": validate::explain(&r)}));
        }
        "check" => {
            // the model doctor: findings with exact fixes
            let json_out = has_flag(&mut args, "--json");
            let publish = has_flag(&mut args, "--publish");
            let lenient_time = has_flag(&mut args, "--lenient-time");
            let fix_all_safe = has_flag(&mut args, "--fix-all-safe");
            let units_only = has_flag(&mut args, "--units");
            let gate = take_flag(&mut args, "--gate");
            let stage = take_flag(&mut args, "--stage").map(|s| dynamist_core::doctor::Stage::parse(&s).unwrap_or_else(|| die(format!("--stage is C1 to C7, not “{s}”"))));
            let experiments = has_flag(&mut args, "--experiments");
            let jobs: usize = take_flag(&mut args, "--jobs").and_then(|s| s.parse().ok()).unwrap_or(4);
            let fix_ids: Vec<usize> = take_flag(&mut args, "--fix").map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
            let out_path = take_flag(&mut args, "--out");
            let file = path(&mut args);
            let (mut m, _) = files::open(&file).unwrap_or_else(|e| die(e));
            // --gate publish runs everything (the experiments included) and grades the model by Part C7; --publish alone also counts warnings as errors
            let gate_publish = gate.as_deref() == Some("publish");
            let opts = dynamist_core::doctor::Options { publish, lenient_time, units_only, stage: stage.or(if gate_publish { Some(dynamist_core::doctor::Stage::C7) } else { None }), experiments: experiments || gate_publish, jobs };
            let report = dynamist_core::doctor::check(&m, &opts);
            let mut applied = 0;
            if !fix_ids.is_empty() || fix_all_safe {
                if fix_all_safe {
                    let (m2, n) = dynamist_core::doctor::apply_safe_fixes(&m, &report);
                    m = m2;
                    applied = n;
                }
                for id in &fix_ids {
                    let Some(f) = report.findings.get(id.wrapping_sub(1)) else { die(format!("there is no finding {id} (there are {})", report.findings.len())) };
                    let Some(fix) = f.fixes.first() else { die(format!("finding {id} ({}) has no fix", f.code)) };
                    dynamist_core::doctor::apply_fix(&mut m, fix).unwrap_or_else(|e| die(format!("fix {id}: {e}")));
                    applied += 1;
                }
                files::save(&m, out_path.as_deref().unwrap_or(&file), None).unwrap_or_else(|e| die(e));
            }
            let findings: Vec<Value> = report.findings.iter().enumerate().map(|(i, f)| { let mut v = serde_json::to_value(f).unwrap(); v["id"] = json!(i + 1); v }).collect();
            let summary = report.line(applied);
            // --gate deliver: the sanity sheet of a model that is handed to someone else (seven rows by program, five by a person)
            let deliver = gate.as_deref() == Some("deliver");
            let sheet = if deliver { Some(dynamist_core::sanity::sheet(&m, jobs)) } else { None };
            if json_out {
                let mut out = json!({"summary": summary, "gate": report.gate, "model": report.summary, "findings": findings});
                if let Some(rows) = &sheet {
                    out["sanity"] = json!({"delivered": dynamist_core::sanity::delivered(rows), "rows": rows});
                }
                print(out);
            } else {
                for (i, f) in report.findings.iter().enumerate() {
                    // a finding no run has confirmed is a possibility, not an error
                    let word = if f.confirmed { format!("{:?}", f.severity) } else { "possible".to_string() };
                    println!("{:>3}. [{}] {} {}: {}", i + 1, word, f.code, f.var.as_deref().unwrap_or("-"), f.message);
                    for (k, fix) in f.fixes.iter().enumerate() {
                        println!("       fix {}{}: {}{}", i + 1, (b'a' + k as u8) as char, fix.title, if fix.keeps_numbers { "  (keeps the numbers)" } else { "" });
                    }
                }
                let s = &report.summary;
                println!("{} stocks, {} flows, {} auxiliaries, {} constants, {} lookups, {} loops; units on {:.0}% and described {:.0}% of the variables", s.stocks, s.flows, s.auxiliaries, s.constants, s.lookups, s.loops, s.units_coverage_pct, s.documentation_coverage_pct);
                println!("{summary}; gate: {:?}", report.gate.unwrap());
                if let Some(rows) = &sheet {
                    println!("\n{}", dynamist_core::sanity::markdown(&m, rows));
                }
                if opts.stage.is_none() && !opts.experiments && !opts.publish {
                    println!("(stages C1 to C4 ran; add --experiments for the automatic experiments of stage C5)");
                }
            }
            let want = gate.as_deref();
            // the exit code follows the gate: a confirmed error (gate Draft), or, with --gate publish, anything short of Publish. A "can" finding that no run confirmed does not stop a build.
            let confirmed_errors = report.findings.iter().any(|f| f.severity == dynamist_core::doctor::Severity::Error && f.confirmed);
            let undelivered = sheet.as_ref().is_some_and(|r| !dynamist_core::sanity::delivered(r));
            if confirmed_errors || undelivered || (want == Some("publish") && report.gate != Some(dynamist_core::doctor::Gate::Publish)) {
                std::process::exit(1);
            }
        }
        "finance" => {
            // dyn finance FILE --cash VAR [--quantity VAR] --rate R (per model time unit) [--timing period_end] [--finance-rate R --reinvest-rate R]
            let cash_name = take_flag(&mut args, "--cash").unwrap_or_else(|| die("--cash VAR is needed: the cash flow stream"));
            let qty_name = take_flag(&mut args, "--quantity");
            let rate: f64 = take_flag(&mut args, "--rate").and_then(|s| s.parse().ok()).unwrap_or_else(|| die("--rate R is needed: the discount rate per model time unit, a number"));
            let timing = take_flag(&mut args, "--timing").unwrap_or_else(|| "period_end".into());
            let fin: Option<f64> = take_flag(&mut args, "--finance-rate").and_then(|s| s.parse().ok());
            let reinv: Option<f64> = take_flag(&mut args, "--reinvest-rate").and_then(|s| s.parse().ok());
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let run = runner::run_with(&m, &[]).unwrap_or_else(|e| die(e));
            let dt = m.settings.time_step;
            let off = match timing.as_str() { "period_start" => 0.0, "mid_period" => 0.5, "period_end" => 1.0, o => die(format!("--timing is period_start, mid_period or period_end, not “{o}”")) };
            let series = |n: &str| -> Vec<f64> { run.get(n).cloned().unwrap_or_else(|| die(format!("there is no variable “{n}” in the results"))).iter().map(|x| x * dt).collect() };
            let cash = series(&cash_name);
            let pv = |x: &[f64], r: f64| -> f64 { dynamist_core::finance::stream_pv(x, r, dt, off) };
            let irr = dynamist_core::finance::stream_irr(&cash, dt, off);
            let tu = m.settings.time_units.trim().to_string();
            let payback = |disc: Option<f64>| -> Option<f64> {
                let mut cum = 0.0;
                for (k, v) in cash.iter().enumerate() {
                    let c = match disc { Some(r) => v / (1.0 + r).powf((k as f64 + 1.0) * dt), None => *v };
                    if cum < 0.0 && cum + c >= 0.0 && c > 0.0 {
                        return Some(run.times[0] + (k as f64 + (-cum) / c) * dt);
                    }
                    cum += c;
                }
                None
            };
            let mut out = json!({
                "npv": {"value": pv(&cash, rate), "convention": format!("rate {rate} per {tu} (the model time unit, whatever the step of {dt}), timing {timing}, each step's cash counted as flow x step; the flow of step k is discounted by (1 + rate)^((k + offset) x step) with offset 0 for period_start, 0.5 for mid_period, 1 for period_end. period_end therefore discounts the time-0 flow by one period; period_start leaves it as it is and matches Excel's NPV(rate, flows after the first) + the first flow")},
                "irr": {"value": irr, "convention": format!("the rate per {tu} that makes the NPV zero, timing {timing}; the rates from -99% to 10 000 000% are scanned for a change of sign and the root nearest to zero is refined; null if there is none")},
                "irr_continuous": {"value": irr.map(|r| (1.0 + r).ln()), "convention": format!("ln(1 + irr): the same rate as a continuous rate per {tu} (the form of the book's chapter 11), null if irr is")},
                "payback_simple": {"value": payback(None), "convention": "cumulative undiscounted cash, interpolated within the step; null if it never turns positive"},
                "payback_discounted": {"value": payback(Some(rate)), "convention": "cumulative cash discounted at the rate, period end, interpolated within the step"},
            });
            if let (Some(f), Some(rr)) = (fin, reinv) {
                out["mirr"] = json!({"value": dynamist_core::finance::stream_mirr(&cash, f, rr, dt), "convention": format!("Excel's MIRR: positives compounded at {rr} per {tu}, negatives discounted at {f} per {tu}, over {} steps of {dt}; the result is per {tu}", cash.len() - 1)});
            }
            if let Some(q) = qty_name {
                let qty = series(&q);
                let d = pv(&qty, rate);
                out["levelised_cost"] = json!({"value": if d.abs() > 1e-300 { Some(pv(&cash, rate) / d) } else { None }, "convention": format!("PV of the cash stream over PV of the quantity stream at {rate} per {tu}, timing {timing}")});
            }
            print(out);
        }
        "validate" | "test" => {
            let strict_units = has_flag(&mut args, "--strict-units");
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let mut r = validate::validate_with(&m, &validate::Options { strict_units });
            if cmd == "test" {
                r.issues.retain(|i| i.code == "check" || i.code == "conservation" || i.code == "equation" || i.code == "run");
            }
            let ok = r.ok();
            print(json!({"ok": ok, "ran": r.ran, "errors": r.errors().count(), "issues": r.issues}));
            if !ok {
                std::process::exit(1);
            }
        }
        "run" => {
            let sets: Vec<(String, String)> = take_all(&mut args, "--set").into_iter().map(|s| s.split_once('=').map(|(a, b)| (a.trim().to_string(), b.trim().to_string())).unwrap_or_else(|| die(format!("--set wants NAME=VALUE, not “{s}”")))).collect();
            let series: Vec<String> = take_flag(&mut args, "--series").map(|s| s.split(',').map(|x| x.trim().to_string()).collect()).unwrap_or_default();
            let every: usize = take_flag(&mut args, "--every").and_then(|s| s.parse().ok()).unwrap_or(0);
            let csv = take_flag(&mut args, "--csv");
            let seed = take_flag(&mut args, "--seed").map(|s| s.parse::<u64>().unwrap_or_else(|_| die(format!("--seed wants a whole number, not “{s}”"))));
            let (mut m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            if let Some(sd) = seed {
                m.settings.seed = sd;
            }
            let run = runner::run_with(&m, &sets).unwrap_or_else(|e| die(e));
            if let Some(out) = csv {
                std::fs::write(&out, runner::to_csv(&run, &series)).unwrap_or_else(|e| die(e));
            }
            let mut out = if every > 0 { runner::series_json(&run, &series, every) } else { runner::summary_json(&run, &series) };
            let beyond = dynamist_core::layers::horizon_report(&m);
            if !beyond.is_empty() {
                out["beyond_horizon"] = json!(beyond);
            }
            out["time_convention"] = json!(match m.settings.time_convention { dynamist_core::model::TimeConvention::Abstract => "abstract", dynamist_core::model::TimeConvention::Calendar => "calendar" });
            out["year_days"] = json!(m.settings.year_days_abstract);
            if let Some(st) = &m.settings.start {
                out["start"] = json!(st);
            }
            let outside = dynamist_core::manifest::range_warnings(&m, &sets);
            if !outside.is_empty() {
                out["outside_declared_range"] = json!(outside);
            }
            print(out);
        }
        "optimize" => {
            use dynamist_core::model::{Lever, OptSpec};
            let goal = take_flag(&mut args, "--goal").unwrap_or_else(|| die("--goal VARIABLE is needed"));
            let measure = dynamist_core::optimize::parse_measure(&take_flag(&mut args, "--measure").unwrap_or_else(|| "final".into())).unwrap_or_else(|e| die(e));
            let maximize = has_flag(&mut args, "--maximize");
            let allow_beyond = has_flag(&mut args, "--allow-beyond-horizon");
            let effort: u8 = take_flag(&mut args, "--effort").and_then(|s| s.parse().ok()).unwrap_or(2);
            let apply = take_flag(&mut args, "--apply");
            let levers: Vec<Lever> = take_all(&mut args, "--lever")
                .into_iter()
                .map(|l| {
                    let p: Vec<&str> = l.rsplitn(3, ':').collect(); // max, min, name (names may contain ':')
                    if p.len() != 3 {
                        die(format!("--lever wants Name:min:max, not “{l}”"));
                    }
                    let (max, min) = (p[0].trim().parse::<f64>(), p[1].trim().parse::<f64>());
                    match (min, max) {
                        (Ok(min), Ok(max)) if min < max => Lever { name: p[2].trim().to_string(), on: true, min, max },
                        _ => die(format!("--lever “{l}” needs numbers with min below max")),
                    }
                })
                .collect();
            if levers.is_empty() {
                die("at least one --lever Name:min:max is needed");
            }
            let limits: Vec<_> = take_all(&mut args, "--limit").iter().map(|l| dynamist_core::optimize::parse_limit(l).unwrap_or_else(|e| die(e))).collect();
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            if !allow_beyond {
                if let Some(why) = dynamist_core::layers::goal_beyond_horizon(&m, &goal, None) {
                    die(why);
                }
            }
            let spec = OptSpec { maximize, goal, measure, levers, limits, effort };
            let p = dynamist_core::optimize::run_blocking(m.clone(), spec.clone()).unwrap_or_else(|e| die(e));
            let mut out = dynamist_core::optimize::result_json(&spec, &p);
            if let Some(file) = apply {
                let mut best = m;
                for (n, v) in p.names.iter().zip(&p.best_x) {
                    best.apply_change(n, &format!("{v}"));
                }
                files::save(&best, &file, None).unwrap_or_else(|e| die(e));
                out["applied_to"] = json!(file);
            }
            print(out);
        }
        "sens" => {
            let var = take_flag(&mut args, "--var").unwrap_or_else(|| die("--var NAME is needed"));
            let runs: usize = take_flag(&mut args, "--runs").and_then(|s| s.parse().ok()).unwrap_or(100);
            let spread: f64 = take_flag(&mut args, "--spread").and_then(|s| s.parse().ok()).unwrap_or(0.2);
            let seed: u64 = take_flag(&mut args, "--seed").and_then(|s| s.parse().ok()).unwrap_or(1);
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            print(runner::sensitivity_json(&m, &var, runs, spread, seed).unwrap_or_else(|e| die(e)));
        }
        "screen" => {
            let (goals, params) = (take_all(&mut args, "--goal"), take_all(&mut args, "--param"));
            let num = |a: &mut Vec<String>, f: &str, d: f64| take_flag(a, f).map_or(d, |s| s.parse().unwrap_or_else(|_| die(format!("{f} needs a number"))));
            let spec = dynamist_core::screen::ScreenSpec {
                trajectories: num(&mut args, "--trajectories", 20.0) as usize,
                levels: num(&mut args, "--levels", 4.0) as usize,
                seed: num(&mut args, "--seed", 1.0) as u64,
                tau: num(&mut args, "--tau", 0.1),
                linear_cut: num(&mut args, "--linear-cut", 0.1),
                converge_tau: num(&mut args, "--converge-tau", 0.9),
                workers: num(&mut args, "--workers", 4.0) as usize,
                keep_design: has_flag(&mut args, "--design"),
            };
            let csv = take_flag(&mut args, "--csv");
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let (p, g) = (dynamist_core::screen::params_of(&m, &params).unwrap_or_else(|e| die(e)), dynamist_core::screen::goals_of(&m, &goals).unwrap_or_else(|e| die(e)));
            let s = dynamist_core::screen::screen(&m, &p, &g, &spec).unwrap_or_else(|e| die(e));
            if let Some(f) = csv {
                std::fs::write(&f, dynamist_core::screen::to_csv(&s)).unwrap_or_else(|e| die(format!("{f}: {e}")));
            }
            print(serde_json::to_value(&s).unwrap());
        }
        "importance" => {
            let (goals, params) = (take_all(&mut args, "--goal"), take_all(&mut args, "--param"));
            let num = |a: &mut Vec<String>, f: &str, d: f64| take_flag(a, f).map_or(d, |s| s.parse().unwrap_or_else(|_| die(format!("{f} needs a number"))));
            let spec = dynamist_core::importance::ImportanceSpec {
                sobol: dynamist_core::sobol::SobolSpec {
                    samples: num(&mut args, "--samples", 512.0) as usize,
                    seed: num(&mut args, "--seed", 1.0) as u64,
                    bootstrap: num(&mut args, "--bootstrap", 100.0) as usize,
                    converge_tol: num(&mut args, "--converge-tol", 0.05),
                    workers: num(&mut args, "--workers", 4.0) as usize,
                    keep_design: has_flag(&mut args, "--design"),
                },
                ensemble: num(&mut args, "--ensemble", 30.0) as usize,
                eigen_at: take_flag(&mut args, "--eigen-at").map(|s| s.parse().unwrap_or_else(|_| die("--eigen-at needs a number"))),
            };
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let (p, g) = (dynamist_core::screen::params_of(&m, &params).unwrap_or_else(|e| die(e)), dynamist_core::screen::goals_of(&m, &goals).unwrap_or_else(|e| die(e)));
            print(dynamist_core::importance::importance_json(&m, &p, &g, &spec).unwrap_or_else(|e| die(e)));
        }
        "dominance" => {
            let csv = take_flag(&mut args, "--csv");
            let method = take_flag(&mut args, "--method").unwrap_or_else(|| "auto".into());
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            if let Some(f) = csv {
                let s = dynamist_core::loopscore::loop_scores(&m).unwrap_or_else(|e| die(e));
                let mut o = String::from("time");
                for l in &s.loops { o += &format!(",loop{}", l.id); }
                o.push('\n');
                for t in 0..s.times.len() {
                    o += &s.times[t].to_string();
                    for i in 0..s.loops.len() { o += &format!(",{}", s.relative[i][t]); }
                    o.push('\n');
                }
                std::fs::write(&f, o).unwrap_or_else(|e| die(format!("{f}: {e}")));
            }
            print(dynamist_core::importance::dominance_json_with(&m, &method).unwrap_or_else(|e| die(e)));
        }
        "reduce" => {
            let goals = take_all(&mut args, "--goal");
            let num = |a: &mut Vec<String>, f: &str, d: f64| take_flag(a, f).map_or(d, |s| s.parse().unwrap_or_else(|_| die(format!("{f} needs a number"))));
            let out = take_flag(&mut args, "--out").unwrap_or_else(|| die("--out FILE is needed: the reduced model is a new file"));
            let log = take_flag(&mut args, "--log");
            let seed = num(&mut args, "--seed", 1.0) as u64;
            let workers = num(&mut args, "--workers", 4.0) as usize;
            let mut rspec = dynamist_core::reduce::ReduceSpec { st_threshold: num(&mut args, "--st-threshold", 0.01), loop_threshold: num(&mut args, "--loop-threshold", 0.01), batch: num(&mut args, "--batch", 4.0) as usize, morris_tau: num(&mut args, "--morris-tau", 0.9), ..Default::default() };
            rspec.sobol = dynamist_core::sobol::SobolSpec { samples: num(&mut args, "--samples", 512.0) as usize, seed, workers, ..Default::default() };
            rspec.screen = dynamist_core::screen::ScreenSpec { trajectories: num(&mut args, "--trajectories", 20.0) as usize, seed, workers, ..Default::default() };
            rspec.verify = dynamist_core::verify::VerifySpec { ensemble: num(&mut args, "--ensemble", 30.0) as usize, seed, ..Default::default() };
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let g = dynamist_core::screen::goals_of(&m, &goals).unwrap_or_else(|e| die(e));
            let r = dynamist_core::reduce::reduce(&m, &g, &rspec).unwrap_or_else(|e| die(e));
            let body = json!({
                "ok": !r.refused(),
                "refused": r.refused,
                "out": if r.refused() { serde_json::Value::Null } else { json!(out) },
                "headline": r.headline,
                "structure_removed": {"summary": r.structure_removed, "changes": r.structure_changes},
                "parameters_fixed": {"scenarios_tested": r.scenarios_tested, "changes": r.parameter_changes},
                "readmitted": r.readmitted,
                "cuts_tried": r.cuts_tried,
                "loop_proposals": r.loop_proposals,
                "verifications": r.verifications,
                "verification_runs": r.verification_runs,
                "verdict": r.verdict,
                "before": r.before,
                "after": r.after,
                "gates": r.gates,
                "sobol": r.sobol,
                "changes": r.changes,
            });
            if let Some(f) = log {
                std::fs::write(&f, dynamist_core::reduce::changes_csv(&r.changes)).unwrap_or_else(|e| die(format!("{f}: {e}")));
            }
            if r.refused() {
                print(body);
                std::process::exit(1);
            }
            files::save(&r.model, &out, None).unwrap_or_else(|e| die(e));
            print(body);
        }
        "verify" => {
            let goals = take_all(&mut args, "--goal");
            let num = |a: &mut Vec<String>, f: &str, d: f64| take_flag(a, f).map_or(d, |s| s.parse().unwrap_or_else(|_| die(format!("{f} needs a number"))));
            let spec = dynamist_core::verify::VerifySpec { ensemble: num(&mut args, "--ensemble", 50.0) as usize, seed: num(&mut args, "--seed", 1.0) as u64, max_nrmse: num(&mut args, "--max-nrmse", 0.05), min_mode_agreement: num(&mut args, "--min-mode-agreement", 0.9), min_goal_tau: num(&mut args, "--min-goal-tau", 0.9) };
            let (full, reduced) = (files::open(&path(&mut args)).unwrap_or_else(|e| die(e)).0, files::open(&path(&mut args)).unwrap_or_else(|e| die(e)).0);
            let g = dynamist_core::screen::goals_of(&full, &goals).unwrap_or_else(|e| die(e));
            let v = dynamist_core::verify::verify(&full, &reduced, &g, &spec).unwrap_or_else(|e| die(e));
            let pass = v.pass;
            print(serde_json::to_value(&v).unwrap());
            if !pass {
                std::process::exit(1);
            }
        }
        "recommend" => {
            let goals = take_all(&mut args, "--goal");
            let allow_beyond = has_flag(&mut args, "--allow-beyond-horizon");
            let only = take_all(&mut args, "--structure");
            let roles: std::collections::HashMap<String, String> = take_all(&mut args, "--role").into_iter().map(|s| s.split_once('=').map(|(a, b)| (a.trim().to_string(), b.trim().to_string())).unwrap_or_else(|| die(format!("--role wants key=value, not “{s}”")))).collect();
            let num = |a: &mut Vec<String>, f: &str, d: f64| take_flag(a, f).map_or(d, |s| s.parse().unwrap_or_else(|_| die(format!("{f} needs a number"))));
            let spec = dynamist_core::recommend::RecSpec { grid: num(&mut args, "--grid", 17.0) as usize, ensemble: num(&mut args, "--ensemble", 20.0) as usize, seed: num(&mut args, "--seed", 1.0) as u64 };
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let g = dynamist_core::screen::goals_of(&m, &goals).unwrap_or_else(|e| die(e));
            if !allow_beyond {
                for x in &g {
                    if let Some(why) = dynamist_core::layers::goal_beyond_horizon(&m, &x.output, dynamist_core::layers::measure_end(&x.measure)) {
                        die(why);
                    }
                }
            }
            let r = dynamist_core::recommend::recommend(&m, &g, &roles, &only, &spec).unwrap_or_else(|e| die(e));
            print(serde_json::to_value(&r).unwrap());
        }
        "convert" => {
            let format = take_flag(&mut args, "--format");
            let (input, output) = (path(&mut args), path(&mut args));
            let (m, mut loss) = files::open(&input).unwrap_or_else(|e| die(e));
            let more = files::save(&m, &output, format.as_deref()).unwrap_or_else(|e| die(e));
            loss.items.extend(more.items);
            print(json!({"ok": true, "lossless": loss.is_lossless(), "loss": loss.items}));
        }
        "diff" | "compare" => {
            let same_draws = has_flag(&mut args, "--same-draws");
            let runs = has_flag(&mut args, "--runs") || same_draws || cmd == "compare";
            let (a, mut b) = (files::open(&path(&mut args)).unwrap_or_else(|e| die(e)).0, files::open(&path(&mut args)).unwrap_or_else(|e| die(e)).0);
            if same_draws {
                // identical random streams: both models draw with the first one's seed
                b.settings.seed = a.settings.seed;
            }
            if runs {
                let (x, y) = (runner::run_with(&a, &[]).unwrap_or_else(|e| die(e)), runner::run_with(&b, &[]).unwrap_or_else(|e| die(e)));
                let d = diff::diff_runs(&x, &y, 1e-9);
                print(json!({"same": d.is_same(), "diff": d}));
            } else {
                let d = diff::diff_models(&a, &b);
                print(json!({"same": d.is_empty(), "differences": d}));
            }
        }
        "list" => {
            let kind = take_flag(&mut args, "--kind");
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            print(ops::list(&m, kind.as_deref()).unwrap_or_else(|e| die(e)));
        }
        "get" | "deps" => {
            let (f, name) = (path(&mut args), path(&mut args));
            let (m, _) = files::open(&f).unwrap_or_else(|e| die(e));
            print(if cmd == "get" { ops::get(&m, &name) } else { ops::dependencies(&m, &name) }.unwrap_or_else(|e| die(e)));
        }
        "export" => {
            // dyn export FILE --xlsx OUT: Cover, Inputs, Scenarios, Series and Checks as an Excel workbook (values from the engine)
            let out = take_flag(&mut args, "--xlsx").unwrap_or_else(|| die("dyn export needs --xlsx OUT"));
            let file = path(&mut args);
            let (m, _) = files::open(&file).unwrap_or_else(|e| die(e));
            let name = std::path::Path::new(&file).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(file.clone());
            let sheets = dynamist_core::workbook::build(&m, &name);
            std::fs::write(&out, xlsx::workbook(&sheets)).unwrap_or_else(|e| die(format!("{out}: {e}")));
            print(json!({"written": out, "sheets": sheets.iter().map(|s| json!({"name": s.name, "rows": s.rows.len()})).collect::<Vec<_>>()}));
        }
        "serve" => {
            // dyn serve FILE [--port 8080] [--bind 127.0.0.1] [--token T | --token-file F] [--log FILE]: the model as a web service
            let port: u16 = take_flag(&mut args, "--port").map(|p| p.parse().unwrap_or_else(|_| die("--port is a number"))).unwrap_or(8080);
            let bind = take_flag(&mut args, "--bind").unwrap_or_else(|| "127.0.0.1".into());
            let token_file = take_flag(&mut args, "--token-file");
            let token = take_flag(&mut args, "--token").or_else(|| token_file.map(|f| std::fs::read_to_string(&f).unwrap_or_else(|e| die(format!("{f}: {e}"))).trim().to_string()));
            let log = take_flag(&mut args, "--log");
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let generated = token.is_none();
            let token = token.unwrap_or_else(serve::random_token);
            let version = serve::version_of(&m);
            let srv = serve::start(m, serve::Config { bind: bind.clone(), port, token: token.clone(), log }).unwrap_or_else(|e| die(e));
            // the address, the version and (when it was made up) the token go to standard output once
            print(json!({"listening": srv.addr.to_string(), "model_version": version, "token": if generated { json!(token) } else { json!("as given") }, "openapi": format!("http://{}/openapi.json", srv.addr)}));
            if bind != "127.0.0.1" && bind != "localhost" {
                eprintln!("note: listening on {bind}, not only on this computer; the token is the only protection and the traffic is not encrypted");
            }
            srv.wait();
        }
        "attribute" => {
            // dyn attribute OLD NEW: split the change in every goal between the groups of edits (Shapley value)
            let json_out = has_flag(&mut args, "--json");
            let out = take_flag(&mut args, "--out");
            let perms: usize = take_flag(&mut args, "--permutations").map_or(200, |s| s.parse().unwrap_or_else(|_| die("--permutations needs a whole number")));
            let seed: u64 = take_flag(&mut args, "--seed").map_or(1, |s| s.parse().unwrap_or_else(|_| die("--seed needs a whole number")));
            let (fa, fb) = (path(&mut args), path(&mut args));
            let (a, b) = (files::open(&fa).unwrap_or_else(|e| die(e)).0, files::open(&fb).unwrap_or_else(|e| die(e)).0);
            let r = dynamist_core::attribute::run(&a, &b, perms, seed, (&fa, &fb)).unwrap_or_else(|e| die(e));
            let md = dynamist_core::attribute::markdown(&r);
            if let Some(p) = out {
                let w = |f: String, t: String| std::fs::write(&f, t).unwrap_or_else(|e| die(format!("{f}: {e}")));
                w(format!("{p}.md"), md.clone());
                w(format!("{p}.json"), serde_json::to_string_pretty(&r).unwrap());
                for (i, g) in r.goals.iter().enumerate() {
                    w(format!("{p}-waterfall-{}.svg", i + 1), dynamist_core::attribute::waterfall_svg(&r, g));
                }
            }
            if json_out {
                print(json!(r));
            } else {
                print!("{md}");
            }
        }
        "hindcast" => {
            let data = take_flag(&mut args, "--data").unwrap_or_else(|| die("--data CSV is needed (two columns: time and value)"));
            let cut = take_flag(&mut args, "--cut").unwrap_or_else(|| die("--cut DATE is needed (a date like 2026-01-31, or a number of model time)"));
            let fit = has_flag(&mut args, "--fit");
            let json_out = has_flag(&mut args, "--json");
            let out = take_flag(&mut args, "--out");
            let num = |a: &mut Vec<String>, f: &str, d: f64| take_flag(a, f).map_or(d, |s| s.parse().unwrap_or_else(|_| die(format!("{f} needs a number"))));
            let levers: Vec<dynamist_core::model::Lever> = take_all(&mut args, "--lever")
                .into_iter()
                .map(|l| {
                    let p: Vec<&str> = l.rsplitn(3, ':').collect();
                    match (p.len(), p.first().and_then(|x| x.trim().parse::<f64>().ok()), p.get(1).and_then(|x| x.trim().parse::<f64>().ok())) {
                        (3, Some(max), Some(min)) if min < max => dynamist_core::model::Lever { name: p[2].trim().to_string(), on: true, min, max },
                        _ => die(format!("--lever wants Name:min:max with min below max, not “{l}”")),
                    }
                })
                .collect();
            let spec = hindcast::Spec { var: take_flag(&mut args, "--var"), cut, fit, levers, runs: num(&mut args, "--runs", 200.0) as usize, spread: num(&mut args, "--spread", 0.2), seed: num(&mut args, "--seed", 1.0) as u64, season: num(&mut args, "--season", 0.0), effort: num(&mut args, "--effort", 2.0) as u8 };
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let text = std::fs::read_to_string(&data).unwrap_or_else(|e| die(format!("{data}: {e}")));
            let (r, svg) = hindcast::run(&m, &text, &spec).unwrap_or_else(|e| die(e));
            if let Some(p) = out {
                let w = |f: String, t: String| std::fs::write(&f, t).unwrap_or_else(|e| die(format!("{f}: {e}")));
                w(format!("{p}.md"), hindcast::markdown(&r));
                w(format!("{p}.json"), serde_json::to_string_pretty(&r).unwrap());
                w(format!("{p}-fan.svg"), svg);
            }
            if json_out {
                print(json!(r));
            } else {
                print!("{}", hindcast::markdown(&r));
            }
        }
        "pack" => {
            let out = take_flag(&mut args, "--out").unwrap_or_else(|| die("--out DIR is needed"));
            let file = path(&mut args);
            let failed = pack::run(&file, &out).unwrap_or_else(|e| die(e));
            print(json!({"ok": failed.is_empty(), "out": out, "failed": failed}));
            if !failed.is_empty() {
                std::process::exit(1);
            }
        }
        "ablate" => {
            // dyn ablate FILE [--only NAME] [--json]: take out one thing at a time and read the goals
            let json_out = has_flag(&mut args, "--json");
            let only = take_flag(&mut args, "--only");
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            let t = dynamist_core::ablate::run(&m, only.as_deref()).unwrap_or_else(|e| die(e));
            if json_out {
                print(json!(t));
            } else {
                println!("{}", dynamist_core::ablate::markdown(&t));
            }
        }
        "loops" => {
            let (m, _) = files::open(&path(&mut args)).unwrap_or_else(|e| die(e));
            print(ops::loops(&m));
        }
        "ops" => {
            let actor = take_flag(&mut args, "--actor").unwrap_or_else(|| "cli".into());
            let no_gate = has_flag(&mut args, "--no-gate");
            let reason = take_flag(&mut args, "--reason");
            let (file, batch) = (path(&mut args), path(&mut args));
            let (model, _) = if std::path::Path::new(&file).exists() { files::open(&file).unwrap_or_else(|e| die(e)) } else { (Default::default(), Default::default()) };
            let text = std::fs::read_to_string(&batch).unwrap_or_else(|e| die(format!("{batch}: {e}")));
            let batch: Vec<ops::Op> = serde_json::from_str(&text).unwrap_or_else(|e| die(format!("the batch is not a list of operations: {e}")));
            let mut s = ops::Session::new(model);
            s.gate = !no_gate;
            match s.apply_with_reason(&actor, reason.as_deref(), &batch) {
                Ok(entry) => {
                    let entry = entry.clone();
                    files::save(&s.model, &file, None).unwrap_or_else(|e| die(e));
                    files::append_log(&file, &entry).unwrap_or_else(|e| die(e));
                    print(json!({"ok": true, "seq": entry.seq, "ops": entry.ops.len(), "warnings": entry.warnings}));
                }
                Err(r) => {
                    if let Some(e) = s.log.last() {
                        let _ = files::append_log(&file, e);
                    }
                    print(json!({"ok": false, "rejected": r}));
                    std::process::exit(1);
                }
            }
        }
        "layers" => layer::layers_cmd(args),
        "layer" => layer::run(args),
        "render" | "chart" | "layout" | "cld" | "set" | "style" => render::run(&cmd, args),
        other => die(format!("unknown command “{other}” (try dyn --help)")),
    }
}
