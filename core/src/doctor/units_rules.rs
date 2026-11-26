//! The units rules of the doctor, on the exact units engine (`units::Unit`).
//!
//! * Sums, differences, comparisons and `MIN`/`MAX`/`IF THEN ELSE` branches need equal dimensions and equal scale.
//! * A flow times the time unit must give its stock's unit, scale included; a flow without declared units is expected
//!   in `stock unit / time unit` (the net flow the importers make).
//! * A rate meeting an amount is an error (the time unit is not ignored) unless `lenient_time` is set.
//! * A percent or ppm unit on a literal constant is a scale of the value: with `units_scale_values` on, "5 ~ %/Day" is
//!   0.05 per Day, and the doctor says so (`unit-percent`); with it off the doctor lists where turning it on would
//!   change a value (`unit-scale-change`).
//!
//! Codes: unit-parse, unit-ambiguous, unit-scale-mismatch, unit-dimension-mismatch, unit-declared, unit-flow,
//! unit-percent, unit-scale-change, units-inferred, money-real-nominal.

use std::collections::HashMap;

use super::fixes;
use super::{Finding, Fix, Options, Severity};
use crate::expr::{self, BinOp, Expr};
use crate::model::{Kind, Model, Var};
use crate::ops::Op;
use crate::units::{self, Unit, UnitCtx};

#[derive(Clone, Debug)]
pub(crate) enum U {
    Known(Unit),
    /// A literal: fits beside anything.
    Num,
    Any,
}

/// One place where two units meet and disagree.
#[derive(Clone, Debug)]
pub(crate) struct Clash {
    pub what: &'static str,
    pub left: Unit,
    pub right: Unit,
    /// the right-hand side was a bare variable of this name (normalised)
    pub right_var: Option<String>,
    pub left_var: Option<String>,
}

pub(crate) struct Engine<'a> {
    pub model: &'a Model,
    pub ctx: UnitCtx,
    pub time: Option<Unit>,
    pub scale_values: bool,
    pub lenient_time: bool,
    /// normalised variable name → its unit as other equations see it
    pub units: HashMap<String, U>,
}

fn is_literal(e: &str) -> bool {
    matches!(expr::parse(e), Ok(Expr::Num(_))) || matches!(expr::parse(e), Ok(Expr::Neg(ref a)) if matches!(**a, Expr::Num(_)))
}

