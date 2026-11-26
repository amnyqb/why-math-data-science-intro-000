//! Units as algebra. A unit written as text ("MB", "Truck/Day", "Barrels per Truck", "1/Day") is
//! read into base units with exponents, and every equation is given a dimension by the same rules
//! as arithmetic: sums and comparisons need equal dimensions, products and quotients combine
//! them, `SMOOTH` and `DELAY` keep their input's, `SQRT` halves, `LN` and `EXP` want nothing.
//! What cannot be known (a variable with no units, a literal, a function not listed) is left
//! alone, so the check only speaks when it is sure.
//!
//! A rate may meet its amount (MB and MB/Day): that is what a time step does, so the time unit
//! is ignored when comparing, unless `strict` is asked for. A flow's units times time must give
//! its stock's units; a variable's declared units must be those of its equation.

use std::collections::{BTreeMap, HashMap};

use crate::expr::{self, BinOp, Expr};
use crate::model::{Kind, Model};

/// Base unit → exponent. Empty means dimensionless.
pub type Dim = BTreeMap<String, i32>;

#[derive(Clone, Debug, PartialEq)]
enum D {
    Known(Dim),
    /// A literal number: fits beside anything and scales without changing the dimension.
    Num,
    /// Not known (a variable with no units, a function not listed): never the cause of a complaint.
    Any,
}

fn singular(w: &str) -> String {
    let w = w.to_lowercase();
    if let Some(stem) = w.strip_suffix("ies") {
        if stem.len() > 1 {
            return format!("{stem}y");
        }
    }
    if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") {
        return w[..w.len() - 1].to_string();
    }
    w
}

/// The same kind of thing under different names or scales (scale is not tracked).
fn alias(w: &str) -> String {
    match w {
        "mb" | "mbbl" | "mmb" | "mmbbl" | "bbl" | "bbls" | "barrel" | "mbd" => "barrel".into(),
        other => other.to_string(),
    }
}

fn is_dimensionless_word(w: &str) -> bool {
    matches!(w, "" | "dmnl" | "dimensionless" | "fraction" | "ratio" | "factor" | "yes" | "no" | "1" | "%" | "percent" | "unitless" | "index")
}

/// The base unit of the model's time axis: the first word of its time unit ("Day of November" → day).
pub fn time_base(m: &Model) -> String {
    singular(m.settings.time_units.split_whitespace().next().unwrap_or(""))
}

/// Read a unit written as text. None when it cannot be read (odd symbols, unbalanced brackets).
/// Accepts `a/b`, `a*b`, `a^n`, "per", and brackets anywhere as a group: `kWh/(kW*Year)` is `kWh/kW/Year`,
/// `(kWh)` is `kWh`. A `/` divides everything up to the next `*` (as before), a bracket group counts as one factor.
pub fn parse_unit(text: &str) -> Option<Dim> {
    let t = text.trim().to_lowercase().replace(" per ", "/");
    let chars: Vec<char> = t.chars().collect();
    let mut pos = 0;
    let dim = parse_seq(&chars, &mut pos, 0)?;
    if pos != chars.len() {
        return None;
    }
    Some(dim)
}

/// One unit name with an optional `^n`, as a dimension (dimensionless words give an empty one).
fn parse_factor(f: &str) -> Option<Dim> {
    let f = f.trim();
    let mut dim = Dim::new();
    if f.is_empty() {
        return Some(dim);
    }
    let (name, pow) = match f.split_once('^') {
        Some((n, p)) => (n.trim().to_string(), p.trim().parse::<i32>().ok()?),
        None => (f.to_string(), 1),
    };
    // "Day of Month" is a day: what follows "of" only says which
    let words: Vec<&str> = name.split_whitespace().take_while(|w| *w != "of").collect();
    let name = alias(&words.iter().map(|w| singular(w)).collect::<Vec<_>>().join("_"));
    if !is_dimensionless_word(&name) {
        *dim.entry(name).or_insert(0) += pow;
    }
    Some(dim)
}

/// A product/quotient sequence up to the closing bracket of its level (or the end of the text).
fn parse_seq(c: &[char], pos: &mut usize, depth: usize) -> Option<Dim> {
    if depth > 8 {
        return None;
    }
    let mut dim = Dim::new();
    let mut sign = 1;
    let mut cur = String::new();
    // a bracket group just closed: it may be followed by `^n`
    let mut group: Option<Dim> = None;
    let mut take = |cur: &mut String, group: &mut Option<Dim>, sign: i32, dim: &mut Dim| -> Option<()> {
        let part = match group.take() {
            Some(g) => {
                // "(kw*year)^2": the exponent after the bracket
                let rest = cur.trim().to_string();
                cur.clear();
                match rest.strip_prefix('^') {
                    Some(p) => {
                        let n: i32 = p.trim().parse().ok()?;
                        g.into_iter().map(|(k, v)| (k, v * n)).collect()
                    }
                    None if rest.is_empty() => g,
                    None => return None,
                }
            }
            None => {
                let f = parse_factor(cur)?;
                cur.clear();
                f
            }
        };
        for (n, e) in part {
            *dim.entry(n).or_insert(0) += sign * e;
        }
        Some(())
    };
    while *pos < c.len() {
        match c[*pos] {
            '*' | '/' => {
                take(&mut cur, &mut group, sign, &mut dim)?;
                sign = if c[*pos] == '/' { -1 } else { 1 };
                *pos += 1;
            }
            '(' => {
                if !cur.trim().is_empty() || group.is_some() {
                    return None;
                }
                *pos += 1;
                group = Some(parse_seq(c, pos, depth + 1)?);
                if *pos >= c.len() || c[*pos] != ')' {
                    return None;
                }
                *pos += 1;
            }
            ')' => {
                if depth == 0 {
                    return None;
                }
                break;
            }
            ch => {
                cur.push(ch);
                *pos += 1;
            }
        }
    }
    take(&mut cur, &mut group, sign, &mut dim)?;
    dim.retain(|_, e| *e != 0);
    Some(dim)
}

