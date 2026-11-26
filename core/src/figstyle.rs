//! The look of book figures: one small JSON file (`book.style.json`) that `dyn style --apply` stores
//! in the model and `dyn render` / `dyn chart` read. Everything is optional; an empty file changes nothing.
//!
//! ```json
//! {
//!   "font": "Georgia", "mono_font": "Menlo", "text_scale": 1.0,
//!   "line_scale": 1.2, "arrow_weight": 1, "arrow_shape": "curved", "arrow_head": "filled",
//!   "diagram_style": "classic", "width_mm": 120, "background": "#ffffff",
//!   "colors": { "stock": "#1f3a5f", "flow": "#2b7a78", "link": "#555555" }
//! }
//! ```
//! Unknown keys are refused, so a typo is not silently ignored.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{ArrowHead, ArrowShape, DiagramStyle, Model};

/// Names of the palette colours a style may set (the same names the app's theme uses).
pub const COLOR_NAMES: [&str; 17] = [
    "text", "muted", "accent", "accent_soft", "stock", "stock_fill", "flow", "aux", "constant", "lookup", "link", "error", "ok", "panel", "surface", "border", "grid",
];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FigStyle {
    /// SVG font family for ordinary text (e.g. "Georgia"). The boxes are laid out with the app's
    /// own font, so a font of similar width keeps text inside its box.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// SVG font family for monospace text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mono_font: Option<String>,
    /// Multiplies the font size of every text (boxes keep their size). Default 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<f32>,
    /// Multiplies every line width (arrows, outlines, pipes). Default 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_scale: Option<f32>,
    /// Width of the finished figure in millimetres (the SVG gets this width; the height follows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f32>,
    /// A background colour for the whole figure ("#rrggbb"); default none (transparent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Palette colours by name (see `COLOR_NAMES`), each "#rrggbb".
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, String>,
    /// "curved", "straight" or "elbow": how arrows are routed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrow_shape: Option<String>,
    /// "filled", "open" or "dot": the tip of an arrow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrow_head: Option<String>,
    /// 0 thin, 1 normal, 2 bold: set on every arrow of the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrow_weight: Option<u8>,
    /// "cards" (default) or "classic" (plain labels, thin dark arrows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_style: Option<String>,
    /// Font size of the caption under a figure (default 13); the source line is 2 smaller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption_size: Option<f32>,
}

pub fn parse_hex(s: &str) -> Result<[u8; 3], String> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("“{s}” is not a colour like #1f3a5f"));
    }
    let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap();
    Ok([v(0), v(2), v(4)])
}

impl FigStyle {
    pub fn from_json(text: &str) -> Result<FigStyle, String> {
        let st: FigStyle = serde_json::from_str(text).map_err(|e| format!("style file: {e}"))?;
        st.check()?;
        Ok(st)
    }

    /// Every value must make sense.
    pub fn check(&self) -> Result<(), String> {
        for (k, v) in &self.colors {
            if !COLOR_NAMES.contains(&k.as_str()) {
                return Err(format!("style file: unknown colour “{k}” (use one of {})", COLOR_NAMES.join(", ")));
            }
            parse_hex(v).map_err(|e| format!("style file: colors.{k}: {e}"))?;
        }
        if let Some(b) = &self.background {
            parse_hex(b).map_err(|e| format!("style file: background: {e}"))?;
        }
        for (n, v) in [("text_scale", self.text_scale), ("line_scale", self.line_scale), ("width_mm", self.width_mm), ("caption_size", self.caption_size)] {
            if v.is_some_and(|x| !(x.is_finite() && x > 0.0)) {
                return Err(format!("style file: {n} must be a positive number"));
            }
        }
        if self.arrow_weight.is_some_and(|w| w > 2) {
            return Err("style file: arrow_weight is 0, 1 or 2".into());
        }
        self.arrow_shape()?;
        self.arrow_head()?;
        self.diagram_style()?;
        Ok(())
    }

