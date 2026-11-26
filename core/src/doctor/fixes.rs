//! Fix builders of the doctor. Each returns `Fix`es made of `ops::Op`s.
//!
//! Unit fixes are ranked: (a) a pure scale factor as a named constant in the "Conversions" group, wrapped round the
//! offending term; (b) a factor with a dimension taken from a variable of the model; (c) a change of declaration that
//! makes the equation consistent; (d) `CONVERT(x, "from", "to")`. Within a rank, fixes that keep the numbers come first.

use super::units_rules::{Clash, Engine, terms};
use super::{Finding, Fix, Severity};
use crate::expr::{self, Expr};
use crate::model::{Kind, Model, Var};
use crate::ops::Op;
use crate::unit_table;
use crate::units::{self, Unit, same_scale};

/// Round to 12 significant digits (the factors are ratios of exact constants) and write the shortest text.
pub fn clean_factor(f: f64) -> String {
    if f == 0.0 || !f.is_finite() {
        return format!("{f}");
    }
    let r: f64 = format!("{:.11e}", f).parse().unwrap_or(f);
    format!("{r}")
}

fn sanitize(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '%' { c } else { ' ' }).collect();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Factors of a unit text: "(kWh/Year)*Dmnl" gives [("kWh", 1), ("Year", -1)]. None if the text is not a plain product.
fn unit_factors(t: &str) -> Option<Vec<(String, i32)>> {
    fn expr(c: &[char], i: &mut usize) -> Option<Vec<(String, i32)>> {
        let mut acc = term(c, i)?;
        loop {
            while *i < c.len() && c[*i] == ' ' {
                *i += 1;
            }
            if *i >= c.len() || !matches!(c[*i], '*' | '/') {
                return Some(acc);
            }
            let div = c[*i] == '/';
            *i += 1;
            let rhs = term(c, i)?;
            acc.extend(rhs.into_iter().map(|(n, e)| (n, if div { -e } else { e })));
        }
    }
    fn term(c: &[char], i: &mut usize) -> Option<Vec<(String, i32)>> {
        while *i < c.len() && c[*i] == ' ' {
            *i += 1;
        }
        let mut f = if *i < c.len() && c[*i] == '(' {
            *i += 1;
            let r = expr(c, i)?;
            while *i < c.len() && c[*i] == ' ' {
                *i += 1;
            }
            if *i >= c.len() || c[*i] != ')' {
                return None;
            }
            *i += 1;
            r
        } else {
            let st = *i;
            while *i < c.len() && !matches!(c[*i], '*' | '/' | '(' | ')' | '^') {
                *i += 1;
            }
            let atom: String = c[st..*i].iter().collect::<String>().trim().to_string();
            if atom.is_empty() {
                return None;
            }
            vec![(atom, 1)]
        };
        if *i < c.len() && c[*i] == '^' {
            *i += 1;
            let st = *i;
            while *i < c.len() && (c[*i].is_ascii_digit() || c[*i] == '-') {
                *i += 1;
            }
            let n: i32 = c[st..*i].iter().collect::<String>().parse().ok()?;
            for x in f.iter_mut() {
                x.1 *= n;
            }
        }
        Some(f)
    }
    let c: Vec<char> = t.chars().collect();
    let mut i = 0;
    let r = expr(&c, &mut i)?;
    if i != c.len() { None } else { Some(r) }
}

