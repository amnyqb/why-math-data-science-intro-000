//! Stage C3: interval analysis. Each variable carries an interval [lo, hi] that its value cannot leave, found by evaluating
//! the equations over intervals in dependency order, to a fixed point (after 5 passes any bound that still moves is widened
//! to infinity, so the analysis always ends).
//!
//! Initial facts: a constant is its value, or its slider range; a stock is [0, +inf) if it is declared never negative and
//! (-inf, +inf) otherwise; a lookup variable is [min y, max y]; Time is [initial, final]. A division by an interval that
//! contains 0, a logarithm of an interval reaching 0 or less, and a lookup input outside the table are recorded as
//! "can" facts; the rules turn them into findings that the base run or an experiment may confirm.

use std::collections::HashMap;

use crate::expr::{self, BinOp, Expr};
use crate::model::{Kind, Model};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iv {
    pub lo: f64,
    pub hi: f64,
}

pub const ALL: Iv = Iv { lo: f64::NEG_INFINITY, hi: f64::INFINITY };

impl Iv {
    pub fn point(x: f64) -> Iv {
        Iv { lo: x, hi: x }
    }
    pub fn new(a: f64, b: f64) -> Iv {
        Iv { lo: a.min(b), hi: a.max(b) }
    }
    pub fn is_finite(&self) -> bool {
        self.lo.is_finite() && self.hi.is_finite()
    }
    pub fn contains_zero(&self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
    }
    pub fn hull(self, o: Iv) -> Iv {
        Iv { lo: self.lo.min(o.lo), hi: self.hi.max(o.hi) }
    }
    fn add(self, o: Iv) -> Iv {
        Iv { lo: self.lo + o.lo, hi: self.hi + o.hi }
    }
    fn sub(self, o: Iv) -> Iv {
        Iv { lo: self.lo - o.hi, hi: self.hi - o.lo }
    }
    fn neg(self) -> Iv {
        Iv { lo: -self.hi, hi: -self.lo }
    }
    fn mul(self, o: Iv) -> Iv {
        // 0 * inf is 0 here: a bound that is exactly zero stays zero
        let m = |a: f64, b: f64| if a == 0.0 || b == 0.0 { 0.0 } else { a * b };
        let p = [m(self.lo, o.lo), m(self.lo, o.hi), m(self.hi, o.lo), m(self.hi, o.hi)];
        Iv { lo: p.iter().cloned().fold(f64::INFINITY, f64::min), hi: p.iter().cloned().fold(f64::NEG_INFINITY, f64::max) }
    }
    /// None when the divisor contains zero.
    fn div(self, o: Iv) -> Option<Iv> {
        if o.contains_zero() {
            return None;
        }
        let inv = Iv::new(1.0 / o.lo, 1.0 / o.hi);
        Some(self.mul(inv))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Facts {
    /// normalised variable name → interval
    pub iv: HashMap<String, Iv>,
    /// (owner variable, text of the divisor, its interval)
    pub div_zero: Vec<(String, String, Iv)>,
    /// (owner variable, text of the argument, its interval)
    pub ln_bad: Vec<(String, String, Iv)>,
    /// (owner variable, lookup name, input interval, x range of the table)
    pub lookup_over: Vec<(String, String, Iv, (f64, f64))>,
}

struct Walker<'a> {
    m: &'a Model,
    iv: &'a HashMap<String, Iv>,
    owner: String,
    t: Iv,
    facts: Option<&'a mut Facts>,
}

fn y_range(table: &[[f64; 2]]) -> Iv {
    Iv { lo: table.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min), hi: table.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max) }
}

fn x_range(table: &[[f64; 2]]) -> (f64, f64) {
    (table.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min), table.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max))
}

/// The exact value of an expression made of numbers only (None if it has a variable or leaves the domain of a function).
fn const_eval(e: &Expr) -> Option<f64> {
    let v = match e {
        Expr::Num(n) => *n,
        Expr::Neg(a) => -const_eval(a)?,
        Expr::Bin(op, a, b) => {
            let (x, y) = (const_eval(a)?, const_eval(b)?);
            match op {
                BinOp::Add => x + y,
                BinOp::Sub => x - y,
                BinOp::Mul => x * y,
                BinOp::Div => x / y,
                BinOp::Pow => x.powf(y),
                _ => return None,
            }
        }
        Expr::Call(f, args) => {
            let a: Vec<f64> = args.iter().map(const_eval).collect::<Option<_>>()?;
            match (f.as_str(), a.as_slice()) {
                ("ABS", [x]) => x.abs(),
                ("SQRT", [x]) if *x >= 0.0 => x.sqrt(),
                ("EXP", [x]) => x.exp(),
                ("LN", [x]) if *x > 0.0 => x.ln(),
                ("POWER", [x, y]) => x.powf(*y),
                ("MIN", [x, y]) => x.min(*y),
                ("MAX", [x, y]) => x.max(*y),
                _ => return None,
            }
        }
        _ => return None,
    };
    v.is_finite().then_some(v)
}

