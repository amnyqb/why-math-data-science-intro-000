//! Editing a figure without a window: positions, signs, delay marks, loop names, the loop-circle
//! layout and building a causal loop sheet from a stock-and-flow model. All of it works on the
//! model file, so `dyn set`, `dyn layout` and `dyn cld` (and the Python wrapper) share it. Drawing is
//! in the `dynamist` crate (`headless`, `svg`).

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::cld::{FoundLoop, find_loops, from_model};
use crate::expr::normalize;
use crate::model::{CldData, CldLink, CldLoop, Model, Sheet, SheetKind};

/// The sheet called `name` (case and spacing do not matter), or, with no name, the first sheet of
/// one of the `kinds`.
pub fn pick_sheet<'a>(m: &'a Model, name: Option<&str>, kinds: &[SheetKind]) -> Result<&'a Sheet, String> {
    match name {
        Some(n) => {
            let k = normalize(n);
            m.sheets.iter().find(|s| normalize(&s.name) == k).ok_or_else(|| format!("no sheet called “{n}” (sheets: {})", sheet_names(m)))
        }
        None => m.sheets.iter().find(|s| kinds.contains(&s.kind)).ok_or_else(|| format!("the model has no {} sheet (sheets: {})", kinds.iter().map(|k| k.label()).collect::<Vec<_>>().join(" or "), sheet_names(m))),
    }
}

pub fn sheet_names(m: &Model) -> String {
    m.sheets.iter().map(|s| format!("“{}”", s.name)).collect::<Vec<_>>().join(", ")
}

// ------------------------------------------------------------------ dyn set

fn cld_node(d: &CldData, text: &str) -> Result<u64, String> {
    let k = normalize(text);
    d.nodes.iter().find(|n| normalize(&n.text) == k).map(|n| n.id).ok_or_else(|| format!("no word “{text}” on the causal loop sheet"))
}

fn var_id(m: &Model, name: &str) -> Result<u64, String> {
    m.by_name(&normalize(name)).map(|v| v.id).ok_or_else(|| format!("no variable called “{name}”"))
}

/// Move a variable (stock and flow sheets) or a word (causal loop sheets) to x, y.
pub fn set_pos(m: &mut Model, sheet: Option<&str>, name: &str, x: f32, y: f32) -> Result<(), String> {
    let s = pick_sheet(m, sheet, &[SheetKind::Diagram, SheetKind::CausalLoop])?.clone();
    if s.kind == SheetKind::CausalLoop {
        let id = cld_node(&s.cld, name)?;
        let sh = m.sheet_mut(s.id).unwrap();
        sh.cld.nodes.iter_mut().find(|n| n.id == id).unwrap().pos = [x, y];
        return Ok(());
    }
    let id = var_id(m, name)?;
    let v = m.var_mut(id).unwrap();
    let (dx, dy) = (x - v.pos[0], y - v.pos[1]);
    v.pos = [x, y];
    // a flow's clouds travel with its valve
    if v.kind == crate::model::Kind::Flow {
        if v.from.is_none() {
            v.from_pt = [v.from_pt[0] + dx, v.from_pt[1] + dy];
        }
        if v.to.is_none() {
            v.to_pt = [v.to_pt[0] + dx, v.to_pt[1] + dy];
        }
    }
    Ok(())
}

/// Set the sign ("+", "-", or "" for unknown) of the arrow A → B.
pub fn set_polarity(m: &mut Model, sheet: Option<&str>, a: &str, b: &str, pol: &str) -> Result<(), String> {
    let pol = match pol {
        "+" | "pos" | "positive" => "+",
        "-" | "−" | "neg" | "negative" => "-",
        "" | "?" | "unknown" => "",
        x => return Err(format!("a sign is + or -, not “{x}”")),
    };
    let s = pick_sheet(m, sheet, &[SheetKind::CausalLoop, SheetKind::Diagram])?.clone();
    if s.kind == SheetKind::CausalLoop {
        let (x, y) = (cld_node(&s.cld, a)?, cld_node(&s.cld, b)?);
        let sh = m.sheet_mut(s.id).unwrap();
        let l = sh.cld.links.iter_mut().find(|l| l.from == x && l.to == y).ok_or_else(|| format!("no arrow from “{a}” to “{b}” on the causal loop sheet"))?;
        l.polarity = pol.into();
        return Ok(());
    }
    let (x, y) = (var_id(m, a)?, var_id(m, b)?);
    let l = m.links.iter_mut().find(|l| l.from == x && l.to == y).ok_or_else(|| format!("no arrow from “{a}” to “{b}”"))?;
    l.polarity = pol.into();
    Ok(())
}

