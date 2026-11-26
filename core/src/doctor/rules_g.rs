//! Stage C2, rules G: the graphs.
//!
//! G2 to G10 and G12 are below the first two.
//!
//! G1: an algebraic loop in the run equations (Tarjan's strongly connected components on the dependency graph with the
//! stocks removed; a component of more than one variable, or a variable that uses itself).
//! G11: double counting: a stock named Total or All whose initial value equals the sum of other stocks' initial values.

use std::collections::HashMap;

use super::{Ctx, Finding, Fix, FnRule, Rule, Severity, Stage};
use crate::expr::{self, BinOp, Expr};
use crate::model::Kind;
use crate::ops::Op;

pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FnRule { id: "G1", stage: Stage::C2, about: "An algebraic loop: variables that depend on each other in the same step with no stock or delay between them.", f: g1 }),
        Box::new(FnRule { id: "G2", stage: Stage::C2, about: "A stock whose equation uses run variables instead of only its initial value (its changes must come through flows).", f: g2 }),
        Box::new(FnRule { id: "G3", stage: Stage::C2, about: "A flow attached to no stock at either end.", f: g3 }),
        Box::new(FnRule { id: "G4", stage: Stage::C2, about: "A flow read as information by another variable.", f: g4 }),
        Box::new(FnRule { id: "G5", stage: Stage::C2, about: "A flow that is just another stock's value.", f: g5 }),
        Box::new(FnRule { id: "G6", stage: Stage::C2, about: "A stock with no flows.", f: g6 }),
        Box::new(FnRule { id: "G7", stage: Stage::C2, about: "A variable that affects no stock, output or KPI.", f: g7 }),
        Box::new(FnRule { id: "G8", stage: Stage::C2, about: "An equation with more than 3 inputs.", f: g8 }),
        Box::new(FnRule { id: "G9", stage: Stage::C2, about: "An arrow that its equation does not use, or an equation input with no arrow.", f: g9 }),
        Box::new(FnRule { id: "G10", stage: Stage::C2, about: "A part of the model with no path to the declared KPIs.", f: g10 }),
        Box::new(FnRule { id: "G11", stage: Stage::C2, about: "Double counting: a stock called Total or All that starts as the sum of other stocks.", f: g11 }),
        Box::new(FnRule { id: "G12", stage: Stage::C2, about: "A causal loop diagram arrow whose sign differs from what the equations give.", f: g12 }),
    ]
}

/// Tarjan's strongly connected components. `adj[v]` lists the successors of v. Returns the components (each a list of nodes).
pub fn tarjan(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct St<'a> {
        adj: &'a [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn go(s: &mut St, v: usize) {
        s.index[v] = Some(s.next);
        s.low[v] = s.next;
        s.next += 1;
        s.stack.push(v);
        s.on[v] = true;
        for &w in &s.adj[v].clone() {
            match s.index[w] {
                None => {
                    go(s, w);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(iw) if s.on[w] => s.low[v] = s.low[v].min(iw),
                _ => {}
            }
        }
        if Some(s.low[v]) == s.index[v] {
            let mut comp = Vec::new();
            while let Some(w) = s.stack.pop() {
                s.on[w] = false;
                comp.push(w);
                if w == v {
                    break;
                }
            }
            s.out.push(comp);
        }
    }
    let n = adj.len();
    let mut s = St { adj, index: vec![None; n], low: vec![0; n], on: vec![false; n], stack: Vec::new(), next: 0, out: Vec::new() };
    for v in 0..n {
        if s.index[v].is_none() {
            go(&mut s, v);
        }
    }
    s.out
}

/// The names a call reads in the same step: the arguments of delays and smoothing are state, not same-step reads.
fn same_step_refs(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Var(n) => out.push(n.clone()),
        Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => same_step_refs(a, out),
        Expr::Bin(_, a, b) => {
            same_step_refs(a, out);
            same_step_refs(b, out);
        }
        Expr::Call(f, args) => {
            if matches!(f.as_str(), "SMOOTH" | "SMOOTHI" | "SMOOTH3" | "SMOOTH3I" | "DELAY1" | "DELAY1I" | "DELAY3" | "DELAY3I" | "DELAY FIXED" | "DELAYINF" | "DELAYMTR" | "TREND" | "INITIAL" | "PREVIOUS" | "SAMPLE IF TRUE" | "QUEUE FIFO") {
                return;
            }
            args.iter().for_each(|a| same_step_refs(a, out));
        }
        Expr::Num(_) => {}
    }
}

