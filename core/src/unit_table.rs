//! The table of unit atoms behind the exact units engine (`units::Unit`): SI prefixes, atoms with exact factors to the
//! base dimensions, and the symbols that have more than one meaning.
//!
//! Base dimensions: `m, kg, s, A, K, mol, cd`; money is `currency:<ISO code>`; a counted thing is `count:<name>`.
//! A factor is the value of one atom in base units (1 bbl = 0.158987294928 m^3).
//!
//! Sources of the factors:
//! - SI brochure (BIPM, 9th ed.): W = J/s, Wh = 3600 J, h = 3600 s, min = 60 s, L = 1e-3 m^3.
//! - barrel (oil, 42 US gallons) = 0.158987294928 m^3; US gallon = 3.785411784 L (both exact by definition).
//! - Btu (International Table) = 1055.05585262 J (ISO 31-4); MMBtu = 1e6 Btu.
//! - tonne of oil equivalent = 41.868e9 J (IEA / OECD convention); metric tonne = 1000 kg.
//! - foot = 0.3048 m, mile = 1609.344 m, pound = 0.45359237 kg (international agreement of 1959).
//! - percent = 0.01, parts per million = 1e-6.
//! - Month and Year come from the model's time convention (`year_days`): Year = year_days * 86400 s, Month = Year / 12.

use std::collections::BTreeMap;

pub type Dims = BTreeMap<String, i32>;

/// One meaning of a symbol: a label (what the modeller picks) and the factor to the atom's own dimensions.
#[derive(Clone, Debug, PartialEq)]
pub struct Meaning {
    pub label: &'static str,
    pub factor: f64,
}