/// A unit text without dimensionless factors, with equal factors cancelled: "(kWh/Year)*Dmnl" becomes "kWh/Year".
/// A text that is not a plain product of units is returned as it is.
pub fn simplify_unit_text(t: &str) -> String {
    let Some(fs) = unit_factors(t) else { return t.trim().to_string() };
    let mut acc: Vec<(String, i32)> = Vec::new();
    for (n, e) in fs {
        let low = n.to_lowercase();
        if matches!(low.as_str(), "dmnl" | "dimensionless" | "unitless" | "1" | "fraction" | "number") {
            continue;
        }
        match acc.iter_mut().find(|(m, _)| m.to_lowercase() == low) {
            Some(x) => x.1 += e,
            None => acc.push((n, e)),
        }
    }
    acc.retain(|(_, e)| *e != 0);
    let show = |l: Vec<String>| if l.len() == 1 { l[0].clone() } else { format!("({})", l.join("*")) };
    let part = |neg: bool| -> Vec<String> { acc.iter().filter(|(_, e)| (*e < 0) == neg).map(|(n, e)| if e.abs() == 1 { n.clone() } else { format!("{n}^{}", e.abs()) }).collect() };
    let (num, den) = (part(false), part(true));
    match (num.is_empty(), den.is_empty()) {
        (true, true) => "Dmnl".into(),
        (false, true) => num.join("*"),
        (true, false) => format!("1/{}", show(den)),
        (false, false) => format!("{}/{}", num.join("*"), show(den)),
    }
}

/// The text of the units of an expression from the declared units of the variables in it (None if one is missing or the
/// expression is not made of products, quotients and powers).
pub fn text_of(m: &Model, e: &Expr) -> Option<String> {
    use crate::expr::BinOp;
    let t = match e {
        Expr::Num(_) => "1".to_string(),
        Expr::Var(n) => {
            if n == "time" {
                m.settings.time_units.trim().to_string()
            } else {
                let u = m.by_name(n)?.units.trim().to_string();
                if u.is_empty() {
                    return None;
                }
                u
            }
        }
        Expr::Neg(a) => text_of(m, a)?,
        Expr::Bin(BinOp::Mul, a, b) => format!("({})*({})", text_of(m, a)?, text_of(m, b)?),
        Expr::Bin(BinOp::Div, a, b) => format!("({})/({})", text_of(m, a)?, text_of(m, b)?),
        Expr::Bin(BinOp::Add | BinOp::Sub, a, b) => {
            let (x, y) = (text_of(m, a), text_of(m, b));
            x.or(y)?
        }
        Expr::Bin(BinOp::Pow, a, b) => match **b {
            Expr::Num(n) if n.fract() == 0.0 => format!("({})^{}", text_of(m, a)?, n as i32),
            _ => return None,
        },
        _ => return None,
    };
    Some(simplify_unit_text(&t))
}

/// "Day" becomes "Days" (the names of the units of time only: "MB per barrel" stays as it is).
fn plural(word: &str) -> String {
    let w = word.trim();
    if ["second", "minute", "hour", "day", "week", "month", "quarter", "year"].contains(&w.to_lowercase().as_str()) { format!("{w}s") } else { w.to_string() }
}

/// " (assuming a 7-day week)" when the units differ between days and weeks: the factor 7 is a choice (a desk that works
/// five days a week needs 5).
pub fn week_note(from: &str, to: &str) -> &'static str {
    let l = |t: &str| t.to_lowercase();
    let has = |t: &str, w: &str| l(t).split(|c: char| !c.is_alphabetic()).any(|x| x == w || x == format!("{w}s"));
    if (has(from, "week") && has(to, "day") && !has(from, "day")) || (has(to, "week") && has(from, "day") && !has(to, "day")) { " (assuming a 7-day week)" } else { "" }
}

const CALENDAR: &[&str] = &["week", "month", "quarter", "year"];

fn is_time_text(t: &str) -> bool {
    units::parse_unit_exact(t).is_ok_and(|u| u.dims.len() == 1 && u.dims.get("s") == Some(&1) && u.pure_scale == 1.0)
}

fn is_calendar_text(t: &str) -> bool {
    let w = t.trim().trim_end_matches('s').to_lowercase();
    CALENDAR.contains(&w.as_str()) || ["wk", "mo", "yr", "a"].contains(&w.as_str())
}

