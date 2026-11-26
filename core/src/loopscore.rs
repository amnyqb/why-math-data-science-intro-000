//! Loops That Matter (Schoenberg, Davidsen and Eberlein 2020): which feedback loop drives the
//! behaviour at each moment?
//!
//! Link score of x -> z at time t: the change in z caused by the change in x alone, over the whole
//! change in z, signed by the direction of the effect. A flow's link into a stock is its share of the
//! change in the stock's net flow (the stock's acceleration). A loop's score is the product of its links' scores; its relative score divides
//! by the sum of the absolute scores of all loops at that time. The score is what the loop contributes
//! to the *change* in the variables on it, so a reinforcing loop has a positive score and a balancing
//! loop a negative one.
//!
//! Limits: Euler integration only. SMOOTH, SMOOTH3, DELAY1, DELAY3 and TREND are followed as hidden stocks, a
//! QUEUE FIFO as a level, DELAY FIXED (and the conveyor that compiles to it) by cutting the loop at the delay (see
//! wiki/Find-What-Matters.md); other stateful functions (NPV, IRR, queue ages...) are not: loops through them are listed
//! but not scored.
//!
//! Two ways to find the loops. `exhaustive`: every simple loop inside the strongly connected components, up to a cap
//! (`MAX_LOOPS`). When the cap would be reached the search switches to `approximate_strongest_path`: at sampled times,
//! a best-first search from each stock over the link graph ordered by the product of the link scores' magnitudes finds
//! the strongest loops; the union is scored over the whole run. This is our own best-first variant, not the published
//! algorithm; it can miss weak loops, and the relative scores are relative to the loops found.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use serde::Serialize;

use crate::model::Model;
use crate::sim::{self, LinkKind};

#[derive(Clone, Debug, Serialize)]
pub struct LoopInfo {
    pub id: usize,
    pub variables: Vec<String>,
    /// "R", "B" or "?" (sign at the time the loop is strongest)
    pub polarity: String,
    pub scored: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LoopScores {
    pub times: Vec<f64>,
    pub loops: Vec<LoopInfo>,
    /// loop x time
    pub score: Vec<Vec<f64>>,
    pub relative: Vec<Vec<f64>>,
    /// the exhaustive enumeration stopped at the cap (only when called with `loop_scores_exhaustive`)
    pub truncated: bool,
    /// loops through a function the scores cannot follow (a lower bound under `approximate_strongest_path`)
    pub unscored_loops: usize,
    /// "exhaustive" or "approximate_strongest_path"
    pub method: String,
    /// "euler" (the only integration the scores are defined for)
    pub integration: String,
    /// the exhaustive enumeration reached the cap and the strongest-path search was used instead
    pub cap_reached: bool,
}

pub const MAX_LOOPS: usize = 20_000;
/// Times at which the strongest-path search runs (evenly spaced over the run).
const SAMPLE_TIMES: usize = 200;
/// Pops of the search queue per (time, starting stock).
const SEARCH_BUDGET: usize = 10000;
/// Loops kept per (time, starting stock).
const LOOPS_PER_START: usize = 30;
/// Times one node may be expanded in one search.
const EXPANSIONS_PER_NODE: usize = 6;
/// Loops kept in all by the strongest-path search.
const MAX_STRONGEST: usize = 5_000;
/// Times at which the published strongest-path algorithm runs, and the loops it may keep in all.
const ES_TIMES: usize = 400;
const ES_MAX_LOOPS: usize = 50_000;
pub const METHOD_PUBLISHED: &str = "strongest_path_eberlein_schoenberg_2020";

fn sccs(n: usize, adj: &[Vec<usize>]) -> Vec<usize> {
    // Tarjan, iterative
    let (mut index, mut low, mut on, mut comp) = (vec![usize::MAX; n], vec![0; n], vec![false; n], vec![usize::MAX; n]);
    let (mut stack, mut next, mut ncomp) = (Vec::new(), 0, 0);
    for s in 0..n {
        if index[s] != usize::MAX {
            continue;
        }
        let mut work = vec![(s, 0usize)];
        index[s] = next;
        low[s] = next;
        next += 1;
        stack.push(s);
        on[s] = true;
        while let Some(&(v, i)) = work.last() {
            if i < adj[v].len() {
                work.last_mut().unwrap().1 += 1;
                let w = adj[v][i];
                if index[w] == usize::MAX {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on[w] = true;
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                work.pop();
                if let Some(&(p, _)) = work.last() {
                    low[p] = low[p].min(low[v]);
                }
                if low[v] == index[v] {
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        comp[w] = ncomp;
                        if w == v {
                            break;
                        }
                    }
                    ncomp += 1;
                }
            }
        }
    }
    comp
}

fn graph(model: &Model) -> Result<(sim::Ltm, Vec<Vec<usize>>), String> {
    let c = sim::compile(model).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("; "))?;
    let (_, ltm) = sim::simulate_ltm(&c, model)?;
    let n = ltm.names.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for l in ltm.links.iter() {
        adj[l.from].push(l.to);
    }
    Ok((ltm, adj))
}

