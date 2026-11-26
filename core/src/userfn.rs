//! Your own functions, written in the equation language.
//!
//! `Effect of price(p) = 1 - 0.3 * LN(p)` is defined once (Edit → My functions…, or a *Functions*
//! sheet in a spreadsheet) and then used like a built-in: `Demand * Effect of price(Price / Base)`.
//! A call is replaced by the function's formula with each parameter replaced by the argument
//! before the equation is read, so a function may use anything a formula can — other functions
//! (up to a limit), set variables, `TIME` — and costs nothing at run time.

use std::cell::RefCell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::expr::{self, Tok};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct UserFn {
    pub name: String,
    /// Parameter names, in order.
    #[serde(default)]
    pub params: Vec<String>,
    /// The formula, written with the parameter names.
    pub body: String,
    #[serde(default)]
    pub description: String,
}

thread_local! {
    /// The functions in force on this thread (set when a model is compiled or loaded), so
    /// that anything that reads an equation sees the same functions.
    static REGISTRY: RefCell<HashMap<String, UserFn>> = RefCell::new(HashMap::new());
}

/// A string that changes whenever the user functions of the current thread do (empty when there are none).
pub fn signature() -> String {
    REGISTRY.with(|r| {
        let r = r.borrow();
        if r.is_empty() {
            return String::new();
        }
        let mut v: Vec<String> = r.iter().map(|(k, f)| format!("{k}|{}|{}", f.params.join(","), f.body)).collect();
        v.sort();
        v.join(";")
    })
}

/// Make these the user functions of the current thread.
pub fn set(fns: &[UserFn]) {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        r.clear();
        for f in fns {
            let n = expr::normalize(&f.name);
            if !n.is_empty() {
                r.insert(n, f.clone());
            }
        }
    });
}

fn lookup(name: &str) -> Option<UserFn> {
    REGISTRY.with(|r| r.borrow().get(&expr::normalize(name)).cloned())
}

fn any() -> bool {
    REGISTRY.with(|r| !r.borrow().is_empty())
}

/// Replace every call of a user function in `src` by its formula.
pub fn expand_text(src: &str) -> Result<String, String> {
    // calls of verbs with named options (DELAY(x, 6, order=3)) become the engine's own functions first
    let src = expr::expand_verbs(src)?;
    let src = src.as_str();
    if !any() {
        return Ok(src.to_string());
    }
    expand_depth(src, 0)
}

const MAX_DEPTH: usize = 12;