/// The name and the units of the conversion constant that turns a quantity written `from` into one written `to`, built
/// from the declared texts only: from "Week" to "Day" the constant is "Days per Week" in "Day/Week".
pub fn conversion_names(from: &str, to: &str) -> (String, String) {
    let (from, to) = (simplify_unit_text(from), simplify_unit_text(to));
    // the constant has the units to / from
    let ratio = simplify_unit_text(&format!("({to})/({from})"));
    let (num, den) = super::units_rules::split_text(&ratio);
    let strip = |s: &str| s.trim_matches(|c| c == '(' || c == ')').trim().to_string();
    let name = match den {
        Some(d) => sanitize(&format!("{} per {}", plural(&strip(&num)), strip(&d))),
        None => sanitize(&format!("{} per {}", strip(&to), strip(&from))),
    };
    (name, ratio)
}

/// Is the conversion from `from` to `to` a choice of the modeller (a week of 7 calendar days or of 5 working days, a month
/// of 30 days or of 30.4375)? The units and their texts decide, never the factor.
pub fn is_calendar_conversion(from: &str, to: &str) -> bool {
    let (nf, df) = super::units_rules::split_text(&simplify_unit_text(from));
    let (nt, dt) = super::units_rules::split_text(&simplify_unit_text(to));
    let strip = |s: &str| s.trim_matches(|c| c == '(' || c == ')').trim().to_string();
    let pairs: Vec<(String, String)> = if strip(&nf).eq_ignore_ascii_case(&strip(&nt)) {
        match (df, dt) {
            (Some(a), Some(b)) => vec![(strip(&a), strip(&b))],
            _ => vec![],
        }
    } else {
        vec![(strip(&nf), strip(&nt))]
    };
    pairs.iter().any(|(a, b)| is_time_text(a) && is_time_text(b) && (is_calendar_text(a) || is_calendar_text(b)))
}

fn display_name(m: &Model, norm: &str) -> String {
    m.by_name(norm).map(|v| v.name.clone()).unwrap_or_else(|| norm.to_string())
}

/// Ops that add a conversion constant (once) and put it in the "Conversions" group.
fn add_conversion(m: &Model, name: &str, unit_text: &str, factor: f64, doc: &str) -> Vec<Op> {
    if m.by_name(&expr::normalize(name)).is_some() {
        return Vec::new();
    }
    vec![
        Op::AddVar { name: name.into(), kind: "constant".into(), equation: clean_factor(factor), units: unit_text.into(), doc: doc.into(), from: None, to: None, non_negative: false, layer: None },
        Op::AddToGroup { name: "Conversions".into(), vars: vec![name.into()] },
    ]
}

/// The units of a variable as declared (simplified); None when it declares none.
fn declared_text(m: &Model, bare: Option<&str>) -> Option<String> {
    let v = bare.and_then(|n| m.by_name(n)).filter(|v| !v.units.trim().is_empty())?;
    Some(simplify_unit_text(&v.units))
}

/// A readable form of a unit for a message when no declared text is at hand (base units: never put in a fix).
fn shown(u: &Unit) -> String {
    simplify_unit_text(&u.show_exact())
}

/// A term of an equation whose unit differs from the one the equation should be in.
pub(crate) fn term_mismatch(m: &Model, owner: &Var, term: &Expr, tu: &Unit, exp: &Unit, exp_text: &str, en: &Engine) -> Finding {
    let bare = if let Expr::Var(n) = term { Some(n.as_str()) } else { None };
    let term_text = declared_text(m, bare).or_else(|| text_of(m, term));
    build(m, owner, bare, tu, exp, Some(simplify_unit_text(exp_text)), term_text, en, terms(&expr::parse(&owner.equation).unwrap_or(Expr::Num(0.0))).len() == 1)
}

