//! Writing the classic Vensim text model format (`.mdl`), including the sketch.
//!
//! The file is written from the model alone: ranges (with mappings) for sets, one equation per
//! variable (or per item where a formula needs it), the control variables, and one sketch view per
//! diagram sheet. Everything that has no place in a `.mdl` file, or that was written in a
//! different but equivalent form, is listed in the returned [`LossReport`].
//!
//! Not verified against Vensim itself (no copy was available): the file is checked by reading it
//! back with [`crate::mdl::read`]. See `wiki/Format-Mapping-Vensim.md` for the assumptions.

use std::collections::HashSet;

use crate::expr;
use crate::loss::LossReport;
use crate::mdl::{LK, Lx, lex};
use crate::model::{ItemSet, Kind, Method, Model, Rollup, SheetKind, Var};

fn nrm(s: &str) -> String {
    expr::normalize(s)
}

/// A name the way Vensim wants it: quoted when it holds anything but letters, digits, spaces,
/// underscores, `$` and `'`.
fn vname(n: &str) -> String {
    let plain = !n.is_empty()
        && n == n.trim()
        && !n.starts_with(|c: char| c.is_ascii_digit())
        && n.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '$' | '\''));
    if plain { n.to_string() } else { format!("\"{}\"", n.replace('"', "'")) }
}

fn num(x: f64) -> String {
    format!("{x}")
}

fn comment_text(s: &str) -> String {
    s.replace('|', "/").replace('~', "-").replace('\n', " ")
}

fn unq(s: &str) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