fn expand_depth(src: &str, depth: usize) -> Result<String, String> {
    let toks = match expr::tokenize(src) {
        Ok(t) => t,
        Err(_) => return Ok(src.to_string()), // the parser will report it
    };
    let mut out = String::new();
    let mut last = 0;
    let mut i = 0;
    while i < toks.len() {
        if let (Tok::Ident(name), span) = &toks[i] {
            if matches!(toks.get(i + 1), Some((Tok::LParen, _))) {
                if let Some(f) = lookup(name) {
                    if depth >= MAX_DEPTH {
                        return Err(format!("'{}' calls itself too deeply (a function can't use itself)", f.name));
                    }
                    // the arguments: split at commas outside any brackets
                    let mut level = 0i32;
                    let mut args: Vec<(usize, usize)> = Vec::new();
                    let mut start = toks[i + 1].1.end;
                    let mut j = i + 2;
                    let mut close = None;
                    while j < toks.len() {
                        match &toks[j].0 {
                            Tok::LParen | Tok::LBracket => level += 1,
                            Tok::RParen | Tok::RBracket if level == 0 => {
                                close = Some(j);
                                break;
                            }
                            Tok::RParen | Tok::RBracket => level -= 1,
                            Tok::Comma if level == 0 => {
                                args.push((start, toks[j].1.start));
                                start = toks[j].1.end;
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    let Some(close) = close else {
                        return Err(format!("{}( is missing its closing bracket", f.name));
                    };
                    let tail = (start, toks[close].1.start);
                    if !(args.is_empty() && src[tail.0..tail.1].trim().is_empty()) {
                        args.push(tail);
                    }
                    if args.len() != f.params.len() {
                        return Err(format!(
                            "{} needs {} value(s): {}({}), but it was given {}",
                            f.name,
                            f.params.len(),
                            f.name,
                            f.params.join(", "),
                            args.len()
                        ));
                    }
                    let given: Vec<String> = args.iter().map(|(a, b)| expand_depth(src[*a..*b].trim(), depth + 1)).collect::<Result<_, _>>()?;
                    // the body with each parameter replaced by its argument
                    let mut body = f.body.clone();
                    for (p, g) in f.params.iter().zip(&given) {
                        body = expr::rename_in(&body, &expr::normalize(p), &format!("({g})"));
                    }
                    let body = expand_depth(&body, depth + 1)?;
                    out.push_str(&src[last..span.start]);
                    out.push_str(&format!("({body})"));
                    last = toks[close].1.end;
                    i = close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out.push_str(&src[last..]);
    Ok(out)
}

/// Problems with the function definitions themselves (empty = fine).
pub fn check(fns: &[UserFn]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for f in fns {
        let n = expr::normalize(&f.name);
        let say = |m: String| (f.name.clone(), m);
        if n.is_empty() {
            out.push(say("A function has no name".into()));
            continue;
        }
        if expr::function_info(&n.to_uppercase()).is_some() {
            out.push(say(format!("'{}' is already a built-in function; choose another name", f.name)));
        }
        if seen.contains(&n) {
            out.push(say(format!("Two functions are called '{}'", f.name)));
        }
        seen.push(n);
        let mut ps: Vec<String> = Vec::new();
        for p in &f.params {
            let pn = expr::normalize(p);
            if pn.is_empty() || ps.contains(&pn) {
                out.push(say(format!("'{}' has an empty or repeated parameter name", f.name)));
            }
            ps.push(pn);
        }
        if f.body.trim().is_empty() {
            out.push(say(format!("'{}' has no formula", f.name)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(fns: Vec<UserFn>, f: impl FnOnce()) {
        set(&fns);
        f();
        set(&[]);
    }

    fn uf(name: &str, params: &[&str], body: &str) -> UserFn {
        UserFn { name: name.into(), params: params.iter().map(|s| s.to_string()).collect(), body: body.into(), description: String::new() }
    }

    #[test]
    fn calls_are_replaced_by_the_formula() {
        with(vec![uf("Effect of price", &["p"], "1 - 0.3 * LN(p)")], || {
            assert_eq!(expand_text("Demand * Effect of price(Price / Base)").unwrap(), "Demand * (1 - 0.3 * LN((Price / Base)))");
            assert_eq!(expand_text("a + 1").unwrap(), "a + 1");
        });
    }

    #[test]
    fn functions_can_use_functions_and_commas_inside_calls() {
        with(vec![uf("Twice", &["x"], "2 * x"), uf("Cap", &["x", "top"], "MIN(Twice(x), top)")], || {
            let e = expand_text("Cap(MAX(a, b), 10)").unwrap();
            assert_eq!(e, "(MIN((2 * ((MAX(a, b)))), (10)))");
            assert!(expr::parse(&e).is_ok());
        });
    }

    #[test]
    fn mistakes_are_explained() {
        with(vec![uf("Twice", &["x"], "2 * x"), uf("Loop", &["x"], "Loop(x)")], || {
            assert!(expand_text("Twice(1, 2)").unwrap_err().contains("needs 1 value"));
            assert!(expand_text("Loop(1)").unwrap_err().contains("itself"));
        });
        let bad = check(&[uf("MAX", &["a"], "a"), uf("f", &["x", "x"], "x"), uf("g", &[], "")]);
        assert_eq!(bad.len(), 3, "{bad:?}");
    }

    #[test]
    fn a_model_runs_with_a_function_that_uses_a_set_variable() {
        use crate::model::{Kind, Method, Model, Settings, Var};
        let mut m = Model::default();
        m.settings = Settings { initial_time: 0.0, final_time: 1.0, time_step: 1.0, time_units: "s".into(), method: Method::Euler, ..Default::default() };
        m.functions.push(uf("Share", &["part"], "part / TOTAL(Load)"));
        m.sets.push(crate::model::ItemSet { id: 1, name: "Unit".into(), items: vec!["a".into(), "b".into()], ..Default::default() });
        let mut load = Var::new(2, "Load", Kind::Constant, [0.0, 0.0]);
        load.equation = "1".into();
        load.set = Some("Unit".into());
        load.by_item.push(crate::model::ItemValue { item: "b".into(), equation: "3".into(), table: vec![] });
        let mut share = Var::new(3, "Load share", Kind::Aux, [0.0, 0.0]);
        share.equation = "Share(Load)".into();
        share.set = Some("Unit".into());
        m.vars.push(load);
        m.vars.push(share);
        let c = crate::sim::compile(&m).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>()).unwrap();
        let r = crate::sim::simulate(&c, &m, "t").unwrap();
        assert_eq!(r.get("Load share[a]").unwrap()[0], 0.25);
        assert_eq!(r.get("Load share[b]").unwrap()[0], 0.75);
    }
}
