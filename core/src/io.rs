//! Reading and writing model files. The native `.sdm` is JSON. It can be written whole, or
//! split into a *meaning* file (variables, equations, sets, checks: stable, human-readable) and a
//! *layout* file (where everything is drawn), so a version-control diff shows what changed in the
//! model and not the coordinates that moved. Other formats plug in here.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::loss::LossReport;
use crate::model::Model;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Sdm,
    Xmile,
    Mdl,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Sdm => "sdm",
            Format::Xmile => "xmile",
            Format::Mdl => "mdl",
        }
    }
    pub fn from_name(s: &str) -> Option<Format> {
        match s.trim().trim_start_matches('.').to_lowercase().as_str() {
            "sdm" | "flowmodel" | "json" | "flow" => Some(Format::Sdm),
            "xmile" | "stmx" | "itmx" | "xml" => Some(Format::Xmile),
            "mdl" | "vensim" => Some(Format::Mdl),
            _ => None,
        }
    }
}

/// Read a model file as text. Files saved by older Vensim versions are often not UTF-8 (Windows-1252 or
/// Latin-1); those bytes are read one character each instead of failing, so names with é or ü survive.
pub fn read_text(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
    })
}

/// Decide the format of a file from its extension, then from its first characters.
pub fn detect(path: &str, text: &str) -> Option<Format> {
    if let Some(ext) = path.rsplit('.').next().filter(|e| *e != path) {
        if let Some(f) = Format::from_name(ext) {
            return Some(f);
        }
    }
    let t = text.trim_start();
    if t.starts_with('{') {
        Some(Format::Sdm)
    } else if t.starts_with('<') {
        Some(Format::Xmile)
    } else if t.contains("~~|") || t.contains("\n\t~\t") || t.starts_with("{UTF-8}") || t.contains("\\\\\\---///") {
        Some(Format::Mdl)
    } else {
        None
    }
}

/// Read model text in any supported format.
pub fn read_str(text: &str, format: Format) -> Result<(Model, LossReport), String> {
    let (m, mut loss) = read_raw(text, format)?;
    loss.source = Some(format.name().into());
    loss.target = Some("sdm".into());
    Ok((m, loss))
}

fn read_raw(text: &str, format: Format) -> Result<(Model, LossReport), String> {
    match format {
        Format::Sdm => {
            let mut m: Model = serde_json::from_str(text).map_err(|e| format!("not a Dynamist model: {e}"))?;
            m.ensure_sheets();
            Ok((m, LossReport::default()))
        }
        Format::Mdl => crate::mdl::read(text),
        Format::Xmile => crate::xmile::read(text),
    }
}

/// Write a model in any supported format, with what the format could not carry.
pub fn write_str(m: &Model, format: Format) -> Result<(String, LossReport), String> {
    let (text, mut loss) = write_raw(m, format)?;
    loss.source = Some("sdm".into());
    loss.target = Some(format.name().into());
    Ok((text, loss))
}

fn write_raw(m: &Model, format: Format) -> Result<(String, LossReport), String> {
    match format {
        Format::Sdm => Ok((write_sdm(m)?, LossReport::default())),
        Format::Mdl => Ok(crate::mdlw::write(m)),
        Format::Xmile => Ok(crate::xmile::write(m)),
    }
}