pub fn show(d: &Dim) -> String {
    if d.is_empty() {
        return "dimensionless".into();
    }
    let part = |neg: bool| {
        d.iter().filter(|(_, e)| (**e < 0) == neg).map(|(n, e)| if e.abs() == 1 { n.clone() } else { format!("{n}^{}", e.abs()) }).collect::<Vec<_>>().join("*")
    };
    let (num, den) = (part(false), part(true));
    match (num.is_empty(), den.is_empty()) {
        (_, true) => num,
        (true, false) => format!("1/{den}"),
        _ => format!("{num}/{den}"),
    }
}

fn mul(a: &Dim, b: &Dim, sign: i32) -> Dim {
    let mut r = a.clone();
    for (n, e) in b {
        *r.entry(n.clone()).or_insert(0) += sign * e;
    }
    r.retain(|_, e| *e != 0);
    r
}

const TIME_WORDS: &[&str] = &["second", "minute", "hour", "day", "week", "month", "quarter", "year"];

/// The dimension with every unit of time taken out (a rate and its amount then agree).
fn without_time(d: &Dim, time: &str) -> Dim {
    let mut r = d.clone();
    r.remove(time);
    r.retain(|k, _| !TIME_WORDS.contains(&k.as_str()));
    r
}

struct Ctx<'a> {
    dims: &'a HashMap<String, D>,
    time: String,
    strict: bool,
    /// Complaints raised while inferring one equation.
    found: &'a mut Vec<String>,
}

impl Ctx<'_> {
    fn same(&self, a: &Dim, b: &Dim) -> bool {
        if self.strict { a == b } else { without_time(a, &self.time) == without_time(b, &self.time) }
    }

    fn join(&mut self, a: D, b: D, what: &str) -> D {
        match (a, b) {
            (D::Known(x), D::Known(y)) => {
                if !self.same(&x, &y) {
                    self.found.push(format!("{what} {} and {}", show(&x), show(&y)));
                }
                D::Known(x)
            }
            (D::Known(x), _) | (_, D::Known(x)) => D::Known(x),
            (D::Num, D::Num) => D::Num,
            _ => D::Any,
        }
    }

    fn infer(&mut self, e: &Expr) -> D {
        match e {
            Expr::Num(_) => D::Num,
            Expr::Var(n) => {
                if n == "time" {
                    return D::Known(Dim::from([(self.time.clone(), 1)]));
                }
                if n == "time step" || n == "initial time" || n == "final time" || n == "saveper" {
                    return D::Known(Dim::from([(self.time.clone(), 1)]));
                }
                self.dims.get(n).cloned().unwrap_or(D::Any)
            }
            Expr::Neg(a) => self.infer(a),
            Expr::Not(a) => {
                self.infer(a);
                D::Known(Dim::new())
            }
            Expr::Bin(op, a, b) => {
                let (x, y) = (self.infer(a), self.infer(b));
                match op {
                    BinOp::Add => self.join(x, y, "adds"),
                    BinOp::Sub => self.join(x, y, "subtracts"),
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne => {
                        self.join(x, y, "compares");
                        D::Known(Dim::new())
                    }
                    BinOp::And | BinOp::Or => D::Known(Dim::new()),
                    BinOp::Mul => match (x, y) {
                        (D::Known(p), D::Known(q)) => D::Known(mul(&p, &q, 1)),
                        (D::Known(p), D::Num) | (D::Num, D::Known(p)) => D::Known(p),
                        (D::Num, D::Num) => D::Num,
                        _ => D::Any,
                    },
                    BinOp::Div => match (x, y) {
                        (D::Known(p), D::Known(q)) => D::Known(mul(&p, &q, -1)),
                        (D::Known(p), D::Num) => D::Known(p),
                        (D::Num, D::Known(q)) => D::Known(mul(&Dim::new(), &q, -1)),
                        (D::Num, D::Num) => D::Num,
                        _ => D::Any,
                    },
                    BinOp::Pow => match (x, &**b) {
                        (D::Known(p), Expr::Num(n)) if n.fract() == 0.0 => D::Known(p.into_iter().map(|(k, v)| (k, v * *n as i32)).collect()),
                        (D::Known(p), _) if p.is_empty() => D::Known(p),
                        _ => D::Any,
                    },
                }
            }
            Expr::LookupCall(name, a) => {
                self.infer(a);
                self.dims.get(name).cloned().unwrap_or(D::Any)
            }
            Expr::WithLookup(a, _) => {
                self.infer(a);
                D::Any
            }
            Expr::Call(f, args) => {
                let ds: Vec<D> = args.iter().map(|a| self.infer(a)).collect();
                let first = ds.first().cloned().unwrap_or(D::Any);
                match f.as_str() {
                    "MIN" | "MAX" => ds.into_iter().reduce(|a, b| self.join(a, b, "compares")).unwrap_or(D::Any),
                    "IF THEN ELSE" if ds.len() == 3 => self.join(ds[1].clone(), ds[2].clone(), "chooses between"),
                    "SMOOTH" | "SMOOTHI" | "SMOOTH3" | "SMOOTH3I" | "DELAY1" | "DELAY1I" | "DELAY3" | "DELAY3I" | "DELAY FIXED" | "DELAYINF" | "DELAYMTR" | "ABS" | "ROUND" | "INTEGER" | "CEILING" | "CLAMP" | "TOTAL" | "AVERAGE" | "MEAN" | "MEDIAN" => first,
                    // the fractional rate of change of the input: per time unit
                    "TREND" => D::Known(Dim::from([(self.time.clone(), -1)])),
                    // safe divisions: a / b
                    "ZIDZ" | "XIDZ" if ds.len() >= 2 => match (&ds[0], &ds[1]) {
                        (D::Known(p), D::Known(q)) => D::Known(mul(p, q, -1)),
                        (D::Known(p), D::Num) => D::Known(p.clone()),
                        _ => D::Any,
                    },
                    "STEP" => first,
                    "SQRT" => match first {
                        D::Known(p) if p.values().all(|e| e % 2 == 0) => D::Known(p.into_iter().map(|(k, v)| (k, v / 2)).collect()),
                        _ => D::Any,
                    },
                    "LN" | "LOG10" | "EXP" | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ATAN" | "SINH" | "COSH" | "TANH" => D::Known(Dim::new()),
                    "RAMP" => match first {
                        D::Known(p) => D::Known(mul(&p, &Dim::from([(self.time.clone(), 1)]), 1)),
                        _ => D::Any,
                    },
                    _ => D::Any,
                }
            }
        }
    }
}