fn split_top(s: &str) -> Vec<String> {
    let (mut out, mut cur) = (Vec::new(), String::new());
    let (mut depth, mut quote) = (0i32, false);
    for c in s.chars() {
        match c {
            '"' => {
                quote = !quote;
                cur.push(c);
            }
            '(' | '[' if !quote => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' if !quote => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 && !quote => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

fn lt<'a>(src: &'a str, t: &Lx) -> &'a str {
    &src[t.s..t.e]
}

fn is_sym(src: &str, t: Option<&Lx>, s: &str) -> bool {
    t.is_some_and(|t| t.k == LK::Sym && lt(src, t) == s)
}

fn matching(src: &str, toks: &[Lx], open: usize) -> Option<usize> {
    let (o, c) = if lt(src, &toks[open]) == "(" { ("(", ")") } else { ("[", "]") };
    let mut depth = 0;
    for (k, t) in toks.iter().enumerate().skip(open) {
        if t.k == LK::Sym {
            let s = lt(src, t);
            if s == o {
                depth += 1;
            } else if s == c {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
        }
    }
    None
}

/// `&&`, `||`, `!`, `!=` and `==` in our spelling → Vensim's.
fn operators(s: &str) -> String {
    let (mut out, mut quote) = (String::new(), false);
    let ch: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        let c = ch[i];
        if c == '"' {
            quote = !quote;
        }
        if !quote {
            let two: String = ch[i..(i + 2).min(ch.len())].iter().collect();
            match two.as_str() {
                "&&" => {
                    out.push_str(" :AND: ");
                    i += 2;
                    continue;
                }
                "||" => {
                    out.push_str(" :OR: ");
                    i += 2;
                    continue;
                }
                "!=" => {
                    out.push_str("<>");
                    i += 2;
                    continue;
                }
                "==" => {
                    out.push('=');
                    i += 2;
                    continue;
                }
                _ => {}
            }
            if c == '!' {
                out.push_str(" :NOT: ");
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// What the formula being written belongs to: the set it is per, and the item if it is written
/// for one item.
#[derive(Clone, Copy)]
struct Wc<'a> {
    set: Option<&'a ItemSet>,
    elem: Option<usize>,
}

struct W<'a> {
    m: &'a Model,
    loss: LossReport,
    said: HashSet<String>,
}

impl<'a> W<'a> {
    fn once(&mut self, k: &str) -> bool {
        self.said.insert(k.to_string())
    }

    fn set_named(&self, n: &str) -> Option<&'a ItemSet> {
        self.m.sets.iter().find(|s| nrm(&s.name) == nrm(n))
    }

    fn set_of(&self, v: &Var) -> Option<&'a ItemSet> {
        v.set.as_deref().and_then(|s| self.set_named(s))
    }

    // ---- references ---------------------------------------------------------------------

    /// A reference to variable `v` inside a formula.
    fn var_ref(&mut self, v: &Var, c: Wc, needs_elem: &mut bool) -> String {
        let name = vname(&v.name);
        let Some(vs) = self.set_of(v) else { return name };
        let Some(cs) = c.set else { return name };
        if nrm(&cs.name) == nrm(&vs.name) {
            return match c.elem {
                Some(k) => format!("{name}[{}]", vname(&cs.items[k])),
                None => format!("{name}[{}]", vname(&cs.name)),
            };
        }
        if cs.parent.as_ref().is_some_and(|p| nrm(p) == nrm(&vs.name)) {
            // read through the group an item belongs to: written item by item
            *needs_elem = true;
            return match c.elem.and_then(|k| cs.parent_item(k)) {
                Some(p) => format!("{name}[{}]", vname(p)),
                None => format!("{name}[{}]", vname(&vs.name)),
            };
        }
        if self.once(&format!("badref {} {}", cs.name, v.name)) {
            self.loss.approximated(format!("reference to {}", v.name), format!("is per {} but is read from a formula per {}: written without a subscript", vs.name, cs.name));
        }
        name
    }

    /// `AGE(stock, every=, open_ended=)` → the class's net ageing flow, written out for the item (Vensim's
    /// SHIFT IF TRUE changes the variable itself, so a flow formula is used instead).
    fn age(&mut self, args: &[String], c: Wc, needs_elem: &mut bool) -> Option<String> {
        let stock = self.m.by_name(&nrm(args.first()?))?;
        let set = self.set_of(stock)?;
        let Some(k) = c.elem else {
            *needs_elem = true;
            return Some("0".into());
        };
        let mut named: Vec<(String, String)> = Vec::new();
        for a in args.iter().skip(1) {
            let (key, v) = a.split_once('=')?;
            named.push((nrm(key), v.trim().to_string()));
        }
        let get = |key: &str| named.iter().find(|(kk, _)| *kk == nrm(key)).map(|(_, v)| v.clone());
        let every = match get("every") {
            Some(e) => self.tr_inner(&e, c, needs_elem),
            None => "1".to_string(),
        };
        let open_ended = !matches!(get("open_ended").map(|t| t.to_lowercase()).as_deref(), Some("false") | Some("0"));
        let sn = vname(&stock.name);
        let own = format!("{sn}[{}]", vname(&set.items[k]));
        let prev = if k > 0 {
            format!("{sn}[{}]", vname(&set.items[k - 1]))
        } else {
            match get("entrants") {
                Some(e) => format!("(({}) * ({every}))", self.tr_inner(&e, c, needs_elem)),
                None => "0".to_string(),
            }
        };
        let leaving = if k + 1 == set.items.len() && open_ended { "0".to_string() } else { own };
        if self.once("age") {
            self.loss.approximated("AGE", "written as the class's net ageing flow, a formula that moves whole classes at each period (Vensim's SHIFT IF TRUE changes the stock itself)");
        }
        Some(format!("(IF THEN ELSE(MODULO(Time + TIME STEP - INITIAL TIME + TIME STEP / 2, {every}) < TIME STEP :AND: Time + TIME STEP > INITIAL TIME + TIME STEP / 2, ({prev} - {leaving}) / TIME STEP, 0))"))
    }

    /// `ALLOCATE(requests, supply, by=, overlap=)` → Vensim's ALLOCATE BY PRIORITY, or the proportional share as a
    /// formula. With `curve=` there is no one-line Vensim form (it needs a profile table): kept as written, reported.
    fn allocate(&mut self, args: &[String], c: Wc, needs_elem: &mut bool) -> Option<String> {
        let req = self.m.by_name(&nrm(args.first()?))?;
        let set = self.set_of(req)?;
        let sn = vname(&set.name);
        let mut named: Vec<(String, String)> = Vec::new();
        for a in args.iter().skip(2) {
            let (k, v) = a.split_once('=')?;
            named.push((nrm(k), v.trim().to_string()));
        }
        let get = |key: &str| named.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        if get("curve").is_some() {
            if self.once("allocate curve") {
                self.loss.preserved_not_simulable("ALLOCATE with curve=", "Vensim's ALLOCATE AVAILABLE needs a table of profiles; the call is kept as written, which Vensim will not read");
            }
            return None;
        }
        let supply = self.tr_inner(args.get(1)?, c, needs_elem);
        let rn = vname(&req.name);
        if let Some(b) = get("by") {
            let pri = self.m.by_name(&nrm(&b))?;
            let width = match get("overlap") {
                Some(w) => self.tr_inner(&w, c, needs_elem),
                None => "0.000000001".to_string(),
            };
            return Some(format!("ALLOCATE BY PRIORITY({rn}[{sn}!], {}[{sn}!], ELMCOUNT({sn}), {width}, {supply})", vname(&pri.name)));
        }
        // in proportion to what each asks; everyone gets it all when the supply covers the requests
        let mine = self.var_ref(req, c, needs_elem);
        if self.once("allocate proportional") {
            self.loss.approximated("ALLOCATE", "written as the proportional share, a formula: Vensim has no function for sharing without priorities");
        }
        Some(format!("{mine} * MIN(1, ZIDZ({supply}, SUM({rn}[{sn}!])))"))
    }

    /// `TOTAL(y)` / `AVERAGE(y)` / with a group label.
    fn aggregate(&mut self, average: bool, args: &[String], c: Wc, needs_elem: &mut bool) -> Option<String> {
        let y = self.m.by_name(&nrm(args.first()?))?;
        let ys = self.set_of(y)?;
        let label = args.get(1).map(|l| nrm(&unq(l)));
        let all: Vec<usize> = (0..ys.items.len()).collect();
        let idx: Vec<usize> = if let Some(l) = &label {
            all.iter().copied().filter(|&i| nrm(&ys.items[i]) == *l || ys.parent_item(i).is_some_and(|p| nrm(p) == *l)).collect()
        } else if let (Some(cs), Some(parent)) = (c.set, ys.parent.as_ref()) {
            if nrm(parent) == nrm(&cs.name) && nrm(&cs.name) != nrm(&ys.name) {
                match c.elem {
                    Some(k) => {
                        let me = &cs.items[k];
                        all.iter().copied().filter(|&i| ys.parent_item(i) == Some(me.as_str())).collect()
                    }
                    None => {
                        *needs_elem = true;
                        all.clone()
                    }
                }
            } else {
                all.clone()
            }
        } else {
            all.clone()
        };
        let yn = vname(&y.name);
        if idx.len() == ys.items.len() {
            let sum = format!("SUM({yn}[{}!])", vname(&ys.name));
            return Some(if average { format!("({sum} / ELMCOUNT({}))", vname(&ys.name)) } else { sum });
        }
        if idx.is_empty() {
            return Some("0".into());
        }
        let terms: Vec<String> = idx.iter().map(|&i| format!("{yn}[{}]", vname(&ys.items[i]))).collect();
        let sum = format!("({})", terms.join(" + "));
        Some(if average { format!("({sum} / {})", idx.len()) } else { sum })
    }

    // ---- formulas -----------------------------------------------------------------------

    /// Our formula → Vensim's.
    fn tr(&mut self, src: &str, c: Wc, needs_elem: &mut bool) -> String {
        let src = match crate::userfn::expand_text(src) {
            Ok(s) => s,
            Err(_) => src.to_string(),
        };
        let src = operators(&src);
        self.tr_inner(&src, c, needs_elem)
    }

    fn tr_inner(&mut self, src: &str, c: Wc, needs_elem: &mut bool) -> String {
        let toks = lex(src);
        let mut out = String::new();
        let mut last = 0;
        let mut i = 0;
        while i < toks.len() {
            let t = toks[i];
            if matches!(t.k, LK::Word | LK::Quoted) {
                let raw = unq(lt(src, &t));
                if is_sym(src, toks.get(i + 1), "(") {
                    if let Some(close) = matching(src, &toks, i + 1) {
                        let inner = &src[toks[i + 1].e..toks[close].s];
                        let args_raw = if inner.trim().is_empty() { vec![] } else { split_top(inner) };
                        let rep = self.call(&raw, &args_raw, c, needs_elem);
                        out.push_str(&src[last..t.s]);
                        out.push_str(&rep);
                        last = toks[close].e;
                        i = close + 1;
                        continue;
                    }
                }
                if is_sym(src, toks.get(i + 1), "[") {
                    if let Some(close) = matching(src, &toks, i + 1) {
                        let item = unq(&src[toks[i + 1].e..toks[close].s]);
                        let rep = match self.m.by_name(&nrm(&raw)).and_then(|v| self.set_of(v).map(|s| (v, s))) {
                            Some((v, s)) => {
                                let it = s.items.iter().find(|x| nrm(x) == nrm(&item)).cloned().unwrap_or(item.clone());
                                format!("{}[{}]", vname(&v.name), vname(&it))
                            }
                            None => format!("{}[{}]", vname(&raw), vname(&item)),
                        };
                        out.push_str(&src[last..t.s]);
                        out.push_str(&rep);
                        last = toks[close].e;
                        i = close + 1;
                        continue;
                    }
                }
                let n = nrm(&raw);
                let rep = match n.as_str() {
                    "time" => "Time".to_string(),
                    "initial time" => "INITIAL TIME".to_string(),
                    "final time" => "FINAL TIME".to_string(),
                    "time step" | "dt" => "TIME STEP".to_string(),
                    _ => match self.m.by_name(&n) {
                        Some(v) => self.var_ref(v, c, needs_elem),
                        None => vname(&raw),
                    },
                };
                out.push_str(&src[last..t.s]);
                out.push_str(&rep);
                last = t.e;
            }
            i += 1;
        }
        out.push_str(&src[last..]);
        out
    }

    /// One function call: `args_raw` are our argument texts.
    fn call(&mut self, name: &str, args_raw: &[String], c: Wc, needs_elem: &mut bool) -> String {
        let up = nrm(name).to_uppercase();
        if up == "TOTAL" || up == "AVERAGE" {
            if let Some(s) = self.aggregate(up == "AVERAGE", args_raw, c, needs_elem) {
                return s;
            }
        }
        if up == "AGE" {
            if let Some(s) = self.age(args_raw, c, needs_elem) {
                return s;
            }
        }
        if up == "ALLOCATE" {
            if let Some(s) = self.allocate(args_raw, c, needs_elem) {
                return s;
            }
        }
        if up == "WITH LOOKUP" && args_raw.len() >= 2 {
            let x = self.tr_inner(&args_raw[0], c, needs_elem);
            let table_src = args_raw[1..].join(", ");
            if let Ok(t) = expr::parse_table(table_src.trim()) {
                return format!("WITH LOOKUP({x}, ({}))", table_text(&t));
            }
        }
        let a: Vec<String> = args_raw.iter().map(|x| self.tr_inner(x, c, needs_elem)).collect();
        let arg = |i: usize| a.get(i).cloned().unwrap_or_else(|| "0".into());
        let lit = |i: usize| args_raw.get(i).and_then(|x| x.trim().parse::<f64>().ok());
        let join = |f: &str| format!("{f}({})", a.join(", "));
        match up.as_str() {
            "IF THEN ELSE" | "ABS" | "SQRT" | "EXP" | "LN" | "INTEGER" | "SIN" | "COS" | "TAN" | "ARCSIN" | "ARCCOS" | "ARCTAN" | "SINH" | "COSH" | "TANH" | "XIDZ" | "ZIDZ" | "STEP" | "PULSE" | "PULSE TRAIN" | "RAMP" | "SMOOTH" | "SMOOTHI" | "SMOOTH3" | "SMOOTH3I" | "DELAY1" | "DELAY1I" | "DELAY3" | "DELAY3I" | "DELAY FIXED" | "TREND" | "INITIAL" | "POWER" | "LOG" => join(&up),
            "QUEUE AGE AVERAGE" | "QUEUE AGE OLDEST" | "QUEUE AGE IN RANGE" => join(&up),
            "QUEUE FIFO" => {
                if self.once("queue fifo") {
                    self.loss.approximated("QUEUE FIFO", "written with three arguments (inflow, outflow, initial); Vensim's version also wants a lookup that spreads the starting crowd over ages and an age range, which a Vensim user adds");
                }
                join("QUEUE FIFO")
            }
            "MODULO" => {
                if self.once("modulo") {
                    self.loss.approximated("MODULO", "the engine's result is never negative for a positive divisor; Vensim's sign convention for negative inputs may differ");
                }
                join("MODULO")
            }
            "RANDOM UNIFORM" | "RANDOM NORMAL" => {
                if self.once("random") {
                    self.loss.approximated(up.clone(), "the same distribution, but Vensim draws a different stream of random numbers");
                }
                join(&up)
            }
            "MIN" | "MAX" => {
                let mut it = a.into_iter().rev();
                let mut acc = it.next().unwrap_or_else(|| "0".into());
                for x in it {
                    acc = format!("{up}({x}, {acc})");
                }
                acc
            }
            "ROUND" => format!("IF THEN ELSE({0} >= 0, INTEGER({0} + 0.5), -INTEGER(-({0}) + 0.5))", arg(0)),
            "FLOOR" => format!("IF THEN ELSE({0} >= 0, INTEGER({0}), INTEGER({0}) - IF THEN ELSE({0} = INTEGER({0}), 0, 1))", arg(0)),
            "CEILING" => format!("IF THEN ELSE({0} <= 0, INTEGER({0}), INTEGER({0}) + IF THEN ELSE({0} = INTEGER({0}), 0, 1))", arg(0)),
            "SIGN" => format!("IF THEN ELSE({0} > 0, 1, IF THEN ELSE({0} < 0, -1, 0))", arg(0)),
            "CBRT" => format!("IF THEN ELSE({0} >= 0, ({0}) ^ (1 / 3), -((-({0})) ^ (1 / 3)))", arg(0)),
            "HYPOT" => format!("SQRT(({}) ^ 2 + ({}) ^ 2)", arg(0), arg(1)),
            "RADIANS" => format!("({}) * 3.141592653589793 / 180", arg(0)),
            "DEGREES" => format!("({}) * 180 / 3.141592653589793", arg(0)),
            "LOG10" => format!("LOG({}, 10)", arg(0)),
            "LOG2" => format!("LOG({}, 2)", arg(0)),
            "CLAMP" => format!("MIN(MAX({}, {}), {})", arg(0), arg(1), arg(2)),
            "MEAN" => format!("(({}) / {})", a.join(" + "), a.len().max(1)),
            "PI" => "3.141592653589793".into(),
            "SINWAVE" | "COSWAVE" => {
                let f = if up == "SINWAVE" { "SIN" } else { "COS" };
                let start = if a.len() > 2 { arg(2) } else { "INITIAL TIME".into() };
                format!("({}) * {f}(6.283185307179586 * (Time - ({start})) / ({}))", arg(0), arg(1))
            }
            "GRAPH" => {
                // GRAPH(x, x0, dx, y1, y2, …) is a lookup spaced evenly from x0
                match (lit(1), lit(2)) {
                    (Some(x0), Some(dx)) if args_raw.len() > 4 && (3..args_raw.len()).all(|k| lit(k).is_some()) => {
                        let pts: Vec<[f64; 2]> = (3..args_raw.len()).map(|k| [x0 + dx * (k - 3) as f64, lit(k).unwrap()]).collect();
                        format!("WITH LOOKUP({}, ({}))", arg(0), table_text(&pts))
                    }
                    _ => self.raw(&up, &a, "its start, step and values must be plain numbers to be written as a lookup"),
                }
            }
            "DELAYINF" | "DELAYMTR" => {
                let order = lit(2).filter(|n| n.fract() == 0.0 && *n >= 1.0).map(|n| n as u32);
                let (x, d) = (arg(0), arg(1));
                match (up.as_str(), order) {
                    ("DELAYINF", Some(1)) => format!("SMOOTH({x}, {d})"),
                    ("DELAYINF", Some(3)) => format!("SMOOTH3({x}, {d})"),
                    ("DELAYMTR", Some(1)) => format!("DELAY1({x}, {d})"),
                    ("DELAYMTR", Some(3)) => format!("DELAY3({x}, {d})"),
                    (f, Some(n)) => {
                        if self.once("ndelay") {
                            self.loss.approximated(f.to_string(), "written with Vensim's SMOOTH N / DELAY N for orders other than 1 and 3; the spelling of those two functions was not checked against Vensim");
                        }
                        format!("{}({x}, {d}, {x}, {n})", if f == "DELAYINF" { "SMOOTH N" } else { "DELAY N" })
                    }
                    (f, None) => self.raw(f, &a, "the order must be a plain whole number"),
                }
            }
            // Vensim has only the logarithm of the gamma function (GAMMA LN), not the function
            "GAMMA" => {
                if self.once("gamma") {
                    self.loss.approximated("GAMMA", "written as EXP(GAMMA LN(x)): Vensim has only the logarithm of the gamma function; equal up to floating-point rounding");
                }
                format!("EXP(GAMMA LN({}))", arg(0))
            }
            "FACTORIAL" => {
                if self.once("factorial") {
                    self.loss.approximated("FACTORIAL", "written as EXP(GAMMA LN(n + 1)): Vensim has no factorial; equal up to floating-point rounding");
                }
                format!("EXP(GAMMA LN(({}) + 1))", arg(0))
            }
            "GRAPHSTEP" | "MEDIAN" | "ERF" | "ARCTAN2" => self.raw(&up, &a, "Vensim has no function with this meaning"),
            _ => {
                // a lookup called like a function, or something Vensim may know under the same name
                if let Some(v) = self.m.by_name(&nrm(name)) {
                    return format!("{}({})", vname(&v.name), a.join(", "));
                }
                self.raw(&up, &a, "not a function the engine knows")
            }
        }
    }

    fn raw(&mut self, up: &str, a: &[String], why: &str) -> String {
        if self.once(&format!("raw {up}")) {
            self.loss.preserved_not_simulable(format!("function {up}"), format!("written unchanged ({why}); Vensim will report it as an unknown function"));
        }
        format!("{up}({})", a.join(", "))
    }
}

fn table_text(t: &[[f64; 2]]) -> String {
    let mut pts = t.to_vec();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let (x0, x1) = (pts.first().map(|p| p[0]).unwrap_or(0.0), pts.last().map(|p| p[0]).unwrap_or(1.0));
    let (y0, y1) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
    let body: Vec<String> = pts.iter().map(|p| format!("({},{})", num(p[0]), num(p[1]))).collect();
    format!("[({},{})-({},{})],{}", num(x0), num(y0), num(x1), num(y1), body.join(","))
}

// ------------------------------------------------------------------ entries

struct Out {
    text: String,
}

impl Out {
    fn entry(&mut self, lhs: &str, rhs: Option<(&str, &str)>, units: &str, comment: &str) {
        match rhs {
            Some((op, r)) => self.text.push_str(&format!("{lhs}{op}\n\t{r}\n")),
            None => self.text.push_str(&format!("{lhs}\n")),
        }
        self.text.push_str(&format!("\t~\t{units}\n\t~\t{}\n\t|\n\n", comment_text(comment)));
    }
}

fn units_text(v: &Var) -> String {
    let u = if v.units.trim().is_empty() { "**undefined**".to_string() } else { v.units.trim().to_string() };
    match v.slider {
        Some([a, b]) if v.kind == Kind::Constant => format!("{u} [{},{}]", num(a), num(b)),
        _ => u,
    }
}

/// Write a model as a Vensim `.mdl` file.
pub fn write(m: &Model) -> (String, LossReport) {
    crate::userfn::set(&m.functions);
    let mut w = W { m, loss: LossReport::default(), said: HashSet::new() };
    let mut out = Out { text: String::from("{UTF-8}\n") };

    // user functions are written out in full at each use
    for f in &m.functions {
        let used = m.vars.iter().any(|v| {
            let toks = lex(&v.equation);
            toks.iter().enumerate().any(|(i, t)| t.k == LK::Word && nrm(lt(&v.equation, t)) == nrm(&f.name) && is_sym(&v.equation, toks.get(i + 1), "("))
        });
        if used {
            w.loss.approximated(format!("function {}", f.name), "written with its formula substituted at each use (no macro)");
        }
    }

    // sets: parents first so a mapping always points at a range that is already defined
    let mut sets: Vec<&ItemSet> = Vec::new();
    for s in &m.sets {
        if s.parent.is_none() {
            sets.push(s);
        }
    }
    for s in &m.sets {
        if s.parent.is_some() {
            sets.push(s);
        }
    }
    for s in &sets {
        let items = s.items.iter().map(|i| vname(i)).collect::<Vec<_>>().join(", ");
        let mapping = match (&s.parent, s.parent_of.len() == s.items.len() && s.parent_of.iter().all(|p| !p.is_empty())) {
            (Some(p), true) => format!(" -> ({}: {})", vname(p), s.parent_of.iter().map(|x| vname(x)).collect::<Vec<_>>().join(", ")),
            (Some(p), false) => {
                w.loss.dropped(format!("set {}", s.name), format!("some items have no {p} to belong to, so the mapping was not written"));
                String::new()
            }
            _ => String::new(),
        };
        out.entry(&format!("{}:", vname(&s.name)), Some((" ", &format!("{items}{mapping}"))), "", &s.note);
    }

    // equations: flows come last-ish in Vensim files, but order does not matter
    for v in &m.vars {
        write_var(&mut w, &mut out, v);
    }

    // control
    out.text.push_str("********************************************************\n\t.Control\n********************************************************~\n\t\tSimulation Control Parameters\n\t|\n\n");
    let tu = if m.settings.time_units.trim().is_empty() { "Month".to_string() } else { m.settings.time_units.clone() };
    out.entry("FINAL TIME  ", Some(("=", &num(m.settings.final_time))), &tu, "The final time for the simulation.");
    out.entry("INITIAL TIME  ", Some(("=", &num(m.settings.initial_time))), &tu, "The initial time for the simulation.");
    out.entry("SAVEPER  ", Some(("=", "TIME STEP")), &tu, "The frequency with which output is stored.");
    out.entry("TIME STEP  ", Some(("=", &num(m.settings.time_step))), &tu, "The time step for the simulation.");
    if m.settings.method != Method::Euler {
        w.loss.approximated("integration method", format!("{:?} is not written: the file carries no integration-type setting that was verified, so Vensim will use Euler", m.settings.method));
    }

    // things with no home in the file
    if !m.modules.is_empty() {
        w.loss.approximated("modules", format!("{} module(s) became sketch views (one per module); ports and module descriptions are not written", m.modules.len()));
    }
    if m.settings.figure_model {
        w.loss.dropped("figure_model setting", "the model is marked as drawn for a figure, so that its unit findings are information; a Vensim file cannot carry the mark, and after reading it again the units rules report errors");
    }
    if m.settings.teaching_draft {
        w.loss.dropped("teaching_draft setting", "the model is marked as a teaching draft, so that the extreme condition experiments report as information; a Vensim file cannot carry the mark");
    }
    if !m.scenarios.is_empty() {
        w.loss.dropped("scenarios", format!("{} scenario(s): Vensim keeps these in separate files", m.scenarios.len()));
    }
    if !m.manifest.is_empty() {
        w.loss.dropped("manifest", "levers with ranges, outputs, scenarios, bindings, validity and test references have no place in a Vensim model");
    }
    if !m.checks.is_empty() {
        w.loss.dropped("model checks", format!("{} check(s): Vensim has no equivalent stored in the model", m.checks.len()));
    }
    if !m.notes.is_empty() {
        w.loss.dropped("notes", format!("{} note(s) on the diagram", m.notes.len()));
    }
    if !m.controls.is_empty() {
        w.loss.dropped("controls", format!("{} slider(s), knob(s), switch(es) or number input(s) on the diagram", m.controls.len()));
    }
    if !m.shapes.is_empty() {
        w.loss.dropped("shapes", format!("{} shape(s) and picture(s) drawn on the diagram", m.shapes.len()));
    }
    for s in &m.sheets {
        if s.kind != SheetKind::Diagram {
            w.loss.dropped(format!("sheet {}", s.name), "tables, results, scenarios, causal loop and optimisation sheets have no place in a Vensim file");
        }
    }

    out.text.push_str(&sketch(m, &mut w));
    (out.text, w.loss)
}

fn write_var(w: &mut W, out: &mut Out, v: &Var) {
    let m = w.m;
    let name = vname(&v.name);
    let units = units_text(v);
    let set = w.set_of(v);
    // a data placeholder written back as data
    if let Some(rest) = v.comment.strip_prefix("[Vensim data: ") {
        if let Some(end) = rest.find(']') {
            let text = &rest[..end];
            let original_comment = rest[end + 1..].trim();
            w.loss.preserved(format!("data variable {}", v.name), "written back as the Vensim data definition it came from");
            if text.starts_with("(no equation") {
                out.entry(&name, None, &units, original_comment);
            } else if text.to_uppercase().contains("CONSTANTS") {
                out.entry(&format!("{name} "), Some(("=", text)), &units, original_comment);
            } else {
                out.entry(&format!("{name} "), Some((":=", text)), &units, original_comment);
            }
            return;
        }
    }
    // what a Vensim model has no field for rides along in the comment, so it comes back when the
    // file is read again; Vensim itself ignores it
    let mut tags: Vec<String> = Vec::new();
    if v.rollup == Rollup::Average && v.set.is_some() {
        tags.push("combine=average".into());
        w.loss.preserved(format!("combine {}", v.name), "the diagram's \"average\" roll-up is kept in the comment as [dynamist: combine=average]; Vensim ignores it");
    }
    if v.non_negative {
        tags.push("never-negative".into());
        w.loss.preserved(format!("never negative {}", v.name), "kept in the comment as [dynamist: never-negative]; Vensim will NOT keep the stock at or above zero (write the limit into the outflow's formula)");
    }
    if v.lever {
        tags.push(if (v.lever_scale - 1.0).abs() > 1e-12 { format!("lever={}", num(v.lever_scale)) } else { "lever".into() });
        w.loss.preserved(format!("key lever {}", v.name), "kept in the comment as [dynamist: lever]; Vensim ignores it");
    }
    let tagged;
    let v: &Var = if tags.is_empty() {
        v
    } else {
        let mut c = v.clone();
        c.comment = format!("{} [dynamist: {}]", v.comment.trim(), tags.join("; ")).trim().to_string();
        tagged = c;
        &tagged
    };
    match set {
        None => {
            let c = Wc { set: None, elem: None };
            let mut ne = false;
            let rate = stock_rate(w, v, c, &mut ne);
            let body = body_text(w, v, &v.equation, &v.table, c, &mut ne, Some(&rate));
            emit(out, &format!("{name} "), &body, &units, &v.comment, v);
        }
        Some(s) => {
            let dim = vname(&s.name);
            // apply-to-all when every item shares the formula and it can be written once
            if v.by_item.is_empty() {
                let c = Wc { set: Some(s), elem: None };
                let mut ne = false;
                let rate = stock_rate(w, v, c, &mut ne);
                let body = body_text(w, v, &v.equation, &v.table, c, &mut ne, Some(&rate));
                if !ne {
                    emit(out, &format!("{name}[{dim}] "), &body, &units, &v.comment, v);
                    return;
                }
            }
            // otherwise item by item
            for (k, item) in s.items.iter().enumerate() {
                let ov = v.by_item.iter().find(|o| nrm(&o.item) == nrm(item));
                // an item's own formula or curve replaces the usual one; a curve alone keeps the input
                let (eq, table) = match ov {
                    Some(o) if !o.equation.trim().is_empty() || !o.table.is_empty() => (if o.equation.trim().is_empty() { v.equation.as_str() } else { o.equation.as_str() }, if o.table.is_empty() { &v.table } else { &o.table }),
                    _ => (v.equation.as_str(), &v.table),
                };
                let c = Wc { set: Some(s), elem: Some(k) };
                let mut ne = false;
                let rate = stock_rate(w, v, c, &mut ne);
                let body = body_text(w, v, eq, table, c, &mut ne, Some(&rate));
                emit(out, &format!("{name}[{}] ", vname(item)), &body, &units, &v.comment, v);
            }
        }
    }
    let _ = m;
}

enum Body {
    Eq(String),
    Lookup(String),
}

fn emit(out: &mut Out, lhs: &str, body: &Body, units: &str, comment: &str, _v: &Var) {
    match body {
        Body::Eq(e) => out.entry(lhs, Some(("=", e)), units, comment),
        Body::Lookup(t) => {
            // `name( table )`: the table directly after the name
            out.text.push_str(&format!("{}(\n\t{}\n)\n", lhs.trim_end(), t));
            out.text.push_str(&format!("\t~\t{units}\n\t~\t{}\n\t|\n\n", comment_text(comment)));
        }
    }
}

/// `INTEG(rate, init)` text for a stock: the flows that fill it minus those that drain it.
fn stock_rate(w: &mut W, v: &Var, c: Wc, ne: &mut bool) -> String {
    if v.kind != Kind::Stock {
        return String::new();
    }
    let mut terms: Vec<(bool, String)> = Vec::new();
    let flows: Vec<&Var> = w.m.vars.iter().filter(|f| f.kind == Kind::Flow && (f.to == Some(v.id) || f.from == Some(v.id))).collect();
    for f in flows {
        let into = f.to == Some(v.id) && f.from != Some(v.id);
        let outof = f.from == Some(v.id) && f.to != Some(v.id);
        if !into && !outof {
            continue;
        }
        let text = match (w.set_of(f), c.set) {
            // a flow per the items of this stock's children: the stock collects them
            (Some(fs), Some(cs)) if nrm(&fs.name) != nrm(&cs.name) && fs.parent.as_ref().is_some_and(|p| nrm(p) == nrm(&cs.name)) => match c.elem {
                Some(k) => {
                    let me = &cs.items[k];
                    let parts: Vec<String> = (0..fs.items.len()).filter(|&i| fs.parent_item(i) == Some(me.as_str())).map(|i| format!("{}[{}]", vname(&f.name), vname(&fs.items[i]))).collect();
                    if parts.is_empty() { "0".to_string() } else { format!("({})", parts.join(" + ")) }
                }
                None => {
                    *ne = true;
                    vname(&f.name)
                }
            },
            _ => w.var_ref(f, c, ne),
        };
        terms.push((into, text));
    }
    let mut s = String::new();
    for (k, (plus, t)) in terms.iter().enumerate() {
        match (k, plus) {
            (0, true) => s.push_str(t),
            (0, false) => s.push_str(&format!("-{t}")),
            (_, true) => s.push_str(&format!(" + {t}")),
            (_, false) => s.push_str(&format!(" - {t}")),
        }
    }
    if s.is_empty() { "0".into() } else { s }
}

fn body_text(w: &mut W, v: &Var, eq: &str, table: &[[f64; 2]], c: Wc, ne: &mut bool, rate: Option<&str>) -> Body {
    match v.kind {
        Kind::Stock => {
            let init = if eq.trim().is_empty() { "0".to_string() } else { w.tr(eq, c, ne) };
            Body::Eq(format!("INTEG({}, {init})", rate.filter(|r| !r.is_empty()).unwrap_or("0")))
        }
        Kind::Lookup => {
            if eq.trim().is_empty() {
                Body::Lookup(table_text(table))
            } else {
                let x = w.tr(eq, c, ne);
                Body::Eq(format!("WITH LOOKUP({x}, ({}))", table_text(table)))
            }
        }
        _ => {
            let t = if eq.trim().is_empty() { "0".to_string() } else { w.tr(eq, c, ne) };
            Body::Eq(t)
        }
    }
}

// ------------------------------------------------------------------ the sketch

fn sk_name(n: &str) -> String {
    if n.contains(',') || n.contains('"') || n.contains('\n') { format!("\"{}\"", n.replace('"', "'").replace('\n', " ")) } else { n.to_string() }
}

fn sketch(m: &Model, w: &mut W) -> String {
    let mut s = String::from("\\\\\\---/// Sketch information - do not modify anything except names\nV300  Do not put anything below this section - it will be ignored\n");
    // views: every diagram sheet, named after the sheet (or the module whose inside it is)
    let first = m.first_diagram();
    let mut views: Vec<(u64, String)> = Vec::new();
    for sh in &m.sheets {
        if sh.kind == SheetKind::Diagram {
            let name = m.modules.iter().find(|x| x.inner == sh.id).map(|x| x.name.clone()).unwrap_or_else(|| sh.name.clone());
            views.push((sh.id, name));
        }
    }
    if views.is_empty() {
        views.push((first, "View 1".into()));
    }
    let view_of = |v: &Var| if v.view == 0 { first } else { v.view };
    let mut crossing = 0;
    for (vi, (vid, vname_)) in views.iter().enumerate() {
        let name = if vname_.trim().is_empty() { format!("View {}", vi + 1) } else { vname_.clone() };
        s.push_str(&format!("*{name}\n$192-192-192,0,Times New Roman|12||0-0-0|0-0-0|0-0-255|-1--1--1|-1--1--1|72,72,100,0\n"));
        let in_view: Vec<&Var> = m.vars.iter().filter(|v| view_of(v) == *vid || (views.len() == 1)).collect();
        let mut id = 0u64;
        let mut obj: std::collections::HashMap<u64, u64> = std::collections::HashMap::new(); // var id -> id used by arrows
        let mut lines: Vec<String> = Vec::new();
        // variables that are not flows
        for v in in_view.iter().filter(|v| v.kind != Kind::Flow) {
            id += 1;
            obj.insert(v.id, id);
            let (shape, wd) = if v.kind == Kind::Stock { (3, 40) } else { (8, 40) };
            lines.push(format!("10,{id},{},{},{},{wd},20,{shape},3,0,0,0,0,0,0", sk_name(&v.name), v.pos[0].round() as i32, v.pos[1].round() as i32));
        }
        // flows: a valve, its label, and pipes to the stocks or clouds at each end
        for f in in_view.iter().filter(|v| v.kind == Kind::Flow) {
            id += 1;
            let valve = id;
            let (fx, fy) = (f.pos[0].round() as i32, f.pos[1].round() as i32);
            lines.push(format!("11,{valve},0,{fx},{fy},6,8,34,3,0,0,1,0,0,0"));
            id += 1;
            lines.push(format!("10,{id},{},{fx},{},49,8,40,3,0,0,-1,0,0,0", sk_name(&f.name), fy + 20));
            obj.insert(f.id, valve);
            // the pipe to the end a flow runs to is flagged 4, the one from its source 100 (as in Vensim's own files)
            for (end, pt, flag) in [(f.from, f.from_pt, 100), (f.to, f.to_pt, 4)] {
                let target = match end.and_then(|e| obj.get(&e).copied()) {
                    Some(t) => t,
                    None => {
                        id += 1;
                        lines.push(format!("12,{id},48,{},{},10,8,0,3,0,0,0,0,0,0", pt[0].round() as i32, pt[1].round() as i32));
                        id
                    }
                };
                id += 1;
                lines.push(format!("1,{id},{valve},{target},{flag},0,0,22,0,192,0,-1--1--1,,1|({fx},{fy})|"));
            }
        }
        // information arrows
        for l in &m.links {
            let (a, b) = (obj.get(&l.from).copied(), obj.get(&l.to).copied());
            let tov = m.var(l.to);
            if tov.is_some_and(|t| t.kind == Kind::Stock) {
                continue; // a stock is only changed by its flows
            }
            match (a, b) {
                (Some(a), Some(b)) => {
                    id += 1;
                    let pol = match l.polarity.as_str() {
                        "+" => 43,
                        "-" => 45,
                        _ => 0,
                    };
                    let _ = pol;
                    lines.push(format!("1,{id},{a},{b},0,0,0,0,0,64,0,-1--1--1,,1|(0,0)|"));
                }
                (None, Some(_)) | (Some(_), None) => {
                    if m.var(l.from).is_some_and(|x| view_of(x) != *vid) || tov.is_some_and(|x| view_of(x) != *vid) {
                        crossing += 1;
                    }
                }
                _ => {}
            }
        }
        for l in lines {
            s.push_str(&l);
            s.push('\n');
        }
    }
    if crossing > 0 {
        // each arrow that crosses between views is counted twice (once per view)
        w.loss.approximated("arrows between views", format!("{} arrow(s) join variables on different sketch views; Vensim would draw them as shadow variables, which are not written", crossing / 2));
    }
    if m.links.iter().any(|l| l.polarity == "+" || l.polarity == "-") {
        w.loss.dropped("arrow polarity", "the + / − marks on arrows are not written");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdl;

    fn run_last(m: &Model, name: &str) -> f64 {
        let c = crate::sim::compile(m).unwrap_or_else(|p| panic!("{:?}", p.iter().map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>()));
        let r = crate::sim::simulate(&c, m, "x").unwrap();
        *r.get(name).unwrap_or_else(|| panic!("no series {name}; have {:?}", r.series.keys().collect::<Vec<_>>())).last().unwrap()
    }

    /// Both models give the same numbers for every variable the first has, at every step.
    fn same_numbers(a: &Model, b: &Model, tol: f64) {
        let ca = crate::sim::compile(a).unwrap();
        let ra = crate::sim::simulate(&ca, a, "a").unwrap();
        let cb = crate::sim::compile(b).unwrap_or_else(|p| panic!("{:?}", p.iter().map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>()));
        let rb = crate::sim::simulate(&cb, b, "b").unwrap();
        assert_eq!(ra.times.len(), rb.times.len());
        let mut checked = 0;
        for (k, sa) in &ra.series {
            let Some(sb) = rb.series.get(k) else { panic!("variable {k} is missing after the round trip") };
            for (x, y) in sa.iter().zip(sb) {
                assert!((x - y).abs() <= tol * (1.0 + x.abs()), "{k}: {x} vs {y}");
            }
            checked += 1;
        }
        assert!(checked > 3);
    }

    #[test]
    fn the_example_model_survives_a_round_trip() {
        let m = Model::example();
        let (text, loss) = write(&m);
        assert!(text.starts_with("{UTF-8}"));
        assert!(text.contains("INTEG("));
        assert!(text.contains("\\\\\\---/// Sketch information"));
        let (back, rloss) = mdl::read(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        // the example has a dashboard, an optimisation sheet and a causal loop diagram: those are reported
        assert!(loss.items.iter().any(|i| i.construct.starts_with("sheet")), "{}", loss.summary());
        assert!(rloss.is_lossless(), "reading what we wrote should lose nothing:\n{}\n{text}", rloss.summary());
        assert_eq!(back.vars.len(), m.vars.len());
        same_numbers(&m, &back, 1e-9);
        // positions survive for everything but flows (which sit between their stocks)
        for v in m.vars.iter().filter(|v| v.kind != Kind::Flow) {
            assert_eq!(back.by_name(&nrm(&v.name)).unwrap().pos, [v.pos[0].round(), v.pos[1].round()], "{}", v.name);
        }
        // arrows survive
        assert!(back.links.len() >= m.links.len() - 3, "{} vs {}", back.links.len(), m.links.len());
    }

    #[test]
    fn a_second_trip_changes_nothing() {
        let (t1, _) = write(&Model::example());
        let (m1, _) = mdl::read(&t1).unwrap();
        let (t2, _) = write(&m1);
        let (m2, _) = mdl::read(&t2).unwrap();
        same_numbers(&m1, &m2, 1e-12);
    }

    fn sets_model() -> Model {
        use crate::model::ItemValue;
        let mut m = Model::default();
        m.settings.final_time = 10.0;
        m.settings.time_step = 1.0;
        m.settings.time_units = "Day".into();
        let region = ItemSet { id: m.new_id(), name: "Region".into(), items: vec!["R1".into(), "R2".into()], ..Default::default() };
        let tank = ItemSet { id: m.new_id(), name: "Tank".into(), items: vec!["T1".into(), "T2".into(), "T3 - big".into()], parent: Some("Region".into()), parent_of: vec!["R1".into(), "R1".into(), "R2".into()], note: String::new() };
        m.sets = vec![region, tank];
        let add = |m: &mut Model, name: &str, kind: Kind, eq: &str, set: Option<&str>| -> u64 {
            let id = m.new_id();
            let mut v = Var::new(id, name, kind, [100.0 + id as f32 * 10.0, 100.0]);
            v.equation = eq.into();
            v.set = set.map(|s| s.to_string());
            v.units = "MB".into();
            m.vars.push(v);
            id
        };
        let stock = add(&mut m, "Stock", Kind::Stock, "100", Some("Tank"));
        let fill = add(&mut m, "Fill", Kind::Flow, "5 + Time * 0", Some("Tank"));
        let burn = add(&mut m, "Burn", Kind::Flow, "1", Some("Tank"));
        m.var_mut(fill).unwrap().to = Some(stock);
        m.var_mut(burn).unwrap().from = Some(stock);
        m.var_mut(burn).unwrap().by_item = vec![ItemValue { item: "T2".into(), equation: "2".into(), table: vec![] }, ItemValue { item: "T3 - big".into(), equation: "3".into(), table: vec![] }];
        add(&mut m, "Total", Kind::Aux, "TOTAL(Stock)", None);
        add(&mut m, "Region stock", Kind::Aux, "TOTAL(Stock)", Some("Region"));
        add(&mut m, "Share", Kind::Aux, "Stock / Region stock", Some("Tank"));
        add(&mut m, "Only T1", Kind::Aux, "Stock[T1] * 2", None);
        add(&mut m, "In R1", Kind::Aux, "TOTAL(Stock, R1)", None);
        m.next_id += 10;
        m
    }

    #[test]
    fn sets_with_parents_totals_and_per_item_values_round_trip() {
        let m = sets_model();
        let (text, loss) = write(&m);
        assert!(text.contains("Tank:"), "{text}");
        assert!(text.contains("-> (Region: R1, R1, R2)"), "{text}");
        assert!(text.contains("SUM(Stock[Tank!])"), "{text}");
        let (back, rloss) = mdl::read(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        let tank = back.sets.iter().find(|s| s.name == "Tank").unwrap();
        assert_eq!(tank.parent.as_deref(), Some("Region"));
        assert_eq!(tank.parent_of, ["R1", "R1", "R2"]);
        // 100 + 10 * (5 - burn): 140, 130, 120
        assert!((run_last(&back, "total") - 390.0).abs() < 1e-9, "{text}");
        assert!((run_last(&m, "total") - 390.0).abs() < 1e-9);
        assert!((run_last(&back, "region stock[R1]") - 270.0).abs() < 1e-9, "per-region sums are written item by item:\n{text}");
        assert!((run_last(&back, "in r1") - 270.0).abs() < 1e-9);
        assert!((run_last(&back, "only t1") - 280.0).abs() < 1e-9);
        assert!((run_last(&back, "share[T3 - big]") - 120.0 / 120.0).abs() < 1e-9);
        let _ = (loss, rloss);
    }

    #[test]
    fn lookups_functions_and_logic_round_trip() {
        let mut m = Model::default();
        m.settings.final_time = 20.0;
        m.settings.time_step = 0.5;
        let add = |m: &mut Model, name: &str, kind: Kind, eq: &str| -> u64 {
            let id = m.new_id();
            let mut v = Var::new(id, name, kind, [100.0, 100.0 + id as f32 * 40.0]);
            v.equation = eq.into();
            m.vars.push(v);
            id
        };
        let curve = add(&mut m, "Curve", Kind::Lookup, "");
        m.var_mut(curve).unwrap().table = vec![[0.0, 0.0], [5.0, 2.0], [10.0, 10.0]];
        let tab = add(&mut m, "Tabled", Kind::Lookup, "Time / 2");
        m.var_mut(tab).unwrap().table = vec![[0.0, 1.0], [10.0, 3.0]];
        add(&mut m, "Uses curve", Kind::Aux, "Curve(Time / 2) + Tabled");
        add(&mut m, "Rounded", Kind::Aux, "ROUND(Time * 0.37) + FLOOR(Time * 0.37) + CEILING(Time * 0.37) * 100");
        add(&mut m, "Signs", Kind::Aux, "SIGN(Time - 10) + CBRT(Time) + HYPOT(Time, 4) + CLAMP(Time, 3, 8)");
        add(&mut m, "Logic", Kind::Aux, "IF THEN ELSE(Time > 5 && Time <= 15 || Time = 2, 1, 0) + IF THEN ELSE(!(Time > 3), 10, 0)");
        add(&mut m, "Mins", Kind::Aux, "MIN(Time, 7, 4, 9) + MAX(Time, 1, 2) + MEAN(Time, 4, 6)");
        add(&mut m, "Logs", Kind::Aux, "LOG10(Time + 1) + LOG2(Time + 1) + LOG(Time + 1, 3) + RADIANS(Time) + DEGREES(1) + PI()");
        add(&mut m, "Waves", Kind::Aux, "SINWAVE(2, 8, 3) + COSWAVE(1, 5)");
        add(&mut m, "Graphed", Kind::Aux, "GRAPH(Time, 0, 5, 1, 4, 2, 8, 3)");
        add(&mut m, "Delayed", Kind::Aux, "DELAYMTR(Time, 3, 3) + DELAYINF(Time, 3, 3) + DELAYMTR(Time, 2, 1) + DELAYINF(Time, 2, 1)");
        add(&mut m, "Smoothed", Kind::Aux, "SMOOTH3(Time, 4) + DELAY1(Time, 2) + TREND(Time, 5, 0) + STEP(5, 3) + PULSE(4, 2) + RAMP(2, 3, 12)");
        add(&mut m, "With lookup", Kind::Aux, "WITH LOOKUP(Time, ((0,0),(10,5),(20,6)))");
        let (text, loss) = write(&m);
        let (back, rloss) = mdl::read(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        same_numbers(&m, &back, 1e-9);
        assert!(rloss.is_lossless(), "{}\n{text}", rloss.summary());
        assert!(loss.is_lossless(), "{}", loss.summary());
        // Vensim spellings
        assert!(text.contains(":AND:") && text.contains(":OR:") && text.contains(":NOT:"), "{text}");
        assert!(!text.contains("&&"));
    }

    #[test]
    fn a_figure_model_mark_is_listed_as_a_loss() {
        let mut m = Model::example();
        assert!(write(&m).1.items.iter().all(|i| !i.construct.contains("figure_model")));
        m.settings.figure_model = true;
        let (_, loss) = write(&m);
        assert!(loss.items.iter().any(|i| i.construct.contains("figure_model") && i.kind == crate::loss::LossKind::Dropped), "{}", loss.summary());
        assert!(!loss.is_lossless());
        let (_, xloss) = crate::xmile::write(&m);
        assert!(xloss.items.iter().any(|i| i.construct.contains("figure_model")), "{}", xloss.summary());
    }

    #[test]
    fn what_cannot_be_written_is_reported() {
        let mut m = Model::example();
        m.settings.method = Method::RungeKutta4;
        let mut v = Var::new(99, "Odd", Kind::Aux, [0.0, 0.0]);
        v.equation = "MEDIAN(1, 2, 3) + ERF(0.5) + RANDOM UNIFORM(0, 1, 5)".into();
        m.vars.push(v);
        m.vars[0].non_negative = true;
        m.vars[1].lever = true;
        let (text, loss) = write(&m);
        let has = |c: &str| loss.items.iter().any(|i| i.construct.contains(c));
        assert!(has("integration method") && has("function MEDIAN") && has("function ERF") && has("RANDOM UNIFORM") && has("never negative") && has("key lever"), "{}", loss.summary());
        assert!(text.contains("MEDIAN("));
        assert!(!loss.is_lossless());
    }

    #[test]
    fn data_placeholders_are_written_back_as_data() {
        let src = "price data ~ $/Unit ~ Observed |\nimports := GET XLS DATA('f.xls','S','A','B2') ~ Unit ~ |\nrate = GET XLS CONSTANTS('f.xls','S','B2') ~ ~ |\nrevenue = price data + imports + rate ~ ~ |\nFINAL TIME = 5 ~ Day ~ |\nINITIAL TIME = 0 ~ Day ~ |\nTIME STEP = 1 ~ Day ~ |\n";
        let (m, _) = mdl::read(src).unwrap();
        let (text, _) = write(&m);
        assert!(text.contains(":= GET XLS DATA") || text.contains(":=\n\tGET XLS DATA"), "{text}");
        assert!(text.contains("GET XLS CONSTANTS"), "{text}");
        let (again, loss) = mdl::read(&text).unwrap();
        assert_eq!(again.by_name("price data").unwrap().kind, Kind::Lookup);
        assert_eq!(loss.items.iter().filter(|i| i.construct.starts_with("data variable")).count(), 3);
    }

    #[test]
    fn views_follow_the_diagram_sheets() {
        // a model with a module: the inside becomes a second view, and variables keep their positions
        let m = crate::model::Model::example();
        let (text, _) = write(&m);
        assert!(text.contains("*"), "{text}");
        assert!(text.matches("\n*").count() >= 1);
        let first = text.find("\n10,").unwrap();
        assert!(first > text.find("V300").unwrap());
    }

    #[test]
    fn vensim_names_are_quoted_only_when_needed() {
        assert_eq!(vname("Heat Loss"), "Heat Loss");
        assert_eq!(vname("rate-of-change"), "\"rate-of-change\"");
        assert_eq!(vname("2nd order"), "\"2nd order\"");
        assert_eq!(vname("Stock[T1]"), "\"Stock[T1]\"");
    }

    #[test]
    fn what_vensim_has_no_field_for_rides_in_the_comment_and_comes_back() {
        let mut m = sets_model();
        let stock = m.vars.iter().position(|v| v.name == "Stock").unwrap();
        m.vars[stock].non_negative = true;
        m.vars[stock].lever = true;
        m.vars[stock].lever_scale = 2.0;
        m.vars[stock].comment = "Fuel in the tank".into();
        let region = m.vars.iter().position(|v| v.name == "Region stock").unwrap();
        m.vars[region].rollup = Rollup::Average;
        let (text, loss) = write(&m);
        assert!(text.contains("[dynamist:"), "{text}");
        assert!(loss.items.iter().all(|i| i.kind != crate::loss::LossKind::Dropped || !i.construct.contains("never")), "{}", loss.summary());
        let (back, rloss) = mdl::read(&text).unwrap();
        let s = back.by_name("stock").unwrap();
        assert!(s.non_negative && s.lever && (s.lever_scale - 2.0).abs() < 1e-12);
        assert_eq!(s.comment, "Fuel in the tank", "the tag is stripped again");
        assert_eq!(back.by_name("region stock").unwrap().rollup, Rollup::Average);
        assert!(rloss.is_lossless(), "{}", rloss.summary());
    }
}
