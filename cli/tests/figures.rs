//! The figure library: the book examples are rebuilt with the `dyn` binary and compared with the
//! SVGs saved in `book/examples`, and the drawing is checked for what the book needs (real text,
//! same bytes every time, style, captions, PNG).
//!
//! `UPDATE_EXPECTED=1 cargo test -p dynamist-cli --test figures` rewrites the saved figures.
//! On Linux x86_64 the SVG must match byte for byte; elsewhere numbers may differ by 0.06 (the
//! maths library rounds a curve point differently), nothing else may.

use std::path::{Path, PathBuf};
use std::process::Command;

use dynamist::headless::{ChartSpec, Headless};
use dynamist_core::figstyle::FigStyle;
use dynamist_core::model::{Model, SheetKind};

fn fc() -> &'static str {
    env!("CARGO_BIN_EXE_dyn")
}

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("book").join("examples")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("dyn-figures-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

/// Same text, and numbers within `tol`.
fn same_svg(a: &str, b: &str, tol: f64) -> Result<(), String> {
    fn split(s: &str) -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        let ch: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < ch.len() {
            let c = ch[i];
            let starts = c.is_ascii_digit() || (c == '-' && ch.get(i + 1).is_some_and(|n| n.is_ascii_digit()));
            let in_word = i > 0 && (ch[i - 1].is_alphabetic() || ch[i - 1] == '#' || ch[i - 1] == '_');
            if starts && !in_word {
                let mut j = i + 1;
                while j < ch.len() && (ch[j].is_ascii_digit() || ch[j] == '.') {
                    j += 1;
                }
                out.push((true, ch[i..j].iter().collect()));
                i = j;
            } else {
                match out.last_mut() {
                    Some((false, t)) => t.push(c),
                    _ => out.push((false, c.to_string())),
                }
                i += 1;
            }
        }
        out
    }
    let (x, y) = (split(a), split(b));
    if x.len() != y.len() {
        return Err(format!("different structure: {} vs {} parts", x.len(), y.len()));
    }
    for (k, (p, q)) in x.iter().zip(&y).enumerate() {
        if p.0 != q.0 {
            return Err(format!("part {k}: {:?} vs {:?}", p.1, q.1));
        }
        if p.0 {
            let (u, v): (f64, f64) = (p.1.parse().unwrap(), q.1.parse().unwrap());
            if (u - v).abs() > tol {
                return Err(format!("part {k}: number {u} vs {v}"));
            }
        } else if p.1 != q.1 {
            let ctx = &p.1[p.1.len().saturating_sub(60)..];
            return Err(format!("part {k}: text differs near “{ctx}”"));
        }
    }
    Ok(())
}

#[test]
fn book_examples_rebuild_to_the_saved_figures() {
    let update = std::env::var("UPDATE_EXPECTED").is_ok();
    let exact = cfg!(all(target_os = "linux", target_arch = "x86_64"));
    let work = scratch("examples");
    let dir = if update { examples() } else { work.join("examples") };
    if !update {
        copy_dir(&examples(), &dir);
    }
    let out = Command::new("bash").arg("build.sh").current_dir(&dir).env("FC", fc()).output().expect("bash is needed to run build.sh");
    assert!(out.status.success(), "build.sh failed:\n{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if update {
        return;
    }
    let mut checked = 0;
    for ch in ["ch10", "ch11", "ch12", "ch13", "ch14", "ch15"] {
        for e in std::fs::read_dir(examples().join(ch)).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().to_string();
            if !(name.ends_with(".svg") || name.ends_with(".sdm")) {
                continue;
            }
            let (want, got) = (std::fs::read_to_string(e.path()).unwrap(), std::fs::read_to_string(dir.join(ch).join(&name)).unwrap_or_else(|_| panic!("{ch}/{name} was not rebuilt")));
            if name.ends_with(".sdm") || exact {
                assert_eq!(want, got, "{ch}/{name} differs from the saved file");
            } else if let Err(why) = same_svg(&want, &got, 0.06) {
                panic!("{ch}/{name}: {why}");
            }
            checked += 1;
        }
    }
    assert!(checked >= 20, "only {checked} files compared");
}

fn sir() -> Model {
    Model::example()
}