fn declared(units: &str) -> D {
    if units.trim().is_empty() { D::Any } else { parse_unit(units).map(D::Known).unwrap_or(D::Any) }
}

/// Unit problems in the model: (variable id, message). Lenient about a rate meeting its amount.
pub fn warnings(m: &Model) -> Vec<(u64, String)> {
    check(m, false)
}

/// As `warnings`, but also complains when a rate and an amount meet (time unit not ignored).
pub fn warnings_strict(m: &Model) -> Vec<(u64, String)> {
    check(m, true)
}

fn check(m: &Model, strict: bool) -> Vec<(u64, String)> {
    let time = time_base(m);
    let dims: HashMap<String, D> = m.vars.iter().map(|v| (expr::normalize(&v.name), declared(&v.units))).collect();
    let mut out = Vec::new();
    for v in &m.vars {
        if !v.units.trim().is_empty() && parse_unit(&v.units).is_none() {
            out.push((v.id, format!("'{}': the units “{}” cannot be read (brackets and symbols are not understood), so they are not checked", v.name, v.units.trim())));
        }
        let Ok(e) = expr::parse(&v.equation) else { continue };
        let mut found = Vec::new();
        let got = Ctx { dims: &dims, time: time.clone(), strict, found: &mut found }.infer_top(&e);
        for f in found {
            out.push((v.id, format!("'{}' {f}: different units", v.name)));
        }
        // the equation must give the units the variable declares (a stock's equation is its start value)
        if let (D::Known(g), D::Known(d)) = (got, declared(&v.units)) {
            let eq_ok = if v.kind == Kind::Flow && strict { without_time(&g, &time) == without_time(&d, &time) } else if strict { g == d } else { without_time(&g, &time) == without_time(&d, &time) };
            if !eq_ok && !matches!(v.kind, Kind::Lookup) {
                out.push((v.id, format!("'{}' is declared in {} but its equation gives {}", v.name, show(&d), show(&g))));
            }
        }
        // a flow, per time, must be in its stock's units
        if v.kind == Kind::Flow {
            if let (D::Known(f), true) = (declared(&v.units), !v.units.trim().is_empty()) {
                for sid in [v.from, v.to].into_iter().flatten() {
                    if let Some(D::Known(s)) = m.var(sid).map(|s| declared(&s.units)) {
                        let ok = if strict { mul(&f, &Dim::from([(time.clone(), 1)]), 1) == s } else { without_time(&f, &time) == without_time(&s, &time) };
                        if !ok {
                            out.push((v.id, format!("Flow '{}' ({}) fills or drains '{}' ({}): a flow should be {} per time", v.name, v.units, m.var(sid).unwrap().name, m.var(sid).unwrap().units, m.var(sid).unwrap().units)));
                        }
                    }
                }
            }
        }
    }
    out
}