fn enumerate(n: usize, adj: &[Vec<usize>], comp: &[usize]) -> (Vec<Vec<usize>>, bool) {
    // enumerate simple cycles; each is reported once, from its smallest node
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    let mut truncated = false;
    fn dfs(start: usize, at: usize, adj: &[Vec<usize>], comp: &[usize], path: &mut Vec<usize>, onp: &mut Vec<bool>, out: &mut Vec<Vec<usize>>, trunc: &mut bool) {
        if out.len() >= MAX_LOOPS {
            *trunc = true;
            return;
        }
        for &w in &adj[at] {
            if comp[w] != comp[start] || w < start {
                continue;
            }
            if w == start {
                out.push(path.clone());
            } else if !onp[w] {
                onp[w] = true;
                path.push(w);
                dfs(start, w, adj, comp, path, onp, out, trunc);
                path.pop();
                onp[w] = false;
            }
        }
    }
    for s in 0..n {
        let mut onp = vec![false; n];
        onp[s] = true;
        dfs(s, s, adj, comp, &mut vec![s], &mut onp, &mut cycles, &mut truncated);
        if truncated {
            break;
        }
    }
    (cycles, truncated)
}

/// Score the given cycles (node lists) with the link scores of a run.
fn score_cycles(ltm: &sim::Ltm, cycles: &[Vec<usize>], method: &str, truncated: bool, cap_reached: bool) -> LoopScores {
    let lidx: HashMap<(usize, usize), usize> = ltm.links.iter().enumerate().map(|(i, l)| ((l.from, l.to), i)).collect();
    let steps = ltm.times.len();
    let mut loops = Vec::new();
    let mut score: Vec<Vec<f64>> = Vec::new();
    let mut unscored = 0;
    for cyc in cycles {
        let links: Vec<usize> = (0..cyc.len()).map(|i| lidx[&(cyc[i], cyc[(i + 1) % cyc.len()])]).collect();
        let scored = links.iter().all(|&l| ltm.links[l].kind != LinkKind::Unscored);
        let names: Vec<String> = cyc.iter().map(|&v| ltm.names[v].clone()).collect();
        let sc: Vec<f64> = if scored { (0..steps).map(|t| links.iter().map(|&l| ltm.links[l].scores[t]).product()).collect() } else { vec![f64::NAN; steps] };
        if !scored {
            unscored += 1;
        }
        loops.push(LoopInfo { id: loops.len() + 1, variables: names, polarity: "?".into(), scored });
        score.push(sc);
    }
    // relative scores and polarity
    let mut relative = vec![vec![0.0; steps]; loops.len()];
    for t in 0..steps {
        let tot: f64 = (0..loops.len()).filter(|&i| loops[i].scored).map(|i| score[i][t].abs()).sum();
        if tot > 0.0 {
            for i in 0..loops.len() {
                if loops[i].scored {
                    relative[i][t] = score[i][t] / tot;
                }
            }
        }
    }
    for (i, l) in loops.iter_mut().enumerate() {
        if l.scored {
            let best = score[i].iter().cloned().fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
            l.polarity = if best > 0.0 { "R" } else if best < 0.0 { "B" } else { "?" }.into();
        }
    }
    LoopScores { times: ltm.times.clone(), loops, score, relative, truncated, unscored_loops: unscored, method: method.into(), integration: "euler".into(), cap_reached }
}

/// Run the model and score its loops: every loop when there are fewer than `MAX_LOOPS` (`method` "exhaustive"),
/// otherwise the strongest ones (`method` "approximate_strongest_path", `cap_reached` true).
pub fn loop_scores(model: &Model) -> Result<LoopScores, String> {
    let (ltm, adj) = graph(model)?;
    let comp = sccs(adj.len(), &adj);
    let (cycles, truncated) = enumerate(adj.len(), &adj, &comp);
    if truncated {
        let cycles = strongest_cycles_es2020(&ltm, &adj, &comp);
        return Ok(score_cycles(&ltm, &cycles, METHOD_PUBLISHED, false, true));
    }
    Ok(score_cycles(&ltm, &cycles, "exhaustive", false, false))
}

/// Exhaustive enumeration only: stops at `MAX_LOOPS` and says so in `truncated`.
pub fn loop_scores_exhaustive(model: &Model) -> Result<LoopScores, String> {
    let (ltm, adj) = graph(model)?;
    let comp = sccs(adj.len(), &adj);
    let (cycles, truncated) = enumerate(adj.len(), &adj, &comp);
    Ok(score_cycles(&ltm, &cycles, "exhaustive", truncated, false))
}

/// The published strongest-path discovery (Eberlein and Schoenberg 2020, arXiv 2006.08425) on its own.
pub fn loop_scores_published(model: &Model) -> Result<LoopScores, String> {
    let (ltm, adj) = graph(model)?;
    let comp = sccs(adj.len(), &adj);
    let cycles = strongest_cycles_es2020(&ltm, &adj, &comp);
    Ok(score_cycles(&ltm, &cycles, METHOD_PUBLISHED, false, false))
}

/// Our own best-first strongest-path discovery (kept for comparison with the published one).
pub fn loop_scores_strongest(model: &Model) -> Result<LoopScores, String> {
    let (ltm, adj) = graph(model)?;
    let comp = sccs(adj.len(), &adj);
    let cycles = strongest_cycles(&ltm, &adj, &comp);
    Ok(score_cycles(&ltm, &cycles, "approximate_strongest_path", false, false))
}

