//! Reading the classic Vensim text model format (`.mdl`).
//!
//! Supported: equations, units (with slider ranges), comments, lookups in every spelling,
//! run settings, `INTEG`, subscripts (ranges, subranges, mappings, `:EXCEPT:`, per-element
//! equations, `SUM`/`VMIN`/`VMAX`/`PROD` over `Dim!`, two or more dimensions), macros,
//! data variables, and the sketch (positions, arrows, several views).
//!
//! Every construct that could not be carried over faithfully is listed in the [`LossReport`]
//! that [`read`] returns. [`import`] is the same without the report.

use std::collections::{HashMap, HashSet};

use crate::expr::{self, BinOp, Expr};
use crate::loss::LossReport;
use crate::model::{ItemSet, ItemValue, Kind, Method, Model, Var};
use crate::userfn::UserFn;

// ------------------------------------------------------------------ text helpers

/// Lexical pieces of an equation, kept with their byte spans so text can be edited in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LK {
    Word,
    Quoted,
    Num,
    Sym,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Lx {
    pub k: LK,
    pub s: usize,
    pub e: usize,
}

pub(crate) fn lex(src: &str) -> Vec<Lx> {
    let ch: Vec<(usize, char)> = src.char_indices().collect();
    let end_of = |i: usize| ch.get(i).map(|c| c.0).unwrap_or(src.len());
    let mut out = Vec::new();
    let mut i = 0;
    while i < ch.len() {
        let c = ch[i].1;
        if c.is_whitespace() || c == '\\' {
            i += 1;
            continue;
        }
        let s = ch[i].0;
        if c == '"' {
            let mut j = i + 1;
            while j < ch.len() && ch[j].1 != '"' {
                j += 1;
            }
            let e = end_of((j + 1).min(ch.len()));
            out.push(Lx { k: LK::Quoted, s, e });
            i = j + 1;
        } else if c.is_ascii_digit() || (c == '.' && ch.get(i + 1).is_some_and(|d| d.1.is_ascii_digit())) {
            let mut j = i;
            while j < ch.len() && (ch[j].1.is_ascii_digit() || ch[j].1 == '.') {
                j += 1;
            }
            if j < ch.len() && (ch[j].1 == 'e' || ch[j].1 == 'E') {
                let mut k = j + 1;
                if k < ch.len() && (ch[k].1 == '+' || ch[k].1 == '-') {
                    k += 1;
                }
                if k < ch.len() && ch[k].1.is_ascii_digit() {
                    while k < ch.len() && ch[k].1.is_ascii_digit() {
                        k += 1;
                    }
                    j = k;
                }
            }
            out.push(Lx { k: LK::Num, s, e: end_of(j) });
            i = j;
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let mut j = i;
            let mut last = i;
            while j < ch.len() && (ch[j].1.is_alphanumeric() || matches!(ch[j].1, '_' | '$' | '\'' | ' ' | '\t')) {
                if !matches!(ch[j].1, ' ' | '\t') {
                    last = j;
                }
                j += 1;
            }
            out.push(Lx { k: LK::Word, s, e: end_of(last + 1) });
            i = last + 1;
        } else if c == ':' {
            // :AND: :OR: :NOT: :NA: :EXCEPT: :INTERPOLATE: …
            let mut j = i + 1;
            while j < ch.len() && (ch[j].1.is_alphabetic() || ch[j].1 == ' ') {
                j += 1;
            }
            if j < ch.len() && ch[j].1 == ':' && j > i + 1 {
                out.push(Lx { k: LK::Sym, s, e: end_of(j + 1) });
                i = j + 1;
            } else if ch.get(i + 1).is_some_and(|d| d.1 == '=') {
                out.push(Lx { k: LK::Sym, s, e: end_of(i + 2) });
                i += 2;
            } else {
                out.push(Lx { k: LK::Sym, s, e: end_of(i + 1) });
                i += 1;
            }
        } else {
            let two: String = ch[i..(i + 2).min(ch.len())].iter().map(|c| c.1).collect();
            if matches!(two.as_str(), "<=" | ">=" | "<>" | "->" | "==") {
                out.push(Lx { k: LK::Sym, s, e: end_of(i + 2) });
                i += 2;
            } else {
                out.push(Lx { k: LK::Sym, s, e: end_of(i + 1) });
                i += 1;
            }
        }
    }
    out
}

fn text<'a>(src: &'a str, t: &Lx) -> &'a str {
    &src[t.s..t.e]
}

fn is_sym(src: &str, t: Option<&Lx>, s: &str) -> bool {
    t.is_some_and(|t| t.k == LK::Sym && text(src, t) == s)
}

/// Index of the token that closes the bracket opened at `open` (`(` or `[`).
fn matching(src: &str, toks: &[Lx], open: usize) -> Option<usize> {
    let (o, c) = if text(src, &toks[open]) == "(" { ("(", ")") } else { ("[", "]") };
    let mut depth = 0;
    for (k, t) in toks.iter().enumerate().skip(open) {
        if t.k == LK::Sym {
            let s = text(src, t);
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

/// Split at a separator that is not inside quotes, parentheses or brackets.
fn split_top(s: &str, sep: char) -> Vec<String> {
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
            c if c == sep && depth == 0 && !quote => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

/// A name written the way our equation language needs it inside `[ ]` or as a reference.
fn q(name: &str) -> String {
    if name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '$' | '\'')) && !name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit()) {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('"', "'"))
    }
}

fn nrm(s: &str) -> String {
    expr::normalize(s)
}

/// Remove `{ … }` comments that are not inside quotes.
fn strip_braces(text: &str) -> String {
    let (mut out, mut depth, mut quote) = (String::new(), 0, false);
    for c in text.chars() {
        match c {
            '"' if depth == 0 => {
                quote = !quote;
                out.push(c);
            }
            '{' if !quote => depth += 1,
            '}' if !quote && depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            _ => out.push(c),
        }
    }
    out
}