/// The dimension of a formula, written in the model's names. None when it cannot be known.
pub fn expression_dim(m: &Model, text: &str) -> Option<Dim> {
    let e = expr::parse(text).ok()?;
    let dims: HashMap<String, D> = m.vars.iter().map(|v| (expr::normalize(&v.name), declared(&v.units))).collect();
    let mut found = Vec::new();
    match (Ctx { dims: &dims, time: time_base(m), strict: false, found: &mut found }).infer_top(&e) {
        D::Known(d) => Some(d),
        _ => None,
    }
}

pub fn without_time_pub(d: &Dim, time: &str) -> Dim {
    without_time(d, time)
}

impl Ctx<'_> {
    fn infer_top(mut self, e: &Expr) -> D {
        self.infer(e)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn brackets_in_units_read_as_groups() {
        assert_eq!(parse_unit("kWh/(kW*Year)"), parse_unit("kWh/kW/Year"));
        assert!(parse_unit("kWh/(kW*Year)").is_some());
        assert_eq!(parse_unit("USD/(kW*Year)"), parse_unit("USD/kW/Year"));
        assert_eq!(parse_unit("(kWh)"), parse_unit("kWh"));
        assert_eq!(parse_unit("kWh per (kW*Year)"), parse_unit("kWh/kW/Year"));
        assert_eq!(parse_unit("m/(s^2)"), parse_unit("m/s^2"));
        assert_eq!(parse_unit("1/(Day)"), parse_unit("1/Day"));
        assert!(parse_unit("kWh/(kW*Year").is_none());
        assert!(parse_unit("kWh)").is_none());
    }

    use super::*;
    use crate::model::Var;

    fn add(m: &mut Model, name: &str, kind: Kind, eq: &str, units: &str) -> u64 {
        let id = m.new_id();
        let mut v = Var::new(id, name, kind, [0.0, 0.0]);
        v.equation = eq.into();
        v.units = units.into();
        m.vars.push(v);
        id
    }

    #[test]
    fn different_kinds_of_units_are_flagged_but_rates_meet_amounts() {
        let mut m = Model::default();
        add(&mut m, "Fuel", Kind::Constant, "1", "MB");
        add(&mut m, "Trucks", Kind::Constant, "1", "Truck");
        add(&mut m, "Rate", Kind::Constant, "1", "MB/Day");
        add(&mut m, "Share", Kind::Constant, "1", "Fraction");
        let bad = add(&mut m, "Mixed", Kind::Aux, "Fuel + Trucks", "MB");
        add(&mut m, "Fine", Kind::Aux, "Fuel + Rate", "MB");
        add(&mut m, "Also fine", Kind::Aux, "Fuel * Share + Fuel", "MB");
        add(&mut m, "Compare", Kind::Aux, "MIN(Fuel, Trucks)", "MB");
        let w = warnings(&m);
        assert_eq!(w.iter().filter(|(id, _)| *id == bad).count(), 1, "{w:?}");
        assert_eq!(w.len(), 2, "{w:?}"); // Mixed and Compare only
        assert!(w[0].1.contains("different units"), "{}", w[0].1);
    }

    #[test]
    fn a_flow_in_other_units_than_its_stock_is_flagged() {
        let mut m = Model::default();
        let s = add(&mut m, "Tank", Kind::Stock, "0", "MB");
        let f = add(&mut m, "Delivery", Kind::Flow, "1", "Truck/Day");
        m.var_mut(f).unwrap().to = Some(s);
        let ok = add(&mut m, "Burn", Kind::Flow, "1", "MB/Day");
        m.var_mut(ok).unwrap().from = Some(s);
        let w = warnings(&m);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].1.contains("Delivery"));
    }
}

#[cfg(test)]
mod algebra_tests {
    use super::*;
    use crate::model::Var;

    fn m_with(vars: &[(&str, Kind, &str, &str)]) -> Model {
        let mut m = Model::default();
        m.settings.time_units = "Day".into();
        for (n, k, eq, u) in vars {
            let id = m.new_id();
            let mut v = Var::new(id, n, *k, [0.0, 0.0]);
            v.equation = (*eq).into();
            v.units = (*u).into();
            m.vars.push(v);
        }
        m
    }

    #[test]
    fn units_are_read_as_algebra() {
        assert_eq!(show(&parse_unit("Truck/Day").unwrap()), "truck/day");
        assert_eq!(show(&parse_unit("Barrels per Truck").unwrap()), "barrel/truck");
        assert_eq!(show(&parse_unit("1/Day").unwrap()), "1/day");
        assert_eq!(show(&parse_unit("MB*Day/Truck^2").unwrap()), "barrel*day/truck^2");
        assert!(parse_unit("Dmnl").unwrap().is_empty());
        assert!(parse_unit("Day of Month").unwrap() == parse_unit("Day").unwrap());
        assert!(parse_unit("kg (approx)").is_none());
    }

