//! Dynamist: a modern stock-and-flow modelling and simulation tool (the desktop window).

#[allow(unused_imports)]
use dynamist::*;


use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut screenshot: Option<PathBuf> = None;
    let mut open: Option<PathBuf> = None;
    // --sheet <n>: start on the n-th sheet tab (1 = first)
    let mut start_sheet: Option<usize> = None;
    // --play: start by playing the run in; --optimize: start the search on an optimize sheet
    let (mut play, mut optimize, mut demo) = (false, false, false);
    // --export-png <file> [--selection]: save the current sheet (or selection) as a picture and quit
    let mut export_png: Option<PathBuf> = None;
    let mut export_sel = false;
    // --functions: open the function reference; --demo-autocomplete: type into an equation
    let (mut functions, mut demo_ac) = (false, false);
    // --customize: open the Customize chart window for the Trends graph
    let mut customize = false;
    // --classic: draw the diagram in the plain-label, thin-arrow look
    let mut classic = false;
    // --turns: give the long arrows two turning points each (to see the smooth-path look)
    let mut turns = false;
    // --hide-results: start with the results panel hidden
    let mut hide_results = false;
    let mut hide_side = false;
    let mut fold_all = false;
    let (mut my_functions, mut python_window, mut checks_window, mut compare_window) = (false, false, false, false);
    let mut layers_window = false;
    let mut outline = false;
    let mut modules_window = false;
    let (mut history_window, mut ops_window, mut compare_demo) = (false, false, false);
    // --plot A,B,C: start with these variables on the graph; --tab trends|table|causes|problems|runs; --set Name=Value (repeatable)
    let (mut plot_arg, mut tab_arg): (Option<String>, Option<String>) = (None, None);
    let mut set_args: Vec<String> = Vec::new();
    // --export-report mdl|xmile: show the conversion report of exporting the current model
    let mut export_report: Option<String> = None;
    let mut sens_window = false;
    let mut focus_arg: Option<String> = None;
    // --inbox: open the Inbox window
    let mut inbox = false;
    let mut theme_arg: Option<String> = None;
    // --save-module <file>: write the last module to a file; --import-module <file>: open the import dialog
    let (mut save_module, mut import_module): (Option<PathBuf>, Option<PathBuf>) = (None, None);
    while let Some(a) = args.next() {
        if a == "--inbox" {
            inbox = true;
        } else if a == "--theme" {
            // --theme light|dark|system: start in that look (screenshots don't read saved preferences)
            theme_arg = args.next();
        } else if a == "--classic" {
            classic = true;
        } else if a == "--turns" {
            turns = true;
        } else if a == "--hide-results" {
            hide_results = true;
        } else if a == "--hide-side" {
            hide_side = true;
        } else if a == "--fold-all" {
            fold_all = true;
        } else if a == "--sens" {
            sens_window = true;
        } else if a == "--outline" {
            outline = true;
        } else if a == "--my-functions" {
            my_functions = true;
        } else if a == "--history-window" {
            history_window = true;
        } else if a == "--plot" {
            plot_arg = args.next();
        } else if a == "--tab" {
            tab_arg = args.next();
        } else if a == "--set" {
            if let Some(v) = args.next() {
                set_args.push(v);
            }
        } else if a == "--compare-demo" {
            compare_demo = true;
        } else if a == "--ops-window" {
            ops_window = true;
        } else if a == "--modules-window" {
            modules_window = true;
        } else if a == "--export-report" {
            export_report = args.next();
        } else if a == "--compare-window" {
            compare_window = true;
        } else if a == "--layers-window" {
            layers_window = true;
        } else if a == "--checks-window" {
            checks_window = true;
        } else if a == "--python-window" {
            python_window = true;
        } else if a == "--focus" {
            // --focus Tank=Item[,Region=Group]: start with that selection in the Showing bar
            focus_arg = args.next();
        } else if a == "--customize" {
            customize = true;
        } else if a == "--functions" {
            functions = true;
        } else if a == "--demo-autocomplete" {
            demo_ac = true;
        } else if a == "--save-module" {
            save_module = args.next().map(PathBuf::from);
        } else if a == "--import-module" {
            import_module = args.next().map(PathBuf::from);
        } else if a == "--export-png" {
            export_png = args.next().map(PathBuf::from);
        } else if a == "--selection" {
            export_sel = true;
        } else if a == "--demo-arrange" {
            demo = true;
        } else if a == "--play" {
            play = true;
        } else if a == "--optimize" {
            optimize = true;
        } else if a == "--sheet" {
            start_sheet = args.next().and_then(|n| n.parse().ok());
        } else if a == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        } else if !a.starts_with('-') {
            open = Some(PathBuf::from(a));
        }
    }
    // --write-docs <dir>: regenerate the wiki's generated pages and templates, then quit
    if let Some(i) = std::env::args().position(|a| a == "--write-docs") {
        let dir = std::env::args().nth(i + 1).unwrap_or_else(|| spreadsheet::WIKI_DIR.into());
        match spreadsheet::write_generated_docs(std::path::Path::new(&dir)) {
            Ok(()) => println!("wrote documentation to {dir}"),
            Err(e) => eprintln!("could not write documentation: {e}"),
        }
        return Ok(());
    }
    // --write-functions <file>: write functions.json (every function with its group, kind, unit rule, equivalents and export), then quit
    if let Some(i) = std::env::args().position(|a| a == "--write-functions") {
        let path = std::env::args().nth(i + 1).unwrap_or_else(|| "functions.json".into());
        match std::fs::write(&path, dynamist::fnjson::functions_json()) {
            Ok(()) => println!("wrote {path}"),
            Err(e) => eprintln!("could not write {path}: {e}"),
        }
        return Ok(());
    }
    // --convert <folder-or-workbook> <out.xlsx>: read a workbook in the template (or a folder of CSV
    // files), build the model, and write it back out as one tidy .xlsx; then quit
    if let Some(i) = std::env::args().position(|a| a == "--convert") {
        let (a, b) = (std::env::args().nth(i + 1), std::env::args().nth(i + 2));
        let (Some(a), Some(b)) = (a, b) else {
            eprintln!("usage: dynamist --convert <folder or workbook> <out.xlsx or out.sdm>");
            return Ok(());
        };
        match spreadsheet::read_book(std::path::Path::new(&a)).and_then(|bk| spreadsheet::model_from_book(&bk)) {
            Ok((m, notes)) => {
                for n in &notes {
                    println!("note: {n}");
                }
                if b.ends_with(".sdm") {
                    // the native model file, for `dyn` and other clients of the core crate
                    match dynamist_core::io::write_sdm(&m).and_then(|t| std::fs::write(&b, t).map_err(|e| e.to_string())) {
                        Ok(()) => println!("{} variables, {} sets → {b}", m.vars.len(), m.sets.len()),
                        Err(e) => eprintln!("could not write {b}: {e}"),
                    }
                    return Ok(());
                }
                match spreadsheet::write_xlsx(&spreadsheet::book_from_model(&m), std::path::Path::new(&b)) {
                    Ok(()) => println!("{} variables, {} sets → {b}", m.vars.len(), m.sets.len()),
                    Err(e) => eprintln!("could not write {b}: {e}"),
                }
            }
            Err(e) => eprintln!("could not read {a}: {e}"),
        }
        return Ok(());
    }
    // --import-sheet <file>: start by importing a spreadsheet
    let import_sheet = std::env::args().position(|a| a == "--import-sheet").and_then(|i| std::env::args().nth(i + 1));
    let shot = screenshot.is_some();
    let screenshot_for_dialog = shot;

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Dynamist")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([900.0, 600.0]);
    if shot {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    if shot {
        options.run_and_return = true;
        #[cfg(target_os = "macos")]
        {
            options.event_loop_builder = Some(Box::new(|b| {
                use winit::platform::macos::EventLoopBuilderExtMacOS;
                b.with_activate_ignoring_other_apps(false);
            }));
        }
    }
    eframe::run_native(
        "Dynamist",
        options,
        Box::new(move |cc| {
            let mut a = app::App::new(&cc.egui_ctx, screenshot);
            if let Some(p) = open {
                a.open_path(p);
            }
            if let Some(f) = export_report.as_deref().and_then(dynamist_core::io::Format::from_name) {
                if let Ok((_, loss)) = dynamist_core::io::write_str(&a.model, f) {
                    a.conversion = Some(convert_ui::Conversion { title: format!("Export as {}", f.name()), summary: "Exported (preview: nothing was written to disk)".into(), report: loss });
                }
            }
            if let Some(t) = theme_arg.as_deref() {
                let c = match t {
                    "dark" => theme::ThemeChoice::Dark,
                    "system" => theme::ThemeChoice::System,
                    _ => theme::ThemeChoice::Light,
                };
                a.set_theme_choice(&cc.egui_ctx, c);
            }
            if classic {
                a.model.diagram_style = model::DiagramStyle::Classic;
            }
            if hide_side {
                a.show_right = false;
            }
            if hide_results {
                a.show_bottom = false;
            }
            if turns {
                let pos: std::collections::HashMap<u64, [f32; 2]> = a.model.vars.iter().map(|v| (v.id, v.pos)).collect();
                for l in a.model.links.iter_mut() {
                    let (Some(p), Some(q)) = (pos.get(&l.from), pos.get(&l.to)) else { continue };
                    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 220.0 {
                        let (nx, ny) = (-dy / len, dx / len);
                        l.via = vec![[p[0] + dx * 0.33 + nx * 45.0, p[1] + dy * 0.33 + ny * 45.0], [p[0] + dx * 0.66 - nx * 45.0, p[1] + dy * 0.66 - ny * 45.0]];
                    }
                }
            }
            if demo {
                // a module for the awareness part, and a multi-selection to show the arrange bar
                a.select_one(None);
                for n in [
                    "Fraction Infected",
                    "Total Population",
                    "Effect of Awareness",
                ] {
                    if let Some(id) = a.model.by_name(&expr::normalize(n)).map(|v| v.id) {
                        a.toggle_sel(app::Sel::Var(id));
                    }
                }
                a.make_module();
                if let Some(m) = a.model.modules.last_mut() {
                    m.name = "Awareness".into();
                    let inner = m.inner;
                    if let Some(s) = a.model.sheet_mut(inner) {
                        s.name = "Awareness".into();
                    }
                }
                a.select_one(None);
                for n in ["Susceptible", "Infected", "Recovered"] {
                    if let Some(id) = a.model.by_name(&expr::normalize(n)).map(|v| v.id) {
                        a.toggle_sel(app::Sel::Var(id));
                    }
                }
            }
            if let Some(s) = start_sheet.and_then(|n| a.model.sheets.get(n.saturating_sub(1))) {
                a.sheet = s.id;
            }
            if customize {
                a.model.trend_style.title = "How the outbreak unfolds".into();
                a.model.trend_style.subtitle = "Each line is a group of people; scenarios dashed".into();
                a.model.trend_style.y_label = "People".into();
                a.customize_chart(chartstyle::ChartTarget::Trends);
            }
            if functions || demo_ac {
                if let Some(id) = a.model.by_name("infection rate").map(|v| v.id) {
                    a.select_one(Some(app::Sel::Var(id)));
                    if functions {
                        a.open_functions(Some(id));
                        if let Some(b) = a.functions.as_mut() {
                            b.selected = "SMOOTH";
                        }
                    } else {
                        a.model.var_mut(id).unwrap().equation = "Contact Rate * Infectivity * SMOOTH(Infected, Ill".into();
                        a.canvas.focus_equation = true;
                    }
                }
            }
            if inbox {
                a.open_inbox(inbox::Tab::Open);
            }
            if let Some(p) = &import_sheet {
                a.import_spreadsheet_path(std::path::Path::new(p));
            }
            if screenshot_for_dialog {
                a.import_notes = None;
            }
            // a sheet asked for on the command line wins over the one an import opens
            if import_sheet.is_some() {
                if let Some(s) = start_sheet.and_then(|n| a.model.sheets.get(n.saturating_sub(1))) {
                    a.sheet = s.id;
                }
            }
            for kv in &set_args {
                if let Some((k, v)) = kv.split_once('=') {
                    a.model.apply_change(k.trim(), v.trim());
                }
            }
            if !set_args.is_empty() {
                a.changed();
                a.simulate();
            }
            if let Some(names) = &plot_arg {
                a.plotted = names.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            }
            if let Some(t) = tab_arg.as_deref() {
                a.bottom_tab = match t {
                    "table" => app::BottomTab::Table,
                    "causes" => app::BottomTab::Causes,
                    "problems" => app::BottomTab::Problems,
                    "runs" => app::BottomTab::Runs,
                    _ => app::BottomTab::Trends,
                };
            }
            if my_functions {
                a.model.functions.push(userfn::UserFn { name: "Effect of price".into(), params: vec!["p".into()], body: "1 - 0.3 * LN(p)".into(), description: "Demand falls as price rises above the base price".into() });
                a.model.functions.push(userfn::UserFn { name: "Capped".into(), params: vec!["x".into(), "top".into()], body: "MIN(CLAMP(x, 0, top), top)".into(), description: "x kept between 0 and a top".into() });
                a.functions_open = true;
            }
            if checks_window {
                a.checks_open = true;
            }
            if layers_window {
                a.layers.open = true;
            }
            if history_window || ops_window || compare_demo {
                let before = a.model.clone();
                // three edits made the way the operations language makes them, so there is something to list
                let steps = [
                    r#"[{"op":"set_equation","name":"Contact Rate","equation":"8"}]"#,
                    r#"[{"op":"add_var","name":"Mask use","kind":"constant","equation":"0.3","units":"Fraction"},{"op":"add_check","name":"Peak stays below 9000","var":"Infected","max":9000}]"#,
                    r#"[{"op":"rename_var","name":"Infectivity","to":"Infection chance"},{"op":"set_setting","key":"final_time","value":"60"}]"#,
                ];
                for t in steps {
                    let batch: Vec<dynamist_core::ops::Op> = serde_json::from_str(t).unwrap();
                    a.checkpoint();
                    for op in &batch {
                        let _ = dynamist_core::ops::apply(&mut a.model, op);
                    }
                    a.changed();
                }
                if compare_demo {
                    let c = a.compare_models("the starter model".into(), &before);
                    a.history.compare = Some(c);
                } else if history_window {
                    a.history.history_open = true;
                } else {
                    a.history.ops_open = true;
                    a.history.ops_text = "[\n  {\"op\": \"set_equation\", \"name\": \"Infection chance\", \"equation\": \"0.9 *\"}\n]".replace("\\n", "\n");
                    a.run_operations();
                }
            }
            if modules_window {
                // a small demo: one Tank module used twice
                let batch: Vec<dynamist_core::ops::Op> = serde_json::from_str(r#"[
                  {"op":"set_setting","key":"final_time","value":"10"},{"op":"set_setting","key":"time_units","value":"Day"},
                  {"op":"define_module","name":"Tank","description":"A tank that drains at a given rate","vars":[
                     {"name":"Level","kind":"stock","equation":"Start","units":"MB"},
                     {"name":"Start","kind":"constant","equation":"100","units":"MB"},
                     {"name":"Draw","kind":"flow","equation":"Rate","units":"MB/Day","from":"Level"},
                     {"name":"Rate","kind":"aux","equation":"1","units":"MB/Day"}],
                   "ports":[{"name":"Rate in","direction":"in","kind":"information","units":"MB/Day","var":"Rate"},
                            {"name":"Level out","direction":"out","kind":"material","units":"MB","var":"Level"}]},
                  {"op":"instantiate","def":"Tank","name":"North","params":{"Start":"50"},"inputs":{"Rate in":"2"}},
                  {"op":"instantiate","def":"Tank","name":"South","inputs":{"Rate in":"3"}}]"#).unwrap();
                for op in &batch {
                    let _ = dynamist_core::ops::apply(&mut a.model, op);
                }
                a.model.title = "Two tanks from one module".into();
                a.changed();
                a.modules_open = true;
            }
            if compare_window {
                // a small demo: customers grow at 4% a month in the model; the data grows a bit faster
                let mut m = model::Model { title: "Customers (demo)".into(), ..Default::default() };
                m.settings.final_time = 36.0;
                m.settings.time_step = 1.0;
                m.settings.time_units = "Month".into();
                let mut stock = model::Var::new(1, "Customers", model::Kind::Stock, [200.0, 200.0]);
                stock.equation = "100".into();
                let mut growth = model::Var::new(2, "Joining", model::Kind::Flow, [400.0, 200.0]);
                growth.equation = "Customers * Rate".into();
                growth.to = Some(1);
                let mut rate = model::Var::new(3, "Rate", model::Kind::Constant, [300.0, 80.0]);
                rate.equation = "0.04".into();
                let mut actual = model::Var::new(4, "Customers actual", model::Kind::Lookup, [200.0, 340.0]);
                actual.equation = "Time".into();
                actual.table = (0..=12).map(|i| { let t = (i * 3) as f64; [t, 100.0 * 1.045f64.powf(t) + 6.0 * (t * 0.9).sin()] }).collect();
                m.vars = vec![stock, growth, rate, actual];
                m.next_id = 10;
                m.ensure_sheets();
                a.model = m;
                a.changed();
                a.compare_open = true;
            }
            if python_window {
                a.py_open = true;
                a.py_form.preview = vec![[-4.0, 0.0], [-2.0, 0.0228], [0.0, 0.5], [2.0, 0.9772], [4.0, 1.0]];
                a.py_form.status = "✓ 41 points, y from 0.0000 to 1.0000".into();
            }
            if sens_window {
                a.sens_open = true;
                if let Some(id) = a.model.by_name("contact rate").map(|v| v.id) {
                    a.model.var_mut(id).unwrap().lever = true;
                }
                let model = a.model.clone();
                let key = "Infected";
                a.sens.var = key.into();
                a.sens.result = sens::run_sensitivity(&model, key, 80, 0.3, 7, 4, &std::sync::atomic::AtomicUsize::new(0)).ok();
            }
            if outline {
                a.show_outline = true;
            }
            if fold_all {
                a.model.modules.iter_mut().for_each(|m| m.collapsed = true);
            }
            if let Some(f) = &focus_arg {
                for kv in f.split(',') {
                    if let Some((k, v)) = kv.split_once('=') {
                        a.focus.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
                a.simulate();
            }
            if let Some(p) = save_module {
                if let Some(f) = a.model.modules.last().and_then(|m| a.module_file(m.id)) {
                    let _ = std::fs::write(&p, serde_json::to_string_pretty(&f).unwrap_or_default());
                }
            }
            if let Some(p) = import_module {
                if let Some(f) = std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<modfile::ModuleFile>(&t).ok()) {
                    a.begin_import(f);
                }
            }
            if let Some(p) = export_png {
                let what = if export_sel {
                    export::ExportWhat::Selection
                } else {
                    export::ExportWhat::Sheet
                };
                a.cli_export = Some((what, p));
            }
            if play {
                a.run_and_play(&cc.egui_ctx);
                a.reveal_secs = 4.0;
            }
            if optimize {
                if let Some(s) = a
                    .model
                    .sheets
                    .iter()
                    .find(|s| s.id == a.sheet && s.kind == model::SheetKind::Optimize)
                {
                    a.opt.job = Some(optimize::start(a.model.clone(), s.opt.clone(), s.id));
                }
            }
            Ok(Box::new(a))
        }),
    )
}