impl<'a> Engine<'a> {
    pub fn new(model: &'a Model, opts: &Options) -> Engine<'a> {
        let ctx = UnitCtx::of(model);
        let time = units::parse_unit_in(&model.settings.time_units, &ctx).ok().filter(|u| u.dims.len() == 1 && u.dims.contains_key("s"));
        let mut e = Engine { model, ctx, time, scale_values: model.settings.units_scale_values, lenient_time: opts.lenient_time, units: HashMap::new() };
        for v in &model.vars {
            let u = e.declared(v);
            e.units.insert(expr::normalize(&v.name), u);
        }
        e
    }

    /// The unit of a variable as other equations see it.
    pub fn declared(&self, v: &Var) -> U {
        if v.units.trim().is_empty() {
            return U::Any;
        }
        match units::parse_unit_in(&v.units, &self.ctx) {
            Ok(u) => {
                // a literal in percent or ppm is a number scaled by it: its value is already in the plain unit
                if self.scale_values && u.pure_scale != 1.0 && is_literal(&v.equation) {
                    U::Known(u.without_pure_scale())
                } else {
                    U::Known(u)
                }
            }
            Err(_) => U::Any,
        }
    }

    fn strip_time(&self, u: &Unit) -> Unit {
        let Some(t) = &self.time else { return u.clone() };
        let n = u.dims.get("s").copied().unwrap_or(0);
        let mut r = u.clone();
        r.dims.remove("s");
        r.scale /= t.scale.powi(n);
        r
    }

    pub fn equal(&self, a: &Unit, b: &Unit) -> bool {
        if self.lenient_time { self.strip_time(a) == self.strip_time(b) } else { a == b }
    }

    fn join(&self, a: U, b: U, what: &'static str, lv: Option<String>, rv: Option<String>, found: &mut Vec<Clash>) -> U {
        match (a, b) {
            (U::Known(x), U::Known(y)) => {
                if !self.equal(&x, &y) {
                    found.push(Clash { what, left: x.clone(), right: y, right_var: rv, left_var: lv });
                } else if x.real_year != y.real_year && x.dims.keys().any(|k| k.starts_with("currency:")) {
                    // real and nominal money (or two price years) added together
                    found.push(Clash { what: "money", left: x.clone(), right: y, right_var: rv, left_var: lv });
                }
                U::Known(x)
            }
            (U::Known(x), _) | (_, U::Known(x)) => U::Known(x),
            (U::Num, U::Num) => U::Num,
            _ => U::Any,
        }
    }

    /// An exponent must be a number without units: `(1 - d)^Time` with Time in years is not.
    fn exponent_check(&self, y: &U, b: &Expr, found: &mut Vec<Clash>) {
        if let U::Known(q) = y {
            if !q.is_dimensionless() {
                let right_var = if let Expr::Var(n) = b { Some(n.clone()) } else { None };
                found.push(Clash { what: "exponent", left: q.clone(), right: Unit::one(), right_var, left_var: None });
            }
        }
    }

    pub fn infer(&self, e: &Expr, found: &mut Vec<Clash>) -> U {
        let var_name = |x: &Expr| -> Option<String> { if let Expr::Var(n) = x { Some(n.clone()) } else { None } };
        match e {
            Expr::Num(_) => U::Num,
            Expr::Var(n) => {
                if matches!(n.as_str(), "time" | "time step" | "initial time" | "final time" | "saveper") {
                    return self.time.clone().map(U::Known).unwrap_or(U::Any);
                }
                self.units.get(n).cloned().unwrap_or(U::Any)
            }
            Expr::Neg(a) => self.infer(a, found),
            Expr::Not(a) => {
                self.infer(a, found);
                U::Known(Unit::one())
            }
            Expr::Bin(op, a, b) => {
                let (x, y) = (self.infer(a, found), self.infer(b, found));
                match op {
                    BinOp::Add => self.join(x, y, "adds", var_name(a), var_name(b), found),
                    BinOp::Sub => self.join(x, y, "subtracts", var_name(a), var_name(b), found),
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne => {
                        self.join(x, y, "compares", var_name(a), var_name(b), found);
                        U::Known(Unit::one())
                    }
                    BinOp::And | BinOp::Or => U::Known(Unit::one()),
                    BinOp::Mul => match (x, y) {
                        (U::Known(p), U::Known(q)) => U::Known(p.mul(&q)),
                        (U::Known(p), U::Num) | (U::Num, U::Known(p)) => U::Known(p),
                        (U::Num, U::Num) => U::Num,
                        _ => U::Any,
                    },
                    BinOp::Div => match (x, y) {
                        (U::Known(p), U::Known(q)) => U::Known(p.div(&q)),
                        (U::Known(p), U::Num) => U::Known(p),
                        (U::Num, U::Known(q)) => U::Known(Unit::one().div(&q)),
                        (U::Num, U::Num) => U::Num,
                        _ => U::Any,
                    },
                    BinOp::Pow => {
                        self.exponent_check(&y, b, found);
                        match (x, &**b) {
                        (U::Known(p), Expr::Num(n)) if n.fract() == 0.0 => U::Known(p.powi(*n as i32)),
                        (U::Known(p), _) if p.is_dimensionless() => U::Known(p),
                        _ => U::Any,
                        }
                    }
                }
            }
            Expr::LookupCall(name, a) => {
                self.infer(a, found);
                self.units.get(name).cloned().unwrap_or(U::Any)
            }
            Expr::WithLookup(a, _) => {
                self.infer(a, found);
                U::Any
            }
            Expr::Call(f, args) => {
                let ds: Vec<U> = args.iter().map(|a| self.infer(a, found)).collect();
                let first = ds.first().cloned().unwrap_or(U::Any);
                match f.as_str() {
                    "POWER" if ds.len() == 2 => {
                        self.exponent_check(&ds[1], &args[1], found);
                        U::Any
                    }
                    "EXP" if ds.len() == 1 => {
                        self.exponent_check(&ds[0], &args[0], found);
                        U::Known(Unit::one())
                    }
                    "MIN" | "MAX" => {
                        let mut it = ds.into_iter().zip(args.iter());
                        let Some((mut acc, _)) = it.next() else { return U::Any };
                        for (d, a) in it {
                            acc = self.join(acc, d, "compares", None, var_name(a), found);
                        }
                        acc
                    }
                    "IF THEN ELSE" if ds.len() == 3 => self.join(ds[1].clone(), ds[2].clone(), "chooses between", var_name(&args[1]), var_name(&args[2]), found),
                    "SMOOTH" | "SMOOTHI" | "SMOOTH3" | "SMOOTH3I" | "DELAY1" | "DELAY1I" | "DELAY3" | "DELAY3I" | "DELAY FIXED" | "DELAYINF" | "DELAYMTR" | "ABS" | "ROUND" | "INTEGER" | "CEILING" | "CLAMP" | "TOTAL" | "AVERAGE" | "MEAN" | "MEDIAN" | "STEP" | "DELAY" => first,
                    "ZIDZ" | "XIDZ" if ds.len() >= 2 => match (&ds[0], &ds[1]) {
                        (U::Known(p), U::Known(q)) => U::Known(p.div(q)),
                        (U::Known(p), U::Num) => U::Known(p.clone()),
                        _ => U::Any,
                    },
                    // the fractional rate of change of the input: per time unit
                    "TREND" => match &self.time {
                        Some(t) => U::Known(Unit::one().div(t)),
                        None => U::Any,
                    },
                    "SQRT" => match first {
                        U::Known(p) if p.dims.values().all(|e| e % 2 == 0) => {
                            U::Known(Unit { dims: p.dims.iter().map(|(k, v)| (k.clone(), v / 2)).collect(), scale: p.scale.sqrt(), pure_scale: p.pure_scale.sqrt(), ..p.clone() })
                        }
                        _ => U::Any,
                    },
                    "PMT" | "FV" | "PV" if ds.len() > 2 => ds[2].clone(),
                    "IPMT" | "PPMT" if ds.len() > 3 => ds[3].clone(),
                    "REAL RATE" | "NOMINAL RATE" | "EFFECTIVE RATE" | "WACC" | "ANNUITY FACTOR" | "NPER" | "RATE" | "DSCR" => U::Known(Unit::one()),
                    "LN" | "LOG10" | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ATAN" | "SINH" | "COSH" | "TANH" => U::Known(Unit::one()),
                    "RAMP" => match (first, &self.time) {
                        (U::Known(p), Some(t)) => U::Known(p.mul(t)),
                        _ => U::Any,
                    },
                    _ => U::Any,
                }
            }
        }
    }
}

/// The additive terms of an equation: (sign, term), through brackets and unary minus.
pub(crate) fn terms(e: &Expr) -> Vec<(i32, &Expr)> {
    fn go<'a>(e: &'a Expr, sign: i32, out: &mut Vec<(i32, &'a Expr)>) {
        match e {
            Expr::Bin(BinOp::Add, a, b) => {
                go(a, sign, out);
                go(b, sign, out);
            }
            Expr::Bin(BinOp::Sub, a, b) => {
                go(a, sign, out);
                go(b, -sign, out);
            }
            Expr::Neg(a) => go(a, -sign, out),
            other => out.push((sign, other)),
        }
    }
    let mut v = Vec::new();
    go(e, 1, &mut v);
    v
}

/// The variable names (as written in the model) of a unit's textual form: "MB/Day" → ("MB", Some("Day")).
pub fn split_text(u: &str) -> (String, Option<String>) {
    let u = u.trim();
    let mut depth = 0;
    for (i, c) in u.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '/' if depth == 0 => return (u[..i].trim().to_string(), Some(u[i + 1..].trim().to_string())),
            _ => {}
        }
    }
    (u.to_string(), None)
}