fn g1(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let idx: HashMap<String, usize> = m.vars.iter().enumerate().map(|(i, v)| (expr::normalize(&v.name), i)).collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); m.vars.len()];
    for (i, v) in m.vars.iter().enumerate() {
        if v.kind == Kind::Stock {
            continue;
        }
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut refs = Vec::new();
        same_step_refs(&e, &mut refs);
        for r in refs {
            if let Some(&j) = idx.get(&r) {
                if m.vars[j].kind != Kind::Stock {
                    adj[i].push(j);
                }
            }
        }
    }
    let mut out = Vec::new();
    for comp in tarjan(&adj) {
        let self_loop = comp.len() == 1 && adj[comp[0]].contains(&comp[0]);
        if comp.len() < 2 && !self_loop {
            continue;
        }
        let mut names: Vec<String> = comp.iter().map(|&i| m.vars[i].name.clone()).collect();
        names.sort();
        let at = comp.iter().min().copied().unwrap();
        let v = &m.vars[at];
        let mut f = Finding::new("G1", Severity::Error, Some(&v.name), format!("Circular logic with no stock in between: {}. The values cannot be worked out", names.join(", ")));
        f.source = "Sterman (2000), §6.3: every feedback loop must contain a stock".into();
        f.fixes.push(Fix {
            title: format!("Put a smoothing delay of one time step into {}: SMOOTHI(…, TIME STEP, 0)", v.name),
            ops: vec![Op::SetEquation { name: v.name.clone(), equation: format!("SMOOTHI({}, TIME STEP, 0)", v.equation.trim()) }],
            keeps_numbers: false, needs_decision: false,
        });
        out.push(f);
    }
    out
}

fn g11(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let init = |v: &crate::model::Var| v.equation.trim().replace(',', "").parse::<f64>().ok();
    let stocks: Vec<&crate::model::Var> = m.vars.iter().filter(|v| v.kind == Kind::Stock).collect();
    let mut out = Vec::new();
    for t in &stocks {
        let n = t.name.to_lowercase();
        if !(n.contains("total") || n.split_whitespace().any(|w| w == "all")) {
            continue;
        }
        let Some(tv) = init(t).filter(|x| *x != 0.0) else { continue };
        let others: Vec<(&crate::model::Var, f64)> = stocks.iter().filter(|s| s.id != t.id).filter_map(|s| init(s).filter(|x| *x != 0.0).map(|x| (*s, x))).collect();
        // a subset sum of at most 4 others (the stocks a total normally adds)
        let k = others.len().min(12);
        let found = (1u32..(1 << k)).find(|mask| mask.count_ones() >= 2 && (0..k).filter(|b| mask >> b & 1 == 1).map(|b| others[b].1).sum::<f64>().sub_abs(tv) <= 1e-9);
        if let Some(mask) = found {
            let parts: Vec<&str> = (0..k).filter(|b| mask >> b & 1 == 1).map(|b| others[b].0.name.as_str()).collect();
            let mut f = Finding::new("G11", Severity::Warning, Some(&t.name), format!("“{}” starts as the sum of {} and is a stock itself: its flows would count the same material twice", t.name, parts.join(" + ")));
            f.source = "Sterman (2000), §6.1: a total is an auxiliary variable, not a stock".into();
            let flows: Vec<&crate::model::Var> = m.vars.iter().filter(|x| x.kind == Kind::Flow && (x.from == Some(t.id) || x.to == Some(t.id))).collect();
            let mut ops: Vec<Op> = flows.iter().map(|x| Op::RemoveVar { name: x.name.clone() }).collect();
            ops.push(Op::RemoveVar { name: t.name.clone() });
            ops.push(Op::AddVar { name: t.name.clone(), kind: "aux".into(), equation: parts.join(" + "), units: t.units.clone(), doc: t.comment.clone(), from: None, to: None, non_negative: false, layer: None });
            f.fixes.push(Fix { title: format!("Make “{}” an auxiliary: {}", t.name, parts.join(" + ")), ops, keeps_numbers: true, needs_decision: false });
            out.push(f);
        }
    }
    out
}