/// The loop discovery of Eberlein and Schoenberg (2020), section "The Strongest Path Technique", repeated at sampled
/// times: out-links of each variable sorted by link-score magnitude; from every stock a depth-first search multiplies
/// the magnitudes (as logarithms, which keeps products from overflowing); a loop back to the start is recorded once;
/// a variable already on the current path is skipped (that loop is found from another stock); a variable already
/// reached with an equal or higher score is not explored again. Heuristic: it does not guarantee the strongest loops.
fn strongest_cycles_es2020(ltm: &sim::Ltm, adj: &[Vec<usize>], comp: &[usize]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let steps = ltm.times.len();
    let mut size: HashMap<usize, usize> = HashMap::new();
    for &c in comp {
        *size.entry(c).or_insert(0) += 1;
    }
    let mut links_out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (i, l) in ltm.links.iter().enumerate() {
        if comp[l.from] == comp[l.to] {
            links_out[l.from].push((l.to, i));
        }
    }
    let starts: Vec<usize> = (0..n).filter(|&v| ltm.is_stock[v] && (size[&comp[v]] > 1 || links_out[v].iter().any(|&(w, _)| w == v))).collect();
    let times: Vec<usize> = if steps <= ES_TIMES { (0..steps).collect() } else { (0..ES_TIMES).map(|i| i * (steps - 1) / (ES_TIMES - 1)).collect() };
    let mut found: HashSet<Vec<usize>> = HashSet::new();
    let mut order: Vec<Vec<usize>> = Vec::new();
    #[allow(clippy::too_many_arguments)]
    fn dfs(node: usize, score: f64, s: usize, sorted: &[Vec<(usize, f64)>], best: &mut [f64], on: &mut [bool], path: &mut Vec<usize>, found: &mut HashSet<Vec<usize>>, order: &mut Vec<Vec<usize>>) {
        for &(w, lw) in &sorted[node] {
            if order.len() >= ES_MAX_LOOPS {
                return;
            }
            let sc = score + lw;
            if w == s {
                let mut p = path.clone();
                let m = (0..p.len()).min_by_key(|&i| p[i]).unwrap();
                p.rotate_left(m);
                if found.insert(p.clone()) {
                    order.push(p);
                }
            } else if on[w] || best[w] >= sc {
                continue;
            } else {
                best[w] = sc;
                on[w] = true;
                path.push(w);
                dfs(w, sc, s, sorted, best, on, path, found, order);
                path.pop();
                on[w] = false;
            }
        }
    }
    for &t in &times {
        let sorted: Vec<Vec<(usize, f64)>> = links_out
            .iter()
            .map(|v| {
                let mut o: Vec<(usize, f64)> = v
                    .iter()
                    .filter_map(|&(w, li)| {
                        let x = ltm.links[li].scores[t];
                        let m = if x.is_nan() { 1e-9 } else { x.abs() };
                        (m > 0.0).then(|| (w, m.ln()))
                    })
                    .collect();
                o.sort_by(|a, b| b.1.total_cmp(&a.1));
                o
            })
            .collect();
        for &s in &starts {
            let mut best = vec![f64::NEG_INFINITY; n];
            let mut on = vec![false; n];
            on[s] = true;
            let mut path = vec![s];
            dfs(s, 0.0, s, &sorted, &mut best, &mut on, &mut path, &mut found, &mut order);
        }
        if order.len() >= ES_MAX_LOOPS {
            break;
        }
    }
    order
}

