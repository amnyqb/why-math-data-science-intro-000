//! The model library catalogue: what each model is, who made it, which skills and features it
//! teaches, how simple it is, so a user can search and filter. The data is `library/catalog.json`;
//! this module reads it, checks it against its own vocabulary, filters it and counts the facets.
//!
//! Two kinds of tags. The *generic* ones come from a fixed vocabulary (`category`, `domain`, `skills`,
//! `features`, `level`) so every model is described with the same words and filters mean the same thing.
//! The *special* ones (`special`) are free: names of the model's own ideas ("beer game", "bullwhip",
//! "overshoot and collapse") that no vocabulary could foresee.

use serde_json::{json, Value};

/// The generic facets, each a list of words in the catalogue's `vocab` (`level` is fixed).
pub const LIST_FACETS: [&str; 4] = ["domain", "skills", "features", "special"];
pub const LEVELS: [&str; 3] = ["beginner", "intermediate", "advanced"];

#[derive(Default, Debug, Clone)]
pub struct Filter {
    /// Words that must all appear somewhere in title, question, lessons, authors, tags or paper.
    pub text: Vec<String>,
    pub category: Vec<String>,
    pub domain: Vec<String>,
    pub skills: Vec<String>,
    pub features: Vec<String>,
    /// A word in any tag field, generic or special.
    pub tag: Vec<String>,
    pub author: Option<String>,
    pub level: Option<String>,
    pub status: Option<String>,
    pub year_from: Option<i64>,
    pub year_to: Option<i64>,
    /// 1 (simplest) to 5; keep models at or below this size/structure score.
    pub max_simplicity: Option<i64>,
    pub max_difficulty: Option<i64>,
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    match v.get(key) {
        Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(|s| s.to_lowercase())).collect(),
        Some(Value::String(s)) => vec![s.to_lowercase()],
        _ => vec![],
    }
}

fn has_all(have: &[String], want: &[String]) -> bool {
    want.iter().all(|w| have.contains(&w.to_lowercase()))
}

/// Does this entry pass every part of the filter?
pub fn matches(e: &Value, f: &Filter) -> bool {
    let all_tags: Vec<String> = ["category", "domain", "skills", "features", "special", "level"].iter().flat_map(|k| strings(e, k)).collect();
    let n = |k: &str| e.get(k).and_then(|x| x.as_i64());
    if !has_all(&strings(e, "category"), &f.category) || !has_all(&strings(e, "domain"), &f.domain) || !has_all(&strings(e, "skills"), &f.skills) || !has_all(&strings(e, "features"), &f.features) || !has_all(&all_tags, &f.tag) {
        return false;
    }
    if let Some(l) = &f.level {
        if strings(e, "level") != vec![l.to_lowercase()] {
            return false;
        }
    }
    if let Some(s) = &f.status {
        if strings(e, "status") != vec![s.to_lowercase()] {
            return false;
        }
    }
    if let Some(a) = &f.author {
        if !strings(e, "authors").iter().any(|x| x.contains(&a.to_lowercase())) {
            return false;
        }
    }
    if let Some(y) = f.year_from {
        if n("year").map_or(true, |x| x < y) {
            return false;
        }
    }
    if let Some(y) = f.year_to {
        if n("year").map_or(true, |x| x > y) {
            return false;
        }
    }
    if let Some(m) = f.max_simplicity {
        if n("simplicity").map_or(true, |x| x > m) {
            return false;
        }
    }
    if let Some(m) = f.max_difficulty {
        if n("difficulty").map_or(true, |x| x > m) {
            return false;
        }
    }
    if !f.text.is_empty() {
        let mut hay = all_tags.join(" ");
        for k in ["title", "question", "slug"] {
            hay.push(' ');
            hay.push_str(&e.get(k).and_then(|x| x.as_str()).unwrap_or("").to_lowercase());
        }
        for k in ["lessons", "authors"] {
            hay.push(' ');
            hay.push_str(&strings(e, k).join(" "));
        }
        hay.push(' ');
        hay.push_str(&e.pointer("/paper/citation").and_then(|x| x.as_str()).unwrap_or("").to_lowercase());
        if !f.text.iter().all(|w| hay.contains(&w.to_lowercase())) {
            return false;
        }
    }
    true
}

pub fn entries(catalog: &Value) -> &[Value] {
    catalog.get("entries").and_then(|e| e.as_array()).map_or(&[], |a| a.as_slice())
}

pub fn query(catalog: &Value, f: &Filter) -> Vec<Value> {
    entries(catalog).iter().filter(|e| matches(e, f)).cloned().collect()
}

