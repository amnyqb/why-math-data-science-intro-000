//! XMILE 1.0 (OASIS) reader and writer.
//!
//! XMILE is a bridge for reading Stella (and other XMILE) files; it is not the format Dynamist
//! works in. The reader is the main part. The writer is kept because it round-trips the starter
//! model and the small models in the tests to within 1e-9, but it is best-effort: anything it
//! cannot express is listed in the report (and in `wiki/Format-Mapping-XMILE.md`).
//!
//! `read` turns an XMILE file into a [`Model`]; `write` turns a [`Model`] into XMILE text. Both
//! return a [`LossReport`] that lists everything that could not be carried over exactly, so a
//! conversion never bends or drops anything silently. The exact correspondence of constructs and
//! functions is written down in `wiki/Format-Mapping-XMILE.md`.
//!
//! Design in short:
//! - A tiny XML parser/writer lives in this file (no dependency).
//! - Equations are translated token by token through a small syntax tree [`X`]: Dynamist text →
//!   tree → XMILE text, and the reverse. Functions that XMILE lacks are rewritten into standard
//!   XMILE expressions (so any XMILE tool can run them); functions that cannot be rewritten are
//!   kept as written and reported.
//! - Sets become XMILE dimensions when they are plain (no parent sets, no `TOTAL(x, group)`);
//!   otherwise the model is written flattened (one variable per item) and the report says so.
//! - What XMILE has no place for (levers, scenarios, checks, roll-up display) is written into an
//!   XML comment (`<!--dynamist:meta …-->`) that other tools ignore and `read` restores.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::expr::{self, Tok};
use crate::loss::LossReport;
use crate::model::{Group, ItemSet, ItemValue, Kind, Method, Model, Rollup, Sheet, SheetKind, Var};

// ===================================================================================== XML

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kid {
    Node(Node),
    Text(String),
    Comment(String),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub kids: Vec<Kid>,
}

impl Node {
    /// The tag name without any namespace prefix.
    pub fn local(&self) -> &str {
        self.name.rsplit(':').next().unwrap_or(&self.name)
    }
    pub fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, _)| n == k || n.rsplit(':').next() == Some(k)).map(|(_, v)| v.as_str())
    }
    pub fn children(&self) -> impl Iterator<Item = &Node> {
        self.kids.iter().filter_map(|k| if let Kid::Node(n) = k { Some(n) } else { None })
    }
    pub fn child(&self, local: &str) -> Option<&Node> {
        self.children().find(|n| n.local() == local)
    }
    pub fn all(&self, local: &str) -> Vec<&Node> {
        self.children().filter(|n| n.local() == local).collect()
    }
    pub fn text(&self) -> String {
        let mut s = String::new();
        for k in &self.kids {
            if let Kid::Text(t) = k {
                s.push_str(t);
            }
        }
        s
    }
    pub fn comments(&self) -> Vec<&str> {
        self.kids.iter().filter_map(|k| if let Kid::Comment(c) = k { Some(c.as_str()) } else { None }).collect()
    }
}

struct XmlParser {
    c: Vec<char>,
    i: usize,
}

impl XmlParser {
    fn err<T>(&self, msg: &str) -> Result<T, String> {
        let line = self.c[..self.i.min(self.c.len())].iter().filter(|c| **c == '\n').count() + 1;
        Err(format!("XML error on line {line}: {msg}"))
    }
    fn starts(&self, s: &str) -> bool {
        let n = s.chars().count();
        self.i + n <= self.c.len() && self.c[self.i..self.i + n].iter().copied().eq(s.chars())
    }
    fn skip_ws(&mut self) {
        while self.i < self.c.len() && self.c[self.i].is_whitespace() {
            self.i += 1;
        }
    }
    /// Skip up to and including `end`.
    fn skip_to(&mut self, end: &str) -> Result<String, String> {
        let start = self.i;
        while self.i < self.c.len() {
            if self.starts(end) {
                let s: String = self.c[start..self.i].iter().collect();
                self.i += end.chars().count();
                return Ok(s);
            }
            self.i += 1;
        }
        self.err(&format!("missing {end}"))
    }
    fn name(&mut self) -> Result<String, String> {
        let start = self.i;
        while self.i < self.c.len() && !self.c[self.i].is_whitespace() && !matches!(self.c[self.i], '/' | '>' | '=' | '<' | '"' | '\'') {
            self.i += 1;
        }
        if start == self.i {
            return self.err("a name was expected");
        }
        Ok(self.c[start..self.i].iter().collect())
    }
    fn document(&mut self) -> Result<Node, String> {
        if self.c.first() == Some(&'\u{feff}') {
            self.i = 1;
        }
        loop {
            self.skip_ws();
            if self.starts("<?") {
                self.skip_to("?>")?;
            } else if self.starts("<!--") {
                self.i += 4;
                self.skip_to("-->")?;
            } else if self.starts("<!") {
                self.skip_to(">")?;
            } else {
                break;
            }
        }
        if !self.starts("<") {
            return self.err("the file does not start with an element");
        }
        self.element()
    }
    fn element(&mut self) -> Result<Node, String> {
        self.i += 1; // <
        let name = self.name()?;
        let mut node = Node { name, ..Default::default() };
        loop {
            self.skip_ws();
            if self.starts("/>") {
                self.i += 2;
                return Ok(node);
            }
            if self.starts(">") {
                self.i += 1;
                break;
            }
            if self.i >= self.c.len() {
                return self.err("a tag was never closed");
            }
            let an = self.name()?;
            self.skip_ws();
            if !self.starts("=") {
                return self.err(&format!("attribute {an} has no value"));
            }
            self.i += 1;
            self.skip_ws();
            let q = match self.c.get(self.i) {
                Some(&q) if q == '"' || q == '\'' => q,
                _ => return self.err("an attribute value must be quoted"),
            };
            self.i += 1;
            let start = self.i;
            while self.i < self.c.len() && self.c[self.i] != q {
                self.i += 1;
            }
            if self.i >= self.c.len() {
                return self.err("an attribute value was never closed");
            }
            let raw: String = self.c[start..self.i].iter().collect();
            self.i += 1;
            node.attrs.push((an, unescape(&raw)));
        }
        loop {
            if self.i >= self.c.len() {
                return self.err(&format!("<{}> was never closed", node.name));
            }
            if self.starts("</") {
                self.i += 2;
                let end = self.name()?;
                self.skip_ws();
                if !self.starts(">") {
                    return self.err("a closing tag must end with >");
                }
                self.i += 1;
                if end != node.name {
                    return self.err(&format!("<{}> is closed by </{}>", node.name, end));
                }
                return Ok(node);
            }
            if self.starts("<!--") {
                self.i += 4;
                let c = self.skip_to("-->")?;
                node.kids.push(Kid::Comment(c));
            } else if self.starts("<![CDATA[") {
                self.i += 9;
                let t = self.skip_to("]]>")?;
                node.kids.push(Kid::Text(t));
            } else if self.starts("<?") {
                self.skip_to("?>")?;
            } else if self.starts("<") {
                let n = self.element()?;
                node.kids.push(Kid::Node(n));
            } else {
                let start = self.i;
                while self.i < self.c.len() && self.c[self.i] != '<' {
                    self.i += 1;
                }
                let raw: String = self.c[start..self.i].iter().collect();
                node.kids.push(Kid::Text(unescape(&raw)));
            }
        }
    }
}

fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        let Some(end) = rest.find(';').filter(|e| *e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let rep = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32),
            _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match rep {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn esc_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn esc_attr(s: &str) -> String {
    esc_text(s).replace('"', "&quot;").replace('\n', "&#10;").replace('\r', "&#13;").replace('\t', "&#9;")
}

/// A small pretty-printing XML writer.
struct Out {
    s: String,
    depth: usize,
}

impl Out {
    fn new() -> Self {
        Out { s: String::new(), depth: 0 }
    }
    fn pad(&mut self) {
        for _ in 0..self.depth {
            self.s.push_str("  ");
        }
    }
    fn attrs(attrs: &[(&str, String)]) -> String {
        attrs.iter().map(|(k, v)| format!(" {k}=\"{}\"", esc_attr(v))).collect()
    }
    fn open(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.pad();
        self.s.push_str(&format!("<{name}{}>\n", Self::attrs(attrs)));
        self.depth += 1;
    }
    fn close(&mut self, name: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.pad();
        self.s.push_str(&format!("</{name}>\n"));
    }
    fn leaf(&mut self, name: &str, attrs: &[(&str, String)], text: &str) {
        self.pad();
        self.s.push_str(&format!("<{name}{}>{}</{name}>\n", Self::attrs(attrs), esc_text(text)));
    }
    fn empty(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.pad();
        self.s.push_str(&format!("<{name}{}/>\n", Self::attrs(attrs)));
    }
    fn comment(&mut self, text: &str) {
        self.pad();
        self.s.push_str(&format!("<!--{text}-->\n"));
    }
}

pub(crate) fn parse_xml(text: &str) -> Result<Node, String> {
    XmlParser { c: text.chars().collect(), i: 0 }.document()
}

// ================================================================================ names, numbers

/// A number as text that reads back exactly (shortest round-trip form).
fn fmt_f64(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let a = v.abs();
    if a >= 1e15 || a < 1e-6 {
        format!("{v:e}")
    } else {
        format!("{v}")
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || (c as u32) > 127
}

/// Words that XMILE reserves: a variable called one of these is quoted and reported.
const RESERVED: &[&str] = &[
    "AND", "OR", "NOT", "IF", "THEN", "ELSE", "MOD", "ABS", "ARCCOS", "ARCSIN", "ARCTAN", "COS", "EXP", "INF", "INT", "LN", "LOG10", "MAX", "MIN", "PI", "SIN", "SQRT", "TAN",
    "EXPRND", "LOGNORMAL", "NORMAL", "POISSON", "RANDOM", "DELAY", "DELAY1", "DELAY3", "DELAYN", "FORCST", "SMTH1", "SMTH3", "SMTHN", "TREND", "PULSE", "RAMP", "STEP", "DT", "STARTTIME",
    "STOPTIME", "TIME", "INIT", "PREVIOUS", "SELF", "SUM", "MEAN", "SIZE", "STDDEV", "MIN", "MAX", "RANK", "PROD",
];

fn is_reserved(word: &str) -> bool {
    RESERVED.iter().any(|r| r.eq_ignore_ascii_case(word))
}

/// Whitespace and underscores are the same thing in XMILE names: collapse them to single spaces.
fn words(display: &str) -> String {
    display.split(|c: char| c == '_' || c.is_whitespace()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// The name as it appears in an `name="…"` attribute.
fn name_attr(display: &str) -> String {
    words(display)
}

/// The name as it appears inside an XMILE equation: underscores for spaces, quoted if it has to be.
fn xm_ident(display: &str) -> String {
    let w = words(display);
    let joined = w.replace(' ', "_");
    let ok = !joined.is_empty() && joined.chars().all(is_ident_char) && !joined.starts_with(|c: char| c.is_ascii_digit() || c == '$') && !is_reserved(&joined);
    if ok {
        joined
    } else {
        let mut q = String::from("\"");
        for c in w.chars() {
            match c {
                '"' => q.push_str("\\\""),
                '\\' => q.push_str("\\\\"),
                '\n' => q.push_str("\\n"),
                c => q.push(c),
            }
        }
        q.push('"');
        q
    }
}

/// The name as it appears inside a Dynamist equation.
fn fc_ident(display: &str) -> String {
    let t = words(display);
    let ok = !t.is_empty()
        && t.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '$' || c == '\'')
        && t.chars().next().is_some_and(|c| c.is_alphabetic() || c == '$');
    if ok { t } else { format!("\"{}\"", t.replace('"', "'")) }
}

/// A name read from XMILE (attribute or identifier): quotes dropped, underscores become spaces.
fn display_of(raw: &str) -> String {
    let t = raw.trim();
    let t = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') { t[1..t.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\") } else { t.to_string() };
    // the escape \n in a name is a whitespace character
    words(&t.replace("\\n", " "))
}

/// Units text, Dynamist → XMILE unit equation: words joined by underscores, odd names quoted.
fn units_to_xmile(u: &str) -> String {
    let mut out = String::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, out: &mut String| {
        let t = words(cur);
        if !t.is_empty() {
            let j = t.replace(' ', "_");
            if j.chars().all(is_ident_char) && !j.starts_with(|c: char| c.is_ascii_digit() && j.len() > 1 && !j.chars().all(|d| d.is_ascii_digit())) {
                out.push_str(&j);
            } else {
                out.push_str(&format!("\"{}\"", t.replace('"', "'")));
            }
        }
        cur.clear();
    };
    for c in u.chars() {
        if matches!(c, '*' | '/' | '^' | '(' | ')') {
            flush(&mut cur, &mut out);
            out.push(c);
        } else {
            cur.push(c);
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// Units text, XMILE → Dynamist: quotes dropped, underscores become spaces.
fn units_from_xmile(u: &str) -> String {
    let mut out = String::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in u.trim().chars() {
        if c == '"' {
            quoted = !quoted;
        } else if !quoted && matches!(c, '*' | '/' | '^' | '(' | ')') {
            out.push_str(&display_of(&cur));
            cur.clear();
            out.push(c);
        } else {
            cur.push(c);
        }
    }
    out.push_str(&display_of(&cur));
    out
}

/// Documentation text: plain XMILE text writes non-printables as `\n`, `\t` and `\\`.
fn doc_to_xmile(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\t', "\\t").replace('\r', "")
}

fn doc_from_xmile(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.trim().chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

// =============================================================================== syntax tree

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

#[derive(Clone, Debug, PartialEq)]
enum X {
    Num(f64),
    /// A variable, by display name.
    Id(String),
    /// Text written exactly as it is (TIME, DT, PI …).
    Raw(String),
    /// A function: built-ins by canonical upper-case name, anything else by display name.
    Call(String, Vec<X>),
    /// `name[subscript, …]`
    Sub(String, Vec<X>),
    /// The `*` wildcard inside a subscript.
    Star,
    Neg(Box<X>),
    Not(Box<X>),
    Bin(Op, Box<X>, Box<X>),
    If(Box<X>, Box<X>, Box<X>),
}

fn num(v: f64) -> X {
    X::Num(v)
}
fn raw(s: &str) -> X {
    X::Raw(s.into())
}
fn bin(op: Op, a: X, b: X) -> X {
    X::Bin(op, Box::new(a), Box::new(b))
}
fn neg(a: X) -> X {
    X::Neg(Box::new(a))
}
fn call(name: &str, args: Vec<X>) -> X {
    X::Call(name.into(), args)
}
fn if_(c: X, a: X, b: X) -> X {
    X::If(Box::new(c), Box::new(a), Box::new(b))
}

/// A literal number (with an optional sign).
fn lit(x: &X) -> Option<f64> {
    match x {
        X::Num(v) => Some(*v),
        X::Neg(a) => lit(a).map(|v| -v),
        _ => None,
    }
}

// ------------------------------------------------------------------------ Dynamist text → tree

struct FcParser {
    t: Vec<Tok>,
    i: usize,
}

impl FcParser {
    fn eat(&mut self, ops: &[&str]) -> Option<&'static str> {
        if let Some(Tok::Op(o)) = self.t.get(self.i) {
            if ops.contains(o) {
                self.i += 1;
                return Some(*o);
            }
        }
        None
    }
    fn expect(&mut self, t: Tok, what: &str) -> Result<(), String> {
        if self.t.get(self.i) == Some(&t) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected {what}"))
        }
    }
    fn expr(&mut self) -> Result<X, String> {
        let mut l = self.and()?;
        while self.eat(&["||"]).is_some() {
            l = bin(Op::Or, l, self.and()?);
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<X, String> {
        let mut l = self.cmp()?;
        while self.eat(&["&&"]).is_some() {
            l = bin(Op::And, l, self.cmp()?);
        }
        Ok(l)
    }
    fn cmp(&mut self) -> Result<X, String> {
        let mut l = self.add()?;
        while let Some(o) = self.eat(&["<", "<=", ">", ">=", "=", "<>"]) {
            let op = match o {
                "<" => Op::Lt,
                "<=" => Op::Le,
                ">" => Op::Gt,
                ">=" => Op::Ge,
                "=" => Op::Eq,
                _ => Op::Ne,
            };
            l = bin(op, l, self.add()?);
        }
        Ok(l)
    }
    fn add(&mut self) -> Result<X, String> {
        let mut l = self.mul()?;
        while let Some(o) = self.eat(&["+", "-"]) {
            l = bin(if o == "+" { Op::Add } else { Op::Sub }, l, self.mul()?);
        }
        Ok(l)
    }
    fn mul(&mut self) -> Result<X, String> {
        let mut l = self.unary()?;
        while let Some(o) = self.eat(&["*", "/"]) {
            l = bin(if o == "*" { Op::Mul } else { Op::Div }, l, self.unary()?);
        }
        Ok(l)
    }
    fn unary(&mut self) -> Result<X, String> {
        if self.eat(&["-"]).is_some() {
            return Ok(neg(self.unary()?));
        }
        if self.eat(&["+"]).is_some() {
            return self.unary();
        }
        if self.eat(&["!"]).is_some() {
            return Ok(X::Not(Box::new(self.unary()?)));
        }
        let base = self.primary()?;
        if self.eat(&["^"]).is_some() {
            return Ok(bin(Op::Pow, base, self.unary()?));
        }
        Ok(base)
    }
    fn signed(&mut self) -> Result<f64, String> {
        let n = self.eat(&["-"]).is_some();
        match self.t.get(self.i).cloned() {
            Some(Tok::Num(v)) => {
                self.i += 1;
                Ok(if n { -v } else { v })
            }
            _ => Err("a number was expected in a lookup table".into()),
        }
    }
    fn primary(&mut self) -> Result<X, String> {
        let Some(t) = self.t.get(self.i).cloned() else { return Err("the formula ends too early".into()) };
        self.i += 1;
        match t {
            Tok::Num(n) => Ok(num(n)),
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen, "')'")?;
                Ok(e)
            }
            Tok::Ident(name) => {
                if self.t.get(self.i) == Some(&Tok::LParen) {
                    self.i += 1;
                    let canon = expr::normalize(&name).to_uppercase();
                    if canon == "WITH LOOKUP" {
                        let input = self.expr()?;
                        self.expect(Tok::Comma, "','")?;
                        self.expect(Tok::LParen, "'('")?;
                        if self.t.get(self.i) == Some(&Tok::LBracket) {
                            while let Some(t) = self.t.get(self.i) {
                                self.i += 1;
                                if *t == Tok::RBracket {
                                    break;
                                }
                            }
                            if self.t.get(self.i) == Some(&Tok::Comma) {
                                self.i += 1;
                            }
                        }
                        let mut args = vec![input];
                        loop {
                            self.expect(Tok::LParen, "'('")?;
                            let x = self.signed()?;
                            self.expect(Tok::Comma, "','")?;
                            let y = self.signed()?;
                            self.expect(Tok::RParen, "')'")?;
                            args.push(num(x));
                            args.push(num(y));
                            if self.t.get(self.i) == Some(&Tok::Comma) {
                                self.i += 1;
                            } else {
                                break;
                            }
                        }
                        self.expect(Tok::RParen, "')'")?;
                        self.expect(Tok::RParen, "')'")?;
                        return Ok(X::Call("WITH LOOKUP".into(), args));
                    }
                    let mut args = Vec::new();
                    if self.t.get(self.i) != Some(&Tok::RParen) {
                        args.push(self.expr()?);
                        while self.t.get(self.i) == Some(&Tok::Comma) {
                            self.i += 1;
                            args.push(self.expr()?);
                        }
                    }
                    self.expect(Tok::RParen, "')'")?;
                    let builtin = expr::function_info(&canon).is_some();
                    Ok(X::Call(if builtin { canon } else { words(&name) }, args))
                } else if self.t.get(self.i) == Some(&Tok::LBracket) {
                    self.i += 1;
                    let mut subs = vec![self.expr()?];
                    while self.t.get(self.i) == Some(&Tok::Comma) {
                        self.i += 1;
                        subs.push(self.expr()?);
                    }
                    self.expect(Tok::RBracket, "']'")?;
                    Ok(X::Sub(words(&name), subs))
                } else {
                    Ok(X::Id(words(&name)))
                }
            }
            _ => Err("unexpected symbol in the formula".into()),
        }
    }
}

fn fc_parse(src: &str) -> Result<X, String> {
    let toks: Vec<Tok> = expr::tokenize(src)?.into_iter().map(|(t, _)| t).collect();
    if toks.is_empty() {
        return Err("the formula is empty".into());
    }
    let mut p = FcParser { t: toks, i: 0 };
    let e = p.expr()?;
    if p.i < p.t.len() {
        return Err("unexpected text after the end of the formula".into());
    }
    Ok(e)
}

// ----------------------------------------------------------------------------- XMILE text → tree

#[derive(Clone, Debug, PartialEq)]
enum XT {
    Num(f64),
    Id(String),
    Quoted(String),
    Op(&'static str),
    LP,
    RP,
    LB,
    RB,
    Comma,
}

fn xm_tokens(src: &str) -> Result<Vec<XT>, String> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '{' {
            while i < b.len() && b[i] != '}' {
                i += 1;
            }
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let s = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == '.') {
                i += 1;
            }
            if i < b.len() && (b[i] == 'e' || b[i] == 'E') {
                let mut j = i + 1;
                if j < b.len() && (b[j] == '+' || b[j] == '-') {
                    j += 1;
                }
                if j < b.len() && b[j].is_ascii_digit() {
                    i = j;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let t: String = b[s..i].iter().collect();
            out.push(XT::Num(t.parse::<f64>().map_err(|_| format!("bad number '{t}'"))?));
        } else if c == '"' {
            i += 1;
            let mut t = String::new();
            while i < b.len() && b[i] != '"' {
                if b[i] == '\\' && i + 1 < b.len() {
                    i += 1;
                    t.push(match b[i] {
                        'n' => ' ',
                        c => c,
                    });
                } else {
                    t.push(b[i]);
                }
                i += 1;
            }
            if i >= b.len() {
                return Err("a quoted name is not closed".into());
            }
            i += 1;
            out.push(XT::Quoted(t));
        } else if c.is_alphabetic() || c == '_' || c == '$' || c == '.' || (c as u32) > 127 {
            let s = i;
            while i < b.len() && (is_ident_char(b[i]) || b[i] == '.') {
                i += 1;
            }
            out.push(XT::Id(b[s..i].iter().collect()));
        } else {
            let two: String = b[i..(i + 2).min(b.len())].iter().collect();
            match two.as_str() {
                "<=" => {
                    out.push(XT::Op("<="));
                    i += 2;
                    continue;
                }
                ">=" => {
                    out.push(XT::Op(">="));
                    i += 2;
                    continue;
                }
                "<>" => {
                    out.push(XT::Op("<>"));
                    i += 2;
                    continue;
                }
                _ => {}
            }
            out.push(match c {
                '+' => XT::Op("+"),
                '-' | '–' => XT::Op("-"),
                '*' => XT::Op("*"),
                '/' => XT::Op("/"),
                '^' => XT::Op("^"),
                '<' => XT::Op("<"),
                '>' => XT::Op(">"),
                '=' => XT::Op("="),
                '(' => XT::LP,
                ')' => XT::RP,
                '[' => XT::LB,
                ']' => XT::RB,
                ',' => XT::Comma,
                other => return Err(format!("unexpected character '{other}'")),
            });
            i += 1;
        }
    }
    Ok(out)
}

struct XmParser {
    t: Vec<XT>,
    i: usize,
}

impl XmParser {
    fn kw(&self, w: &str) -> bool {
        matches!(self.t.get(self.i), Some(XT::Id(s)) if s.eq_ignore_ascii_case(w))
    }
    fn eat_kw(&mut self, w: &str) -> bool {
        if self.kw(w) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn eat(&mut self, ops: &[&str]) -> Option<&'static str> {
        if let Some(XT::Op(o)) = self.t.get(self.i) {
            if ops.contains(o) {
                self.i += 1;
                return Some(*o);
            }
        }
        None
    }
    fn expr(&mut self) -> Result<X, String> {
        if self.eat_kw("IF") {
            let c = self.expr()?;
            if !self.eat_kw("THEN") {
                return Err("IF needs THEN".into());
            }
            let a = self.expr()?;
            if !self.eat_kw("ELSE") {
                return Err("IF needs ELSE".into());
            }
            let b = self.expr()?;
            return Ok(if_(c, a, b));
        }
        let mut l = self.and()?;
        while self.eat_kw("OR") {
            l = bin(Op::Or, l, self.and()?);
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<X, String> {
        let mut l = self.eq()?;
        while self.eat_kw("AND") {
            l = bin(Op::And, l, self.eq()?);
        }
        Ok(l)
    }
    fn eq(&mut self) -> Result<X, String> {
        let mut l = self.rel()?;
        while let Some(o) = self.eat(&["=", "<>"]) {
            l = bin(if o == "=" { Op::Eq } else { Op::Ne }, l, self.rel()?);
        }
        Ok(l)
    }
    fn rel(&mut self) -> Result<X, String> {
        let mut l = self.add()?;
        while let Some(o) = self.eat(&["<", "<=", ">", ">="]) {
            let op = match o {
                "<" => Op::Lt,
                "<=" => Op::Le,
                ">" => Op::Gt,
                _ => Op::Ge,
            };
            l = bin(op, l, self.add()?);
        }
        Ok(l)
    }
    fn add(&mut self) -> Result<X, String> {
        let mut l = self.mul()?;
        while let Some(o) = self.eat(&["+", "-"]) {
            l = bin(if o == "+" { Op::Add } else { Op::Sub }, l, self.mul()?);
        }
        Ok(l)
    }
    fn mul(&mut self) -> Result<X, String> {
        let mut l = self.unary()?;
        loop {
            if let Some(o) = self.eat(&["*", "/"]) {
                l = bin(if o == "*" { Op::Mul } else { Op::Div }, l, self.unary()?);
            } else if self.eat_kw("MOD") {
                l = bin(Op::Mod, l, self.unary()?);
            } else {
                return Ok(l);
            }
        }
    }
    fn unary(&mut self) -> Result<X, String> {
        if self.eat(&["-"]).is_some() {
            return Ok(neg(self.unary()?));
        }
        if self.eat(&["+"]).is_some() {
            return self.unary();
        }
        if self.eat_kw("NOT") {
            return Ok(X::Not(Box::new(self.unary()?)));
        }
        let base = self.primary()?;
        if self.eat(&["^"]).is_some() {
            return Ok(bin(Op::Pow, base, self.unary()?));
        }
        Ok(base)
    }
    fn primary(&mut self) -> Result<X, String> {
        if self.kw("IF") {
            return self.expr();
        }
        let Some(t) = self.t.get(self.i).cloned() else { return Err("the equation ends too early".into()) };
        self.i += 1;
        match t {
            XT::Num(n) => Ok(num(n)),
            XT::LP => {
                let e = self.expr()?;
                if self.t.get(self.i) != Some(&XT::RP) {
                    return Err("a ')' is missing".into());
                }
                self.i += 1;
                Ok(e)
            }
            XT::Op("*") => Ok(X::Star),
            XT::Id(name) | XT::Quoted(name) => {
                let quoted = matches!(t_of(&self.t, self.i - 1), Some(XT::Quoted(_)));
                if self.t.get(self.i) == Some(&XT::LP) {
                    self.i += 1;
                    let mut args = Vec::new();
                    if self.t.get(self.i) != Some(&XT::RP) {
                        args.push(self.expr()?);
                        while self.t.get(self.i) == Some(&XT::Comma) {
                            self.i += 1;
                            args.push(self.expr()?);
                        }
                    }
                    if self.t.get(self.i) != Some(&XT::RP) {
                        return Err("a ')' is missing".into());
                    }
                    self.i += 1;
                    Ok(X::Call(display_of(&name), args))
                } else if self.t.get(self.i) == Some(&XT::LB) {
                    self.i += 1;
                    let mut subs = vec![self.expr()?];
                    while self.t.get(self.i) == Some(&XT::Comma) {
                        self.i += 1;
                        subs.push(self.expr()?);
                    }
                    if self.t.get(self.i) != Some(&XT::RB) {
                        return Err("a ']' is missing".into());
                    }
                    self.i += 1;
                    Ok(X::Sub(display_of(&name), subs))
                } else {
                    // a quoted name is one name, dots and all; a bare one with dots is a qualified name
                    Ok(X::Id(if quoted { display_of(&name) } else { name_keep_dots(&name) }))
                }
            }
            _ => Err("unexpected symbol in the equation".into()),
        }
    }
}

fn t_of(t: &[XT], i: usize) -> Option<&XT> {
    t.get(i)
}

/// Underscores become spaces but dots (qualified names) stay.
fn name_keep_dots(raw: &str) -> String {
    raw.split('.').map(display_of).collect::<Vec<_>>().join(".")
}

fn xm_parse(src: &str) -> Result<X, String> {
    let toks = xm_tokens(src)?;
    if toks.is_empty() {
        return Err("the equation is empty".into());
    }
    let mut p = XmParser { t: toks, i: 0 };
    let e = p.expr()?;
    if p.i < p.t.len() {
        return Err("unexpected text after the end of the equation".into());
    }
    Ok(e)
}

// ----------------------------------------------------------------------------- tree → text

#[derive(Clone, Copy, PartialEq)]
enum Dialect {
    Fc,
    Xm,
}

fn prec(x: &X) -> u8 {
    match x {
        X::If(..) => 0,
        X::Bin(op, ..) => match op {
            Op::Or => 1,
            Op::And => 2,
            Op::Eq | Op::Ne => 3,
            Op::Lt | Op::Le | Op::Gt | Op::Ge => 4,
            Op::Add | Op::Sub => 5,
            Op::Mul | Op::Div | Op::Mod => 6,
            Op::Pow => 8,
        },
        X::Neg(_) | X::Not(_) => 7,
        X::Num(v) if *v < 0.0 => 7,
        _ => 9,
    }
}

/// Dynamist has one level for all six comparisons; XMILE has two (relational above equality).
fn precd(x: &X, d: Dialect) -> u8 {
    let p = prec(x);
    if d == Dialect::Fc && p == 4 { 3 } else { p }
}

fn emit(x: &X, d: Dialect) -> String {
    emit_min(x, d, 0)
}

fn emit_min(x: &X, d: Dialect, min: u8) -> String {
    let s = emit_raw(x, d);
    let p = if d == Dialect::Fc && matches!(x, X::If(..)) { 9 } else { precd(x, d) };
    if p < min { format!("({s})") } else { s }
}

fn call_name(name: &str, d: Dialect) -> String {
    match d {
        Dialect::Fc => {
            if expr::function_info(&name.to_uppercase()).is_some() { name.to_uppercase() } else { fc_ident(name) }
        }
        Dialect::Xm => {
            if name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') { name.to_string() } else { xm_ident(name) }
        }
    }
}

fn emit_raw(x: &X, d: Dialect) -> String {
    match x {
        X::Num(v) => fmt_f64(*v),
        X::Id(n) => match d {
            Dialect::Fc => fc_ident(n),
            Dialect::Xm => xm_ident(n),
        },
        X::Raw(s) => s.clone(),
        X::Star => "*".into(),
        X::Call(n, args) if d == Dialect::Fc && n == "WITH LOOKUP" && !args.is_empty() => {
            let pts: Vec<String> = args[1..].chunks(2).filter(|p| p.len() == 2).map(|p| format!("({}, {})", emit(&p[0], d), emit(&p[1], d))).collect();
            format!("WITH LOOKUP({}, ({}))", emit(&args[0], d), pts.join(", "))
        }
        X::Call(n, args) => {
            let a: Vec<String> = args.iter().map(|a| emit_min(a, d, 0)).collect();
            format!("{}({})", call_name(n, d), a.join(", "))
        }
        X::Sub(n, subs) => {
            let s: Vec<String> = subs.iter().map(|a| emit_min(a, d, 0)).collect();
            let name = match d {
                Dialect::Fc => fc_ident(n),
                Dialect::Xm => xm_ident(n),
            };
            format!("{name}[{}]", s.join(", "))
        }
        X::Neg(a) => format!("-{}", emit_min(a, d, 8)),
        X::Not(a) => match d {
            Dialect::Fc => format!("!{}", emit_min(a, d, 7)),
            Dialect::Xm => format!("NOT {}", emit_min(a, d, 7)),
        },
        X::Bin(op, l, r) => {
            let p = precd(x, d);
            let cmp = matches!(op, Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge);
            let lmin = if *op == Op::Pow || (d == Dialect::Fc && cmp) { p + 1 } else { p };
            // a power is right-associative: x^y^z is x^(y^z)
            let rmin = if *op == Op::Pow { p } else { p + 1 };
            let sym = match (op, d) {
                (Op::Or, Dialect::Fc) => " || ",
                (Op::Or, Dialect::Xm) => " OR ",
                (Op::And, Dialect::Fc) => " && ",
                (Op::And, Dialect::Xm) => " AND ",
                (Op::Eq, _) => " = ",
                (Op::Ne, _) => " <> ",
                (Op::Lt, _) => " < ",
                (Op::Le, _) => " <= ",
                (Op::Gt, _) => " > ",
                (Op::Ge, _) => " >= ",
                (Op::Add, _) => " + ",
                (Op::Sub, _) => " - ",
                (Op::Mul, _) => " * ",
                (Op::Div, _) => " / ",
                (Op::Mod, _) => " MOD ",
                (Op::Pow, _) => "^",
            };
            format!("{}{}{}", emit_min(l, d, lmin), sym, emit_min(r, d, rmin))
        }
        X::If(c, a, b) => match d {
            Dialect::Fc => format!("IF THEN ELSE({}, {}, {})", emit(c, d), emit(a, d), emit(b, d)),
            Dialect::Xm => format!("IF {} THEN {} ELSE {}", emit(c, d), emit(a, d), emit(b, d)),
        },
    }
}

// ======================================================================= loss-report helper

/// Add a report line unless the same one is already there.
fn note(rep: &mut LossReport, kind: crate::loss::LossKind, construct: &str, detail: &str) {
    if !rep.items.iter().any(|i| i.kind == kind && i.construct == construct && i.detail == detail) {
        rep.add(kind, construct, detail);
    }
}

// ============================================================== Dynamist equation → XMILE

/// A graphical function that stands in for GRAPH, GRAPHSTEP or WITH LOOKUP in an equation.
struct Helper {
    name: String,
    discrete: bool,
    pts: Vec<[f64; 2]>,
}

struct WCtx<'a> {
    rep: &'a mut LossReport,
    helpers: Vec<Helper>,
    /// auxiliaries that stand in for an expression inside INIT(…), which takes a variable only
    aux_helpers: Vec<(String, X)>,
    t0: f64,
    dt: f64,
}

fn to_xm(x: &X, c: &mut WCtx) -> X {
    match x {
        X::Num(_) | X::Raw(_) | X::Star => x.clone(),
        X::Id(n) => match expr::normalize(n).as_str() {
            "time" => raw("TIME"),
            "initial time" => raw("STARTTIME"),
            "final time" => raw("STOPTIME"),
            "time step" | "dt" => raw("DT"),
            _ => x.clone(),
        },
        // subscripts are item names, not variables: leave them alone
        X::Sub(n, subs) => X::Sub(n.clone(), subs.clone()),
        X::Neg(a) => neg(to_xm(a, c)),
        X::Not(a) => X::Not(Box::new(to_xm(a, c))),
        X::Bin(op, l, r) => bin(*op, to_xm(l, c), to_xm(r, c)),
        X::If(a, b, d) => if_(to_xm(a, c), to_xm(b, c), to_xm(d, c)),
        X::Call(name, args) => {
            let a: Vec<X> = args.iter().map(|a| to_xm(a, c)).collect();
            call_to_xm(name, a, c)
        }
    }
}

fn nested(name: &str, a: Vec<X>) -> X {
    let mut it = a.into_iter().rev();
    let mut acc = it.next().unwrap_or(num(0.0));
    for x in it {
        acc = call(name, vec![x, acc]);
    }
    acc
}

fn sum_of(a: Vec<X>) -> X {
    let mut it = a.into_iter();
    let mut acc = it.next().unwrap_or(num(0.0));
    for x in it {
        acc = bin(Op::Add, acc, x);
    }
    acc
}

fn call_to_xm(name: &str, a: Vec<X>, c: &mut WCtx) -> X {
    use crate::loss::LossKind::{Approximated, Dropped, Preserved};
    let g = |i: usize| a[i].clone();
    let n = a.len();
    match name {
        "IF THEN ELSE" if n == 3 => if_(g(0), g(1), g(2)),
        "MIN" | "MAX" if n >= 2 => nested(name, a),
        "ABS" | "SQRT" | "EXP" | "LN" | "SIN" | "COS" | "TAN" | "ARCTAN" | "ARCSIN" | "ARCCOS" | "LOG10" => call(name, a),
        // Dynamist INTEGER cuts toward zero; XMILE INT rounds down
        "INTEGER" => if_(bin(Op::Ge, g(0), num(0.0)), call("INT", vec![g(0)]), neg(call("INT", vec![neg(g(0))]))),
        "FLOOR" => call("INT", a),
        "CEILING" => neg(call("INT", vec![neg(g(0))])),
        // Dynamist ROUND rounds halves away from zero
        "ROUND" => if_(bin(Op::Ge, g(0), num(0.0)), call("INT", vec![bin(Op::Add, g(0), num(0.5))]), neg(call("INT", vec![bin(Op::Add, neg(g(0)), num(0.5))]))),
        "MODULO" => {
            if !lit(&a[1]).is_some_and(|v| v > 0.0) {
                note(c.rep, Approximated, "MODULO", "Dynamist MODULO always returns a remainder that is 0 or more; XMILE MOD takes the sign of the divisor. They differ only when the divisor can be negative.");
            }
            bin(Op::Mod, g(0), g(1))
        }
        "POWER" => bin(Op::Pow, g(0), g(1)),
        "LOG" => bin(Op::Div, call("LN", vec![g(0)]), call("LN", vec![g(1)])),
        "LOG2" => bin(Op::Div, call("LN", vec![g(0)]), call("LN", vec![num(2.0)])),
        // as Vensim: the divisor counts as zero below 1e-6 in magnitude
        "XIDZ" => if_(bin(Op::Lt, call("ABS", vec![g(1)]), num(1e-6)), g(2), bin(Op::Div, g(0), g(1))),
        "ZIDZ" => if_(bin(Op::Lt, call("ABS", vec![g(1)]), num(1e-6)), num(0.0), bin(Op::Div, g(0), g(1))),
        "SIGN" => if_(bin(Op::Gt, g(0), num(0.0)), num(1.0), if_(bin(Op::Lt, g(0), num(0.0)), num(-1.0), num(0.0))),
        "SINH" => bin(Op::Div, bin(Op::Sub, call("EXP", vec![g(0)]), call("EXP", vec![neg(g(0))])), num(2.0)),
        "COSH" => bin(Op::Div, bin(Op::Add, call("EXP", vec![g(0)]), call("EXP", vec![neg(g(0))])), num(2.0)),
        "TANH" => bin(
            Op::Div,
            bin(Op::Sub, call("EXP", vec![g(0)]), call("EXP", vec![neg(g(0))])),
            bin(Op::Add, call("EXP", vec![g(0)]), call("EXP", vec![neg(g(0))])),
        ),
        "ARCTAN2" => {
            // atan2(y, x), written with ARCTAN and the four quadrants
            let (y, x) = (g(0), g(1));
            let at = call("ARCTAN", vec![bin(Op::Div, y.clone(), x.clone())]);
            if_(
                bin(Op::Gt, x.clone(), num(0.0)),
                at.clone(),
                if_(
                    bin(Op::Lt, x.clone(), num(0.0)),
                    bin(Op::Add, at, if_(bin(Op::Ge, y.clone(), num(0.0)), raw("PI"), neg(raw("PI")))),
                    if_(bin(Op::Gt, y.clone(), num(0.0)), bin(Op::Div, raw("PI"), num(2.0)), if_(bin(Op::Lt, y, num(0.0)), neg(bin(Op::Div, raw("PI"), num(2.0))), num(0.0))),
                ),
            )
        }
        "CBRT" => if_(
            bin(Op::Ge, g(0), num(0.0)),
            bin(Op::Pow, g(0), bin(Op::Div, num(1.0), num(3.0))),
            neg(bin(Op::Pow, neg(g(0)), bin(Op::Div, num(1.0), num(3.0)))),
        ),
        "HYPOT" => call("SQRT", vec![bin(Op::Add, bin(Op::Pow, g(0), num(2.0)), bin(Op::Pow, g(1), num(2.0)))]),
        "RADIANS" => bin(Op::Div, bin(Op::Mul, g(0), raw("PI")), num(180.0)),
        "DEGREES" => bin(Op::Div, bin(Op::Mul, g(0), num(180.0)), raw("PI")),
        "PI" => raw("PI"),
        "CLAMP" => call("MIN", vec![call("MAX", vec![g(0), call("MIN", vec![g(1), g(2)])]), call("MAX", vec![g(1), g(2)])]),
        "MEAN" => bin(Op::Div, sum_of(a.clone()), num(n as f64)),
        "MEDIAN" if n == 2 => bin(Op::Div, bin(Op::Add, g(0), g(1)), num(2.0)),
        "MEDIAN" if n == 3 => bin(
            Op::Sub,
            bin(Op::Sub, sum_of(a.clone()), nested("MIN", a.clone())),
            nested("MAX", a),
        ),
        // In Stella (and others) GAMMA(...) is a random-number generator, so Dynamist's gamma
        // function is never written under that name.
        "GAMMA" => {
            note(
                c.rep,
                Approximated,
                "function GAMMA",
                "written as EXP(GAMMALN(x)). GAMMALN is an isee function that is not in XMILE 1.0, so other tools may not run it; the name GAMMA is avoided because in Stella it draws random numbers",
            );
            call("EXP", vec![call("GAMMALN", a)])
        }
        "FACTORIAL" => {
            note(c.rep, Preserved, "function FACTORIAL", "FACTORIAL is built in to Stella but is not in XMILE 1.0; it is written unchanged");
            call(name, a)
        }
        "ERF" | "MEDIAN" => {
            note(c.rep, Dropped, &format!("function {name}"), &format!("{name} has no XMILE or Stella equivalent. It is written unchanged so Dynamist can read it back, but other tools will not run the equation."));
            call(name, a)
        }
        "SINWAVE" | "COSWAVE" => {
            let start = if n > 2 { g(2) } else { raw("STARTTIME") };
            let arg = bin(Op::Div, bin(Op::Mul, bin(Op::Mul, num(2.0), raw("PI")), bin(Op::Sub, raw("TIME"), start)), g(1));
            bin(Op::Mul, g(0), call(if name == "SINWAVE" { "SIN" } else { "COS" }, vec![arg]))
        }
        "GRAPH" | "GRAPHSTEP" => {
            let ys: Option<Vec<f64>> = a[3..].iter().map(lit).collect();
            match (lit(&a[1]), lit(&a[2]), ys) {
                (Some(x0), Some(dx), Some(ys)) if ys.len() >= 2 => {
                    let discrete = name == "GRAPHSTEP";
                    let mut pts: Vec<[f64; 2]> = ys.iter().enumerate().map(|(i, y)| [x0 + dx * i as f64, *y]).collect();
                    if discrete {
                        // XMILE's last two points of a step function must be equal
                        let last = *pts.last().unwrap();
                        pts.push([last[0] + dx, last[1]]);
                    }
                    let hname = format!("fc_{}_{}", if discrete { "graphstep" } else { "graph" }, c.helpers.len() + 1);
                    c.helpers.push(Helper { name: hname.clone(), discrete, pts });
                    call(&hname, vec![g(0)])
                }
                _ => {
                    note(c.rep, Dropped, &format!("function {name}"), "its start, step and values must be plain numbers to become an XMILE graphical function; the call is written unchanged and other tools will not run it");
                    call(name, a)
                }
            }
        }
        "WITH LOOKUP" => {
            let pts: Vec<[f64; 2]> = a[1..].chunks(2).filter_map(|p| Some([lit(&p[0])?, lit(p.get(1)?)?])).collect();
            let hname = format!("fc_lookup_{}", c.helpers.len() + 1);
            c.helpers.push(Helper { name: hname.clone(), discrete: false, pts });
            call(&hname, vec![g(0)])
        }
        "DELAYINF" | "DELAYMTR" => {
            let order = match a.get(2) {
                None => Some(1),
                Some(x) => lit(x).filter(|v| v.fract() == 0.0 && (1.0..=20.0).contains(v)).map(|v| v as usize),
            };
            match order {
                Some(k) => {
                    note(
                        c.rep,
                        Approximated,
                        &format!("function {name}"),
                        if name == "DELAYINF" {
                            "written as SMTHN (a chain of smooths, each delay ÷ order). Inside Dynamist this is the same run as the chain it was; it has not been run in another tool"
                        } else {
                            "written as DELAYN (a chain of first-order material delays, each delay ÷ order). Inside Dynamist it matches DELAY3 for order 3; it has not been run in another tool"
                        },
                    );
                    call(if name == "DELAYINF" { "SMTHN" } else { "DELAYN" }, vec![g(0), g(1), num(k as f64)])
                }
                None => {
                    note(c.rep, Dropped, &format!("function {name}"), "its order must be a whole number written as a number; the call is written unchanged");
                    call(name, a)
                }
            }
        }
        "SMOOTH" | "SMOOTHI" => call("SMTH1", a),
        "SMOOTH3" | "SMOOTH3I" => call("SMTH3", a),
        "DELAY1" | "DELAY1I" => call("DELAY1", a),
        "DELAY3" | "DELAY3I" => call("DELAY3", a),
        "DELAY FIXED" => call("DELAY", a),
        "TREND" => call("TREND", a),
        "INITIAL" => match &a[0] {
            X::Id(_) => call("INIT", a),
            _ => {
                // INIT takes the name of a variable: hold the expression in an auxiliary of its own
                let hname = format!("fc_init_{}", c.aux_helpers.len() + 1);
                c.aux_helpers.push((hname.clone(), a[0].clone()));
                call("INIT", vec![X::Id(hname)])
            }
        },
        "STEP" => {
            let on_grid = lit(&a[1]).is_some_and(|s| {
                let k = (s - c.t0) / c.dt;
                (k - k.round()).abs() < 1e-9
            });
            if !on_grid {
                note(c.rep, Approximated, "STEP", "Dynamist moves a step to the nearest time step; XMILE steps at the first time step at or after the start. They agree when the start time falls on the time grid.");
            }
            call("STEP", a)
        }
        // Dynamist PULSE(start, width) is 1 for a while; XMILE PULSE is a one-step spike of size/DT.
        "PULSE" => {
            let t2 = bin(Op::Add, raw("TIME"), bin(Op::Div, raw("DT"), num(2.0)));
            if_(
                bin(Op::And, bin(Op::Ge, t2.clone(), g(0)), bin(Op::Lt, t2, bin(Op::Add, g(0), call("MAX", vec![g(1), raw("DT")])))),
                num(1.0),
                num(0.0),
            )
        }
        "PULSE TRAIN" => {
            let t2 = bin(Op::Add, raw("TIME"), bin(Op::Div, raw("DT"), num(2.0)));
            if_(
                bin(Op::Or, bin(Op::Or, bin(Op::Lt, t2.clone(), g(0)), bin(Op::Gt, raw("TIME"), g(3))), bin(Op::Le, g(2), num(0.0))),
                num(0.0),
                if_(bin(Op::Lt, bin(Op::Mod, bin(Op::Sub, t2, g(0)), g(2)), call("MAX", vec![g(1), raw("DT")])), num(1.0), num(0.0)),
            )
        }
        "RAMP" => if_(
            bin(Op::Gt, raw("TIME"), g(1)),
            bin(Op::Mul, g(0), bin(Op::Sub, call("MIN", vec![raw("TIME"), g(2)]), g(1))),
            num(0.0),
        ),
        "RANDOM UNIFORM" => {
            note(c.rep, Approximated, "RANDOM UNIFORM", "written as RANDOM; random numbers differ between tools, so runs will not match number for number");
            call("RANDOM", a)
        }
        "RANDOM NORMAL" => {
            note(c.rep, Approximated, "RANDOM NORMAL", "written as MAX(min, MIN(max, NORMAL(mean, sd, seed))): the values are clipped to [min, max], whereas Vensim and Stella redraw values outside the limits. Random numbers also differ between tools, so runs will not match number for number");
            call("MAX", vec![g(0), call("MIN", vec![g(1), call("NORMAL", if n > 4 { vec![g(2), g(3), g(4)] } else { vec![g(2), g(3)] })])])
        }
        "TOTAL" | "AVERAGE" => match a.as_slice() {
            [X::Id(v)] => call(if name == "TOTAL" { "SUM" } else { "MEAN" }, vec![X::Sub(v.clone(), vec![X::Star])]),
            _ => {
                note(c.rep, Dropped, &format!("{name}(x, group)"), "adding up part of a set has no XMILE form; the call is written unchanged");
                call(name, a)
            }
        },
        // the person's own functions (written as macros) and lookups called like functions
        _ => call(name, a),
    }
}

// ============================================================== XMILE equation → Dynamist

#[derive(Clone)]
struct HelperDef {
    kind: HelperKind,
    pts: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, PartialEq)]
enum HelperKind {
    Graph,
    GraphStep,
    Lookup,
}

fn helper_kind(name: &str) -> Option<HelperKind> {
    let n = expr::normalize(name);
    if n.starts_with("fc graph step ") || n.starts_with("fc graphstep ") {
        Some(HelperKind::GraphStep)
    } else if n.starts_with("fc graph ") {
        Some(HelperKind::Graph)
    } else if n.starts_with("fc lookup ") {
        Some(HelperKind::Lookup)
    } else {
        None
    }
}

struct RCtx<'a> {
    rep: &'a mut LossReport,
    helpers: &'a HashMap<String, HelperDef>,
    dims: &'a HashSet<String>,
    sets: &'a [ItemSet],
    var_set: &'a HashMap<String, String>,
    /// auxiliaries Dynamist wrote to stand in for an expression inside INIT(…)
    inits: &'a HashMap<String, String>,
    /// Inside a submodel: the flattened prefix its names get ("" at the root).
    prefix: &'a str,
    t0: f64,
    dt: f64,
}

impl RCtx<'_> {
    /// `.Name` is a name at the top level; inside a submodel every other name gets the module's prefix.
    fn qualify(&self, name: &str) -> String {
        if let Some(rest) = name.strip_prefix('.') {
            return rest.to_string();
        }
        if self.prefix.is_empty() { name.to_string() } else { format!("{}.{}", self.prefix, name) }
    }
    fn items_of(&self, var: &str) -> Option<&ItemSet> {
        let s = self.var_set.get(&expr::normalize(var))?;
        self.sets.iter().find(|x| expr::normalize(&x.name) == expr::normalize(s))
    }
}

fn from_xm(x: &X, c: &mut RCtx) -> X {
    use crate::loss::LossKind::{Approximated, Dropped};
    match x {
        X::Num(_) | X::Raw(_) | X::Star => x.clone(),
        X::Id(n) => {
            let norm = expr::normalize(n);
            match norm.as_str() {
                "time" => return raw("TIME"),
                "dt" | "time step" => return raw("DT"),
                "starttime" | "initial time" => return raw("INITIAL TIME"),
                "stoptime" | "final time" => return raw("FINAL TIME"),
                "pi" => return raw("PI()"),
                "inf" => {
                    note(c.rep, Approximated, "INF", "infinity is written as 1e300");
                    return num(1e300);
                }
                _ => {}
            }
            if c.dims.contains(&norm) {
                note(c.rep, Dropped, "array dimension used as a value", &format!("“{n}” is a dimension name used inside an equation; Dynamist has no such construct"));
            }
            X::Id(c.qualify(n))
        }
        X::Sub(name, subs) => {
            let qn = c.qualify(name);
            match subs.as_slice() {
                [X::Id(s)] => {
                    let ns = expr::normalize(s);
                    if c.dims.contains(&ns) {
                        X::Id(qn) // the same element of the variable's own dimension
                    } else {
                        let item = c.items_of(name).and_then(|set| set.items.iter().find(|i| expr::normalize(i) == ns).cloned());
                        X::Sub(qn, vec![X::Id(item.unwrap_or_else(|| s.clone()))])
                    }
                }
                [X::Num(k)] if k.fract() == 0.0 && *k >= 1.0 => {
                    let item = c.items_of(name).and_then(|set| set.items.get(*k as usize - 1).cloned());
                    match item {
                        Some(i) => X::Sub(qn, vec![X::Id(i)]),
                        None => {
                            note(c.rep, Dropped, "subscript", &format!("{name}[{k}] has no such element"));
                            X::Id(qn)
                        }
                    }
                }
                _ => {
                    note(c.rep, Dropped, "multi-dimensional or computed subscript", &format!("a subscript on “{name}” is not a single element; only one-dimensional arrays are supported"));
                    X::Id(qn)
                }
            }
        }
        X::Neg(a) => neg(from_xm(a, c)),
        X::Not(a) => X::Not(Box::new(from_xm(a, c))),
        X::Bin(Op::Mod, l, r) => {
            if !lit(r).is_some_and(|v| v > 0.0) {
                note(c.rep, Approximated, "MOD", "XMILE MOD takes the sign of the divisor; Dynamist MODULO always returns 0 or more. They differ only when the divisor can be negative.");
            }
            call("MODULO", vec![from_xm(l, c), from_xm(r, c)])
        }
        X::Bin(op, l, r) => bin(*op, from_xm(l, c), from_xm(r, c)),
        X::If(a, b, d) => if_(from_xm(a, c), from_xm(b, c), from_xm(d, c)),
        X::Call(name, args) => {
            let canon = expr::normalize(name).to_uppercase();
            // EXP(GAMMALN(x)) is the gamma function
            if canon == "EXP" && args.len() == 1 {
                if let X::Call(inner, ia) = &args[0] {
                    if expr::normalize(inner).to_uppercase() == "GAMMALN" && ia.len() == 1 {
                        return call("GAMMA", vec![from_xm(&ia[0], c)]);
                    }
                }
            }
            // SUM(x[*]) and MEAN(x[*]) add up an array
            if (canon == "SUM" || canon == "MEAN") && args.len() == 1 {
                if let X::Sub(v, subs) = &args[0] {
                    if subs.as_slice() == [X::Star] {
                        return call(if canon == "SUM" { "TOTAL" } else { "AVERAGE" }, vec![X::Id(c.qualify(v))]);
                    }
                }
                if canon == "SUM" {
                    note(c.rep, Dropped, "SUM", "only SUM(array[*]) is supported");
                }
            }
            let a: Vec<X> = args.iter().map(|a| from_xm(a, c)).collect();
            call_from_xm(name, &canon, a, c, &Approximated, &Dropped)
        }
    }
}

fn call_from_xm(name: &str, canon: &str, a: Vec<X>, c: &mut RCtx, approx: &crate::loss::LossKind, dropped: &crate::loss::LossKind) -> X {
    let n = a.len();
    let g = |i: usize| a[i].clone();
    let zero = num(0.0);
    let init_or = |i: usize, default: X| a.get(i).cloned().unwrap_or(default);
    // order of an Nth-order function
    let order = |i: usize| a.get(i).and_then(lit).filter(|v| v.fract() == 0.0 && *v >= 1.0).map(|v| v as usize);
    match (canon, n) {
        ("ABS" | "SQRT" | "EXP" | "LN" | "LOG10" | "SIN" | "COS" | "TAN" | "ARCSIN" | "ARCCOS" | "ARCTAN", _) => call(canon, a),
        ("MIN" | "MAX", 2..) => call(canon, a),
        ("INT", 1) => {
            note(c.rep, *approx, "INT", "XMILE INT rounds down (INT(-8.9) = -9), as FLOOR does. Vensim's INTEGER truncates toward zero and some converters write it as INT, so files from them may differ for negative values");
            call("FLOOR", a)
        }
        ("PI", 0) => raw("PI()"),
        ("IF THEN ELSE", 3) => if_(g(0), g(1), g(2)),
        ("SMTH1", 2) => call("SMOOTH", a),
        ("SMTH1", 3) => call("SMOOTHI", a),
        ("SMTH3", 2) => call("SMOOTH3", a),
        ("SMTH3", 3) => call("SMOOTH3I", a),
        ("DELAY1", 2) => call("DELAY1", a),
        ("DELAY1", 3) => call("DELAY1I", a),
        ("DELAY3", 2) => call("DELAY3", a),
        ("DELAY3", 3) => call("DELAY3I", a),
        ("SMTHN" | "DELAYN", 3 | 4) => {
            let smooth = canon == "SMTHN";
            let k = order(2);
            let has_init = n == 4;
            match (k, has_init) {
                (Some(1), _) => call(if smooth { if has_init { "SMOOTHI" } else { "SMOOTH" } } else if has_init { "DELAY1I" } else { "DELAY1" }, {
                    let mut v = vec![g(0), g(1)];
                    if has_init {
                        v.push(g(3));
                    }
                    v
                }),
                (Some(3), _) => call(if smooth { if has_init { "SMOOTH3I" } else { "SMOOTH3" } } else if has_init { "DELAY3I" } else { "DELAY3" }, {
                    let mut v = vec![g(0), g(1)];
                    if has_init {
                        v.push(g(3));
                    }
                    v
                }),
                (Some(k), _) if k <= 20 => {
                    if has_init {
                        note(c.rep, *approx, canon, "an initial value on an order other than 1 or 3 is ignored: Dynamist starts at the input's value");
                    }
                    call(if smooth { "DELAYINF" } else { "DELAYMTR" }, vec![g(0), g(1), num(k as f64)])
                }
                _ => {
                    note(c.rep, *dropped, canon, "the order must be a whole number from 1 to 20");
                    call(name, a)
                }
            }
        }
        ("DELAY", 2) => call("DELAY FIXED", vec![g(0), g(1), call("INITIAL", vec![g(0)])]),
        ("DELAY", 3) => call("DELAY FIXED", a),
        ("FORCST", 3 | 4) => bin(
            Op::Mul,
            g(0),
            bin(Op::Add, num(1.0), bin(Op::Mul, call("TREND", vec![g(0), g(1), init_or(3, zero)]), g(2))),
        ),
        ("TREND", 2 | 3) => call("TREND", vec![g(0), g(1), init_or(2, zero)]),
        ("INIT", 1) => {
            if let X::Id(n) = &a[0] {
                if let Some(text) = c.inits.get(&expr::normalize(n)).cloned() {
                    return match xm_parse(&text) {
                        Ok(x) => call("INITIAL", vec![from_xm(&x, c)]),
                        Err(_) => call("INITIAL", a),
                    };
                }
            }
            call("INITIAL", a)
        }
        ("GAMMALN", 1) => {
            note(c.rep, *approx, "GAMMALN", "read as LN(ABS(GAMMA(x))); Dynamist has no separate log-gamma function");
            call("LN", vec![call("ABS", vec![call("GAMMA", a)])])
        }
        ("GAMMA", _) => {
            note(c.rep, *dropped, "function GAMMA", "in Stella GAMMA(…) is a random-number generator, not the gamma function; Dynamist cannot run it and the equation will not compile");
            call("GAMMA RANDOM", a)
        }
        ("SAFEDIV", 2 | 3) => {
            note(c.rep, *approx, "SAFEDIV", "read as IF b = 0 THEN fallback ELSE a / b; SAFEDIV also substitutes the fallback for tiny divisors and results that are not finite");
            if_(bin(Op::Eq, g(1), num(0.0)), init_or(2, zero), bin(Op::Div, g(0), g(1)))
        }
        ("NORMAL", 5) => {
            note(c.rep, *approx, "NORMAL", "the limits are applied by clipping (Stella and Vensim redraw), and random numbers differ between tools");
            call("RANDOM NORMAL", vec![g(3), g(4), g(0), g(1), g(2)])
        }
        ("STEP", 2) => {
            let on_grid = lit(&a[1]).is_some_and(|s| {
                let k = (s - c.t0) / c.dt;
                (k - k.round()).abs() < 1e-9
            });
            if !on_grid {
                note(c.rep, *approx, "STEP", "Dynamist moves a step to the nearest time step; XMILE steps at the first time step at or after the start. They agree when the start time falls on the time grid.");
            }
            call("STEP", a)
        }
        ("PULSE", 2 | 3) => {
            let height = bin(Op::Div, g(0), raw("DT"));
            match a.get(2) {
                Some(iv) if lit(iv) != Some(0.0) => bin(Op::Mul, height, call("PULSE TRAIN", vec![g(1), raw("DT"), iv.clone(), num(1e300)])),
                _ => bin(Op::Mul, height, call("PULSE", vec![g(1), raw("DT")])),
            }
        }
        ("RAMP", 2) => call("RAMP", vec![g(0), g(1), raw("FINAL TIME")]),
        ("NORMAL", 2 | 3) => {
            note(c.rep, *approx, "NORMAL", "random numbers differ between tools, so runs will not match number for number");
            call("RANDOM NORMAL", vec![num(-1e300), num(1e300), g(0), g(1), init_or(2, zero)])
        }
        ("RANDOM", 2 | 3) => {
            note(c.rep, *approx, "RANDOM", "random numbers differ between tools, so runs will not match number for number");
            call("RANDOM UNIFORM", vec![g(0), g(1), init_or(2, zero)])
        }
        ("MEAN", 2..) => call("MEAN", a),
        ("EXPRND" | "LOGNORMAL" | "POISSON" | "PREVIOUS" | "SELF" | "SIZE" | "RANK" | "STDDEV" | "PROD" | "SUM", _) => {
            note(c.rep, *dropped, &format!("function {canon}"), "Dynamist does not have it; the call is kept as written and the equation will not compile");
            call(name, a)
        }
        _ => {
            // a graphical function called by name, or one of the person's own functions
            if let Some(h) = c.helpers.get(&expr::normalize(name)) {
                if n == 1 {
                    let pts = &h.pts;
                    return match h.kind {
                        HelperKind::Lookup => {
                            let mut v = vec![g(0)];
                            for p in pts {
                                v.push(num(p[0]));
                                v.push(num(p[1]));
                            }
                            call("WITH LOOKUP", v)
                        }
                        HelperKind::Graph | HelperKind::GraphStep => {
                            let take = if h.kind == HelperKind::GraphStep { pts.len().saturating_sub(1) } else { pts.len() };
                            let dx = if pts.len() > 1 { pts[1][0] - pts[0][0] } else { 1.0 };
                            let mut v = vec![g(0), num(pts[0][0]), num(dx)];
                            v.extend(pts[..take].iter().map(|p| num(p[1])));
                            call(if h.kind == HelperKind::GraphStep { "GRAPHSTEP" } else { "GRAPH" }, v)
                        }
                    };
                }
            }
            call(&c.qualify_fn(name), a)
        }
    }
}

impl RCtx<'_> {
    /// Functions of the file's macros keep their own names (macros are global, not per submodel).
    fn qualify_fn(&self, name: &str) -> String {
        display_of(name)
    }
}

// ====================================================================== what Dynamist alone knows

fn one() -> f64 {
    1.0
}

#[derive(Serialize, Deserialize, Default)]
struct VarMeta {
    #[serde(default)]
    lever: bool,
    #[serde(default = "one")]
    lever_scale: f64,
    #[serde(default)]
    average: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct Meta {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    vars: BTreeMap<String, VarMeta>,
    #[serde(default)]
    scenarios: Vec<crate::model::Scenario>,
    #[serde(default)]
    checks: Vec<crate::checks::Check>,
    #[serde(default)]
    manifest: crate::manifest::Manifest,
}

const META_TAG: &str = "dynamist:meta ";

fn meta_comment(m: &Meta) -> Option<String> {
    let json = serde_json::to_string(m).ok()?;
    // "--" may not appear inside an XML comment; inside a JSON string - is the same character
    Some(format!("{META_TAG}{}", json.replace("--", "-\\u002d")))
}

// =========================================================================== reading XMILE

/// A graphical function: its points, and whether it is a step function.
struct Gf {
    pts: Vec<[f64; 2]>,
    discrete: bool,
    extrapolate: bool,
}

fn numbers(text: &str, sep: Option<&str>) -> Result<Vec<f64>, String> {
    let sep = sep.and_then(|s| s.chars().next()).unwrap_or(',');
    text.split(|c: char| c == sep || c.is_whitespace())
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.trim().parse::<f64>().map_err(|_| format!("“{t}” is not a number")))
        .collect()
}

fn read_gf(n: &Node) -> Result<Gf, String> {
    let ys = numbers(&n.child("ypts").map(|y| y.text()).unwrap_or_default(), n.child("ypts").and_then(|y| y.attr("sep")))?;
    if ys.is_empty() {
        return Err("a graphical function has no y values".into());
    }
    let xs = if let Some(xp) = n.child("xpts") {
        numbers(&xp.text(), xp.attr("sep"))?
    } else if let Some(xs) = n.child("xscale") {
        let (lo, hi) = (xs.attr("min").and_then(|v| v.trim().parse::<f64>().ok()), xs.attr("max").and_then(|v| v.trim().parse::<f64>().ok()));
        match (lo, hi) {
            (Some(lo), Some(hi)) => {
                if ys.len() == 1 {
                    vec![lo]
                } else {
                    (0..ys.len()).map(|i| lo + (hi - lo) * i as f64 / (ys.len() - 1) as f64).collect()
                }
            }
            _ => return Err("a graphical function's x scale needs min and max".into()),
        }
    } else {
        return Err("a graphical function needs xpts or xscale".into());
    };
    if xs.len() != ys.len() {
        return Err(format!("a graphical function has {} x values but {} y values", xs.len(), ys.len()));
    }
    let ty = n.attr("type").unwrap_or("continuous").trim().to_lowercase();
    let mut pts: Vec<[f64; 2]> = xs.into_iter().zip(ys).map(|(x, y)| [x, y]).collect();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    Ok(Gf { pts, discrete: ty == "discrete", extrapolate: ty == "extrapolate" })
}

fn is_even(pts: &[[f64; 2]]) -> Option<(f64, f64)> {
    if pts.len() < 2 {
        return None;
    }
    let dx = pts[1][0] - pts[0][0];
    if dx <= 0.0 {
        return None;
    }
    let ok = pts.windows(2).all(|w| ((w[1][0] - w[0][0]) - dx).abs() <= 1e-9 * dx.abs().max(1.0));
    ok.then_some((pts[0][0], dx))
}

struct Reader {
    m: Model,
    rep: LossReport,
    helpers: HashMap<String, HelperDef>,
    /// helper auxiliaries for INIT(expression): name → XMILE equation text
    inits: HashMap<String, String>,
    dims: HashSet<String>,
    var_set: HashMap<String, String>,
    by_name: HashMap<String, u64>,
    /// (stock id, flow name, is an inflow) to wire once every variable exists
    pending: Vec<(u64, String, bool)>,
    /// (variable that is an input, what it is connected to)
    connects: Vec<(String, String)>,
    submodels: Vec<Node>,
    groups: Vec<(String, Vec<String>)>,
    t0: f64,
    dt: f64,
    /// `<behavior>`: stocks / flows that are non-negative unless they say otherwise
    nn_stock: bool,
    nn_flow: bool,
    /// variables over two or more dimensions, by normalised full name: (display name, dimension names);
    /// each combination becomes its own variable `name[a,b]`
    flat: HashMap<String, (String, Vec<String>)>,
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") }
}

impl Reader {
    fn eq(&mut self, text: &str, prefix: &str, owner: &str) -> String {
        use crate::loss::LossKind::Dropped;
        let flat_text = self.flatten_refs(text, prefix);
        let text = flat_text.as_str();
        // only a {comment}: Stella's placeholder for a module input, whose value comes from the connection
        if text.trim().starts_with('{') && text.trim().ends_with('}') && !text.trim()[1..].contains('{') {
            note(&mut self.rep, Dropped, &format!("equation of {owner}"), &format!("is only a comment (“{}”); 0 stands in until it is connected", text.trim()));
            return "0".into();
        }
        match xm_parse(text) {
            Ok(x) => {
                let mut c = RCtx { rep: &mut self.rep, helpers: &self.helpers, dims: &self.dims, sets: &self.m.sets, var_set: &self.var_set, inits: &self.inits, prefix, t0: self.t0, dt: self.dt };
                let y = from_xm(&x, &mut c);
                emit(&y, Dialect::Fc)
            }
            Err(e) => {
                // keep the text: the model then shows the problem and refuses to run, instead of running on a 0
                // nobody chose (Vensim's XMILE export writes `???` for equations it could not convert)
                let _ = Dropped;
                note(&mut self.rep, crate::loss::LossKind::PreservedNotSimulable, &format!("equation of {owner}"), &format!("could not be read ({e}): “{}”; kept as written, so the model cannot run until it is fixed", text.trim()));
                text.trim().to_string()
            }
        }
    }

    /// `name[a, b]` with `name` a variable over two or more dimensions → the quoted per-combination name
    /// `"name[a,b]"` (items spelled as their set spells them). Subscripts that are not all items are left alone.
    fn flatten_refs(&self, text: &str, prefix: &str) -> String {
        if self.flat.is_empty() {
            return text.to_string();
        }
        let b: Vec<char> = text.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            let start_ok = i == 0 || !(b[i - 1].is_alphanumeric() || b[i - 1] == '_' || b[i - 1] == '.');
            if start_ok && (c.is_alphabetic() || c == '_' || c == '"') {
                // a name, quoted or not
                let mut j = i;
                if c == '"' {
                    j += 1;
                    while j < b.len() && b[j] != '"' {
                        j += 1;
                    }
                    j = (j + 1).min(b.len());
                } else {
                    while j < b.len() && (b[j].is_alphanumeric() || b[j] == '_' || b[j] == '.') {
                        j += 1;
                    }
                }
                let name: String = b[i..j].iter().collect();
                let mut k = j;
                while k < b.len() && b[k] == ' ' {
                    k += 1;
                }
                if k < b.len() && b[k] == '[' {
                    if let Some(close) = (k..b.len()).find(|&x| b[x] == ']') {
                        let inner: String = b[k + 1..close].iter().collect();
                        let key = expr::normalize(&join(prefix, &display_of(&name)));
                        let key = if self.flat.contains_key(&key) { key } else { expr::normalize(&display_of(&name)) };
                        if let Some((disp, dims)) = self.flat.get(&key) {
                            let subs: Vec<String> = inner.split(',').map(|x| display_of(x)).collect();
                            let items: Option<Vec<String>> = subs.iter().zip(dims.iter()).map(|(sub, d)| {
                                self.m.sets.iter().find(|st| expr::normalize(&st.name) == expr::normalize(d)).and_then(|st| st.items.iter().find(|it| expr::normalize(it) == expr::normalize(sub)).cloned())
                            }).collect();
                            if let Some(items) = items.filter(|v| v.len() == dims.len() && subs.len() == dims.len()) {
                                out.push('"');
                                out.push_str(&format!("{disp}[{}]", items.join(",")).replace('"', "\\\""));
                                out.push('"');
                                i = close + 1;
                                continue;
                            }
                        }
                    }
                }
                out.push_str(&name);
                i = j;
                continue;
            }
            out.push(c);
            i += 1;
        }
        out
    }

    /// One variable over two or more dimensions: one variable per combination, named `name[a,b]`, each with
    /// its own equation (an `<element>`, one value of a comma list, or the general equation with the
    /// dimensions fixed to the combination's items).
    fn read_var_flat(&mut self, n: &Node, prefix: &str, display: &str, dims: &[String]) {
        use crate::loss::LossKind::{Approximated, Dropped};
        let mut lists: Vec<Vec<String>> = Vec::new();
        for d in dims {
            match self.m.sets.iter().find(|s| expr::normalize(&s.name) == expr::normalize(d)) {
                Some(s) => lists.push(s.items.clone()),
                None => {
                    note(&mut self.rep, Dropped, &format!("variable {display}"), &format!("uses the dimension “{d}”, which is not defined; the variable was left out"));
                    return;
                }
            }
        }
        let total: usize = lists.iter().map(|l| l.len()).product();
        if total == 0 || total > 5000 {
            note(&mut self.rep, Dropped, &format!("variable {display}"), &format!("{total} combinations; left out"));
            return;
        }
        note(&mut self.rep, Approximated, &format!("variable {display}[{}]", dims.join(",")), &format!("{total} combinations became {total} separate variables named {display}[a,b] (sets hold one dimension)"));
        let general = n.child("eqn").map(|e| e.text().trim().to_string()).filter(|t| !t.is_empty());
        // a comma list of numbers: one value per combination, the last dimension fastest
        let list: Option<Vec<String>> = general.as_ref().and_then(|g| {
            let parts: Vec<String> = g.split(|c| c == ',' || c == ';').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            (parts.len() == total && parts.iter().all(|p| p.parse::<f64>().is_ok())).then_some(parts)
        });
        let elements: Vec<(Vec<String>, &Node)> = n.all("element").into_iter().map(|e| (e.attr("subscript").unwrap_or("").split(',').map(|x| expr::normalize(&display_of(x))).collect(), e)).collect();
        let mut idx = vec![0usize; lists.len()];
        for c in 0..total {
            let combo: Vec<String> = idx.iter().zip(&lists).map(|(i, l)| l[*i].clone()).collect();
            let ckey: Vec<String> = combo.iter().map(|x| expr::normalize(x)).collect();
            let el = elements.iter().find(|(k, _)| *k == ckey).map(|(_, e)| *e);
            let text: Option<String> = match (el, &list) {
                (Some(e), _) => e.child("eqn").map(|x| x.text().trim().to_string()),
                (None, Some(l)) => Some(l[c].clone()),
                (None, None) => general.clone(),
            };
            // fix the dimensions to this combination's items inside every subscript
            let text = text.map(|t| {
                let mut out = String::new();
                let mut depth = 0;
                let mut word = String::new();
                let flush = |word: &mut String, out: &mut String, depth: i32| {
                    if !word.is_empty() {
                        let w = display_of(word);
                        let hit = if depth > 0 { dims.iter().position(|d| expr::normalize(d) == expr::normalize(&w)) } else { None };
                        match hit {
                            Some(p) => out.push_str(&combo[p].replace(' ', "_")),
                            None => out.push_str(word),
                        }
                        word.clear();
                    }
                };
                for ch in t.chars() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                        word.push(ch);
                        continue;
                    }
                    flush(&mut word, &mut out, depth);
                    if ch == '[' {
                        depth += 1;
                    } else if ch == ']' {
                        depth -= 1;
                    }
                    out.push(ch);
                }
                flush(&mut word, &mut out, depth);
                out
            });
            // a copy of the node for this combination: its own name and equation, no dimensions or elements
            let mut cn = n.clone();
            let cname = format!("{display}[{}]", combo.join(","));
            cn.attrs.retain(|(k, _)| k != "name");
            cn.attrs.push(("name".into(), format!("\"{}\"", cname.replace('"', "\\\""))));
            cn.kids.retain(|k| !matches!(k, Kid::Node(x) if matches!(x.local(), "dimensions" | "element" | "eqn" | "inflow" | "outflow")));
            if let Some(t) = text {
                cn.kids.push(Kid::Node(Node { name: "eqn".into(), attrs: vec![], kids: vec![Kid::Text(t)] }));
            }
            // flows of a multi-dimensional stock are the same flows' combinations
            for tag in ["inflow", "outflow"] {
                for f in n.all(tag) {
                    let fname = display_of(&f.text());
                    let fkey = expr::normalize(&join(prefix, &fname));
                    let fname_c = if self.flat.contains_key(&fkey) { format!("\"{fname}[{}]\"", combo.join(",")) } else { f.text() };
                    cn.kids.push(Kid::Node(Node { name: tag.into(), attrs: vec![], kids: vec![Kid::Text(fname_c)] }));
                }
            }
            self.read_var(&cn, prefix);
            for k in (0..idx.len()).rev() {
                idx[k] += 1;
                if idx[k] < lists[k].len() {
                    break;
                }
                idx[k] = 0;
            }
        }
    }

    fn read_vars(&mut self, vars: &Node, prefix: &str) {
        use crate::loss::LossKind::Dropped;
        // first: which variables have two or more dimensions (references to them are flattened)
        for n in vars.children() {
            if matches!(n.local(), "stock" | "flow" | "aux" | "gf") {
                if let Some(d) = n.child("dimensions") {
                    let names: Vec<String> = d.all("dim").iter().filter_map(|x| x.attr("name")).map(display_of).collect();
                    if names.len() >= 2 {
                        let display = display_of(n.attr("name").unwrap_or(""));
                        self.flat.insert(expr::normalize(&join(prefix, &display)), (display, names));
                    }
                }
            }
        }
        for n in vars.children() {
            match n.local() {
                "stock" | "flow" | "aux" | "gf" => self.read_var(n, prefix),
                "module" => self.flatten_module(n, prefix),
                "group" => {
                    let members: Vec<String> = n.all("entity").iter().filter_map(|e| e.attr("name")).map(|s| join(prefix, &display_of(s))).collect();
                    self.groups.push((display_of(n.attr("name").unwrap_or("Group")), members));
                }
                other => note(&mut self.rep, Dropped, &format!("tag <{other}>"), "is not part of Dynamist and was left out"),
            }
        }
    }

    fn flatten_module(&mut self, module: &Node, scope: &str) {
        use crate::loss::LossKind::{Approximated, Dropped};
        let mname = display_of(module.attr("name").unwrap_or(""));
        let full = join(scope, &mname);
        let model = self.submodels.iter().find(|n| expr::normalize(n.attr("name").unwrap_or("")) == expr::normalize(&mname)).cloned();
        let Some(model) = model else {
            note(&mut self.rep, Dropped, &format!("module {mname}"), "its submodel is not in the file (external resources are not followed)");
            return;
        };
        note(
            &mut self.rep,
            crate::loss::LossKind::Flattened,
            "submodels",
            "XMILE submodels were flattened into the main model: their variables are named Module.Variable and inputs became equations that read the connected value",
        );
        if let Some(v) = model.child("variables") {
            self.read_vars(v, &full);
        }
        for cn in module.all("connect") {
            let (to, from) = (cn.attr("to").unwrap_or(""), cn.attr("from").unwrap_or(""));
            let resolve = |s: &str, base: &str| -> String {
                match s.strip_prefix('.') {
                    Some(rest) => name_keep_dots(rest),
                    None => join(base, &name_keep_dots(s)),
                }
            };
            self.connects.push((resolve(to, &full), resolve(from, scope)));
        }
    }

    fn read_var(&mut self, n: &Node, prefix: &str) {
        use crate::loss::LossKind::{Approximated, Dropped};
        let tag = n.local();
        let display = display_of(n.attr("name").unwrap_or(""));
        if display.is_empty() {
            note(&mut self.rep, Dropped, &format!("<{tag}>"), "has no name and was left out");
            return;
        }
        let full = join(prefix, &display);
        let key = expr::normalize(&full);
        // Vensim-style control variables: they are the run settings, which Dynamist keeps in the settings
        if prefix.is_empty() && matches!(key.as_str(), "time" | "initial time" | "final time" | "time step" | "dt") {
            note(&mut self.rep, crate::loss::LossKind::Preserved, "run-setting variables", "INITIAL TIME, FINAL TIME and TIME STEP defined as variables are the run settings; references to them use the built-in names");
            return;
        }
        if self.by_name.contains_key(&key) {
            note(&mut self.rep, Dropped, &format!("variable {full}"), "the name is used twice (names ignore case, spaces and underscores); the second one was left out");
            return;
        }
        // helper graphical functions written by Dynamist for GRAPH / WITH LOOKUP are not variables
        if tag == "gf" && helper_kind(&display).is_some() && prefix.is_empty() {
            return;
        }
        if prefix.is_empty() && self.inits.contains_key(&key) {
            return;
        }
        // arrays
        let mut set: Option<String> = None;
        if let Some(d) = n.child("dimensions") {
            let names: Vec<String> = d.all("dim").iter().filter_map(|x| x.attr("name")).map(display_of).collect();
            match names.as_slice() {
                [one] => match self.m.sets.iter().find(|s| expr::normalize(&s.name) == expr::normalize(one)) {
                    Some(s) => set = Some(s.name.clone()),
                    None => {
                        note(&mut self.rep, Dropped, &format!("variable {full}"), &format!("uses the dimension “{one}”, which is not defined; the variable was left out"));
                        return;
                    }
                },
                _ => {
                    return self.read_var_flat(n, prefix, &display, &names);
                }
            }
        }
        let units = n.child("units").map(|u| units_from_xmile(&u.text())).unwrap_or_default();
        let comment = n.child("doc").map(|d| doc_from_xmile(&d.text())).unwrap_or_default();
        let eqn_text = n.child("eqn").map(|e| e.text().trim().to_string()).filter(|t| !t.is_empty());
        let gf = match (if tag == "gf" { Some(n) } else { n.child("gf") }).map(read_gf) {
            Some(Ok(g)) => Some(g),
            Some(Err(e)) => {
                note(&mut self.rep, Dropped, &format!("graphical function of {full}"), &format!("could not be read: {e}"));
                None
            }
            None => None,
        };
        let id = self.m.new_id();
        let kind;
        let mut equation = String::new();
        let mut table: Vec<[f64; 2]> = Vec::new();
        match tag {
            "stock" => {
                kind = Kind::Stock;
                match &eqn_text {
                    Some(t) => equation = self.eq(t, prefix, &full),
                    None => {
                        equation = "0".into();
                        note(&mut self.rep, Dropped, &format!("equation of {full}"), "a stock without an initial value starts at 0");
                    }
                }
            }
            "flow" => {
                kind = Kind::Flow;
                match (&eqn_text, &gf) {
                    (Some(t), _) => equation = self.eq(t, prefix, &full),
                    (None, _) => {
                        equation = "0".into();
                        note(&mut self.rep, Dropped, &format!("equation of {full}"), "a flow without an equation is 0");
                    }
                }
            }
            _ => {
                // an auxiliary, or a graphical function standing alone
                match (&eqn_text, &gf) {
                    (_, Some(g)) => {
                        kind = Kind::Lookup;
                        if g.extrapolate {
                            note(&mut self.rep, Approximated, "graphical function type extrapolate", &format!("{full}: values outside the points are held flat instead of extrapolated"));
                        }
                        table = g.pts.clone();
                        let input = match &eqn_text {
                            Some(t) => self.eq(t, prefix, &full),
                            None => String::new(),
                        };
                        if g.discrete {
                            // a step function: only an aux with an input can say GRAPHSTEP
                            match (is_even(&g.pts), input.is_empty()) {
                                (Some((x0, dx)), false) if g.pts.len() >= 3 => {
                                    let ys: Vec<String> = g.pts[..g.pts.len() - 1].iter().map(|p| fmt_f64(p[1])).collect();
                                    equation = format!("GRAPHSTEP({input}, {}, {}, {})", fmt_f64(x0), fmt_f64(dx), ys.join(", "));
                                    // it is now an ordinary variable that evaluates the step function
                                    let id2 = id;
                                    let _ = id2;
                                    return self.finish_var(n, prefix, &display, &full, id, Kind::Aux, equation, Vec::new(), units, comment, set);
                                }
                                _ => note(&mut self.rep, Approximated, "graphical function type discrete", &format!("{full}: read as a continuous curve (Dynamist lookups interpolate)")),
                            }
                        }
                        equation = input;
                    }
                    (Some(t), None) => {
                        let parsed = xm_parse(t);
                        if let Some(v) = parsed.ok().as_ref().and_then(lit) {
                            kind = Kind::Constant;
                            equation = fmt_f64(v);
                        } else {
                            kind = Kind::Aux;
                            equation = self.eq(t, prefix, &full);
                        }
                    }
                    (None, None) => {
                        kind = Kind::Aux;
                        if n.child("element").is_none() {
                            equation = "0".into();
                            if !(prefix.is_empty() || n.attr("access") == Some("input")) {
                                note(&mut self.rep, Dropped, &format!("equation of {full}"), "has no equation and was set to 0");
                            }
                        }
                    }
                }
            }
        }
        self.finish_var(n, prefix, &display, &full, id, kind, equation, table, units, comment, set);
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_var(&mut self, n: &Node, prefix: &str, display: &str, full: &str, id: u64, kind: Kind, equation: String, table: Vec<[f64; 2]>, units: String, comment: String, set: Option<String>) {
        use crate::loss::LossKind::{Approximated, Dropped};
        let key = expr::normalize(full);
        let mut v = Var::new(id, full, kind, [0.0, 0.0]);
        v.equation = equation;
        v.units = units;
        v.comment = comment;
        v.table = table;
        v.set = set.clone();
        let _ = display;
        if let Some(r) = n.child("range") {
            if let (Some(lo), Some(hi)) = (r.attr("min").and_then(|x| x.trim().parse::<f64>().ok()), r.attr("max").and_then(|x| x.trim().parse::<f64>().ok())) {
                if kind == Kind::Constant {
                    v.slider = Some([lo, hi]);
                }
            }
        }
        let own_nn = n.child("non_negative").map(|nn| !nn.text().trim().eq_ignore_ascii_case("false"));
        let mut nn_flow = false;
        match kind {
            Kind::Stock => v.non_negative = own_nn.unwrap_or(self.nn_stock),
            Kind::Flow => nn_flow = own_nn.unwrap_or(self.nn_flow),
            _ => {}
        }
        // per-element equations of an array
        let elements = n.all("element");
        if let Some(sname) = &set {
            let items = self.m.sets.iter().find(|s| &s.name == sname).map(|s| s.items.clone()).unwrap_or_default();
            // a comma list of numbers, one per item (as Vensim's XMILE export writes array constants)
            if elements.is_empty() {
                let raw = n.child("eqn").map(|e| e.text()).unwrap_or_default();
                let parts: Vec<String> = raw.split(|c| c == ',' || c == ';').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                if parts.len() == items.len() && parts.len() > 1 && parts.iter().all(|p| p.parse::<f64>().is_ok()) {
                    v.equation = parts[0].clone();
                    v.kind = Kind::Constant;
                    for (it, val) in items.iter().zip(&parts).skip(1) {
                        v.by_item.push(ItemValue { item: it.clone(), equation: val.clone(), table: Vec::new() });
                    }
                }
            }
            for e in &elements {
                let sub = e.attr("subscript").unwrap_or("").trim().to_string();
                let item = if let Ok(k) = sub.parse::<usize>() { items.get(k.wrapping_sub(1)).cloned() } else { items.iter().find(|i| expr::normalize(i) == expr::normalize(&display_of(&sub))).cloned() };
                let Some(item) = item else {
                    note(&mut self.rep, Dropped, &format!("element {sub} of {full}"), "is not one element of a one-dimensional array and was left out");
                    continue;
                };
                let eq = e.child("eqn").map(|x| x.text().trim().to_string()).filter(|t| !t.is_empty()).map(|t| self.eq(&t, prefix, full)).unwrap_or_default();
                let tbl = match e.child("gf").map(read_gf) {
                    Some(Ok(g)) => g.pts,
                    _ => Vec::new(),
                };
                v.by_item.push(ItemValue { item, equation: eq, table: tbl });
            }
            if v.equation.is_empty() || (v.equation == "0" && !elements.is_empty()) {
                if let Some(first) = v.by_item.iter().find(|o| !o.equation.is_empty()) {
                    v.equation = first.equation.clone();
                }
            }
            if kind == Kind::Lookup && v.table.is_empty() {
                if let Some(first) = v.by_item.iter().find(|o| !o.table.is_empty()) {
                    v.table = first.table.clone();
                }
            }
            self.var_set.insert(key.clone(), sname.clone());
        } else if !elements.is_empty() {
            note(&mut self.rep, Dropped, &format!("elements of {full}"), "elements without dimensions were ignored");
        }
        if nn_flow {
            note(&mut self.rep, Approximated, "non-negative flow", &format!("{full}: Dynamist flows have no non-negative setting; the equation was wrapped as MAX(0, …)"));
            v.equation = format!("MAX(0, {})", v.equation);
            for o in v.by_item.iter_mut().filter(|o| !o.equation.is_empty()) {
                o.equation = format!("MAX(0, {})", o.equation);
            }
        }
        // flows in and out of a stock
        if kind == Kind::Stock {
            for f in n.all("inflow") {
                self.pending.push((id, expr::normalize(&join(prefix, &display_of(&f.text()))), true));
            }
            for f in n.all("outflow") {
                self.pending.push((id, expr::normalize(&join(prefix, &display_of(&f.text()))), false));
            }
        }
        // things XMILE can say that Dynamist has no place for
        for k in n.children() {
            match k.local() {
                "eqn" | "units" | "doc" | "range" | "inflow" | "outflow" | "non_negative" | "gf" | "dimensions" | "element" | "xscale" | "yscale" | "xpts" | "ypts" => {}
                "conveyor" | "queue" => note(&mut self.rep, Dropped, &format!("{} on {full}", k.local()), "Dynamist has no conveyors or queues; the stock is an ordinary stock"),
                "multiplier" => note(&mut self.rep, Dropped, &format!("unit conversion multiplier on {full}"), "ignored"),
                "event_poster" => note(&mut self.rep, Dropped, "event poster", &format!("on {full}")),
                "scale" | "format" => note(&mut self.rep, Approximated, "scale and number format", "display settings on variables were not kept"),
                "leak" | "leak_integers" | "overflow" => note(&mut self.rep, Dropped, &format!("{} on {full}", k.local()), "queue and conveyor options are not supported"),
                "mathml" => note(&mut self.rep, Dropped, &format!("MathML equation of {full}"), "only the text equation is read"),
                other => note(&mut self.rep, Dropped, &format!("tag <{other}> on {full}"), "is not part of Dynamist and was left out"),
            }
        }
        self.by_name.insert(key, id);
        self.m.vars.push(v);
    }
}