/// Split the equation part of the file into entries at `|` (outside quotes).
fn split_entries(text: &str) -> Vec<String> {
    let (mut out, mut cur, mut quote) = (Vec::new(), String::new(), false);
    for c in text.chars() {
        if c == '"' {
            quote = !quote;
        }
        if c == '|' && !quote {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Split "Dmnl [0,10,0.1]" into units and a slider range.
fn split_units(u: &str) -> (String, Option<[f64; 2]>) {
    let u = u.trim();
    if let Some(p) = u.find('[') {
        let inner = u[p + 1..].trim_end_matches(']').trim_end();
        let inner = inner.trim_end_matches(']');
        let nums: Vec<f64> = inner.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        let range = if nums.len() >= 2 { Some([nums[0], nums[1]]) } else { None };
        return (clean_units(u[..p].trim()), range);
    }
    (clean_units(u), None)
}

fn clean_units(u: &str) -> String {
    if u == "**undefined**" { String::new() } else { u.to_string() }
}

/// Flatten "a + b - c" into signed variable names; None if it's not that simple.
fn signed_terms(e: &Expr, sign: f64, out: &mut Vec<(String, f64)>) -> bool {
    match e {
        Expr::Var(n) => {
            out.push((n.clone(), sign));
            true
        }
        Expr::Bin(BinOp::Add, a, b) => signed_terms(a, sign, out) && signed_terms(b, sign, out),
        Expr::Bin(BinOp::Sub, a, b) => signed_terms(a, sign, out) && signed_terms(b, -sign, out),
        Expr::Neg(a) => signed_terms(a, -sign, out),
        _ => false,
    }
}

/// Split the arguments of "INTEG(rate, init)" at the top-level comma.
fn integ_args(rhs: &str) -> Option<(String, String)> {
    let up = rhs.trim();
    if !up.to_uppercase().starts_with("INTEG") {
        return None;
    }
    let open = up.find('(')?;
    if !up[5..open].trim().is_empty() {
        return None;
    }
    let close = up.rfind(')')?;
    if up[close + 1..].trim() != "" {
        return None;
    }
    let parts = split_top(&up[open + 1..close], ',');
    if parts.len() == 2 { Some((parts[0].trim().to_string(), parts[1].trim().to_string())) } else { None }
}

/// Evaluate a formula made of numbers only (control settings such as `1/4`).
fn const_eval(src: &str) -> Option<f64> {
    fn ev(e: &Expr) -> Option<f64> {
        Some(match e {
            Expr::Num(n) => *n,
            Expr::Neg(a) => -ev(a)?,
            Expr::Bin(op, a, b) => {
                let (x, y) = (ev(a)?, ev(b)?);
                match op {
                    BinOp::Add => x + y,
                    BinOp::Sub => x - y,
                    BinOp::Mul => x * y,
                    BinOp::Div => x / y,
                    BinOp::Pow => x.powf(y),
                    _ => return None,
                }
            }
            _ => return None,
        })
    }
    ev(&expr::parse(src).ok()?)
}

// ------------------------------------------------------------------ entries

#[derive(Clone, Debug, PartialEq)]
enum Op {
    /// `name = rhs`
    Eq,
    /// `name := rhs` (a data equation)
    Data,
    /// `name( table )`
    Lookup,
    /// `name ~ units ~ comment` with no equation: a data variable
    Bare,
}

#[derive(Clone, Debug)]
struct Def {
    base: String,
    subs: Vec<String>,
    except: Vec<String>,
    op: Op,
    rhs: String,
    units: String,
    comment: String,
}

#[derive(Clone, Debug)]
struct RangeDef {
    name: String,
    items: Vec<String>,
    /// `-> (Other: a, b, c)`: the item of `Other` each item is mapped to.
    map: Option<(String, Vec<String>)>,
    alias_of: Option<String>,
}

enum Parsed {
    Def(Def),
    Range(RangeDef),
}

fn sub_tokens(inner: &str) -> Vec<String> {
    split_top(inner, ',').into_iter().map(|s| unquote(&s)).filter(|s| !s.is_empty()).collect()
}

/// `name[subs]` → (name, subs)
fn split_lhs(lhs: &str) -> (String, Vec<String>) {
    let lhs = lhs.trim();
    // a bracket outside quotes
    let mut quote = false;
    for (i, c) in lhs.char_indices() {
        match c {
            '"' => quote = !quote,
            '[' if !quote => {
                let inner = lhs[i + 1..].trim_end().trim_end_matches(']');
                return (unquote(&lhs[..i]), sub_tokens(inner));
            }
            _ => {}
        }
    }
    (unquote(lhs), Vec::new())
}

fn parse_range_items(spec: &str) -> Vec<String> {
    let spec = spec.trim();
    // (A1-A5): numbered range
    if spec.starts_with('(') && spec.ends_with(')') && spec.contains('-') {
        let inner = &spec[1..spec.len() - 1];
        if let Some((a, b)) = inner.split_once('-') {
            let (a, b) = (a.trim(), b.trim());
            let split = |s: &str| {
                let k = s.rfind(|c: char| !c.is_ascii_digit()).map(|p| p + 1).unwrap_or(0);
                (s[..k].to_string(), s[k..].to_string())
            };
            let ((pa, na), (pb, nb)) = (split(a), split(b));
            if pa == pb {
                if let (Ok(x), Ok(y)) = (na.parse::<i64>(), nb.parse::<i64>()) {
                    if x <= y && y - x < 10_000 {
                        return (x..=y).map(|n| format!("{pa}{n}")).collect();
                    }
                }
            }
        }
    }
    split_top(spec, ',').into_iter().map(|s| unquote(&s)).filter(|s| !s.is_empty()).collect()
}

fn parse_chunk(raw: &str) -> Option<Parsed> {
    let mut parts = raw.splitn(3, '~');
    let eq = parts.next()?.replace("\\\n", " ").replace('\n', " ").replace('\t', " ");
    let units = parts.next().unwrap_or("").trim().to_string();
    let comment = parts.next().unwrap_or("").trim().replace("\\\n", " ").replace('\n', " ").replace('\t', " ");
    let eq = eq.trim();
    if eq.is_empty() {
        return None;
    }
    // top-level scan for the first '=', '(' or ':'
    let chars: Vec<(usize, char)> = eq.char_indices().collect();
    let (mut quote, mut br, mut pa) = (false, 0i32, 0i32);
    let mut found: Option<(usize, char)> = None; // byte pos, kind: '=', ':' (data), '(' , 'r' (range) , 'a' (alias)
    let upper = eq.to_uppercase();
    let except_at = upper.find(":EXCEPT:");
    let mut skip_to = 0usize;
    for (k, &(p, c)) in chars.iter().enumerate() {
        if k < skip_to {
            continue;
        }
        match c {
            '"' => quote = !quote,
            _ if quote => {}
            '[' => br += 1,
            ']' => br -= 1,
            '(' if br == 0 && pa == 0 => {
                found = Some((p, '('));
                break;
            }
            '(' => pa += 1,
            ')' => pa -= 1,
            '=' if br == 0 && pa == 0 => {
                let prev = if k > 0 { chars[k - 1].1 } else { ' ' };
                found = Some((if prev == ':' { chars[k - 1].0 } else { p }, if prev == ':' { ':' } else { '=' }));
                break;
            }
            '<' if br == 0 && pa == 0 && eq[p..].starts_with("<->") => {
                found = Some((p, 'a'));
                break;
            }
            ':' if br == 0 && pa == 0 => {
                let rest = &upper[p..];
                if rest.starts_with(":=") {
                    found = Some((p, ':'));
                    break;
                }
                if rest.starts_with(":EXCEPT:") {
                    skip_to = k + 8;
                    continue;
                }
                found = Some((p, 'r'));
                break;
            }
            _ => {}
        }
    }
    let _ = except_at;
    let Some((p, kind)) = found else {
        // a bare name: a data variable
        let (base, subs) = split_lhs(eq);
        if base.is_empty() || base.starts_with("****") {
            return None;
        }
        return Some(Parsed::Def(Def { base, subs, except: vec![], op: Op::Bare, rhs: String::new(), units, comment }));
    };
    let lhs = &eq[..p];
    match kind {
        'r' => {
            let spec = &eq[p + 1..];
            // `-> mapping`
            let (items_part, map_part) = match spec.find("->") {
                Some(a) => (&spec[..a], Some(spec[a + 2..].trim())),
                None => (spec, None),
            };
            let map = map_part.map(|m| {
                let m = m.trim();
                if let Some(inner) = m.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
                    if let Some((d, list)) = inner.split_once(':') {
                        return (unquote(d), parse_range_items(list));
                    }
                }
                (unquote(m), Vec::new())
            });
            Some(Parsed::Range(RangeDef { name: unquote(lhs), items: parse_range_items(items_part), map, alias_of: None }))
        }
        'a' => {
            let other = unquote(&eq[p + 3..]);
            Some(Parsed::Range(RangeDef { name: unquote(lhs), items: Vec::new(), map: None, alias_of: Some(other) }))
        }
        '(' => {
            let (base, subs) = split_lhs(lhs);
            if base.is_empty() || base.starts_with("****") {
                return None;
            }
            Some(Parsed::Def(Def { base, subs, except: vec![], op: Op::Lookup, rhs: eq[p..].trim().to_string(), units, comment }))
        }
        _ => {
            // `x[Tank] :EXCEPT: [T1] = …`
            let mut lhs_main = lhs.to_string();
            let mut except = Vec::new();
            if let Some(x) = lhs.to_uppercase().find(":EXCEPT:") {
                lhs_main = lhs[..x].to_string();
                let tail = lhs[x + 8..].trim();
                let inner = tail.trim_start_matches('[').trim_end().trim_end_matches(']');
                except = sub_tokens(inner);
            }
            let (base, subs) = split_lhs(&lhs_main);
            if base.is_empty() || base.starts_with("****") {
                return None;
            }
            let rhs = eq[p..].trim_start_matches(['=', ':']).trim().to_string();
            Some(Parsed::Def(Def { base, subs, except, op: if kind == ':' { Op::Data } else { Op::Eq }, rhs, units, comment }))
        }
    }
}

// ------------------------------------------------------------------ the reader

#[derive(Clone, Debug)]
enum Shape {
    Scalar,
    Set(String),
    Flat(Vec<String>),
}

/// What a left-hand side fixes: for each dimension (normalised) the item, if one is fixed.
#[derive(Clone, Default)]
struct Lhs {
    dims: Vec<(String, Option<String>)>,
    scalar: bool,
}

struct Reader {
    m: Model,
    loss: LossReport,
    ranges: Vec<RangeDef>,
    shapes: HashMap<String, Shape>,
    known_fns: HashSet<String>,
    /// scalar constants written as plain numbers, by normalised name (to read a delay's order from a name)
    consts: HashMap<String, f64>,
    var_names: HashSet<String>,
    said: HashSet<String>,
    /// stocks to turn into flows: (stock id, per-item rates)
    stock_rates: Vec<(u64, String, Vec<(String, String)>)>,
}

const REDUCERS: &[&str] = &["sum", "prod", "vmin", "vmax", "elmcount"];

/// Functions Vensim has that the engine does not, written the way Vensim spells them.
fn vensim_only(name: &str) -> bool {
    !matches!(name, "" | "INTEG")
}

impl Reader {
    fn once(&mut self, key: &str) -> bool {
        self.said.insert(key.to_string())
    }

    fn range(&self, name: &str) -> Option<&RangeDef> {
        let n = nrm(name);
        self.ranges.iter().find(|r| nrm(&r.name) == n)
    }

    /// The range an item belongs to (first declared), and the item as the range spells it.
    fn element(&self, item: &str) -> Option<(&RangeDef, String)> {
        let n = nrm(item);
        self.ranges.iter().find_map(|r| r.items.iter().find(|i| nrm(i) == n).map(|i| (r, i.clone())))
    }

    // ---- equation rewriting -------------------------------------------------------------

    /// Vensim equation text → our equation text, for a left-hand side. Sets `needs_elem` when the
    /// formula can only be written one item at a time.
    fn rewrite(&mut self, s: &str, lhs: &Lhs, who: &str, needs_elem: &mut bool) -> String {
        let mut cur = s.to_string();
        // Vensim's missing value and the named constants of VECTOR SELECT
        if cur.to_uppercase().contains(":NA:") {
            let up = cur.to_uppercase();
            let mut out = String::new();
            let mut i = 0;
            while let Some(p) = up[i..].find(":NA:") {
                out.push_str(&cur[i..i + p]);
                out.push_str("(-1.298074214633707e33)");
                i += p + 4;
            }
            out.push_str(&cur[i..]);
            cur = out;
        }
        for (k, v) in [("VSSUM", "0"), ("VSPRODUCT", "1"), ("VSMIN", "2"), ("VSMAX", "3"), ("VSERRNONE", "0"), ("VSERRATLEASTONE", "1"), ("VSERRATMOSTONE", "2")] {
            if cur.to_uppercase().contains(k) && !self.var_names.contains(&nrm(k)) {
                let toks = lex(&cur);
                let mut out = String::new();
                let mut last = 0;
                for t in &toks {
                    if t.k == LK::Word && text(&cur, t).trim().eq_ignore_ascii_case(k) {
                        out.push_str(&cur[last..t.s]);
                        out.push_str(v);
                        last = t.e;
                    }
                }
                out.push_str(&cur[last..]);
                cur = out;
            }
        }
        // array functions first: they read whole vectors (`x[dim!]`), written out element by element
        for _ in 0..16 {
            match self.vector_once(&cur, lhs, who, needs_elem) {
                Some(next) => cur = next,
                None => break,
            }
        }
        // reducers first (each pass handles one), then subscripted references
        for _ in 0..64 {
            match self.reduce_once(&cur, lhs, who) {
                Some(next) => cur = next,
                None => break,
            }
        }
        let cur = self.rewrite_refs(&cur, lhs, who, needs_elem);
        self.rewrite_words(&cur)
    }

    /// `base[a, d!, b!]` → every element over the dimensions marked `!`, last fastest; other subscripts kept.
    fn bang_elements(&self, arg: &str) -> Option<(Vec<String>, Vec<String>)> {
        self.vector_elements(arg, 0)
    }

    /// As `bang_elements`; when no subscript is marked `!`, the last `last` subscripts that are dimensions
    /// are the vector's (Vensim lets array functions be written without `!`).
    fn vector_elements(&self, arg: &str, last: usize) -> Option<(Vec<String>, Vec<String>)> {
        let arg = arg.trim();
        let open = arg.find('[')?;
        let close = arg.rfind(']')?;
        let base = arg[..open].trim().to_string();
        let mut subs = sub_tokens(&arg[open + 1..close]);
        if last > 0 {
            let n = subs.len();
            for t in subs.iter_mut().skip(n.saturating_sub(last)) {
                if !t.ends_with('!') && self.range(t.trim()).is_some() {
                    *t = format!("{}!", t.trim());
                }
            }
        }
        let mut lists: Vec<Vec<String>> = Vec::new();
        let mut bang_dims = Vec::new();
        for t in &subs {
            if let Some(d) = t.strip_suffix('!') {
                let r = self.range(d.trim())?;
                bang_dims.push(r.name.clone());
                lists.push(r.items.clone());
            } else {
                lists.push(vec![t.clone()]);
            }
        }
        let total: usize = lists.iter().map(|l| l.len()).product();
        let mut out = Vec::with_capacity(total);
        let mut idx = vec![0usize; lists.len()];
        for _ in 0..total {
            let parts: Vec<String> = idx.iter().zip(&lists).map(|(i, l)| l[*i].clone()).collect();
            out.push(format!("{}[{}]", base, parts.join(", ")));
            for k in (0..idx.len()).rev() {
                idx[k] += 1;
                if idx[k] < lists[k].len() {
                    break;
                }
                idx[k] = 0;
            }
        }
        Some((out, bang_dims))
    }

    /// The 0-based place, within dimension `d`, of the item the left-hand side fixes for it.
    fn lhs_place(&self, d: &str, lhs: &Lhs) -> Option<usize> {
        let r = self.range(d)?;
        let e = lhs.dims.iter().find(|(x, _)| *x == nrm(&r.name)).and_then(|(_, e)| e.clone()).or_else(|| self.mapped_item(&r.name, lhs))?;
        r.items.iter().position(|i| nrm(i) == nrm(&e))
    }

    /// One array function call (ALLOCATE AVAILABLE, ALLOCATE BY PRIORITY, VECTOR SORT ORDER, VECTOR RANK,
    /// VECTOR REORDER, VECTOR ELM MAP, VECTOR SELECT, INVERT MATRIX) → the engine's element-by-element form.
    fn vector_once(&mut self, s: &str, lhs: &Lhs, who: &str, needs_elem: &mut bool) -> Option<String> {
        let toks = lex(s);
        for (i, t) in toks.iter().enumerate() {
            if t.k != LK::Word || !is_sym(s, toks.get(i + 1), "(") {
                continue;
            }
            let f = text(s, t).to_uppercase();
            let f = f.split_whitespace().collect::<Vec<_>>().join(" ");
            if !matches!(f.as_str(), "VECTOR LOOKUP" | "ALLOCATE AVAILABLE" | "ALLOCATE BY PRIORITY" | "VECTOR SORT ORDER" | "VECTOR RANK" | "VECTOR REORDER" | "VECTOR ELM MAP" | "VECTOR SELECT" | "INVERT MATRIX") {
                continue;
            }
            let close = matching(s, &toks, i + 1)?;
            let args: Vec<String> = split_top(&s[toks[i + 1].e..toks[close].s], ',').iter().map(|a| a.trim().to_string()).collect();
            let whole_set = lhs.dims.iter().any(|(_, e)| e.is_none());
            let mut place = |this: &mut Self, d: &str| -> Option<usize> {
                let p = this.lhs_place(d, lhs);
                if p.is_none() && whole_set {
                    *needs_elem = true;
                }
                p
            };
            let rep: Option<String> = match f.as_str() {
                "ALLOCATE AVAILABLE" if args.len() == 3 => (|| {
                    let (req, dims) = self.vector_elements(&args[0], 1)?;
                    // the profile may name the first item of its dimension (`pp[region, ptype]`): read the whole dimension
                    let a1 = {
                        let a = args[1].trim();
                        let open = a.find('[')?;
                        let mut subs = sub_tokens(&a[open + 1..a.rfind(']')?]);
                        if let Some(lst) = subs.last_mut() {
                            let t = lst.trim_end_matches('!').trim().to_string();
                            if self.range(&t).is_none() {
                                if let Some((r, _)) = self.element(&t) {
                                    *lst = format!("{}!", r.name);
                                }
                            }
                        }
                        format!("{}[{}]", a[..open].trim(), subs.join(", "))
                    };
                    let (pp, _) = self.vector_elements(&a1, 2)?;
                    let k = place(self, dims.last()?)?;
                    Some(format!("ALLOCATE AVAILABLE ITEM({k}, {}, {}, {}, {})", req.len(), req.join(", "), pp.join(", "), args[2]))
                })(),
                "ALLOCATE BY PRIORITY" if args.len() == 5 => (|| {
                    let (req, dims) = self.vector_elements(&args[0], 1)?;
                    let (pri, _) = self.vector_elements(&args[1], 1)?;
                    let k = place(self, dims.last()?)?;
                    Some(format!("ALLOCATE BY PRIORITY ITEM({k}, {}, {}, {}, {}, {})", req.len(), req.join(", "), pri.join(", "), args[3], args[4]))
                })(),
                "VECTOR SORT ORDER" | "VECTOR RANK" if args.len() == 2 => (|| {
                    let (x, dims) = self.vector_elements(&args[0], 1)?;
                    let k = place(self, dims.last()?)?;
                    Some(format!("{f} ITEM({k}, {}, {})", args[1], x.join(", ")))
                })(),
                "VECTOR REORDER" if args.len() == 2 => (|| {
                    let (x, dims) = self.vector_elements(&args[0], 1)?;
                    let (o, _) = self.vector_elements(&args[1], 1)?;
                    let k = place(self, dims.last()?)?;
                    Some(format!("VECTOR REORDER ITEM({k}, {}, {}, {})", x.len(), x.join(", "), o.join(", ")))
                })(),
                "VECTOR LOOKUP" if args.len() == 5 => (|| {
                    // VECTOR LOOKUP(vec[first], x, xmin, xmax, mode): the y values are every element of vec's last dimension
                    let a = args[0].trim();
                    let open = a.find('[')?;
                    let mut subs = sub_tokens(&a[open + 1..a.rfind(']')?]);
                    let last = subs.last_mut()?;
                    let t = last.trim_end_matches('!').trim().to_string();
                    let dim = match self.range(&t) {
                        Some(r) => r.name.clone(),
                        None => self.element(&t)?.0.name.clone(),
                    };
                    *last = format!("{dim}!");
                    let (ys, _) = self.bang_elements(&format!("{}[{}]", a[..open].trim(), subs.join(", ")))?;
                    Some(format!("VECTOR LOOKUP ITEMS({}, {}, {}, {}, {}, {})", ys.len(), ys.join(", "), args[1], args[2], args[3], args[4]))
                })(),
                "VECTOR ELM MAP" if args.len() == 2 => (|| {
                    // VECTOR ELM MAP(x[start], offset): the element `offset` places after `start`, in x's dimension
                    let a = args[0].trim();
                    let open = a.find('[')?;
                    let base = a[..open].trim();
                    let subs = sub_tokens(&a[open + 1..a.rfind(']')?]);
                    let last = subs.last()?.trim_end_matches('!').trim().to_string();
                    let as_range = self.range(&last).map(|r| r.name.clone());
                    let (dim, start) = match as_range {
                        Some(rn) => { let p = place(self, &rn)?; (rn, p) }
                        None => {
                            let (r, e) = self.element(&last)?;
                            (r.name.clone(), r.items.iter().position(|x| nrm(x) == nrm(&e))?)
                        }
                    };
                    let mut bang = subs.clone();
                    *bang.last_mut()? = format!("{dim}!");
                    let (x, _) = self.bang_elements(&format!("{base}[{}]", bang.join(", ")))?;
                    Some(format!("VECTOR ELM MAP ITEM({start}, {}, {})", args[1], x.join(", ")))
                })(),
                "VECTOR SELECT" if args.len() == 5 => (|| {
                    // the selection's marked dimensions set the combinations; the values follow them, repeated over
                    // any marked dimension they do not have
                    let (sel, sdims) = self.vector_elements(&args[0], 1)?;
                    let lists: Vec<Vec<String>> = sdims.iter().map(|d| self.range(d).map(|r| r.items.clone())).collect::<Option<_>>()?;
                    let a = args[1].trim();
                    let open = a.find('[')?;
                    let vbase = a[..open].trim().to_string();
                    let vsubs = sub_tokens(&a[open + 1..a.rfind(']')?]);
                    let mut val = Vec::with_capacity(sel.len());
                    let mut idx = vec![0usize; lists.len()];
                    for _ in 0..sel.len() {
                        let subs: Vec<String> = vsubs.iter().map(|t| {
                            let d = t.trim_end_matches('!').trim();
                            match sdims.iter().position(|x| nrm(x) == nrm(d)) {
                                Some(p) => lists[p][idx[p]].clone(),
                                None => t.trim_end_matches('!').to_string(),
                            }
                        }).collect();
                        val.push(format!("{vbase}[{}]", subs.join(", ")));
                        for k in (0..idx.len()).rev() {
                            idx[k] += 1;
                            if idx[k] < lists[k].len() {
                                break;
                            }
                            idx[k] = 0;
                        }
                    }
                    Some(format!("VECTOR SELECT ITEMS({}, {}, {}, {}, {})", sel.len(), sel.join(", "), val.join(", "), args[2], args[3]))
                })(),
                "INVERT MATRIX" if !args.is_empty() => (|| {
                    // INVERT MATRIX(m[A, B], size): the left-hand side's items pick the element of the inverse
                    let a = args[0].trim();
                    let open = a.find('[')?;
                    let subs = sub_tokens(&a[open + 1..a.rfind(']')?]);
                    if subs.len() < 2 {
                        return None;
                    }
                    let (d1, d2) = (subs[subs.len() - 2].trim_end_matches('!').to_string(), subs[subs.len() - 1].trim_end_matches('!').to_string());
                    let (i, j) = if nrm(&d1) == nrm(&d2) {
                        // the same dimension twice (`m[c, c]`): the left-hand side's first and second items, in order
                        let r = self.range(&d1)?.clone();
                        let fixed: Vec<usize> = lhs.dims.iter().filter(|(x, _)| *x == nrm(&r.name)).filter_map(|(_, e)| e.as_ref().and_then(|e| r.items.iter().position(|it| nrm(it) == nrm(e)))).collect();
                        if fixed.len() < 2 {
                            // a whole-set left-hand side: `place` marks it for writing item by item
                            let _ = place(self, &d1);
                            return None;
                        }
                        (fixed[0], fixed[1])
                    } else {
                        (place(self, &d1)?, place(self, &d2)?)
                    };
                    let mut bang = subs.clone();
                    let n = bang.len();
                    bang[n - 2] = format!("{d1}!");
                    bang[n - 1] = format!("{d2}!");
                    let (m, _) = self.bang_elements(&format!("{}[{}]", a[..open].trim(), bang.join(", ")))?;
                    let size = self.range(&d1)?.items.len();
                    Some(format!("INVERT MATRIX ITEM({i}, {j}, {size}, {})", m.join(", ")))
                })(),
                _ => None,
            };
            match rep {
                Some(r) => return Some(format!("{}{}{}", &s[..t.s], r, &s[toks[close].e..])),
                None => {
                    if !*needs_elem && self.once(&format!("vecfn {who} {f}")) {
                        self.loss.preserved_not_simulable(format!("{f} in {who}"), "the arguments could not be matched to the dimensions; kept as written");
                    }
                    return None;
                }
            }
        }
        None
    }

    fn reduce_once(&mut self, s: &str, lhs: &Lhs, who: &str) -> Option<String> {
        let toks = lex(s);
        for (i, t) in toks.iter().enumerate() {
            if t.k != LK::Word || !is_sym(s, toks.get(i + 1), "(") {
                continue;
            }
            let f = nrm(text(s, t));
            if !REDUCERS.contains(&f.as_str()) {
                continue;
            }
            let close = matching(s, &toks, i + 1)?;
            let inner = &s[toks[i + 1].e..toks[close].s];
            if f == "elmcount" {
                let n = self.range(&unquote(inner)).map(|r| r.items.len())?;
                return Some(format!("{}{}{}", &s[..t.s], n, &s[toks[close].e..]));
            }
            // dimensions marked with '!'
            let itoks = lex(inner);
            let mut dims: Vec<String> = Vec::new();
            for (k, it) in itoks.iter().enumerate() {
                if it.k == LK::Word && is_sym(inner, itoks.get(k + 1), "!") {
                    let d = nrm(text(inner, it));
                    if !dims.contains(&d) {
                        dims.push(d);
                    }
                }
            }
            if dims.is_empty() {
                continue;
            }
            // SUM(y[Dim!]) in a plain (unsubscripted) formula is TOTAL(y)
            if f == "sum" && lhs.scalar && dims.len() == 1 {
                if let [a, b, c, d, e] = itoks.iter().collect::<Vec<_>>()[..] {
                    if a.k == LK::Word && is_sym(inner, Some(b), "[") && c.k == LK::Word && is_sym(inner, Some(d), "!") && is_sym(inner, Some(e), "]") && itoks.len() == 5 {
                        let base = text(inner, a).trim().to_string();
                        if matches!(self.shapes.get(&nrm(&base)), Some(Shape::Set(set)) if nrm(set) == dims[0]) {
                            return Some(format!("{}TOTAL({}){}", &s[..t.s], q(&base), &s[toks[close].e..]));
                        }
                    }
                }
            }
            // otherwise write the sum out item by item
            let ranges: Vec<RangeDef> = dims.iter().filter_map(|d| self.range(d).cloned()).collect();
            if ranges.len() != dims.len() {
                if self.once(&format!("badreducer {who}")) {
                    self.loss.approximated(format!("{} in {who}", text(s, t).to_uppercase()), "sums over a dimension that is not defined; left as written");
                }
                return None;
            }
            let total: usize = ranges.iter().map(|r| r.items.len().max(1)).product();
            if total > 2000 {
                self.loss.dropped(format!("{} in {who}", text(s, t).to_uppercase()), format!("sums over {total} combinations; too many to write out, left as written"));
                return None;
            }
            let mut terms = Vec::new();
            let mut idx = vec![0usize; ranges.len()];
            'combos: loop {
                let mut term = String::new();
                let mut last = 0;
                for (k, it) in itoks.iter().enumerate() {
                    if it.k == LK::Word && is_sym(inner, itoks.get(k + 1), "!") {
                        let d = nrm(text(inner, it));
                        let pos = dims.iter().position(|x| *x == d).unwrap();
                        term.push_str(&inner[last..it.s]);
                        term.push_str(&q(&ranges[pos].items[idx[pos]]));
                        last = itoks[k + 1].e;
                    }
                }
                term.push_str(&inner[last..]);
                terms.push(term);
                // next combination
                let mut k = ranges.len();
                loop {
                    if k == 0 {
                        break 'combos;
                    }
                    k -= 1;
                    idx[k] += 1;
                    if idx[k] < ranges[k].items.len() {
                        break;
                    }
                    idx[k] = 0;
                }
            }
            let joined = match f.as_str() {
                "sum" => format!("({})", terms.join(" + ")),
                "prod" => format!("({})", terms.iter().map(|t| format!("({t})")).collect::<Vec<_>>().join(" * ")),
                other => {
                    let func = if other == "vmin" { "MIN" } else { "MAX" };
                    let mut acc = terms.pop().unwrap_or_else(|| "0".into());
                    while let Some(t2) = terms.pop() {
                        acc = format!("{func}({t2}, {acc})");
                    }
                    acc
                }
            };
            if self.once(&format!("reducer {who} {f}")) {
                self.loss.approximated(format!("{} in {who}", f.to_uppercase()), "written out item by item (the engine has TOTAL only for a plain sum over one set)");
            }
            return Some(format!("{}{}{}", &s[..t.s], joined, &s[toks[close].e..]));
        }
        None
    }

    fn rewrite_refs(&mut self, s: &str, lhs: &Lhs, who: &str, needs_elem: &mut bool) -> String {
        let toks = lex(s);
        let mut out = String::new();
        let mut last = 0;
        let mut i = 0;
        while i < toks.len() {
            let t = toks[i];
            // a dimension's name used as a value is the item's position, 1 for the first item
            if t.k == LK::Word && !is_sym(s, toks.get(i + 1), "[") && !is_sym(s, toks.get(i + 1), "(") && !is_sym(s, toks.get(i + 1), "!") {
                let w = text(s, &t);
                if !self.var_names.contains(&nrm(w)) {
                    if let Some(r) = self.range(w).cloned() {
                        let fixed = lhs.dims.iter().find(|(d, _)| *d == nrm(&r.name)).and_then(|(_, e)| e.clone()).or_else(|| self.mapped_item(&r.name, lhs));
                        // the number of an item is its place in the first dimension that declares it (Vensim:
                        // `dim1 = dim3` compares items, even when dim3 lists them in another order)
                        let pos = |e: &str| self.element(e).and_then(|(rr, x)| rr.items.iter().position(|i| nrm(i) == nrm(&x)));
                        match fixed.and_then(|e| pos(&e)) {
                            Some(k) => {
                                out.push_str(&s[last..t.s]);
                                out.push_str(&(k + 1).to_string());
                                last = t.e;
                            }
                            None => *needs_elem = true,
                        }
                        i += 1;
                        continue;
                    }
                    // an item's name used as a value is its position in its dimension
                    if let Some((r, e)) = self.element(w).map(|(r, e)| (r.clone(), e)) {
                        if let Some(k) = r.items.iter().position(|x| nrm(x) == nrm(&e)) {
                            out.push_str(&s[last..t.s]);
                            out.push_str(&(k + 1).to_string());
                            last = t.e;
                            i += 1;
                            continue;
                        }
                    }
                }
            }
            if matches!(t.k, LK::Word | LK::Quoted) && is_sym(s, toks.get(i + 1), "[") {
                let Some(close) = matching(s, &toks, i + 1) else {
                    i += 1;
                    continue;
                };
                let base = unquote(text(s, &t));
                let inner = &s[toks[i + 1].e..toks[close].s];
                let subs = sub_tokens(inner);
                let rep = self.one_ref(&base, &subs, lhs, who, needs_elem);
                out.push_str(&s[last..t.s]);
                out.push_str(&rep);
                last = toks[close].e;
                i = close + 1;
                continue;
            }
            i += 1;
        }
        out.push_str(&s[last..]);
        out
    }

    /// The item of dimension `d` that goes with the left-hand side's fixed items through a mapping
    /// (`d: x, y -> L` or `L: a, b -> d`), or through `d` being a subrange of a fixed dimension.
    fn mapped_item(&self, d: &str, lhs: &Lhs) -> Option<String> {
        let rd = self.range(d)?;
        for (ld, e) in &lhs.dims {
            let Some(e) = e else { continue };
            let Some(rl) = self.range(ld) else { continue };
            // d maps onto the left-hand dimension: d's item in the same place as e
            if let Some((target, list)) = &rd.map {
                if nrm(target) == nrm(&rl.name) {
                    let k = if list.is_empty() { rl.items.iter().position(|i| nrm(i) == nrm(e)) } else { list.iter().position(|i| nrm(i) == nrm(e)) };
                    if let Some(k) = k {
                        if let Some(x) = rd.items.get(k) {
                            return Some(x.clone());
                        }
                    }
                }
            }
            // the left-hand dimension maps onto d
            if let Some((target, list)) = &rl.map {
                if nrm(target) == nrm(&rd.name) {
                    if let Some(k) = rl.items.iter().position(|i| nrm(i) == nrm(e)) {
                        let x = if list.is_empty() { rd.items.get(k).cloned() } else { list.get(k).cloned() };
                        if x.is_some() {
                            return x;
                        }
                    }
                }
            }
        }
        // otherwise: the fixed item is itself one of d's items (d is a subrange or superset holding it)
        if rd.map.is_none() {
            for (_, e) in &lhs.dims {
                if let Some(e) = e {
                    if rd.items.iter().any(|i| nrm(i) == nrm(e)) {
                        return Some(e.clone());
                    }
                }
            }
        }
        None
    }

    /// One `name[subs]` reference.
    fn one_ref(&mut self, base: &str, subs: &[String], lhs: &Lhs, who: &str, needs_elem: &mut bool) -> String {
        let shape = self.shapes.get(&nrm(base)).cloned();
        // resolve each subscript to Item(elem) or Dim(range)
        enum Sb {
            Item(String),
            Dim(String),
            Unknown(String),
        }
        let res: Vec<Sb> = subs
            .iter()
            .map(|t| {
                let t = t.trim_end_matches('!').trim();
                if let Some(r) = self.range(t) {
                    // a range name that is also an item name is read as the range
                    Sb::Dim(r.name.clone())
                } else if let Some((_, e)) = self.element(t) {
                    Sb::Item(e)
                } else {
                    Sb::Unknown(t.to_string())
                }
            })
            .collect();
        let fixed = |d: &str| -> Option<Option<String>> {
            let dn = nrm(d);
            lhs.dims.iter().find(|(x, _)| *x == dn).map(|(_, e)| e.clone())
        };
        if let Some(Shape::Flat(_)) = shape {
            let mut names = Vec::new();
            for r in &res {
                match r {
                    Sb::Item(e) => names.push(e.clone()),
                    Sb::Dim(d) => match fixed(d).flatten().or_else(|| self.mapped_item(d, lhs)) {
                        Some(e) => names.push(e),
                        // the left-hand side covers a whole set: writing it item by item fixes this position
                        None if lhs.dims.iter().any(|(_, e)| e.is_none()) && !lhs.scalar => {
                            *needs_elem = true;
                            names.push(d.clone());
                        }
                        None => {
                            if self.once(&format!("flatdim {who} {base}")) {
                                self.loss.approximated(format!("{base}[…] in {who}"), "a reference to a two-dimensional variable by dimension could not be matched to the left-hand side; left unresolved");
                            }
                            names.push(d.clone());
                        }
                    },
                    Sb::Unknown(u) => names.push(u.clone()),
                }
            }
            return q(&format!("{base}[{}]", names.join(",")));
        }
        if res.len() != 1 {
            if self.once(&format!("manysubs {who} {base}")) {
                self.loss.approximated(format!("{base}[…] in {who}"), "more subscripts than the variable has dimensions; the first was used");
            }
        }
        match res.into_iter().next() {
            Some(Sb::Item(e)) => format!("{}[{}]", q(base), q(&e)),
            Some(Sb::Dim(d)) => match fixed(&d) {
                Some(Some(e)) => format!("{}[{}]", q(base), q(&e)),
                // read through a set that is not the variable's own (a subrange of it, say): item by item
                Some(None) if matches!(&shape, Some(Shape::Set(vs)) if nrm(vs) != nrm(&d)) => {
                    *needs_elem = true;
                    q(base)
                }
                Some(None) => q(base),
                None if self.mapped_item(&d, lhs).is_some() => {
                    let e = self.mapped_item(&d, lhs).unwrap();
                    format!("{}[{}]", q(base), q(&e))
                }
                None if lhs.dims.iter().any(|(ld, e)| e.is_none() && self.range(ld).is_some_and(|rl| rl.map.as_ref().is_some_and(|(t, _)| nrm(t) == nrm(&d)) || self.range(&d).is_some_and(|rd| rd.map.as_ref().is_some_and(|(t, _)| nrm(t) == nrm(&rl.name))))) => {
                    // a mapping between the left-hand set and d: only item-by-item writing can express it
                    *needs_elem = true;
                    q(base)
                }
                None => {
                    // read through another dimension: fine when the lhs set belongs to it
                    let ok = lhs.dims.iter().any(|(ld, _)| {
                        self.range(ld).is_some_and(|r| r.map.as_ref().is_some_and(|(p, _)| nrm(p) == nrm(&d)))
                    });
                    let sub = lhs.dims.iter().any(|(ld, _)| {
                        let (Some(a), Some(b)) = (self.range(ld), self.range(&d)) else { return false };
                        a.items.iter().all(|i| b.items.iter().any(|j| nrm(i) == nrm(j))) && a.items.len() < b.items.len()
                    });
                    if !ok {
                        // a whole-set formula reading through another dimension: written item by item, each
                        // item then resolves the dimension (as a subrange, a superset or a mapping)
                        let whole_set = lhs.dims.iter().any(|(_, e)| e.is_none());
                        if sub || lhs.scalar || whole_set {
                            *needs_elem = true;
                        } else if self.once(&format!("unmapped {who} {base} {d}")) {
                            self.loss.approximated(format!("{base}[{d}] in {who}"), format!("read through the dimension {d} without a mapping to the variable's own; the engine may not be able to resolve it"));
                        }
                    }
                    q(base)
                }
            },
            Some(Sb::Unknown(u)) => {
                if self.once(&format!("unknownsub {u}")) {
                    self.loss.dropped(format!("subscript {u}"), "not a declared dimension or item; the reference was kept without a subscript");
                }
                q(base)
            }
            None => q(base),
        }
    }

    /// Constants and calls that need a different spelling.
    fn rewrite_words(&mut self, s: &str) -> String {
        let toks = lex(s);
        let mut out = String::new();
        let mut last = 0;
        let mut i = 0;
        while i < toks.len() {
            let t = toks[i];
            if t.k == LK::Word {
                let n = nrm(text(s, &t));
                let call = is_sym(s, toks.get(i + 1), "(");
                // PI → PI()
                if n == "pi" && !call && !self.var_names.contains("pi") {
                    out.push_str(&s[last..t.s]);
                    out.push_str("PI()");
                    last = t.e;
                    i += 1;
                    continue;
                }
                if call {
                    let upper = n.to_uppercase();
                    // RANDOM 0 1() → RANDOM UNIFORM(0, 1, 0)
                    if upper == "RANDOM 0 1" {
                        if let Some(close) = matching(s, &toks, i + 1) {
                            out.push_str(&s[last..t.s]);
                            out.push_str("RANDOM UNIFORM(0, 1, 0)");
                            last = toks[close].e;
                            i = close + 1;
                            if self.once("random01") {
                                self.loss.approximated("RANDOM 0 1", "mapped to RANDOM UNIFORM(0, 1, 0); the random numbers differ from Vensim's stream");
                            }
                            continue;
                        }
                    }
                    // QUEUE FIFO(inflow, outflow, profile, initial, age range): the profile that spreads the initial
                    // content over ages is dropped; the content starts as one step old
                    if upper == "QUEUE FIFO" {
                        if let Some(close) = matching(s, &toks, i + 1) {
                            let args: Vec<String> = split_top(&s[toks[i + 1].e..toks[close].s], ',').iter().map(|a| a.trim().to_string()).collect();
                            if args.len() == 5 {
                                out.push_str(&s[last..t.s]);
                                out.push_str(&format!("QUEUE FIFO({}, {}, {})", args[0], args[1], args[3]));
                                last = toks[close].e;
                                i = close + 1;
                                if self.once("queue profile") {
                                    self.loss.approximated("QUEUE FIFO", "the profile that spreads the initial content over ages is not read: the initial content starts as one time step old");
                                }
                                continue;
                            }
                        }
                    }
                    // functions that are other functions in disguise
                    if matches!(upper.as_str(), "FORECAST" | "TIME BASE" | "REINITIAL" | "DELAY INFORMATION") {
                        if let Some(close) = matching(s, &toks, i + 1) {
                            let args: Vec<String> = split_top(&s[toks[i + 1].e..toks[close].s], ',').iter().map(|a| a.trim().to_string()).collect();
                            let rep = match (upper.as_str(), args.len()) {
                                // FORECAST(input, average time, horizon[, initial trend]) = input (1 + trend × horizon)
                                ("FORECAST", 3 | 4) => Some(format!("(({}) * (1 + TREND({}, {}, {}) * ({})))", args[0], args[0], args[1], args.get(3).cloned().unwrap_or_else(|| "0".into()), args[2])),
                                // TIME BASE(t, dt) = t + dt × Time
                                ("TIME BASE", 2) => Some(format!("(({}) + ({}) * Time)", args[0], args[1])),
                                // REINITIAL(x): re-evaluated only when a run is re-initialised; in a run it is INITIAL
                                ("REINITIAL", 1) => Some(format!("INITIAL({})", args[0])),
                                // DELAY INFORMATION(input, delay, initial) is a first-order information delay
                                ("DELAY INFORMATION", 3) => Some(format!("SMOOTHI({}, {}, {})", args[0], args[1], args[2])),
                                _ => None,
                            };
                            if let Some(r) = rep {
                                out.push_str(&s[last..t.s]);
                                out.push_str(&r);
                                last = toks[close].e;
                                i = close + 1;
                                continue;
                            }
                        }
                    }
                    // GAME(x) → (x): outside Vensim's gaming mode the value is the formula itself
                    if upper == "GAME" {
                        if let Some(close) = matching(s, &toks, i + 1) {
                            let args = split_top(&s[toks[i + 1].e..toks[close].s], ',');
                            if args.len() == 1 {
                                out.push_str(&s[last..t.s]);
                                out.push_str(&format!("({})", args[0].trim()));
                                last = toks[close].e;
                                i = close + 1;
                                if self.once("game") {
                                    self.loss.approximated("GAME", "gaming inputs are read as their formula; the model runs as in a normal (non-gaming) Vensim simulation");
                                }
                                continue;
                            }
                        }
                    }
                    // SMOOTH N / DELAY N with the input as the initial value
                    if upper == "SMOOTH N" || upper == "DELAY N" {
                        if let Some(close) = matching(s, &toks, i + 1) {
                            let args = split_top(&s[toks[i + 1].e..toks[close].s], ',');
                            // SMOOTH N / DELAY N(input, delay, initial, order)
                            // the order may be a named constant: the engine needs it as a number
                            let order = args.get(3).and_then(|o| {
                                let o = o.trim();
                                o.parse::<u32>().ok().or_else(|| self.consts.get(&nrm(o)).filter(|v| v.fract() == 0.0 && **v >= 1.0).map(|v| *v as u32))
                            });
                            if args.len() == 4 && order.is_some() {
                                let f = if upper == "SMOOTH N" { "DELAYINF" } else { "DELAYMTR" };
                                let mut args = args.clone();
                                if args[3].trim().parse::<u32>().is_err() {
                                    if self.once(&format!("order {}", args[3].trim())) {
                                        self.loss.approximated(format!("{upper} order {}", args[3].trim()), format!("the order is read once as {}; changing that constant later does not change the number of stages", order.unwrap()));
                                    }
                                    args[3] = order.unwrap().to_string();
                                }
                                out.push_str(&s[last..t.s]);
                                if args[2].trim() == args[0].trim() {
                                    out.push_str(&format!("{f}({}, {}, {})", args[0].trim(), args[1].trim(), args[3].trim()));
                                } else {
                                    out.push_str(&format!("{f}({}, {}, {}, {})", args[0].trim(), args[1].trim(), args[3].trim(), args[2].trim()));
                                }
                                last = toks[close].e;
                                i = close + 1;
                                continue;
                            }
                        }
                    }
                    // anything else Vensim has and we do not
                    let known = expr::function_info(&upper).is_some()
                        || self.known_fns.contains(&n)
                        || self.var_names.contains(&n)
                        || matches!(upper.as_str(), "INTEG" | "WITH LOOKUP" | "INITIAL" | "TOTAL" | "AVERAGE");
                    if !known && vensim_only(&upper) && self.once(&format!("fn {upper}")) {
                        self.loss.preserved_not_simulable(format!("function {upper}"), "Vensim function with no equivalent in the engine; kept as written, so the model reports it as a problem and cannot run it");
                    }
                }
            }
            i += 1;
        }
        out.push_str(&s[last..]);
        out
    }

    // ---- sets ---------------------------------------------------------------------------

    fn add_sets(&mut self) {
        let ranges = self.ranges.clone();
        let mut done: HashSet<String> = HashSet::new();
        for r in &ranges {
            let (items, alias) = match &r.alias_of {
                Some(a) => (self.range(a).map(|x| x.items.clone()).unwrap_or_default(), true),
                None => (r.items.clone(), false),
            };
            if !done.insert(nrm(&r.name)) {
                continue;
            }
            let id = self.m.new_id();
            let mut set = ItemSet { id, name: r.name.clone(), items, ..Default::default() };
            if alias {
                self.loss.approximated(format!("dimension {}", r.name), "an alias dimension (<->) became a separate set with the same items");
            }
            if let Some((dim, list)) = &r.map {
                let target = ranges.iter().find(|x| nrm(&x.name) == nrm(dim));
                match target {
                    Some(t) if list.len() == set.items.len() && list.iter().all(|e| t.items.iter().any(|i| nrm(i) == nrm(e))) => {
                        set.parent = Some(t.name.clone());
                        set.parent_of = list.iter().map(|e| t.items.iter().find(|i| nrm(i) == nrm(e)).cloned().unwrap_or_default()).collect();
                    }
                    Some(t) if list.is_empty() && t.items.len() == set.items.len() => {
                        set.parent = Some(t.name.clone());
                        set.parent_of = t.items.clone();
                        self.loss.approximated(format!("mapping {} -> {}", r.name, t.name), "a positional mapping became \"each item belongs to the item in the same place\"");
                    }
                    _ => self.loss.dropped(format!("mapping {} -> {}", r.name, dim), "a mapping that is not one item-to-one-parent could not be expressed; the set has no parent"),
                }
            }
            self.m.sets.push(set);
        }
        // subranges are sets of their own
        for r in &ranges {
            let sub_of = ranges.iter().find(|o| nrm(&o.name) != nrm(&r.name) && o.items.len() > r.items.len() && !r.items.is_empty() && r.items.iter().all(|i| o.items.iter().any(|j| nrm(i) == nrm(j))));
            if let Some(o) = sub_of {
                if r.alias_of.is_none() && self.once(&format!("sub {}", r.name)) {
                    self.loss.approximated(format!("subrange {} of {}", r.name, o.name), "became an independent set; equations over it are written item by item where they mix with the larger set");
                }
            }
        }
    }

    // ---- building variables -------------------------------------------------------------

    fn items_covered(&self, tok: &str, except: &[String]) -> Vec<String> {
        let ex: Vec<String> = except.iter().flat_map(|e| match self.range(e) {
            Some(r) => r.items.iter().map(|i| nrm(i)).collect::<Vec<_>>(),
            None => vec![nrm(e)],
        }).collect();
        match self.range(tok) {
            Some(r) => r.items.iter().filter(|i| !ex.contains(&nrm(i))).cloned().collect(),
            None => match self.element(tok) {
                Some((_, e)) if !ex.contains(&nrm(&e)) => vec![e],
                _ => vec![],
            },
        }
    }

    /// `x[A, B] = 1, 2; 3, 4;` → one definition per combination of A and B, each a single number.
    fn split_table_constant(&mut self, d: Def) -> Vec<Def> {
        if d.op != Op::Eq || d.subs.is_empty() || !(d.rhs.contains(',') || d.rhs.contains(';')) {
            return vec![d];
        }
        let nums: Option<Vec<String>> = d
            .rhs
            .split(|c| c == ',' || c == ';')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.parse::<f64>().ok().map(|_| t.to_string()))
            .collect();
        let Some(nums) = nums else { return vec![d] };
        let lists: Vec<Vec<String>> = d.subs.iter().map(|t| self.items_covered(t.trim_end_matches('!'), &d.except)).collect();
        if lists.iter().any(|l| l.is_empty()) {
            return vec![d];
        }
        let total: usize = lists.iter().map(|l| l.len()).product();
        if total != nums.len() {
            self.loss.dropped(format!("variable {}", d.base), format!("a table of {} numbers for {total} combinations; left as written", nums.len()));
            return vec![d];
        }
        let mut out = Vec::with_capacity(total);
        let mut idx = vec![0usize; lists.len()];
        for v in nums {
            let subs = idx.iter().zip(&lists).map(|(i, l)| l[*i].clone()).collect();
            out.push(Def { subs, except: vec![], rhs: v, ..d.clone() });
            for k in (0..idx.len()).rev() {
                idx[k] += 1;
                if idx[k] < lists[k].len() {
                    break;
                }
                idx[k] = 0;
            }
        }
        out
    }

    /// Decide how each variable is shaped, before any equation is read.
    fn plan(&mut self, groups: &[(String, Vec<Def>)]) {
        for (name, defs) in groups {
            let n = defs.iter().map(|d| d.subs.len()).max().unwrap_or(0);
            if n == 0 {
                self.shapes.insert(nrm(name), Shape::Scalar);
                continue;
            }
            if n == 1 {
                let mut union: Vec<String> = Vec::new();
                let mut whole: Vec<String> = Vec::new();
                for d in defs {
                    let Some(tok) = d.subs.first() else { continue };
                    let cov = self.items_covered(tok, &d.except);
                    if self.range(tok).is_some() {
                        whole.push(self.range(tok).unwrap().name.clone());
                    }
                    for c in cov {
                        if !union.iter().any(|u| nrm(u) == nrm(&c)) {
                            union.push(c);
                        }
                    }
                }
                let fits: Vec<&RangeDef> = self.ranges.iter().filter(|r| r.alias_of.is_none() && union.iter().all(|u| r.items.iter().any(|i| nrm(i) == nrm(u)))).collect();
                let pick = whole
                    .iter()
                    .find_map(|w| fits.iter().find(|r| nrm(&r.name) == nrm(w)).map(|r| r.name.clone()))
                    .or_else(|| fits.iter().min_by_key(|r| r.items.len()).map(|r| r.name.clone()));
                match pick {
                    Some(set) => {
                        self.shapes.insert(nrm(name), Shape::Set(set));
                    }
                    None => {
                        self.shapes.insert(nrm(name), Shape::Scalar);
                        self.loss.dropped(format!("variable {name}"), "its subscripts are not declared dimensions, so it was read without them");
                    }
                }
                continue;
            }
            // two or more dimensions: one variable for each combination
            let mut dims = Vec::new();
            for k in 0..n {
                let toks: Vec<&String> = defs.iter().filter_map(|d| d.subs.get(k)).collect();
                let dim = toks.iter().find_map(|t| self.range(t).map(|r| r.name.clone())).or_else(|| {
                    let els: Vec<String> = toks.iter().filter_map(|t| self.element(t).map(|(_, e)| e)).collect();
                    self.ranges.iter().filter(|r| els.iter().all(|e| r.items.iter().any(|i| nrm(i) == nrm(e)))).min_by_key(|r| r.items.len()).map(|r| r.name.clone())
                });
                dims.push(dim.unwrap_or_default());
            }
            self.shapes.insert(nrm(name), Shape::Flat(dims));
        }
    }

    fn build_group(&mut self, name: &str, defs: &[Def]) {
        let shape = self.shapes.get(&nrm(name)).cloned().unwrap_or(Shape::Scalar);
        match shape {
            Shape::Scalar => {
                let Some(d) = defs.first() else { return };
                if defs.len() > 1 {
                    self.loss.dropped(format!("variable {name}"), "defined more than once; the first definition was used");
                }
                let lhs = Lhs { dims: vec![], scalar: true };
                let mut ne = false;
                let id = self.make_scalar(name, d, &lhs, &mut ne);
                let _ = id;
            }
            Shape::Set(set) => self.build_set_var(name, &set, defs),
            Shape::Flat(dims) => self.build_flat(name, &dims, defs),
        }
    }

    /// Classify one right-hand side and create the variable.
    fn make_scalar(&mut self, name: &str, d: &Def, lhs: &Lhs, ne: &mut bool) -> u64 {
        let id = self.m.new_id();
        let mut v = Var::new(id, name.trim_matches('"'), Kind::Aux, [0.0, 0.0]);
        let (units, slider) = split_units(&d.units);
        v.units = units;
        v.comment = d.comment.clone();
        self.fill_var(&mut v, d, lhs, ne, slider);
        self.m.vars.push(v);
        id
    }

    /// Set kind, equation and table of `v` from a definition.
    fn fill_var(&mut self, v: &mut Var, d: &Def, lhs: &Lhs, ne: &mut bool, slider: Option<[f64; 2]>) {
        let who = v.name.clone();
        // data variables
        let upper = d.rhs.trim().to_uppercase();
        let is_get = upper.starts_with("GET ") || upper.starts_with("GET_");
        if d.op == Op::Bare || d.op == Op::Data || is_get {
            let constants = upper.contains("CONSTANTS");
            let text = if d.rhs.trim().is_empty() { "(no equation: a data variable)".to_string() } else { d.rhs.trim().to_string() };
            if constants {
                v.kind = Kind::Constant;
                v.equation = "0".into();
            } else {
                v.kind = Kind::Lookup;
                v.equation = "Time".into();
                v.table = vec![[self.m.settings.initial_time, 0.0], [self.m.settings.initial_time + 1.0, 0.0]];
            }
            v.comment = format!("[Vensim data: {text}] {}", v.comment).trim().to_string();
            self.loss.dropped(format!("data variable {who}"), format!("{} is not imported; {} stands in until real values are loaded (Vensim: {text})", if is_get { "the file data" } else { "the data" }, if constants { "a constant 0" } else { "a lookup over Time that is zero everywhere" }));
            return;
        }
        if d.op == Op::Lookup {
            let mut body = d.rhs.clone();
            for kw in [":INTERPOLATE:", ":HOLD BACKWARD:", ":LOOK FORWARD:", ":RAW:"] {
                if let Some(p) = body.to_uppercase().find(kw) {
                    body.replace_range(p..p + kw.len(), "");
                    if kw != ":INTERPOLATE:" {
                        self.loss.approximated(format!("lookup {who}"), format!("{} lookups are read with straight-line interpolation", kw.trim_matches(':').to_lowercase()));
                    }
                }
            }
            match expr::parse_table(body.trim()) {
                Ok(t) => {
                    v.kind = Kind::Lookup;
                    v.table = t;
                    // a lookup written as `name(table)` is used as a function: no input
                }
                Err(e) => {
                    self.loss.dropped(format!("lookup {who}"), format!("the table could not be read ({e}); the variable is empty"));
                    v.kind = Kind::Aux;
                    v.equation = "0".into();
                }
            }
            return;
        }
        let rhs = d.rhs.trim();
        if let Some((rate, init)) = integ_args(rhs) {
            v.kind = Kind::Stock;
            let init = self.rewrite(&init, lhs, &who, ne);
            v.equation = init;
            // the rate is handled once all variables exist
            let rate = self.rewrite(&rate, lhs, &who, ne);
            self.stock_rates.push((v.id, rate, vec![]));
            return;
        }
        let rewritten = self.rewrite(rhs, lhs, &who, ne);
        if let Ok(Expr::WithLookup(_, table)) = expr::parse(&rewritten) {
            v.kind = Kind::Lookup;
            v.table = table;
            let src = rewritten.trim();
            let inner = &src[src.find('(').map(|p| p + 1).unwrap_or(0)..];
            let parts = split_top(inner, ',');
            v.equation = parts.first().map(|s| s.trim().to_string()).unwrap_or_default();
            return;
        }
        if upper.contains("INTEG") && integ_args(rhs).is_none() && upper.contains("INTEG(") || upper.contains("INTEG (") {
            self.loss.approximated(format!("stock {who}"), "INTEG inside a larger formula; kept as written");
        }
        v.equation = rewritten;
        if rhs.parse::<f64>().is_ok() {
            v.kind = Kind::Constant;
            v.slider = slider;
        }
    }

    fn build_flat(&mut self, name: &str, dims: &[String], defs: &[Def]) {
        let ranges: Vec<Option<RangeDef>> = dims.iter().map(|d| self.range(d).cloned()).collect();
        if ranges.iter().any(|r| r.is_none()) {
            self.loss.dropped(format!("variable {name}"), "one of its dimensions is not declared; it was left out");
            return;
        }
        let ranges: Vec<RangeDef> = ranges.into_iter().flatten().collect();
        if let Some(r) = ranges.iter().find(|r| r.items.is_empty()) {
            // a dimension we could not expand (e.g. defined by a mapping or an external file) has no items
            self.loss.dropped(format!("variable {name}"), format!("dimension {} has no items the reader could expand; the variable was left out", r.name));
            return;
        }
        let total: usize = ranges.iter().map(|r| r.items.len().max(1)).product();
        if total > 5000 {
            self.loss.dropped(format!("variable {name}"), format!("{total} combinations of {}: too many to write out one by one", dims.join(" × ")));
            return;
        }
        self.loss.approximated(format!("variable {name}[{}]", dims.join(",")), format!("{total} combinations of {} became {total} separate variables named {name}[a,b] (sets hold one dimension)", dims.join(" × ")));
        let mut idx = vec![0usize; ranges.len()];
        loop {
            let combo: Vec<String> = idx.iter().zip(&ranges).map(|(i, r)| r.items[*i].clone()).collect();
            // the definition that covers this combination
            let hit = defs.iter().rev().find(|d| {
                d.subs.iter().enumerate().all(|(k, tok)| {
                    let cov = self.items_covered(tok, &d.except);
                    // a position with an excepted list applies the exception to that position only
                    cov.iter().any(|c| nrm(c) == nrm(&combo[k.min(combo.len() - 1)]))
                })
            });
            if let Some(d) = hit {
                let lhs = Lhs { dims: ranges.iter().zip(&combo).map(|(r, e)| (nrm(&r.name), Some(e.clone()))).collect(), scalar: false };
                let mut ne = false;
                let full = format!("{name}[{}]", combo.join(","));
                self.make_scalar(&full, d, &lhs, &mut ne);
                self.var_names.insert(nrm(&full));
            }
            let mut k = ranges.len();
            loop {
                if k == 0 {
                    return;
                }
                k -= 1;
                idx[k] += 1;
                if idx[k] < ranges[k].items.len() {
                    break;
                }
                idx[k] = 0;
            }
        }
    }

    fn build_set_var(&mut self, name: &str, set: &str, defs: &[Def]) {
        let Some(rs) = self.range(set).cloned() else { return };
        let items = rs.items.clone();
        // which definition covers which item
        let mut cover: Vec<Option<usize>> = vec![None; items.len()];
        let mut whole: Vec<bool> = vec![false; defs.len()];
        for (di, d) in defs.iter().enumerate() {
            let Some(tok) = d.subs.first() else { continue };
            let cov = self.items_covered(tok, &d.except);
            whole[di] = self.range(tok).is_some_and(|r| nrm(&r.name) == nrm(set)) && d.except.is_empty();
            for c in cov {
                if let Some(k) = items.iter().position(|i| nrm(i) == nrm(&c)) {
                    cover[k] = Some(di);
                }
            }
        }
        // a definition over exactly the set is written once; anything else item by item
        let id = self.m.new_id();
        let mut v = Var::new(id, name.trim_matches('"'), Kind::Aux, [0.0, 0.0]);
        let first = defs.iter().position(|_| true).unwrap_or(0);
        let base_def = defs.iter().enumerate().find(|(i, _)| whole[*i]).map(|(i, _)| i).or_else(|| cover.iter().flatten().next().copied()).unwrap_or(first);
        let (units, slider) = split_units(&defs[base_def].units);
        v.units = units;
        v.comment = defs[base_def].comment.clone();
        v.set = Some(rs.name.clone());
        let who = v.name.clone();
        let base = &defs[base_def];
        // try the base definition as "the same for every item"
        let lhs_all = Lhs { dims: vec![(nrm(&rs.name), None)], scalar: false };
        let mut ne = false;
        let mark_stock = self.stock_rates.len();
        self.fill_var(&mut v, base, &lhs_all, &mut ne, slider);
        let expand_all = ne;
        let mut per_item_rates: Vec<(String, String)> = Vec::new();
        if expand_all {
            // the formula reads another set in a way only item-by-item writing can express
            self.stock_rates.truncate(mark_stock);
            if self.once(&format!("expand {who}")) {
                self.loss.approximated(format!("variable {who}"), format!("reads a variable over a different or smaller dimension, so it is written one {} at a time", rs.name));
            }
        }
        let mut base_set = !expand_all;
        for (k, item) in items.iter().enumerate() {
            let Some(di) = cover[k] else { continue };
            let d = &defs[di];
            // the base definition is already written for every item, unless it had to be expanded item by item
            if di == base_def && base_set && !expand_all {
                continue;
            }
            let lhs = Lhs { dims: self.dims_for(item), scalar: false };
            let mut tmp = Var::new(0, "", Kind::Aux, [0.0, 0.0]);
            tmp.id = id;
            let mark = self.stock_rates.len();
            let mut ne2 = false;
            self.fill_var(&mut tmp, d, &lhs, &mut ne2, None);
            let rate = if self.stock_rates.len() > mark { Some(self.stock_rates.remove(mark).1) } else { None };
            if !base_set {
                // first item defines the base
                v.kind = tmp.kind;
                v.equation = tmp.equation.clone();
                v.table = tmp.table.clone();
                v.slider = tmp.slider;
                base_set = true;
                if let Some(r) = rate {
                    per_item_rates.push((item.clone(), r));
                }
                continue;
            }
            if tmp.kind != v.kind && !(matches!(tmp.kind, Kind::Aux | Kind::Constant) && matches!(v.kind, Kind::Aux | Kind::Constant)) {
                self.loss.dropped(format!("{who}[{item}]"), format!("is a {} while the other items are a {}; the formula was kept but not the kind", tmp.kind.label().to_lowercase(), v.kind.label().to_lowercase()));
            }
            if tmp.kind == Kind::Constant && v.kind == Kind::Aux {
                // one item is a plain number: still a formula
            }
            if tmp.equation != v.equation || tmp.table != v.table {
                v.by_item.push(ItemValue { item: item.clone(), equation: tmp.equation, table: tmp.table });
            }
            if let Some(r) = rate {
                per_item_rates.push((item.clone(), r));
            }
        }
        // the constant with some items that are formulas is a formula
        if v.kind == Kind::Constant && v.by_item.iter().any(|b| b.equation.trim().parse::<f64>().is_err() && b.table.is_empty()) {
            v.kind = Kind::Aux;
        }
        // stock rates: the base rate plus item differences
        if v.kind == Kind::Stock {
            let mut base_rate = String::new();
            if let Some(pos) = (!expand_all).then(|| self.stock_rates.iter().position(|(sid, _, _)| *sid == id)).flatten() {
                base_rate = self.stock_rates.remove(pos).1;
            }
            let mut diffs: Vec<(String, String)> = per_item_rates.into_iter().filter(|(_, r)| *r != base_rate).collect();
            if expand_all && !diffs.is_empty() {
                base_rate = diffs.remove(0).1;
            }
            self.stock_rates.push((id, base_rate, diffs));
        }
        self.m.vars.push(v);
    }

    /// The left-hand-side dimensions that fix `item`: every range it belongs to.
    fn dims_for(&self, item: &str) -> Vec<(String, Option<String>)> {
        // a range with a mapping (`a: x, y -> b`) is not fixed by the item: it is read through its mapping
        self.ranges.iter().filter(|r| r.map.is_none() && r.items.iter().any(|i| nrm(i) == nrm(item))).map(|r| (nrm(&r.name), Some(item.to_string()))).collect()
    }

    // ---- stocks and flows ---------------------------------------------------------------

    fn resolve_stocks(&mut self) {
        let rates = std::mem::take(&mut self.stock_rates);
        for (sid, rate, diffs) in rates {
            let parsed = expr::parse(&rate);
            let mut terms = Vec::new();
            let simple = diffs.is_empty()
                && !rate.contains('[')
                && parsed.as_ref().map(|e| signed_terms(e, 1.0, &mut terms)).unwrap_or(false)
                && !terms.is_empty()
                && terms.iter().all(|(n, _)| {
                    self.m.by_name(n).is_some_and(|v| {
                        matches!(v.kind, Kind::Aux | Kind::Flow)
                            && v.set == self.m.var(sid).and_then(|s| s.set.clone())
                            && v.id != sid
                    })
                })
                // a flow can fill one stock and drain one stock, once each
                && terms.iter().all(|(n, sign)| {
                    let v = self.m.by_name(n).unwrap();
                    if *sign > 0.0 { v.to.is_none() || v.to == Some(sid) } else { v.from.is_none() || v.from == Some(sid) }
                });
            if simple {
                for (n, sign) in terms {
                    let fid = self.m.by_name(&n).unwrap().id;
                    let f = self.m.var_mut(fid).unwrap();
                    f.kind = Kind::Flow;
                    if sign > 0.0 {
                        f.to = Some(sid);
                    } else {
                        f.from = Some(sid);
                    }
                }
            } else {
                let (sname, sset) = {
                    let s = self.m.var(sid).unwrap();
                    (s.name.clone(), s.set.clone())
                };
                let id = self.m.new_id();
                let mut f = Var::new(id, &format!("{sname} net flow"), Kind::Flow, [0.0, 0.0]);
                f.equation = rate;
                f.to = Some(sid);
                f.set = sset;
                for (item, r) in diffs {
                    f.by_item.push(ItemValue { item, equation: r, table: vec![] });
                }
                self.m.vars.push(f);
                self.var_names.insert(nrm(&format!("{sname} net flow")));
            }
        }
    }

    // ---- macros -------------------------------------------------------------------------

    fn read_macro(&mut self, block: &str) {
        // block: header line "NAME(arg, arg)" then entries
        let block = block.trim();
        let (head, rest) = block.split_once('\n').unwrap_or((block, ""));
        let head = head.trim();
        let (mname, params) = match head.find('(') {
            Some(p) => (head[..p].trim().to_string(), split_top(head[p + 1..].trim_end().trim_end_matches(')'), ',').into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>()),
            None => (head.to_string(), vec![]),
        };
        let mut defs: Vec<(String, String, String, String)> = Vec::new(); // name, rhs, units, comment
        for raw in split_entries(rest) {
            if let Some(Parsed::Def(d)) = parse_chunk(&raw) {
                defs.push((d.base, d.rhs, d.units, d.comment));
            }
        }
        let Some(out) = defs.iter().find(|d| nrm(&d.0) == nrm(&mname)).cloned() else {
            self.loss.dropped(format!("macro {mname}"), "no output equation found");
            return;
        };
        if defs.iter().any(|d| d.1.to_uppercase().contains("INTEG")) {
            self.loss.dropped(format!("macro {mname}"), "contains a stock (INTEG), which a function written in the equation language cannot hold; calls to it will report a problem");
            return;
        }
        // inline the helper equations into the output formula
        let mut body = out.1.clone();
        for _ in 0..8 {
            let mut changed = false;
            for (n, rhs, _, _) in defs.iter().filter(|d| nrm(&d.0) != nrm(&mname)) {
                if params.iter().any(|p| nrm(p) == nrm(n)) {
                    continue;
                }
                let toks = lex(&body);
                let mut s = String::new();
                let mut last = 0;
                for t in &toks {
                    if matches!(t.k, LK::Word | LK::Quoted) && nrm(text(&body, t)) == nrm(n) {
                        s.push_str(&body[last..t.s]);
                        s.push_str(&format!("({})", rhs.trim()));
                        last = t.e;
                        changed = true;
                    }
                }
                s.push_str(&body[last..]);
                body = s;
            }
            if !changed {
                break;
            }
        }
        self.known_fns.insert(nrm(&mname));
        self.m.functions.push(UserFn { name: mname.clone(), params, body, description: out.3.clone() });
        self.loss.approximated(format!("macro {mname}"), "became a function of the equation language (the macro body is substituted at each call)");
    }
}