/// x raised to y: positive when the base is above zero, whatever the exponent.
fn pow_iv(x: Iv, y: Iv) -> Iv {
    if x.lo >= 0.0 && y.is_finite() && x.is_finite() {
        let c = [x.lo.powf(y.lo), x.lo.powf(y.hi), x.hi.powf(y.lo), x.hi.powf(y.hi)];
        let lo = c.iter().cloned().fold(f64::INFINITY, f64::min);
        // a base strictly above zero gives a strictly positive power, even where the float underflows
        Iv { lo: if x.lo > 0.0 { lo.max(f64::MIN_POSITIVE) } else { lo }, hi: c.iter().cloned().fold(f64::NEG_INFINITY, f64::max) }
    } else if x.lo > 0.0 {
        Iv { lo: f64::MIN_POSITIVE, hi: f64::INFINITY }
    } else {
        ALL
    }
}

impl Walker<'_> {
    fn eval(&mut self, e: &Expr) -> Iv {
        // an expression of numbers only has one value, exactly
        if !matches!(e, Expr::Num(_)) {
            if let Some(v) = const_eval(e) {
                return Iv::point(v);
            }
        }
        match e {
            Expr::Num(n) => Iv::point(*n),
            Expr::Var(n) => match n.as_str() {
                "time" => self.t,
                "initial time" => Iv::point(self.t.lo),
                "final time" => Iv::point(self.t.hi),
                "time step" => Iv::point(self.m.settings.time_step),
                _ => self.iv.get(n).copied().unwrap_or(ALL),
            },
            Expr::Neg(a) => self.eval(a).neg(),
            Expr::Not(a) => {
                self.eval(a);
                Iv::new(0.0, 1.0)
            }
            Expr::Bin(op, a, b) => {
                let (x, y) = (self.eval(a), self.eval(b));
                match op {
                    BinOp::Add => x.add(y),
                    BinOp::Sub => x.sub(y),
                    BinOp::Mul => x.mul(y),
                    BinOp::Div => match x.div(y) {
                        Some(r) => r,
                        None => {
                            if !matches!(**b, Expr::Num(_)) {
                                let text = super::base::expr_text(b, self.m);
                                if let Some(f) = self.facts.as_deref_mut() {
                                    f.div_zero.push((self.owner.clone(), text, y));
                                }
                            }
                            ALL
                        }
                    },
                    BinOp::Pow => match &**b {
                        Expr::Num(n) if n.fract() == 0.0 && *n >= 0.0 => {
                            let k = *n as i32;
                            if k % 2 == 0 {
                                if x.lo >= 0.0 {
                                    Iv::new(x.lo.powi(k), x.hi.powi(k))
                                } else if x.hi <= 0.0 {
                                    Iv::new(x.hi.powi(k), x.lo.powi(k))
                                } else {
                                    Iv { lo: 0.0, hi: x.lo.abs().max(x.hi.abs()).powi(k) }
                                }
                            } else {
                                Iv::new(x.lo.powi(k), x.hi.powi(k))
                            }
                        }
                        _ => pow_iv(x, y),
                    },
                    BinOp::And | BinOp::Or | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne => Iv::new(0.0, 1.0),
                }
            }
            Expr::LookupCall(name, a) => {
                let x = self.eval(a);
                match self.m.by_name(name).filter(|l| l.table.len() >= 2) {
                    Some(l) => {
                        let xr = x_range(&l.table);
                        if x.is_finite() && (x.lo < xr.0 - 1e-9 || x.hi > xr.1 + 1e-9) {
                            if let Some(f) = self.facts.as_deref_mut() {
                                f.lookup_over.push((self.owner.clone(), l.name.clone(), x, xr));
                            }
                        }
                        y_range(&l.table)
                    }
                    None => ALL,
                }
            }
            Expr::WithLookup(a, table) => {
                let x = self.eval(a);
                if table.len() < 2 {
                    return ALL;
                }
                let xr = x_range(table);
                if x.is_finite() && (x.lo < xr.0 - 1e-9 || x.hi > xr.1 + 1e-9) {
                    if let Some(f) = self.facts.as_deref_mut() {
                        f.lookup_over.push((self.owner.clone(), "WITH LOOKUP".into(), x, xr));
                    }
                }
                y_range(table)
            }
            Expr::Call(f, args) => {
                let v: Vec<Iv> = args.iter().map(|a| self.eval(a)).collect();
                match f.as_str() {
                    "MIN" => v.iter().copied().reduce(|a, b| Iv { lo: a.lo.min(b.lo), hi: a.hi.min(b.hi) }).unwrap_or(ALL),
                    "MAX" => v.iter().copied().reduce(|a, b| Iv { lo: a.lo.max(b.lo), hi: a.hi.max(b.hi) }).unwrap_or(ALL),
                    "ABS" => {
                        let x = v[0];
                        if x.lo >= 0.0 { x } else if x.hi <= 0.0 { x.neg() } else { Iv { lo: 0.0, hi: x.lo.abs().max(x.hi.abs()) } }
                    }
                    "EXP" => Iv { lo: v[0].lo.exp(), hi: v[0].hi.exp() },
                    "LN" => {
                        if v[0].lo <= 0.0 {
                            let text = super::base::expr_text(&args[0], self.m);
                            if let Some(fa) = self.facts.as_deref_mut() {
                                fa.ln_bad.push((self.owner.clone(), text, v[0]));
                            }
                            Iv { lo: f64::NEG_INFINITY, hi: if v[0].hi > 0.0 { v[0].hi.ln() } else { f64::INFINITY } }
                        } else {
                            Iv { lo: v[0].lo.ln(), hi: v[0].hi.ln() }
                        }
                    }
                    "SQRT" => Iv { lo: v[0].lo.max(0.0).sqrt(), hi: v[0].hi.max(0.0).sqrt() },
                    "POWER" if v.len() == 2 => pow_iv(v[0], v[1]),
                    // delays and smoothing keep the interval of their input
                    "SMOOTH" | "SMOOTHI" | "SMOOTH3" | "SMOOTH3I" | "DELAY1" | "DELAY1I" | "DELAY3" | "DELAY3I" | "DELAY FIXED" | "DELAY" | "PREVIOUS" => {
                        let mut r = v[0];
                        if matches!(f.as_str(), "SMOOTHI" | "SMOOTH3I" | "DELAY1I" | "DELAY3I" | "DELAY FIXED") {
                            r = r.hull(*v.last().unwrap());
                        }
                        r
                    }
                    "IF THEN ELSE" if v.len() == 3 => v[1].hull(v[2]),
                    "STEP" => v[0].hull(Iv::point(0.0)),
                    "PULSE" | "PULSE TRAIN" => Iv::new(0.0, 1.0),
                    "RANDOM UNIFORM" | "RANDOM NORMAL" => Iv::new(v[0].lo, v[1].hi),
                    "MODULO" if v.len() == 2 && v[1].lo > 0.0 => Iv { lo: 0.0, hi: v[1].hi },
                    "ZIDZ" | "XIDZ" => ALL,
                    _ => ALL,
                }
            }
        }
    }
}