struct Cand {
    prod: f64,
    closing: bool,
}
impl PartialEq for Cand {
    fn eq(&self, o: &Self) -> bool {
        self.prod == o.prod
    }
}
impl Eq for Cand {}
impl PartialOrd for Cand {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cand {
    fn cmp(&self, o: &Self) -> Ordering {
        self.prod.total_cmp(&o.prod)
    }
}

/// Best-first search for the strongest loops at sampled times. Returns each loop once, as nodes starting at its smallest.
fn strongest_cycles(ltm: &sim::Ltm, adj: &[Vec<usize>], comp: &[usize]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let steps = ltm.times.len();
    // out-links of each node inside its component: (to, link index)
    let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (i, l) in ltm.links.iter().enumerate() {
        if comp[l.from] == comp[l.to] {
            out[l.from].push((l.to, i));
        }
    }
    // components that can hold a loop: more than one node, or a node linked to itself
    let mut size: HashMap<usize, usize> = HashMap::new();
    for &c in comp {
        *size.entry(c).or_insert(0) += 1;
    }
    let starts: Vec<usize> = (0..n).filter(|&v| ltm.is_stock[v] && (size[&comp[v]] > 1 || out[v].iter().any(|&(w, _)| w == v))).collect();
    let times: Vec<usize> = if steps <= SAMPLE_TIMES { (0..steps).collect() } else { (0..SAMPLE_TIMES).map(|i| i * (steps - 1) / (SAMPLE_TIMES - 1)).collect() };
    let mut found: HashSet<Vec<usize>> = HashSet::new();
    let mut order: Vec<Vec<usize>> = Vec::new();
    // arena of partial paths: (node, parent)
    let mut arena: Vec<(usize, usize)> = Vec::new();
    for &t in &times {
        let weight = |li: usize| -> f64 {
            let x = ltm.links[li].scores[t];
            if x.is_nan() { 1e-9 } else { x.abs() }
        };
        for &s in &starts {
            arena.clear();
            arena.push((s, usize::MAX));
            let mut heap: BinaryHeap<(Cand, usize)> = BinaryHeap::new();
            heap.push((Cand { prod: 1.0, closing: false }, 0));
            let mut expansions: HashMap<usize, usize> = HashMap::new();
            let (mut pops, mut got) = (0usize, 0usize);
            while let Some((cand, idx)) = heap.pop() {
                pops += 1;
                if pops > SEARCH_BUDGET || got >= LOOPS_PER_START {
                    break;
                }
                let node = arena[idx].0;
                if cand.closing {
                    // a closed loop: the path from the root to idx
                    let mut path = Vec::new();
                    let mut i = arena[idx].1;
                    while i != usize::MAX {
                        path.push(arena[i].0);
                        i = arena[i].1;
                    }
                    path.reverse();
                    let m = (0..path.len()).min_by_key(|&i| path[i]).unwrap();
                    path.rotate_left(m);
                    if found.insert(path.clone()) {
                        order.push(path);
                        if order.len() >= MAX_STRONGEST {
                            return order;
                        }
                    }
                    got += 1;
                    continue;
                }
                let e = expansions.entry(node).or_insert(0);
                if *e >= EXPANSIONS_PER_NODE {
                    continue;
                }
                *e += 1;
                for &(w, li) in &out[node] {
                    let wt = weight(li);
                    if wt == 0.0 {
                        continue;
                    }
                    let prod = cand.prod * wt;
                    if w == s {
                        // closing the loop is queued with its strength so that loops come out strongest first;
                        // the closing entry points at the last node of the path
                        arena.push((s, idx));
                        heap.push((Cand { prod, closing: true }, arena.len() - 1));
                        continue;
                    }
                    let mut on = false;
                    let mut i = idx;
                    while i != usize::MAX {
                        if arena[i].0 == w {
                            on = true;
                            break;
                        }
                        i = arena[i].1;
                    }
                    if !on {
                        arena.push((w, idx));
                        heap.push((Cand { prod, closing: false }, arena.len() - 1));
                    }
                }
            }
        }
    }
    order
}

/// For each time, the loop with the largest absolute relative score (`None` when nothing is scored).
pub fn dominant(s: &LoopScores) -> Vec<Option<usize>> {
    (0..s.times.len())
        .map(|t| {
            (0..s.loops.len())
                .filter(|&i| s.loops[i].scored && s.relative[i][t].abs() > 0.0)
                .max_by(|&a, &b| s.relative[a][t].abs().total_cmp(&s.relative[b][t].abs()))
        })
        .collect()
}

/// Variables that cannot influence any of the goals' outputs: no path in the causal graph leads from them to an output.
/// Returns (name, reason) for each removable variable.
pub fn unreachable_from_goals(model: &Model, outputs: &[String]) -> Result<Vec<String>, String> {
    let c = sim::compile(model).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("; "))?;
    let (names, edges) = sim::influence_graph(&c);
    let n_vars = c.names.len();
    let key = |s: &str| crate::expr::normalize(s);
    let mut feeds: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for (a, b) in edges {
        feeds[b].push(a);
    }
    // names used anywhere in an equation also feed it: initial values of stocks, delay times and options
    // are not links of the causal graph, but removing what they read would break the model
    let index: HashMap<String, usize> = (0..n_vars).map(|i| (key(&names[i]), i)).collect();
    for i in 0..n_vars {
        let Some(v) = model.by_name(&key(&names[i])) else { continue };
        let texts = std::iter::once(v.equation.clone()).chain(v.by_item.iter().map(|x| x.equation.clone()));
        for t in texts {
            if let Ok(toks) = crate::expr::tokenize(&t) {
                for (tok, _) in toks {
                    if let crate::expr::Tok::Ident(id) = tok {
                        if let Some(&j) = index.get(&key(&id)) {
                            if j != i && !feeds[i].contains(&j) {
                                feeds[i].push(j);
                            }
                        }
                    }
                }
            }
        }
    }
    let mut seen = vec![false; names.len()];
    let mut stack: Vec<usize> = Vec::new();
    for o in outputs {
        match (0..n_vars).find(|&i| key(&names[i]) == key(o)) {
            Some(i) => stack.push(i),
            None => return Err(format!("“{o}” is not a variable of the model")),
        }
    }
    while let Some(v) = stack.pop() {
        if seen[v] {
            continue;
        }
        seen[v] = true;
        stack.extend(feeds[v].iter().cloned());
    }
    Ok((0..n_vars).filter(|&i| !seen[i] && c.has_series[i]).map(|i| names[i].clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{Op, Session};

    fn model(ops: &str) -> Model {
        let mut s = Session::new(Model::default());
        let ops: Vec<Op> = serde_json::from_str(ops).unwrap();
        s.apply("t", &ops).unwrap();
        s.model
    }

    fn settings(final_time: &str) -> String {
        format!(r#"{{"op":"set_setting","key":"final_time","value":"{final_time}"}},{{"op":"set_setting","key":"time_step","value":"0.125"}},{{"op":"set_setting","key":"method","value":"euler"}}"#)
    }

    #[test]
    fn exponential_growth_has_one_reinforcing_loop_that_always_dominates() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"1"}},
          {{"op":"add_var","name":"Growth","kind":"flow","equation":"Stock * 0.1","to":"Stock"}}]"#, settings("20")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.loops.len(), 1, "{:?}", s.loops);
        assert_eq!(s.loops[0].polarity, "R");
        assert!(s.relative[0].iter().all(|r| (r - 1.0).abs() < 1e-9), "{:?}", &s.relative[0][..3]);
    }

    #[test]
    fn goal_seeking_has_one_balancing_loop() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
          {{"op":"add_var","name":"Goal","kind":"constant","equation":"100"}},
          {{"op":"add_var","name":"Gap","kind":"aux","equation":"Goal - Stock"}},
          {{"op":"add_var","name":"Adjust","kind":"flow","equation":"Gap / 5","to":"Stock"}}]"#, settings("20")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.loops.len(), 1);
        assert_eq!(s.loops[0].polarity, "B");
        assert!(s.relative[0].iter().all(|r| (r + 1.0).abs() < 1e-9), "{:?}", &s.relative[0][..3]);
    }

    #[test]
    fn s_shaped_growth_changes_dominance_from_reinforcing_to_balancing_near_the_inflection() {
        // logistic: net growth = S * Fraction, Fraction = g * (1 - S / K): S acts on its own growth directly (reinforcing)
        // and through the crowding fraction (balancing); the two elasticities are equal where S = K / 2
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"S","kind":"stock","equation":"1"}},
          {{"op":"add_var","name":"Fraction","kind":"aux","equation":"0.5 * (1 - S / 1000)"}},
          {{"op":"add_var","name":"Net growth","kind":"flow","equation":"S * Fraction","to":"S"}}]"#, settings("30")));
        let s = loop_scores(&m).unwrap();
        let pol: HashMap<String, usize> = s.loops.iter().map(|l| (l.polarity.clone(), l.id - 1)).collect();
        let (r, b) = (pol["R"], pol["B"]);
        assert!(s.relative[r][4].abs() > s.relative[b][4].abs(), "early");
        let last = s.times.len() - 1;
        assert!(s.relative[b][last].abs() > s.relative[r][last].abs(), "late");
        // the switch happens where growth peaks: S = K/2 = 500
        let run = crate::runner::run_with(&m, &[]).unwrap();
        let sv = run.get("s").unwrap();
        let switch = (0..s.times.len()).find(|&t| s.relative[b][t].abs() > s.relative[r][t].abs()).unwrap();
        let at = sv[switch + 1];
        assert!((at - 500.0).abs() < 80.0, "switch at S = {at}");
    }

    #[test]
    fn births_and_deaths_swap_dominance_where_net_growth_peaks() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"S","kind":"stock","equation":"1"}},
          {{"op":"add_var","name":"Births","kind":"flow","equation":"S * 0.5","to":"S"}},
          {{"op":"add_var","name":"Deaths","kind":"flow","equation":"S * S * 0.5 / 1000","from":"S"}}]"#, settings("30")));
        let s = loop_scores(&m).unwrap();
        let pol: HashMap<String, usize> = s.loops.iter().map(|l| (l.polarity.clone(), l.id - 1)).collect();
        let (r, b) = (pol["R"], pol["B"]);
        assert!(s.relative[r][4].abs() > s.relative[b][4].abs(), "early");
        let sv = crate::runner::run_with(&m, &[]).unwrap().get("s").unwrap().clone();
        let switch = (0..s.times.len()).find(|&t| s.relative[b][t].abs() > s.relative[r][t].abs()).unwrap();
        assert!((sv[switch + 1] - 500.0).abs() < 80.0, "switch at S = {}", sv[switch + 1]);
    }

    #[test]
    fn variables_with_no_path_to_the_goal_are_found() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"1"}},
          {{"op":"add_var","name":"Growth","kind":"flow","equation":"Stock * 0.1","to":"Stock"}},
          {{"op":"add_var","name":"Side","kind":"stock","equation":"5"}},
          {{"op":"add_var","name":"Side flow","kind":"flow","equation":"Stock * 0.2","to":"Side"}},
          {{"op":"add_var","name":"Unused","kind":"constant","equation":"3"}}]"#, settings("5")));
        let mut r = unreachable_from_goals(&m, &["Stock".into()]).unwrap();
        r.sort();
        assert_eq!(r, vec!["Side", "Side flow", "Unused"]);
    }

    #[test]
    fn what_an_initial_value_or_a_delay_time_reads_is_not_removable() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Start","kind":"constant","equation":"7"}},
          {{"op":"add_var","name":"Lag","kind":"constant","equation":"3"}},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"Start"}},
          {{"op":"add_var","name":"Late","kind":"aux","equation":"DELAY3(Stock, Lag)"}},
          {{"op":"add_var","name":"Adjust","kind":"flow","equation":"(100 - Late) / 5","to":"Stock"}}]"#, settings("5")));
        assert!(unreachable_from_goals(&m, &["Stock".into()]).unwrap().is_empty());
    }

    #[test]
    fn a_loop_through_smooth_and_delays_is_scored_and_through_other_stateful_functions_is_listed_unscored() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
          {{"op":"add_var","name":"Perceived","kind":"aux","equation":"SMOOTH(Stock, 4)"}},
          {{"op":"add_var","name":"Adjust","kind":"flow","equation":"(100 - Perceived) / 5","to":"Stock"}}]"#, settings("30")));
        let s = loop_scores(&m).unwrap();
        assert!(s.loops.iter().any(|l| l.scored && l.polarity == "B"), "{:?}", s.loops);
        for f in ["SMOOTH3(Stock, 4)", "DELAY1(Stock, 4)", "DELAY3(Stock, 4)"] {
            let mm = model(&format!(r#"[{},
              {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
              {{"op":"add_var","name":"Perceived","kind":"aux","equation":"{f}"}},
              {{"op":"add_var","name":"Adjust","kind":"flow","equation":"(100 - Perceived) / 5","to":"Stock"}}]"#, settings("30")));
            let ss = loop_scores(&mm).unwrap();
            assert_eq!(ss.unscored_loops, 0, "{f}: {:?}", ss.loops);
            assert!(ss.loops.iter().any(|l| l.scored && l.polarity == "B" && l.variables.iter().any(|v| v == "Stock")), "{f}");
            // the scores of all loops at one time add up to a relative score of 1 in absolute value
            let t = ss.times.len() / 2;
            let tot: f64 = (0..ss.loops.len()).map(|i| ss.relative[i][t].abs()).sum();
            assert!((tot - 1.0).abs() < 1e-9, "{f}: {tot}");
        }
        // NPV is not followed: loops through it stay unscored
        let m2 = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
          {{"op":"add_var","name":"Late","kind":"aux","equation":"NPV(Stock, 0.1, 0, 1)"}},
          {{"op":"add_var","name":"Adjust","kind":"flow","equation":"(100 - Late) / 5","to":"Stock"}}]"#, settings("30")));
        let s2 = loop_scores(&m2).unwrap();
        assert!(s2.unscored_loops >= 1, "{:?}", s2.loops);
        assert_eq!(s2.integration, "euler");
    }

    // ---- W3: DELAY FIXED, TREND, QUEUE FIFO, conveyors ----

    fn abs_sum_is_one(s: &LoopScores, from: usize) {
        for t in from..s.times.len() {
            let tot: f64 = (0..s.loops.len()).map(|i| s.relative[i][t].abs()).sum();
            assert!((tot - 1.0).abs() < 1e-9, "t={t}: {tot}");
        }
    }

    #[test]
    fn goal_seeking_through_delay_fixed_is_one_balancing_loop() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
          {{"op":"add_var","name":"Late","kind":"aux","equation":"DELAY FIXED(Stock, 4, 0)"}},
          {{"op":"add_var","name":"Adjust","kind":"flow","equation":"(100 - Late) / 5","to":"Stock"}}]"#, settings("40")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.unscored_loops, 0);
        assert_eq!(s.loops.len(), 1, "{:?}", s.loops);
        assert_eq!(s.loops[0].polarity, "B");
        // once the delayed value has begun to move (after 4 time units = 32 steps) the loop has the whole score
        for t in 40..s.times.len() {
            assert!((s.relative[0][t] + 1.0).abs() < 1e-9, "t={t}: {}", s.relative[0][t]);
        }
        // before that the delayed value has not changed: nothing to attribute
        assert!(s.relative[0][3].abs() < 1e-12);
    }

    #[test]
    fn a_conveyor_with_leak_is_scored_like_delay_fixed() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
          {{"op":"add_var","name":"Late","kind":"aux","equation":"DELAY FIXED((Stock) * 0.9, 4, 0)"}},
          {{"op":"add_var","name":"Adjust","kind":"flow","equation":"(100 - Late) / 5","to":"Stock"}}]"#, settings("40")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.unscored_loops, 0);
        assert_eq!(s.loops.len(), 1);
        assert_eq!(s.loops[0].polarity, "B");
        assert!((s.relative[0][60] + 1.0).abs() < 1e-9);
    }

    #[test]
    fn delay_fixed_shares_the_change_between_the_variables_of_its_input() {
        // Late = DELAY FIXED(A + 2 B, ...): A and B are stocks fed by the late value; the two loops split the score
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"A","kind":"stock","equation":"0"}},
          {{"op":"add_var","name":"B","kind":"stock","equation":"0"}},
          {{"op":"add_var","name":"Late","kind":"aux","equation":"DELAY FIXED(A + 2 * B, 2, 0)"}},
          {{"op":"add_var","name":"Adjust A","kind":"flow","equation":"(60 - Late) / 10","to":"A"}},
          {{"op":"add_var","name":"Adjust B","kind":"flow","equation":"(60 - Late) / 20","to":"B"}}]"#, settings("30")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.unscored_loops, 0);
        assert_eq!(s.loops.len(), 2, "{:?}", s.loops);
        assert!(s.loops.iter().all(|l| l.polarity == "B"));
        abs_sum_is_one(&s, 30);
        // dA : d(2B) = 1/10 : 2/20 = 1 : 1, so the loops share evenly
        assert!((s.relative[0][40].abs() - 0.5).abs() < 1e-6, "{}", s.relative[0][40]);
    }

    #[test]
    fn trend_is_followed_through_its_average_and_its_input() {
        // growth that is damped by the perceived trend of the stock itself
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"10"}},
          {{"op":"add_var","name":"Trend of stock","kind":"aux","equation":"TREND(Stock, 5, 0.02)"}},
          {{"op":"add_var","name":"Net","kind":"flow","equation":"Stock * (0.05 - 2 * Trend of stock)","to":"Stock"}}]"#, settings("30")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.unscored_loops, 0, "{:?}", s.loops);
        assert!(s.loops.iter().any(|l| l.variables.iter().any(|v| v.starts_with("trend stage"))), "{:?}", s.loops);
        abs_sum_is_one(&s, 1);
    }

    #[test]
    fn goal_seeking_on_a_trend_sums_consistently() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"10"}},
          {{"op":"add_var","name":"Rise","kind":"aux","equation":"TREND(Stock, 4, 0.2)"}},
          {{"op":"add_var","name":"Push","kind":"flow","equation":"20 * Stock * (0.2 - Rise) / 10","to":"Stock"}}]"#, settings("30")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.unscored_loops, 0);
        assert!(!s.loops.is_empty());
        abs_sum_is_one(&s, 1);
    }

    #[test]
    fn a_queue_is_a_level_with_an_inflow_loop_and_an_outflow_loop() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Arrive","kind":"aux","equation":"(50 - Q) / 4"}},
          {{"op":"add_var","name":"Serve","kind":"aux","equation":"Q / 10"}},
          {{"op":"add_var","name":"Q","kind":"aux","equation":"QUEUE FIFO(Arrive, Serve, 0)"}}]"#, settings("30")));
        let s = loop_scores(&m).unwrap();
        assert_eq!(s.unscored_loops, 0, "{:?}", s.loops);
        assert_eq!(s.loops.len(), 2, "{:?}", s.loops);
        assert!(s.loops.iter().all(|l| l.polarity == "B"));
        // the net change in (arrive - serve) per unit of change in Q is -1/4 - 1/10: inflow share 0.25 / 0.35, outflow 0.1 / 0.35
        let t = 40;
        let mut r: Vec<f64> = (0..2).map(|i| s.relative[i][t]).collect();
        r.sort_by(|a, b| a.total_cmp(b));
        assert!((r[0] + 0.25 / 0.35).abs() < 1e-6 && (r[1] + 0.1 / 0.35).abs() < 1e-6, "{r:?}");
        // the queue reaches the balance 50 / (1 + 4 / 10)
        let q = crate::runner::run_with(&m, &[]).unwrap().get("q").unwrap().clone();
        assert!((q[q.len() - 1] - 50.0 / 1.4).abs() < 1.0);
    }

    #[test]
    fn queue_age_functions_stay_unscored() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Arrive","kind":"aux","equation":"10 - Age"}},
          {{"op":"add_var","name":"Serve","kind":"aux","equation":"2"}},
          {{"op":"add_var","name":"Q","kind":"aux","equation":"QUEUE FIFO(Arrive, Serve, 0)"}},
          {{"op":"add_var","name":"Age","kind":"aux","equation":"QUEUE AGE OLDEST(Q)"}}]"#, settings("20")));
        let s = loop_scores(&m).unwrap();
        assert!(s.unscored_loops >= 1, "{:?}", s.loops);
    }

    // ---- W1: strongest-path discovery ----

    fn dominant_sets(s: &LoopScores) -> HashSet<Vec<String>> {
        dominant(s).into_iter().flatten().map(|d| s.loops[d].variables.clone()).collect()
    }

    fn assert_full_recall(m: &Model, what: &str) {
        let ex = loop_scores_exhaustive(m).unwrap();
        let st = loop_scores_strongest(m).unwrap();
        assert_eq!(ex.method, "exhaustive");
        assert_eq!(st.method, "approximate_strongest_path");
        let (want, got) = (dominant_sets(&ex), dominant_sets(&st));
        assert!(!want.is_empty(), "{what}: nothing dominant");
        let found: HashSet<Vec<String>> = st.loops.iter().map(|l| l.variables.clone()).collect();
        for w in &want {
            assert!(found.contains(w), "{what}: dominant loop {w:?} not found; found {found:?}");
        }
        assert_eq!(want, got, "{what}");
        // the published algorithm must do the same
        let pu = loop_scores_published(m).unwrap();
        assert_eq!(pu.method, METHOD_PUBLISHED);
        let found: HashSet<Vec<String>> = pu.loops.iter().map(|l| l.variables.clone()).collect();
        for w in &want {
            assert!(found.contains(w), "{what}: published algorithm missed {w:?}");
        }
        assert_eq!(want, dominant_sets(&pu), "{what} (published)");
    }

    #[test]
    fn strongest_path_recalls_every_dominant_loop_of_the_small_models() {
        let sets = [
            ("exponential", format!(r#"[{},
              {{"op":"add_var","name":"Stock","kind":"stock","equation":"1"}},
              {{"op":"add_var","name":"Growth","kind":"flow","equation":"Stock * 0.1","to":"Stock"}}]"#, settings("20"))),
            ("goal seeking", format!(r#"[{},
              {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
              {{"op":"add_var","name":"Goal","kind":"constant","equation":"100"}},
              {{"op":"add_var","name":"Gap","kind":"aux","equation":"Goal - Stock"}},
              {{"op":"add_var","name":"Adjust","kind":"flow","equation":"Gap / 5","to":"Stock"}}]"#, settings("20"))),
            ("s-shape births and deaths", format!(r#"[{},
              {{"op":"add_var","name":"S","kind":"stock","equation":"1"}},
              {{"op":"add_var","name":"Births","kind":"flow","equation":"S * 0.5","to":"S"}},
              {{"op":"add_var","name":"Deaths","kind":"flow","equation":"S * S * 0.5 / 1000","from":"S"}}]"#, settings("30"))),
            ("logistic", format!(r#"[{},
              {{"op":"add_var","name":"S","kind":"stock","equation":"1"}},
              {{"op":"add_var","name":"Fraction","kind":"aux","equation":"0.5 * (1 - S / 1000)"}},
              {{"op":"add_var","name":"Net growth","kind":"flow","equation":"S * Fraction","to":"S"}}]"#, settings("30"))),
            ("goal seeking through smooth and delay fixed", format!(r#"[{},
              {{"op":"add_var","name":"Stock","kind":"stock","equation":"0"}},
              {{"op":"add_var","name":"Seen","kind":"aux","equation":"SMOOTH3(Stock, 3)"}},
              {{"op":"add_var","name":"Late","kind":"aux","equation":"DELAY FIXED(Seen, 2, 0)"}},
              {{"op":"add_var","name":"Adjust","kind":"flow","equation":"(100 - Late) / 8 + (100 - Stock) / 30","to":"Stock"}}]"#, settings("60"))),
        ];
        for (what, ops) in sets {
            assert_full_recall(&model(&ops), what);
        }
    }

    #[test]
    fn strongest_path_recalls_every_dominant_loop_of_the_workforce_inventory_model() {
        let text = include_str!("../../benchmarks/b3/models/b2-1-workforce-inventory-core.sdm");
        let (m, _) = crate::io::read_str(text, crate::io::Format::Sdm).unwrap();
        let ex = loop_scores_exhaustive(&m).unwrap();
        assert!(ex.loops.len() >= 2, "{:?}", ex.loops.len());
        assert_full_recall(&m, "workforce inventory");
    }

    #[test]
    fn small_models_use_the_exhaustive_method() {
        let m = model(&format!(r#"[{},
          {{"op":"add_var","name":"Stock","kind":"stock","equation":"1"}},
          {{"op":"add_var","name":"Growth","kind":"flow","equation":"Stock * 0.1","to":"Stock"}}]"#, settings("5")));
        let s = loop_scores(&m).unwrap();
        assert_eq!((s.method.as_str(), s.cap_reached, s.truncated, s.integration.as_str()), ("exhaustive", false, false, "euler"));
    }

    #[test]
    fn a_model_with_more_loops_than_the_cap_falls_back_to_the_strongest_path_search() {
        // 6 layers of 6 variables, each reading the whole layer before it: 6^6 = 46 656 loops through the stock
        let (layers, width) = (6, 6);
        let mut ops = vec![settings("10"), r#"{"op":"add_var","name":"S","kind":"stock","equation":"1"}"#.to_string()];
        for l in 0..layers {
            for j in 0..width {
                let prev: Vec<String> = if l == 0 { vec!["S".into()] } else { (0..width).map(|i| format!("L{}x{}", l - 1, i)).collect() };
                let eq = prev.iter().enumerate().map(|(i, v)| format!("{} * {}", v, 0.1 + 0.01 * ((i + j) % 5) as f64)).collect::<Vec<_>>().join(" + ");
                ops.push(format!(r#"{{"op":"add_var","name":"L{l}x{j}","kind":"aux","equation":"{eq}"}}"#));
            }
        }
        let last: Vec<String> = (0..width).map(|i| format!("L{}x{}", layers - 1, i)).collect();
        ops.push(format!(r#"{{"op":"add_var","name":"F","kind":"flow","equation":"({}) * 0.001","to":"S"}}"#, last.join(" + ")));
        let m = model(&format!("[{}]", ops.join(",")));
        let ex = loop_scores_exhaustive(&m).unwrap();
        assert!(ex.truncated);
        let s = loop_scores(&m).unwrap();
        assert_eq!((s.method.as_str(), s.cap_reached, s.truncated), (METHOD_PUBLISHED, true, false));
        assert!(!s.loops.is_empty() && s.loops.len() < MAX_LOOPS);
        assert!(s.loops.iter().all(|l| l.scored && l.polarity == "R"));
        assert!(dominant(&s).iter().any(|d| d.is_some()));
    }
}