fn read_number(n: Option<&Node>, what: &str) -> Result<Option<f64>, String> {
    let Some(n) = n else { return Ok(None) };
    let t = n.text();
    match xm_parse(t.trim()).ok().as_ref().and_then(lit) {
        Some(v) => Ok(Some(v)),
        None => Err(format!("{what} must be a number, but it is “{}”", t.trim())),
    }
}

fn read_method(s: &str, rep: &mut LossReport) -> Method {
    for p in s.split(',').map(|x| x.trim().to_lowercase()) {
        match p.as_str() {
            "euler" => return Method::Euler,
            "rk2" => return Method::RungeKutta2,
            "rk4" => return Method::RungeKutta4,
            _ => {}
        }
    }
    note(rep, crate::loss::LossKind::Approximated, &format!("integration method {s}"), "Dynamist does not have it; the model runs with Runge-Kutta 4");
    Method::RungeKutta4
}

/// The centre of a diagram object, from the position attributes XMILE gives it.
fn view_pos(o: &Node) -> Option<[f32; 2]> {
    let f = |k: &str| o.attr(k).and_then(|v| v.trim().parse::<f32>().ok());
    let (x, y) = (f("x")?, f("y")?);
    if o.child("shape").is_some() {
        return Some([x, y]);
    }
    match (f("width"), f("height")) {
        (Some(w), Some(h)) => Some([x + w / 2.0, y + h / 2.0]),
        _ => Some([x, y]),
    }
}