    #[test]
    fn products_and_quotients_must_give_the_declared_units() {
        let ok = m_with(&[("Trucks", Kind::Constant, "4", "Truck/Day"), ("Load", Kind::Constant, "215", "Barrel/Truck"), ("Moved", Kind::Aux, "Trucks * Load", "Barrel/Day")]);
        assert!(warnings(&ok).is_empty(), "{:?}", warnings(&ok));
        let bad = m_with(&[("Trucks", Kind::Constant, "4", "Truck/Day"), ("Load", Kind::Constant, "215", "Barrel/Truck"), ("Moved", Kind::Aux, "Trucks / Load", "Barrel/Day")]);
        let w = warnings(&bad);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].1.contains("declared in barrel/day"), "{}", w[0].1);
    }

    #[test]
    fn a_rate_meets_an_amount_only_when_not_strict() {
        let m = m_with(&[("Stock", Kind::Constant, "1", "MB"), ("Rate", Kind::Constant, "1", "MB/Day"), ("Sum", Kind::Aux, "Stock + Rate", "MB")]);
        assert!(warnings(&m).is_empty());
        assert_eq!(warnings_strict(&m).len(), 1);
    }

    #[test]
    fn functions_keep_or_change_dimensions() {
        let m = m_with(&[
            ("Level", Kind::Constant, "4", "Truck"),
            ("Smoothed", Kind::Aux, "SMOOTH(Level, 3)", "Truck"),
            ("Root", Kind::Aux, "SQRT(Level * Level)", "Truck"),
            ("Bad root", Kind::Aux, "SQRT(Level)", "Truck"),
            ("Logged", Kind::Aux, "LN(2)", "Dmnl"),
            ("Pick", Kind::Aux, "IF THEN ELSE(Level > 2, Level, Level * 2)", "Truck"),
        ]);
        let w = warnings(&m);
        assert!(w.is_empty(), "{w:?}"); // SQRT of an odd power is simply unknown, not an error
    }

    #[test]
    fn time_and_literals_behave() {
        let m = m_with(&[("Gap", Kind::Constant, "4", "MB"), ("Per day", Kind::Aux, "Gap / 10", "MB"), ("Wait", Kind::Aux, "TIME * 2", "Day")]);
        assert!(warnings(&m).is_empty(), "{:?}", warnings(&m));
        let bad = m_with(&[("Gap", Kind::Constant, "4", "MB"), ("Wait", Kind::Aux, "Gap + TIME", "MB")]);
        assert_eq!(warnings(&bad).len(), 1);
    }
}

// =====================================================================================================================
// The exact units engine: a unit is base dimensions plus a scale (and an offset for temperature scales). Sums and
// comparisons need equal dimensions and a scale ratio of 1 ± 1e-12; a flow times the time unit must give its stock's
// unit, scale included. The older name-based `Dim` checks above stay for `validate`; the doctor uses this engine.
// =====================================================================================================================

use crate::unit_table;

/// A unit: `dims` (base dimensions m, kg, s, A, K, mol, cd, `currency:XXX`, `count:name`), the factor to the base unit
/// (`scale`), an offset (temperature scales only), the price year of money (`real_year`), and the symbols used without a
/// stated meaning (`ambiguous`).
#[derive(Clone, Debug)]
pub struct Unit {
    pub dims: Dim,
    pub scale: f64,
    pub offset: f64,
    pub real_year: Option<i32>,
    pub ambiguous: Vec<String>,
    /// the part of `scale` that is a pure number scale (percent, ppm): what `units_scale_values` applies to a value
    pub pure_scale: f64,
}

impl PartialEq for Unit {
    fn eq(&self, o: &Self) -> bool {
        self.dims == o.dims && same_scale(self.scale, o.scale)
    }
}

pub fn same_scale(a: f64, b: f64) -> bool {
    (a / b - 1.0).abs() <= 1e-12
}

impl Unit {
    pub fn one() -> Unit {
        Unit { dims: Dim::new(), scale: 1.0, offset: 0.0, real_year: None, ambiguous: Vec::new(), pure_scale: 1.0 }
    }
    pub fn is_dimensionless(&self) -> bool {
        self.dims.is_empty()
    }
    pub fn mul(&self, o: &Unit) -> Unit {
        self.combine(o, 1)
    }
    pub fn div(&self, o: &Unit) -> Unit {
        self.combine(o, -1)
    }
    fn combine(&self, o: &Unit, sign: i32) -> Unit {
        let mut dims = self.dims.clone();
        for (n, e) in &o.dims {
            *dims.entry(n.clone()).or_insert(0) += sign * e;
        }
        dims.retain(|_, e| *e != 0);
        let mut ambiguous = self.ambiguous.clone();
        for a in &o.ambiguous {
            if !ambiguous.contains(a) {
                ambiguous.push(a.clone());
            }
        }
        let (scale, pure) = if sign > 0 { (self.scale * o.scale, self.pure_scale * o.pure_scale) } else { (self.scale / o.scale, self.pure_scale / o.pure_scale) };
        Unit { dims, scale, offset: 0.0, real_year: self.real_year.or(o.real_year), ambiguous, pure_scale: pure }
    }
    pub fn powi(&self, n: i32) -> Unit {
        Unit {
            dims: self.dims.iter().map(|(k, v)| (k.clone(), v * n)).filter(|(_, v)| *v != 0).collect(),
            scale: self.scale.powi(n),
            offset: 0.0,
            real_year: self.real_year,
            ambiguous: self.ambiguous.clone(),
            pure_scale: self.pure_scale.powi(n),
        }
    }
    /// This unit with the pure-number scale (percent, ppm) taken out: what a value declared in it means once scaled.
    pub fn without_pure_scale(&self) -> Unit {
        Unit { scale: self.scale / self.pure_scale, pure_scale: 1.0, ..self.clone() }
    }
    /// A readable form in base units, e.g. "m^3/s" or "currency:USD/(kg*s)" with the factor.
    pub fn show_exact(&self) -> String {
        let d = show(&self.dims);
        if same_scale(self.scale, 1.0) { d } else { format!("{d} × {}", crate::fmt::fmt_num(self.scale)) }
    }
}