/// The analysis of a model.
pub fn analyse(m: &Model) -> Facts {
    let t = Iv::new(m.settings.initial_time, m.settings.final_time);
    let mut iv: HashMap<String, Iv> = HashMap::new();
    let parsed: Vec<Option<Expr>> = m.vars.iter().map(|v| expr::parse(&v.equation).ok()).collect();
    for (v, e) in m.vars.iter().zip(&parsed) {
        let key = expr::normalize(&v.name);
        let init = match v.kind {
            Kind::Constant => match (&v.slider, e) {
                (Some(s), _) => Iv::new(s[0], s[1]),
                (None, Some(Expr::Num(n))) => Iv::point(*n),
                (None, Some(Expr::Neg(a))) if matches!(**a, Expr::Num(_)) => match **a { Expr::Num(n) => Iv::point(-n), _ => ALL },
                _ => ALL,
            },
            Kind::Lookup if v.table.len() >= 2 => y_range(&v.table),
            Kind::Stock => {
                if v.non_negative { Iv { lo: 0.0, hi: f64::INFINITY } } else { ALL }
            }
            _ => ALL,
        };
        iv.insert(key, init);
    }
    // Start from "anything" and evaluate the equations again and again: every pass gives a sound, tighter interval, so
    // stopping after 5 passes (rather than waiting for a fixed point on a feedback loop that never settles) loses only
    // precision; a bound that is still moving then stays at infinity, which is the widening the stage asks for.
    let order: Vec<usize> = (0..m.vars.len()).filter(|i| matches!(m.vars[*i].kind, Kind::Aux | Kind::Flow)).collect();
    for _pass in 0..5 {
        let mut moved = false;
        for &i in &order {
            let Some(e) = &parsed[i] else { continue };
            let key = expr::normalize(&m.vars[i].name);
            let new = Walker { m, iv: &iv, owner: m.vars[i].name.clone(), t, facts: None }.eval(e);
            if iv.get(&key) != Some(&new) {
                iv.insert(key, new);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    // the final pass records the "can" facts
    let mut facts = Facts { iv: iv.clone(), ..Default::default() };
    for &i in &order {
        if let Some(e) = &parsed[i] {
            Walker { m, iv: &iv, owner: m.vars[i].name.clone(), t, facts: Some(&mut facts) }.eval(e);
        }
    }
    facts
}