/// Read XMILE text into a model. The report lists everything that could not be carried over.
pub fn read(text: &str) -> Result<(Model, LossReport), String> {
    use crate::loss::LossKind::{Approximated, Dropped, Preserved};
    let root = parse_xml(text)?;
    if root.local() != "xmile" {
        return Err(format!("This is not an XMILE file: the first element is <{}>, not <xmile>", root.name));
    }
    let models = root.all("model");
    if models.is_empty() {
        return Err("The XMILE file has no <model>".into());
    }
    let root_model = models.iter().find(|n| n.attr("name").is_none()).copied().unwrap_or(models[0]);
    let subs: Vec<Node> = models.iter().filter(|n| !std::ptr::eq(**n, root_model)).map(|n| (*n).clone()).collect();
    let mut rep = LossReport::default();
    let mut m = Model { title: "Imported XMILE model".into(), settings: crate::model::Settings { units_scale_values: false, ..Default::default() }, ..Default::default() };
    if let Some(name) = root.child("header").and_then(|h| h.child("name")) {
        let t = name.text();
        if !t.trim().is_empty() {
            m.title = t.trim().to_string();
        }
    }
    // simulation specifications
    let specs = root_model.child("sim_specs").or_else(|| root.child("sim_specs")).ok_or("The XMILE file has no <sim_specs> (start and stop time are required)")?;
    let t0 = read_number(specs.child("start"), "the start time")?.ok_or("The simulation specifications have no <start>")?;
    let t1 = read_number(specs.child("stop"), "the stop time")?.ok_or("The simulation specifications have no <stop>")?;
    let mut dt = read_number(specs.child("dt"), "the time step")?.unwrap_or(1.0);
    if specs.child("dt").and_then(|d| d.attr("reciprocal")).is_some_and(|r| r.trim().eq_ignore_ascii_case("true")) && dt != 0.0 {
        dt = 1.0 / dt;
    }
    if dt <= 0.0 || t1 < t0 {
        return Err("The simulation specifications are not valid: the time step must be positive and the stop time cannot be before the start time".into());
    }
    m.settings.initial_time = t0;
    m.settings.final_time = t1;
    m.settings.time_step = dt;
    if let Some(u) = specs.attr("time_units") {
        m.settings.time_units = display_of(u);
    }
    if let Some(meth) = specs.attr("method") {
        m.settings.method = read_method(meth, &mut rep);
    }
    if specs.attr("pause").is_some() || specs.child("run").is_some() {
        note(&mut rep, Dropped, "pause interval and run selection", "not used by Dynamist");
    }
    // dimensions → sets
    let mut dims = HashSet::new();
    for dn in root.child("dimensions").into_iter().chain(root_model.child("dimensions")) {
        for d in dn.all("dim") {
            let name = display_of(d.attr("name").unwrap_or(""));
            if name.is_empty() || m.sets.iter().any(|s| expr::normalize(&s.name) == expr::normalize(&name)) {
                continue;
            }
            let items: Vec<String> = if d.all("elem").is_empty() {
                let k = d.attr("size").and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(0);
                (1..=k).map(|i| i.to_string()).collect()
            } else {
                d.all("elem").iter().map(|e| display_of(e.attr("name").unwrap_or(""))).collect()
            };
            let id = m.new_id();
            dims.insert(expr::normalize(&name));
            m.sets.push(ItemSet { id, name, items, parent: None, parent_of: Vec::new(), note: String::new() });
        }
    }
    let mut r = Reader { m, rep, helpers: HashMap::new(), inits: HashMap::new(), dims, var_set: HashMap::new(), by_name: HashMap::new(), pending: Vec::new(), connects: Vec::new(), submodels: subs, groups: Vec::new(), t0, dt, nn_stock: false, nn_flow: false, flat: HashMap::new() };
    // graphical functions that stand in for GRAPH / WITH LOOKUP, and which variable is an array
    if let Some(vars) = root_model.child("variables") {
        for v in vars.children() {
            let name = display_of(v.attr("name").unwrap_or(""));
            if let (Some(kind), Some(g)) = (helper_kind(&name), v.child("gf").or(if v.local() == "gf" { Some(v) } else { None })) {
                if let Ok(g) = read_gf(g) {
                    r.helpers.insert(expr::normalize(&name), HelperDef { kind, pts: g.pts });
                }
            }
            if v.local() == "aux" && expr::normalize(&name).starts_with("fc init ") {
                if let Some(e) = v.child("eqn") {
                    r.inits.insert(expr::normalize(&name), e.text().trim().to_string());
                }
            }
            if let Some(d) = v.child("dimensions") {
                if let Some(dim) = d.all("dim").first().and_then(|x| x.attr("name")) {
                    r.var_set.insert(expr::normalize(&name), display_of(dim));
                }
            }
        }
    }
    // default behaviours: <behavior><non_negative/></behavior>, or per kind of variable
    for b in root.child("behavior").into_iter().chain(root_model.child("behavior")) {
        let on = |n: &Node| !n.text().trim().eq_ignore_ascii_case("false");
        if let Some(nn) = b.child("non_negative") {
            r.nn_stock = on(nn);
            r.nn_flow = on(nn);
        }
        if let Some(nn) = b.child("stock").and_then(|s| s.child("non_negative")) {
            r.nn_stock = on(nn);
        }
        if let Some(nn) = b.child("flow").and_then(|s| s.child("non_negative")) {
            r.nn_flow = on(nn);
        }
    }
    for n in ["style", "data", "model_units"] {
        if root.child(n).is_some_and(|x| x.children().next().is_some()) {
            note(&mut r.rep, Dropped, &format!("<{n}>"), "is not used by Dynamist");
        }
    }
    // the person's own functions (macros)
    for mn in root.all("macro") {
        let name = display_of(mn.attr("name").unwrap_or(""));
        if name.is_empty() {
            continue;
        }
        if mn.child("variables").is_some() {
            note(&mut r.rep, Dropped, &format!("macro {name}"), "macros that contain stocks, flows or auxiliaries are not supported");
            continue;
        }
        let doc = mn.child("doc").map(|d| doc_from_xmile(&d.text())).unwrap_or_default();
        let parms = mn.all("parm");
        if parms.iter().any(|p| p.attr("default").is_some()) {
            note(&mut r.rep, Dropped, &format!("macro {name}"), "default values of parameters are not supported; every parameter must be given");
        }
        let params: Vec<String> = parms.iter().map(|p| display_of(&p.text())).collect();
        let body = mn.child("eqn").map(|e| e.text().trim().to_string()).unwrap_or_default();
        let body = r.eq(&body, "", &format!("macro {name}"));
        r.m.functions.push(crate::userfn::UserFn { name, params, body, description: doc });
    }
    if let Some(vars) = root_model.child("variables") {
        r.read_vars(vars, "");
    }
    // flows in and out of stocks
    for (sid, fname, inflow) in std::mem::take(&mut r.pending) {
        let Some(&fid) = r.by_name.get(&fname) else {
            note(&mut r.rep, Dropped, &format!("flow {fname}"), "is named by a stock but is not defined; the connection was left out");
            continue;
        };
        let stock_name = r.m.var(sid).map(|v| v.name.clone()).unwrap_or_default();
        // a stock may name an <aux> as its flow (Vensim's XMILE export does): it is a flow, then
        if let Some(v) = r.m.var_mut(fid).filter(|f| matches!(f.kind, Kind::Aux | Kind::Constant)) {
            v.kind = Kind::Flow;
            let n = v.name.clone();
            note(&mut r.rep, crate::loss::LossKind::Approximated, &format!("flow {n}"), &format!("declared as an auxiliary but named by {stock_name} as its flow; read as a flow"));
        }
        let Some(flow) = r.m.var_mut(fid).filter(|f| f.kind == Kind::Flow) else {
            note(&mut r.rep, Dropped, &format!("flow {fname}"), &format!("{stock_name} names it as a flow, but it is not one"));
            continue;
        };
        let slot = if inflow { &mut flow.to } else { &mut flow.from };
        match slot {
            Some(other) if *other != sid => {
                // the same flow on a second stock: a copy that reads the original, so both stocks move
                let fname = flow.name.clone();
                let id = r.m.new_id();
                let mut copy = Var::new(id, &format!("{fname} into {stock_name}"), Kind::Flow, [0.0, 0.0]);
                copy.equation = format!("\"{}\"", fname.replace('"', "'"));
                if inflow { copy.to = Some(sid) } else { copy.from = Some(sid) }
                r.m.vars.push(copy);
                note(&mut r.rep, crate::loss::LossKind::Approximated, &format!("flow {fname}"), &format!("is an {} of more than one stock; {stock_name} got its own flow that equals it", if inflow { "inflow" } else { "outflow" }));
            }
            _ => *slot = Some(sid),
        }
    }
    // submodel inputs: the connected value becomes the equation
    for (to, from) in std::mem::take(&mut r.connects) {
        match r.by_name.get(&expr::normalize(&to)).copied() {
            Some(id) => {
                let src = fc_ident(&from);
                if let Some(v) = r.m.var_mut(id) {
                    v.equation = src;
                }
            }
            None => note(&mut r.rep, Dropped, &format!("connection to {to}"), "the submodel input does not exist"),
        }
    }
    // groups
    let first = {
        r.m.ensure_sheets();
        r.m.first_diagram()
    };
    for (name, members) in std::mem::take(&mut r.groups) {
        let ids: Vec<u64> = members.iter().filter_map(|n| r.by_name.get(&expr::normalize(n)).copied()).collect();
        if ids.len() >= 2 {
            let id = r.m.new_id();
            r.m.groups.push(Group { id, name, members: ids, view: first, color: None });
            note(&mut r.rep, Approximated, "groups", "XMILE groups were kept as diagram groups (frames), not as modules");
        }
    }
    // diagram views
    let mut placed: HashSet<u64> = HashSet::new();
    let views: Vec<&Node> = root_model.child("views").map(|v| v.all("view")).unwrap_or_default();
    let mut sheet_ids: Vec<u64> = Vec::new();
    for (i, vw) in views.iter().enumerate() {
        if vw.attr("type").is_some_and(|t| t != "stock_flow") {
            note(&mut r.rep, Dropped, &format!("view of type {}", vw.attr("type").unwrap_or("")), "interface views are not read");
            continue;
        }
        let sid = if sheet_ids.is_empty() {
            first
        } else {
            let id = r.m.new_id();
            let name = vw.attr("name").map(display_of).unwrap_or_else(|| format!("View {}", i + 1));
            r.m.sheets.push(Sheet::new(id, &name, SheetKind::Diagram));
            id
        };
        sheet_ids.push(sid);
        for o in vw.children() {
            match o.local() {
                "stock" | "flow" | "aux" => {
                    let Some(&id) = r.by_name.get(&expr::normalize(&display_of(o.attr("name").unwrap_or("")))) else { continue };
                    if placed.contains(&id) {
                        continue;
                    }
                    let Some(p) = view_pos(o) else { continue };
                    placed.insert(id);
                    let pts: Vec<[f32; 2]> = o
                        .child("pts")
                        .map(|p| p.all("pt").iter().filter_map(|pt| Some([pt.attr("x")?.trim().parse::<f32>().ok()?, pt.attr("y")?.trim().parse::<f32>().ok()?])).collect())
                        .unwrap_or_default();
                    if let Some(v) = r.m.var_mut(id) {
                        v.pos = p;
                        v.view = sid;
                        if v.kind == Kind::Flow && pts.len() >= 2 {
                            v.from_pt = pts[0];
                            v.to_pt = pts[pts.len() - 1];
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // everything XMILE draws but Dynamist does not: connectors are read after the variables are placed
    for vw in &views {
        for o in vw.children().filter(|o| o.local() == "connector") {
            let name_of = |tag: &str| o.child(tag).map(|n| display_of(n.text().trim())).filter(|s| !s.is_empty());
            let (Some(f), Some(t)) = (name_of("from"), name_of("to")) else { continue };
            let (Some(&fa), Some(&ta)) = (r.by_name.get(&expr::normalize(&f)), r.by_name.get(&expr::normalize(&t))) else { continue };
            if r.m.add_link(fa, ta) {
                if let Some(l) = r.m.links.last_mut() {
                    l.polarity = match o.attr("polarity") {
                        Some("+") => "+".into(),
                        Some("-") => "-".into(),
                        _ => String::new(),
                    };
                    l.delay_mark = o.attr("delay_mark").is_some_and(|d| d.eq_ignore_ascii_case("true"));
                    l.dashed = o.attr("line_style") == Some("dashed");
                }
            }
        }
    }
    // variables the views did not place: a simple grid
    let mut k = 0;
    for v in r.m.vars.iter_mut() {
        if !placed.contains(&v.id) {
            v.pos = [120.0 + (k % 6) as f32 * 200.0, 120.0 + (k / 6) as f32 * 130.0];
            v.view = first;
            if v.kind == Kind::Flow {
                v.from_pt = [v.pos[0] - 70.0, v.pos[1]];
                v.to_pt = [v.pos[0] + 70.0, v.pos[1]];
            }
            k += 1;
        }
    }
    // arrows for what the equations use
    crate::userfn::set(&r.m.functions);
    let eqs: Vec<(u64, String)> = r.m.vars.iter().filter(|v| !v.equation.trim().is_empty()).map(|v| (v.id, v.equation.clone())).collect();
    for (id, eq) in eqs {
        if let Ok(e) = expr::parse(&eq) {
            let mut refs = Vec::new();
            expr::references(&e, &mut refs);
            for name in refs {
                if let Some(src) = r.m.by_name(&name).map(|v| v.id) {
                    if src != id {
                        r.m.add_link(src, id);
                    }
                }
            }
        }
    }
    // what only Dynamist knows (kept in a comment by `write`)
    for c in root.comments() {
        let t = c.trim();
        let Some(json) = t.strip_prefix(META_TAG.trim_end()).or_else(|| t.strip_prefix("flowcanvas:meta")) else { continue };
        match serde_json::from_str::<Meta>(json.trim()) {
            Ok(meta) => {
                for (name, vm) in &meta.vars {
                    if let Some(&id) = r.by_name.get(&expr::normalize(name)) {
                        if let Some(v) = r.m.var_mut(id) {
                            v.lever = vm.lever;
                            v.lever_scale = vm.lever_scale;
                            if vm.average {
                                v.rollup = Rollup::Average;
                            }
                        }
                    }
                }
                for mut s in meta.scenarios {
                    s.id = r.m.new_id();
                    r.m.scenarios.push(s);
                }
                r.m.checks = meta.checks;
                r.m.manifest = meta.manifest;
                note(&mut r.rep, Preserved, "Dynamist settings", "levers, scenarios, checks and roll-up settings were restored from the dynamist:meta comment in the file");
            }
            Err(e) => note(&mut r.rep, Dropped, "dynamist:meta comment", &format!("could not be read: {e}")),
        }
    }
    let mut m = r.m;
    m.ensure_sheets();
    Ok((m, r.rep))
}

// =========================================================================== writing XMILE

fn f32s(v: f32) -> String {
    format!("{v}")
}

/// A name inside `<inflow>`, `<from>`, `<entity>` and the like: underscores for spaces.
fn name_text(display: &str) -> String {
    words(display).replace(' ', "_")
}

/// Can this model be written with real XMILE dimensions (no parent sets, no partial TOTALs)?
fn plain_sets(m: &Model) -> bool {
    if m.sets.iter().any(|s| s.parent.is_some() || !s.parent_of.is_empty()) {
        return false;
    }
    fn partial(x: &X) -> bool {
        match x {
            X::Call(n, a) => ((n == "TOTAL" || n == "AVERAGE") && a.len() != 1) || a.iter().any(partial),
            X::Sub(_, a) => a.iter().any(partial),
            X::Neg(a) | X::Not(a) => partial(a),
            X::Bin(_, a, b) => partial(a) || partial(b),
            X::If(a, b, c) => partial(a) || partial(b) || partial(c),
            _ => false,
        }
    }
    m.vars.iter().filter(|v| v.set.is_some()).all(|v| {
        std::iter::once(&v.equation).chain(v.by_item.iter().map(|o| &o.equation)).filter(|e| !e.trim().is_empty()).all(|e| fc_parse(e).map(|x| !partial(&x)).unwrap_or(true))
    })
}

struct Writer<'a> {
    o: Out,
    ctx: WCtx<'a>,
    native: bool,
}

impl Writer<'_> {
    fn tx(&mut self, text: &str, owner: &str) -> String {
        use crate::loss::LossKind::Dropped;
        if text.trim().is_empty() {
            return "0".into();
        }
        match fc_parse(text) {
            Ok(x) => {
                let y = to_xm(&x, &mut self.ctx);
                emit(&y, Dialect::Xm)
            }
            Err(e) => {
                note(self.ctx.rep, Dropped, &format!("equation of {owner}"), &format!("could not be translated ({e}); it is written as it was typed"));
                text.trim().to_string()
            }
        }
    }

    fn gf(&mut self, name: Option<&str>, pts: &[[f64; 2]], discrete: bool) {
        let mut attrs: Vec<(&str, String)> = Vec::new();
        if let Some(n) = name {
            attrs.push(("name", name_attr(n)));
        }
        if discrete {
            attrs.push(("type", "discrete".into()));
        }
        self.o.open("gf", &attrs);
        match is_even(pts) {
            Some(_) => self.o.empty("xscale", &[("min", fmt_f64(pts[0][0])), ("max", fmt_f64(pts[pts.len() - 1][0]))]),
            None => self.o.leaf("xpts", &[], &pts.iter().map(|p| fmt_f64(p[0])).collect::<Vec<_>>().join(",")),
        }
        self.o.leaf("ypts", &[], &pts.iter().map(|p| fmt_f64(p[1])).collect::<Vec<_>>().join(","));
        self.o.close("gf");
    }

    fn variable(&mut self, w: &Model, v: &Var) {
        use crate::loss::LossKind::{Approximated, Dropped};
        let name = name_attr(&v.name);
        // a lookup used like a function: a named graphical function on its own
        if v.kind == Kind::Lookup && v.equation.trim().is_empty() && v.set.is_none() && v.units.trim().is_empty() && v.comment.trim().is_empty() {
            let pts = v.table.clone();
            if pts.is_empty() {
                note(self.ctx.rep, Dropped, &format!("lookup {}", v.name), "has no points and was left out");
                return;
            }
            self.gf(Some(&v.name), &pts, false);
            return;
        }
        let tag = match v.kind {
            Kind::Stock => "stock",
            Kind::Flow => "flow",
            _ => "aux",
        };
        self.o.open(tag, &[("name", name)]);
        let dims = self.native && v.set.is_some();
        if let (true, Some(s)) = (dims, &v.set) {
            self.o.open("dimensions", &[]);
            self.o.empty("dim", &[("name", name_attr(s))]);
            self.o.close("dimensions");
        }
        // a constant that is per a set with a lever scale: bake the scale into the values
        let scale = if v.kind == Kind::Constant && v.set.is_some() && (v.lever_scale - 1.0).abs() > 1e-12 { Some(v.lever_scale) } else { None };
        if scale.is_some() {
            note(self.ctx.rep, Approximated, "lever scale", &format!("{}: the all-items scale was multiplied into the values", v.name));
        }
        let wrap = |t: String| -> String { if let Some(sc) = scale { format!("({t}) * {}", fmt_f64(sc)) } else { t } };
        let per_item_eq = v.by_item.iter().any(|o| !o.equation.trim().is_empty());
        let per_item_tbl = v.by_item.iter().any(|o| !o.table.is_empty());
        let items: Vec<String> = if dims { w.items_of(v) } else { Vec::new() };
        // a lookup used like a function (no input) has no equation, only its curve
        let no_eqn = v.kind == Kind::Lookup && v.equation.trim().is_empty();
        if !(dims && per_item_eq) && !no_eqn {
            let e = self.tx(&v.equation, &v.name);
            self.o.leaf("eqn", &[], &wrap(e));
        }
        if v.kind == Kind::Lookup && !v.table.is_empty() && !(dims && per_item_tbl) {
            let pts = v.table.clone();
            self.gf(None, &pts, false);
        }
        if dims && (per_item_eq || per_item_tbl) {
            for it in &items {
                let o = v.by_item.iter().find(|o| expr::normalize(&o.item) == expr::normalize(it));
                self.o.open("element", &[("subscript", name_attr(it))]);
                if per_item_eq {
                    let text = o.map(|o| o.equation.clone()).filter(|e| !e.trim().is_empty()).unwrap_or_else(|| v.equation.clone());
                    let e = self.tx(&text, &format!("{}[{}]", v.name, it));
                    self.o.leaf("eqn", &[], &wrap(e));
                }
                if per_item_tbl {
                    let pts = o.map(|o| o.table.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| v.table.clone());
                    if !pts.is_empty() {
                        self.gf(None, &pts, false);
                    }
                }
                self.o.close("element");
            }
        }
        if v.kind == Kind::Stock {
            for f in w.vars.iter().filter(|f| f.kind == Kind::Flow && f.to == Some(v.id) && f.from != Some(v.id)) {
                self.o.leaf("inflow", &[], &name_text(&f.name));
            }
            for f in w.vars.iter().filter(|f| f.kind == Kind::Flow && f.from == Some(v.id) && f.to != Some(v.id)) {
                self.o.leaf("outflow", &[], &name_text(&f.name));
            }
            if v.non_negative {
                self.o.empty("non_negative", &[]);
            }
        }
        if !v.units.trim().is_empty() {
            self.o.leaf("units", &[], &units_to_xmile(&v.units));
        }
        if !v.comment.trim().is_empty() {
            self.o.leaf("doc", &[], &doc_to_xmile(&v.comment));
        }
        if let (Kind::Constant, Some(r)) = (v.kind, v.slider) {
            self.o.empty("range", &[("min", fmt_f64(r[0])), ("max", fmt_f64(r[1]))]);
        }
        self.o.close(tag);
    }
}

/// The names of the variables on a sheet, with the path of module names for nesting.
fn module_groups(m: &Model) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let path = |start: &crate::model::Module| -> String {
        let mut names = vec![start.name.clone()];
        let mut cur = start;
        for _ in 0..16 {
            match m.modules.iter().find(|p| p.inner == cur.view) {
                Some(p) => {
                    names.push(p.name.clone());
                    cur = p;
                }
                None => break,
            }
        }
        names.reverse();
        names.join(" / ")
    };
    for md in &m.modules {
        let members: Vec<String> = m.vars.iter().filter(|v| v.view == md.inner).map(|v| v.name.clone()).collect();
        if !members.is_empty() {
            out.push((path(md), members));
        }
    }
    for g in &m.groups {
        let members: Vec<String> = g.members.iter().filter_map(|id| m.var(*id)).map(|v| v.name.clone()).collect();
        if members.len() >= 2 {
            out.push((g.name.clone(), members));
        }
    }
    out
}

/// Write a model as XMILE 1.0 text. The report lists everything that could not be carried over.
pub fn write(m: &Model) -> (String, LossReport) {
    use crate::loss::LossKind::{Approximated, Dropped, Preserved};
    let mut rep = LossReport::default();
    crate::userfn::set(&m.functions);
    // sets: real dimensions when they are plain, otherwise one variable per item
    let mut native = false;
    let mut w = m.clone();
    let mut baked: HashSet<String> = HashSet::new();
    if !m.sets.is_empty() {
        if plain_sets(m) {
            native = true;
        } else {
            match crate::sets::expand(m) {
                Ok(ex) => {
                    for v in m.vars.iter().filter(|v| v.set.is_some()) {
                        if let Some(set) = m.sets.iter().find(|s| Some(&s.name) == v.set.as_ref()) {
                            for it in &set.items {
                                baked.insert(expr::normalize(&crate::sets::flat_name(&v.name, it)));
                            }
                        }
                    }
                    w = ex.model;
                    note(
                        &mut rep,
                        crate::loss::LossKind::Flattened,
                        "sets",
                        "the model uses sets with parents or partial totals, which XMILE dimensions cannot express; it was written flattened, one variable per item (named Name[item]). The roll-up totals shown on the diagram are not variables and were not written.",
                    );
                }
                Err(p) => {
                    for (var, msg) in p {
                        note(&mut rep, Dropped, "sets", &format!("could not be flattened ({}{msg}); the set variables were written without their sets", var.map(|v| format!("{v}: ")).unwrap_or_default()));
                    }
                    for v in w.vars.iter_mut() {
                        v.set = None;
                        v.by_item.clear();
                    }
                    w.sets.clear();
                }
            }
        }
    }
    let t0 = w.settings.initial_time;
    let dt = w.settings.time_step;
    let mut wr = Writer { o: Out::new(), ctx: WCtx { rep: &mut rep, helpers: Vec::new(), aux_helpers: Vec::new(), t0, dt }, native };

    // the body first, so that helper graphical functions are known before the header options
    let mut body = Out::new();
    std::mem::swap(&mut wr.o, &mut body);
    wr.o.depth = 2;
    wr.o.open("variables", &[]);
    for v in &w.vars {
        wr.variable(&w, v);
    }
    // graphical functions that stand in for GRAPH / GRAPHSTEP / WITH LOOKUP
    let helpers: Vec<(String, bool, Vec<[f64; 2]>)> = wr.ctx.helpers.iter().map(|h| (h.name.clone(), h.discrete, h.pts.clone())).collect();
    for (name, discrete, pts) in &helpers {
        wr.gf(Some(name), pts, *discrete);
    }
    let aux_helpers: Vec<(String, X)> = wr.ctx.aux_helpers.clone();
    for (name, x) in &aux_helpers {
        wr.o.open("aux", &[("name", name_attr(name))]);
        wr.o.leaf("eqn", &[], &emit(x, Dialect::Xm));
        wr.o.close("aux");
    }
    let groups = module_groups(&w);
    for (name, members) in &groups {
        wr.o.open("group", &[("name", name_attr(name))]);
        for mem in members {
            wr.o.empty("entity", &[("name", name_attr(mem))]);
        }
        wr.o.close("group");
    }
    wr.o.close("variables");
    // views
    let diagrams: Vec<&Sheet> = w.sheets.iter().filter(|s| s.kind == SheetKind::Diagram).collect();
    let first_sheet = diagrams.first().map(|s| s.id).unwrap_or(0);
    let known: HashSet<u64> = diagrams.iter().map(|s| s.id).collect();
    let view_of = |v: &Var| if known.contains(&v.view) { v.view } else { first_sheet };
    let mut n_views = 0;
    let mut uid = 0;
    for sh in diagrams.iter() {
        let vars: Vec<&Var> = w.vars.iter().filter(|v| view_of(v) == sh.id).collect();
        if vars.is_empty() {
            continue;
        }
        if n_views == 0 {
            wr.o.open("views", &[]);
        }
        let max_x = vars.iter().map(|v| v.pos[0]).fold(0.0f32, f32::max) + 120.0;
        let max_y = vars.iter().map(|v| v.pos[1]).fold(0.0f32, f32::max) + 120.0;
        wr.o.open(
            "view",
            &[
                ("type", "stock_flow".into()),
                ("order", n_views.to_string()),
                ("width", f32s(max_x.max(400.0))),
                ("height", f32s(max_y.max(300.0))),
                ("zoom", "100".into()),
                ("page_width", "800".into()),
                ("page_height", "600".into()),
                ("page_sequence", "row".into()),
                ("page_orientation", "landscape".into()),
                ("show_pages", "false".into()),
                ("home_page", "0".into()),
                ("home_view", if n_views == 0 { "true".into() } else { "false".into() }),
            ],
        );
        for v in &vars {
            let xy = [("name", name_attr(&v.name)), ("x", f32s(v.pos[0])), ("y", f32s(v.pos[1]))];
            match v.kind {
                Kind::Stock => {
                    wr.o.open("stock", &xy);
                    wr.o.empty("shape", &[("type", "rectangle".into()), ("width", "60".into()), ("height", "40".into())]);
                    wr.o.close("stock");
                }
                Kind::Flow => {
                    wr.o.open("flow", &xy);
                    let end = |id: Option<u64>, pt: [f32; 2]| id.and_then(|i| w.var(i)).map(|s| s.pos).unwrap_or(pt);
                    let (a, b) = (end(v.from, v.from_pt), end(v.to, v.to_pt));
                    wr.o.open("pts", &[]);
                    wr.o.empty("pt", &[("x", f32s(a[0])), ("y", f32s(a[1]))]);
                    if (a[0] - b[0]).abs() > 0.01 && (a[1] - b[1]).abs() > 0.01 {
                        wr.o.empty("pt", &[("x", f32s(b[0])), ("y", f32s(a[1]))]);
                    }
                    wr.o.empty("pt", &[("x", f32s(b[0])), ("y", f32s(b[1]))]);
                    wr.o.close("pts");
                    wr.o.close("flow");
                }
                _ => {
                    wr.o.open("aux", &xy);
                    wr.o.empty("shape", &[("type", "circle".into()), ("radius", "18".into())]);
                    wr.o.close("aux");
                }
            }
        }
        let on_sheet: HashMap<u64, [f32; 2]> = vars.iter().map(|v| (v.id, v.pos)).collect();
        for l in &w.links {
            let (Some(a), Some(b)) = (on_sheet.get(&l.from), on_sheet.get(&l.to)) else { continue };
            let (Some(fv), Some(tv)) = (w.var(l.from), w.var(l.to)) else { continue };
            uid += 1;
            let mut ang = (-(b[1] - a[1])).atan2(b[0] - a[0]).to_degrees();
            if ang < 0.0 {
                ang += 360.0;
            }
            let mut attrs = vec![("uid", uid.to_string()), ("x", f32s(a[0])), ("y", f32s(a[1])), ("angle", format!("{ang:.1}"))];
            if l.polarity == "+" || l.polarity == "-" {
                attrs.push(("polarity", l.polarity.clone()));
            }
            if l.delay_mark {
                attrs.push(("delay_mark", "true".into()));
            }
            if l.dashed {
                attrs.push(("line_style", "dashed".into()));
            }
            wr.o.open("connector", &attrs);
            wr.o.leaf("from", &[], &name_text(&fv.name));
            wr.o.leaf("to", &[], &name_text(&tv.name));
            wr.o.close("connector");
        }
        wr.o.close("view");
        n_views += 1;
    }
    if n_views > 0 {
        wr.o.close("views");
    }
    // macros: the person's own functions
    let mut macros = Out::new();
    macros.depth = 1;
    for f in &m.functions {
        macros.open("macro", &[("name", name_attr(&f.name))]);
        for p in &f.params {
            macros.leaf("parm", &[], &name_text(p));
        }
        let text = match fc_parse(&f.body) {
            Ok(x) => emit(&to_xm(&x, &mut wr.ctx), Dialect::Xm),
            Err(e) => {
                note(wr.ctx.rep, Dropped, &format!("function {}", f.name), &format!("its formula could not be translated ({e})"));
                f.body.clone()
            }
        };
        macros.leaf("eqn", &[], &text);
        if !f.description.trim().is_empty() {
            macros.leaf("doc", &[], &doc_to_xmile(&f.description));
        }
        macros.close("macro");
    }
    let body_text = std::mem::replace(&mut wr.o, Out::new()).s;
    drop(wr);

    // ---- what XMILE has no place for
    let mut meta = Meta { version: 1, ..Default::default() };
    for v in &w.vars {
        let scale_baked = baked.contains(&expr::normalize(&v.name)) || (native && v.set.is_some());
        let lever_scale = if scale_baked { 1.0 } else { v.lever_scale };
        if v.lever || (lever_scale - 1.0).abs() > 1e-12 || v.rollup == Rollup::Average {
            meta.vars.insert(words(&v.name), VarMeta { lever: v.lever, lever_scale, average: v.rollup == Rollup::Average });
        }
    }
    meta.scenarios = m.scenarios.clone();
    meta.checks = m.checks.clone();
    meta.manifest = m.manifest.clone();
    if m.settings.figure_model {
        note(&mut rep, Dropped, "figure_model setting", "the model is marked as drawn for a figure, so that its unit findings are information; the XMILE file does not carry the mark, and after reading it again the units rules report errors");
    }
    if m.settings.teaching_draft {
        note(&mut rep, Dropped, "teaching_draft setting", "the model is marked as a teaching draft, so that the extreme condition experiments report as information; the XMILE file does not carry the mark");
    }
    let has_meta = !meta.vars.is_empty() || !meta.scenarios.is_empty() || !meta.checks.is_empty() || !meta.manifest.is_empty();
    if has_meta {
        let mut what = Vec::new();
        if !meta.vars.is_empty() {
            what.push("key levers and roll-up settings");
        }
        if !meta.scenarios.is_empty() {
            what.push("scenarios");
        }
        if !meta.checks.is_empty() {
            what.push("model checks");
        }
        if !meta.manifest.is_empty() {
            what.push("the manifest");
        }
        note(&mut rep, Preserved, "Dynamist settings", &format!("{} are kept in a dynamist:meta comment in the file; other tools ignore it", what.join(", ")));
    }
    for (kind, label) in [
        (SheetKind::Table, "tables"),
        (SheetKind::Dashboard, "dashboards"),
        (SheetKind::CausalLoop, "causal loop diagrams"),
        (SheetKind::Optimize, "optimisation setups"),
        (SheetKind::Scenarios, "scenario sheets"),
        (SheetKind::Entities, "entity cards"),
        (SheetKind::Architecture, "architecture views"),
    ] {
        let names: Vec<String> = w.sheets.iter().filter(|s| s.kind == kind).map(|s| s.name.clone()).collect();
        if !names.is_empty() && !(kind == SheetKind::Scenarios && has_meta) {
            note(&mut rep, Dropped, label, &format!("{} are not part of XMILE and were left out ({})", label, names.join(", ")));
        }
    }
    if !w.notes.is_empty() {
        note(&mut rep, Dropped, "text notes", "notes on the diagram were left out");
    }
    if !w.controls.is_empty() {
        note(&mut rep, Dropped, "controls", "sliders, knobs, switches and number inputs on the diagram were left out");
    }
    if !w.shapes.is_empty() {
        note(&mut rep, Dropped, "shapes", "shapes and pictures drawn on the diagram were left out");
    }
    if w.links.iter().any(|l| l.weight != 1 || l.color.is_some() || l.shape.is_some() || l.head.is_some() || !l.via.is_empty() || l.bend != 0.0) {
        note(&mut rep, Approximated, "arrow styles", "only the arrow's ends, sign, delay mark and dashes are kept; curves, weights, colours and turning points are not");
    }
    if !w.modules.is_empty() {
        note(&mut rep, Approximated, "modules", "modules were written as XMILE groups (name and members); their inputs, outputs and purposes are not kept, and nested modules become groups named Outer / Inner");
    }
    if !w.groups.is_empty() && w.modules.is_empty() {
        note(&mut rep, Approximated, "groups", "diagram groups were written as XMILE groups (name and members)");
    }
    if w.settings.method == Method::RungeKutta3 {
        note(&mut rep, Approximated, "integration method", "Runge-Kutta 3 is not in XMILE; it was written as Runge-Kutta 4");
    }
    for v in &w.vars {
        let joined = words(&v.name).replace(' ', "_");
        if is_reserved(&joined) {
            note(&mut rep, Approximated, &format!("name {}", v.name), "is a reserved XMILE word: it is quoted in equations, but other tools may reject it");
        }
    }

    // ---- assemble
    let mut out = Out::new();
    out.s.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.open("xmile", &[("version", "1.0".into()), ("xmlns", "http://docs.oasis-open.org/xmile/ns/XMILE/v1.0".into())]);
    out.open("header", &[]);
    out.leaf("vendor", &[], "Dynamist");
    out.leaf("product", &[("version", "0.1.0".into())], "Dynamist");
    if !m.title.trim().is_empty() {
        out.leaf("name", &[], m.title.trim());
    }
    out.open("options", &[]);
    if native {
        out.empty("uses_arrays", &[("maximum_dimensions", "1".into())]);
    }
    if !m.functions.is_empty() {
        out.empty("uses_macros", &[]);
    }
    if n_views > 0 {
        out.empty("has_model_view", &[]);
    }
    out.close("options");
    out.close("header");
    if has_meta {
        if let Some(c) = meta_comment(&meta) {
            out.comment(&c);
        }
    }
    let method = match w.settings.method {
        Method::Euler => "euler",
        Method::RungeKutta2 => "rk2",
        Method::RungeKutta3 | Method::RungeKutta4 => "rk4",
    };
    let mut specs: Vec<(&str, String)> = vec![("method", method.into())];
    if !w.settings.time_units.trim().is_empty() {
        specs.push(("time_units", name_attr(&w.settings.time_units)));
    }
    out.open("sim_specs", &specs);
    out.leaf("start", &[], &fmt_f64(w.settings.initial_time));
    out.leaf("stop", &[], &fmt_f64(w.settings.final_time));
    out.leaf("dt", &[], &fmt_f64(w.settings.time_step));
    out.close("sim_specs");
    if native {
        out.open("dimensions", &[]);
        for s in &m.sets {
            out.open("dim", &[("name", name_attr(&s.name))]);
            for it in &s.items {
                out.empty("elem", &[("name", name_attr(it))]);
            }
            out.close("dim");
        }
        out.close("dimensions");
    }
    out.open("model", &[]);
    out.s.push_str(&body_text);
    out.close("model");
    out.s.push_str(&macros.s);
    out.close("xmile");
    (out.s, rep)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::sim;

    pub(super) fn run(m: &Model) -> sim::Run {
        crate::userfn::set(&m.functions);
        let c = sim::compile(m).unwrap_or_else(|p| panic!("does not compile: {:?}", p.iter().map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>()));
        sim::simulate(&c, m, "x").unwrap()
    }

    /// Every series of `a` that `b` also has must agree.
    pub(super) fn same(a: &Model, b: &Model, what: &str) -> usize {
        let (ra, rb) = (run(a), run(b));
        assert_eq!(ra.times.len(), rb.times.len(), "{what}: different number of steps");
        let mut n = 0;
        for (k, sa) in &ra.series {
            let Some(sb) = rb.series.get(k) else { continue };
            for (i, (x, y)) in sa.iter().zip(sb).enumerate() {
                assert!((x - y).abs() <= 1e-9 * (1.0 + x.abs()), "{what}: {k} differs at step {i}: {x} vs {y}");
            }
            n += 1;
        }
        n
    }

    pub(super) fn roundtrip(m: &Model) -> (Model, LossReport, LossReport, String) {
        let (text, wrep) = write(m);
        let (back, rrep) = read(&text).unwrap_or_else(|e| panic!("cannot read what was written: {e}\n{text}"));
        (back, wrep, rrep, text)
    }

    pub(super) fn var(m: &mut Model, name: &str, kind: Kind, eq: &str) -> u64 {
        let id = m.new_id();
        let n = m.vars.len() as f32;
        let mut v = Var::new(id, name, kind, [100.0 + 150.0 * (n % 5.0), 100.0 + 120.0 * (n / 5.0).floor()]);
        v.equation = eq.into();
        m.vars.push(v);
        id
    }

    pub(super) fn base(final_time: f64, dt: f64) -> Model {
        let mut m = Model { title: "Test".into(), ..Default::default() };
        m.settings.initial_time = 0.0;
        m.settings.final_time = final_time;
        m.settings.time_step = dt;
        m.settings.time_units = "Day".into();
        m.ensure_sheets();
        m
    }

    // ------------------------------------------------------------------------------- XML

    #[test]
    fn the_xml_reader_handles_entities_cdata_comments_and_prefixes() {
        let n = parse_xml("<?xml version=\"1.0\"?>\n<!-- c --><a:x k=\"1 &lt; 2\" q='&quot;'><y>t &amp; u</y><![CDATA[<raw>]]><!--note--><z/></a:x>").unwrap();
        assert_eq!(n.local(), "x");
        assert_eq!(n.attr("k"), Some("1 < 2"));
        assert_eq!(n.attr("q"), Some("\""));
        assert_eq!(n.child("y").unwrap().text(), "t & u");
        assert_eq!(n.text(), "<raw>");
        assert_eq!(n.comments(), ["note"]);
        assert!(n.child("z").is_some());
        assert!(parse_xml("<a><b></a>").unwrap_err().contains("line 1"));
        assert!(parse_xml("no xml").is_err());
        assert_eq!(unescape("&#65;&#x42;&bogus;&"), "AB&bogus;&");
    }

    #[test]
    fn what_the_writer_writes_the_xml_reader_reads() {
        let m = Model::example();
        let (text, _) = write(&m);
        let root = parse_xml(&text).unwrap();
        assert_eq!(root.local(), "xmile");
        assert_eq!(root.attr("version"), Some("1.0"));
        assert!(root.child("header").is_some() && root.child("sim_specs").is_some());
        assert!(root.child("model").unwrap().child("variables").is_some());
    }

    // -------------------------------------------------------------------------- equations

    fn to_x(fc: &str) -> String {
        let mut rep = LossReport::default();
        let mut c = WCtx { rep: &mut rep, helpers: Vec::new(), aux_helpers: Vec::new(), t0: 0.0, dt: 1.0 };
        emit(&to_xm(&fc_parse(fc).unwrap(), &mut c), Dialect::Xm)
    }

    fn to_f(xm: &str) -> String {
        let mut rep = LossReport::default();
        let (helpers, dims, sets, var_set) = (HashMap::new(), HashSet::new(), Vec::new(), HashMap::new());
        let inits = HashMap::new();
        let mut c = RCtx { rep: &mut rep, helpers: &helpers, dims: &dims, sets: &sets, var_set: &var_set, inits: &inits, prefix: "", t0: 0.0, dt: 1.0 };
        emit(&from_xm(&xm_parse(xm).unwrap(), &mut c), Dialect::Fc)
    }

    #[test]
    fn equations_go_from_dynamist_to_xmile() {
        assert_eq!(to_x("IF THEN ELSE(Stock > 5, 1, 2)"), "IF Stock > 5 THEN 1 ELSE 2");
        assert_eq!(to_x("Contact Rate * Population / 3"), "Contact_Rate * Population / 3");
        assert_eq!(to_x("a && b || !c"), "a AND b OR NOT c");
        assert_eq!(to_x("SMOOTH(x, 3)"), "SMTH1(x, 3)");
        assert_eq!(to_x("SMOOTH3I(x, 3, 0)"), "SMTH3(x, 3, 0)");
        assert_eq!(to_x("DELAY FIXED(x, 4, 0)"), "DELAY(x, 4, 0)");
        assert_eq!(to_x("DELAYINF(x, 6, 3)"), "SMTHN(x, 6, 3)");
        assert_eq!(to_x("DELAYMTR(x, 6, 3)"), "DELAYN(x, 6, 3)");
        assert_eq!(to_x("INITIAL(x)"), "INIT(x)");
        assert_eq!(to_x("Time + Initial Time + Final Time + Time Step"), "TIME + STARTTIME + STOPTIME + DT");
        assert_eq!(to_x("MIN(a, b, c)"), "MIN(a, MIN(b, c))");
        assert_eq!(to_x("a ^ -b"), "a^(-b)");
        assert_eq!(to_x("-a ^ 2"), "-a^2");
        assert_eq!(to_x("(a + b) * c"), "(a + b) * c");
        assert_eq!(to_x("a - (b - c)"), "a - (b - c)");
        assert_eq!(to_x("FLOOR(x)"), "INT(x)");
        assert_eq!(to_x("TOTAL(Stock)"), "SUM(Stock[*])");
        assert_eq!(to_x("Stock[Item One]"), "Stock[Item_One]");
        assert_eq!(to_x("\"Odd name[a - b]\" + 1"), "\"Odd name[a - b]\" + 1");
        // the lookup call and the person's own function pass through
        assert_eq!(to_x("Effect of price(p / 2)"), "Effect_of_price(p / 2)");
    }

    #[test]
    fn equations_go_from_xmile_to_dynamist() {
        assert_eq!(to_f("IF Stock > 5 THEN 1 ELSE 2"), "IF THEN ELSE(Stock > 5, 1, 2)");
        assert_eq!(to_f("Contact_Rate * Population"), "Contact Rate * Population");
        assert_eq!(to_f("a AND b OR NOT c"), "a && b || !c");
        assert_eq!(to_f("SMTH1(x, 3)"), "SMOOTH(x, 3)");
        assert_eq!(to_f("SMTH3(x, 3, 1)"), "SMOOTH3I(x, 3, 1)");
        assert_eq!(to_f("SMTHN(x, 3, 1)"), "SMOOTH(x, 3)");
        assert_eq!(to_f("SMTHN(x, 3, 5)"), "DELAYINF(x, 3, 5)");
        assert_eq!(to_f("DELAYN(x, 3, 3)"), "DELAY3(x, 3)");
        assert_eq!(to_f("DELAY(x, 3)"), "DELAY FIXED(x, 3, INITIAL(x))");
        assert_eq!(to_f("INT(x)"), "FLOOR(x)");
        assert_eq!(to_f("TIME + STARTTIME + STOPTIME + DT + PI"), "TIME + INITIAL TIME + FINAL TIME + DT + PI()");
        assert_eq!(to_f("x MOD 3"), "MODULO(x, 3)");
        assert_eq!(to_f("PULSE(20, 12, 5)"), "20 / DT * PULSE TRAIN(12, DT, 5, 1e300)");
        assert_eq!(to_f("PULSE(20, 12)"), "20 / DT * PULSE(12, DT)");
        assert_eq!(to_f("RAMP(2, 5)"), "RAMP(2, 5, FINAL TIME)");
        assert_eq!(to_f("TREND(x, 5)"), "TREND(x, 5, 0)");
        assert_eq!(to_f("FORCST(x, 5, 10)"), "x * (1 + TREND(x, 5, 0) * 10)");
        assert_eq!(to_f("MEAN(a, b)"), "MEAN(a, b)");
        assert_eq!(to_f("a {a comment} + 1e3"), "a + 1000");
        assert_eq!(to_f("x ^ 2 ^ 3"), "x^2^3");
        assert_eq!(to_f("(1 + 2) ^ 2"), "(1 + 2)^2");
        assert_eq!(to_f("\"my var\" * 2"), "my var * 2");
        assert_eq!(to_f("\"a-b\" * 2"), "\"a-b\" * 2");
    }

    #[test]
    fn precedence_survives_both_ways() {
        for fc in ["a - (b - c)", "a / (b * c)", "-(a + b)", "(a + b)^2", "a^b^c", "(a^b)^c", "!(a = b)", "(a < b) = (c < d)", "a * (IF THEN ELSE(x > 1, 2, 3) + 1)", "-a^2", "(-a)^2"] {
            let once = to_x(fc);
            let back = to_f(&once);
            let again = to_x(&back);
            assert_eq!(once, again, "{fc}: {once} → {back}");
        }
    }
}

#[cfg(test)]
mod more_tests {
    use super::tests::*;
    use super::*;

    #[test]
    fn the_starter_model_survives_a_trip_through_xmile() {
        let m = Model::example();
        let (back, wrep, _rrep, text) = roundtrip(&m);
        assert_eq!(back.vars.len(), m.vars.len(), "{}", wrep.summary());
        assert_eq!(back.title, m.title);
        let n = same(&m, &back, "example");
        assert_eq!(n, run(&m).series.len(), "every series must come back");
        // units, notes, slider ranges, lookups and flow ends survive
        for v in &m.vars {
            let b = back.by_name(&expr::normalize(&v.name)).unwrap_or_else(|| panic!("{} is gone", v.name));
            assert_eq!(b.kind, v.kind, "{}", v.name);
            assert_eq!(b.units, v.units, "{}", v.name);
            assert_eq!(b.comment, v.comment, "{}", v.name);
            assert_eq!(b.slider, v.slider, "{}", v.name);
            assert_eq!(b.table.len(), v.table.len(), "{}", v.name);
            assert_eq!(b.non_negative, v.non_negative, "{}", v.name);
            let end = |m: &Model, id: Option<u64>| id.and_then(|i| m.var(i)).map(|s| s.name.clone());
            assert_eq!(end(&back, b.from), end(&m, v.from), "{} from", v.name);
            assert_eq!(end(&back, b.to), end(&m, v.to), "{} to", v.name);
        }
        assert!(text.contains("<stock name="));
    }

    /// One formula per built-in that XMILE lacks or defines differently: each must come back with the same values.
    #[test]
    fn every_builtin_gives_the_same_values_after_a_trip() {
        let mut m = base(20.0, 0.25);
        let x = var(&mut m, "x", Kind::Aux, "Time * 0.7 - 5");
        let _ = x;
        var(&mut m, "y", Kind::Aux, "SINWAVE(3, 8, 1) + 2");
        var(&mut m, "p", Kind::Constant, "2.5");
        let cases: &[(&str, &str)] = &[
            ("if", "IF THEN ELSE(x > 0 && y < 4 || !(p = 2.5), x, -x)"),
            ("min3", "MIN(x, y, p)"),
            ("max3", "MAX(x, y, p, 1)"),
            ("abs", "ABS(x)"),
            ("sqrt", "SQRT(ABS(x))"),
            ("exp", "EXP(x / 10)"),
            ("ln", "LN(ABS(x) + 1)"),
            ("log", "LOG(ABS(x) + 2, 3)"),
            ("log10", "LOG10(ABS(x) + 1)"),
            ("log2", "LOG2(ABS(x) + 1)"),
            ("power", "POWER(ABS(x), 1.5)"),
            ("caret", "x ^ 2 - (-x) ^ 2 + 2 ^ 3 ^ 2 / 100"),
            ("integer", "INTEGER(x)"),
            ("round", "ROUND(x)"),
            ("round_half", "ROUND(x + 0.5)"),
            ("floor", "FLOOR(x)"),
            ("ceiling", "CEILING(x)"),
            ("modulo", "MODULO(x, 3)"),
            ("sign", "SIGN(x)"),
            ("sin", "SIN(x) + COS(x) + TAN(x / 10)"),
            ("arc", "ARCTAN(x) + ARCSIN(x / 20) + ARCCOS(x / 20)"),
            ("atan2", "ARCTAN2(x, y - 2) + ARCTAN2(-x, -(y - 2)) + ARCTAN2(x, 0) + ARCTAN2(0, x)"),
            ("hyper", "SINH(x / 4) + COSH(x / 4) + TANH(x / 4)"),
            ("cbrt", "CBRT(x)"),
            ("hypot", "HYPOT(x, y)"),
            ("angles", "RADIANS(x * 10) + DEGREES(x / 10)"),
            ("clamp", "CLAMP(x, -2, 3) + CLAMP(x, 3, -2)"),
            ("mean", "MEAN(x, y, p)"),
            ("median2", "MEDIAN(x, y)"),
            ("median3", "MEDIAN(x, y, p)"),
            ("pi", "PI() * x"),
            ("xidz", "XIDZ(x, y - 2, 99) + ZIDZ(x, y - 2)"),
            ("sinwave", "SINWAVE(2, 5) + COSWAVE(3, 7, 2)"),
            ("graph", "GRAPH(x, -5, 5, 1, 4, 2, 8)"),
            ("graphstep", "GRAPHSTEP(x, -5, 5, 1, 4, 2, 8)"),
            ("lookup", "WITH LOOKUP(x, ((-5, 0), (0, 1), (3, 5), (9, 2)))"),
            ("delayinf", "DELAYINF(x, 3, 2)"),
            ("delaymtr", "DELAYMTR(x, 3, 2)"),
            ("smooth", "SMOOTH(x, 2) + SMOOTHI(x, 2, 7) + SMOOTH3(x, 2) + SMOOTH3I(x, 2, 7)"),
            ("delay", "DELAY1(x, 2) + DELAY1I(x, 2, 7) + DELAY3(x, 2) + DELAY3I(x, 2, 7)"),
            ("fixed", "DELAY FIXED(x, 2, 1)"),
            ("trend", "TREND(x + 20, 4, 0.1)"),
            ("initial", "INITIAL(x + y)"),
            ("step", "STEP(5, 4) + STEP(2, 10.5)"),
            ("pulse", "PULSE(3, 2) + PULSE(10, 0.5)"),
            ("pulsetrain", "PULSE TRAIN(2, 1, 5, 17) + PULSE TRAIN(0, 0.25, 3, 100)"),
            ("ramp", "RAMP(0.5, 3, 12)"),
            ("steady", "Time + Initial Time + Final Time + Time Step"),
        ];
        for (name, eq) in cases {
            var(&mut m, &format!("v {name}"), Kind::Aux, eq);
        }
        let (back, wrep, rrep, text) = roundtrip(&m);
        let n = same(&m, &back, "builtins");
        assert!(n >= cases.len(), "compared {n} series");
        // the only things reported are the step at 10.5, which is off the time grid by 0 here (0.25 steps) …
        assert!(wrep.items.iter().all(|i| i.construct != "MODULO"), "{}", wrep.summary());
        // writing reports only the two delay chains; reading reports only that INT rounds down
        assert!(wrep.items.iter().all(|i| i.construct == "function DELAYINF" || i.construct == "function DELAYMTR"), "{}", wrep.summary());
        assert!(rrep.items.iter().all(|i| i.construct == "INT"), "{}", rrep.summary());
        // INIT takes a variable: an expression goes into an auxiliary of its own
        assert!(text.contains("INIT(fc_init_1)") && !text.contains("INIT(x"), "{text}");
        assert_eq!(back.vars.len(), m.vars.len(), "helper auxiliaries are not variables");
        // XMILE text uses only XMILE words
        for forbidden in ["SMOOTH(", "DELAY1I", "SINWAVE", "GRAPH(", "WITH LOOKUP", "INTEGER(", "DELAYINF", "MODULO", "ARCTAN2", "PULSE TRAIN"] {
            assert!(!text.contains(forbidden), "{forbidden} leaked into the XMILE file");
        }
    }

    #[test]
    fn functions_with_no_xmile_form_are_kept_and_reported() {
        let mut m = base(5.0, 1.0);
        m.settings.method = Method::RungeKutta3;
        var(&mut m, "g", Kind::Aux, "GAMMA(Time + 1) + ERF(0.5) + FACTORIAL(3) + MEDIAN(1, 2, 3, 4, 9)");
        var(&mut m, "r", Kind::Aux, "RANDOM UNIFORM(0, 1, 7) + RANDOM NORMAL(-9, 9, 0, 1, 7)");
        var(&mut m, "s", Kind::Aux, "STEP(1, 2.3)");
        var(&mut m, "q", Kind::Aux, "MODULO(Time, Time - 2.5)");
        let (text, wrep) = write(&m);
        let has = |c: &str| wrep.items.iter().any(|i| i.construct.contains(c));
        for c in ["function GAMMA", "function ERF", "function FACTORIAL", "function MEDIAN", "RANDOM UNIFORM", "RANDOM NORMAL", "STEP", "MODULO", "integration method"] {
            assert!(has(c), "missing {c} in\n{}", wrep.summary());
        }
        assert!(!wrep.is_lossless());
        assert!(text.contains("method=\"rk4\""));
        // Dynamist can still read the kept calls back
        let (back, _) = read(&text).unwrap();
        assert!(back.by_name("g").unwrap().equation.contains("GAMMA"));
        assert!(back.by_name("r").unwrap().equation.contains("RANDOM"));
    }
}

#[cfg(test)]
mod scenario_tests {
    use super::tests::*;
    use super::*;
    use crate::loss::LossKind;

    fn series(m: &Model, name: &str) -> Vec<f64> {
        run(m).get(name).unwrap_or_else(|| panic!("no series {name}")).clone()
    }

    fn at(m: &Model, name: &str, t: f64) -> f64 {
        let r = run(m);
        let i = r.times.iter().position(|x| (*x - t).abs() < 1e-9).unwrap_or_else(|| panic!("no time {t}"));
        r.get(name).unwrap_or_else(|| panic!("no series {name}"))[i]
    }

    fn pump_model() -> Model {
        let mut m = base(20.0, 0.5);
        m.title = "Pumps".into();
        let tank = var(&mut m, "Water Tank", Kind::Stock, "50");
        let sump = var(&mut m, "Sump", Kind::Stock, "5");
        let pump = var(&mut m, "Pump Rate", Kind::Flow, "MIN(Capacity, Water Tank / 4)");
        let leak = var(&mut m, "Leak", Kind::Flow, "Water Tank * 0.02");
        let rain = var(&mut m, "Rain", Kind::Flow, "PULSE(3, 2) * 12 + 1");
        let cap = var(&mut m, "Capacity", Kind::Constant, "6");
        let curve = var(&mut m, "Efficiency", Kind::Lookup, "");
        m.var_mut(curve).unwrap().table = vec![[0.0, 0.2], [10.0, 0.6], [40.0, 1.0]];
        let eff = var(&mut m, "Effective", Kind::Aux, "Efficiency(Water Tank) * Pump Rate");
        m.var_mut(tank).unwrap().non_negative = true;
        m.var_mut(tank).unwrap().units = "Litre".into();
        m.var_mut(tank).unwrap().comment = "The tank.\nSecond line with a \\ backslash.".into();
        m.var_mut(cap).unwrap().slider = Some([1.0, 20.0]);
        m.var_mut(cap).unwrap().units = "Litre/Day".into();
        m.var_mut(pump).unwrap().from = Some(tank);
        m.var_mut(pump).unwrap().to = Some(sump);
        m.var_mut(leak).unwrap().from = Some(tank);
        m.var_mut(rain).unwrap().to = Some(tank);
        let _ = eff;
        for (a, b) in [(cap, pump), (tank, pump), (tank, leak), (curve, eff), (pump, eff), (tank, eff)] {
            m.add_link(a, b);
        }
        m.links[0].polarity = "+".into();
        m.links[1].delay_mark = true;
        m
    }

    #[test]
    fn stocks_flows_clouds_lookups_and_the_diagram_survive() {
        let m = pump_model();
        let (back, wrep, rrep, text) = roundtrip(&m);
        assert!(wrep.items.is_empty(), "{}", wrep.summary());
        assert!(rrep.items.is_empty(), "{}", rrep.summary());
        same(&m, &back, "pumps");
        let get = |n: &str| back.by_name(&expr::normalize(n)).unwrap_or_else(|| panic!("{n} missing"));
        let tank = get("Water Tank");
        assert!(tank.non_negative && tank.kind == Kind::Stock);
        assert_eq!(tank.units, "Litre");
        assert_eq!(tank.comment, "The tank.\nSecond line with a \\ backslash.");
        assert_eq!(get("Capacity").slider, Some([1.0, 20.0]));
        assert_eq!(get("Capacity").kind, Kind::Constant);
        assert_eq!(get("Efficiency").kind, Kind::Lookup);
        assert!(get("Efficiency").equation.is_empty());
        assert_eq!(get("Efficiency").table, vec![[0.0, 0.2], [10.0, 0.6], [40.0, 1.0]]);
        // flow ends
        assert_eq!(get("Pump Rate").from, Some(tank.id));
        assert_eq!(get("Pump Rate").to, Some(get("Sump").id));
        assert_eq!(get("Rain").from, None);
        assert_eq!(get("Rain").to, Some(tank.id));
        // positions and arrows
        for v in &m.vars {
            let b = get(&v.name);
            assert_eq!(b.pos, v.pos, "{} position", v.name);
        }
        assert_eq!(back.links.len(), m.links.len());
        assert!(back.links.iter().any(|l| l.polarity == "+") && back.links.iter().any(|l| l.delay_mark));
        // the file says what it should
        assert!(text.contains("<inflow>Rain</inflow>") && text.contains("<outflow>Pump_Rate</outflow>"), "{text}");
        assert!(text.contains("<non_negative/>") && text.contains("<range min=\"1\" max=\"20\"/>"));
        assert!(text.contains("<units>Litre/Day</units>"));
    }

    #[test]
    fn a_plain_model_has_a_clean_loss_report() {
        let (_, wrep, rrep, _) = roundtrip(&pump_model());
        assert!(wrep.is_lossless() && wrep.items.is_empty());
        assert!(rrep.is_lossless() && rrep.items.is_empty());
        assert_eq!(wrep.summary(), "No loss.");
    }

    // ---------------------------------------------------------------------------- sets

    fn tank_set(m: &mut Model, parent: bool) {
        let id = m.new_id();
        m.sets.push(ItemSet { id, name: "Tank".into(), items: vec!["Aa".into(), "Bb".into(), "Cc Dd".into()], parent: parent.then(|| "Region".to_string()), parent_of: if parent { vec!["North".into(), "North".into(), "South".into()] } else { vec![] }, note: String::new() });
    }

    fn per(m: &mut Model, id: u64) {
        m.var_mut(id).unwrap().set = Some("Tank".into());
    }

    #[test]
    fn plain_sets_become_xmile_dimensions_and_come_back() {
        let mut m = base(10.0, 0.5);
        tank_set(&mut m, false);
        let level = var(&mut m, "Level", Kind::Stock, "10");
        let burn = var(&mut m, "Burn", Kind::Flow, "Level * Rate");
        let rate = var(&mut m, "Rate", Kind::Constant, "0.1");
        let total = var(&mut m, "Total level", Kind::Aux, "TOTAL(Level)");
        let mean = var(&mut m, "Mean level", Kind::Aux, "AVERAGE(Level) + Level[Bb] * 0");
        let share = var(&mut m, "Share", Kind::Aux, "Level / Total level");
        for id in [level, burn, rate, share] {
            per(&mut m, id);
        }
        m.var_mut(burn).unwrap().from = Some(level);
        m.var_mut(level).unwrap().by_item = vec![ItemValue { item: "Bb".into(), equation: "20".into(), table: vec![] }];
        m.var_mut(rate).unwrap().by_item = vec![ItemValue { item: "Cc Dd".into(), equation: "0.3".into(), table: vec![] }];
        let _ = (total, mean);
        let (back, wrep, _rrep, text) = roundtrip(&m);
        assert!(text.contains("<uses_arrays maximum_dimensions=\"1\"/>"), "{text}");
        assert!(text.contains("<dim name=\"Tank\">") && text.contains("<elem name=\"Cc Dd\"/>"), "{text}");
        assert!(text.contains("<element subscript=\"Bb\">"));
        assert!(text.contains("SUM(Level[*])") && text.contains("MEAN(Level[*])"));
        assert!(wrep.items.is_empty(), "{}", wrep.summary());
        // the sets and the per-item values come back
        assert_eq!(back.sets.len(), 1);
        assert_eq!(back.sets[0].items, ["Aa", "Bb", "Cc Dd"]);
        let lv = back.by_name("level").unwrap();
        assert_eq!(lv.set.as_deref(), Some("Tank"));
        assert!(back.by_name("rate").unwrap().by_item.iter().any(|o| o.item == "Cc Dd" && o.equation == "0.3"));
        let n = same(&m, &back, "plain sets");
        assert!(n >= 10, "compared {n}");
        assert!((at(&back, "level[bb]", 0.0) - 20.0).abs() < 1e-12);
        assert!(series(&back, "total level")[0] > 39.0);
    }

    #[test]
    fn sets_with_parents_are_written_flattened_and_reported() {
        let mut m = base(6.0, 1.0);
        let id = m.new_id();
        m.sets.push(ItemSet { id, name: "Region".into(), items: vec!["North".into(), "South".into()], parent: None, parent_of: vec![], note: String::new() });
        tank_set(&mut m, true);
        let cap = var(&mut m, "Cap", Kind::Constant, "10");
        m.var_mut(cap).unwrap().set = Some("Region".into());
        m.var_mut(cap).unwrap().by_item = vec![ItemValue { item: "South".into(), equation: "40".into(), table: vec![] }];
        m.var_mut(cap).unwrap().lever = true;
        m.var_mut(cap).unwrap().lever_scale = 1.5;
        let fill = var(&mut m, "Fill", Kind::Aux, "Cap * 2 + Time");
        per(&mut m, fill);
        var(&mut m, "North fill", Kind::Aux, "TOTAL(Fill, North)");
        var(&mut m, "All fill", Kind::Aux, "TOTAL(Fill)");
        let (back, wrep, _rrep, text) = roundtrip(&m);
        assert!(wrep.items.iter().any(|i| i.construct == "sets" && i.kind == LossKind::Flattened), "{}", wrep.summary());
        assert!(!text.contains("<dimensions"), "flattened files have no dimensions");
        assert!(back.sets.is_empty());
        let n = same(&m, &back, "flattened sets");
        assert!(n >= 6, "compared {n}");
        // North has tanks Aa and Bb: 2 * (10 * 1.5 * 2 + t); the lever scale was multiplied into the values
        assert!((at(&back, "north fill", 2.0) - 2.0 * (30.0 + 2.0)).abs() < 1e-9);
        assert!((at(&back, "all fill", 2.0) - (2.0 * (30.0 + 2.0) + (120.0 + 2.0))).abs() < 1e-9);
    }

    // ----------------------------------------------------------------------- reading files

    const RABBITS: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<xmile version="1.0" xmlns="http://docs.oasis-open.org/xmile/ns/XMILE/v1.0">
  <header><vendor>Test</vendor><product version="1.0">T</product><name>Rabbits</name></header>
  <sim_specs method="Euler, rk4" time_units="Months"><start>0</start><stop>10</stop><dt reciprocal="true">4</dt></sim_specs>
  <model>
    <variables>
      <stock name="Rabbit_Population">
        <eqn>100</eqn><inflow>births</inflow><outflow>deaths</outflow><non_negative/>
        <units>rabbits</units><doc>All the rabbits.</doc>
      </stock>
      <flow name="births"><eqn>Rabbit_Population*birth_rate*Food_Effect {a comment}</eqn><units>rabbits/month</units></flow>
      <flow name="deaths"><eqn>IF Rabbit_Population &gt; 150 THEN Rabbit_Population / lifetime ELSE 0</eqn></flow>
      <aux name="birth_rate"><eqn>0.05</eqn><range min="0" max="0.2"/></aux>
      <aux name="lifetime"><eqn>20</eqn></aux>
      <aux name="Food Effect"><eqn>Rabbit_Population / 200</eqn><gf><xscale min="0" max="1"/><ypts>1,0.8,0.5,0.2,0</ypts></gf></aux>
      <aux name="Smoothed"><eqn>SMTH1(Rabbit_Population, 3)</eqn></aux>
      <aux name="Shock"><eqn>STEP(10, 5)</eqn></aux>
      <gf name="Curve"><xpts>0,1,3</xpts><ypts>0,2,2</ypts></gf>
      <aux name="Use_Curve"><eqn>Curve(Rabbit_Population/100) + "odd name!"</eqn></aux>
      <aux name="odd name!"><eqn>1</eqn></aux>
    </variables>
    <views>
      <view type="stock_flow" order="0" width="800" height="600">
        <stock name="Rabbit_Population" x="300" y="200" width="60" height="40"/>
        <flow name="births" x="180" y="200"><pts><pt x="100" y="200"/><pt x="270" y="200"/></pts></flow>
        <aux name="birth_rate"><shape type="circle" radius="18"/></aux>
        <aux name="Food Effect" x="150" y="80"><shape type="circle" radius="18"/></aux>
        <connector uid="1" angle="45" polarity="+"><from>Rabbit_Population</from><to>births</to></connector>
        <connector uid="2" angle="10"><from>Food_Effect</from><to>births</to></connector>
      </view>
    </views>
  </model>
</xmile>"#;

    #[test]
    fn a_hand_written_xmile_file_reads_with_names_equations_and_diagram() {
        let (m, rep) = read(RABBITS).unwrap();
        assert_eq!(m.title, "Rabbits");
        assert_eq!(m.settings.time_step, 0.25);
        assert_eq!(m.settings.final_time, 10.0);
        assert_eq!(m.settings.time_units, "Months");
        assert_eq!(m.settings.method, Method::Euler);
        let get = |n: &str| m.by_name(&expr::normalize(n)).unwrap_or_else(|| panic!("{n} missing"));
        let pop = get("Rabbit Population");
        assert_eq!((pop.kind, pop.equation.as_str(), pop.units.as_str(), pop.comment.as_str(), pop.non_negative), (Kind::Stock, "100", "rabbits", "All the rabbits.", true));
        assert_eq!(get("births").equation, "Rabbit Population * birth rate * Food Effect");
        assert_eq!(get("births").to, Some(pop.id));
        assert_eq!(get("deaths").from, Some(pop.id));
        assert_eq!(get("deaths").equation, "IF THEN ELSE(Rabbit Population > 150, Rabbit Population / lifetime, 0)");
        assert_eq!(get("birth rate").kind, Kind::Constant);
        assert_eq!(get("birth rate").slider, Some([0.0, 0.2]));
        let fe = get("Food Effect");
        assert_eq!((fe.kind, fe.equation.as_str(), fe.table.len()), (Kind::Lookup, "Rabbit Population / 200", 5));
        assert_eq!(fe.table[2], [0.5, 0.5]);
        assert_eq!(get("Smoothed").equation, "SMOOTH(Rabbit Population, 3)");
        assert_eq!(get("Shock").equation, "STEP(10, 5)");
        assert_eq!(get("Curve").kind, Kind::Lookup);
        assert!(get("Curve").equation.is_empty() && get("Curve").table == vec![[0.0, 0.0], [1.0, 2.0], [3.0, 2.0]]);
        assert_eq!(get("Use Curve").equation, "Curve(Rabbit Population / 100) + \"odd name!\"");
        // the diagram
        assert_eq!(pop.pos, [330.0, 220.0], "a stock with width and height is placed by its top-left corner");
        assert_eq!(get("births").pos, [180.0, 200.0]);
        assert_eq!(get("births").from_pt, [100.0, 200.0]);
        assert_eq!(get("Food Effect").pos, [150.0, 80.0]);
        assert!(m.links.iter().any(|l| l.from == pop.id && l.to == get("births").id && l.polarity == "+"));
        // the run starts as the numbers say: births = 100 * 0.05 * f(0.5) = 2.5 per month
        assert!((at(&m, "Rabbit Population", 0.25) - 100.625).abs() < 1e-9);
        assert!((at(&m, "Shock", 4.75) - 0.0).abs() < 1e-12 && (at(&m, "Shock", 5.0) - 10.0).abs() < 1e-12);
        assert!(rep.items.is_empty(), "{}", rep.summary());
    }

    #[test]
    fn arrays_in_a_file_become_sets_and_unsupported_shapes_are_reported() {
        let xml = r#"<xmile version="1.0" xmlns="http://docs.oasis-open.org/xmile/ns/XMILE/v1.0">
<header><options><uses_arrays/></options></header>
<sim_specs><start>0</start><stop>2</stop></sim_specs>
<dimensions><dim name="Location"><elem name="Boston"/><elem name="Chicago"/></dim><dim name="N" size="3"/></dimensions>
<model><variables>
  <aux name="Base"><dimensions><dim name="Location"/></dimensions>
     <element subscript="Boston"><eqn>10</eqn></element><element subscript="Chicago"><eqn>20</eqn></element></aux>
  <aux name="Scaled"><dimensions><dim name="Location"/></dimensions><eqn>Base[Location] * 2 + TIME</eqn></aux>
  <aux name="Plain"><dimensions><dim name="N"/></dimensions><eqn>7</eqn></aux>
  <aux name="Sum all"><eqn>SUM(Scaled[*])</eqn></aux>
  <aux name="Pick"><eqn>Base[Chicago] + Base[1]</eqn></aux>
  <aux name="Grid"><dimensions><dim name="Location"/><dim name="N"/></dimensions><eqn>1</eqn></aux>
</variables></model></xmile>"#;
        let (m, rep) = read(xml).unwrap();
        assert_eq!(m.sets.iter().map(|s| (s.name.as_str(), s.items.len())).collect::<Vec<_>>(), [("Location", 2), ("N", 3)]);
        assert_eq!(m.by_name("scaled").unwrap().set.as_deref(), Some("Location"));
        assert_eq!(m.by_name("base").unwrap().by_item.len(), 2);
        assert_eq!(m.by_name("sum all").unwrap().equation, "TOTAL(Scaled)");
        assert_eq!(m.by_name("pick").unwrap().equation, "Base[Chicago] + Base[Boston]");
        assert_eq!(at(&m, "scaled[chicago]", 1.0), 41.0);
        assert_eq!(at(&m, "sum all", 1.0), 62.0);
        assert_eq!(at(&m, "pick", 0.0), 30.0);
        // a two-dimensional array becomes one variable per combination, and the report says so
        assert!(m.by_name("grid").is_none());
        assert_eq!(at(&m, "grid[chicago,3]", 0.0), 1.0);
        assert!(rep.items.iter().any(|i| i.kind == LossKind::Approximated && i.construct.contains("Grid")), "{}", rep.summary());
    }

    #[test]
    fn submodels_are_flattened_with_their_connections() {
        let xml = r#"<xmile version="1.0" xmlns="http://docs.oasis-open.org/xmile/ns/XMILE/v1.0">
<header><options><uses_submodels/></options></header>
<sim_specs method="Euler" time_units="Months"><start>0</start><stop>5</stop><dt>1</dt></sim_specs>
<model><variables>
  <module name="Sub_Model"><connect to="Input" from=".Root_Out"/><connect to=".Root_In" from="Sub_Model.Output"/></module>
  <aux name="Root_Out"><eqn>TIME * 2</eqn></aux>
  <aux name="Root_In" access="input"><eqn>0</eqn></aux>
  <aux name="Result"><eqn>Root_In + Sub_Model.Output</eqn></aux>
</variables></model>
<model name="Sub_Model"><variables>
  <aux name="Input" access="input"/>
  <aux name="Output" access="output"><eqn>Input + 1</eqn></aux>
</variables></model></xmile>"#;
        let (m, rep) = read(xml).unwrap();
        assert!(m.by_name("sub model.output").is_some(), "{:?}", m.vars.iter().map(|v| &v.name).collect::<Vec<_>>());
        assert_eq!(at(&m, "Result", 3.0), 14.0);
        assert!(rep.items.iter().any(|i| i.construct == "submodels" && i.kind == LossKind::Flattened));
    }

    #[test]
    fn macros_and_the_persons_functions_travel_both_ways() {
        let mut m = base(6.0, 1.0);
        m.functions = vec![crate::userfn::UserFn { name: "Effect of price".into(), params: vec!["p".into()], body: "1 - 0.3 * LN(p)".into(), description: "Demand falls with price".into() }];
        var(&mut m, "Price", Kind::Constant, "4");
        var(&mut m, "Demand", Kind::Aux, "100 * Effect of price(Price / 2) + Time");
        let (back, wrep, rrep, text) = roundtrip(&m);
        assert!(text.contains("<macro name=\"Effect of price\">") && text.contains("Effect_of_price(Price / 2)"), "{text}");
        assert!(text.contains("<uses_macros/>"));
        assert_eq!(back.functions.len(), 1);
        assert_eq!((back.functions[0].name.as_str(), back.functions[0].params.as_slice(), back.functions[0].description.as_str()), ("Effect of price", &["p".to_string()][..], "Demand falls with price"));
        assert!(wrep.items.is_empty() && rrep.items.is_empty(), "{} / {}", wrep.summary(), rrep.summary());
        same(&m, &back, "macros");
    }

    #[test]
    fn what_only_dynamist_knows_is_kept_in_a_comment() {
        let mut m = pump_model();
        let cap = m.by_name("capacity").unwrap().id;
        m.var_mut(cap).unwrap().lever = true;
        let sc = crate::model::Scenario { id: 77, name: "Big pump".into(), color: None, enabled: true, changes: vec![crate::model::Change { var: "Capacity".into(), value: "12".into() }], notes: String::new() };
        m.scenarios.push(sc);
        m.checks.push(crate::checks::Check { name: "Never dry".into(), var: "Water Tank".into(), min: Some(0.0), max: None, when: crate::checks::CheckWhen::Always, ..Default::default() });
        let tank = m.by_name("water tank").unwrap().id;
        m.var_mut(tank).unwrap().rollup = Rollup::Average;
        let (back, wrep, rrep, text) = roundtrip(&m);
        assert!(text.contains("<!--dynamist:meta "), "{text}");
        assert!(!text.contains("--\\") && !text.replace("<!--", "").replace("-->", "").contains("--"));
        assert!(back.by_name("capacity").unwrap().lever);
        assert_eq!(back.by_name("water tank").unwrap().rollup, Rollup::Average);
        assert_eq!(back.scenarios.len(), 1);
        assert_eq!(back.scenarios[0].name, "Big pump");
        assert_eq!(back.scenarios[0].changes[0].value, "12");
        assert_eq!(back.checks, m.checks);
        assert!(wrep.items.iter().any(|i| i.kind == LossKind::Preserved && i.construct == "Dynamist settings"), "{}", wrep.summary());
        assert!(rrep.items.iter().any(|i| i.kind == LossKind::Preserved));
        assert!(wrep.is_lossless(), "preserved settings are not loss: {}", wrep.summary());
    }

    #[test]
    fn modules_are_written_as_groups_and_the_rest_is_reported() {
        let mut m = pump_model();
        let inner = m.new_id();
        let mut sh = Sheet::new(inner, "Pumping", SheetKind::Diagram);
        let mid = m.new_id();
        sh.module = Some(mid);
        m.sheets.push(sh);
        let first = m.first_diagram();
        m.modules.push(crate::model::Module { id: mid, name: "Pumping".into(), view: first, inner, pos: [0.0, 0.0], description: "moves water".into(), collapsed: false, layer: None });
        for n in ["Pump Rate", "Leak"] {
            let id = m.by_name(&expr::normalize(n)).unwrap().id;
            m.var_mut(id).unwrap().view = inner;
        }
        let dash = m.new_id();
        m.sheets.push(Sheet::new(dash, "Overview", SheetKind::Dashboard));
        let (back, wrep, _rrep, text) = roundtrip(&m);
        assert!(text.contains("<group name=\"Pumping\">") && text.contains("<entity name=\"Leak\"/>"), "{text}");
        assert_eq!(back.groups.len(), 1);
        assert_eq!(back.groups[0].members.len(), 2);
        let has = |c: &str, k: LossKind| wrep.items.iter().any(|i| i.construct == c && i.kind == k);
        assert!(has("modules", LossKind::Approximated), "{}", wrep.summary());
        assert!(has("dashboards", LossKind::Dropped), "{}", wrep.summary());
        // both diagrams are written as views, and the model still runs the same
        assert!(text.matches("<view ").count() == 2, "{text}");
        same(&m, &back, "modules");
        assert_eq!(back.sheets.iter().filter(|s| s.kind == SheetKind::Diagram).count(), 2);
    }

    #[test]
    fn bad_files_are_refused_with_a_reason() {
        for (text, why) in [
            ("hello", "XML"),
            ("<a/>", "not an XMILE file"),
            ("<xmile><header/></xmile>", "no <model>"),
            ("<xmile><model><variables/></model></xmile>", "sim_specs"),
            ("<xmile><sim_specs><start>5</start><stop>1</stop></sim_specs><model/></xmile>", "not valid"),
            ("<xmile><sim_specs><start>0</start><stop>1</stop><dt>0</dt></sim_specs><model/></xmile>", "not valid"),
            ("<xmile><sim_specs><start>x</start><stop>1</stop></sim_specs><model/></xmile>", "start time"),
        ] {
            let e = read(text).unwrap_err();
            assert!(e.contains(why), "{text}: {e}");
        }
    }

    #[test]
    fn unknown_things_in_a_file_are_reported_not_ignored() {
        let xml = r#"<xmile version="1.0"><sim_specs method="gear"><start>0</start><stop>3</stop></sim_specs>
<model><variables>
  <stock name="Queue"><eqn>0</eqn><queue/><inflow>arrive</inflow></stock>
  <flow name="arrive"><eqn>NORMAL(1, 0.1, 3) + PREVIOUS(SELF, 0)</eqn><non_negative/></flow>
  <aux name="Hmm"><eqn>(1 +</eqn></aux>
  <aux name="Inf"><eqn>INF + 1 MOD (0 - 3)</eqn></aux>
  <vendor:thing xmlns:vendor="x" name="odd"/>
  <aux name="Twice"><eqn>1</eqn></aux><aux name="twice"><eqn>2</eqn></aux>
</variables></model></xmile>"#;
        let (m, rep) = read(xml).unwrap();
        let has = |c: &str| rep.items.iter().any(|i| i.construct.contains(c));
        for c in ["integration method", "queue", "non-negative flow", "NORMAL", "PREVIOUS", "equation of Hmm", "INF", "MOD", "thing", "variable twice"] {
            assert!(has(c), "missing {c}:\n{}", rep.summary());
        }
        assert_eq!(m.settings.method, Method::RungeKutta4);
        // an unreadable equation is kept as written (the model then refuses to run), not replaced by 0
        assert_eq!(m.by_name("hmm").unwrap().equation, "(1 +");
        assert!(m.by_name("twice").is_some() && m.vars.iter().filter(|v| expr::normalize(&v.name) == "twice").count() == 1);
    }
}

#[cfg(test)]
mod name_tests {
    use super::tests::*;
    use super::*;

    #[test]
    fn names_with_dots_brackets_digits_and_quotes_survive() {
        let mut m = base(5.0, 1.0);
        var(&mut m, "Plant A.B. - Diesel", Kind::Constant, "4");
        var(&mut m, "Stock[Plant 7.C. - Gas]", Kind::Constant, "10");
        var(&mut m, "3rd try", Kind::Aux, "\"Plant A.B. - Diesel\" * 2 + \"Stock[Plant 7.C. - Gas]\"");
        var(&mut m, "Tab\tand  spaces", Kind::Aux, "\"3rd try\" + Time");
        let (back, wrep, rrep, text) = roundtrip(&m);
        assert!(wrep.items.is_empty() && rrep.items.is_empty(), "{} / {}", wrep.summary(), rrep.summary());
        let n = same(&m, &back, "odd names");
        assert!(n >= 4, "compared {n}");
        assert!(back.by_name("plant a.b. - diesel").is_some(), "{text}");
        assert_eq!(back.by_name("3rd try").unwrap().equation, "\"Plant A.B. - Diesel\" * 2 + \"Stock[Plant 7.C. - Gas]\"");
    }
}

/// Reads every `.xmile` file under the folder named by `XMILE_CORPUS`, runs it, and compares the
/// result with the `output.csv` (or `output.tab`) next to it. Meant for a corpus such as the
/// PySD/SDXorg `test-models` repository; ignored by default because it needs files on disk:
/// `XMILE_CORPUS=/path/to/test-models cargo test corpus_xmile -- --ignored --nocapture`.
#[cfg(test)]
mod corpus {
    use super::*;
    use std::path::{Path, PathBuf};

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "xmile") {
                out.push(p);
            }
        }
    }

    fn table(path: &Path) -> Option<(Vec<String>, Vec<Vec<f64>>)> {
        let text = std::fs::read_to_string(path).ok()?;
        let sep = if path.extension().is_some_and(|e| e == "tab") { '\t' } else { ',' };
        let mut lines = text.split(|c| c == '\n' || c == '\r').filter(|l| !l.trim().is_empty());
        let head: Vec<String> = lines.next()?.split(sep).map(|h| h.trim().trim_matches('"').to_string()).collect();
        let rows = lines.map(|l| l.split(sep).map(|c| c.trim().trim_matches('"').parse::<f64>().unwrap_or(f64::NAN)).collect()).collect();
        Some((head, rows))
    }

    #[test]
    #[ignore]
    fn corpus_xmile() {
        let Ok(dir) = std::env::var("XMILE_CORPUS") else { return };
        let mut files = Vec::new();
        walk(Path::new(&dir), &mut files);
        let (mut pass, mut fail, mut unread, mut nocheck) = (0, 0, 0, 0);
        for f in &files {
            let rel = f.strip_prefix(&dir).unwrap_or(f).display().to_string();
            let text = std::fs::read_to_string(f).unwrap();
            let (m, rep) = match read(&text) {
                Ok(x) => x,
                Err(e) => {
                    println!("UNREAD   {rel}: {e}");
                    unread += 1;
                    continue;
                }
            };
            let loss: Vec<String> = rep.items.iter().filter(|i| i.kind != crate::loss::LossKind::Preserved).map(|i| format!("{}", i.construct)).collect();
            crate::userfn::set(&m.functions);
            let c = match crate::sim::compile(&m) {
                Ok(c) => c,
                Err(p) => {
                    println!("NOCOMPILE {rel}: {} | loss: {}", p.iter().take(2).map(|x| format!("{:?}: {}", x.var, x.message)).collect::<Vec<_>>().join("; "), loss.join(", "));
                    fail += 1;
                    continue;
                }
            };
            let run = match crate::sim::simulate(&c, &m, "x") {
                Ok(r) => r,
                Err(e) => {
                    println!("NORUN    {rel}: {e}");
                    fail += 1;
                    continue;
                }
            };
            let dir_of = f.parent().unwrap();
            let expected = ["output.csv", "output.tab"].iter().map(|n| dir_of.join(n)).find(|p| p.exists()).and_then(|p| table(&p));
            let Some((head, rows)) = expected else {
                println!("NOCHECK  {rel} (no expected output) loss: {}", loss.join(", "));
                nocheck += 1;
                continue;
            };
            let mut bad: Vec<String> = Vec::new();
            let mut compared = 0;
            for (ci, name) in head.iter().enumerate().skip(1) {
                if matches!(expr::normalize(name).as_str(), "initial time" | "final time" | "time step" | "saveper") && run.get(name).is_none() {
                    continue;
                }
                let Some(series) = run.get(name) else {
                    bad.push(format!("no series “{name}”"));
                    continue;
                };
                for row in &rows {
                    let t = row[0];
                    let Some(i) = run.times.iter().position(|x| (*x - t).abs() < 1e-6) else { continue };
                    let (want, got) = (row[ci], series[i]);
                    if want.is_nan() {
                        continue;
                    }
                    compared += 1;
                    if (want - got).abs() > 1e-3 * want.abs().max(got.abs()) + 1e-6 {
                        bad.push(format!("{name} at t={t}: expected {want}, got {got}"));
                        break;
                    }
                }
            }
            if bad.is_empty() && compared > 0 {
                println!("PASS     {rel} ({compared} values) loss: {}", loss.join(", "));
                pass += 1;
            } else {
                println!("FAIL     {rel}: {} | loss: {}", bad.iter().take(3).cloned().collect::<Vec<_>>().join("; "), loss.join(", "));
                fail += 1;
            }
        }
        println!("\n{} files: {pass} pass, {fail} fail, {unread} unread, {nocheck} without expected output", files.len());
    }
}
