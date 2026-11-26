//! Equation language: tokenizer, parser and the list of built-in functions.
//!
//! Names may contain spaces ("Contact Rate"), are case-insensitive and treat
//! underscores like spaces, so `contact_rate` and `Contact  Rate` are the same.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(f64),
    /// A variable reference, by normalised name.
    Var(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    /// Built-in function call; name is the canonical upper-case name.
    Call(String, Vec<Expr>),
    /// Calling a lookup (graphical function) variable by name.
    LookupCall(String, Box<Expr>),
    /// `WITH LOOKUP(input, ((x,y),...))`
    WithLookup(Box<Expr>, Vec<[f64; 2]>),
}

pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut space = false;
    for c in name.trim().trim_matches('"').chars() {
        if c == '_' || c.is_whitespace() {
            space = true;
        } else {
            if space && !out.is_empty() {
                out.push(' ');
            }
            space = false;
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// Built-in functions: (name, signature, explanation, min args, max args).
pub struct FnInfo {
    pub name: &'static str,
    pub signature: &'static str,
    pub help: &'static str,
    #[allow(dead_code)]
    pub group: &'static str,
    pub min: usize,
    pub max: usize,
}

pub const FUNCTIONS: &[FnInfo] = &[
    FnInfo {
        name: "IF THEN ELSE",
        signature: "IF THEN ELSE(condition, a, b)",
        help: "a when the condition is true, otherwise b",
        group: "Logic",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "MIN",
        signature: "MIN(a, b)",
        help: "Smaller of the values",
        group: "Math",
        min: 2,
        max: 16,
    },
    FnInfo {
        name: "MAX",
        signature: "MAX(a, b)",
        help: "Larger of the values",
        group: "Math",
        min: 2,
        max: 16,
    },
    FnInfo {
        name: "ABS",
        signature: "ABS(x)",
        help: "Absolute value",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "SQRT",
        signature: "SQRT(x)",
        help: "Square root",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "EXP",
        signature: "EXP(x)",
        help: "e raised to x",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "LN",
        signature: "LN(x)",
        help: "Natural logarithm",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "LOG",
        signature: "LOG(x, base)",
        help: "Logarithm of x in the given base",
        group: "Math",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "POWER",
        signature: "POWER(x, y)",
        help: "x raised to y (same as x ^ y)",
        group: "Math",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "INTEGER",
        signature: "INTEGER(x)",
        help: "Whole-number part of x",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "ROUND",
        signature: "ROUND(x)",
        help: "x rounded to the nearest whole number",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "MODULO",
        signature: "MODULO(x, y)",
        help: "Remainder of x divided by y",
        group: "Math",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "SIN",
        signature: "SIN(x)",
        help: "Sine (radians)",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "COS",
        signature: "COS(x)",
        help: "Cosine (radians)",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "TAN",
        signature: "TAN(x)",
        help: "Tangent (radians)",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "ARCTAN",
        signature: "ARCTAN(x)",
        help: "Inverse tangent",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "XIDZ",
        signature: "XIDZ(a, b, x)",
        help: "a / b, or x when b is zero",
        group: "Math",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "SINH",
        signature: "SINH(x)",
        help: "Hyperbolic sine.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "COSH",
        signature: "COSH(x)",
        help: "Hyperbolic cosine.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "TANH",
        signature: "TANH(x)",
        help: "Hyperbolic tangent: an S-shaped curve from −1 to 1, handy as a smooth switch.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "ARCSIN",
        signature: "ARCSIN(x)",
        help: "The angle (radians) whose sine is x; x must be between −1 and 1.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "ARCCOS",
        signature: "ARCCOS(x)",
        help: "The angle (radians) whose cosine is x; x must be between −1 and 1.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "ARCTAN2",
        signature: "ARCTAN2(y, x)",
        help: "The angle (radians) of the point (x, y), in the right quadrant.",
        group: "Math",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "FLOOR",
        signature: "FLOOR(x)",
        help: "Rounds down to the whole number below.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "SIGN",
        signature: "SIGN(x)",
        help: "−1, 0 or 1 according to the sign of x.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "LOG10",
        signature: "LOG10(x)",
        help: "Logarithm to base 10.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "LOG2",
        signature: "LOG2(x)",
        help: "Logarithm to base 2.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "CBRT",
        signature: "CBRT(x)",
        help: "Cube root.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "HYPOT",
        signature: "HYPOT(a, b)",
        help: "√(a² + b²): the length of the hypotenuse.",
        group: "Math",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "RADIANS",
        signature: "RADIANS(degrees)",
        help: "Converts degrees to radians.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "DEGREES",
        signature: "DEGREES(radians)",
        help: "Converts radians to degrees.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "CLAMP",
        signature: "CLAMP(x, low, high)",
        help: "Keeps x between a lower and an upper limit.",
        group: "Math",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "MEAN",
        signature: "MEAN(a, b, …)",
        help: "The average of two or more values.",
        group: "Math",
        min: 2,
        max: 16,
    },
    FnInfo {
        name: "MEDIAN",
        signature: "MEDIAN(a, b, …)",
        help: "The middle value of two or more values.",
        group: "Math",
        min: 2,
        max: 16,
    },
    FnInfo {
        name: "PI",
        signature: "PI()",
        help: "The number π (3.14159…). Write it with brackets: PI().",
        group: "Math",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "ERF",
        signature: "ERF(x)",
        help: "The error function (accuracy about 1e-7); NormalCDF(x) = (1 + ERF(x / SQRT(2))) / 2.",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "GAMMA",
        signature: "GAMMA(x)",
        help: "The gamma function (a continuous factorial: GAMMA(n + 1) = n!).",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "FACTORIAL",
        signature: "FACTORIAL(n)",
        help: "n! for a whole number n (up to 170).",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "SINWAVE",
        signature: "SINWAVE(amplitude, period, start)",
        help: "A sine wave: amplitude × sin(2π (TIME − start) / period)",
        group: "Test inputs",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "COSWAVE",
        signature: "COSWAVE(amplitude, period, start)",
        help: "A cosine wave: amplitude × cos(2π (TIME − start) / period)",
        group: "Test inputs",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "GRAPH",
        signature: "GRAPH(x, x0, dx, y1, y2, …)",
        help: "A curve given by evenly spaced values: y1 at x0, y2 at x0 + dx, … (linear between, flat beyond)",
        group: "Lookups",
        min: 5,
        max: 40,
    },
    FnInfo {
        name: "GRAPHSTEP",
        signature: "GRAPHSTEP(x, x0, dx, y1, y2, …)",
        help: "Like GRAPH but stepwise: each value holds until the next point",
        group: "Lookups",
        min: 5,
        max: 40,
    },
    FnInfo {
        name: "DELAYINF",
        signature: "DELAYINF(input, delay, order, [initial])",
        help: "Information delay of a chosen order: a chain of smooths, each delay ÷ order; starts at the initial value if given, else at the input",
        group: "Delays",
        min: 2,
        max: 4,
    },
    FnInfo {
        name: "DELAYMTR",
        signature: "DELAYMTR(input, delay, order, [initial])",
        help: "Material delay of a chosen order: a chain of stocks, each delay ÷ order; the outflow starts at the initial value if given, else at the input",
        group: "Delays",
        min: 2,
        max: 4,
    },
    FnInfo {
        name: "CEILING",
        signature: "CEILING(x)",
        help: "Round up to the next whole number",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "MAX OF",
        signature: "MAX OF(name [, group])",
        help: "The largest value of a variable that is per a set (Vensim's VMAX)",
        group: "Sets",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "MIN OF",
        signature: "MIN OF(name [, group])",
        help: "The smallest value of a variable that is per a set (Vensim's VMIN)",
        group: "Sets",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "COUNT OF",
        signature: "COUNT OF(name [, group])",
        help: "The number of items a variable that is per a set has (Vensim's ELMCOUNT and SIZE)",
        group: "Sets",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "TOTAL",
        signature: "TOTAL(name) or TOTAL(name, group)",
        help: "Add up a variable over all items of its set (or one group)",
        group: "Sets",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "AVERAGE",
        signature: "AVERAGE(name) or AVERAGE(name, group)",
        help: "Average of a variable over all items of its set (or one group)",
        group: "Sets",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "ZIDZ",
        signature: "ZIDZ(a, b)",
        help: "a / b, or 0 when b is zero",
        group: "Math",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "STEP",
        signature: "STEP(height, start)",
        help: "0 until start, then height",
        group: "Test inputs",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "PULSE",
        signature: "PULSE(start, width)",
        help: "1 from start for width, otherwise 0",
        group: "Test inputs",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "PULSE TRAIN",
        signature: "PULSE TRAIN(start, width, interval, end)",
        help: "Repeating pulses until end",
        group: "Test inputs",
        min: 4,
        max: 4,
    },
    FnInfo {
        name: "RAMP",
        signature: "RAMP(slope, start, end)",
        help: "Rises with slope between start and end",
        group: "Test inputs",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "RANDOM UNIFORM",
        signature: "RANDOM UNIFORM(min, max, [seed])",
        help: "Random number between min and max. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Test inputs",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "RANDOM NORMAL",
        signature: "RANDOM NORMAL(min, max, mean, sd, [seed])",
        help: "Normally distributed random number, clipped. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Test inputs",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "SMOOTH",
        signature: "SMOOTH(input, delay time)",
        help: "First-order exponential smoothing",
        group: "Delays",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "SMOOTHI",
        signature: "SMOOTHI(input, delay time, initial)",
        help: "SMOOTH with an initial value",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "SMOOTH3",
        signature: "SMOOTH3(input, delay time)",
        help: "Third-order smoothing",
        group: "Delays",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "SMOOTH3I",
        signature: "SMOOTH3I(input, delay time, initial)",
        help: "SMOOTH3 with an initial value",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "DELAY1",
        signature: "DELAY1(input, delay time)",
        help: "First-order material delay",
        group: "Delays",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "DELAY1I",
        signature: "DELAY1I(input, delay time, initial)",
        help: "DELAY1 with an initial value",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "DELAY3",
        signature: "DELAY3(input, delay time)",
        help: "Third-order material delay",
        group: "Delays",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "DELAY3I",
        signature: "DELAY3I(input, delay time, initial)",
        help: "DELAY3 with an initial value",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "DELAY FIXED",
        signature: "DELAY FIXED(input, delay time, initial)",
        help: "Input exactly delay time ago (pipeline)",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "TREND",
        signature: "TREND(input, average time, initial trend)",
        help: "Fractional rate of change of the input",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "ALLOCATE AVAILABLE ITEM",
        signature: "ALLOCATE AVAILABLE ITEM(k, n, request…, profile…, available)",
        help: "One requester's share of what is available (Vensim's ALLOCATE AVAILABLE, written out when a model is read)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "ALLOCATE BY PRIORITY ITEM",
        signature: "ALLOCATE BY PRIORITY ITEM(k, n, request…, priority…, width, supply)",
        help: "One requester's share by priority (Vensim's ALLOCATE BY PRIORITY, written out when a model is read)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "VECTOR SORT ORDER ITEM",
        signature: "VECTOR SORT ORDER ITEM(k, direction, x…)",
        help: "The 0-based index at place k when x is sorted (Vensim's VECTOR SORT ORDER)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "VECTOR RANK ITEM",
        signature: "VECTOR RANK ITEM(k, direction, x…)",
        help: "The 1-based rank of element k (Vensim's VECTOR RANK)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "VECTOR REORDER ITEM",
        signature: "VECTOR REORDER ITEM(k, n, x…, order…)",
        help: "x at the index order[k] (Vensim's VECTOR REORDER)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "VECTOR ELM MAP ITEM",
        signature: "VECTOR ELM MAP ITEM(start, offset, x…)",
        help: "x at start + offset (Vensim's VECTOR ELM MAP)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "VECTOR SELECT ITEMS",
        signature: "VECTOR SELECT ITEMS(n, selection…, value…, missing, action)",
        help: "Combines the selected values (Vensim's VECTOR SELECT)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "INVERT MATRIX ITEM",
        signature: "INVERT MATRIX ITEM(i, j, n, m…)",
        help: "Element (i, j) of the inverse of an n×n matrix given row by row (Vensim's INVERT MATRIX)",
        group: "Sets",
        min: 2,
        max: 100000,
    },
    FnInfo {
        name: "AGE",
        signature: "AGE(stock, every=, entrants=, open_ended=True|False)",
        help: "The net effect of ageing on one class of a set: its people move to the next class every so often, and the previous class moves into it",
        group: "Sets",
        min: 1,
        max: 3,
    },
    FnInfo {
        name: "QUEUE FIFO",
        signature: "QUEUE FIFO(inflow, outflow, initial)",
        help: "A waiting line: a level that remembers how long each part has waited; the oldest leaves first",
        group: "Delays",
        min: 2,
        max: 5,
    },
    FnInfo {
        name: "QUEUE AGE AVERAGE",
        signature: "QUEUE AGE AVERAGE(queue, [n])",
        help: "The average time those waiting have waited (the oldest n, or everyone)",
        group: "Delays",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "QUEUE AGE OLDEST",
        signature: "QUEUE AGE OLDEST(queue)",
        help: "How long the one who has waited longest has waited",
        group: "Delays",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "QUEUE AGE IN RANGE",
        signature: "QUEUE AGE IN RANGE(queue, min age, max age)",
        help: "How many waiting have waited at least min age and at most max age",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "QUEUE",
        signature: "QUEUE(arrivals, served, init=)",
        help: "A waiting line that remembers how long each has waited; read it with WAIT",
        group: "Delays",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "WAIT",
        signature: "WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)",
        help: "How long people in a queue have waited, or how many have waited longer than a limit",
        group: "Delays",
        min: 1,
        max: 5,
    },
    FnInfo {
        name: "EXPECT",
        signature: "EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)",
        help: "What people believe about x: its level as they perceive it, its trend, or where they expect it to be some time ahead",
        group: "Delays",
        min: 1,
        max: 5,
    },
    FnInfo {
        name: "LOOK",
        signature: "LOOK(curve, x, mode=lookup|forward|backward|inverse|extrapolate|slope|area, start=, outside=)",
        help: "Read a lookup curve: normally, as steps, the other way round, beyond its ends, its slope, or the area under it",
        group: "Lookups",
        min: 2,
        max: 5,
    },
    FnInfo {
        name: "INTERNAL RATE OF RETURN",
        signature: "INTERNAL RATE OF RETURN(stream, period, initial)",
        help: "The rate of return that makes the stream worth nothing at the start, per period; :NA: when there is none",
        group: "Delays",
        min: 1,
        max: 4,
    },
    FnInfo {
        name: "VALUE",
        signature: "VALUE(stream, rate, init=, factor=, timing=continuous|period_end) · VALUE(stream, measure=irr) · VALUE(stream, writeoff=time)",
        help: "The present value of a stream of money, discounted at a rate: the value so far, from the start of the run",
        group: "Delays",
        min: 1,
        max: 5,
    },
    FnInfo {
        name: "ALLOCATE",
        signature: "ALLOCATE(requests, supply, by=, overlap=, curve=rectangular|triangular|normal|exponential, spread=)",
        help: "Share a limited supply among the items of a set when it is short: by priority, in proportion to what each asks, or along a smooth priority curve",
        group: "Sets",
        min: 2,
        max: 6,
    },
    FnInfo {
        name: "DELAY",
        signature: "DELAY(x, time, [order], init=, kind=material|information, exact=True|False, leak=)",
        help: "Delay x by a time: material (what goes in comes out later) or information (a smoothed perception); optionally of a given order, with a starting value, or as an exact pipeline",
        group: "Delays",
        min: 2,
        max: 6,
    },
    FnInfo {
        name: "LOOKUP SLOPE",
        signature: "LOOKUP SLOPE(lookup, x, m)",
        help: "The slope of the lookup at x; outside its points m = 0 gives 0, 1 the nearest segment's slope, −1 :NA:",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "LOOKUP AREA",
        signature: "LOOKUP AREA(lookup, start, end)",
        help: "The area under the lookup between start and end",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "GET DATA AT TIME",
        signature: "GET DATA AT TIME(data, time)",
        help: "The data value at exactly that time, or :NA:",
        group: "Lookups",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "GET DATA BETWEEN TIMES",
        signature: "GET DATA BETWEEN TIMES(data, time, mode)",
        help: "The data value at a time between points: 0 interpolates, 1 takes the later point, −1 the earlier; :NA: outside the data",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "GET DATA FIRST TIME",
        signature: "GET DATA FIRST TIME(data)",
        help: "The time of the first data point",
        group: "Lookups",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "GET DATA LAST TIME",
        signature: "GET DATA LAST TIME(data)",
        help: "The time of the last data point",
        group: "Lookups",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "GET DATA MAX",
        signature: "GET DATA MAX(data, start, end)",
        help: "The largest data value between two times",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "GET DATA MIN",
        signature: "GET DATA MIN(data, start, end)",
        help: "The smallest data value between two times",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "GET DATA MEAN",
        signature: "GET DATA MEAN(data, start, end)",
        help: "The mean of the data values between two times",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "GET DATA MEDIAN",
        signature: "GET DATA MEDIAN(data, start, end)",
        help: "The median of the data values between two times",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "GET DATA STDV",
        signature: "GET DATA STDV(data, start, end)",
        help: "The sample standard deviation of the data values between two times",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "GET DATA TOTAL",
        signature: "GET DATA TOTAL(data, start, end)",
        help: "The sum of the data values between two times",
        group: "Lookups",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "NPVE",
        signature: "NPVE(stream, discount rate, initial value, factor)",
        help: "Net present value with a discrete period rate, valued at the end of the period (as Excel)",
        group: "Delays",
        min: 4,
        max: 4,
    },
    FnInfo {
        name: "DERIVN",
        signature: "DERIVN(x, order)",
        help: "The nth-order time derivative of x, from its past values",
        group: "Delays",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "EXCELDATESERIAL",
        signature: "EXCELDATESERIAL(year, month, day)",
        help: "The Excel serial number of a date (39448 is 1 January 2008)",
        group: "Math",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "EXCELDATESERIAL YEAR",
        signature: "EXCELDATESERIAL YEAR(serial)",
        help: "The year of an Excel serial date",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "EXCELDATESERIAL MONTH OF YEAR",
        signature: "EXCELDATESERIAL MONTH OF YEAR(serial)",
        help: "The month (1–12) of an Excel serial date",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "EXCELDATESERIAL DAY OF MONTH",
        signature: "EXCELDATESERIAL DAY OF MONTH(serial)",
        help: "The day of the month (1–31) of an Excel serial date",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "EXCELDATESERIAL DAY OF YEAR",
        signature: "EXCELDATESERIAL DAY OF YEAR(serial)",
        help: "The day of the year (1–366) of an Excel serial date",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "DAYS IN STEP",
        signature: "DAYS IN STEP()",
        help: "The number of calendar days in the current step (needs a start date; a month step of February 2028 has 29)",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "HOURS IN STEP",
        signature: "HOURS IN STEP()",
        help: "The number of hours in the current step: 24 times the days in the step",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "YEAR FRACTION",
        signature: "YEAR FRACTION(t1, t2, basis)",
        help: "The fraction of a year between two times, as Excel's YEARFRAC; basis 0 = 30/360 US, 1 = actual/actual, 2 = actual/360, 3 = actual/365, 4 = 30E/360 (needs a start date)",
        group: "Calendar",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "REAL RATE",
        signature: "REAL RATE(nominal, inflation)",
        help: "Rates are per period, as in Excel. The real rate (1 + nominal) / (1 + inflation) - 1",
        group: "Finance",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "NOMINAL RATE",
        signature: "NOMINAL RATE(real, inflation)",
        help: "Rates are per period, as in Excel. The nominal rate (1 + real) * (1 + inflation) - 1",
        group: "Finance",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "EFFECTIVE RATE",
        signature: "EFFECTIVE RATE(nominal, m)",
        help: "Rates are per period, as in Excel. The effective rate (1 + nominal / m)^m - 1 for m compounding periods (alias EFFECT)",
        group: "Finance",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "WACC",
        signature: "WACC(D, kd, ke, [T=0])",
        help: "Rates are per period, as in Excel. Weighted average cost of capital D * kd * (1 - T) + (1 - D) * ke",
        group: "Finance",
        min: 3,
        max: 4,
    },
    FnInfo {
        name: "ANNUITY FACTOR",
        signature: "ANNUITY FACTOR(r, n)",
        help: "Rates are per period, as in Excel. The capital recovery factor r / (1 - (1 + r)^-n), or 1/n at r = 0 (alias CRF)",
        group: "Finance",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "PMT",
        signature: "PMT(r, n, pv, [fv=0], [type=0])",
        help: "Rates are per period, as in Excel. The level payment of a loan; paid out is negative",
        group: "Finance",
        min: 3,
        max: 5,
    },
    FnInfo {
        name: "FV",
        signature: "FV(r, n, pmt, [pv=0], [type=0])",
        help: "Rates are per period, as in Excel. The future value of payments and a present amount",
        group: "Finance",
        min: 3,
        max: 5,
    },
    FnInfo {
        name: "PV",
        signature: "PV(r, n, pmt, [fv=0], [type=0])",
        help: "Rates are per period, as in Excel. The present value of payments and a future amount",
        group: "Finance",
        min: 3,
        max: 5,
    },
    FnInfo {
        name: "NPER",
        signature: "NPER(r, pmt, pv, [fv=0], [type=0])",
        help: "Rates are per period, as in Excel. The number of periods that pay off a loan",
        group: "Finance",
        min: 3,
        max: 5,
    },
    FnInfo {
        name: "IPMT",
        signature: "IPMT(r, per, n, pv, [fv=0], [type=0])",
        help: "Rates are per period, as in Excel. The interest part of payment number per",
        group: "Finance",
        min: 4,
        max: 6,
    },
    FnInfo {
        name: "PPMT",
        signature: "PPMT(r, per, n, pv, [fv=0], [type=0])",
        help: "Rates are per period, as in Excel. The principal part of payment number per",
        group: "Finance",
        min: 4,
        max: 6,
    },
    FnInfo {
        name: "RATE",
        signature: "RATE(n, pmt, pv, [fv=0], [type=0], [guess=0.1])",
        help: "Rates are per period, as in Excel. The rate per period that solves the loan equation (Newton, then bisection); missing if there is none",
        group: "Finance",
        min: 3,
        max: 6,
    },
    FnInfo {
        name: "DSCR",
        signature: "DSCR(cash, debt service)",
        help: "Debt service cover ratio: cash / debt service, or 1e9 when there is no debt service",
        group: "Finance",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "AND",
        signature: "AND(a, b, …)",
        help: "1 when every argument is true (not zero)",
        group: "Logic",
        min: 1,
        max: 16,
    },
    FnInfo {
        name: "OR",
        signature: "OR(a, b, …)",
        help: "1 when any argument is true",
        group: "Logic",
        min: 1,
        max: 16,
    },
    FnInfo {
        name: "NOT",
        signature: "NOT(x)",
        help: "1 when x is zero, else 0",
        group: "Logic",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "BETWEEN",
        signature: "BETWEEN(x, low, high)",
        help: "1 if low <= x <= high",
        group: "Logic",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "DATE",
        signature: "DATE()",
        help: "The Excel serial date of the current time (needs a start date)",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "YEAR",
        signature: "YEAR()",
        help: "The calendar year of the current time",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "MONTH",
        signature: "MONTH()",
        help: "The month (1 to 12) of the current time",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "DAY OF MONTH",
        signature: "DAY OF MONTH()",
        help: "The day of the month of the current time",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "DAY OF YEAR",
        signature: "DAY OF YEAR()",
        help: "The day of the year (1 to 366) of the current time",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "HOUR OF DAY",
        signature: "HOUR OF DAY()",
        help: "The hour of the day (0 to 23) of the current time",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "WEEK NUMBER",
        signature: "WEEK NUMBER()",
        help: "The ISO 8601 week number of the current time",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "DAY OF WEEK",
        signature: "DAY OF WEEK()",
        help: "1 Monday to 7 Sunday for the current time",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "IS WEEKEND",
        signature: "IS WEEKEND()",
        help: "1 on a weekend day (setting weekend: sat-sun or fri-sat)",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "IS WORKDAY",
        signature: "IS WORKDAY()",
        help: "1 when the day is neither a weekend nor a holiday",
        group: "Calendar",
        min: 0,
        max: 0,
    },
    FnInfo {
        name: "WORKDAYS BETWEEN",
        signature: "WORKDAYS BETWEEN(t1, t2)",
        help: "The workdays from time t1 up to but not including time t2",
        group: "Calendar",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "PROFILE",
        signature: "PROFILE(period, v1, …, vn)",
        help: "A repeating profile of n values over the period, interpolated between slot centres and wrapping",
        group: "Calendar",
        min: 3,
        max: 64,
    },
    FnInfo {
        name: "PROFILE STEP",
        signature: "PROFILE STEP(period, v1, …, vn)",
        help: "A repeating profile of n values over the period, constant in each slot",
        group: "Calendar",
        min: 3,
        max: 64,
    },
    FnInfo {
        name: "SEASON",
        signature: "SEASON(low, high, peak time, period)",
        help: "low + (high - low) * (1 + cos(2 pi (Time - peak) / period)) / 2",
        group: "Calendar",
        min: 4,
        max: 4,
    },
    FnInfo {
        name: "DAYLIGHT",
        signature: "DAYLIGHT(sunrise, sunset)",
        help: "A half-sine of daylight between sunrise and sunset hours, 0 at night (needs a start date)",
        group: "Calendar",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "AT TIME",
        signature: "AT TIME(t)",
        help: "1 when |Time - t| < DT/2, else 0",
        group: "Calendar",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "EVERY",
        signature: "EVERY(period, [start])",
        help: "1 when the time is within DT/2 of a multiple of the period after start",
        group: "Calendar",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "NPV AT",
        signature: "NPV AT(stream, rate, init, factor, offset)",
        help: "Present value with the discount factor (1 + r)^-(k + offset); the engine form of VALUE(timing=period_start or mid_period)",
        group: "Delays",
        min: 5,
        max: 5,
    },
    FnInfo {
        name: "LEVELISED STATE",
        signature: "LEVELISED STATE(cost, quantity, rate, init, timing)",
        help: "The engine form of LEVELISED",
        group: "Finance",
        min: 5,
        max: 5,
    },
    FnInfo {
        name: "LOAN STATE",
        signature: "LOAN STATE(drawdown, rate, term, type, grace, measure)",
        help: "The engine form of LOAN",
        group: "Finance",
        min: 6,
        max: 6,
    },
    FnInfo {
        name: "TAX STATE",
        signature: "TAX STATE(income, rate, carryforward, years)",
        help: "The engine form of TAX",
        group: "Finance",
        min: 4,
        max: 4,
    },
    FnInfo {
        name: "PAYBACK",
        signature: "PAYBACK(stream, [rate=0])",
        help: "The time at which the cumulative (discounted, period end) stream first becomes positive after being negative, interpolated within the step; missing until then",
        group: "Finance",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "ESCALATE",
        signature: "ESCALATE(base, rate, [from])",
        help: "base * (1 + rate)^((Time - from) / period of the rate); from defaults to the initial time",
        group: "Finance",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "LEVELISED",
        signature: "LEVELISED(cost stream, quantity stream, rate, [initial cost], timing=period_end)",
        help: "Levelised cost: present value of costs over present value of quantity (alias LCOE)",
        group: "Finance",
        min: 3,
        max: 4,
    },
    FnInfo {
        name: "LOAN",
        signature: "LOAN(drawdown, rate, term, type=annuity, grace=0, measure=balance)",
        help: "A loan: tranches opened by the drawdown, repaid as an annuity, linearly or at the end; measure is balance, payment, interest or principal",
        group: "Finance",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "TAX",
        signature: "TAX(taxable income, rate, carryforward=yes, years=0)",
        help: "Tax on income with losses carried forward (years = 0 means they never expire)",
        group: "Finance",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "RANDOM TRIANGULAR",
        signature: "RANDOM TRIANGULAR(min, max, mode, [seed])",
        help: "Triangular random numbers. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 3,
        max: 4,
    },
    FnInfo {
        name: "RANDOM LOGNORMAL",
        signature: "RANDOM LOGNORMAL(min, max, mean, sd, [seed])",
        help: "Log-normal random numbers with the mean and standard deviation of the variable itself. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM GAMMA",
        signature: "RANDOM GAMMA(min, max, shape, scale, [seed])",
        help: "Gamma random numbers. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM BETA",
        signature: "RANDOM BETA(min, max, a, b, [seed])",
        help: "Beta random numbers stretched over min to max. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM WEIBULL",
        signature: "RANDOM WEIBULL(min, max, shape, scale, [seed])",
        help: "Weibull random numbers. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM POISSON",
        signature: "RANDOM POISSON(min, max, mean, [seed])",
        help: "Poisson random counts. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 3,
        max: 4,
    },
    FnInfo {
        name: "RANDOM BINOMIAL",
        signature: "RANDOM BINOMIAL(min, max, n, p, [seed])",
        help: "Binomial random counts. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM NEGATIVE BINOMIAL",
        signature: "RANDOM NEGATIVE BINOMIAL(min, max, r, p, [seed])",
        help: "Failures before the r-th success. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM GEOMETRIC",
        signature: "RANDOM GEOMETRIC(min, max, p, [seed])",
        help: "Failures before the first success. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 3,
        max: 4,
    },
    FnInfo {
        name: "RANDOM PARETO",
        signature: "RANDOM PARETO(min, max, shape, scale, [seed])",
        help: "Pareto random numbers. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM PERT",
        signature: "RANDOM PERT(min, max, mode, shape, [seed])",
        help: "PERT random numbers (a stretched beta around the mode; shape 4 is usual). Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "RANDOM 0 1",
        signature: "RANDOM 0 1([seed])",
        help: "A uniform random number between 0 and 1. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 0,
        max: 1,
    },
    FnInfo {
        name: "TRIANGULAR",
        signature: "TRIANGULAR(low, mode, high, [seed])",
        help: "Triangular random numbers. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 3,
        max: 4,
    },
    FnInfo {
        name: "LOGNORMAL",
        signature: "LOGNORMAL(mean, sd, [seed])",
        help: "Log-normal random numbers with the mean and standard deviation of the variable itself. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "WEIBULL",
        signature: "WEIBULL(shape, scale, [seed])",
        help: "Weibull random numbers. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "POISSON",
        signature: "POISSON(mean, [seed])",
        help: "Poisson random counts. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "NORMAL",
        signature: "NORMAL(mean, sd, [seed])",
        help: "Normal random numbers (Stella). Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "UNIFORM",
        signature: "UNIFORM(min, max, [seed])",
        help: "Uniform random numbers (Stella). Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 2,
        max: 3,
    },
    FnInfo {
        name: "EXPRND",
        signature: "EXPRND(mean, [seed])",
        help: "Exponential random numbers (Stella). Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Random",
        min: 1,
        max: 2,
    },
    FnInfo {
        name: "NORMAL CDF",
        signature: "NORMAL CDF(x, mean, sd)",
        help: "The cumulative normal distribution",
        group: "Statistics",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "NORMAL INV",
        signature: "NORMAL INV(p, mean, sd)",
        help: "The inverse of the cumulative normal distribution",
        group: "Statistics",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "LOGNORMAL CDF",
        signature: "LOGNORMAL CDF(x, mu, sigma)",
        help: "The cumulative log-normal distribution (mu and sigma of the log)",
        group: "Statistics",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "LOGNORMAL INV",
        signature: "LOGNORMAL INV(p, mu, sigma)",
        help: "The inverse of the cumulative log-normal distribution",
        group: "Statistics",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "PREVIOUS",
        signature: "PREVIOUS(x, initial)",
        help: "The value x had at the previous step (initial at the start)",
        group: "Statistics",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "HISTORY",
        signature: "HISTORY(x, time)",
        help: "The value x had at an earlier time",
        group: "Statistics",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "ROUND WITH CARRY",
        signature: "ROUND WITH CARRY(flow)",
        help: "A flow moved in whole units: the remainder carries to the next step, so the long-run average equals the input",
        group: "Statistics",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "VECTOR LOOKUP ITEMS",
        signature: "VECTOR LOOKUP ITEMS(n, y…, x, xmin, xmax, mode)",
        help: "A lookup whose y values are the elements of an array, over equal steps from xmin to xmax (Vensim's VECTOR LOOKUP)",
        group: "Sets",
        min: 6,
        max: 100000,
    },
    FnInfo {
        name: "NPV",
        signature: "NPV(stream, discount rate, initial value, factor)",
        help: "Net present value of a stream: the discounted sum so far, times the factor",
        group: "Delays",
        min: 4,
        max: 4,
    },
    FnInfo {
        name: "DEPRECIATE STRAIGHTLINE",
        signature: "DEPRECIATE STRAIGHTLINE(input, depreciation time, [fiscal], [initial])",
        help: "Each input written off evenly over the depreciation time; the amount written off now",
        group: "Delays",
        min: 2,
        max: 4,
    },
    FnInfo {
        name: "GAMMA LN",
        signature: "GAMMA LN(x)",
        help: "Natural log of the gamma function",
        group: "Math",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "RANDOM EXPONENTIAL",
        signature: "RANDOM EXPONENTIAL(min, max, shift, stretch, [seed])",
        help: "Exponentially distributed random number, shifted and stretched, kept within min and max. Without a seed the call has its own stream, derived from the model seed, the variable and the call, so adding a variable never changes another's draws.",
        group: "Test inputs",
        min: 4,
        max: 5,
    },
    FnInfo {
        name: "LOOKUP FORWARD",
        signature: "LOOKUP FORWARD(lookup, x)",
        help: "The y of the next point at or after x, without interpolating",
        group: "Lookups",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "LOOKUP BACKWARD",
        signature: "LOOKUP BACKWARD(lookup, x)",
        help: "The y of the last point at or before x, without interpolating",
        group: "Lookups",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "LOOKUP INVERT",
        signature: "LOOKUP INVERT(lookup, y)",
        help: "The x at which the lookup reaches y (the table read the other way)",
        group: "Lookups",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "LOOKUP EXTRAPOLATE",
        signature: "LOOKUP EXTRAPOLATE(lookup, x)",
        help: "Like the lookup, but continues in straight lines beyond the first and last points",
        group: "Lookups",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "SAMPLE IF TRUE",
        signature: "SAMPLE IF TRUE(condition, input, initial)",
        help: "The input while the condition holds; otherwise the value last taken (the initial value until then)",
        group: "Delays",
        min: 3,
        max: 3,
    },
    FnInfo {
        name: "QUANTUM",
        signature: "QUANTUM(x, q)",
        help: "x rounded down toward zero to a multiple of q (x itself if q is 0 or less)",
        group: "Math",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "ACTIVE INITIAL",
        signature: "ACTIVE INITIAL(x, initial value)",
        help: "x during the run; the initial value only while initial values are computed (breaks loops among initial values)",
        group: "Delays",
        min: 2,
        max: 2,
    },
    FnInfo {
        name: "INITIAL",
        signature: "INITIAL(x)",
        help: "Value of x at the start, held constant",
        group: "Delays",
        min: 1,
        max: 1,
    },
    FnInfo {
        name: "WITH LOOKUP",
        signature: "WITH LOOKUP(input, ((0,0),(1,1)))",
        help: "Graphical function written inline",
        group: "Lookups",
        min: 1,
        max: 1,
    },
];

pub fn function_info(name: &str) -> Option<&'static FnInfo> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

/// Built-in time variables usable anywhere in equations.
pub const TIME_VARS: &[(&str, &str)] = &[
    ("time", "Current simulation time"),
    ("initial time", "Start of the simulation"),
    ("final time", "End of the simulation"),
    ("time step", "Integration step (dt)"),
];

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(f64),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
}

pub fn tokenize(src: &str) -> Result<Vec<(Tok, Range<usize>)>, String> {
    let b: Vec<char> = src.chars().collect();
    // byte offsets of each char so spans index the original string
    let offs: Vec<usize> = src
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(src.len()))
        .collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() || c == '\\' {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() || (c == '.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
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
            let s: String = b[start..i].iter().collect();
            let v = s.parse::<f64>().map_err(|_| format!("Bad number '{s}'"))?;
            out.push((Tok::Num(v), offs[start]..offs[i]));
            continue;
        }
        if c == '"' {
            i += 1;
            while i < b.len() && b[i] != '"' {
                i += 1;
            }
            if i >= b.len() {
                return Err("Missing closing quote".into());
            }
            let s: String = b[start + 1..i].iter().collect();
            i += 1;
            out.push((Tok::Ident(s), offs[start]..offs[i]));
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '$' {
            let mut end = i;
            while i < b.len()
                && (b[i].is_alphanumeric()
                    || b[i] == '_'
                    || b[i] == ' '
                    || b[i] == '$'
                    || b[i] == '\'')
            {
                i += 1;
                if b[i - 1] != ' ' {
                    end = i;
                }
            }
            i = end;
            let s: String = b[start..end].iter().collect();
            out.push((Tok::Ident(s), offs[start]..offs[end]));
            continue;
        }
        if c == ':' {
            // :AND: :OR: :NOT:
            let rest: String = b[i..].iter().take(6).collect::<String>().to_uppercase();
            for (kw, op, n) in [(":AND:", "&&", 5), (":OR:", "||", 4), (":NOT:", "!:", 5)] {
                if rest.starts_with(kw) {
                    out.push((Tok::Op(op), offs[i]..offs[i + n]));
                    i += n;
                    break;
                }
            }
            if i == start {
                return Err("Unexpected ':'".into());
            }
            continue;
        }
        let two: String = b[i..(i + 2).min(b.len())].iter().collect();
        let op2 = match two.as_str() {
            "<=" => Some("<="),
            ">=" => Some(">="),
            "<>" | "!=" => Some("<>"),
            "==" => Some("="),
            "&&" => Some("&&"),
            "||" => Some("||"),
            _ => None,
        };
        if let Some(op) = op2 {
            out.push((Tok::Op(op), offs[i]..offs[i + 2]));
            i += 2;
            continue;
        }
        let tok = match c {
            '+' => Tok::Op("+"),
            '-' => Tok::Op("-"),
            '*' => Tok::Op("*"),
            '/' => Tok::Op("/"),
            '^' => Tok::Op("^"),
            '<' => Tok::Op("<"),
            '>' => Tok::Op(">"),
            '=' => Tok::Op("="),
            '!' => Tok::Op("!"),
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            ',' => Tok::Comma,
            _ => return Err(format!("Unexpected character '{c}'")),
        };
        out.push((tok, offs[i]..offs[i + 1]));
        i += 1;
    }
    Ok(out)
}

/// Names that other tools use for functions Dynamist has under another name: (alias, target). `SAFEDIV` is XIDZ with three
/// arguments and ZIDZ with two. One table, used by the parser (and so by the importers, which go through it).
pub const ALIASES: &[(&str, &str)] = &[
    ("SUM", "TOTAL"),
    ("VMAX", "MAX OF"),
    ("VMIN", "MIN OF"),
    ("ELMCOUNT", "COUNT OF"),
    ("SIZE", "COUNT OF"),
    ("MOD", "MODULO"),
    ("INT", "INTEGER"),
    ("SMTH1", "SMOOTH"),
    ("SMTH3", "SMOOTH3"),
    ("SMTHN", "SMOOTH N"),
    ("EFFECT", "EFFECTIVE RATE"),
    ("CRF", "ANNUITY FACTOR"),
    ("LCOE", "LEVELISED"),
    ("IRR", "INTERNAL RATE OF RETURN"),
];

/// The canonical name of `name` with `nargs` arguments, if it is an alias of a function that exists.
pub fn alias_of(name: &str, nargs: usize) -> Option<&'static str> {
    if name == "SAFEDIV" {
        return Some(if nargs == 3 { "XIDZ" } else { "ZIDZ" });
    }
    ALIASES.iter().find(|(a, _)| *a == name).map(|(_, t)| *t)
}