/// An exponent that carries units: divide it by one unit of itself (a constant in the Conversions group).
fn exponent(m: &Model, owner: &Var, c: &Clash, en: &Engine) -> Finding {
    let what = match c.right_var.as_deref() {
        Some("time") => "Time".to_string(),
        Some(b) => display_name(m, b),
        None => "an exponent".into(),
    };
    let unit_text = match c.right_var.as_deref() {
        Some("time") => Some(simplify_unit_text(m.settings.time_units.trim())),
        other => declared_text(m, other),
    };
    let shown_text = unit_text.clone().unwrap_or_else(|| shown(&c.left));
    let mut f = Finding::new("unit-dimension-mismatch", Severity::Error, Some(&owner.name), format!("“{}”: an exponent must be a number without units, but {what} is in {shown_text}", owner.name));
    f.source = "Dynamist units rules (dimensional analysis: an exponent is dimensionless)".into();
    let (Some(var), Some(text)) = (c.right_var.as_deref(), unit_text) else { return f };
    // the single token that follows a ^ and names the variable
    let Ok(toks) = expr::tokenize(&owner.equation) else { return f };
    let span = toks.windows(2).find_map(|w| match (&w[0].0, &w[1].0) {
        (expr::Tok::Op("^"), expr::Tok::Ident(n)) if expr::normalize(n) == var => Some(w[1].1.clone()),
        _ => None,
    });
    let Some(span) = span else { return f };
    let name = if units::parse_unit_exact(&text).is_ok_and(|u| u.dims.len() == 1 && u.dims.contains_key("s")) { format!("{} Length", sanitize(&text)) } else { sanitize(&format!("One {text}")) };
    let new_eq = format!("{}({} / {}){}", &owner.equation[..span.start], what, name, &owner.equation[span.end..]);
    let mut ops = add_conversion(m, &name, &text, 1.0, &format!("One {text}: the unit that an exponent measured in {text} is divided by"));
    ops.push(Op::SetEquation { name: owner.name.clone(), equation: new_eq });
    // the numbers stay when the exponent is in the model's own time unit
    let keeps = en.time.as_ref().is_some_and(|t| *t == c.left);
    f.fixes.push(Fix { title: format!("Divide the exponent {what} by one unit of itself: ^({what} / {name}), with “{name}” = 1 {text}"), ops, keeps_numbers: keeps, needs_decision: false });
    f
}

pub(crate) fn clash(m: &Model, owner: &Var, c: &Clash, en: &Engine) -> Finding {
    if c.what == "exponent" {
        return exponent(m, owner, c, en);
    }
    if c.what == "money" {
        let show = |y: Option<i32>| y.map(|y| format!("real {y}")).unwrap_or_else(|| "nominal".into());
        let mut f = Finding::new(
            "money-real-nominal",
            Severity::Error,
            Some(&owner.name),
            format!("“{}” adds or compares money in different price bases: {} and {}. Convert one with REAL RATE or NOMINAL RATE (an inflation index) first", owner.name, show(c.left.real_year), show(c.right.real_year)),
        );
        f.source = "Dynamist units rules (real and nominal money, Excel/NREL practice)".into();
        return f;
    }
    let exp_text = declared_text(m, c.left_var.as_deref());
    let term_text = declared_text(m, c.right_var.as_deref());
    build(m, owner, c.right_var.as_deref(), &c.right, &c.left, exp_text, term_text, en, false)
}