    pub fn arrow_shape(&self) -> Result<Option<ArrowShape>, String> {
        match self.arrow_shape.as_deref().map(str::to_lowercase).as_deref() {
            None => Ok(None),
            Some("curved") => Ok(Some(ArrowShape::Curved)),
            Some("straight") => Ok(Some(ArrowShape::Straight)),
            Some("elbow") => Ok(Some(ArrowShape::Elbow)),
            Some(x) => Err(format!("style file: arrow_shape “{x}” is not curved, straight or elbow")),
        }
    }
    pub fn arrow_head(&self) -> Result<Option<ArrowHead>, String> {
        match self.arrow_head.as_deref().map(str::to_lowercase).as_deref() {
            None => Ok(None),
            Some("filled") => Ok(Some(ArrowHead::Filled)),
            Some("open") => Ok(Some(ArrowHead::Open)),
            Some("dot") => Ok(Some(ArrowHead::Dot)),
            Some(x) => Err(format!("style file: arrow_head “{x}” is not filled, open or dot")),
        }
    }
    pub fn diagram_style(&self) -> Result<Option<DiagramStyle>, String> {
        match self.diagram_style.as_deref().map(str::to_lowercase).as_deref() {
            None => Ok(None),
            Some("cards") => Ok(Some(DiagramStyle::Cards)),
            Some("classic") => Ok(Some(DiagramStyle::Classic)),
            Some(x) => Err(format!("style file: diagram_style “{x}” is not cards or classic")),
        }
    }

    /// Store the style in the model and set the parts of it the model itself carries
    /// (arrow shape and head, diagram style, arrow weight).
    pub fn apply_to(&self, m: &mut Model) -> Result<(), String> {
        self.check()?;
        if let Some(s) = self.arrow_shape()? {
            m.arrow_shape = s;
            for l in m.links.iter_mut() {
                l.shape = None;
            }
        }
        if let Some(h) = self.arrow_head()? {
            m.arrow_head = h;
            for l in m.links.iter_mut() {
                l.head = None;
            }
        }
        if let Some(d) = self.diagram_style()? {
            m.diagram_style = d;
        }
        if let Some(w) = self.arrow_weight {
            for l in m.links.iter_mut() {
                l.weight = w;
            }
        }
        m.figure_style = Some(self.clone());
        Ok(())
    }

    /// `other` on top of `self`: what `other` sets wins.
    pub fn merged(&self, other: &FigStyle) -> FigStyle {
        let mut s = self.clone();
        macro_rules! take {
            ($($f:ident),*) => { $( if other.$f.is_some() { s.$f = other.$f.clone(); } )* };
        }
        take!(font, mono_font, text_scale, line_scale, width_mm, background, arrow_shape, arrow_head, arrow_weight, diagram_style, caption_size);
        for (k, v) in &other.colors {
            s.colors.insert(k.clone(), v.clone());
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_file_is_checked() {
        assert!(FigStyle::from_json("{}").is_ok());
        assert!(FigStyle::from_json(r##"{"font":"Georgia","colors":{"stock":"#1f3a5f"},"arrow_shape":"elbow"}"##).is_ok());
        assert!(FigStyle::from_json(r##"{"colors":{"stok":"#1f3a5f"}}"##).is_err());
        assert!(FigStyle::from_json(r##"{"colors":{"stock":"blue"}}"##).is_err());
        assert!(FigStyle::from_json(r#"{"fonts":"x"}"#).is_err());
        assert!(FigStyle::from_json(r#"{"arrow_head":"star"}"#).is_err());
        assert!(FigStyle::from_json(r#"{"line_scale":0}"#).is_err());
    }

    #[test]
    fn applying_sets_the_model_parts() {
        let mut m = Model::default();
        let st = FigStyle::from_json(r#"{"arrow_shape":"straight","diagram_style":"classic","arrow_weight":2}"#).unwrap();
        st.apply_to(&mut m).unwrap();
        assert_eq!(m.arrow_shape, ArrowShape::Straight);
        assert_eq!(m.diagram_style, DiagramStyle::Classic);
        assert!(m.figure_style.is_some());
    }
}