/// Put the two delay bars on the arrow A → B (or take them off).
pub fn set_delay(m: &mut Model, sheet: Option<&str>, a: &str, b: &str, on: bool) -> Result<(), String> {
    let s = pick_sheet(m, sheet, &[SheetKind::CausalLoop, SheetKind::Diagram])?.clone();
    if s.kind == SheetKind::CausalLoop {
        let (x, y) = (cld_node(&s.cld, a)?, cld_node(&s.cld, b)?);
        let sh = m.sheet_mut(s.id).unwrap();
        sh.cld.links.iter_mut().find(|l| l.from == x && l.to == y).ok_or_else(|| format!("no arrow from “{a}” to “{b}” on the causal loop sheet"))?.delay = on;
        return Ok(());
    }
    let (x, y) = (var_id(m, a)?, var_id(m, b)?);
    m.links.iter_mut().find(|l| l.from == x && l.to == y).ok_or_else(|| format!("no arrow from “{a}” to “{b}”"))?.delay_mark = on;
    Ok(())
}

/// A loop marker is labelled "B1" or "B1 Inventory control": its id, then its name.
pub fn loop_label_matches(label: &str, id: &str) -> bool {
    let l = label.trim();
    l.eq_ignore_ascii_case(id) || l.to_lowercase().starts_with(&format!("{} ", id.to_lowercase()))
}

/// Name a loop: its marker then reads "B1 Inventory control". `id` is R1, B2… as `dyn cld` lists them.
pub fn set_loop_name(m: &mut Model, sheet: Option<&str>, id: &str, name: &str) -> Result<(), String> {
    let s = pick_sheet(m, sheet, &[SheetKind::CausalLoop])?.clone();
    if s.kind != SheetKind::CausalLoop {
        return Err(format!("“{}” is not a causal loop sheet", s.name));
    }
    let sh = m.sheet_mut(s.id).unwrap();
    let markers = sh.cld.loops.iter().map(|l| l.label.clone()).collect::<Vec<_>>().join(", ");
    let lp = sh.cld.loops.iter_mut().find(|l| loop_label_matches(&l.label, id)).ok_or_else(|| format!("no loop marker {id} on the sheet (markers: {markers})"))?;
    let id = id.to_uppercase();
    lp.label = if name.trim().is_empty() { id } else { format!("{id} {}", name.trim()) };
    Ok(())
}

/// Set a sheet's caption and source line.
pub fn set_caption(m: &mut Model, sheet: Option<&str>, caption: Option<&str>, source: Option<&str>) -> Result<(), String> {
    let id = pick_sheet(m, sheet, &[SheetKind::Diagram, SheetKind::CausalLoop])?.id;
    let sh = m.sheet_mut(id).unwrap();
    if let Some(c) = caption {
        sh.caption = c.into();
    }
    if let Some(s) = source {
        sh.source = s.into();
    }
    Ok(())
}

// ------------------------------------------------------------------ loops

/// A loop of a causal loop diagram with its R1/B2 name.
pub struct NamedLoop {
    pub id: String,
    pub found: FoundLoop,
}