/// What the parser needs to know about the model: the length of a Year and the meanings chosen for ambiguous symbols.
#[derive(Clone, Debug)]
pub struct UnitCtx {
    pub year_days: f64,
    pub choices: std::collections::BTreeMap<String, String>,
}

impl Default for UnitCtx {
    fn default() -> Self {
        UnitCtx { year_days: 365.25, choices: Default::default() }
    }
}

impl UnitCtx {
    pub fn of(m: &Model) -> UnitCtx {
        UnitCtx { year_days: m.settings.year_days_abstract, choices: m.settings.unit_choices.clone() }
    }
}

/// A unit that cannot be read: where, why, and the closest known atom.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitParseError {
    pub pos: usize,
    pub message: String,
    pub suggestion: Option<String>,
}

impl std::fmt::Display for UnitParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at {})", self.message, self.pos)?;
        if let Some(s) = &self.suggestion {
            write!(f, ": did you mean “{s}”?")?;
        }
        Ok(())
    }
}

/// Read a unit with the default context (Year = 365.25 days, no ambiguity choices).
pub fn parse_unit_exact(text: &str) -> Result<Unit, UnitParseError> {
    parse_unit_in(text, &UnitCtx::default())
}

thread_local! {
    /// Units are read again and again (every variable, every rule); the same text with the same context gives the same unit.
    static PARSED: std::cell::RefCell<std::collections::HashMap<(String, u64, String), Result<Unit, UnitParseError>>> = Default::default();
}

pub fn parse_unit_in(text: &str, ctx: &UnitCtx) -> Result<Unit, UnitParseError> {
    let key = (text.to_string(), ctx.year_days.to_bits(), format!("{:?}", ctx.choices));
    if let Some(hit) = PARSED.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let r = parse_unit_uncached(text, ctx);
    PARSED.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 20_000 {
            c.clear();
        }
        c.insert(key, r.clone());
    });
    r
}

fn parse_unit_uncached(text: &str, ctx: &UnitCtx) -> Result<Unit, UnitParseError> {
    let chars: Vec<char> = text.trim().chars().collect();
    let mut pos = 0;
    let u = parse_seq_exact(&chars, &mut pos, 0, ctx)?;
    if pos != chars.len() {
        return Err(UnitParseError { pos, message: "unbalanced bracket".into(), suggestion: None });
    }
    Ok(u)
}

fn atom_unit(a: &unit_table::Atom, factor: f64, name: &str, ctx: &UnitCtx, symbol: &str) -> Unit {
    let mut factor = factor;
    let mut ambiguous = Vec::new();
    if !a.meanings.is_empty() {
        match ctx.choices.get(symbol).and_then(|c| a.meanings.iter().find(|m| m.label.eq_ignore_ascii_case(c))) {
            Some(m) => factor = m.factor * (factor / a.factor),
            None => ambiguous.push(symbol.to_string()),
        }
    }
    let _ = name;
    Unit {
        dims: a.dims.iter().filter(|(_, e)| *e != 0).map(|(n, e)| (n.to_string(), *e)).collect(),
        scale: factor,
        offset: 0.0,
        real_year: None,
        ambiguous,
        pure_scale: if a.pure_number { factor } else { 1.0 },
    }
}