/// How many models carry each word of each facet (so a user sees what can be filtered on).
pub fn facets(catalog: &Value, within: &[Value]) -> Value {
    let mut out = serde_json::Map::new();
    for k in ["category", "level", "status"].iter().chain(LIST_FACETS.iter()) {
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for e in within {
            for w in strings(e, k) {
                *counts.entry(w).or_default() += 1;
            }
        }
        out.insert(k.to_string(), json!(counts));
    }
    let _ = catalog;
    Value::Object(out)
}

/// Problems in the catalogue itself: a word not in the vocabulary, a missing field, a scale out of range,
/// a duplicate slug. (Whether the files exist is checked by the caller, which knows the folder.)
pub fn check(catalog: &Value) -> Vec<String> {
    let mut p = vec![];
    let vocab = |k: &str| -> Vec<String> { strings(&catalog["vocab"], k) };
    let mut seen = std::collections::HashSet::new();
    for e in entries(catalog) {
        let slug = e.get("slug").and_then(|s| s.as_str()).unwrap_or("");
        if slug.is_empty() || !seen.insert(slug.to_string()) {
            p.push(format!("entry “{slug}”: the slug is missing or repeated"));
        }
        for k in ["title", "category", "level", "status", "question"] {
            if e.get(k).is_none() {
                p.push(format!("{slug}: no {k}"));
            }
        }
        for (k, vk) in [("category", "category"), ("domain", "domain"), ("skills", "skills"), ("features", "features")] {
            for w in strings(e, k) {
                if !vocab(vk).contains(&w) {
                    p.push(format!("{slug}: “{w}” is not in the {vk} vocabulary (add it there, or use a special tag)"));
                }
            }
        }
        if !strings(e, "level").iter().all(|l| LEVELS.contains(&l.as_str())) {
            p.push(format!("{slug}: level must be beginner, intermediate or advanced"));
        }
        if !matches!(e.get("status").and_then(|s| s.as_str()), Some("built" | "planned")) {
            p.push(format!("{slug}: status must be built or planned"));
        }
        for k in ["simplicity", "difficulty"] {
            if !e.get(k).and_then(|x| x.as_i64()).map_or(false, |x| (1..=5).contains(&x)) {
                p.push(format!("{slug}: {k} must be a whole number from 1 to 5"));
            }
        }
        if e.get("status").and_then(|s| s.as_str()) == Some("built") && e.get("file").is_none() {
            p.push(format!("{slug}: a built model needs a file"));
        }
        if e.get("authors").is_none() || e.get("year").is_none() {
            p.push(format!("{slug}: authors and year are needed (use the source's, or ours for original lessons)"));
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        json!({"vocab": {"category": ["classic", "lesson"], "domain": ["health"], "skills": ["delays"], "features": ["lookup"]},
          "entries": [
            {"slug": "a", "title": "SIR", "category": ["classic"], "domain": ["health"], "skills": ["delays"], "features": [], "special": ["epidemic"], "level": "beginner", "status": "built", "file": "a.sdm", "question": "q", "simplicity": 1, "difficulty": 2, "authors": ["Kermack"], "year": 1927},
            {"slug": "b", "title": "Beer game", "category": ["lesson"], "domain": ["health"], "skills": [], "features": ["lookup"], "special": ["bullwhip"], "level": "advanced", "status": "planned", "question": "q", "simplicity": 4, "difficulty": 4, "authors": ["Sterman"], "year": 1989}]})
    }

    #[test]
    fn filters_combine_and_tags_match_generic_and_special_words() {
        let c = sample();
        assert_eq!(query(&c, &Filter { tag: vec!["epidemic".into()], ..Default::default() }).len(), 1);
        assert_eq!(query(&c, &Filter { domain: vec!["health".into()], ..Default::default() }).len(), 2);
        assert_eq!(query(&c, &Filter { domain: vec!["health".into()], max_simplicity: Some(2), ..Default::default() }).len(), 1);
        assert_eq!(query(&c, &Filter { year_from: Some(1950), author: Some("ster".into()), ..Default::default() }).len(), 1);
        assert_eq!(query(&c, &Filter { text: vec!["beer".into(), "lookup".into()], ..Default::default() }).len(), 1);
        assert_eq!(query(&c, &Filter { text: vec!["beer".into(), "delays".into()], ..Default::default() }).len(), 0);
        assert_eq!(facets(&c, entries(&c))["domain"]["health"], 2);
    }

    #[test]
    fn the_check_names_what_is_wrong() {
        let mut c = sample();
        assert!(check(&c).is_empty(), "{:?}", check(&c));
        c["entries"][0]["skills"] = json!(["telepathy"]);
        c["entries"][1]["simplicity"] = json!(9);
        let p = check(&c).join("|");
        assert!(p.contains("telepathy") && p.contains("simplicity"), "{p}");
    }
}