/// Number the loops the way the app's side panel does: R1, R2… and B1, B2… in the finder's order.
/// Loops with an unknown sign are "U1", "U2"…
pub fn named_loops(d: &CldData) -> Vec<NamedLoop> {
    let (mut r, mut b, mut u) = (0, 0, 0);
    find_loops(d)
        .into_iter()
        .map(|f| {
            let id = match f.reinforcing {
                Some(true) => {
                    r += 1;
                    format!("R{r}")
                }
                Some(false) => {
                    b += 1;
                    format!("B{b}")
                }
                None => {
                    u += 1;
                    format!("U{u}")
                }
            };
            NamedLoop { id, found: f }
        })
        .collect()
}

fn loop_path(d: &CldData, l: &NamedLoop) -> Vec<String> {
    l.found.nodes.iter().filter_map(|n| d.nodes.iter().find(|x| x.id == *n)).map(|n| n.text.clone()).collect()
}

/// Every loop of the model, named, with the words around it (for `dyn cld --list`).
pub fn list_model_loops(m: &Model) -> Value {
    let mut copy = m.clone();
    let d = from_model(&mut copy);
    json!(named_loops(&d).iter().map(|l| json!({"id": l.id, "reinforcing": l.found.reinforcing, "variables": loop_path(&d, l)})).collect::<Vec<_>>())
}

/// A new causal loop sheet made from the model, keeping only the loops named in `keep` (R1, B2…).
/// With an empty `keep`, every loop. Returns the sheet's id and what was kept.
pub fn cld_from_model(m: &mut Model, keep: &[String], sheet_name: &str) -> Result<(u64, Value), String> {
    let mut copy = m.clone();
    let mut all = from_model(&mut copy);
    all.loops.clear();
    let named = named_loops(&all);
    if named.is_empty() {
        return Err("the model has no feedback loops".into());
    }
    let chosen: Vec<&NamedLoop> = if keep.is_empty() {
        named.iter().collect()
    } else {
        let mut v = Vec::new();
        for k in keep {
            let l = named.iter().find(|l| l.id.eq_ignore_ascii_case(k.trim())).ok_or_else(|| format!("no loop {k} (the model has {})", named.iter().map(|l| l.id.clone()).collect::<Vec<_>>().join(", ")))?;
            if !v.iter().any(|x: &&NamedLoop| x.id == l.id) {
                v.push(l);
            }
        }
        v
    };
    let mut d = CldData::default();
    let mut ids: HashMap<u64, u64> = HashMap::new();
    for l in &chosen {
        for n in &l.found.nodes {
            if !ids.contains_key(n) {
                let src = all.nodes.iter().find(|x| x.id == *n).unwrap();
                let id = m.new_id();
                ids.insert(*n, id);
                d.nodes.push(crate::model::CldNode { id, text: src.text.clone(), pos: src.pos });
            }
        }
        for li in &l.found.links {
            let src: &CldLink = all.links.iter().find(|x| x.id == *li).unwrap();
            if !d.links.iter().any(|x| x.from == ids[&src.from] && x.to == ids[&src.to]) {
                let id = m.new_id();
                d.links.push(CldLink { id, from: ids[&src.from], to: ids[&src.to], bend: src.bend, polarity: src.polarity.clone(), delay: false });
            }
        }
    }
    for l in &chosen {
        let pts: Vec<[f32; 2]> = l.found.nodes.iter().filter_map(|n| d.nodes.iter().find(|x| x.id == ids[n])).map(|n| n.pos).collect();
        let k = pts.len().max(1) as f32;
        let c = pts.iter().fold([0.0, 0.0], |a, q| [a[0] + q[0], a[1] + q[1]]);
        let id = m.new_id();
        d.loops.push(CldLoop { id, pos: [c[0] / k, c[1] / k], reinforcing: l.found.reinforcing.unwrap_or(true), label: l.id.clone(), clockwise: true });
    }
    let kept = json!(chosen.iter().map(|l| json!({"id": l.id, "reinforcing": l.found.reinforcing, "variables": loop_path(&all, l)})).collect::<Vec<_>>());
    let sid = m.new_id();
    let mut sh = Sheet::new(sid, sheet_name, SheetKind::CausalLoop);
    sh.cld = d;
    // replace a sheet of the same name
    m.sheets.retain(|s| normalize(&s.name) != normalize(sheet_name));
    m.sheets.push(sh);
    Ok((sid, kept))
}