trait SubAbs {
    fn sub_abs(self, o: f64) -> f64;
}
impl SubAbs for f64 {
    fn sub_abs(self, o: f64) -> f64 {
        (self - o).abs()
    }
}

fn refs_of(v: &crate::model::Var) -> Vec<String> {
    let mut r = Vec::new();
    if let Ok(e) = expr::parse(&v.equation) {
        expr::references(&e, &mut r);
    }
    r.sort();
    r.dedup();
    r
}

fn g2(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for s in m.vars.iter().filter(|v| v.kind == Kind::Stock) {
        let refs: Vec<&crate::model::Var> = refs_of(s).into_iter().filter_map(|r| m.by_name(&r)).collect();
        // a stock reading itself has no start value
        let own = refs.iter().any(|x| x.id == s.id);
        // a start value that reads flows is an equilibrium start (Sterman 2000, ch. 4): the stock begins where the flows balance
        let flows: Vec<String> = refs.iter().filter(|x| x.kind == Kind::Flow).map(|x| x.name.clone()).collect();
        if own {
            let mut f = Finding::new("G2", Severity::Error, Some(&s.name), format!("The equation of the stock “{}” uses itself: a stock may use only its initial value, and changes only through its flows", s.name));
            f.source = "MIT D-4851 checklist, item 6".into();
            let flow = format!("{} adjustment", s.name);
            if m.by_name(&expr::normalize(&flow)).is_none() {
                f.fixes.push(Fix {
                    title: format!("Move the term into a new flow “{flow}” and start “{}” at 0", s.name),
                    ops: vec![
                        Op::AddVar { name: flow.clone(), kind: "flow".into(), equation: s.equation.trim().to_string(), units: String::new(), doc: format!("Was the equation of {}", s.name), from: None, to: Some(s.name.clone()), non_negative: false, layer: None },
                        Op::SetEquation { name: s.name.clone(), equation: "0".into() },
                    ],
                    keeps_numbers: false,
                    needs_decision: false,
                });
            }
            out.push(f);
        } else if !flows.is_empty() {
            let mut f = Finding::new("G2", Severity::Info, Some(&s.name), format!("The start value of the stock “{}” reads {}: an equilibrium start, where the stock begins at the value the flows balance at. Fine when the flows do not depend on the stock at the start", s.name, flows.join(", ")));
            f.source = "Sterman (2000), ch. 4 and MIT D-4851 checklist, item 6".into();
            out.push(f);
        }
    }
    out
}

fn g3(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    m.vars
        .iter()
        .filter(|v| v.kind == Kind::Flow && v.from.is_none() && v.to.is_none())
        .map(|v| {
            let mut f = Finding::new("G3", Severity::Warning, Some(&v.name), format!("The flow “{}” is attached to no stock", v.name));
            f.source = "MIT D-4851 checklist, item 7".into();
            f.fixes.push(Fix {
                title: format!("Make “{}” an auxiliary variable", v.name),
                ops: vec![
                    Op::RemoveVar { name: v.name.clone() },
                    Op::AddVar { name: v.name.clone(), kind: "aux".into(), equation: v.equation.clone(), units: v.units.clone(), doc: v.comment.clone(), from: None, to: None, non_negative: false, layer: None },
                ],
                keeps_numbers: true, needs_decision: false,
            });
            f
        })
        .collect()
}

/// Does the equation divide something with a stock in it by the flow `flow` (a normalised name)?
fn littles_law(m: &crate::model::Model, e: &Expr, flow: &str) -> bool {
    fn has_stock(m: &crate::model::Model, e: &Expr) -> bool {
        let mut r = Vec::new();
        expr::references(e, &mut r);
        r.iter().any(|n| m.by_name(n).is_some_and(|v| v.kind == Kind::Stock))
    }
    match e {
        Expr::Bin(BinOp::Div, a, b) => (matches!(&**b, Expr::Var(n) if n == flow) && has_stock(m, a)) || littles_law(m, a, flow) || littles_law(m, b, flow),
        Expr::Bin(_, a, b) => littles_law(m, a, flow) || littles_law(m, b, flow),
        Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => littles_law(m, a, flow),
        Expr::Call(_, args) => args.iter().any(|a| littles_law(m, a, flow)),
        _ => false,
    }
}