/// One atom word (no operators), with an optional `^n` handled by the caller.
fn resolve_atom(word: &str, ctx: &UnitCtx, pos: usize) -> Result<Unit, UnitParseError> {
    let w = word.trim();
    if unit_table::is_dimensionless_word(w) {
        return Ok(Unit::one());
    }
    // money: USD, USD_2024 (real dollars of 2024), $, €, £
    let money = |code: &str, year: Option<i32>| Unit { dims: Dim::from([(format!("currency:{code}"), 1)]), real_year: year, ..Unit::one() };
    match w {
        "$" => return Ok(money("USD", None)),
        "€" => return Ok(money("EUR", None)),
        "£" => return Ok(money("GBP", None)),
        "dollar" | "dollars" => return Ok(money("USD", None)),
        "euro" | "euros" => return Ok(money("EUR", None)),
        _ => {}
    }
    let (head, tail) = match w.split_once('_') {
        Some((h, t)) if t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()) => (h, t.parse::<i32>().ok()),
        _ => (w, None),
    };
    if unit_table::CURRENCIES.contains(&head) {
        return Ok(money(head, tail));
    }
    // thousands and millions of money: kUSD, MEUR
    for (p, f) in [("k", 1e3), ("M", 1e6), ("G", 1e9)] {
        if let Some(code) = head.strip_prefix(p) {
            if unit_table::CURRENCIES.contains(&code) {
                let mut u = money(code, tail);
                u.scale = f;
                return Ok(u);
            }
        }
    }
    let find = |name: &str| -> Option<Unit> {
        if let Some(a) = unit_table::lookup(name, ctx.year_days) {
            return Some(atom_unit(&a, a.factor, name, ctx, name));
        }
        for (p, f) in unit_table::PREFIXES {
            if let Some(rest) = name.strip_prefix(p) {
                if rest.is_empty() {
                    continue;
                }
                if let Some(a) = unit_table::lookup(rest, ctx.year_days) {
                    if a.metric {
                        return Some(atom_unit(&a, a.factor * f, name, ctx, name));
                    }
                }
            }
        }
        None
    };
    if let Some(u) = find(w) {
        return Ok(u);
    }
    // plurals: barrels, kWhs? , trucks, companies
    let singular_w = if let Some(stem) = w.strip_suffix("ies") { format!("{stem}y") } else if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") { w[..w.len() - 1].to_string() } else { w.to_string() };
    if singular_w != w {
        if let Some(u) = find(&singular_w) {
            return Ok(u);
        }
    }
    // case-insensitive: kwh, mwh, DAY, Kw; the capital M wins over milli (MW, not mW)
    let lw = w.to_lowercase();
    let lw_s = singular_w.to_lowercase();
    for cand in [&lw, &lw_s] {
        for n in unit_table::ATOM_NAMES {
            if n.to_lowercase() == *cand {
                if let Some(u) = find(n) {
                    return Ok(u);
                }
            }
        }
        let mut prefixes: Vec<(&str, f64)> = unit_table::PREFIXES.to_vec();
        prefixes.sort_by_key(|(p, _)| if *p == "M" { 0 } else { 1 });
        for (p, f) in &prefixes {
            if let Some(rest) = cand.strip_prefix(&p.to_lowercase()) {
                for n in unit_table::ATOM_NAMES {
                    if n.to_lowercase() == rest {
                        if let Some(a) = unit_table::lookup(n, ctx.year_days) {
                            if a.metric && !rest.is_empty() {
                                return Ok(atom_unit(&a, a.factor * f, n, ctx, &format!("{p}{n}")));
                            }
                        }
                    }
                }
            }
        }
    }
    // a typo of a known unit is an error with a suggestion; any other word is a counted thing
    if let Some(s) = unit_table::closest(w) {
        if s.to_lowercase() != lw && s.to_lowercase() != lw_s {
            return Err(UnitParseError { pos, message: format!("unknown unit “{w}”"), suggestion: Some(s.to_string()) });
        }
    }
    let name = singular(&w.replace(' ', "_"));
    Ok(Unit { dims: Dim::from([(format!("count:{name}"), 1)]), ..Unit::one() })
}

fn factor_exact(text: &str, ctx: &UnitCtx, pos: usize) -> Result<Unit, UnitParseError> {
    let f = text.trim();
    if f.is_empty() {
        return Ok(Unit::one());
    }
    let (name, pow) = match f.split_once('^') {
        Some((n, p)) => (n.trim(), p.trim().parse::<i32>().map_err(|_| UnitParseError { pos, message: format!("the exponent “{}” is not a whole number", p.trim()), suggestion: None })?),
        None => (f, 1),
    };
    // "Day of Month" is a day: what follows "of" only says which
    let words: Vec<&str> = name.split_whitespace().take_while(|w| *w != "of").collect();
    let joined = words.join(" ");
    // a multi-word unit that is itself a unit ("metric ton") or a counted thing ("Barrels per Truck" is split earlier)
    let u = if words.len() > 1 && unit_table::lookup(&joined, ctx.year_days).is_none() {
        resolve_atom(&words.join("_"), ctx, pos)?
    } else {
        resolve_atom(&joined, ctx, pos)?
    };
    Ok(u.powi(pow))
}