// ------------------------------------------------------------------ the sketch

fn split_csv_quoted(line: &str) -> Vec<String> {
    let (mut out, mut cur, mut quote) = (Vec::new(), String::new(), false);
    for c in line.chars() {
        match c {
            '"' => {
                quote = !quote;
                cur.push(c);
            }
            ',' if !quote => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

// ------------------------------------------------------------------ public entry points

/// Read a Vensim model, with a report of everything that was approximated or left out.
pub fn read(text: &str) -> Result<(Model, LossReport), String> {
    let text = text.replace("\r\n", "\n");
    let text = text.trim_start_matches('\u{feff}').to_string();
    // the file = equations, then the sketch, then the run-settings block
    let (eqs, tail) = match text.find("\\\\\\---///") {
        Some(p) => (text[..p].to_string(), text[p..].to_string()),
        None => (text.clone(), String::new()),
    };
    let (sketch, settings_block) = match tail.find("///---\\\\\\") {
        Some(p) => (tail[..p].to_string(), tail[p..].to_string()),
        None => (tail.clone(), String::new()),
    };
    let eqs = strip_braces(&eqs);

    let mut r = Reader {
        m: Model { title: "Imported model".into(), settings: crate::model::Settings { units_scale_values: false, ..Default::default() }, ..Default::default() },
        loss: LossReport::default(),
        ranges: Vec::new(),
        shapes: HashMap::new(),
        known_fns: HashSet::new(),
        consts: HashMap::new(),
        var_names: HashSet::new(),
        said: HashSet::new(),
        stock_rates: Vec::new(),
    };

    // macros first: they sit between :MACRO: and :END OF MACRO:
    let mut body = String::new();
    let upper = eqs.to_uppercase();
    let mut pos = 0;
    while pos < eqs.len() {
        match upper[pos..].find(":MACRO:") {
            Some(a) => {
                body.push_str(&eqs[pos..pos + a]);
                let start = pos + a + 7;
                let end = upper[start..].find(":END OF MACRO:").map(|e| start + e).unwrap_or(eqs.len());
                r.read_macro(&eqs[start..end]);
                pos = (end + ":END OF MACRO:".len()).min(eqs.len());
            }
            None => {
                body.push_str(&eqs[pos..]);
                break;
            }
        }
    }
    crate::userfn::set(&r.m.functions);

    // entries
    let mut defs: Vec<Def> = Vec::new();
    for raw in split_entries(&body) {
        let t = raw.trim();
        if t.is_empty() || t.starts_with("****") {
            continue;
        }
        match parse_chunk(t) {
            Some(Parsed::Range(rd)) => r.ranges.push(rd),
            Some(Parsed::Def(d)) => defs.push(d),
            None => {}
        }
    }
    if defs.is_empty() && r.ranges.is_empty() {
        return Err("No equations were found in this file".into());
    }
    // a dimension may list other dimensions among its items (`dim: A, subdim1, B`): put their items in place,
    // in order, once all ranges are read (nested, a few levels deep)
    for _ in 0..8 {
        let mut changed = false;
        for k in 0..r.ranges.len() {
            let mut out: Vec<String> = Vec::new();
            let mut hit = false;
            for it in &r.ranges[k].items {
                match r.ranges.iter().position(|x| nrm(&x.name) == nrm(it)) {
                    Some(j) if j != k && !r.ranges[j].items.is_empty() => {
                        out.extend(r.ranges[j].items.iter().cloned());
                        hit = true;
                    }
                    _ => out.push(it.clone()),
                }
            }
            if hit {
                r.ranges[k].items = out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // an alias (`a <-> b`) may come before `b` in the file: give it b's items once all ranges are read
    for k in 0..r.ranges.len() {
        let mut target = r.ranges[k].alias_of.clone();
        let mut hops = 0;
        while let Some(t) = target.clone() {
            hops += 1;
            match r.ranges.iter().find(|x| nrm(&x.name) == nrm(&t)) {
                Some(x) if x.alias_of.is_none() || hops > 10 => {
                    r.ranges[k].items = x.items.clone();
                    break;
                }
                Some(x) => target = x.alias_of.clone(),
                None => break,
            }
        }
    }

    // run settings
    let mut kept: Vec<Def> = Vec::new();
    let mut saveper: Option<String> = None;
    for d in defs {
        let n = nrm(&d.base);
        let val = if d.subs.is_empty() { const_eval(&d.rhs) } else { None };
        match (n.as_str(), val) {
            ("initial time", Some(v)) => r.m.settings.initial_time = v,
            ("final time", Some(v)) => {
                r.m.settings.final_time = v;
                r.m.settings.time_units = d.units.clone();
            }
            ("time step", Some(v)) => r.m.settings.time_step = v,
            ("saveper", _) => saveper = Some(d.rhs.trim().to_string()),
            _ => kept.push(d),
        }
    }
    r.m.settings.method = Method::Euler;
    if let Some(s) = saveper {
        if nrm(&s) != "time step" && const_eval(&s) != Some(r.m.settings.time_step) {
            r.loss.dropped("SAVEPER", format!("the save interval ({s}) is ignored: every step is kept"));
        }
    }
    if !settings_block.trim().is_empty() {
        r.loss.approximated("run settings block", "the integration type and output options Vensim stores after the sketch are not read; Euler is assumed");
    }

    // sets and shapes
    r.add_sets();
    // array constants written as a table: `x[A, B] = 1, 2; 3, 4;` gives one value per combination, the last
    // subscript varying fastest (`,` between columns, `;` between rows), as Vensim lays them out
    let kept: Vec<Def> = kept.into_iter().flat_map(|d| r.split_table_constant(d)).collect();
    for d in &kept {
        if d.op == Op::Eq && d.subs.is_empty() {
            if let Some(v) = const_eval(&d.rhs) {
                r.consts.insert(nrm(&d.base), v);
            }
        }
    }
    // group definitions by variable, in order of first appearance
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<Def>> = HashMap::new();
    for d in kept {
        let key = nrm(&d.base);
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(d);
    }
    let named: Vec<(String, Vec<Def>)> = order.iter().map(|k| (groups[k][0].base.clone(), groups[k].clone())).collect();
    r.plan(&named);
    for (name, _) in &named {
        r.var_names.insert(nrm(name));
    }
    for (name, defs) in &named {
        r.build_group(name, defs);
    }
    r.resolve_stocks();

    // sketch: positions and arrows, view by view
    let mut m = r.m;
    let mut loss = r.loss;
    read_sketch(&mut m, &mut loss, &sketch);
    // arrows implied by equations but missing from the sketch
    let pairs: Vec<(u64, u64)> = m
        .vars
        .iter()
        .flat_map(|v| {
            let mut refs = Vec::new();
            if let Ok(e) = expr::parse(&v.equation) {
                expr::references(&e, &mut refs);
            }
            let to = v.id;
            refs.into_iter().filter_map(|r| m.by_name(&r).map(|s| (s.id, to))).collect::<Vec<_>>()
        })
        .collect();
    for (a, b) in pairs {
        m.add_link(a, b);
    }
    apply_tags(&mut m);
    crate::userfn::set(&m.functions);
    Ok((m, loss))
}

/// `[dynamist: combine=average; never-negative; lever=2]` at the end of a comment restores what
/// a Vensim model has no field for (written by `mdlw`).
fn apply_tags(m: &mut Model) {
    for v in &mut m.vars {
        let (a, open) = match (v.comment.rfind("[dynamist:"), v.comment.rfind("[flowcanvas:")) {
            (Some(a), _) => (a, "[dynamist:".len()),
            (None, Some(a)) => (a, "[flowcanvas:".len()),
            _ => continue,
        };
        let Some(b) = v.comment[a..].find(']') else { continue };
        let tags: Vec<String> = v.comment[a + open..a + b].split(';').map(|t| t.trim().to_string()).collect();
        let rest = format!("{}{}", &v.comment[..a], &v.comment[a + b + 1..]).trim().to_string();
        for t in tags {
            match t.as_str() {
                "combine=average" => v.rollup = crate::model::Rollup::Average,
                "never-negative" => v.non_negative = true,
                "lever" => v.lever = true,
                other => {
                    if let Some(x) = other.strip_prefix("lever=").and_then(|x| x.parse::<f64>().ok()) {
                        v.lever = true;
                        v.lever_scale = x;
                    }
                }
            }
        }
        v.comment = rest;
    }
}

/// Read a Vensim model; the loss report is dropped.
pub fn import(text: &str) -> Result<Model, String> {
    read(text).map(|(m, _)| m)
}

fn read_sketch(m: &mut Model, loss: &mut LossReport, sketch: &str) {
    let mut placed = vec![false; m.vars.len()];
    let mut views = 0;
    let mut view_names: Vec<String> = Vec::new();
    let mut seen_elsewhere = false;
    let mut other = 0;
    let mut arrows: Vec<(u64, u64)> = Vec::new();
    // ids are only unique within a view; a flow's valve (type 11) stands for its label (type 10, shape 40)
    let mut ids: HashMap<String, u64> = HashMap::new();
    let mut valve: Option<String> = None;
    let mut pending: Vec<(String, String)> = Vec::new();
    fn flush(ids: &HashMap<String, u64>, pending: &mut Vec<(String, String)>, arrows: &mut Vec<(u64, u64)>) {
        for (a, b) in pending.drain(..) {
            if let (Some(&x), Some(&y)) = (ids.get(&a), ids.get(&b)) {
                arrows.push((x, y));
            }
        }
    }
    for line in sketch.lines() {
        if let Some(name) = line.strip_prefix('*') {
            flush(&ids, &mut pending, &mut arrows);
            ids.clear();
            valve = None;
            views += 1;
            view_names.push(name.trim().to_string());
            continue;
        }
        let f = split_csv_quoted(line);
        if f.len() < 5 {
            continue;
        }
        match f[0].as_str() {
            "10" => {
                let name = nrm(&unquote(&f[2]));
                let (Ok(x), Ok(y)) = (f[3].parse::<f32>(), f[4].parse::<f32>()) else { continue };
                if let Some(i) = m.vars.iter().position(|v| nrm(&v.name) == name) {
                    if !placed[i] {
                        m.vars[i].pos = [x, y];
                        placed[i] = true;
                    } else {
                        seen_elsewhere = true;
                    }
                    ids.insert(f[1].to_string(), m.vars[i].id);
                    if f.get(7).map(|s| s.trim()) == Some("40") {
                        if let Some(vl) = valve.take() {
                            ids.insert(vl, m.vars[i].id);
                        }
                    }
                }
            }
            "11" => valve = Some(f[1].to_string()),
            "12" => {
                // clouds (code 48) are redrawn from the flows; anything else is a comment object
                if f[2].trim() != "48" {
                    other += 1;
                }
            }
            "1" => pending.push((f[2].clone(), f[3].clone())),
            _ => {}
        }
    }
    flush(&ids, &mut pending, &mut arrows);
    arrows.sort();
    arrows.dedup();
    for (a, b) in arrows {
        let (ka, kb) = (m.var(a).map(|v| v.kind), m.var(b).map(|v| v.kind));
        if kb == Some(Kind::Stock) {
            continue; // a pipe into a stock
        }
        // a pipe from a stock into a flow is not an information arrow unless the flow reads the stock
        if ka == Some(Kind::Stock) && kb == Some(Kind::Flow) {
            let reads = {
                let (s, f) = (m.var(a).unwrap(), m.var(b).unwrap());
                expr::parse(&f.equation)
                    .map(|e| {
                        let mut refs = Vec::new();
                        expr::references(&e, &mut refs);
                        refs.contains(&nrm(&s.name))
                    })
                    .unwrap_or(false)
            };
            if !reads {
                continue;
            }
        }
        m.add_link(a, b);
    }
    // anything the sketch didn't place goes in a grid; flows sit between their stocks
    let mut col = 0;
    for i in 0..m.vars.len() {
        if i >= placed.len() || !placed[i] {
            let row = col / 5;
            m.vars[i].pos = [120.0 + (col % 5) as f32 * 190.0, 120.0 + row as f32 * 130.0];
            col += 1;
        }
    }
    let snapshot = m.vars.clone();
    for v in m.vars.iter_mut().filter(|v| v.kind == Kind::Flow) {
        let a = v.from.and_then(|id| snapshot.iter().find(|s| s.id == id)).map(|s| s.pos);
        let b = v.to.and_then(|id| snapshot.iter().find(|s| s.id == id)).map(|s| s.pos);
        match (a, b) {
            (Some(a), Some(b)) => v.pos = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
            (Some(a), None) => {
                v.pos = [a[0] + 120.0, a[1]];
                v.to_pt = [a[0] + 220.0, a[1]];
            }
            (None, Some(b)) => {
                v.pos = [b[0] - 120.0, b[1]];
                v.from_pt = [b[0] - 220.0, b[1]];
            }
            (None, None) => {
                v.from_pt = [v.pos[0] - 80.0, v.pos[1]];
                v.to_pt = [v.pos[0] + 80.0, v.pos[1]];
            }
        }
    }
    if views > 1 || seen_elsewhere {
        loss.approximated("views", format!("{views} sketch views were merged: each variable keeps the position of its first appearance, and the view names ({}) are not kept", view_names.join(", ")));
    }
    if other > 0 {
        loss.dropped("sketch comments", format!("{other} comment object(s) were not read"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_simple_model() {
        let src = "{UTF-8}\nPopulation= INTEG (\n\tbirths-deaths,\n\t\t100)\n\t~\tPeople\n\t~\t\t|\n\nbirths=\n\tPopulation*birth rate\n\t~\tPeople/Year\n\t~\t\t|\n\ndeaths = Population / lifetime ~ People/Year ~ |\nbirth rate = 0.03 ~ 1/Year [0,0.1] ~ |\nlifetime = 70 ~ Year ~ |\neffect(\n\t[(0,0)-(2,2)],(0,1),(1,0.5),(2,0)) ~ Dmnl ~ |\n********************************************************\n\t.Control\n********************************************************~\n\t\tSimulation Control Parameters\n\t|\n\nFINAL TIME  = 100\n\t~\tYear\n\t~\tThe final time for the simulation.\n\t|\n\nINITIAL TIME  = 0 ~ Year ~ |\nSAVEPER = TIME STEP ~ Year ~ |\nTIME STEP  = 0.5 ~ Year ~ |\n\n\\\\\\---/// Sketch information - do not modify anything except names\nV300  Do not put anything below this section - it will be ignored\n*View 1\n10,1,Population,300,200,40,20,3,3,0,0,0,0,0,0\n10,2,birth rate,150,100,40,20,8,3,0,0,0,0,0,0\n1,3,2,1,0,0,0,0,0,64,0,-1--1--1,,1|(0,0)|\n";
        let m = super::import(src).unwrap();
        assert_eq!(m.settings.final_time, 100.0);
        assert_eq!(m.settings.time_step, 0.5);
        let p = m.by_name("population").unwrap();
        assert_eq!(p.kind, crate::model::Kind::Stock);
        assert_eq!(p.pos, [300.0, 200.0]);
        assert_eq!(m.by_name("births").unwrap().to, Some(p.id));
        assert_eq!(m.by_name("deaths").unwrap().from, Some(p.id));
        assert_eq!(m.by_name("birth rate").unwrap().slider, Some([0.0, 0.1]));
        assert_eq!(m.by_name("effect").unwrap().table.len(), 3);
        let c = crate::sim::compile(&m).ok().unwrap();
        assert!(crate::sim::simulate(&c, &m, "x").is_ok());
    }

    fn last(m: &Model, name: &str) -> f64 {
        let c = crate::sim::compile(m).unwrap_or_else(|p| panic!("{:?}", p.iter().map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>()));
        let r = crate::sim::simulate(&c, m, "x").unwrap();
        *r.get(name).unwrap_or_else(|| panic!("no series {name}; have {:?}", r.series.keys().collect::<Vec<_>>())).last().unwrap()
    }

    const CONTROL: &str = "\nFINAL TIME = 10 ~ Day ~ |\nINITIAL TIME = 0 ~ Day ~ |\nSAVEPER = TIME STEP ~ Day ~ |\nTIME STEP = 1 ~ Day ~ |\n";

    #[test]
    fn subscripts_become_sets_with_per_item_values_and_totals() {
        let src = format!("Tank: T1, T2, T3 -> (Region: R1, R1, R2) ~ ~ |\nRegion: R1, R2 ~ ~ |\n\
            Stock[Tank] = INTEG(fill[Tank] - burn[Tank], 100) ~ MB ~ |\n\
            fill[Tank] = 5 ~ MB/Day ~ |\n\
            burn[T1] = 1 ~ MB/Day ~ |\nburn[T2] = 2 ~ MB/Day ~ |\nburn[T3] = 3 ~ MB/Day ~ |\n\
            Total stock = SUM(Stock[Tank!]) ~ MB ~ |\n\
            Region stock[Region] = SUM(Stock[Tank!]) ~ MB ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        assert_eq!(m.sets.len(), 2);
        let tank = m.sets.iter().find(|s| s.name == "Tank").unwrap();
        assert_eq!(tank.items, ["T1", "T2", "T3"]);
        assert_eq!(tank.parent.as_deref(), Some("Region"));
        assert_eq!(tank.parent_of, ["R1", "R1", "R2"]);
        let stock = m.by_name("stock").unwrap();
        assert_eq!(stock.kind, Kind::Stock);
        assert_eq!(stock.set.as_deref(), Some("Tank"));
        assert_eq!(m.by_name("burn").unwrap().by_item.len(), 2, "the first item is the base, the others differ");
        // per-item: 100 + 10*(5-burn): 140, 130, 120
        assert!((last(&m, "total stock") - 390.0).abs() < 1e-9);
        assert!((last(&m, "stock[T2]") - 130.0).abs() < 1e-9);
        // a sum over all tanks inside a per-region formula is written out
        assert!((last(&m, "region stock[R1]") - 390.0).abs() < 1e-9, "Vensim's SUM(Stock[Tank!]) is all tanks, for each region");
        assert!(loss.items.iter().any(|i| i.construct.contains("SUM")), "{}", loss.summary());
        // the rate is made of constants, so it becomes one net flow per tank
        let net = m.by_name("stock net flow").unwrap();
        assert_eq!((net.kind, net.to, net.set.as_deref()), (Kind::Flow, Some(stock.id), Some("Tank")));
    }

    #[test]
    fn except_and_subranges_cover_the_right_items() {
        let src = format!("Dim: A, B, C, D ~ ~ |\nSub: B, C ~ ~ |\n\
            x[Dim] = 1 ~ ~ |\nx[Dim] :EXCEPT: [A] = 2 ~ ~ |\n\
            y[Sub] = 7 ~ ~ |\n\
            sx = x[A] + x[B] + x[D] ~ ~ |{CONTROL}");
        let (m, _) = read(&src).unwrap();
        // x: A is 1 ... but the second equation covers B, C, D with 2
        assert!((last(&m, "sx") - (1.0 + 2.0 + 2.0)).abs() < 1e-9);
        let y = m.by_name("y").unwrap();
        assert_eq!(y.set.as_deref(), Some("Sub"));
    }

    #[test]
    fn two_dimensions_become_separate_variables_and_say_so() {
        let src = format!("Prod: A, B ~ ~ |\nReg: N, S ~ ~ |\n\
            d[Prod,Reg] = 1 ~ ~ |\nd[A,S] = 10 ~ ~ |\n\
            tot = SUM(d[Prod!,Reg!]) ~ ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        assert!(m.by_name("d[a,n]").is_some() && m.by_name("d[b,s]").is_some());
        assert!((last(&m, "tot") - 13.0).abs() < 1e-9);
        assert!(loss.items.iter().any(|i| i.detail.contains("separate variables")), "{}", loss.summary());
    }

    #[test]
    fn macros_become_functions() {
        let src = format!(":MACRO: DOUBLE(x)\nDOUBLE = twice ~ ~ |\ntwice = x * 2 ~ ~ |\n:END OF MACRO:\ny = DOUBLE(3) + 1 ~ ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].params, ["x"]);
        assert!((last(&m, "y") - 7.0).abs() < 1e-9);
        assert!(loss.items.iter().any(|i| i.construct.contains("macro DOUBLE")));
        // a macro with a stock cannot become a function
        let src = format!(":MACRO: LEAK(x)\nLEAK = INTEG(x, 0) ~ ~ |\n:END OF MACRO:\ny = 1 ~ ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        assert!(m.functions.is_empty());
        assert!(loss.items.iter().any(|i| i.kind == crate::loss::LossKind::Dropped && i.construct.contains("LEAK")), "{}", loss.summary());
    }

    #[test]
    fn data_variables_become_placeholders_and_are_reported() {
        let src = format!("price data ~ $/Unit ~ Observed prices |\nimports = GET XLS DATA('f.xls', 'Sheet1', 'A', 'B2') ~ Unit ~ |\nrate = GET XLS CONSTANTS('f.xls','Sheet1','B2') ~ ~ |\nrevenue = price data * 2 + imports + rate ~ ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        let p = m.by_name("price data").unwrap();
        assert_eq!(p.kind, Kind::Lookup);
        assert!(p.comment.starts_with("[Vensim data:"), "{}", p.comment);
        assert_eq!(m.by_name("rate").unwrap().kind, Kind::Constant);
        assert_eq!(last(&m, "revenue"), 0.0);
        assert_eq!(loss.items.iter().filter(|i| i.construct.starts_with("data variable")).count(), 3, "{}", loss.summary());
    }

    #[test]
    fn vensim_only_functions_are_kept_and_reported() {
        let src = format!("x = FIND ZERO(Time, 5, 0) + 2 ~ ~ |\nok = MAX(1, 2) ~ ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        assert!(m.by_name("x").unwrap().equation.contains("FIND ZERO"));
        assert!(loss.items.iter().any(|i| i.construct == "function FIND ZERO" && i.kind == crate::loss::LossKind::PreservedNotSimulable), "{}", loss.summary());
        assert!(!loss.items.iter().any(|i| i.construct.contains("MAX")));
        assert!(crate::sim::compile(&m).is_err(), "the engine cannot run it, and says so");
    }

    #[test]
    fn lookups_in_every_spelling_and_control_expressions() {
        let src = "a( [(0,0)-(10,10)], (0,0), (5,2), (10,10) ) ~ Dmnl ~ |\nb( (0,1), (10,3) ) :INTERPOLATE: ~ ~ |\nc = WITH LOOKUP(Time, ( [(0,0)-(10,10)], (0,0), (10,10) )) ~ ~ |\nd = a(4) + b(5) ~ ~ |\nTIME STEP = 1/4 ~ Day ~ |\nFINAL TIME = 10 ~ Day ~ |\nINITIAL TIME = 0 ~ Day ~ |\nSAVEPER = 1 ~ Day ~ |\n";
        let (m, loss) = read(src).unwrap();
        assert_eq!(m.settings.time_step, 0.25);
        assert_eq!(m.by_name("a").unwrap().table.len(), 3);
        assert_eq!(m.by_name("b").unwrap().table.len(), 2);
        assert_eq!(m.by_name("c").unwrap().kind, Kind::Lookup);
        // d = a(4) + b(5) = 1.6 + 2 = 3.6
        assert!((last(&m, "d") - 3.6).abs() < 1e-9);
        assert!(loss.items.iter().any(|i| i.construct == "SAVEPER"), "{}", loss.summary());
    }

    #[test]
    fn classic_model_forms_world3_urban_market() {
        // old DYNAMO-style table: x values then y values (World Dynamics, Market Growth)
        // ACTIVE INITIAL breaks a loop among initial values (World3-03); GAME is a pass-through (Urban Dynamics);
        // an alias dimension may come before the dimension it copies (Vensim's project model)
        let src = format!(
            "tab ( 0, 10, 0, 5 ) ~ Dmnl ~ |\nlooked = tab(4) ~ ~ |\n\
             S = INTEG(inflow, s0) ~ ~ |\ns0 = 2 * x ~ ~ |\ninflow = 0 ~ ~ |\n\
             x = ACTIVE INITIAL(S / 2, 7) ~ ~ |\n\
             g = GAME(3 * looked) ~ ~ |\n\
             other <-> base ~ |\nbase : b1, b2 ~ |\nv[other] = 2 ~ ~ |\nw = v[b2] ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        assert!((last(&m, "looked") - 2.0).abs() < 1e-9);
        // S starts at 2 * x(initial) = 14; during the run x = S / 2 = 7
        assert!((last(&m, "s") - 14.0).abs() < 1e-9);
        assert!((last(&m, "x") - 7.0).abs() < 1e-9);
        assert!((last(&m, "g") - 6.0).abs() < 1e-9);
        assert!((last(&m, "w") - 2.0).abs() < 1e-9);
    }

    #[test]
    fn table_constants_delay_n_sample_if_true_quantum() {
        // array constants as a table, last subscript fastest; DELAY N with its own initial value and a named order;
        // SAMPLE IF TRUE holds its value between true conditions; QUANTUM cuts toward zero
        let src = format!(
            "A : a1, a2 ~ |\nB : b1, b2, b3 ~ |\nm[A, B] = 1, 2, 3; 4, 5, 6; ~ ~ |\nm21 = m[a2, b1] ~ ~ |\n\
             n = 4 ~ ~ |\nout = DELAY N(0, 4, 10, n) ~ ~ |\n\
             held = SAMPLE IF TRUE(Time <= 3, Time, -1) ~ ~ |\nq = QUANTUM(-7.5, 2) ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        assert!((last(&m, "m21") - 4.0).abs() < 1e-12);
        // with 4 stages of 1 time unit and no input, the outflow is still 10 for the first step
        let c = crate::sim::compile(&m).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        assert!((r.get("out").unwrap()[1] - 10.0).abs() < 1e-9);
        assert!((last(&m, "held") - 3.0).abs() < 1e-12);
        assert!((last(&m, "q") + 6.0).abs() < 1e-12);
    }

    #[test]
    fn subranges_mappings_and_dimensions_as_values() {
        // a stock defined in parts over subranges, read through a positional mapping (Vensim's AGE model);
        // a dimension or item name used as a value is its place; :NOT: covers the whole comparison;
        // a dimension may list another dimension among its items
        let src = format!(
            "age : infant, child, old ~ |\nolder : child, old ~ |\nprev : infant, child -> older ~ |\n\
             aging[age] = 1 ~ ~ |\npop[infant] = INTEG(-aging[infant], 10) ~ ~ |\n\
             pop[older] = INTEG(aging[prev] - aging[older], 10) ~ ~ |\n\
             pos[age] = age ~ ~ |\nis2[age] = IF THEN ELSE(:NOT: age = child, 0, 1) ~ ~ |\n\
             big : x, age ~ |\nn = ELMCOUNT(big) ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        // each step: infant loses 1; child gains 1 from infant and loses 1; old gains 1 from child and loses 1
        assert!((last(&m, "pop[infant]") - 0.0).abs() < 1e-9);
        assert!((last(&m, "pop[child]") - 10.0).abs() < 1e-9);
        assert!((last(&m, "pos[old]") - 3.0).abs() < 1e-12);
        assert!((last(&m, "is2[child]") - 1.0).abs() < 1e-12 && last(&m, "is2[old]").abs() < 1e-12);
        assert!((last(&m, "n") - 4.0).abs() < 1e-12);
    }

    #[test]
    fn vensim_functions_lookups_vectors_and_matrices() {
        // lookup modes, GAMMA LN, NPV, FORECAST, ZIDZ's threshold, VECTOR SORT ORDER / RANK, INVERT MATRIX,
        // ALLOCATE BY PRIORITY (each checked against Vensim's own output in benchmark B2)
        let src = format!(
            "tab( (0,0), (10,100) ) ~ ~ |\nfw = LOOKUP FORWARD(tab, 5) ~ ~ |\nbw = LOOKUP BACKWARD(tab, 5) ~ ~ |\n\
             inv = LOOKUP INVERT(tab, 50) ~ ~ |\nex = LOOKUP EXTRAPOLATE(tab, 20) ~ ~ |\ng = GAMMA LN(5) ~ ~ |\n\
             z = ZIDZ(1, 1e-9) ~ ~ |\n\
             s : s1, s2, s3 ~ |\nx[s] = 3, 1, 2 ~ ~ |\no[s] = VECTOR SORT ORDER(x[s], 1) ~ ~ |\nr[s] = VECTOR RANK(x[s], 1) ~ ~ |\n\
             c : c1, c2 ~ |\nm[c, c] = 4, 7; 2, 6; ~ ~ |\nmi[c, c] = INVERT MATRIX(m[c, c], 2) ~ ~ |\n\
             req[s] = 5, 5, 5 ~ ~ |\npri[s] = 3, 2, 1 ~ ~ |\nshare[s] = ALLOCATE BY PRIORITY(req[s], pri[s], 3, 0.5, 7) ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        assert!((last(&m, "fw") - 100.0).abs() < 1e-12 && last(&m, "bw").abs() < 1e-12);
        assert!((last(&m, "inv") - 5.0).abs() < 1e-12 && (last(&m, "ex") - 200.0).abs() < 1e-12);
        assert!((last(&m, "g") - 24f64.ln()).abs() < 1e-12);
        assert!(last(&m, "z").abs() < 1e-12);
        // sorted up: s2 (1), s3 (2), s1 (3) → order 1, 2, 0; ranks 3, 1, 2
        assert!((last(&m, "o[s1]") - 1.0).abs() < 1e-12 && last(&m, "o[s3]").abs() < 1e-12);
        assert!((last(&m, "r[s1]") - 3.0).abs() < 1e-12);
        // inverse of [[4, 7], [2, 6]] is [[0.6, -0.7], [-0.2, 0.4]]
        assert!((last(&m, "mi[c1,c2]") + 0.7).abs() < 1e-12 && (last(&m, "mi[c2,c2]") - 0.4).abs() < 1e-12);
        // 7 for priorities 3, 2, 1 at width 0.5: the first two in full, then 0
        assert!((last(&m, "share[s1]") - 5.0).abs() < 1e-9 && (last(&m, "share[s2]") - 2.0).abs() < 1e-9 && last(&m, "share[s3]").abs() < 1e-9);
    }

    #[test]
    fn help_documented_values_for_lookup_slope_area_dates_and_derivatives() {
        // the values and forms the Vensim help gives for these functions (the help's printed LOOKUP SLOPE table
        // does not match its own examples: they are consistent with the points (0,0), (1,1), (2,1.5))
        let src = format!(
            "L( (0,0),(1,1),(2,1.5) ) ~ ~ |\nsl = LOOKUP SLOPE(L, 1, 0) ~ ~ |\nso = LOOKUP SLOPE(L, 5, 0) ~ ~ |\n\
             A( (0,1),(1,1),(2,2) ) ~ ~ |\nar = LOOKUP AREA(A, 0.5, 2.5) ~ ~ |\n\
             d = EXCELDATESERIAL(2008, 1, 1) ~ ~ |\ny = EXCELDATESERIAL YEAR(39448) ~ ~ |\ndy = EXCELDATESERIAL DAY OF YEAR(39488) ~ ~ |\n\
             x = Time * Time ~ ~ |\ndx = DERIVN(x, 1) ~ ~ |\nddx = DERIVN(x, 2) ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        assert!((last(&m, "sl") - 0.75).abs() < 1e-12 && last(&m, "so").abs() < 1e-12);
        assert!((last(&m, "ar") - 3.0).abs() < 1e-12);
        assert_eq!(last(&m, "d"), 39448.0);
        assert_eq!(last(&m, "y"), 2008.0);
        assert_eq!(last(&m, "dy"), 41.0);
        // x = Time^2 with a step of 1: first difference 2t − 1, second difference 2
        assert!((last(&m, "dx") - 19.0).abs() < 1e-9 && (last(&m, "ddx") - 2.0).abs() < 1e-9);
    }

    #[test]
    fn delay_with_named_options_equals_the_functions_it_replaces() {
        // DELAY(x, time, order=, init=, kind=, exact=) is the same as the Vensim function for each combination
        let pairs = [
            ("DELAY(x, 4)", "DELAY1(x, 4)"),
            ("DELAY(x, 4, 3)", "DELAY3(x, 4)"),
            ("DELAY(x, 4, order=5)", "DELAYMTR(x, 4, 5)"),
            ("DELAY(x, 4, init=2)", "DELAY1I(x, 4, 2)"),
            ("DELAY(x, 4, kind=information)", "SMOOTH(x, 4)"),
            ("DELAY(x, 4, order=3, kind=information, init=1)", "SMOOTH3I(x, 4, 1)"),
            ("DELAY(x, 4, exact=True)", "DELAY FIXED(x, 4, x)"),
        ];
        let mut src = String::from("x = 5 + STEP(10, 3) ~ ~ |\n");
        for (k, (a, b)) in pairs.iter().enumerate() {
            src.push_str(&format!("a{k} = {a} ~ ~ |\nb{k} = {b} ~ ~ |\n"));
        }
        src.push_str(CONTROL);
        let (m, _) = read(&src).unwrap();
        let c = crate::sim::compile(&m).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        for k in 0..pairs.len() {
            let (a, b) = (r.get(&format!("a{k}")).unwrap(), r.get(&format!("b{k}")).unwrap());
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12), "{} differs from {}", pairs[k].0, pairs[k].1);
        }
        // a misspelt option says what the options are
        let e = crate::expr::parse("DELAY(x, 4, ordr=3)").unwrap_err();
        assert!(e.contains("did you mean 'order'") && e.contains("Options"), "{e}");
        assert!(crate::expr::parse("DELAY(x, 4, 3, exact=True)").is_err());
    }

    #[test]
    fn allocate_with_named_options_equals_vensims_allocation_functions() {
        // ALLOCATE(requests, supply, by=, overlap=, curve=, spread=) shares a short supply exactly as Vensim's
        // ALLOCATE BY PRIORITY and ALLOCATE AVAILABLE do
        let src = format!(
            "region : r1, r2, r3 ~ |\nptype : pt, pp, pw, pe ~ |\ndemand[region] = 3, 2, 4 ~ ~ |\npri[region] = 1, 2, 3 ~ ~ |\nsupply = Time * 0.4 ~ ~ |\n\
             v1[region] = ALLOCATE BY PRIORITY(demand[region!], pri[region!], 3, 0.5, supply) ~ ~ |\nw1[region] = ALLOCATE(demand, supply, by=pri, overlap=0.5) ~ ~ |\n\
             v2[region] = ALLOCATE BY PRIORITY(demand[region!], pri[region!], 3, 0.000000001, supply) ~ ~ |\nw2[region] = ALLOCATE(demand, supply, by=pri) ~ ~ |\n\
             prof[region, pt] = 3 ~ ~ |\nprof[region, pp] = 1, 2, 3 ~ ~ |\nprof[region, pw] = 1 ~ ~ |\nprof[region, pe] = 0 ~ ~ |\n\
             v3[region] = ALLOCATE AVAILABLE(demand[region!], prof[region, pt], supply) ~ ~ |\nw3[region] = ALLOCATE(demand, supply, by=pri, curve=normal, spread=1) ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        let c = crate::sim::compile(&m).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        for k in 1..=3 {
            for it in ["r1", "r2", "r3"] {
                let (a, b) = (r.get(&format!("v{k}[{it}]")).unwrap(), r.get(&format!("w{k}[{it}]")).unwrap());
                assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12), "pair {k} item {it}");
            }
        }
        // strict priority: at supply 4 the highest priority (r3, asks 4) takes it all
        assert!((r.get("w2[r3]").unwrap()[10] - 4.0).abs() < 1e-9 && r.get("w2[r1]").unwrap()[10].abs() < 1e-9);
    }

    #[test]
    fn value_equals_npv_and_npve() {
        let src = format!(
            "cash = -100 * PULSE(2, 1) + STEP(3, 5) ~ ~ |\na0 = VALUE(cash, 0.01) ~ ~ |\nb0 = NPV(cash, 0.01, 0, 1) ~ ~ |\n\
             a1 = VALUE(cash, 0.02, init=5, factor=2, timing=period_end) ~ ~ |\nb1 = NPVE(cash, 0.02, 5, 2) ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        for (a, b) in [("a0", "b0"), ("a1", "b1")] {
            assert!((last(&m, a) - last(&m, b)).abs() < 1e-12, "{a} differs from {b}");
        }
        assert!(crate::expr::parse("VALUE(cash, 0.01, timing=weekly)").is_err());
    }

    /// Run one of the book's example files and return its series.
    fn book_example(text: &str) -> crate::sim::Run {
        let (m, _) = read(text).unwrap();
        let c = crate::sim::compile(&m).unwrap_or_else(|p| panic!("{:?}", p.iter().map(|x| x.message.clone()).collect::<Vec<_>>()));
        crate::sim::simulate(&c, &m, "x").unwrap()
    }

    #[test]
    fn the_books_examples_run_and_give_the_numbers_the_text_quotes() {
        // book/examples/ch12-water-shortage.mdl: supply 50 at week 1, 80 from week 8
        let r = book_example(include_str!("../../book/examples/ch12-water-shortage.mdl"));
        let at = |name: &str, t: usize| r.get(name).unwrap()[t];
        for (name, want) in [("equal share[north]", 20.0), ("equal share[valley]", 12.5), ("equal share[coast]", 17.5), ("by priority[north]", 0.0), ("by priority[valley]", 25.0), ("by priority[coast]", 25.0)] {
            assert!((at(name, 1) - want).abs() < 1e-9, "{name} at week 1");
        }
        assert!((at("softened priority[north]", 1) - 8.1).abs() < 0.05 && (at("softened priority[valley]", 1) - 22.0).abs() < 0.05);
        assert!((at("by priority[north]", 10) - 20.0).abs() < 1e-9 && (at("by priority[coast]", 10) - 35.0).abs() < 1e-9);
        // book/examples/ch14-investment.mdl: present value of a 400 spend and 12 a month from month 6, at 1% a month
        let r = book_example(include_str!("../../book/examples/ch14-investment.mdl"));
        let pv = r.get("present value").unwrap();
        assert!((pv[0] + 400.0).abs() < 0.05 && (pv[30] + 148.5).abs() < 0.05 && (pv[60] - 81.2).abs() < 0.05, "{:?}", [pv[0], pv[30], pv[60]]);
        // book/examples/ch15-beliefs.mdl: demand ramps 2 a week from week 10 to 40; the manager notices with a lag of 6
        let r = book_example(include_str!("../../book/examples/ch15-beliefs.mdl"));
        let at15 = |n: &str, week: f64| r.get(n).unwrap()[(week / 0.25) as usize];
        assert!((at15("demand", 20.0) - 120.0).abs() < 1e-9 && (at15("perceived demand", 20.0) - 110.2).abs() < 0.05 && (at15("expected demand in 8 weeks", 20.0) - 134.2).abs() < 0.05);
        assert!((at15("perceived demand", 40.0) - 148.1).abs() < 0.05);
        // book/examples/ch17-price-effect.mdl: one curve read five ways
        let r = book_example(include_str!("../../book/examples/ch17-price-effect.mdl"));
        for (n, want) in [("demand effect", 0.84), ("price for a target", 1.375), ("steepness", -0.8), ("steps", 0.6), ("beyond the data", 0.2)] {
            assert!((r.get(n).unwrap()[0] - want).abs() < 1e-9, "{n}");
        }
        // book/examples/ch13-ageing-population.mdl: fourteen five-year classes, read at the years the classes move
        let r = book_example(include_str!("../../book/examples/ch13-ageing-population.mdl"));
        assert!((r.get("total").unwrap()[0] - 828.0).abs() < 1e-9);
        let old = r.get("population[a65plus]").unwrap();
        assert!((old[5] - 81.0).abs() < 0.05 && (old[20] - 149.4).abs() < 0.05 && (old[50] - 236.5).abs() < 0.05);
        assert!((r.get("old share").unwrap()[20] - 0.157).abs() < 0.0005);
        // book/examples/ch11-emergency-room.mdl: 12 arrive an hour for 12 hours against a capacity of 10
        let r = book_example(include_str!("../../book/examples/ch11-emergency-room.mdl"));
        let at4 = |n: &str, hour: f64| r.get(n).unwrap()[(hour / 0.25) as usize];
        assert!((at4("waiting", 18.0) - 24.0).abs() < 1e-9 && at4("waiting", 30.0).abs() < 1e-9);
        assert!((at4("longest wait", 18.0) - 2.0).abs() < 1e-9 && (at4("average wait", 18.0) - 1.12).abs() < 0.01);
        assert!((at4("waiting over 2 hours", 18.0) - 3.0).abs() < 1e-9);
        // book/examples/ch10-shipment-pipeline.mdl: orders step from 100 to 160 at week 10, delay 6, step 0.25
        let r = book_example(include_str!("../../book/examples/ch10-shipment-pipeline.mdl"));
        let first = |name: &str, level: f64| r.get(name).unwrap().iter().position(|v| *v >= level).map(|i| i as f64 * 0.25);
        assert_eq!(first("pipeline arrival", 130.0), Some(16.0));
        assert_eq!(first("bunched arrival", 154.0), Some(20.25));
        assert_eq!(first("smooth arrival", 154.0), Some(23.75));
        assert!((r.get("pipeline arrival").unwrap().last().unwrap() - 160.0).abs() < 1e-9);
    }

    #[test]
    fn queue_and_wait_follow_first_in_first_out() {
        // 10 arrive every step; nobody is served before step 5, then 6 a step. Worked by hand:
        // at step 6, 54 wait: 4 who arrived at step 0 (age 6), then 10 each aged 5, 4, 3, 2 and 1.
        let src = format!(
            "arr = 10 ~ ~ |\nsrv = IF THEN ELSE(Time >= 5, 6, 0) ~ ~ |\nq = QUEUE(arr, srv) ~ ~ |\navg = WAIT(q) ~ ~ |\nold = WAIT(q, measure=oldest) ~ ~ |\n\
             cnt = WAIT(q, measure=count, older_than=4) ~ ~ |\ntop = WAIT(q, among=14) ~ ~ |\n\
             v = QUEUE FIFO(arr, srv, 0, 0, 20) ~ ~ |{CONTROL}"
        );
        let (m, loss) = read(&src).unwrap();
        let c = crate::sim::compile(&m).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        let at = |n: &str, t: usize| r.get(n).unwrap()[t];
        assert!((at("q", 6) - 54.0).abs() < 1e-9 && (at("old", 6) - 6.0).abs() < 1e-9);
        assert!((at("avg", 6) - 174.0 / 54.0).abs() < 1e-9, "{}", at("avg", 6));
        assert!((at("cnt", 6) - 24.0).abs() < 1e-9);
        assert!((at("top", 6) - (24.0 + 50.0) / 14.0).abs() < 1e-9);
        // Vensim's five-argument QUEUE FIFO is read, and says what it dropped
        assert!((at("v", 6) - 54.0).abs() < 1e-9);
        assert!(loss.items.iter().any(|i| i.construct == "QUEUE FIFO"), "{}", loss.summary());
        // the options are checked
        assert!(crate::expr::parse("WAIT(q, measure=count, among=3)").is_err());
    }

    #[test]
    fn age_moves_whole_classes_up_every_period() {
        // 30, 20, 10 people in three classes; the classes move up every 2 years (at times 2, 4, 6)
        let src = format!(
            "age : young, middle, old ~ |\nstart[age] = 30, 20, 10 ~ ~ |\npop[age] = INTEG(net[age], start[age]) ~ ~ |\nnet[age] = AGE(pop, every=2) ~ ~ |\n\
             pop2[age] = INTEG(net2[age], start[age]) ~ ~ |\nnet2[age] = AGE(pop2, every=2, open_ended=False) ~ ~ |\n\
             pop3[age] = INTEG(net3[age], start[age]) ~ ~ |\nnet3[age] = AGE(pop3, every=2, entrants=5) ~ ~ |{CONTROL}"
        );
        let src = src.replace("FINAL TIME = 10", "FINAL TIME = 6");
        let (m, _) = read(&src).unwrap();
        let c = crate::sim::compile(&m).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        let at = |n: &str, t: usize| r.get(n).unwrap()[t];
        // the oldest class keeps its people: 60 in total all the time; the classes move at times 2 and 4
        assert_eq!([at("pop[young]", 1), at("pop[middle]", 1), at("pop[old]", 1)], [30.0, 20.0, 10.0]);
        assert_eq!([at("pop[young]", 2), at("pop[middle]", 2), at("pop[old]", 2)], [0.0, 30.0, 30.0]);
        assert_eq!([at("pop[young]", 4), at("pop[middle]", 4), at("pop[old]", 4)], [0.0, 0.0, 60.0]);
        // open_ended=False: the oldest leave the model at each move
        assert_eq!([at("pop2[young]", 2), at("pop2[middle]", 2), at("pop2[old]", 2)], [0.0, 30.0, 20.0]);
        assert_eq!(at("pop2[old]", 6), 0.0);
        // entrants=5 a year: the youngest class is refilled with 2 years' worth (10) at each move
        assert_eq!([at("pop3[young]", 2), at("pop3[middle]", 2), at("pop3[old]", 2)], [10.0, 30.0, 30.0]);
        // a misspelt option is refused when the set is expanded, naming the options
        let bad = format!("age : a, b ~ |\npop[age] = 1 ~ ~ |\nnet[age] = AGE(pop, evry=2) ~ ~ |{CONTROL}");
        let (m, _) = read(&bad).unwrap();
        let e = crate::sim::compile(&m).err().map(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join(" ")).unwrap_or_default();
        assert!(e.contains("did you mean 'every'"), "{e}");
    }

    #[test]
    fn expect_look_value_and_leak_options() {
        let src = format!(
            "x = 10 + RAMP(1, 5, 40) ~ ~ |\ne1 = EXPECT(x, 4) ~ ~ |\nf1 = SMOOTH(x, 4) ~ ~ |\ne2 = EXPECT(x, 4, measure=trend) ~ ~ |\nf2 = TREND(x, 4, 0) ~ ~ |\n\
             e3 = EXPECT(x, 4, ahead=6) ~ ~ |\nf3 = x * (1 + TREND(x, 4, 0) * 6) ~ ~ |\n\
             curve( (0,0), (10,100), (20,150) ) ~ ~ |\ndown( (1,10), (2,6), (3,5) ) ~ ~ |\n\
             l1 = LOOK(curve, 5) ~ ~ |\nl2 = LOOK(curve, 120, mode=inverse) ~ ~ |\nl3 = LOOK(curve, 15, mode=slope) ~ ~ |\nl4 = LOOK(curve, 15, mode=area, start=10) ~ ~ |\nl5 = LOOK(down, 8, mode=inverse) ~ ~ |\n\
             cash = -100 * PULSE(0, 1) + 110 * PULSE(1, 1) ~ ~ |\nirr = VALUE(cash, measure=irr) ~ ~ |\nflow = 100 ~ ~ |\nleaky = DELAY(flow, 4, exact=True, leak=0.1) ~ ~ |{CONTROL}"
        );
        let (m, _) = read(&src).unwrap();
        let c = crate::sim::compile(&m).unwrap();
        let r = crate::sim::simulate(&c, &m, "x").unwrap();
        for (a, b) in [("e1", "f1"), ("e2", "f2"), ("e3", "f3")] {
            let (u, v) = (r.get(a).unwrap(), r.get(b).unwrap());
            assert!(u.iter().zip(v).all(|(p, q)| (p - q).abs() < 1e-12), "{a} differs from {b}");
        }
        let at = |n: &str, t: usize| r.get(n).unwrap()[t];
        assert!((at("l1", 0) - 50.0).abs() < 1e-9 && (at("l2", 0) - 14.0).abs() < 1e-9 && (at("l3", 0) - 5.0).abs() < 1e-9 && (at("l4", 0) - 562.5).abs() < 1e-9);
        // a falling curve read the other way: 8 lies halfway between the points (1, 10) and (2, 6)... at x = 1.5
        assert!((at("l5", 0) - 1.5).abs() < 1e-9, "{}", at("l5", 0));
        // 100 spent now, 110 back after one period: a rate of return of 10%; not available before the money comes back
        assert!((at("irr", 1) - 0.1).abs() < 1e-9 && at("irr", 0) < -1e30);
        // a conveyor that loses 10% a step takes 4 steps: 100 enters, 65.61 arrives
        assert!((at("leaky", 4) - 65.61).abs() < 1e-9);
        assert!(crate::expr::expand_verbs("EXPECT(x)").is_err() && crate::expr::expand_verbs("LOOK(c, 1, mode=wavy)").is_err() && crate::expr::expand_verbs("DELAY(x, 4, leak=0.1)").is_err());
    }

    #[test]
    fn quoted_names_and_comments_in_braces_survive() {
        let src = format!("\"rate-of-change\" = 3 {{ not part of it }} ~ 1/Day ~ |\nz = \"rate-of-change\" * 2 ~ ~ |{CONTROL}");
        let (m, _) = read(&src).unwrap();
        assert_eq!(m.by_name("rate-of-change").unwrap().kind, Kind::Constant);
        assert!((last(&m, "z") - 6.0).abs() < 1e-9);
    }

    #[test]
    fn views_are_merged_with_a_note_and_arrows_are_per_view() {
        let src = format!("a = 1 ~ ~ |\nb = a * 2 ~ ~ |\nc = b + a ~ ~ |{CONTROL}\n\\\\\\---/// Sketch information\nV300  Do not put anything below this section - it will be ignored\n*Main\n10,1,a,100,100,40,20,8,3,0,0,0,0,0,0\n10,2,b,200,100,40,20,8,3,0,0,0,0,0,0\n1,3,1,2,0,0,0,0,0,64,0,-1--1--1,,1|(0,0)|\n*Second\n10,1,c,300,300,40,20,8,3,0,0,0,0,0,0\n10,2,b,200,100,40,20,8,3,0,0,0,0,0,0\n1,3,2,1,0,0,0,0,0,64,0,-1--1--1,,1|(0,0)|\n///---\\\\\\\n:L<%^E!@\n1:Current.vdf\n");
        let (m, loss) = read(&src).unwrap();
        assert_eq!(m.by_name("a").unwrap().pos, [100.0, 100.0]);
        assert_eq!(m.by_name("c").unwrap().pos, [300.0, 300.0]);
        // the arrow b -> c in the second view uses that view's ids
        let (b, c) = (m.by_name("b").unwrap().id, m.by_name("c").unwrap().id);
        assert!(m.links.iter().any(|l| l.from == b && l.to == c));
        assert!(loss.items.iter().any(|i| i.construct == "views"), "{}", loss.summary());
        assert!(loss.items.iter().any(|i| i.construct == "run settings block"));
    }

    #[test]
    fn a_plain_model_reports_no_loss() {
        let src = format!("Population = INTEG(births, 100) ~ People ~ |\nbirths = Population * 0.03 ~ People/Day ~ |{CONTROL}");
        let (_, loss) = read(&src).unwrap();
        assert!(loss.is_lossless(), "{}", loss.summary());
    }

    /// A model as Vensim itself saves it (the teacup of the standard tutorials).
    const TEACUP: &str = "{UTF-8}\nCharacteristic Time=\n\t10\n\t~\tMinutes\n\t~\t\t|\n\nHeat Loss to Room=\n\t(Teacup Temperature - Room Temperature) / Characteristic Time\n\t~\tDegrees F/Minute\n\t~\tThis is the rate at which heat flows from the cup into the room. We can \\\n\t\tignore it at this point.\n\t|\n\nRoom Temperature=\n\t70\n\t~\tDegrees F\n\t~\t\t|\n\nTeacup Temperature= INTEG (\n\t-Heat Loss to Room,\n\t\t180)\n\t~\tDegrees F\n\t~\t\t|\n\n********************************************************\n\t.Control\n********************************************************~\n\t\tSimulation Control Parameters\n\t|\n\nFINAL TIME  = 30\n\t~\tMinute\n\t~\tThe final time for the simulation.\n\t|\n\nINITIAL TIME  = 0\n\t~\tMinute\n\t~\tThe initial time for the simulation.\n\t|\n\nSAVEPER  = \n        TIME STEP\n\t~\tMinute [0,?]\n\t~\tThe frequency with which output is stored.\n\t|\n\nTIME STEP  = 0.125\n\t~\tMinute [0,?]\n\t~\tThe time step for the simulation.\n\t|\n\n\\\\\\---/// Sketch information - do not modify anything except names\nV300  Do not put anything below this section - it will be ignored\n*View 1\n$192-192-192,0,Times New Roman|12||0-0-0|0-0-0|0-0-255|-1--1--1|-1--1--1|72,72,100,0\n10,1,Teacup Temperature,574,298,40,20,3,3,0,0,0,0,0,0\n12,2,48,846,300,10,8,0,3,0,0,0,0,0,0\n1,3,5,2,4,0,0,22,0,192,0,-1--1--1,,1|(791,300)|\n1,4,5,1,100,0,0,22,0,192,0,-1--1--1,,1|(653,300)|\n11,5,48,730,300,6,8,34,3,0,0,1,0,0,0\n10,6,Heat Loss to Room,730,328,49,8,40,3,0,0,-1,0,0,0\n10,7,Room Temperature,730,190,40,20,8,3,0,0,0,0,0,0\n10,8,Characteristic Time,860,200,40,20,8,3,0,0,0,0,0,0\n1,9,7,5,0,0,0,0,0,64,0,-1--1--1,,1|(730,240)|\n1,10,8,5,0,0,0,0,0,64,0,-1--1--1,,1|(800,250)|\n1,11,1,5,1,0,0,0,0,64,0,-1--1--1,,1|(650,330)|\n///---\\\\\\\n:L<%^E!@\n9:Current\n15:0,0,0,0,0,0\n19:100,0\n27:2,\n34:0,\n4:Time\n5:Teacup Temperature\n35:Date\n36:YYYY-MM-DD\n37:2000\n38:1\n39:1\n40:2\n41:0\n42:1\n24:0\n25:0\n26:0\n";

    #[test]
    fn a_model_as_vensim_saves_it() {
        let (m, loss) = read(TEACUP).unwrap();
        let stock = m.by_name("teacup temperature").unwrap();
        let flow = m.by_name("heat loss to room").unwrap();
        assert_eq!(stock.kind, Kind::Stock);
        assert_eq!((flow.kind, flow.from, flow.to), (Kind::Flow, Some(stock.id), None));
        assert!(flow.comment.starts_with("This is the rate at which heat flows"), "{}", flow.comment);
        assert_eq!(m.settings.time_units, "Minute");
        assert_eq!(m.by_name("characteristic time").unwrap().units, "Minutes");
        assert_eq!(stock.pos, [574.0, 298.0]);
        // arrows: the valve (5) stands for the flow, the pipes are not information arrows
        let from = |n: &str| m.by_name(n).unwrap().id;
        assert!(m.links.iter().any(|l| l.from == from("room temperature") && l.to == flow.id));
        assert!(m.links.iter().any(|l| l.from == from("characteristic time") && l.to == flow.id));
        assert!(m.links.iter().any(|l| l.from == stock.id && l.to == flow.id), "the flow reads the stock");
        assert_eq!(m.links.len(), 3, "{:?}", m.links.iter().map(|l| (l.from, l.to)).collect::<Vec<_>>());
        assert!((last(&m, "teacup temperature") - 75.367).abs() < 0.02);
        // what was not read is said
        assert!(loss.items.iter().any(|i| i.construct == "run settings block"), "{}", loss.summary());
        assert!(!loss.items.iter().any(|i| i.kind == crate::loss::LossKind::Dropped), "{}", loss.summary());
    }

    #[test]
    fn numbered_ranges_quoted_items_and_per_item_stock_starts() {
        let src = format!("Unit: (U1-U3) ~ ~ |\nSite: \"North - A\", \"South, B\" ~ ~ |\n\
            level[Unit] = INTEG(1, start[Unit]) ~ ~ |\nstart[U1] = 10 ~ ~ |\nstart[U2] = 20 ~ ~ |\nstart[U3] = 30 ~ ~ |\n\
            flag = IF THEN ELSE(Time > 3 :AND: Time < 8, 1, 0) + IF THEN ELSE(:NOT: (Time > 1) :OR: Time = 9, 100, 0) ~ ~ |\n\
            site cap[Site] = 5 ~ ~ |\nsite total = SUM(site cap[Site!]) ~ ~ |{CONTROL}");
        let (m, _) = read(&src).unwrap();
        assert_eq!(m.sets.iter().find(|s| s.name == "Unit").unwrap().items, ["U1", "U2", "U3"]);
        assert_eq!(m.sets.iter().find(|s| s.name == "Site").unwrap().items, ["North - A", "South, B"]);
        // 10 + 10 steps of 1 = 20; per-item starts differ
        assert!((last(&m, "level[U1]") - 20.0).abs() < 1e-9);
        assert!((last(&m, "level[U3]") - 40.0).abs() < 1e-9);
        // flag at the end (Time 10): both IFs are 0
        assert_eq!(last(&m, "flag"), 0.0);
        assert!((last(&m, "site total") - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_mapping_that_does_not_fit_is_dropped_and_said() {
        let src = format!("Region: R1, R2 ~ ~ |\nTank: T1, T2, T3 -> (Region: R1, R2) ~ ~ |\nx[Tank] = 1 ~ ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        assert!(m.sets.iter().find(|s| s.name == "Tank").unwrap().parent.is_none());
        assert!(loss.items.iter().any(|i| i.construct.starts_with("mapping Tank") && i.kind == crate::loss::LossKind::Dropped), "{}", loss.summary());
    }

    #[test]
    fn a_positional_mapping_and_an_alias_are_approximated() {
        let src = format!("Region: R1, R2 ~ ~ |\nTank: T1, T2 -> Region ~ ~ |\nAlias <-> Region ~ ~ |\nx[Tank] = 1 ~ ~ |{CONTROL}");
        let (m, loss) = read(&src).unwrap();
        let t = m.sets.iter().find(|s| s.name == "Tank").unwrap();
        assert_eq!((t.parent.as_deref(), t.parent_of.clone()), (Some("Region"), vec!["R1".to_string(), "R2".to_string()]));
        assert!(m.sets.iter().any(|s| s.name == "Alias" && s.items == ["R1", "R2"]));
        assert!(loss.items.iter().any(|i| i.construct.contains("Alias")), "{}", loss.summary());
    }
}
