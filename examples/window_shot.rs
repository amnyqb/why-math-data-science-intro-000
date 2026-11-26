//! Draw the whole window to a PNG without a display: `cargo run --example window_shot -- model.sdm Home out.png [w h]`.
use dynamist::headless::Headless;
use dynamist_core::model::Model;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!("usage: window_shot MODEL.sdm TAB OUT.png [WIDTH HEIGHT]");
        std::process::exit(2);
    }
    let model: Model = if a[1] == "-" { Model::example() } else { serde_json::from_str(&std::fs::read_to_string(&a[1]).expect("read")).expect("a .sdm file") };
    let mut model = model;
    if a[1] == "-" {
        // the example model's note, dressed up, so the Text tab has something to show
        if let Some(n) = model.notes.first_mut() {
            n.fmt = dynamist_core::model::TextFmt { size: 16.0, bold: true, italic: true, underline: true, color: Some([20, 40, 120]), fill: Some([220, 235, 255]), align: dynamist_core::model::TextAlign::Center, ..Default::default() };
        }
    }
    if a[1] == "-" && std::env::var("WITH_SHAPES").is_ok() {
        use dynamist_core::model::{ImageAsset, Shape, ShapeKind};
        let view = model.first_diagram();
        // a small gradient picture, made here so the example needs no file
        let mut im = image::RgbaImage::new(120, 80);
        for (x, y, p) in im.enumerate_pixels_mut() {
            *p = image::Rgba([(x * 2) as u8, (y * 3) as u8, 160, if (x / 10 + y / 10) % 2 == 0 { 255 } else { 200 }]);
        }
        let mut png = Vec::new();
        im.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let aid = model.new_id();
        model.images.push(ImageAsset { id: aid, name: "gradient.png".into(), data: dynamist_core::b64::encode(&png), w: 120, h: 80 });
        let mut add = |kind, pos: [f32; 2], size: [f32; 2], f: &dyn Fn(&mut Shape)| {
            let id = model.new_id();
            let mut s = Shape::new(id, view, kind, pos, size);
            f(&mut s);
            model.shapes.push(s);
        };
        add(ShapeKind::Rounded, [20.0, 80.0], [260.0, 110.0], &|s| { s.fill = Some([255, 244, 214]); s.stroke = Some([214, 160, 40]); s.weight = 2.0; });
        add(ShapeKind::Ellipse, [330.0, 180.0], [170.0, 110.0], &|s| { s.stroke = Some([200, 60, 60]); s.weight = 2.5; });
        add(ShapeKind::Arrow, [560.0, 420.0], [-120.0, -60.0], &|s| { s.stroke = Some([40, 90, 200]); s.weight = 3.0; });
        add(ShapeKind::Line, [40.0, 400.0], [200.0, 0.0], &|_| {});
        add(ShapeKind::Picture, [700.0, 380.0], [180.0, 120.0], &|s| s.image = Some(aid));
    }
    if a[1] == "-" && std::env::var("WITH_CONTROLS").is_ok() {
        use dynamist_core::model::{Control, ControlKind, Kind};
        let view = model.first_diagram();
        let consts: Vec<u64> = model.vars.iter().filter(|v| v.kind == Kind::Constant).map(|v| v.id).collect();
        for (i, kind) in ControlKind::ALL.into_iter().enumerate() {
            let id = model.new_id();
            let var = consts[i % consts.len()];
            model.controls.push(Control::new(id, view, kind, var, [40.0 + 210.0 * i as f32, 470.0]));
        }
    }
    let w: f32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(1400.0);
    let h: f32 = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(820.0);
    let mut hl = Headless::new(model, None);
    let png = hl.window_png(Some(&a[2]), (w, h), 1.5).expect("png");
    std::fs::write(&a[3], png).expect("write");
    // and the diagram alone, as `dyn render` draws it, to see the notes
    if let Ok(fig) = hl.sheet(None) {
        std::fs::write(format!("{}.sheet.png", a[3]), fig.png(1.5).expect("png")).expect("write");
    }
}