fn g4(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux)) {
        // a flow read through a measurement (SMOOTH, DELAY) is the cure, not the problem
        let mut same = Vec::new();
        if let Ok(e) = expr::parse(&v.equation) {
            same_step_refs(&e, &mut same);
        }
        let same: Vec<String> = same.into_iter().map(|n| expr::normalize(&n)).collect();
        for r in refs_of(v).into_iter().filter(|r| same.contains(r)) {
            let Some(fl) = m.by_name(&r).filter(|x| x.kind == Kind::Flow && (x.from.is_some() || x.to.is_some())) else { continue };
            // Little's law: a stock over a flow is the time the stuff stays, a ratio of two levels of the same system
            let little = expr::parse(&v.equation).is_ok_and(|e| littles_law(m, &e, &r));
            if little {
                let mut f = Finding::new("G4", Severity::Info, Some(&v.name), format!("“{}” divides a stock by the flow “{}” (Little's law: the time the stuff stays in the stock): fine as a result, but a decision should not read it as information at an instant", v.name, fl.name));
                f.source = "Little (1961), the law L = λW; MIT D-4851 checklist, item 8".into();
                out.push(f);
                break;
            }
            let mut f = Finding::new("G4", Severity::Warning, Some(&v.name), format!("“{}” reads the flow “{}” as information: a rate is not observable at an instant; use a measured or smoothed version", v.name, fl.name));
            f.source = "MIT D-4851 checklist, item 8".into();
            let tm = format!("{} Measurement Time", fl.name);
            let mut ops = Vec::new();
            if m.by_name(&expr::normalize(&tm)).is_none() {
                ops.push(Op::AddVar { name: tm.clone(), kind: "constant".into(), equation: super::fixes::clean_factor(4.0 * m.settings.time_step), units: m.settings.time_units.clone(), doc: format!("How long it takes to measure {}", fl.name), from: None, to: None, non_negative: false, layer: None });
            }
            ops.push(Op::SetEquation { name: v.name.clone(), equation: expr::rename_in(&v.equation, &r, &format!("SMOOTH({}, {tm})", fl.name)) });
            f.fixes.push(Fix { title: format!("Read a smoothed {}: SMOOTH({}, {tm})", fl.name, fl.name), ops, keeps_numbers: false, needs_decision: false });
            out.push(f);
            break;
        }
    }
    out
}

fn g5(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for fl in m.vars.iter().filter(|v| v.kind == Kind::Flow) {
        let Ok(Expr::Var(n)) = expr::parse(&fl.equation) else { continue };
        let Some(st) = m.by_name(&n).filter(|x| x.kind == Kind::Stock) else { continue };
        let mut f = Finding::new("G5", Severity::Error, Some(&fl.name), format!("The flow “{}” is the level of the stock “{}” itself: a stock cannot be a flow; the flow must come out of it at a rate", fl.name, st.name));
        f.source = "MIT D-4851 checklist, item 9".into();
        let t = format!("{} Drawdown Time", st.name);
        if m.by_name(&expr::normalize(&t)).is_none() {
            f.fixes.push(Fix {
                title: format!("Draw “{}” down over a time: {} / {t}", st.name, st.name),
                ops: vec![
                    Op::AddVar { name: t.clone(), kind: "constant".into(), equation: "1".into(), units: m.settings.time_units.clone(), doc: "How long the stock takes to flow out at its present rate".into(), from: None, to: None, non_negative: false, layer: None },
                    Op::SetEquation { name: fl.name.clone(), equation: format!("{} / {t}", st.name) },
                ],
                keeps_numbers: false, needs_decision: false,
            });
        }
        out.push(f);
    }
    out
}

