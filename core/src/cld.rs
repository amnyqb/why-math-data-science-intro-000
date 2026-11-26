//! Loop finding and the causal loop diagram built from a stock-and-flow model.

use std::collections::HashMap;

use crate::model::{CldData, CldLink, CldLoop, CldNode, Kind, Model};

/// A feedback loop found in the diagram.
#[derive(Clone, Debug)]
pub struct FoundLoop {
    pub nodes: Vec<u64>,
    pub links: Vec<u64>,
    /// Some(true) reinforcing, Some(false) balancing, None if a sign is unknown.
    pub reinforcing: Option<bool>,
}

/// Every simple feedback loop (capped, for very tangled diagrams).
pub fn find_loops(d: &CldData) -> Vec<FoundLoop> {
    let idx: HashMap<u64, usize> = d.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    let mut out_links: Vec<Vec<&CldLink>> = vec![Vec::new(); d.nodes.len()];
    for l in &d.links {
        if let (Some(&a), Some(_)) = (idx.get(&l.from), idx.get(&l.to)) {
            out_links[a].push(l);
        }
    }
    let mut found = Vec::new();
    fn dfs<'a>(
        start: usize,
        at: usize,
        idx: &HashMap<u64, usize>,
        out: &[Vec<&'a CldLink>],
        path: &mut Vec<usize>,
        lpath: &mut Vec<&'a CldLink>,
        found: &mut Vec<(Vec<usize>, Vec<&'a CldLink>)>,
    ) {
        if found.len() >= 200 || path.len() > 14 {
            return;
        }
        for l in &out[at] {
            let nxt = idx[&l.to];
            if nxt == start {
                let mut lp = lpath.clone();
                lp.push(l);
                found.push((path.clone(), lp));
            } else if nxt > start && !path.contains(&nxt) {
                path.push(nxt);
                lpath.push(l);
                dfs(start, nxt, idx, out, path, lpath, found);
                path.pop();
                lpath.pop();
            }
        }
    }
    let mut raw = Vec::new();
    for s in 0..d.nodes.len() {
        dfs(
            s,
            s,
            &idx,
            &out_links,
            &mut vec![s],
            &mut Vec::new(),
            &mut raw,
        );
    }
    for (nodes, links) in raw {
        let unknown = links.iter().any(|l| l.polarity.is_empty());
        let negatives = links.iter().filter(|l| l.polarity == "-").count();
        found.push(FoundLoop {
            nodes: nodes.iter().map(|i| d.nodes[*i].id).collect(),
            links: links.iter().map(|l| l.id).collect(),
            reinforcing: if unknown {
                None
            } else {
                Some(negatives % 2 == 0)
            },
        });
    }
    found.sort_by_key(|f| f.nodes.len());
    found
}

/// Build a causal loop diagram from the stock-and-flow model: every variable becomes
/// a word, arrows keep their signs, and flows push their stocks up (+) or down (−).
pub fn from_model(m: &mut Model) -> CldData {
    let mut d = CldData::default();
    let mut map = HashMap::new();
    let vars: Vec<(u64, String, [f32; 2], Kind)> = m
        .vars
        .iter()
        .map(|v| (v.id, v.name.clone(), v.pos, v.kind))
        .collect();
    let links: Vec<(u64, u64, f32, String)> = m
        .links
        .iter()
        .map(|l| (l.from, l.to, l.bend, l.polarity.clone()))
        .collect();
    let flows: Vec<(u64, Option<u64>, Option<u64>)> = m
        .vars
        .iter()
        .filter(|v| v.kind == Kind::Flow)
        .map(|v| (v.id, v.from, v.to))
        .collect();
    for (id, name, pos, _) in &vars {
        let nid = m.new_id();
        map.insert(*id, nid);
        d.nodes.push(CldNode {
            id: nid,
            text: name.clone(),
            pos: *pos,
        });
    }
    let add = |m: &mut Model, d: &mut CldData, a: u64, b: u64, bend: f32, pol: &str| {
        if let (Some(&x), Some(&y)) = (map.get(&a), map.get(&b)) {
            if !d.links.iter().any(|l| l.from == x && l.to == y) {
                let id = m.new_id();
                d.links.push(CldLink {
                    id,
                    from: x,
                    to: y,
                    bend,
                    polarity: if pol.is_empty() {
                        "+".into()
                    } else {
                        pol.into()
                    },
                    delay: false,
                });
            }
        }
    };
    for (a, b, bend, pol) in links {
        add(m, &mut d, a, b, bend, &pol);
    }
    for (f, from, to) in flows {
        if let Some(t) = to {
            add(m, &mut d, f, t, 22.0, "+");
        }
        if let Some(s) = from {
            add(m, &mut d, f, s, -22.0, "-");
        }
    }
    // a marker in the middle of each short loop
    let found = find_loops(&d);
    let (mut r, mut b) = (0, 0);
    for f in found.iter().filter(|f| f.nodes.len() <= 3).take(6) {
        let c = f
            .nodes
            .iter()
            .filter_map(|id| d.nodes.iter().find(|n| n.id == *id))
            .fold((0.0, 0.0, 0.0), |acc, n| {
                (acc.0 + n.pos[0], acc.1 + n.pos[1], acc.2 + 1.0)
            });
        let reinforcing = f.reinforcing.unwrap_or(true);
        let label = if reinforcing {
            r += 1;
            format!("R{r}")
        } else {
            b += 1;
            format!("B{b}")
        };
        let id = m.new_id();
        d.loops.push(CldLoop {
            id,
            pos: [c.0 / c.2, c.1 / c.2 + 30.0],
            reinforcing,
            label,
            clockwise: true,
        });
    }
    d
}