#[allow(clippy::too_many_arguments)]
fn build(m: &Model, owner: &Var, bare: Option<&str>, tu: &Unit, exp: &Unit, exp_text: Option<String>, term_text: Option<String>, en: &Engine, whole_equation: bool) -> Finding {
    let term_name = bare.map(|b| display_name(m, b)).unwrap_or_else(|| "a term".into());
    // fixes are written in declared texts only; a message may show the base units when nothing is declared
    let (exp_declared, term_declared) = (exp_text.clone(), term_text.clone());
    let exp_text = exp_text.unwrap_or_else(|| shown(exp));
    let term_text = term_text.unwrap_or_else(|| shown(tu));
    let same_dims = tu.dims == exp.dims;
    let factor = tu.scale / exp.scale;
    // which stock the equation feeds, for the message
    let stock = [owner.to, owner.from].into_iter().flatten().filter_map(|id| m.var(id)).find(|s| s.kind == Kind::Stock);
    let target = match stock {
        Some(s) => format!("{} ({})", s.name, s.units.trim()),
        None => format!("{} ({})", owner.name, exp_text),
    };
    let (code, message) = if same_dims {
        (
            "unit-scale-mismatch",
            format!("Scale mismatch: {term_name} ({term_text}) is added to {target}: the same kind of quantity but a factor {} apart{} ({term_name} × {} would be in {exp_text})", super::num(factor), week_note(&term_text, &exp_text), clean_factor(factor)),
        )
    } else {
        (
            "unit-dimension-mismatch",
            format!("Dimension mismatch: {term_name} ({term_text}) meets {target}; they differ by {}", units::show(&tu.div(exp).dims)),
        )
    };
    let mut f = Finding::new(code, Severity::Error, Some(&bare.map(|b| display_name(m, b)).unwrap_or_else(|| owner.name.clone())), message);
    f.source = "Dynamist units rules (dimensional analysis)".into();
    let mut fixes: Vec<Fix> = Vec::new();
    let new_equation = |factor_text: &str, op: &str| -> Option<String> {
        if let Some(b) = bare {
            Some(expr::rename_in(&owner.equation, b, &format!("({} {op} {factor_text})", display_name(m, b))))
        } else if whole_equation {
            Some(format!("({}) {op} {factor_text}", owner.equation.trim()))
        } else {
            None
        }
    };
    // (a) a pure scale factor
    if same_dims {
        if let (Some(from), Some(to)) = (&term_declared, &exp_declared) {
            let (name, utext) = conversion_names(from, to);
            if let Some(eq) = new_equation(&name, "*") {
                let decision = is_calendar_conversion(from, to);
                let mut ops = add_conversion(m, &name, &utext, factor, &format!("Conversion from {from} to {to}"));
                ops.push(Op::SetEquation { name: owner.name.clone(), equation: eq });
                let title = format!("Add the constant “{name}” = {} and multiply {term_name} by it in {}{}", clean_factor(factor), owner.name, if decision { " (a calendar choice: check the number)" } else { "" });
                fixes.push(Fix { title, ops, keeps_numbers: false, needs_decision: decision });
            }
        }
    } else if let Some(b) = bare {
        // (b) a factor with a dimension: a variable of the model whose unit is the missing factor (or its inverse)
        let missing = exp.div(tu);
        for cand in m.vars.iter().filter(|c| c.id != owner.id && !c.units.trim().is_empty() && c.kind != Kind::Stock) {
            let Ok(cu) = units::parse_unit_in(&cand.units, &en.ctx) else { continue };
            let (mult, ok) = if cu == missing { ("*", true) } else if Unit::one().div(&cu) == missing { ("/", true) } else { ("", false) };
            if ok {
                let eq = expr::rename_in(&owner.equation, b, &format!("({} {mult} {})", display_name(m, b), cand.name));
                fixes.push(Fix { title: format!("{} {term_name} {} “{}” ({}) in {}", if mult == "*" { "Multiply" } else { "Divide" }, if mult == "*" { "by" } else { "by" }, cand.name, cand.units.trim(), owner.name), ops: vec![Op::SetEquation { name: owner.name.clone(), equation: eq }], keeps_numbers: false, needs_decision: false });
                break;
            }
        }
    }
    // (c) a declaration that makes the equation consistent
    if let Some(b) = bare {
        if let Some(tv) = m.by_name(b) {
            if tv.kind != Kind::Stock && exp_declared.is_some() && !exp_text.contains("{") {
                fixes.push(Fix {
                    title: format!("Declare “{}” in {} (its number {} then means {})", tv.name, exp_text, tv.equation.trim(), exp_text),
                    ops: vec![Op::SetUnits { name: tv.name.clone(), units: exp_text.to_string() }],
                    keeps_numbers: true, needs_decision: false,
                });
            }
        }
    }
    f.fixes = fixes;
    f
}