#[derive(Clone, Debug)]
pub struct Atom {
    pub dims: Vec<(&'static str, i32)>,
    pub factor: f64,
    /// SI prefixes may be put in front of it.
    pub metric: bool,
    /// Pure-number atoms (`%`, `ppm`): their scale is a scale of the *value*, which `units_scale_values` applies.
    pub pure_number: bool,
    /// Symbols with several meanings: the first is the default when the modeller has not chosen.
    pub meanings: Vec<Meaning>,
}

fn atom(dims: &[(&'static str, i32)], factor: f64) -> Atom {
    Atom { dims: dims.to_vec(), factor, metric: false, pure_number: false, meanings: Vec::new() }
}
fn metric(dims: &[(&'static str, i32)], factor: f64) -> Atom {
    Atom { metric: true, ..atom(dims, factor) }
}
fn ambiguous(dims: &[(&'static str, i32)], meanings: &[(&'static str, f64)]) -> Atom {
    Atom { meanings: meanings.iter().map(|(label, factor)| Meaning { label, factor: *factor }).collect(), ..atom(dims, meanings[0].1) }
}

pub const BBL_M3: f64 = 0.158_987_294_928;
pub const GAL_M3: f64 = 3.785_411_784e-3;
pub const BTU_J: f64 = 1055.055_852_62;
pub const TOE_J: f64 = 41.868e9;

/// SI prefixes (symbol, factor). Only applied to metric atoms.
pub const PREFIXES: &[(&str, f64)] = &[
    ("k", 1e3), ("M", 1e6), ("G", 1e9), ("T", 1e12), ("P", 1e15), ("m", 1e-3), ("µ", 1e-6), ("u", 1e-6), ("n", 1e-9),
];

/// ISO 4217 codes the table knows as money.
pub const CURRENCIES: &[&str] = &[
    "USD", "EUR", "GBP", "JPY", "CNY", "SAR", "AED", "CHF", "CAD", "AUD", "INR", "BRL", "KRW", "RUB", "MXN", "NOK", "SEK", "DKK", "PLN", "TRY", "ZAR", "EGP", "KWD", "QAR", "SGD", "HKD", "NZD",
];

/// Names for the table, case-sensitive first. `year_days` is the abstract length of a Year.
pub fn lookup(name: &str, year_days: f64) -> Option<Atom> {
    let year = year_days * 86_400.0;
    let s = &[("s", 1)];
    let m3 = &[("m", 3)];
    let j = &[("kg", 1), ("m", 2), ("s", -2)];
    let w = &[("kg", 1), ("m", 2), ("s", -3)];
    Some(match name {
        // length, mass, time
        "m" | "meter" | "metre" => metric(&[("m", 1)], 1.0),
        "ft" | "foot" | "feet" => atom(&[("m", 1)], 0.3048),
        "mile" | "mi" => atom(&[("m", 1)], 1609.344),
        "g" | "gram" => metric(&[("kg", 1)], 1e-3),
        "t" | "tonne" | "metric ton" => metric(&[("kg", 1)], 1000.0),
        "lb" | "pound" => atom(&[("kg", 1)], 0.453_592_37),
        "ton" => ambiguous(&[("kg", 1)], &[("metric ton", 1000.0), ("short ton", 907.184_74), ("long ton", 1016.046_908_8)]),
        "s" | "sec" | "second" => metric(s, 1.0),
        "min" | "minute" => atom(s, 60.0),
        "h" | "hr" | "hour" => atom(s, 3600.0),
        "Day" | "day" => atom(s, 86_400.0),
        "Week" | "week" => atom(s, 7.0 * 86_400.0),
        "Month" | "month" => atom(s, year / 12.0),
        "Quarter" | "quarter" => atom(s, year / 4.0),
        "Year" | "year" | "yr" => atom(s, year),
        "Hz" => metric(&[("s", -1)], 1.0),
        // electricity, energy
        "W" | "watt" => metric(w, 1.0),
        "Wh" => metric(j, 3600.0),
        "J" | "joule" => metric(j, 1.0),
        "kWp" => atom(w, 1000.0),
        "MWp" => atom(w, 1e6),
        "Btu" | "BTU" => atom(j, BTU_J),
        "MMBtu" | "MMBTU" => atom(j, 1e6 * BTU_J),
        "toe" => atom(j, TOE_J),
        "cal" => metric(j, 4.184),
        // volume
        "L" | "l" | "litre" | "liter" => metric(m3, 1e-3),
        "bbl" | "barrel" => atom(m3, BBL_M3),
        "gal" | "gallon" => ambiguous(m3, &[("US gallon", GAL_M3), ("UK gallon", 4.546_09e-3)]),
        // oil-trade multipliers: ambiguous (see the table in the spec)
        "MB" | "Mbbl" => ambiguous(m3, &[("thousand barrels", 1e3 * BBL_M3), ("million barrels", 1e6 * BBL_M3)]),
        "MMB" | "MMbbl" => ambiguous(m3, &[("million barrels", 1e6 * BBL_M3), ("billion barrels", 1e9 * BBL_M3)]),
        "MCF" => ambiguous(m3, &[("thousand cubic feet", 1e3 * 0.028_316_846_592), ("million cubic feet", 1e6 * 0.028_316_846_592)]),
        "cf" | "cubic foot" => atom(m3, 0.028_316_846_592),
        // others
        "A" | "amp" => metric(&[("A", 1)], 1.0),
        "K" | "kelvin" => atom(&[("K", 1)], 1.0),
        "mol" => metric(&[("mol", 1)], 1.0),
        "cd" => atom(&[("cd", 1)], 1.0),
        "Pa" => metric(&[("kg", 1), ("m", -1), ("s", -2)], 1.0),
        "%" | "percent" => Atom { pure_number: true, ..atom(&[], 0.01) },
        "ppm" => Atom { pure_number: true, ..atom(&[], 1e-6) },
        _ => return None,
    })
}

/// The known atom names (for "did you mean" suggestions).
pub const SUGGEST: &[&str] = &[
    "kg", "Day", "Week", "Month", "Year", "kW", "MW", "GW", "TW", "Wh", "kWh", "MWh", "GWh", "TWh", "kJ", "MJ", "GJ",
    "Btu", "MMBtu", "toe", "bbl", "barrel", "MMB", "MCF", "ppm", "USD", "EUR", "GBP", "kWp", "tonne", "Quarter",
];

/// Every atom name of `lookup` (for case-insensitive matching).
pub const ATOM_NAMES: &[&str] = &[
    "m", "meter", "metre", "ft", "foot", "feet", "mile", "mi", "g", "gram", "t", "tonne", "lb", "pound", "ton", "s", "sec", "second", "min", "minute", "h", "hr", "hour", "Day", "day", "Week", "week",
    "Month", "month", "Quarter", "quarter", "Year", "year", "yr", "Hz", "W", "watt", "Wh", "J", "joule", "kWp", "MWp", "Btu", "BTU", "MMBtu", "MMBTU", "toe", "cal", "L", "l", "litre", "liter", "bbl", "barrel",
    "gal", "gallon", "MB", "Mbbl", "MMB", "MMbbl", "MCF", "cf", "A", "amp", "K", "kelvin", "mol", "cd", "Pa", "%", "percent", "ppm",
];

/// Words that are plainly "no unit".
pub fn is_dimensionless_word(w: &str) -> bool {
    matches!(w.to_lowercase().as_str(), "" | "dmnl" | "dimensionless" | "fraction" | "ratio" | "factor" | "yes" | "no" | "1" | "unitless" | "index" | "number" | "units" | "unit")
}

/// Edit distance (Levenshtein) of two words.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + usize::from(a[i - 1] != b[j - 1]));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The closest known atom to `word` (case-insensitive distance), if it is close enough to be a typo.
pub fn closest(word: &str) -> Option<&'static str> {
    let lw = word.to_lowercase();
    let mut best: Option<(&str, usize)> = None;
    for s in SUGGEST {
        let d = edit_distance(&lw, &s.to_lowercase());
        let limit = if s.chars().count() <= 2 { 0 } else if s.chars().count() <= 4 { 1 } else { 2 };
        if d <= limit && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((s, d));
        }
    }
    best.map(|(s, _)| s)
}