/// Normalise a function name (collapse spaces, upper-case).
fn fn_name(s: &str) -> String {
    normalize(s).to_uppercase()
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

thread_local! {
    /// The rules of the doctor read every equation again and again; the same text (with the same user functions) parses to the same tree.
    static PARSED: std::cell::RefCell<std::collections::HashMap<(String, String), Result<Expr, String>>> = Default::default();
}

pub fn parse(src: &str) -> Result<Expr, String> {
    let key = (src.to_string(), crate::userfn::signature());
    if let Some(hit) = PARSED.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let r = parse_uncached(src);
    PARSED.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 50_000 {
            c.clear();
        }
        c.insert(key, r.clone());
    });
    r
}

fn parse_uncached(src: &str) -> Result<Expr, String> {
    // calls of the person's own functions are replaced by their formulas first
    let expanded = crate::userfn::expand_text(src)?;
    let src = expanded.as_str();
    let toks = tokenize(src)?
        .into_iter()
        .map(|(t, _)| t)
        .collect::<Vec<_>>();
    if toks.is_empty() {
        return Err("Equation is empty".into());
    }
    let mut p = Parser { toks, pos: 0 };
    let e = p.expr()?;
    if p.pos < p.toks.len() {
        return Err(format!(
            "Unexpected {} after the end of the formula",
            describe(&p.toks[p.pos])
        ));
    }
    Ok(e)
}