/// An equation whose unit differs from the one the variable declares.
pub(crate) fn declared_mismatch(m: &Model, v: &Var, got: &Unit, declared: &Unit, en: &Engine) -> Finding {
    let same_dims = got.dims == declared.dims;
    let factor = got.scale / declared.scale;
    let msg = if same_dims {
        format!("“{}” is declared in {} but its equation gives a value a factor {} off{}", v.name, v.units.trim(), super::num(factor), expr::parse(&v.equation).ok().and_then(|e| text_of(m, &e)).map_or("", |g| week_note(&g, &v.units)))
    } else {
        format!("“{}” is declared in {} but its equation gives {}", v.name, v.units.trim(), units::show(&got.dims))
    };
    let mut f = Finding::new(if same_dims { "unit-scale-mismatch" } else { "unit-declared" }, Severity::Error, Some(&v.name), msg);
    f.source = "Dynamist units rules (dimensional analysis)".into();
    // a factor of a percent or a ppm with the scale setting off: turning it on is the first remedy
    if same_dims && !en.scale_values && [0.01, 100.0, 1e-6, 1e6].iter().any(|k| same_scale(factor, *k)) {
        f.fixes.push(Fix { title: "Turn units_scale_values on: a constant declared in % or ppm then carries its scale (5 % is 0.05)".into(), ops: vec![Op::SetSetting { key: "units_scale_values".into(), value: "true".into() }], keeps_numbers: false, needs_decision: false });
    }
    if same_dims {
        let got_text = expr::parse(&v.equation).ok().and_then(|e| text_of(m, &e));
        let (name, utext, decision) = match &got_text {
            Some(g) => {
                let (n, u) = conversion_names(g, &v.units);
                (n, u, is_calendar_conversion(g, &v.units))
            }
            None => (format!("{} correction", v.name), "dmnl".to_string(), false),
        };
        let mut ops = add_conversion(m, &name, &utext, factor, &format!("Scale between the equation of {} and its declared units", v.name));
        ops.push(Op::SetEquation { name: v.name.clone(), equation: format!("({}) * {}", v.equation.trim(), name) });
        f.fixes.push(Fix { title: format!("Multiply the equation of {} by the constant “{}” = {}{}", v.name, name, clean_factor(factor), if decision { " (a calendar choice: check the number)" } else { "" }), ops, keeps_numbers: false, needs_decision: decision });
    }
    f
}

/// A flow declared in a unit that, per time unit, is not its stock's.
pub(crate) fn flow_stock(m: &Model, flow: &Var, stock: &Var, per_time: &Unit, stock_unit: &Unit, en: &Engine) -> Finding {
    let _ = en;
    let same_dims = per_time.dims == stock_unit.dims;
    let factor = per_time.scale / stock_unit.scale;
    let msg = format!(
        "Flow “{}” ({}) fills or drains “{}” ({}): per time unit it should be in {}{}",
        flow.name,
        flow.units.trim(),
        stock.name,
        stock.units.trim(),
        stock.units.trim(),
        if same_dims { format!(" (a factor {} apart{})", super::num(factor), week_note(flow.units.trim(), &format!("{}/{}", stock.units.trim(), m.settings.time_units.trim()))) } else { String::new() }
    );
    let mut f = Finding::new(if same_dims { "unit-scale-mismatch" } else { "unit-flow" }, Severity::Error, Some(&flow.name), msg);
    f.source = "Dynamist units rules (flow × time = stock)".into();
    let ttext = m.settings.time_units.trim();
    let want = format!("{}/{}", stock.units.trim(), ttext);
    if same_dims {
        if let Ok(Expr::Num(x)) = expr::parse(&flow.equation) {
            // a plain number is converted where it stands
            let decision = is_calendar_conversion(flow.units.trim(), &want);
            f.fixes.push(Fix { title: format!("Write {} in {}: {} becomes {}{}", flow.name, want, clean_factor(x), clean_factor(x / factor), if decision { " (a calendar choice: check the number)" } else { "" }), ops: vec![Op::SetEquation { name: flow.name.clone(), equation: clean_factor(x / factor) }, Op::SetUnits { name: flow.name.clone(), units: want.clone() }], keeps_numbers: false, needs_decision: decision });
        }
    }
    f.fixes.push(Fix { title: format!("Declare “{}” in {}", flow.name, want), ops: vec![Op::SetUnits { name: flow.name.clone(), units: want }], keeps_numbers: true, needs_decision: false });
    f
}

