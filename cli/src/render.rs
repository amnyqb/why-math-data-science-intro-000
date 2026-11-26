//! The figure commands of `dyn`: render, layout, cld, set, style, chart. Each prints JSON like the others.
//! The work is in `dynamist_core::figure` / `figstyle` (editing) and `dynamist::headless` (drawing).

use std::path::Path;

use dynamist::headless::{ChartSpec, Figure, Headless};
use dynamist_core::figstyle::FigStyle;
use dynamist_core::figure;
use dynamist_core::model::{ChartKind, Model, SheetKind};
use serde_json::{Value, json};

use super::{die, files, has_flag, print, take_all, take_flag};

pub fn run(cmd: &str, mut args: Vec<String>) {
    match cmd {
        "render" => render(&mut args),
        "chart" => chart(&mut args),
        "layout" => layout(&mut args),
        "cld" => cld(&mut args),
        "set" => set(&mut args),
        "style" => style(&mut args),
        _ => die(format!("unknown figure command “{cmd}”")),
    }
}

fn file_arg(args: &mut Vec<String>) -> String {
    match args.iter().position(|a| !a.starts_with("--")) {
        Some(i) => args.remove(i),
        None => die("a model file is needed"),
    }
}

fn no_extra(args: &[String]) {
    if let Some(a) = args.first() {
        die(format!("unexpected “{a}” (see dyn --help)"));
    }
}

fn num(args: &mut Vec<String>, flag: &str) -> Option<f64> {
    take_flag(args, flag).map(|s| s.parse().unwrap_or_else(|_| die(format!("{flag} needs a number, not “{s}”"))))
}

/// The model's own figure style with `--style FILE` on top.
fn style_for(m: &Model, args: &mut Vec<String>) -> FigStyle {
    let mut st = m.figure_style.clone().unwrap_or_default();
    if let Some(f) = take_flag(args, "--style") {
        let text = std::fs::read_to_string(&f).unwrap_or_else(|e| die(format!("{f}: {e}")));
        st = st.merged(&FigStyle::from_json(&text).unwrap_or_else(|e| die(e)));
    }
    st
}

/// Where the SVG and/or PNG go. `--out x.svg --png` writes x.svg and x.png; `--out x.png` only the PNG.
fn outputs(fig: &Figure, out: &str, png: bool, scale: f32, style: &FigStyle, extra: Value) {
    let p = Path::new(out);
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut res = json!({"ok": true});
    let (mut want_svg, mut want_png) = (true, png);
    if ext == "png" {
        (want_svg, want_png) = (false, true);
    }
    if want_svg {
        let svg = fig.svg(style).unwrap_or_else(|e| die(e));
        let path = if ext == "svg" || ext.is_empty() { out.to_string() } else { format!("{out}.svg") };
        std::fs::write(&path, &svg.text).unwrap_or_else(|e| die(format!("{path}: {e}")));
        res["svg"] = json!(path);
        res["width"] = json!(svg.width);
        res["height"] = json!(svg.height);
        let sk = &svg.skipped;
        if sk.meshes + sk.bands + sk.callbacks > 0 {
            res["skipped"] = json!({"meshes": sk.meshes, "bands": sk.bands, "callbacks": sk.callbacks});
        }
    }
    if want_png {
        let bytes = fig.png(scale).unwrap_or_else(|e| die(e));
        let path = if ext == "png" { out.to_string() } else { p.with_extension("png").to_string_lossy().to_string() };
        std::fs::write(&path, bytes).unwrap_or_else(|e| die(format!("{path}: {e}")));
        res["png"] = json!(path);
        res["scale"] = json!(scale);
    }
    if let (Some(a), Some(b)) = (res.as_object_mut(), extra.as_object()) {
        a.extend(b.clone());
    }
    print(res);
}

fn render(args: &mut Vec<String>) {
    let sheet = take_flag(args, "--sheet");
    let out = take_flag(args, "--out").unwrap_or_else(|| die("--out fig.svg is needed"));
    let png = has_flag(args, "--png");
    let scale = num(args, "--scale").unwrap_or(3.0) as f32;
    if !(0.5..=8.0).contains(&scale) {
        die("--scale is between 0.5 and 8");
    }
    let with_caption = has_flag(args, "--with-caption");
    let (m, _) = files::open(&file_arg(args)).unwrap_or_else(|e| die(e));
    let style = style_for(&m, args);
    no_extra(args);
    style.check().unwrap_or_else(|e| die(e));
    let mut h = Headless::new(m, Some(&style));
    let mut fig = h.sheet(sheet.as_deref()).unwrap_or_else(|e| die(e));
    let (cap, src) = h.caption_of(sheet.as_deref());
    if with_caption {
        h.add_caption(&mut fig, &cap, &src, &style);
    }
    outputs(&fig, &out, png, scale, &style, json!({"sheet": fig.title, "caption": with_caption && !cap.is_empty()}));
}