fn g6(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    m.vars
        .iter()
        .filter(|v| v.kind == Kind::Stock && !m.vars.iter().any(|f| f.kind == Kind::Flow && (f.from == Some(v.id) || f.to == Some(v.id))))
        .map(|v| {
            let mut f = Finding::new("G6", Severity::Warning, Some(&v.name), format!("The stock “{}” has no flows, so it never changes", v.name));
            f.source = "Dynamist".into();
            f.fixes.push(Fix {
                title: format!("Make “{}” a constant", v.name),
                ops: vec![
                    Op::RemoveVar { name: v.name.clone() },
                    Op::AddVar { name: v.name.clone(), kind: "constant".into(), equation: v.equation.clone(), units: v.units.clone(), doc: v.comment.clone(), from: None, to: None, non_negative: false, layer: None },
                ],
                keeps_numbers: true, needs_decision: false,
            });
            f
        })
        .collect()
}

/// The ids of the variables that count as results: manifest outputs, the objectives' KPIs, the variables of checks.
fn kpis(m: &crate::model::Model) -> Vec<u64> {
    let mut names: Vec<String> = m.manifest.outputs.iter().map(|o| o.variable.clone()).collect();
    if let Some(p) = &m.purpose {
        names.extend(p.objectives.iter().map(|o| o.var.clone()));
    }
    names.extend(m.checks.iter().map(|c| c.var.clone()));
    names.iter().filter(|n| !n.is_empty()).filter_map(|n| m.by_name(&expr::normalize(n)).map(|v| v.id)).collect()
}

/// users[i]: the variables whose equations read variable i (flows also feed the stocks they are attached to).
pub(crate) fn users(m: &crate::model::Model) -> Vec<Vec<usize>> {
    let idx: HashMap<String, usize> = m.vars.iter().enumerate().map(|(i, v)| (expr::normalize(&v.name), i)).collect();
    let by_id: HashMap<u64, usize> = m.vars.iter().enumerate().map(|(i, v)| (v.id, i)).collect();
    let mut u = vec![Vec::new(); m.vars.len()];
    for (i, v) in m.vars.iter().enumerate() {
        for r in refs_of(v) {
            if let Some(&j) = idx.get(&r) {
                u[j].push(i);
            }
        }
        if v.kind == Kind::Flow {
            for id in [v.from, v.to].into_iter().flatten() {
                if let Some(&j) = by_id.get(&id) {
                    u[i].push(j);
                }
            }
        }
    }
    u
}

fn g7(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let u = users(m);
    let k = kpis(m);
    let live = |start: usize| -> bool {
        let mut seen = vec![false; m.vars.len()];
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut seen[i], true) {
                continue;
            }
            if m.vars[i].kind == Kind::Stock || k.contains(&m.vars[i].id) {
                return true;
            }
            stack.extend(u[i].iter().copied());
        }
        false
    };
    let mut out = Vec::new();
    for (i, v) in m.vars.iter().enumerate() {
        if matches!(v.kind, Kind::Stock) || live(i) {
            continue;
        }
        // with no outputs or KPIs declared, a variable that nobody reads is an output of the model: a note, not a defect
        let declared = !k.is_empty();
        let mut f = if declared {
            Finding::new("G7", Severity::Warning, Some(&v.name), format!("“{}” affects no stock, output or KPI", v.name))
        } else {
            Finding::new("G7", Severity::Info, Some(&v.name), format!("“{}” affects no stock: no outputs or KPIs are declared, so read it as an output of the model (declare the outputs to have unused variables reported)", v.name))
        };
        f.source = "SDM-Doc (Martinez-Moyano 2012): no orphans".into();
        // a variable nobody reads is the end of a chain, never something to delete on a rule's word
        if declared && !u[i].is_empty() {
            f.fixes.push(Fix { title: format!("Remove “{}”", v.name), ops: vec![Op::RemoveVar { name: v.name.clone() }], keeps_numbers: true, needs_decision: false });
        }
        out.push(f);
    }
    out
}