#[test]
fn same_file_and_style_give_the_same_bytes() {
    let st = FigStyle::from_json(r#"{"font":"Georgia","line_scale":1.2}"#).unwrap();
    let a = Headless::new(sir(), Some(&st)).sheet(None).unwrap().svg(&st).unwrap().text;
    let b = Headless::new(sir(), Some(&st)).sheet(None).unwrap().svg(&st).unwrap().text;
    assert_eq!(a, b);
    // another style, another picture
    let other = Headless::new(sir(), None).sheet(None).unwrap().svg(&FigStyle::default()).unwrap().text;
    assert_ne!(a, other);
}

#[test]
fn stock_and_flow_sheet_has_the_app_shapes_and_real_text() {
    let st = FigStyle::from_json(r##"{"font":"Georgia","mono_font":"Menlo","colors":{"stock":"#123456"}}"##).unwrap();
    let svg = Headless::new(sir(), Some(&st)).sheet(None).unwrap().svg(&st).unwrap();
    let t = &svg.text;
    assert!(t.starts_with("<?xml"), "an SVG document");
    for w in ["Susceptible", "Infected", "Recovered"] {
        assert!(t.contains(&format!(">{w}</text>")) || t.contains(w), "{w} is text");
    }
    assert!(t.contains("font-family=\"Georgia,"), "the style's font is used");
    assert!(t.contains("font-family=\"Menlo,"), "monospace font from the style");
    assert!(t.contains("#123456"), "the stock colour is the style's");
    assert!(t.contains("<rect") && t.contains("<polyline") && t.contains("<polygon"), "cards, pipes and arrow heads");
    assert!(!t.contains("<image") && !t.contains("<use"), "no pictures of text");
    assert_eq!((svg.skipped.meshes, svg.skipped.bands, svg.skipped.callbacks), (0, 0, 0), "nothing the converter could not draw");
    // an ordinary valve and cloud are drawn: the flows of the example have two pipe lines each
    assert!(t.matches("<polyline").count() > 10);
}

#[test]
fn caption_and_source_are_written_under_the_figure() {
    let mut m = sir();
    dynamist_core::figure::set_caption(&mut m, None, Some("Figure 1.1  An epidemic."), Some("Kermack and McKendrick")).unwrap();
    let st = FigStyle::default();
    let mut h = Headless::new(m, Some(&st));
    let plain = h.sheet(None).unwrap().svg(&st).unwrap();
    let mut fig = h.sheet(None).unwrap();
    let (cap, src) = h.caption_of(None);
    h.add_caption(&mut fig, &cap, &src, &st);
    let with = fig.svg(&st).unwrap();
    assert!(!plain.text.contains("epidemic"));
    assert!(with.text.contains("epidemic") && with.text.contains("Source: Kermack"));
    assert!(with.height > plain.height, "the caption adds room underneath");
}

#[test]
fn chart_draws_a_window_of_the_run() {
    let st = FigStyle::default();
    let mut h = Headless::new(sir(), Some(&st));
    let spec = ChartSpec { vars: vec!["Susceptible".into(), "Infected".into()], from: Some(10.0), to: Some(40.0), ..Default::default() };
    let svg = h.chart(&spec).unwrap().svg(&st).unwrap();
    assert!(svg.text.contains(">Susceptible</text>") || svg.text.contains("Susceptible"));
    assert!(svg.text.contains("<polyline"));
    let err = h.chart(&ChartSpec { vars: vec!["Nope".into()], ..Default::default() }).err().unwrap();
    assert!(err.contains("Nope"));
}

#[test]
fn png_comes_from_the_apps_own_rasterizer() {
    let st = FigStyle::default();
    let fig = Headless::new(sir(), Some(&st)).sheet(None).unwrap();
    let (a, b) = (fig.png(2.0).unwrap(), fig.png(4.0).unwrap());
    assert!(a.starts_with(&[0x89, b'P', b'N', b'G']));
    let w = |p: &[u8]| u32::from_be_bytes(p[16..20].try_into().unwrap());
    assert!(w(&b) > w(&a) * 3 / 2, "4x is wider than 2x");
}

fn run(args: &[&str]) -> serde_json::Value {
    let out = Command::new(fc()).args(args).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    serde_json::from_str(&text).unwrap_or_else(|_| panic!("dyn {args:?} printed {text:?} {}", String::from_utf8_lossy(&out.stderr)))
}

#[test]
fn set_layout_cld_and_style_commands_edit_the_file() {
    let d = scratch("cmds");
    let f = d.join("sir.sdm");
    std::fs::write(&f, dynamist_core::io::write_sdm(&sir()).unwrap()).unwrap();
    let f = f.to_str().unwrap();
    // name the loops, build a causal loop sheet that keeps one of them
    let l = run(&["cld", f, "--list"]);
    let loops = l["loops"].as_array().unwrap();
    assert!(!loops.is_empty(), "the example has feedback loops: {l}");
    let id = loops[0]["id"].as_str().unwrap().to_string();
    let cld = d.join("cld.sdm");
    let r = run(&["cld", f, "--from-model", "--loops", &id, "--out", cld.to_str().unwrap()]);
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["kept"].as_array().unwrap().len(), 1);
    let cld = cld.to_str().unwrap();
    assert_eq!(run(&["layout", cld, "--sheet", "Causal loops", "--mode", "loops"])["ok"], true);
    let first = r["kept"][0]["variables"][0].as_str().unwrap().to_string();
    assert_eq!(run(&["set", cld, "--sheet", "Causal loops", "--pos", &first, "10", "20", "--loop-name", &id, "A name", "--caption", "Cap"])["ok"], true);
    let (m, _) = dynamist_core::io::read_str(&std::fs::read_to_string(cld).unwrap(), dynamist_core::io::Format::Sdm).unwrap();
    let sh = m.sheets.iter().find(|s| s.kind == SheetKind::CausalLoop).unwrap();
    assert_eq!(sh.caption, "Cap");
    assert!(sh.cld.loops.iter().any(|l| l.label == format!("{id} A name")));
    assert!(sh.cld.nodes.iter().any(|n| n.text == first && n.pos == [10.0, 20.0]));
    // a bad name is refused and says why
    let bad = run(&["set", cld, "--sheet", "Causal loops", "--polarity", "Nothing", "Nowhere", "+"]);
    assert_eq!(bad["ok"], false);
    // render the causal loop sheet and tidy the stock and flow sheet
    let svg = d.join("loops.svg");
    assert_eq!(run(&["render", cld, "--sheet", "Causal loops", "--out", svg.to_str().unwrap(), "--with-caption"])["ok"], true);
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.contains("A name") && text.contains(">Cap<") || text.contains("Cap"), "marker name and caption are in the figure");
    assert_eq!(run(&["layout", cld, "--sheet", "Model", "--mode", "tidy"])["ok"], true);
    // a style file is checked, then kept in the model
    let st = d.join("s.json");
    std::fs::write(&st, r#"{"font":"Georgia","arrow_shape":"straight"}"#).unwrap();
    assert_eq!(run(&["style", cld, "--apply", st.to_str().unwrap()])["ok"], true);
    std::fs::write(&st, r#"{"fnot":"Georgia"}"#).unwrap();
    assert_eq!(run(&["style", cld, "--apply", st.to_str().unwrap()])["ok"], false);
    let shown = run(&["style", cld, "--show"]);
    assert_eq!(shown["style"]["font"], "Georgia");
}

#[test]
fn the_figure_commands_work_through_the_binary() {
    let d = scratch("render");
    let f = d.join("sir.sdm");
    std::fs::write(&f, dynamist_core::io::write_sdm(&sir()).unwrap()).unwrap();
    let (f, svg) = (f.to_str().unwrap(), d.join("fig.svg"));
    let r = run(&["render", f, "--out", svg.to_str().unwrap(), "--png", "--scale", "2"]);
    assert_eq!(r["ok"], true, "{r}");
    assert!(d.join("fig.png").exists() && svg.exists());
    let chart = d.join("chart.svg");
    let r = run(&["chart", f, "--vars", "Susceptible,Infected", "--from", "0", "--to", "30", "--out", chart.to_str().unwrap()]);
    assert_eq!(r["ok"], true, "{r}");
    // the SVG is valid for a real renderer, when there is one
    if Command::new("rsvg-convert").arg("--version").output().is_ok() {
        for s in [&svg, &chart] {
            let o = Command::new("rsvg-convert").arg(s).arg("-o").arg(d.join("x.png")).output().unwrap();
            assert!(o.status.success(), "rsvg-convert: {}", String::from_utf8_lossy(&o.stderr));
        }
    }
    // unknown sheets and variables are told apart
    let bad = run(&["render", f, "--sheet", "Nope", "--out", svg.to_str().unwrap()]);
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("Nope"));
}

#[test]
fn a_figure_puts_the_valve_halfway_along_the_pipe_and_keeps_the_clouds_where_they_were_put() {
    use dynamist_core::ops::{self, Op};
    let mut m = Model::default();
    for op in [
        r#"{"op":"add_var","name":"Tank","kind":"stock","equation":"10","units":"widget"}"#,
        r#"{"op":"add_var","name":"Intake","kind":"flow","equation":"1","units":"widget/Month","to":"Tank"}"#,
    ] {
        let op: Op = serde_json::from_str(op).unwrap();
        ops::apply(&mut m, &op).unwrap();
    }
    let (tank, intake) = (m.by_name("tank").unwrap().id, m.by_name("intake").unwrap().id);
    m.var_mut(tank).unwrap().pos = [800.0, 300.0];
    {
        let f = m.var_mut(intake).unwrap();
        f.from_pt = [100.0, 300.0];
        f.pos = [450.0, 300.0];
    }
    let mut h = Headless::new(m, None);
    h.sheet(None).unwrap();
    let f = h.model().var(intake).unwrap();
    // 700 away from the stock: the window would pull the cloud to 200 and put the valve against the box
    assert_eq!(f.from_pt, [100.0, 300.0], "the cloud stays where it was put");
    assert_eq!(f.pos, [450.0, 300.0], "the valve is halfway between the cloud and the stock");
}