/// Write a model in the native format.
pub fn write_sdm(m: &Model) -> Result<String, String> {
    serde_json::to_string_pretty(m).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- meaning and layout apart

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VarLayout {
    pub pos: [f32; 2],
    pub from_pt: [f32; 2],
    pub to_pt: [f32; 2],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LinkLayout {
    pub bend: f32,
    pub via: Vec<[f32; 2]>,
}

/// Where everything is drawn, keyed by names (not numbers) so it survives reordering.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Layout {
    pub vars: BTreeMap<String, VarLayout>,
    /// "from -> to"
    pub links: BTreeMap<String, LinkLayout>,
    pub modules: BTreeMap<String, [f32; 2]>,
    pub notes: BTreeMap<u64, [f32; 2]>,
    /// Drawn shapes and the pictures they show: all presentation, so they live here and not in the meaning file.
    #[serde(default)]
    pub shapes: Vec<crate::model::Shape>,
    #[serde(default)]
    pub images: Vec<crate::model::ImageAsset>,
    #[serde(default)]
    pub controls: Vec<crate::model::Control>,
}

fn link_key(m: &Model, from: u64, to: u64) -> String {
    let n = |i: u64| m.var(i).map(|v| v.name.clone()).unwrap_or_default();
    format!("{} -> {}", n(from), n(to))
}

pub fn layout_of(m: &Model) -> Layout {
    Layout {
        vars: m.vars.iter().map(|v| (v.name.clone(), VarLayout { pos: v.pos, from_pt: v.from_pt, to_pt: v.to_pt })).collect(),
        links: m.links.iter().map(|l| (link_key(m, l.from, l.to), LinkLayout { bend: l.bend, via: l.via.clone() })).collect(),
        modules: m.modules.iter().map(|x| (x.name.clone(), x.pos)).collect(),
        notes: m.notes.iter().map(|n| (n.id, n.pos)).collect(),
        shapes: m.shapes.clone(),
        images: m.images.clone(),
        controls: m.controls.clone(),
    }
}

/// The same model with every position and arrow bend set to zero.
pub fn without_layout(m: &Model) -> Model {
    let mut s = m.clone();
    for v in s.vars.iter_mut() {
        v.pos = [0.0, 0.0];
        v.from_pt = [0.0, 0.0];
        v.to_pt = [0.0, 0.0];
    }
    for l in s.links.iter_mut() {
        l.bend = 0.0;
        l.via.clear();
    }
    for x in s.modules.iter_mut() {
        x.pos = [0.0, 0.0];
    }
    for n in s.notes.iter_mut() {
        n.pos = [0.0, 0.0];
    }
    s.shapes.clear();
    s.images.clear();
    s.controls.clear();
    s
}

/// Put a layout back onto a model; anything the layout does not know keeps its place.
pub fn apply_layout(m: &mut Model, l: &Layout) {
    let names: Vec<(u64, u64, String)> = m.links.iter().map(|k| (k.from, k.to, link_key(m, k.from, k.to))).collect();
    for v in m.vars.iter_mut() {
        if let Some(p) = l.vars.get(&v.name) {
            v.pos = p.pos;
            v.from_pt = p.from_pt;
            v.to_pt = p.to_pt;
        }
    }
    for (k, (_, _, key)) in m.links.iter_mut().zip(names) {
        if let Some(p) = l.links.get(&key) {
            k.bend = p.bend;
            k.via = p.via.clone();
        }
    }
    for x in m.modules.iter_mut() {
        if let Some(p) = l.modules.get(&x.name) {
            x.pos = *p;
        }
    }
    for n in m.notes.iter_mut() {
        if let Some(p) = l.notes.get(&n.id) {
            n.pos = *p;
        }
    }
    if !l.shapes.is_empty() || !l.images.is_empty() || !l.controls.is_empty() {
        m.shapes = l.shapes.clone();
        m.images = l.images.clone();
        m.controls = l.controls.clone();
    }
}

/// (meaning file text, layout file text)
pub fn write_split(m: &Model) -> Result<(String, String), String> {
    Ok((write_sdm(&without_layout(m))?, serde_json::to_string_pretty(&layout_of(m)).map_err(|e| e.to_string())?))
}

pub fn read_split(meaning: &str, layout: Option<&str>) -> Result<Model, String> {
    let (mut m, _) = read_str(meaning, Format::Sdm)?;
    if let Some(l) = layout {
        apply_layout(&mut m, &serde_json::from_str::<Layout>(l).map_err(|e| format!("layout file: {e}"))?);
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_are_recognised_by_name_and_by_content() {
        assert_eq!(detect("a.sdm", ""), Some(Format::Sdm));
        assert_eq!(detect("old.flowmodel", ""), Some(Format::Sdm), "files saved before the rename still open");
        assert_eq!(detect("a.stmx", ""), Some(Format::Xmile));
        assert_eq!(detect("model.mdl", ""), Some(Format::Mdl));
        assert_eq!(detect("noext", "<?xml version=\"1.0\"?><xmile>"), Some(Format::Xmile));
        assert_eq!(detect("noext", "{\"title\":\"x\"}"), Some(Format::Sdm));
        assert_eq!(detect("noext", "x = 1 ~ ~ |"), None);
    }

    #[test]
    fn the_meaning_file_does_not_move_when_the_drawing_does() {
        let mut m = Model::example();
        let (a, _) = write_split(&m).unwrap();
        for v in m.vars.iter_mut() {
            v.pos[0] += 37.0;
            v.pos[1] -= 11.0;
        }
        let (b, layout) = write_split(&m).unwrap();
        assert_eq!(a, b, "meaning must be identical when only positions change");
        // and putting the layout back restores the drawing
        let back = read_split(&b, Some(&layout)).unwrap();
        assert_eq!(back.vars.iter().map(|v| v.pos).collect::<Vec<_>>(), m.vars.iter().map(|v| v.pos).collect::<Vec<_>>());
        assert_eq!(back.vars.len(), m.vars.len());
    }
}