fn g8(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in m.vars.iter().filter(|v| matches!(v.kind, Kind::Aux | Kind::Flow)) {
        let r = refs_of(v);
        let inputs = r.iter().filter(|n| !matches!(n.as_str(), "time" | "time step" | "initial time" | "final time")).count();
        if inputs <= 3 {
            continue;
        }
        let mut f = Finding::new("G8", Severity::Info, Some(&v.name), format!("The equation of “{}” has {inputs} inputs: split it into named intermediate variables", v.name));
        f.source = "Rahmandad, MIT 15.879: keep equations short".into();
        if let Ok(Expr::Bin(op, a, b)) = expr::parse(&v.equation) {
            let sym = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                _ => "",
            };
            let part = format!("{} part", v.name);
            // the engine forms of LEVELISED, LOAN and the like cannot be written back as the modeller wrote them
            let mut has_verb = false;
            fn scan(e: &Expr, f: &mut bool) {
                match e {
                    Expr::Call(n, a) => {
                        if n.ends_with(" STATE") || n == "NPV AT" {
                            *f = true;
                        }
                        a.iter().for_each(|x| scan(x, f));
                    }
                    Expr::Bin(_, a, b) => {
                        scan(a, f);
                        scan(b, f);
                    }
                    Expr::Neg(a) | Expr::Not(a) | Expr::LookupCall(_, a) | Expr::WithLookup(a, _) => scan(a, f),
                    _ => {}
                }
            }
            if let Ok(whole) = expr::parse(&v.equation) {
                scan(&whole, &mut has_verb);
            }
            if !sym.is_empty() && !has_verb && m.by_name(&expr::normalize(&part)).is_none() {
                let (pa, pb) = (super::base::expr_text(&a, m), super::base::expr_text(&b, m));
                let (head, tail) = if refs_of_expr(&a) >= refs_of_expr(&b) { (pa, format!("{part} {sym} {pb}")) } else { (pb, format!("{pa} {sym} {part}")) };
                let head_refs: Vec<String> = { let mut r = Vec::new(); if let Ok(h) = expr::parse(&head) { expr::references(&h, &mut r); } r };
                let tail_refs: Vec<String> = { let mut r = Vec::new(); if let Ok(t) = expr::parse(&tail) { expr::references(&t, &mut r); } r };
                let mut ops = vec![
                    Op::AddVar { name: part.clone(), kind: "aux".into(), equation: head, units: v.units.clone(), doc: format!("A part of {}", v.name), from: None, to: None, non_negative: false, layer: None },
                    Op::SetEquation { name: v.name.clone(), equation: tail },
                ];
                // the arrows follow the equations: the moved inputs now reach the part, and the part reaches the variable
                for r in head_refs.iter().filter(|r| !tail_refs.contains(r)) {
                    if let Some(src) = m.by_name(r) {
                        if m.links.iter().any(|l| l.from == src.id && l.to == v.id) {
                            ops.push(Op::Disconnect { from: src.name.clone(), to: v.name.clone() });
                        }
                    }
                }
                f.fixes.push(Fix { title: format!("Name the larger part “{part}”"), ops, keeps_numbers: true, needs_decision: false });
            }
        }
        out.push(f);
    }
    out
}

fn refs_of_expr(e: &Expr) -> usize {
    let mut r = Vec::new();
    expr::references(e, &mut r);
    r.sort();
    r.dedup();
    r.len()
}

fn g9(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut out = Vec::new();
    for v in &m.vars {
        let refs = refs_of(v);
        for l in m.links.iter().filter(|l| l.to == v.id) {
            let Some(s) = m.var(l.from) else { continue };
            if !refs.contains(&expr::normalize(&s.name)) && !(v.kind == Kind::Lookup && v.equation.trim().is_empty()) && v.kind != Kind::Flow {
                let mut f = Finding::new("G9", Severity::Warning, Some(&v.name), format!("There is an arrow from “{}” to “{}”, but the equation does not use it", s.name, v.name));
                f.source = "Dynamist".into();
                f.fixes.push(Fix { title: format!("Remove the arrow from {} to {}", s.name, v.name), ops: vec![Op::Disconnect { from: s.name.clone(), to: v.name.clone() }], keeps_numbers: true, needs_decision: false });
                out.push(f);
            }
        }
        for r in &refs {
            if let Some(s) = m.by_name(r) {
                if s.id != v.id && !m.links.iter().any(|l| l.from == s.id && l.to == v.id) {
                    let mut f = Finding::new("G9", Severity::Warning, Some(&v.name), format!("“{}” uses “{}” but there is no arrow between them", v.name, s.name));
                    f.source = "Dynamist".into();
                    f.fixes.push(Fix { title: format!("Draw the arrow from {} to {}", s.name, v.name), ops: vec![Op::Connect { from: s.name.clone(), to: v.name.clone(), polarity: String::new() }], keeps_numbers: true, needs_decision: false });
                    out.push(f);
                }
            }
        }
    }
    out
}