/// Parse a lookup table literal such as `[(0,0)-(1,1)],(0,0),(0.5,0.7),(1,1)`.
pub fn parse_table(src: &str) -> Result<Vec<[f64; 2]>, String> {
    let toks = tokenize(src)?
        .into_iter()
        .map(|(t, _)| t)
        .collect::<Vec<_>>();
    // Old Vensim / DYNAMO form: `( x1, x2, ..., xn, y1, y2, ..., yn )`, a flat list of 2n numbers,
    // the x values first and then the y values (World Dynamics, Market Growth and other classic models).
    let flat: Option<Vec<f64>> = (|| {
        let inner = match (toks.first(), toks.last()) {
            (Some(Tok::LParen), Some(Tok::RParen)) => &toks[1..toks.len() - 1],
            _ => &toks[..],
        };
        let mut nums = Vec::new();
        let mut i = 0;
        while i < inner.len() {
            let neg = matches!(&inner[i], Tok::Op("-"));
            if neg {
                i += 1;
            }
            match inner.get(i) {
                Some(Tok::Num(n)) => nums.push(if neg { -n } else { *n }),
                _ => return None,
            }
            i += 1;
            match inner.get(i) {
                None => break,
                Some(Tok::Comma) => i += 1,
                _ => return None,
            }
        }
        (nums.len() >= 2 && nums.len() % 2 == 0).then_some(nums)
    })();
    if let Some(nums) = flat {
        let n = nums.len() / 2;
        return Ok((0..n).map(|k| [nums[k], nums[n + k]]).collect());
    }
    let mut p = Parser { toks, pos: 0 };
    let wrapped = p.peek() == Some(&Tok::LParen)
        && matches!(p.toks.get(1), Some(Tok::LParen) | Some(Tok::LBracket));
    if wrapped {
        p.pos += 1;
    }
    let t = p.table_body()?;
    if wrapped {
        p.expect(Tok::RParen)?;
    }
    Ok(t)
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Num(n) => format!("number {n}"),
        Tok::Ident(s) => format!("'{s}'"),
        Tok::Op(o) => format!("'{o}'"),
        Tok::LParen => "'('".into(),
        Tok::RParen => "')'".into(),
        Tok::LBracket => "'['".into(),
        Tok::RBracket => "']'".into(),
        Tok::Comma => "','".into(),
    }
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn eat_op(&mut self, ops: &[&'static str]) -> Option<&'static str> {
        if let Some(Tok::Op(o)) = self.peek() {
            if let Some(found) = ops.iter().find(|x| **x == *o) {
                self.pos += 1;
                return Some(found);
            }
        }
        None
    }
    fn expect(&mut self, t: Tok) -> Result<(), String> {
        match self.next() {
            Some(ref x) if *x == t => Ok(()),
            Some(x) => Err(format!(
                "Expected {} but found {}",
                describe(&t),
                describe(&x)
            )),
            None => Err(format!("Expected {} but the formula ended", describe(&t))),
        }
    }
    fn expr(&mut self) -> Result<Expr, String> {
        let mut l = self.and()?;
        while self.eat_op(&["||"]).is_some() {
            let r = self.and()?;
            l = Expr::Bin(BinOp::Or, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut l = self.not_cmp()?;
        while self.eat_op(&["&&"]).is_some() {
            let r = self.not_cmp()?;
            l = Expr::Bin(BinOp::And, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    /// Vensim's `:NOT:` applies to a whole comparison (`:NOT: a = b` is `!(a = b)`), unlike `!`.
    fn not_cmp(&mut self) -> Result<Expr, String> {
        if self.eat_op(&["!:"]).is_some() {
            return Ok(Expr::Not(Box::new(self.not_cmp()?)));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<Expr, String> {
        let mut l = self.add()?;
        while let Some(op) = self.eat_op(&["<", "<=", ">", ">=", "=", "<>"]) {
            let r = self.add()?;
            let op = match op {
                "<" => BinOp::Lt,
                "<=" => BinOp::Le,
                ">" => BinOp::Gt,
                ">=" => BinOp::Ge,
                "=" => BinOp::Eq,
                _ => BinOp::Ne,
            };
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn add(&mut self) -> Result<Expr, String> {
        let mut l = self.mul()?;
        while let Some(op) = self.eat_op(&["+", "-"]) {
            let r = self.mul()?;
            l = Expr::Bin(
                if op == "+" { BinOp::Add } else { BinOp::Sub },
                Box::new(l),
                Box::new(r),
            );
        }
        Ok(l)
    }
    fn mul(&mut self) -> Result<Expr, String> {
        let mut l = self.unary()?;
        while let Some(op) = self.eat_op(&["*", "/"]) {
            let r = self.unary()?;
            l = Expr::Bin(
                if op == "*" { BinOp::Mul } else { BinOp::Div },
                Box::new(l),
                Box::new(r),
            );
        }
        Ok(l)
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat_op(&["-"]).is_some() {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat_op(&["+"]).is_some() {
            return self.unary();
        }
        if self.eat_op(&["!"]).is_some() {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        self.power()
    }
    fn power(&mut self) -> Result<Expr, String> {
        let base = self.primary()?;
        if self.eat_op(&["^"]).is_some() {
            let e = self.unary()?;
            return Ok(Expr::Bin(BinOp::Pow, Box::new(base), Box::new(e)));
        }
        Ok(base)
    }
    fn signed_num(&mut self) -> Result<f64, String> {
        let neg = self.eat_op(&["-"]).is_some();
        match self.next() {
            Some(Tok::Num(n)) => Ok(if neg { -n } else { n }),
            Some(t) => Err(format!(
                "Expected a number in the table but found {}",
                describe(&t)
            )),
            None => Err("Table ended early".into()),
        }
    }
    fn pair(&mut self) -> Result<[f64; 2], String> {
        self.expect(Tok::LParen)?;
        let x = self.signed_num()?;
        self.expect(Tok::Comma)?;
        let y = self.signed_num()?;
        self.expect(Tok::RParen)?;
        Ok([x, y])
    }
    /// `[(x0,y0)-(x1,y1)], (x,y), (x,y) ...` – the range part is optional.
    fn table_body(&mut self) -> Result<Vec<[f64; 2]>, String> {
        if self.peek() == Some(&Tok::LBracket) {
            let mut depth = 0;
            while let Some(t) = self.next() {
                match t {
                    Tok::LBracket => depth += 1,
                    Tok::RBracket => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if self.peek() == Some(&Tok::Comma) {
                self.pos += 1;
            }
        }
        let mut pts = vec![self.pair()?];
        while self.peek() == Some(&Tok::Comma) {
            self.pos += 1;
            pts.push(self.pair()?);
        }
        Ok(pts)
    }
    /// One argument of a call. `seed=3` (the word seed, an equals sign) is the seed argument written by name: it reads as
    /// the next positional argument.
    fn arg(&mut self) -> Result<Expr, String> {
        if let (Some(Tok::Ident(n)), Some(Tok::Op("="))) = (self.toks.get(self.pos), self.toks.get(self.pos + 1)) {
            if normalize(n) == "seed" {
                self.pos += 2;
            }
        }
        self.expr()
    }
    fn primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::LParen) => {
                let e = self.expr()?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            Some(Tok::Ident(name)) => {
                if self.peek() == Some(&Tok::LParen) {
                    self.pos += 1;
                    let f = fn_name(&name);
                    if f == "WITH LOOKUP" {
                        let input = self.expr()?;
                        self.expect(Tok::Comma)?;
                        self.expect(Tok::LParen)?;
                        let table = self.table_body()?;
                        self.expect(Tok::RParen)?;
                        self.expect(Tok::RParen)?;
                        return Ok(Expr::WithLookup(Box::new(input), table));
                    }
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        args.push(self.arg()?);
                        while self.peek() == Some(&Tok::Comma) {
                            self.pos += 1;
                            args.push(self.arg()?);
                        }
                    }
                    self.expect(Tok::RParen)?;
                    let f = match alias_of(&f, args.len()) {
                        // only when the target takes this many arguments: `Effect(x)` with one argument is a lookup of that name, not EFFECT
                        Some(t) if function_info(&f).is_none() && (function_info(t).is_some_and(|i| args.len() >= i.min && args.len() <= i.max) || is_verb(t)) => t.to_string(),
                        _ => f,
                    };
                    if let Some(info) = function_info(&f) {
                        if args.len() < info.min || args.len() > info.max {
                            return Err(format!("{} needs {}", info.name, info.signature));
                        }
                        Ok(Expr::Call(f, args))
                    } else {
                        if args.len() != 1 {
                            return Err(format!("Unknown function '{name}'"));
                        }
                        Ok(Expr::LookupCall(
                            normalize(&name),
                            Box::new(args.pop().unwrap()),
                        ))
                    }
                } else {
                    // `Name[item]` picks one item of a set variable; for reading the formula it
                    // is still a reference to `Name` (see sets.rs for what it means when run)
                    if self.peek() == Some(&Tok::LBracket)
                        && matches!(self.toks.get(self.pos + 1), Some(Tok::Ident(_)))
                        && self.toks.get(self.pos + 2) == Some(&Tok::RBracket)
                    {
                        self.pos += 3;
                    }
                    Ok(Expr::Var(normalize(&name)))
                }
            }
            Some(t) => Err(format!("Unexpected {}", describe(&t))),
            None => Err("The formula ends too early".into()),
        }
    }
}

// ----------------------------------------------------------------------------- verbs with named options

/// The verbs: one function with options, in place of a family of functions. A verb call is rewritten, as text and
/// before a formula is read or written to another format, into the engine's own functions; the model file keeps
/// the readable form.
pub fn is_verb(name: &str) -> bool {
    matches!(name.to_uppercase().as_str(), "DELAY" | "VALUE" | "QUEUE" | "WAIT" | "EXPECT" | "LOOK" | "LEVELISED" | "LOAN" | "TAX")
}

/// An option as written: a word (`kind=information`, `exact=True`) or an expression's text.
#[derive(Clone, Debug)]
enum Opt {
    Word(String),
    Text(String),
}

/// Does `word` name a choice of option `opt` of verb `f` (so `kind=information` reads `information` as a word,
/// not as a variable)?
fn is_option_word(f: &str, opt: &str, word: &str) -> bool {
    let w = word.to_lowercase();
    match (f, normalize(opt).as_str()) {
        ("DELAY", "kind") => matches!(w.as_str(), "material" | "information" | "info"),
        ("VALUE", "timing") | ("LEVELISED", "timing") => matches!(w.as_str(), "continuous" | "period_end" | "period_start" | "mid_period"),
        ("LOAN", "type") => matches!(w.as_str(), "annuity" | "linear" | "bullet"),
        ("LOAN", "measure") => matches!(w.as_str(), "balance" | "payment" | "interest" | "principal"),
        ("TAX", "carryforward") => matches!(w.as_str(), "yes" | "no"),
        ("WAIT", "measure") => matches!(w.as_str(), "average" | "oldest" | "count"),
        ("EXPECT", "measure") => matches!(w.as_str(), "level" | "trend" | "forecast"),
        ("VALUE", "measure") => matches!(w.as_str(), "value" | "irr"),
        ("LOOK", "mode") => matches!(w.as_str(), "lookup" | "forward" | "backward" | "inverse" | "extrapolate" | "slope" | "area"),
        _ => false,
    }
}

/// Reject any option the verb does not have, and say which it has.
fn check_options(verb: &str, named: &[(String, Opt)], allowed: &[&str]) -> Result<(), String> {
    for (k, _) in named {
        if !allowed.iter().any(|a| normalize(a) == *k) {
            let near = allowed.iter().find(|a| a.starts_with(&k[..k.len().min(2)]) || k.starts_with(&a[..a.len().min(2)]));
            return Err(match near {
                Some(n) => format!("{verb} has no option '{k}' — did you mean '{n}'? Options: {}", allowed.join(", ")),
                None => format!("{verb} has no option '{k}'. Options: {}", allowed.join(", ")),
            });
        }
    }
    Ok(())
}

fn take_opt(named: &mut Vec<(String, Opt)>, key: &str) -> Option<Opt> {
    let key = normalize(key);
    named.iter().position(|(k, _)| *k == key).map(|i| named.remove(i).1)
}

fn opt_flag(named: &mut Vec<(String, Opt)>, key: &str, verb: &str) -> Result<Option<bool>, String> {
    Ok(match take_opt(named, key) {
        None => None,
        Some(Opt::Word(w)) if w == "true" => Some(true),
        Some(Opt::Word(w)) if w == "false" => Some(false),
        Some(Opt::Text(t)) if t.trim() == "1" => Some(true),
        Some(Opt::Text(t)) if t.trim() == "0" => Some(false),
        Some(_) => return Err(format!("{verb}: {key} is True or False")),
    })
}

/// One verb call → the engine's function, from the call's arguments as text.
fn lower_verb(f: &str, mut pos: Vec<String>, mut named: Vec<(String, Opt)>) -> Result<String, String> {
    match f {
        "DELAY" => {
            // DELAY(x, time, [order], init=, kind=material|information, exact=)
            check_options("DELAY", &named, &["order", "init", "kind", "exact", "leak"])?;
            if pos.len() < 2 {
                return Err("DELAY needs the thing to delay and the delay time: DELAY(x, time)".into());
            }
            if pos.len() > 3 {
                return Err("DELAY takes x, time and optionally the order; give the rest by name: init=, kind=, exact=".into());
            }
            let order_pos = if pos.len() == 3 { pos.pop() } else { None };
            let order_named = take_opt(&mut named, "order");
            if order_pos.is_some() && order_named.is_some() {
                return Err("DELAY: the order is given twice".into());
            }
            let order_text = order_pos.or(match order_named {
                Some(Opt::Text(t)) => Some(t),
                Some(Opt::Word(_)) => return Err("DELAY: order is a whole number from 1 to 20".into()),
                None => None,
            });
            let order = match order_text.as_deref().map(|t| t.trim().parse::<f64>()) {
                None => 1usize,
                Some(Ok(n)) if n.fract() == 0.0 && (1.0..=20.0).contains(&n) => n as usize,
                Some(_) => return Err("DELAY: order is a whole number from 1 to 20, written as a number".into()),
            };
            let init = match take_opt(&mut named, "init") {
                Some(Opt::Text(t)) => Some(t),
                Some(Opt::Word(w)) => Some(w),
                None => None,
            };
            let information = match take_opt(&mut named, "kind") {
                None => false,
                Some(Opt::Word(w)) if w == "material" => false,
                Some(Opt::Word(w)) if w == "information" || w == "info" => true,
                Some(_) => return Err("DELAY: kind is material or information".into()),
            };
            let exact = opt_flag(&mut named, "exact", "DELAY")?.unwrap_or(false);
            let leak = match take_opt(&mut named, "leak") {
                Some(Opt::Text(t)) => Some(t),
                Some(Opt::Word(w)) => Some(w),
                None => None,
            };
            let (x, t) = (pos.remove(0), pos.remove(0));
            if leak.is_some() && !exact {
                return Err("DELAY: leak= is for an exact pipeline (a conveyor): write exact=True".into());
            }
            let call = |name: &str, args: Vec<&str>| format!("{name}({})", args.join(", "));
            if exact {
                if order != 1 {
                    return Err("DELAY: exact=True is a pipeline with one fixed transit time; it has no order".into());
                }
                // the pipeline holds the initial input until the first arrival
                if let Some(l) = leak {
                    // a conveyor: each step everything on it loses the fraction `leak`, so what enters comes out
                    // reduced by (1 - leak × step) once for every step of the transit
                    let scaled = format!("(({x}) * (1 - ({l}) * TIME STEP) ^ (({t}) / TIME STEP))");
                    return Ok(call("DELAY FIXED", vec![&scaled, &t, init.as_deref().unwrap_or(&x)]));
                }
                return Ok(call("DELAY FIXED", vec![&x, &t, init.as_deref().unwrap_or(&x)]));
            }
            let n = order.to_string();
            Ok(match (information, order, init.as_deref()) {
                (false, 1, None) => call("DELAY1", vec![&x, &t]),
                (false, 1, Some(i)) => call("DELAY1I", vec![&x, &t, i]),
                (false, 3, None) => call("DELAY3", vec![&x, &t]),
                (false, 3, Some(i)) => call("DELAY3I", vec![&x, &t, i]),
                (false, _, None) => call("DELAYMTR", vec![&x, &t, &n]),
                (false, _, Some(i)) => call("DELAYMTR", vec![&x, &t, &n, i]),
                (true, 1, None) => call("SMOOTH", vec![&x, &t]),
                (true, 1, Some(i)) => call("SMOOTHI", vec![&x, &t, i]),
                (true, 3, None) => call("SMOOTH3", vec![&x, &t]),
                (true, 3, Some(i)) => call("SMOOTH3I", vec![&x, &t, i]),
                (true, _, None) => call("DELAYINF", vec![&x, &t, &n]),
                (true, _, Some(i)) => call("DELAYINF", vec![&x, &t, &n, i]),
            })
        }
        "VALUE" => {
            // VALUE(stream, rate, init=, factor=, timing=): present value · VALUE(stream, measure=irr, init=): the rate of
            // return that makes it worth nothing · VALUE(stream, writeoff=time): straight-line write-off
            check_options("VALUE", &named, &["init", "factor", "timing", "measure", "writeoff", "period"])?;
            let text = |o: Option<Opt>, default: &str| match o {
                Some(Opt::Text(t)) => t,
                Some(Opt::Word(w)) => w,
                None => default.to_string(),
            };
            let measure = match take_opt(&mut named, "measure") {
                None => "value".to_string(),
                Some(Opt::Word(w)) => w,
                Some(_) => return Err("VALUE: measure is value or irr".into()),
            };
            let writeoff = take_opt(&mut named, "writeoff").map(|o| text(Some(o), ""));
            if let Some(w) = writeoff {
                if pos.len() != 1 || !named.is_empty() {
                    return Err("VALUE(stream, writeoff=time) takes the stream and the write-off time, nothing else".into());
                }
                return Ok(format!("DEPRECIATE STRAIGHTLINE({}, {w})", pos[0]));
            }
            if measure == "irr" {
                if pos.len() != 1 {
                    return Err("VALUE(stream, measure=irr) takes the stream; the rate is what it finds".into());
                }
                let init = text(take_opt(&mut named, "init"), "0");
                let period = text(take_opt(&mut named, "period"), "1");
                if take_opt(&mut named, "factor").is_some() || take_opt(&mut named, "timing").is_some() {
                    return Err("VALUE: factor= and timing= go with a discount rate, not with measure=irr".into());
                }
                return Ok(format!("INTERNAL RATE OF RETURN({}, {period}, {init})", pos[0]));
            }
            if pos.len() != 2 {
                return Err("VALUE needs the stream of money and the discount rate: VALUE(Cash Flow, 0.08)".into());
            }
            let init = text(take_opt(&mut named, "init"), "0");
            let factor = text(take_opt(&mut named, "factor"), "1");
            let period_end = match take_opt(&mut named, "timing") {
                None => false,
                Some(Opt::Word(w)) if w == "continuous" => false,
                Some(Opt::Word(w)) if w == "period_end" => true,
                // discount factor (1 + r)^-(k + 0): the flow of a step counts at its start, or (k + 0.5) in the middle
                Some(Opt::Word(w)) if w == "period_start" => return Ok(format!("NPV AT({}, {}, {init}, {factor}, 0)", pos[0], pos[1])),
                Some(Opt::Word(w)) if w == "mid_period" => return Ok(format!("NPV AT({}, {}, {init}, {factor}, 0.5)", pos[0], pos[1])),
                Some(_) => return Err("VALUE: timing is continuous, period_end, period_start or mid_period".into()),
            };
            let name = if period_end { "NPVE" } else { "NPV" };
            Ok(format!("{name}({}, {}, {init}, {factor})", pos[0], pos[1]))
        }
        "LEVELISED" => {
            // LEVELISED(cost stream, quantity stream, rate, [initial cost=0], timing=period_end)
            check_options("LEVELISED", &named, &["timing", "init"])?;
            if pos.len() < 3 || pos.len() > 4 {
                return Err("LEVELISED(cost stream, quantity stream, rate, [initial cost]): the cost per unit of quantity with both discounted".into());
            }
            let init = pos.get(3).cloned().or_else(|| match take_opt(&mut named, "init") { Some(Opt::Text(t)) => Some(t), Some(Opt::Word(w)) => Some(w), None => None }).unwrap_or_else(|| "0".into());
            let code = match take_opt(&mut named, "timing") {
                None => 2,
                Some(Opt::Word(w)) => match w.as_str() {
                    "period_start" => 0,
                    "mid_period" => 1,
                    "period_end" => 2,
                    _ => return Err("LEVELISED: timing is period_end, period_start or mid_period".into()),
                },
                Some(_) => return Err("LEVELISED: timing is period_end, period_start or mid_period".into()),
            };
            Ok(format!("LEVELISED STATE({}, {}, {}, {init}, {code})", pos[0], pos[1], pos[2]))
        }
        "LOAN" => {
            // LOAN(drawdown, rate, term, type=annuity|linear|bullet, grace=0, measure=balance|payment|interest|principal)
            check_options("LOAN", &named, &["type", "grace", "measure"])?;
            if pos.len() != 3 {
                return Err("LOAN(drawdown, rate, term, type=, grace=, measure=)".into());
            }
            let ty = match take_opt(&mut named, "type") { None => 0, Some(Opt::Word(w)) => match w.as_str() { "annuity" => 0, "linear" => 1, _ => 2 }, Some(_) => return Err("LOAN: type is annuity, linear or bullet".into()) };
            let me = match take_opt(&mut named, "measure") { None => 0, Some(Opt::Word(w)) => match w.as_str() { "balance" => 0, "payment" => 1, "interest" => 2, _ => 3 }, Some(_) => return Err("LOAN: measure is balance, payment, interest or principal".into()) };
            let grace = match take_opt(&mut named, "grace") { Some(Opt::Text(t)) => t, Some(Opt::Word(w)) => w, None => "0".into() };
            Ok(format!("LOAN STATE({}, {}, {}, {ty}, {grace}, {me})", pos[0], pos[1], pos[2]))
        }
        "TAX" => {
            // TAX(taxable income, rate, carryforward=yes, years=0)
            check_options("TAX", &named, &["carryforward", "years"])?;
            if pos.len() != 2 {
                return Err("TAX(taxable income, rate, carryforward=yes, years=0)".into());
            }
            let cf = match take_opt(&mut named, "carryforward") { None => 1, Some(Opt::Word(w)) if w == "yes" => 1, Some(Opt::Word(w)) if w == "no" => 0, Some(_) => return Err("TAX: carryforward is yes or no".into()) };
            let years = match take_opt(&mut named, "years") { Some(Opt::Text(t)) => t, Some(Opt::Word(w)) => w, None => "0".into() };
            Ok(format!("TAX STATE({}, {}, {cf}, {years})", pos[0], pos[1]))
        }
        "EXPECT" => {
            // EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)
            check_options("EXPECT", &named, &["time", "ahead", "init", "measure"])?;
            let text = |o: Option<Opt>| match o {
                Some(Opt::Text(t)) => Some(t),
                Some(Opt::Word(w)) => Some(w),
                None => None,
            };
            let time_named = text(take_opt(&mut named, "time"));
            if pos.len() > 2 || pos.is_empty() || (pos.len() == 2 && time_named.is_some()) {
                return Err("EXPECT needs the thing to look at and the time it takes to notice: EXPECT(Sales, 6)".into());
            }
            let x = pos.remove(0);
            let t = pos.pop().or(time_named).ok_or("EXPECT: how long does it take to notice? EXPECT(x, time)")?;
            let ahead = text(take_opt(&mut named, "ahead"));
            let init = text(take_opt(&mut named, "init"));
            let measure = match take_opt(&mut named, "measure") {
                None => if ahead.is_some() { "forecast".to_string() } else { "level".to_string() },
                Some(Opt::Word(w)) => w,
                Some(_) => return Err("EXPECT: measure is level, trend or forecast".into()),
            };
            match measure.as_str() {
                "level" => {
                    if ahead.is_some() {
                        return Err("EXPECT: ahead= goes with measure=forecast".into());
                    }
                    Ok(match init {
                        Some(i) => format!("SMOOTHI({x}, {t}, {i})"),
                        None => format!("SMOOTH({x}, {t})"),
                    })
                }
                "trend" => {
                    if ahead.is_some() {
                        return Err("EXPECT: ahead= goes with measure=forecast".into());
                    }
                    Ok(format!("TREND({x}, {t}, {})", init.unwrap_or_else(|| "0".into())))
                }
                "forecast" => {
                    let h = ahead.ok_or("EXPECT: measure=forecast needs ahead=, how far to look")?;
                    Ok(format!("(({x}) * (1 + TREND({x}, {t}, {}) * ({h})))", init.unwrap_or_else(|| "0".into())))
                }
                _ => Err("EXPECT: measure is level, trend or forecast".into()),
            }
        }
        "LOOK" => {
            // LOOK(curve, x, mode=, start=, outside=): a lookup read in the way asked for
            check_options("LOOK", &named, &["mode", "start", "outside"])?;
            if pos.len() != 2 {
                return Err("LOOK needs the curve and where to read it: LOOK(Effect of price, Price)".into());
            }
            let text = |o: Option<Opt>, default: &str| match o {
                Some(Opt::Text(t)) => t,
                Some(Opt::Word(w)) => w,
                None => default.to_string(),
            };
            let mode = match take_opt(&mut named, "mode") {
                None => "lookup".to_string(),
                Some(Opt::Word(w)) => w,
                Some(_) => return Err("LOOK: mode is lookup, forward, backward, inverse, extrapolate, slope or area".into()),
            };
            let start = take_opt(&mut named, "start");
            let outside = take_opt(&mut named, "outside");
            if (start.is_some() && mode != "area") || (outside.is_some() && mode != "slope") {
                return Err("LOOK: start= goes with mode=area and outside= with mode=slope".into());
            }
            let (c, x) = (&pos[0], &pos[1]);
            Ok(match mode.as_str() {
                "lookup" => format!("{c}({x})"),
                "forward" => format!("LOOKUP FORWARD({c}, {x})"),
                "backward" => format!("LOOKUP BACKWARD({c}, {x})"),
                "inverse" => format!("LOOKUP INVERT({c}, {x})"),
                "extrapolate" => format!("LOOKUP EXTRAPOLATE({c}, {x})"),
                "slope" => format!("LOOKUP SLOPE({c}, {x}, {})", text(outside, "0")),
                "area" => format!("LOOKUP AREA({c}, {}, {x})", text(start, "0")),
                _ => return Err("LOOK: mode is lookup, forward, backward, inverse, extrapolate, slope or area".into()),
            })
        }
        "QUEUE" => {
            // QUEUE(arrivals, served, init=): a waiting line that remembers how long each has waited
            check_options("QUEUE", &named, &["init"])?;
            if pos.len() != 2 {
                return Err("QUEUE needs what arrives and what is served: QUEUE(Arrivals, Served)".into());
            }
            let init = match take_opt(&mut named, "init") {
                Some(Opt::Text(t)) => t,
                Some(Opt::Word(w)) => w,
                None => "0".to_string(),
            };
            Ok(format!("QUEUE FIFO({}, {}, {init})", pos[0], pos[1]))
        }
        "WAIT" => {
            // WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)
            check_options("WAIT", &named, &["measure", "among", "older_than", "younger_than"])?;
            if pos.len() != 1 {
                return Err("WAIT needs the name of a queue: WAIT(Waiting)".into());
            }
            let measure = match take_opt(&mut named, "measure") {
                None => "average".to_string(),
                Some(Opt::Word(w)) => w,
                Some(_) => return Err("WAIT: measure is average, oldest or count".into()),
            };
            let text = |o: Option<Opt>| match o {
                Some(Opt::Text(t)) => Some(t),
                Some(Opt::Word(w)) => Some(w),
                None => None,
            };
            let (among, older, younger) = (text(take_opt(&mut named, "among")), text(take_opt(&mut named, "older_than")), text(take_opt(&mut named, "younger_than")));
            let q = &pos[0];
            match measure.as_str() {
                "average" => {
                    if older.is_some() || younger.is_some() {
                        return Err("WAIT: older_than= and younger_than= go with measure=count".into());
                    }
                    Ok(match among {
                        Some(a) => format!("QUEUE AGE AVERAGE({q}, {a})"),
                        None => format!("QUEUE AGE AVERAGE({q})"),
                    })
                }
                "oldest" => {
                    if among.is_some() || older.is_some() || younger.is_some() {
                        return Err("WAIT: measure=oldest takes no other options".into());
                    }
                    Ok(format!("QUEUE AGE OLDEST({q})"))
                }
                "count" => {
                    if among.is_some() {
                        return Err("WAIT: among= goes with measure=average".into());
                    }
                    Ok(format!("QUEUE AGE IN RANGE({q}, {}, {})", older.unwrap_or_else(|| "0".into()), younger.unwrap_or_else(|| "1000000000000".into())))
                }
                _ => Err("WAIT: measure is average, oldest or count".into()),
            }
        }
        _ => Err(format!("Unknown function '{f}'")),
    }
}

/// For the call whose name is token `i` (followed by `(`): the index of its closing parenthesis and the byte range
/// of each argument's text.
pub fn call_pieces(src: &str, toks: &[(Tok, Range<usize>)], i: usize) -> Option<(usize, Vec<(usize, usize)>)> {
    let mut level = 0i32;
    let mut cuts: Vec<usize> = Vec::new();
    let mut j = i + 1;
    let mut close = None;
    while j < toks.len() {
        match &toks[j].0 {
            Tok::LParen | Tok::LBracket => level += 1,
            Tok::RParen | Tok::RBracket => {
                level -= 1;
                if level == 0 {
                    close = Some(j);
                    break;
                }
            }
            Tok::Comma if level == 1 => cuts.push(j),
            _ => {}
        }
        j += 1;
    }
    let close = close?;
    let mut pieces = Vec::new();
    let mut start = toks[i + 1].1.end;
    for c in &cuts {
        pieces.push((start, toks[*c].1.start));
        start = toks[*c].1.end;
    }
    if close > i + 2 || !cuts.is_empty() {
        pieces.push((start, toks[close].1.start));
    }
    let _ = src;
    Some((close, pieces))
}

/// Split call arguments into plain ones and `name=value` options (the value as text).
pub fn split_options(src: &str, pieces: &[(usize, usize)]) -> Result<(Vec<String>, Vec<(String, String)>), String> {
    let (mut pos, mut named): (Vec<String>, Vec<(String, String)>) = (Vec::new(), Vec::new());
    for (a, b) in pieces {
        let text = src[*a..*b].trim().to_string();
        let t2 = tokenize(&text).unwrap_or_default();
        match (t2.first(), t2.get(1)) {
            (Some((Tok::Ident(opt), _)), Some((Tok::Op("="), eq))) => {
                let key = normalize(opt);
                if named.iter().any(|(k, _)| *k == key) {
                    return Err(format!("the option '{opt}' is given twice"));
                }
                named.push((key, text[eq.end..].trim().to_string()));
            }
            _ => {
                if !named.is_empty() {
                    return Err("options with a name (like by=Priority) come after the plain arguments".into());
                }
                pos.push(text);
            }
        }
    }
    Ok((pos, named))
}

/// Say which options a verb has when it is given one it does not.
pub fn unknown_option(verb: &str, named: &[(String, String)], allowed: &[&str]) -> Result<(), String> {
    let as_opts: Vec<(String, Opt)> = named.iter().map(|(k, v)| (k.clone(), Opt::Text(v.clone()))).collect();
    check_options(verb, &as_opts, allowed)
}

/// Rewrite every verb call in `src` into the engine's functions (verbs inside verbs first). Text without a verb
/// is returned unchanged.
pub fn expand_verbs(src: &str) -> Result<String, String> {
    let Ok(toks) = tokenize(src) else { return Ok(src.to_string()) };
    let mut out = String::new();
    let mut last = 0;
    let mut i = 0;
    while i < toks.len() {
        if let (Tok::Ident(name), span) = &toks[i] {
            if is_verb(&fn_name(name)) && matches!(toks.get(i + 1), Some((Tok::LParen, _))) {
                // the call's closing parenthesis, and the commas that separate its arguments
                let mut level = 0i32;
                let mut cuts: Vec<usize> = Vec::new();
                let mut j = i + 1;
                let mut close = None;
                while j < toks.len() {
                    match &toks[j].0 {
                        Tok::LParen | Tok::LBracket => level += 1,
                        Tok::RParen | Tok::RBracket => {
                            level -= 1;
                            if level == 0 {
                                close = Some(j);
                                break;
                            }
                        }
                        Tok::Comma if level == 1 => cuts.push(j),
                        _ => {}
                    }
                    j += 1;
                }
                let Some(close) = close else { return Ok(src.to_string()) };
                let mut pieces: Vec<(usize, usize)> = Vec::new(); // byte ranges of each argument's text
                let mut start = toks[i + 1].1.end;
                for c in &cuts {
                    pieces.push((start, toks[*c].1.start));
                    start = toks[*c].1.end;
                }
                if close > i + 2 || !cuts.is_empty() {
                    pieces.push((start, toks[close].1.start));
                }
                let f = fn_name(name);
                let (mut pos, mut named): (Vec<String>, Vec<(String, Opt)>) = (Vec::new(), Vec::new());
                for (a, b) in pieces {
                    let text = expand_verbs(src[a..b].trim())?;
                    let t2 = tokenize(&text).unwrap_or_default();
                    match (t2.first(), t2.get(1)) {
                        (Some((Tok::Ident(opt), _)), Some((Tok::Op("="), eq_span))) => {
                            let value = text[eq_span.end..].trim().to_string();
                            let single_word = tokenize(&value).map(|v| v.len() == 1 && matches!(v[0].0, Tok::Ident(_))).unwrap_or(false);
                            let key = normalize(opt);
                            if named.iter().any(|(k, _)| *k == key) {
                                return Err(format!("{f}: the option '{opt}' is given twice"));
                            }
                            let v = if single_word && (matches!(value.to_lowercase().as_str(), "true" | "false") || is_option_word(&f, opt, &value)) {
                                Opt::Word(value.to_lowercase())
                            } else {
                                Opt::Text(value)
                            };
                            named.push((key, v));
                        }
                        _ => {
                            if !named.is_empty() {
                                return Err(format!("{f}: options with a name (like order=3) come after the plain arguments"));
                            }
                            pos.push(text);
                        }
                    }
                }
                out.push_str(&src[last..span.start]);
                out.push_str(&lower_verb(&f, pos, named)?);
                last = toks[close].1.end;
                i = close + 1;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&src[last..]);
    Ok(out)
}

/// Every variable name the expression refers to (normalised), including lookups.
pub fn references(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Num(_) => {}
        Expr::Var(n) => {
            if !out.contains(n) {
                out.push(n.clone())
            }
        }
        Expr::Neg(a) | Expr::Not(a) => references(a, out),
        Expr::Bin(_, a, b) => {
            references(a, out);
            references(b, out);
        }
        // the second argument of TOTAL / AVERAGE is a group label, not a variable
        Expr::Call(f, args) if f == "TOTAL" || f == "AVERAGE" => {
            if let Some(a) = args.first() {
                references(a, out);
            }
        }
        Expr::Call(_, args) => args.iter().for_each(|a| references(a, out)),
        Expr::LookupCall(n, a) => {
            if !out.contains(n) {
                out.push(n.clone())
            }
            references(a, out);
        }
        Expr::WithLookup(a, _) => references(a, out),
    }
}

/// Replace every reference to `old` (normalised) in `src` with `new`, keeping the rest of the text.
pub fn rename_in(src: &str, old: &str, new: &str) -> String {
    let Ok(toks) = tokenize(src) else {
        return src.to_string();
    };
    let mut out = String::new();
    let mut last = 0;
    for (t, span) in toks {
        if let Tok::Ident(s) = t {
            if normalize(&s) == old {
                out.push_str(&src[last..span.start]);
                out.push_str(new);
                last = span.end;
            }
        }
    }
    out.push_str(&src[last..]);
    out
}

/// Linear interpolation in a lookup table, clamped at both ends.
pub fn lookup(table: &[[f64; 2]], x: f64) -> f64 {
    if table.is_empty() {
        return 0.0;
    }
    if x <= table[0][0] {
        return table[0][1];
    }
    for w in table.windows(2) {
        let (a, b) = (w[0], w[1]);
        if x <= b[0] {
            let span = b[0] - a[0];
            if span <= 0.0 {
                return b[1];
            }
            return a[1] + (b[1] - a[1]) * (x - a[0]) / span;
        }
    }
    table[table.len() - 1][1]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_names_with_spaces() {
        let e = parse("Contact Rate * Infected / IF THEN ELSE(Time > 5, 2, 1)").unwrap();
        let mut r = vec![];
        references(&e, &mut r);
        assert_eq!(r, vec!["contact rate", "infected", "time"]);
    }
    #[test]
    fn parses_with_lookup() {
        let e = parse("WITH LOOKUP(Time, ([(0,0)-(10,10)],(0,0),(5,2),(10,1)))").unwrap();
        assert!(matches!(e, Expr::WithLookup(_, ref t) if t.len() == 3));
    }
    #[test]
    fn renames() {
        assert_eq!(rename_in("a b + A_B*2", "a b", "c"), "c + c*2");
    }
}