fn chart(args: &mut Vec<String>) {
    let vars: Vec<String> = take_flag(args, "--vars").unwrap_or_else(|| die("--vars A,B is needed")).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let out = take_flag(args, "--out").unwrap_or_else(|| die("--out chart.svg is needed"));
    let from = num(args, "--from");
    let to = num(args, "--to");
    let png = has_flag(args, "--png");
    let scale = num(args, "--scale").unwrap_or(3.0) as f32;
    let kind = match take_flag(args, "--kind").unwrap_or_else(|| "line".into()).to_lowercase().as_str() {
        "line" => ChartKind::Line,
        "area" => ChartKind::Area,
        "step" => ChartKind::Step,
        "stacked" => ChartKind::Stacked,
        "bars" => ChartKind::Bars,
        "phase" => ChartKind::Phase,
        "donut" => ChartKind::Donut,
        k => die(format!("--kind “{k}” is not line, area, step, stacked, bars, phase or donut")),
    };
    let title = take_flag(args, "--title").unwrap_or_default();
    let y_label = take_flag(args, "--y-label").unwrap_or_default();
    let scenarios = take_flag(args, "--scenarios").map(|s| match s.as_str() {
        "none" => None,
        "all" => Some(Vec::new()),
        l => Some(l.split(',').map(|x| x.trim().to_string()).collect()),
    });
    let set: Vec<(String, String)> = take_all(args, "--set").into_iter().map(|s| s.split_once('=').map(|(a, b)| (a.trim().to_string(), b.trim().to_string())).unwrap_or_else(|| die(format!("--set wants NAME=VALUE, not “{s}”")))).collect();
    let (m, _) = files::open(&file_arg(args)).unwrap_or_else(|e| die(e));
    let style = style_for(&m, args);
    no_extra(args);
    style.check().unwrap_or_else(|e| die(e));
    let mut h = Headless::new(m, Some(&style));
    let spec = ChartSpec { vars, from, to, kind, title, y_label, scenarios: scenarios.flatten(), set };
    let fig = h.chart(&spec).unwrap_or_else(|e| die(e));
    outputs(&fig, &out, png, scale, &style, json!({"chart": spec.vars}));
}

/// Write the model back: to `out` if given, else to the file itself (which must be a .sdm).
fn save(m: &Model, file: &str, out: Option<&str>) -> String {
    let target = out.unwrap_or(file);
    let ext = Path::new(target).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "sdm" | "flowmodel" | "json") {
        die(format!("{target}: positions and figure settings are kept in a .sdm file; convert first (dyn convert FILE model.sdm) or pass --out NAME.sdm"));
    }
    files::save(m, target, Some("sdm")).unwrap_or_else(|e| die(e));
    target.to_string()
}

fn csv(s: Option<String>) -> Vec<String> {
    s.map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
}

fn layout(args: &mut Vec<String>) {
    let sheet = take_flag(args, "--sheet");
    let mode = take_flag(args, "--mode").unwrap_or_else(|| "tidy".into());
    let loops = csv(take_flag(args, "--loops"));
    let out = take_flag(args, "--out");
    let file = file_arg(args);
    no_extra(args);
    let (mut m, _) = files::open(&file).unwrap_or_else(|e| die(e));
    let s = figure::pick_sheet(&m, sheet.as_deref(), &[SheetKind::Diagram, SheetKind::CausalLoop]).unwrap_or_else(|e| die(e)).clone();
    let mut res = json!({"ok": true, "sheet": s.name, "mode": mode});
    match (mode.as_str(), s.kind) {
        ("tidy", SheetKind::Diagram) => dynamist::spreadsheet::tidy(&mut m, s.id),
        ("tidy", SheetKind::CausalLoop) | ("loops", SheetKind::CausalLoop) => {
            let done = figure::layout_loops(&mut m, Some(&s.name), &loops).unwrap_or_else(|e| die(e));
            res["loops"] = json!(done);
        }
        ("loops", _) => die(format!("“{}” is a stock and flow sheet; loops layout draws a causal loop sheet (make one with dyn cld --from-model)", s.name)),
        ("tidy", _) | (_, _) => die(format!("--mode is tidy or loops, and “{}” is a {} sheet", s.name, s.kind.label())),
    }
    res["saved"] = json!(save(&m, &file, out.as_deref()));
    print(res);
}