fn parse_seq_exact(c: &[char], pos: &mut usize, depth: usize, ctx: &UnitCtx) -> Result<Unit, UnitParseError> {
    if depth > 8 {
        return Err(UnitParseError { pos: *pos, message: "brackets nested too deep".into(), suggestion: None });
    }
    // "per" between words reads as a slash
    let mut unit = Unit::one();
    let mut sign = 1;
    let mut cur = String::new();
    let mut cur_start = *pos;
    let mut group: Option<Unit> = None;
    let mut take = |cur: &mut String, group: &mut Option<Unit>, sign: i32, unit: &mut Unit, at: usize| -> Result<(), UnitParseError> {
        let part = match group.take() {
            Some(g) => {
                let rest = cur.trim().to_string();
                cur.clear();
                match rest.strip_prefix('^') {
                    Some(p) => g.powi(p.trim().parse::<i32>().map_err(|_| UnitParseError { pos: at, message: "the exponent after the bracket is not a whole number".into(), suggestion: None })?),
                    None if rest.is_empty() => g,
                    None => return Err(UnitParseError { pos: at, message: format!("unexpected “{rest}” after a bracket"), suggestion: None }),
                }
            }
            None => {
                let text = std::mem::take(cur);
                // " per " between words
                let text = text.replace(" per ", "\u{1}");
                let mut part = Unit::one();
                let mut s = 1;
                for (k, piece) in text.split('\u{1}').enumerate() {
                    let u = factor_exact(piece, ctx, at)?;
                    part = if k == 0 { part.mul(&u) } else if s > 0 { part.mul(&u) } else { part.div(&u) };
                    s = -1;
                }
                part
            }
        };
        *unit = if sign > 0 { unit.mul(&part) } else { unit.div(&part) };
        Ok(())
    };
    while *pos < c.len() {
        match c[*pos] {
            '*' | '/' => {
                take(&mut cur, &mut group, sign, &mut unit, cur_start)?;
                sign = if c[*pos] == '/' { -1 } else { 1 };
                *pos += 1;
                cur_start = *pos;
            }
            '(' => {
                if !cur.trim().is_empty() || group.is_some() {
                    return Err(UnitParseError { pos: *pos, message: "a bracket must follow * or /".into(), suggestion: None });
                }
                *pos += 1;
                group = Some(parse_seq_exact(c, pos, depth + 1, ctx)?);
                if *pos >= c.len() || c[*pos] != ')' {
                    return Err(UnitParseError { pos: *pos, message: "missing closing bracket".into(), suggestion: None });
                }
                *pos += 1;
            }
            ')' => {
                if depth == 0 {
                    return Err(UnitParseError { pos: *pos, message: "closing bracket without an opening one".into(), suggestion: None });
                }
                break;
            }
            ch => {
                if cur.is_empty() {
                    cur_start = *pos;
                }
                cur.push(ch);
                *pos += 1;
            }
        }
    }
    take(&mut cur, &mut group, sign, &mut unit, cur_start)?;
    Ok(unit)
}

#[cfg(test)]
mod exact_tests {
    use super::*;

    fn u(t: &str) -> Unit {
        parse_unit_exact(t).unwrap_or_else(|e| panic!("{t}: {e}"))
    }

    #[test]
    fn conversions_are_exact() {
        // 1 MWh = 1000 kWh
        assert!(same_scale(u("MWh").scale, 1000.0 * u("kWh").scale));
        assert_eq!(u("MWh").dims, u("kWh").dims);
        // 1 bbl = 0.158987294928 m^3
        assert!(same_scale(u("bbl").scale, 0.158_987_294_928));
        assert_eq!(show(&u("bbl").dims), "m^3");
        // 1 MMBtu = 1e6 Btu = 1.05505585262e9 J
        assert!(same_scale(u("MMBtu").scale, 1.055_055_852_62e9));
        assert!(same_scale(u("toe").scale, 41.868e9));
        assert!(same_scale(u("kWh").scale, 3.6e6));
        assert!(same_scale(u("%").scale, 0.01));
        assert!(same_scale(u("Year").scale, 365.25 * 86_400.0));
        assert!(same_scale(u("Month").scale * 12.0, u("Year").scale));
    }

    #[test]
    fn brackets_and_per() {
        assert_eq!(u("kWh/(kW*Year)"), u("kWh/kW/Year"));
        assert_eq!(u("USD/(kW*Year)"), u("USD/kW/Year"));
        assert_eq!(u("Barrels per Truck").dims.get("count:truck"), Some(&-1));
        assert_eq!(u("MW"), u("MW"));
        assert!(!same_scale(u("MW").scale, u("mW").scale));
        // kWh per (kW Year): dimensionless apart from the hours in a year
        assert_eq!(u("kWh/(kW*Year)").dims.len(), 0);
        assert!(same_scale(u("kWh/(kW*Year)").scale, 3600.0 / (365.25 * 86_400.0)));
    }

    #[test]
    fn typos_get_a_suggestion_and_ambiguous_symbols_are_flagged() {
        let e = parse_unit_exact("kwhh").unwrap_err();
        assert_eq!(e.suggestion.as_deref(), Some("kWh"), "{e:?}");
        assert!(parse_unit_exact("kwh").is_ok());
        assert!(parse_unit_exact("Truck").is_ok());
        assert_eq!(u("MB").ambiguous, vec!["MB".to_string()]);
        let mut ctx = UnitCtx::default();
        ctx.choices.insert("MB".into(), "thousand barrels".into());
        let mb = parse_unit_in("MB", &ctx).unwrap();
        assert!(mb.ambiguous.is_empty());
        assert!(same_scale(mb.scale, 1000.0 * 0.158_987_294_928));
        ctx.choices.insert("MB".into(), "million barrels".into());
        assert!(same_scale(parse_unit_in("MB", &ctx).unwrap().scale, 1e6 * 0.158_987_294_928));
    }

    #[test]
    fn money_carries_a_price_year() {
        assert!(same_scale(u("kUSD").scale, 1000.0));
        assert_eq!(u("kUSD").dims, u("USD").dims);
        assert_eq!(u("USD_2024").real_year, Some(2024));
        assert_eq!(u("USD").real_year, None);
        assert_eq!(u("USD_2024/MWh").dims, u("USD/MWh").dims);
    }
}