fn model_var<'a>(m: &'a Model, norm: &str) -> Option<&'a Var> {
    m.by_name(norm)
}

/// The unit a flow's equation is expected in: its declared unit, else its stock's unit per time unit (as text and parsed).
fn expected_flow_unit(en: &Engine, v: &Var) -> Option<(Unit, String)> {
    if !v.units.trim().is_empty() {
        return units::parse_unit_in(&v.units, &en.ctx).ok().map(|u| (u, v.units.trim().to_string()));
    }
    let stock = [v.to, v.from].into_iter().flatten().filter_map(|id| en.model.var(id)).find(|s| s.kind == Kind::Stock && !s.units.trim().is_empty())?;
    let su = units::parse_unit_in(&stock.units, &en.ctx).ok()?;
    let t = en.time.clone()?;
    let (num, den) = split_text(&stock.units);
    let ttext = en.model.settings.time_units.trim();
    let text = match den {
        None => format!("{}/{}", num, ttext),
        Some(d) => format!("{}/({}*{})", num, d, ttext),
    };
    Some((su.div(&t), text))
}

pub fn check(m: &Model, opts: &Options) -> Vec<Finding> {
    let en = Engine::new(m, opts);
    let mut out: Vec<Finding> = Vec::new();
    let mut ambiguous_seen: Vec<String> = Vec::new();
    for v in &m.vars {
        // 1. the declaration itself
        if !v.units.trim().is_empty() {
            match units::parse_unit_in(&v.units, &en.ctx) {
                Err(e) => {
                    let mut f = Finding::new("unit-parse", Severity::Warning, Some(&v.name), format!("“{}”: the units “{}” cannot be read: {}", v.name, v.units.trim(), e));
                    if let Some(s) = &e.suggestion {
                        f.fixes.push(Fix { title: format!("Write the units of “{}” as “{}”", v.name, s), ops: vec![Op::SetUnits { name: v.name.clone(), units: v.units.replace(&bad_word(&v.units, &e), s) }], keeps_numbers: true, needs_decision: false });
                    }
                    f.source = "Dynamist units rules".into();
                    out.push(f);
                }
                Ok(u) => {
                    for sym in &u.ambiguous {
                        if ambiguous_seen.contains(sym) {
                            continue;
                        }
                        ambiguous_seen.push(sym.clone());
                        out.push(fixes::ambiguous(sym, &v.name));
                    }
                    // the percent reading, said out loud
                    if u.pure_scale != 1.0 && is_literal(&v.equation) && v.kind == Kind::Constant {
                        let val = expr::parse(&v.equation).ok().map(|e| literal(&e)).unwrap_or(f64::NAN);
                        if en.scale_values {
                            out.push(Finding::new(
                                "unit-percent",
                                Severity::Info,
                                Some(&v.name),
                                format!("“{}” is declared in {}: its value {} is read as {} {}", v.name, v.units.trim(), super::num(val), super::num(val * u.pure_scale), plain_text(&v.units)),
                            ));
                        } else {
                            out.push(fixes::scale_change(&v.name, val, u.pure_scale, &v.units));
                        }
                    }
                }
            }
        }
        // 2. the equation against its declaration and its stock
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut clashes = Vec::new();
        let got = en.infer(&e, &mut clashes);
        // a flow's terms are held to the flow's expected unit; everything else only to itself
        let expected = if v.kind == Kind::Flow { expected_flow_unit(&en, v) } else { None };
        let mut reported: Vec<String> = Vec::new();
        if let Some((exp, exp_text)) = &expected {
            for (_, t) in terms(&e) {
                let mut sub = Vec::new();
                if let U::Known(tu) = en.infer(t, &mut sub) {
                    if !en.equal(&tu, exp) {
                        out.push(fixes::term_mismatch(m, v, t, &tu, exp, exp_text, &en));
                        reported.push(term_key(t));
                    }
                }
            }
        }
        for c in clashes {
            // a clash already reported as a flow term is not reported twice
            if let (Some(_), Some(rv)) = (&expected, &c.right_var) {
                if reported.contains(rv) {
                    continue;
                }
            }
            if expected.is_some() && c.left_var.as_ref().is_some_and(|l| reported.contains(l)) {
                continue;
            }
            out.push(fixes::clash(m, v, &c, &en));
        }
        // declared against got (a stock's equation is its start value; a flow's is held to `expected` above)
        if v.kind != Kind::Flow || !v.units.trim().is_empty() {
            if let (U::Known(g), U::Known(d)) = (&got, en.declared(v)) {
                if !matches!(v.kind, Kind::Lookup) && !en.equal(g, &d) {
                    out.push(fixes::declared_mismatch(m, v, g, &d, &en));
                }
            }
        }
        // a flow, per time unit, must be in its stock's unit
        if v.kind == Kind::Flow && !v.units.trim().is_empty() {
            if let (Ok(fu), Some(t)) = (units::parse_unit_in(&v.units, &en.ctx), en.time.clone()) {
                for sid in [v.from, v.to].into_iter().flatten() {
                    let Some(s) = m.var(sid) else { continue };
                    if s.kind != Kind::Stock {
                        continue;
                    }
                    if let U::Known(su) = en.declared(s) {
                        let per = if en.lenient_time { fu.clone() } else { fu.mul(&t) };
                        let ok = if en.lenient_time { en.equal(&per, &su) } else { per == su };
                        if !ok {
                            out.push(fixes::flow_stock(m, v, s, &per, &su, &en));
                        }
                    }
                }
            }
        }
    }
    out
}

fn literal(e: &Expr) -> f64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Neg(a) => -literal(a),
        _ => f64::NAN,
    }
}

/// "%/Day" → "/Day" (what follows the percent sign), for the sentence "read as 0.05 per Day".
fn plain_text(units: &str) -> String {
    let (num, den) = split_text(units);
    let n = num.trim_start_matches('%').trim();
    let n = if num.trim() == "%" || num.trim().eq_ignore_ascii_case("percent") { "".to_string() } else { n.to_string() };
    match den {
        Some(d) if n.is_empty() => format!("per {d}"),
        Some(d) => format!("{n} per {d}"),
        None => n,
    }
}

fn term_key(t: &Expr) -> String {
    match t {
        Expr::Var(n) => n.clone(),
        other => format!("{other:?}"),
    }
}

/// The word of a unit text that the parser complained about (its position is a character offset in the trimmed text).
fn bad_word(text: &str, e: &units::UnitParseError) -> String {
    let t = text.trim();
    let chars: Vec<char> = t.chars().collect();
    let start = e.pos.min(chars.len());
    let rest: String = chars[start..].iter().collect();
    rest.chars().take_while(|c| !matches!(c, '*' | '/' | '(' | ')' | '^')).collect::<String>().trim().to_string()
}