fn cld(args: &mut Vec<String>) {
    let from_model = has_flag(args, "--from-model");
    let list = has_flag(args, "--list");
    let loops = csv(take_flag(args, "--loops"));
    let out = take_flag(args, "--out");
    let name = take_flag(args, "--sheet-name").unwrap_or_else(|| "Causal loops".into());
    let layout = take_flag(args, "--layout").unwrap_or_else(|| "keep".into());
    let file = file_arg(args);
    no_extra(args);
    if !from_model && !list {
        die("dyn cld needs --from-model (build a causal loop sheet) or --list (name the loops)");
    }
    let (mut m, _) = files::open(&file).unwrap_or_else(|e| die(e));
    if list && !from_model {
        return print(json!({"ok": true, "loops": figure::list_model_loops(&m)}));
    }
    let out = out.unwrap_or_else(|| die("--out cld.sdm is needed (the model with the new sheet)"));
    let (_, kept) = figure::cld_from_model(&mut m, &loops, &name).unwrap_or_else(|e| die(e));
    if layout == "loops" {
        figure::layout_loops(&mut m, Some(&name), &[]).unwrap_or_else(|e| die(e));
    } else if layout != "keep" {
        die("--layout is keep or loops");
    }
    let saved = save(&m, &file, Some(&out));
    print(json!({"ok": true, "sheet": name, "kept": kept, "all_loops": figure::list_model_loops(&m).as_array().map(|a| a.len()), "saved": saved}));
}

fn set(args: &mut Vec<String>) {
    let sheet = take_flag(args, "--sheet");
    let out = take_flag(args, "--out");
    let file = file_arg(args);
    let (mut m, _) = files::open(&file).unwrap_or_else(|e| die(e));
    let mut done: Vec<Value> = Vec::new();
    let sh = sheet.as_deref();
    let need = |args: &mut Vec<String>, n: usize, what: &str| -> Vec<String> {
        if args.len() < n {
            die(format!("{what} needs {n} values"));
        }
        args.drain(..n).collect()
    };
    while !args.is_empty() {
        let flag = args.remove(0);
        let r: Result<Value, String> = match flag.as_str() {
            "--pos" => {
                let v = need(args, 3, "--pos VAR X Y");
                let (x, y): (f32, f32) = (v[1].parse().unwrap_or_else(|_| die("--pos X must be a number")), v[2].parse().unwrap_or_else(|_| die("--pos Y must be a number")));
                figure::set_pos(&mut m, sh, &v[0], x, y).map(|_| json!({"pos": v[0], "x": x, "y": y}))
            }
            "--polarity" => {
                let v = need(args, 3, "--polarity A B SIGN");
                figure::set_polarity(&mut m, sh, &v[0], &v[1], &v[2]).map(|_| json!({"polarity": [v[0], v[1], v[2]]}))
            }
            "--delay" => {
                let v = need(args, 2, "--delay A B");
                let on = !(args.first().is_some_and(|a| a == "off"));
                if !on {
                    args.remove(0);
                }
                figure::set_delay(&mut m, sh, &v[0], &v[1], on).map(|_| json!({"delay": [v[0], v[1]], "on": on}))
            }
            "--loop-name" => {
                let v = need(args, 2, "--loop-name ID NAME");
                figure::set_loop_name(&mut m, sh, &v[0], &v[1]).map(|_| json!({"loop": v[0], "name": v[1]}))
            }
            "--caption" => {
                let v = need(args, 1, "--caption TEXT");
                figure::set_caption(&mut m, sh, Some(&v[0]), None).map(|_| json!({"caption": v[0]}))
            }
            "--source" => {
                let v = need(args, 1, "--source TEXT");
                figure::set_caption(&mut m, sh, None, Some(&v[0])).map(|_| json!({"source": v[0]}))
            }
            other => Err(format!("unknown option “{other}” (--pos, --polarity, --delay, --loop-name, --caption, --source)")),
        };
        match r {
            Ok(v) => done.push(v),
            Err(e) => die(e),
        }
    }
    if done.is_empty() {
        die("nothing to set: use --pos VAR X Y, --polarity A B +, --delay A B, --loop-name B1 \"name\", --caption TEXT or --source TEXT");
    }
    let saved = save(&m, &file, out.as_deref());
    print(json!({"ok": true, "set": done, "saved": saved}));
}

fn style(args: &mut Vec<String>) {
    let apply = take_flag(args, "--apply");
    let show = has_flag(args, "--show");
    let out = take_flag(args, "--out");
    let file = file_arg(args);
    no_extra(args);
    let (mut m, _) = files::open(&file).unwrap_or_else(|e| die(e));
    if show || apply.is_none() {
        return print(json!({"ok": true, "style": m.figure_style, "colors": dynamist_core::figstyle::COLOR_NAMES}));
    }
    let f = apply.unwrap();
    let text = std::fs::read_to_string(&f).unwrap_or_else(|e| die(format!("{f}: {e}")));
    let st = FigStyle::from_json(&text).unwrap_or_else(|e| die(e));
    st.apply_to(&mut m).unwrap_or_else(|e| die(e));
    let saved = save(&m, &file, out.as_deref());
    print(json!({"ok": true, "applied": f, "saved": saved}));
}