/// A unit symbol with several meanings: one fix per meaning.
pub(crate) fn ambiguous(sym: &str, var: &str) -> Finding {
    let meanings: Vec<&'static str> = unit_table::lookup(sym, 365.25).map(|a| a.meanings.iter().map(|m| m.label).collect()).unwrap_or_default();
    let mut f = Finding::new(
        "unit-ambiguous",
        Severity::Warning,
        Some(var),
        format!("The unit symbol “{sym}” (first used in “{var}”) has more than one meaning: {}. Choose the one the model means", meanings.join(" or ")),
    );
    f.source = "Dynamist unit table".into();
    for lab in meanings {
        f.fixes.push(Fix { title: format!("Read “{sym}” as {lab}"), ops: vec![Op::SetSetting { key: "unit_choice".into(), value: format!("{sym}={lab}") }], keeps_numbers: true, needs_decision: false });
    }
    f
}

/// units_scale_values is off and a literal is declared in percent or ppm: turning it on would change the value.
pub(crate) fn scale_change(var: &str, value: f64, scale: f64, units: &str) -> Finding {
    let mut f = Finding::new(
        "unit-scale-change",
        Severity::Info,
        Some(var),
        format!("“{var}” is declared in {}: with units_scale_values on its value would change from {} to {}", units.trim(), super::num(value), super::num(value * scale)),
    );
    f.source = "Dynamist units rules".into();
    f.fixes.push(Fix { title: "Turn units_scale_values on (declared units then carry value scale)".into(), ops: vec![Op::SetSetting { key: "units_scale_values".into(), value: "true".into() }], keeps_numbers: false, needs_decision: false });
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_texts_are_simplified_and_conversions_are_named_from_declared_texts() {
        assert_eq!(simplify_unit_text("(kWh/Year)*Dmnl"), "kWh/Year");
        assert_eq!(simplify_unit_text("MB*Day/Day"), "MB");
        assert_eq!(simplify_unit_text("Dmnl/Year"), "1/Year");
        assert_eq!(simplify_unit_text("Unit (approx)/Day"), "Unit (approx)/Day");
        assert_eq!(conversion_names("Week", "Day"), ("Days per Week".to_string(), "Day/Week".to_string()));
        assert_eq!(conversion_names("ticket/Week", "ticket/Day"), ("Weeks per Day".to_string(), "Week/Day".to_string()));
        assert!(is_calendar_conversion("Week", "Day") && is_calendar_conversion("ticket/Month", "ticket/Day") && is_calendar_conversion("Year", "Day"));
        assert!(!is_calendar_conversion("Hour", "Minute") && !is_calendar_conversion("MB", "GB"));
    }

    #[test]
    fn a_week_to_day_fix_is_a_decision_with_a_readable_name_and_is_not_applied_automatically() {
        let mut m = Model::default();
        m.settings.time_units = "Day".into();
        m.settings.final_time = 10.0;
        let mut a = Var::new(1, "Ticket Week", Kind::Constant, [0.0, 0.0]);
        a.equation = "2".into();
        a.units = "Week".into();
        let mut b = Var::new(2, "Cycle Time", Kind::Aux, [0.0, 0.0]);
        b.equation = "Ticket Week".into();
        b.units = "Day".into();
        m.vars = vec![a, b];
        m.next_id = 10;
        let r = super::super::check(&m, &super::super::Options { units_only: true, ..Default::default() });
        let f = r.findings.iter().find(|f| f.code == "unit-scale-mismatch").expect("a scale mismatch");
        let fix = f.fixes.iter().find(|x| x.title.contains("constant")).expect("a named constant");
        assert!(fix.needs_decision && !fix.keeps_numbers, "{fix:?}");
        assert!(fix.title.contains("Days per Week") && !fix.title.contains("157"), "{}", fix.title);
        let (after, n) = super::super::apply_safe_fixes(&m, &r);
        assert_eq!(n, 0);
        assert_eq!(after.vars.len(), m.vars.len());
    }
}

#[allow(dead_code)]
fn _keep(_: bool) -> bool {
    same_scale(1.0, 1.0)
}