// ------------------------------------------------------------------ layout

const LOOP_GAP: f32 = 140.0;

/// Draw each loop of a causal loop sheet as a circle with its words evenly spaced, clockwise in
/// the direction of the arrows, the marker in the middle. Loops are named like `dyn cld` names them;
/// `only` picks some (empty = all). A word shared by two loops stays where the first loop put it.
/// Words in no loop go in a row underneath. Returns the loops laid out.
pub fn layout_loops(m: &mut Model, sheet: Option<&str>, only: &[String]) -> Result<Vec<String>, String> {
    let s = pick_sheet(m, sheet, &[SheetKind::CausalLoop])?.clone();
    if s.kind != SheetKind::CausalLoop {
        return Err(format!("“{}” is not a causal loop sheet", s.name));
    }
    let d = &s.cld;
    let named = named_loops(d);
    let chosen: Vec<&NamedLoop> = named.iter().filter(|l| only.is_empty() || only.iter().any(|o| o.eq_ignore_ascii_case(&l.id))).collect();
    if chosen.is_empty() {
        return Err("no loops to lay out".into());
    }
    let mut pos: HashMap<u64, [f32; 2]> = HashMap::new();
    let mut bends: HashMap<u64, f32> = HashMap::new();
    let mut centres: Vec<(String, [f32; 2], bool)> = Vec::new();
    let mut x = 0.0f32;
    let mut bottom = 0.0f32;
    for l in &chosen {
        let n = l.found.nodes.len();
        // radius from the widest word so neighbours do not touch
        let widest = l.found.nodes.iter().filter_map(|id| d.nodes.iter().find(|q| q.id == *id)).map(|q| q.text.chars().count() as f32 * 8.5 + 20.0).fold(60.0f32, f32::max);
        let r = (n as f32 * (widest + 24.0) / std::f32::consts::TAU).max(110.0 + 10.0 * n as f32);
        let c = [x + r + widest / 2.0, r + 60.0];
        for (i, id) in l.found.nodes.iter().enumerate() {
            pos.entry(*id).or_insert_with(|| {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * i as f32 / n as f32;
                [c[0] + r * a.cos(), c[1] + r * a.sin()]
            });
        }
        // the arrows bulge outward along the circle (clockwise on the screen = a negative bend)
        let chord = 2.0 * r * (std::f32::consts::PI / n as f32).sin();
        let sag = r - (r * r - chord * chord / 4.0).max(0.0).sqrt();
        for li in &l.found.links {
            bends.entry(*li).or_insert(-sag);
        }
        centres.push((l.id.clone(), c, l.found.reinforcing.unwrap_or(true)));
        x = c[0] + r + widest / 2.0 + LOOP_GAP;
        bottom = bottom.max(c[1] + r + 40.0);
    }
    let sh = m.sheet_mut(s.id).unwrap();
    for n in sh.cld.nodes.iter_mut() {
        if let Some(p) = pos.get(&n.id) {
            n.pos = *p;
        }
    }
    // words in no chosen loop: a row underneath
    let mut rest_x = 0.0f32;
    for n in sh.cld.nodes.iter_mut().filter(|n| !pos.contains_key(&n.id)) {
        n.pos = [rest_x + 60.0, bottom + 60.0];
        rest_x += n.text.chars().count() as f32 * 8.5 + 60.0;
    }
    for l in sh.cld.links.iter_mut() {
        if let Some(b) = bends.get(&l.id) {
            l.bend = *b;
        }
    }
    // one marker per loop, in the middle of its words (its own circle's centre when none is shared)
    let at: HashMap<u64, [f32; 2]> = sh.cld.nodes.iter().map(|n| (n.id, n.pos)).collect();
    let centres: Vec<(String, [f32; 2], bool)> = chosen
        .iter()
        .zip(&centres)
        .map(|(l, (id, _, r))| {
            let pts: Vec<[f32; 2]> = l.found.nodes.iter().filter_map(|n| at.get(n).copied()).collect();
            let k = pts.len().max(1) as f32;
            let c = pts.iter().fold([0.0, 0.0], |a, q| [a[0] + q[0], a[1] + q[1]]);
            (id.clone(), [c[0] / k, c[1] / k], *r)
        })
        .collect();
    for (id, c, reinforcing) in &centres {
        if let Some(lp) = sh.cld.loops.iter_mut().find(|q| loop_label_matches(&q.label, id)) {
            lp.pos = *c;
            lp.clockwise = true;
        } else {
            let _ = reinforcing;
        }
    }
    let missing: Vec<(String, [f32; 2], bool)> = centres.iter().filter(|(id, _, _)| !sh.cld.loops.iter().any(|q| loop_label_matches(&q.label, id))).cloned().collect();
    let base = m.next_id;
    let mut k = 0;
    let sh = m.sheet_mut(s.id).unwrap();
    for (id, c, reinforcing) in missing {
        sh.cld.loops.push(CldLoop { id: base + k, pos: c, reinforcing, label: id, clockwise: true });
        k += 1;
    }
    m.next_id = base + k;
    Ok(chosen.iter().map(|l| l.id.clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CldNode, CldLink};

    fn two_loops() -> Model {
        let mut m = Model::default();
        let mut d = CldData::default();
        for (i, t) in ["Stock", "Orders", "Gap"].iter().enumerate() {
            d.nodes.push(CldNode { id: 10 + i as u64, text: (*t).into(), pos: [0.0, 0.0] });
        }
        let links = [(10, 11, "+"), (11, 12, "+"), (12, 10, "-")];
        for (i, (a, b, p)) in links.iter().enumerate() {
            d.links.push(CldLink { id: 20 + i as u64, from: *a, to: *b, bend: 0.0, polarity: (*p).into(), delay: false });
        }
        d.loops.push(CldLoop { id: 30, pos: [0.0, 0.0], reinforcing: false, label: "B1".into(), clockwise: true });
        let mut sh = Sheet::new(5, "Loops", SheetKind::CausalLoop);
        sh.cld = d;
        m.sheets.push(sh);
        m.next_id = 100;
        m
    }

    #[test]
    fn loops_are_named_like_the_side_panel() {
        let m = two_loops();
        let n = named_loops(&m.sheets[0].cld);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].id, "B1");
    }

    #[test]
    fn circle_layout_spaces_words_evenly() {
        let mut m = two_loops();
        let done = layout_loops(&mut m, Some("Loops"), &[]).unwrap();
        assert_eq!(done, vec!["B1"]);
        let d = &m.sheets[0].cld;
        let c = d.loops[0].pos;
        let radii: Vec<f32> = d.nodes.iter().map(|n| ((n.pos[0] - c[0]).powi(2) + (n.pos[1] - c[1]).powi(2)).sqrt()).collect();
        assert!(radii.iter().all(|r| (r - radii[0]).abs() < 0.5), "{radii:?}");
        assert!(d.links.iter().all(|l| l.bend < 0.0));
    }

    #[test]
    fn set_edits_polarity_delay_and_loop_name() {
        let mut m = two_loops();
        set_polarity(&mut m, None, "Gap", "Stock", "+").unwrap();
        set_delay(&mut m, None, "Orders", "Gap", true).unwrap();
        set_loop_name(&mut m, None, "B1", "Inventory control").unwrap();
        set_pos(&mut m, None, "Orders", 5.0, 6.0).unwrap();
        let d = &m.sheets[0].cld;
        assert_eq!(d.links[2].polarity, "+");
        assert!(d.links[1].delay);
        assert_eq!(d.loops[0].label, "B1 Inventory control");
        assert_eq!(d.nodes[1].pos, [5.0, 6.0]);
        assert!(set_polarity(&mut m, None, "Gap", "Orders", "+").is_err());
        // the name can be changed again by the same id
        set_loop_name(&mut m, None, "b1", "Stock control").unwrap();
        assert_eq!(m.sheets[0].cld.loops[0].label, "B1 Stock control");
    }
}