fn g10(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let k = kpis(m);
    if k.is_empty() {
        return Vec::new();
    }
    // weakly connected components of the dependency graph; a component with no KPI is disconnected from the purpose
    let u = users(m);
    let n = m.vars.len();
    let mut adj = vec![Vec::new(); n];
    for (i, us) in u.iter().enumerate() {
        for &j in us {
            adj[i].push(j);
            adj[j].push(i);
        }
    }
    let mut comp = vec![usize::MAX; n];
    let mut c = 0;
    for s in 0..n {
        if comp[s] != usize::MAX {
            continue;
        }
        let mut st = vec![s];
        while let Some(i) = st.pop() {
            if comp[i] != usize::MAX {
                continue;
            }
            comp[i] = c;
            st.extend(adj[i].iter().copied());
        }
        c += 1;
    }
    let kc: Vec<usize> = m.vars.iter().enumerate().filter(|(_, v)| k.contains(&v.id)).map(|(i, _)| comp[i]).collect();
    let mut out = Vec::new();
    for cc in 0..c {
        if kc.contains(&cc) {
            continue;
        }
        let members: Vec<&str> = (0..n).filter(|i| comp[*i] == cc).map(|i| m.vars[i].name.as_str()).collect();
        let mut f = Finding::new("G10", Severity::Warning, Some(members[0]), format!("{} are not connected to any KPI of the purpose", members.join(", ")));
        f.source = "Dynamist architecture specification".into();
        f.fixes.push(Fix { title: format!("Remove the disconnected part ({})", members.join(", ")), ops: members.iter().map(|n| Op::RemoveVar { name: n.to_string() }).collect(), keeps_numbers: true, needs_decision: false });
        out.push(f);
    }
    out
}

fn g12(ctx: &Ctx) -> Vec<Finding> {
    let m = ctx.model;
    let mut pairs: Vec<(String, String, String, String)> = Vec::new();
    for sh in &m.sheets {
        for l in &sh.cld.links {
            let (Some(a), Some(b)) = (sh.cld.nodes.iter().find(|n| n.id == l.from), sh.cld.nodes.iter().find(|n| n.id == l.to)) else { continue };
            if (l.polarity == "+" || l.polarity == "-") && m.by_name(&expr::normalize(&a.text)).is_some() && m.by_name(&expr::normalize(&b.text)).is_some() {
                pairs.push((sh.name.clone(), a.text.clone(), b.text.clone(), l.polarity.clone()));
            }
        }
    }
    if pairs.is_empty() {
        return Vec::new();
    }
    let probe: Vec<(String, String)> = pairs.iter().map(|p| (p.1.clone(), p.2.clone())).collect();
    let Ok((_, signs)) = crate::sim::link_signs(m, &probe) else { return Vec::new() };
    let mut out = Vec::new();
    for ((sheet, a, b, drawn), s) in pairs.iter().zip(signs) {
        let pos = s.iter().filter(|x| **x > 0).count();
        let neg = s.iter().filter(|x| **x < 0).count();
        let eq_sign = match (pos > 0, neg > 0) {
            (true, false) => "+",
            (false, true) => "-",
            _ => continue,
        };
        if eq_sign != drawn {
            let mut f = Finding::new("G12", Severity::Warning, Some(b), format!("On the sheet “{sheet}” the arrow from “{a}” to “{b}” is drawn {drawn}, but the equations give {eq_sign}"));
            f.source = "Dynamist loops".into();
            f.fixes.push(Fix { title: format!("Draw the arrow from {a} to {b} as {eq_sign}"), ops: vec![Op::SetCldPolarity { from: a.clone(), to: b.clone(), polarity: eq_sign.into(), sheet: Some(sheet.clone()) }], keeps_numbers: true, needs_decision: false });
            out.push(f);
        }
    }
    out
}
