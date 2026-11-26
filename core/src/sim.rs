//! Turns a model into a runnable form and simulates it over time.

use std::collections::HashMap;

use crate::expr::{self, BinOp, Expr};
use crate::model::{Kind, Method, Model};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Pure {
    DateFn(u8),
    /// random draws (`stats::RANDOMS`): the code and the number of parameters; the last argument is the call's own stream key
    Rnd(u8, usize),
    /// NORMAL CDF, NORMAL INV, LOGNORMAL CDF, LOGNORMAL INV
    Dist(u8),
    /// AND, OR, NOT, BETWEEN
    Logic(u8),
    /// PROFILE (0 interpolated, 1 step), SEASON (2), DAYLIGHT (3), AT TIME (4), EVERY (5)
    Seas(u8),
    /// scalar finance (`finance::FUNCTIONS`)
    Fin(u8),
    /// calendar functions: 0 DAYS IN STEP, 1 HOURS IN STEP, 2 YEAR FRACTION
    Cal(u8),
    VecFn(u8),
    GammaLn,
    RandExp,
    Quantum,
    Min,
    Max,
    Abs,
    Sqrt,
    Exp,
    Ln,
    Log,
    Pow,
    Int,
    Ceil,
    SinWave,
    CosWave,
    Graph,
    GraphStep,
    Sinh,
    Cosh,
    Tanh,
    Asin,
    Acos,
    Atan2,
    Floor,
    Sign,
    Log10,
    Log2,
    Cbrt,
    Hypot,
    Rad,
    Deg,
    Clamp,
    Mean,
    Median,
    Pi,
    Erf,
    Gamma,
    Fact,
    Round,
    Mod,
    Sin,
    Cos,
    Tan,
    Atan,
    Xidz,
    Zidz,
    Step,
    Pulse,
    PulseTrain,
    Ramp,
    RandU,
    RandN,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SKind {
    Smooth,
    Smooth3,
    Delay1,
    Delay3,
    DelayFixed,
    Trend,
    Initial,
    /// SAMPLE IF TRUE(condition, input, initial): the input when the condition holds, else the value held.
    SampleIfTrue,
    /// NPV(stream, discount rate, initial value, factor): two hidden stocks, the discount factor and the
    /// discounted sum (Vensim's definition, as SDEverywhere writes it out).
    Npv,
    /// NPVE: as NPV but with the discrete-period discount factor Excel uses.
    Npve,
    /// DERIVN(x, order): the nth backward difference of x over the time step.
    Derivn,
    /// DEPRECIATE STRAIGHTLINE(input, depreciation time, …): each input spread evenly over the depreciation time.
    Depreciate,
    /// INTERNAL RATE OF RETURN(stream, period, initial): the rate that makes the stream so far worth nothing.
    Irr,
    /// QUEUE FIFO(inflow, outflow, initial): a level that remembers how long each part has waited. Slots: the
    /// total that has come in, the total that has gone out, the initial content. The oldest leaves first.
    Queue,
    /// NPV AT(stream, rate, init, factor, offset, period): the flow of a step is discounted by (1 + r)^-(k + offset),
    /// k = (t - t0) / period of the rate. Slots: the discounted sum.
    NpvAt,
    /// LEVELISED STATE(cost, quantity, rate, init, timing, period): slots: present value of costs, of quantity.
    Levelised,
    /// PAYBACK(stream, rate, period): slots: cumulative, crossing time (-1e300 until found), has been negative.
    Payback,
    /// LOAN STATE(draw, rate, term, type, grace, measure, rate period, term scale): tranches kept in the history.
    Loan,
    /// TAX STATE(income, rate, carryforward, years, years scale): the unused losses kept in the history.
    Tax,
    /// PREVIOUS(x, initial): x as it was one step ago.
    Previous,
    /// ROUND WITH CARRY(flow): whole units moved, the remainder carried (slot: the remainder).
    Carry,
    /// HISTORY(x, time): x as it was at an earlier time.
    History,
}

#[derive(Clone, Debug)]
enum C {
    Num(f64),
    Var(usize),
    Time,
    T0,
    T1,
    Dt,
    Neg(Box<C>),
    Not(Box<C>),
    Bin(BinOp, Box<C>, Box<C>),
    If(Box<C>, Box<C>, Box<C>),
    Pure(Pure, Vec<C>),
    Table(usize, Box<C>),
    /// ACTIVE INITIAL(x, init): x during the run, init while initial values are computed.
    ActiveInit(Box<C>, Box<C>),
    /// A lookup read in another mode: 1 forward (next point's y), 2 backward (previous point's y),
    /// 3 invert (x for a given y), 4 extrapolate (straight lines beyond the end points).
    TableMode(usize, u8, Vec<C>),
    /// What a queue says about its contents: 1 average age of the oldest `n`, 2 age of the oldest, 3 amount between
    /// two ages. First field: the variable that holds the queue.
    QueueInfo(usize, u8, Vec<C>),
    /// Output of a stateful function (smooth, delay...).
    State(usize),
}

struct SNode {
    kind: SKind,
    /// input, delay time, optional initial value
    args: Vec<C>,
    has_init: bool,
    slot: usize,
    width: usize,
    /// DELAY FIXED history
    hist: usize,
}

pub struct Compiled {
    pub names: Vec<String>,
    kinds: Vec<Kind>,
    exprs: Vec<Option<C>>,
    stock_slot: Vec<Option<usize>>,
    stock_flows: Vec<(usize, Vec<usize>, Vec<usize>)>,
    non_negative: Vec<bool>,
    order_run: Vec<usize>,
    order_init: Vec<usize>,
    snodes: Vec<SNode>,
    n_states: usize,
    n_hist: usize,
    tables: Vec<Vec<[f64; 2]>>,
    /// variables whose value is a series (everything except pure lookup functions)
    pub has_series: Vec<bool>,
    /// set variables: the items to combine into one number (see `sets.rs`)
    rollups: Vec<crate::sets::Rollup>,
    /// the calendar (start date, unit, convention), when the model has one
    calendar: Option<crate::calendar::CalCtx>,
    /// Calendar convention: per flow, how many of its own time unit make one step of the clock (a flow declared per
    /// Day in a monthly calendar model counts the real days of each step; per Hour, the real hours)
    flow_per: Vec<Option<FlowPer>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FlowPer {
    Day,
    Hour,
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub var: Option<String>,
    pub message: String,
}

struct Builder<'a> {
    model: &'a Model,
    /// the variable being compiled and how many random calls it has had so far
    cur: String,
    calls: u64,
    calendar: Option<crate::calendar::CalCtx>,
    index: HashMap<String, usize>,
    tables: Vec<Vec<[f64; 2]>>,
    table_of: HashMap<usize, usize>,
    snodes: Vec<SNode>,
    n_states: usize,
    n_hist: usize,
}

impl Builder<'_> {
    /// Seconds in the model's time unit.
    fn model_unit_secs(&self) -> Option<f64> {
        let ctx = crate::units::UnitCtx::of(self.model);
        let u = crate::units::parse_unit_in(&self.model.settings.time_units, &ctx).ok()?;
        (u.dims.len() == 1 && u.dims.contains_key("s")).then_some(u.scale)
    }
    /// The declared unit of a bare variable argument.
    fn arg_unit(&self, e: &Expr) -> Option<crate::units::Unit> {
        let Expr::Var(n) = e else { return None };
        let v = self.model.by_name(n)?;
        crate::units::parse_unit_in(&v.units, &crate::units::UnitCtx::of(self.model)).ok()
    }
    /// A rate's period in model time units: a rate declared per Year in a monthly model has a period of 12.
    /// A rate with no time in its unit is per model time unit.
    fn rate_period(&self, e: &Expr) -> f64 {
        match (self.arg_unit(e), self.model_unit_secs()) {
            (Some(u), Some(m)) if u.dims.len() == 1 && u.dims.get("s") == Some(&-1) => (1.0 / u.scale) / m,
            _ => 1.0,
        }
    }
    /// How many model time units one unit of a time-valued argument is (a term of 30 Year in a monthly model: 12 per unit).
    fn time_scale(&self, e: &Expr) -> f64 {
        match (self.arg_unit(e), self.model_unit_secs()) {
            (Some(u), Some(m)) if u.dims.len() == 1 && u.dims.get("s") == Some(&1) => u.scale / m,
            _ => 1.0,
        }
    }
    fn year_in_model_units(&self) -> f64 {
        self.model_unit_secs().map(|m| self.model.settings.year_days_abstract * 86_400.0 / m).unwrap_or(1.0)
    }
    fn table_for(&mut self, var: usize) -> usize {
        if let Some(t) = self.table_of.get(&var) {
            return *t;
        }
        let mut t = self.model.vars[var].table.clone();
        t.sort_by(|a, b| a[0].total_cmp(&b[0]));
        self.tables.push(t);
        self.table_of.insert(var, self.tables.len() - 1);
        self.tables.len() - 1
    }

    fn compile(&mut self, e: &Expr) -> Result<C, String> {
        Ok(match e {
            Expr::Num(n) => C::Num(*n),
            Expr::Var(n) => match n.as_str() {
                "time" => C::Time,
                "initial time" => C::T0,
                "final time" => C::T1,
                "time step" | "dt" => C::Dt,
                _ => {
                    let Some(&i) = self.index.get(n) else {
                        return Err(format!("Unknown name '{n}'"));
                    };
                    let v = &self.model.vars[i];
                    if v.kind == Kind::Lookup && v.equation.trim().is_empty() {
                        return Err(format!(
                            "'{}' is a lookup: use it like {}(x)",
                            v.name, v.name
                        ));
                    }
                    C::Var(i)
                }
            },
            Expr::Neg(a) => C::Neg(Box::new(self.compile(a)?)),
            Expr::Not(a) => C::Not(Box::new(self.compile(a)?)),
            Expr::Bin(op, a, b) => {
                C::Bin(*op, Box::new(self.compile(a)?), Box::new(self.compile(b)?))
            }
            Expr::LookupCall(n, a) => {
                let Some(&i) = self.index.get(n) else {
                    return Err(format!("Unknown function or lookup '{n}'"));
                };
                if self.model.vars[i].table.len() < 1 {
                    return Err(format!("'{}' has no lookup table", self.model.vars[i].name));
                }
                let t = self.table_for(i);
                C::Table(t, Box::new(self.compile(a)?))
            }
            Expr::WithLookup(a, table) => {
                let mut t = table.clone();
                t.sort_by(|a, b| a[0].total_cmp(&b[0]));
                self.tables.push(t);
                C::Table(self.tables.len() - 1, Box::new(self.compile(a)?))
            }
            Expr::Call(f, args) if f == "DELAYINF" || f == "DELAYMTR" => {
                // a chain of `order` first-order stages, each with delay ÷ order
                let order = match args.get(2) {
                    None => 1,
                    Some(Expr::Num(n)) if n.fract() == 0.0 && (1.0..=20.0).contains(n) => *n as usize,
                    Some(_) => return Err(format!("{f}: the order must be a whole number from 1 to 20, written as a number")),
                };
                let mut prev = self.compile(&args[0])?;
                let stage = C::Bin(BinOp::Div, Box::new(self.compile(&args[1])?), Box::new(C::Num(order as f64)));
                // optional 4th argument: the value every stage (and so the output) starts at
                let init = match args.get(3) {
                    Some(a) => Some(self.compile(a)?),
                    None => None,
                };
                for _ in 0..order {
                    let slot = self.n_states;
                    self.n_states += 1;
                    let mut a = vec![prev, stage.clone()];
                    if let Some(i) = &init {
                        a.push(i.clone());
                    }
                    self.snodes.push(SNode {
                        kind: if f == "DELAYINF" { SKind::Smooth } else { SKind::Delay1 },
                        args: a,
                        has_init: init.is_some(),
                        slot,
                        width: 1,
                        hist: self.n_hist,
                    });
                    prev = C::State(self.snodes.len() - 1);
                }
                prev
            }
            Expr::Call(f, args) if matches!(f.as_str(), "QUEUE AGE AVERAGE" | "QUEUE AGE OLDEST" | "QUEUE AGE IN RANGE") => {
                let name = match args.first() {
                    Some(Expr::Var(n)) => n.clone(),
                    _ => return Err(format!("{f}: the first argument must be the name of a queue")),
                };
                let Some(&i) = self.index.get(&name) else {
                    return Err(format!("{f}: unknown queue '{name}'"));
                };
                let mode = match f.as_str() { "QUEUE AGE AVERAGE" => 1, "QUEUE AGE OLDEST" => 2, _ => 3 };
                let rest: Vec<C> = args.iter().skip(1).map(|a| self.compile(a)).collect::<Result<_, _>>()?;
                C::QueueInfo(i, mode, rest)
            }
            Expr::Call(f, args) if matches!(f.as_str(), "LOOKUP FORWARD" | "LOOKUP BACKWARD" | "LOOKUP INVERT" | "LOOKUP EXTRAPOLATE" | "LOOKUP SLOPE" | "LOOKUP AREA"
                | "GET DATA AT TIME" | "GET DATA BETWEEN TIMES" | "GET DATA FIRST TIME" | "GET DATA LAST TIME" | "GET DATA MAX" | "GET DATA MIN" | "GET DATA MEAN" | "GET DATA MEDIAN" | "GET DATA STDV" | "GET DATA TOTAL" | "GET DATA SUM") => {
                // the first argument names a lookup variable
                let name = match args.first() {
                    Some(Expr::Var(n)) => n.clone(),
                    Some(Expr::LookupCall(n, _)) => n.clone(),
                    _ => return Err(format!("{f}: the first argument must be the name of a lookup")),
                };
                let Some(&i) = self.index.get(&name) else {
                    return Err(format!("{f}: unknown lookup '{name}'"));
                };
                if self.model.vars[i].table.is_empty() {
                    return Err(format!("'{}' has no lookup table", self.model.vars[i].name));
                }
                let t = self.table_for(i);
                let mode = match f.as_str() {
                    "LOOKUP FORWARD" => 1,
                    "LOOKUP BACKWARD" => 2,
                    "LOOKUP INVERT" => 3,
                    "LOOKUP EXTRAPOLATE" => 4,
                    "LOOKUP SLOPE" => 5,
                    "LOOKUP AREA" => 6,
                    "GET DATA AT TIME" => 7,
                    "GET DATA BETWEEN TIMES" => 8,
                    "GET DATA FIRST TIME" => 9,
                    "GET DATA LAST TIME" => 10,
                    "GET DATA MAX" => 11,
                    "GET DATA MIN" => 12,
                    "GET DATA MEAN" => 13,
                    "GET DATA MEDIAN" => 14,
                    "GET DATA STDV" => 15,
                    _ => 16,
                };
                let rest: Vec<C> = args.iter().skip(1).map(|a| self.compile(a)).collect::<Result<_, _>>()?;
                C::TableMode(t, mode, rest)
            }
            Expr::Call(f, args) => {
                let mut c: Vec<C> = args
                    .iter()
                    .map(|a| self.compile(a))
                    .collect::<Result<_, _>>()?;
                // the unit of a rate or a term decides how many model time units one "period" or one term unit is
                match f.as_str() {
                    "NPV AT" => c.push(C::Num(self.rate_period(&args[1]))),
                    "LEVELISED STATE" => c.push(C::Num(self.rate_period(&args[2]))),
                    "PAYBACK" => {
                        if c.len() < 2 {
                            c.push(C::Num(0.0));
                        }
                        let p = args.get(1).map(|a| self.rate_period(a)).unwrap_or(1.0);
                        c.push(C::Num(p));
                    }
                    "LOAN STATE" => {
                        c.push(C::Num(self.rate_period(&args[1])));
                        c.push(C::Num(self.time_scale(&args[2])));
                        c.push(C::Num(self.time_scale(&args[4])));
                    }
                    "TAX STATE" => c.push(C::Num(self.year_in_model_units())),
                    "ESCALATE" => {
                        if c.len() < 3 {
                            c.push(C::T0);
                        }
                        c.push(C::Num(self.rate_period(&args[1])));
                    }
                    _ => {}
                }
                let stateful = |k: SKind, w: usize, init: bool| (k, w, init);
                let sk = match f.as_str() {
                    "SMOOTH" => Some(stateful(SKind::Smooth, 1, false)),
                    "SMOOTHI" => Some(stateful(SKind::Smooth, 1, true)),
                    "SMOOTH3" => Some(stateful(SKind::Smooth3, 3, false)),
                    "SMOOTH3I" => Some(stateful(SKind::Smooth3, 3, true)),
                    "DELAY1" => Some(stateful(SKind::Delay1, 1, false)),
                    "DELAY1I" => Some(stateful(SKind::Delay1, 1, true)),
                    "DELAY3" => Some(stateful(SKind::Delay3, 3, false)),
                    "DELAY3I" => Some(stateful(SKind::Delay3, 3, true)),
                    "DELAY FIXED" => Some(stateful(SKind::DelayFixed, 1, true)),
                    "TREND" => Some(stateful(SKind::Trend, 1, true)),
                    "INITIAL" => Some(stateful(SKind::Initial, 1, false)),
                    "SAMPLE IF TRUE" => Some(stateful(SKind::SampleIfTrue, 1, true)),
                    "QUEUE FIFO" => Some(stateful(SKind::Queue, 3, false)),
                    "NPV" => Some(stateful(SKind::Npv, 2, false)),
                    "NPVE" => Some(stateful(SKind::Npve, 2, false)),
                    "NPV AT" => Some(stateful(SKind::NpvAt, 1, false)),
                    "PREVIOUS" => Some(stateful(SKind::Previous, 1, false)),
                    "ROUND WITH CARRY" => Some(stateful(SKind::Carry, 1, false)),
                    "HISTORY" => Some(stateful(SKind::History, 0, false)),
                    "LEVELISED STATE" => Some(stateful(SKind::Levelised, 2, false)),
                    "PAYBACK" => Some(stateful(SKind::Payback, 3, false)),
                    "LOAN STATE" => Some(stateful(SKind::Loan, 0, false)),
                    "TAX STATE" => Some(stateful(SKind::Tax, 0, false)),
                    "INTERNAL RATE OF RETURN" => Some(stateful(SKind::Irr, 0, false)),
                    "DERIVN" => Some(stateful(SKind::Derivn, 0, false)),
                    "DEPRECIATE STRAIGHTLINE" => Some(stateful(SKind::Depreciate, 0, false)),
                    _ => None,
                };
                if let Some((kind, width, has_init)) = sk {
                    let slot = self.n_states;
                    self.n_states += width;
                    let hist = self.n_hist;
                    if kind == SKind::DelayFixed || kind == SKind::Depreciate || kind == SKind::Derivn || kind == SKind::Queue || kind == SKind::Irr || kind == SKind::Loan || kind == SKind::Tax || kind == SKind::History {
                        self.n_hist += 1;
                    }
                    self.snodes.push(SNode {
                        kind,
                        args: c,
                        has_init,
                        slot,
                        width,
                        hist,
                    });
                    return Ok(C::State(self.snodes.len() - 1));
                }
                if f == "ACTIVE INITIAL" && c.len() == 2 {
                    let init = c.pop().unwrap();
                    let run = c.pop().unwrap();
                    return Ok(C::ActiveInit(Box::new(run), Box::new(init)));
                }
                if f == "IF THEN ELSE" {
                    let e = c.pop().unwrap();
                    let t = c.pop().unwrap();
                    let cnd = c.pop().unwrap();
                    return Ok(C::If(Box::new(cnd), Box::new(t), Box::new(e)));
                }
                if let Some((code, np)) = crate::stats::random_of(f) {
                    // the call's own stream: derived from the model seed, the variable and the call, so that adding a
                    // variable never changes the draws of another
                    let key = crate::stats::stream_seed(self.model.settings.seed, &self.cur, self.calls) & ((1u64 << 52) - 1);
                    self.calls += 1;
                    c.push(C::Num(key as f64));
                    return Ok(C::Pure(Pure::Rnd(code, np), c));
                }
                // RANDOM UNIFORM, NORMAL and EXPONENTIAL: the seed is optional, as in every other random function; left out, the
                // call's own stream is used (the model seed, the variable and the call), so a model that gives it keeps its numbers
                if matches!((f.as_str(), c.len()), ("RANDOM UNIFORM", 2) | ("RANDOM NORMAL", 4) | ("RANDOM EXPONENTIAL", 4)) {
                    let key = crate::stats::stream_seed(self.model.settings.seed, &self.cur, self.calls) & ((1u64 << 52) - 1);
                    self.calls += 1;
                    c.push(C::Num(key as f64));
                }
                let p = match f.as_str() {
                    "NORMAL CDF" => Pure::Dist(0),
                    "NORMAL INV" => Pure::Dist(1),
                    "LOGNORMAL CDF" => Pure::Dist(2),
                    "LOGNORMAL INV" => Pure::Dist(3),
                    "MIN" => Pure::Min,
                    "MAX" => Pure::Max,
                    "ABS" => Pure::Abs,
                    "SQRT" => Pure::Sqrt,
                    "EXP" => Pure::Exp,
                    "LN" => Pure::Ln,
                    "LOG" => Pure::Log,
                    "POWER" => Pure::Pow,
                    "INTEGER" => Pure::Int,
                    "QUANTUM" => Pure::Quantum,
                    "GAMMA LN" => Pure::GammaLn,
                    other if crate::finance::code_of(other).is_some() => Pure::Fin(crate::finance::code_of(other).unwrap()),
                    "DAYS IN STEP" | "HOURS IN STEP" | "YEAR FRACTION" if self.calendar.is_none() => {
                        return Err(format!("{f} needs a start date: set start_date in the model settings"));
                    }
                    "ESCALATE" => Pure::Seas(6),
                    "AND" => Pure::Logic(0),
                    "OR" => Pure::Logic(1),
                    "NOT" => Pure::Logic(2),
                    "BETWEEN" => Pure::Logic(3),
                    "PROFILE" => Pure::Seas(0),
                    "PROFILE STEP" => Pure::Seas(1),
                    "SEASON" => Pure::Seas(2),
                    "AT TIME" => Pure::Seas(4),
                    "EVERY" => Pure::Seas(5),
                    "DATE" | "YEAR" | "MONTH" | "DAY OF MONTH" | "DAY OF YEAR" | "HOUR OF DAY" | "WEEK NUMBER" | "DAY OF WEEK" | "IS WEEKEND" | "IS WORKDAY" | "WORKDAYS BETWEEN" | "DAYLIGHT" if self.calendar.is_none() => {
                        return Err(format!("{f} needs a start date: set start_date in the model settings"));
                    }
                    "DATE" | "YEAR" | "MONTH" | "DAY OF MONTH" | "DAY OF YEAR" | "HOUR OF DAY" | "WEEK NUMBER" | "DAY OF WEEK" | "IS WEEKEND" | "IS WORKDAY" | "WORKDAYS BETWEEN" | "DAYLIGHT"
                        if !matches!(self.calendar.as_ref().unwrap().unit.as_str(), "hour" | "day" | "week" | "month" | "year") => {
                        return Err(format!("{f} needs a time unit of Hour, Day, Week, Month or Year, not {}", self.model.settings.time_units));
                    }
                    "DATE" => Pure::Cal(3),
                    "YEAR" => Pure::Cal(4),
                    "MONTH" => Pure::Cal(5),
                    "DAY OF MONTH" => Pure::Cal(6),
                    "DAY OF YEAR" => Pure::Cal(7),
                    "HOUR OF DAY" => Pure::Cal(8),
                    "WEEK NUMBER" => Pure::Cal(9),
                    "DAY OF WEEK" => Pure::Cal(10),
                    "IS WEEKEND" => Pure::Cal(11),
                    "IS WORKDAY" => Pure::Cal(12),
                    "WORKDAYS BETWEEN" => Pure::Cal(13),
                    "DAYLIGHT" => Pure::Seas(3),
                    "DAYS IN STEP" => Pure::Cal(0),
                    "HOURS IN STEP" => Pure::Cal(1),
                    "YEAR FRACTION" => Pure::Cal(2),
                    "EXCELDATESERIAL" => Pure::DateFn(0),
                    "EXCELDATESERIAL YEAR" => Pure::DateFn(1),
                    "EXCELDATESERIAL MONTH OF YEAR" => Pure::DateFn(2),
                    "EXCELDATESERIAL DAY OF MONTH" => Pure::DateFn(3),
                    "EXCELDATESERIAL DAY OF YEAR" => Pure::DateFn(4),
                    // array functions written out element by element when a model is read (see mdl.rs)
                    "ALLOCATE AVAILABLE ITEM" => Pure::VecFn(1),
                    "ALLOCATE BY PRIORITY ITEM" => Pure::VecFn(2),
                    "VECTOR SORT ORDER ITEM" => Pure::VecFn(3),
                    "VECTOR RANK ITEM" => Pure::VecFn(4),
                    "VECTOR REORDER ITEM" => Pure::VecFn(5),
                    "VECTOR ELM MAP ITEM" => Pure::VecFn(6),
                    "VECTOR SELECT ITEMS" => Pure::VecFn(7),
                    "VECTOR LOOKUP ITEMS" => Pure::VecFn(9),
                    "INVERT MATRIX ITEM" => Pure::VecFn(8),
                    "RANDOM EXPONENTIAL" => Pure::RandExp,
                    "CEILING" => Pure::Ceil,
                    "SINWAVE" => Pure::SinWave,
                    "COSWAVE" => Pure::CosWave,
                    "GRAPH" => Pure::Graph,
                    "GRAPHSTEP" => Pure::GraphStep,
                    "SINH" => Pure::Sinh,
                    "COSH" => Pure::Cosh,
                    "TANH" => Pure::Tanh,
                    "ARCSIN" => Pure::Asin,
                    "ARCCOS" => Pure::Acos,
                    "ARCTAN2" => Pure::Atan2,
                    "FLOOR" => Pure::Floor,
                    "SIGN" => Pure::Sign,
                    "LOG10" => Pure::Log10,
                    "LOG2" => Pure::Log2,
                    "CBRT" => Pure::Cbrt,
                    "HYPOT" => Pure::Hypot,
                    "RADIANS" => Pure::Rad,
                    "DEGREES" => Pure::Deg,
                    "CLAMP" => Pure::Clamp,
                    "MEAN" => Pure::Mean,
                    "MEDIAN" => Pure::Median,
                    "PI" => Pure::Pi,
                    "ERF" => Pure::Erf,
                    "GAMMA" => Pure::Gamma,
                    "FACTORIAL" => Pure::Fact,
                    "ROUND" => Pure::Round,
                    "MODULO" => Pure::Mod,
                    "SIN" => Pure::Sin,
                    "COS" => Pure::Cos,
                    "TAN" => Pure::Tan,
                    "ARCTAN" => Pure::Atan,
                    "XIDZ" => Pure::Xidz,
                    "ZIDZ" => Pure::Zidz,
                    "STEP" => Pure::Step,
                    "PULSE" => Pure::Pulse,
                    "PULSE TRAIN" => Pure::PulseTrain,
                    "RAMP" => Pure::Ramp,
                    "RANDOM UNIFORM" => Pure::RandU,
                    "RANDOM NORMAL" => Pure::RandN,
                    "TOTAL" | "AVERAGE" | "MAX OF" | "MIN OF" | "COUNT OF" => {
                        return Err(format!("{f} adds up the items of a variable that is per a set; here nothing is"));
                    }
                    other => return Err(format!("Function {other} isn't supported")),
                };
                C::Pure(p, c)
            }
        })
    }
}

/// Variables needed to compute `c` during the run (stocks and stateful outputs excluded).
fn run_deps(c: &C, stock: &[Option<usize>], snodes: &[SNode], out: &mut Vec<usize>) {
    match c {
        C::Var(i) => {
            if stock[*i].is_none() && !out.contains(i) {
                out.push(*i)
            }
        }
        C::Neg(a) | C::Not(a) | C::Table(_, a) => run_deps(a, stock, snodes, out),
        C::TableMode(_, _, a) | C::QueueInfo(_, _, a) => a.iter().for_each(|a| run_deps(a, stock, snodes, out)),
        C::Bin(_, a, b) => {
            run_deps(a, stock, snodes, out);
            run_deps(b, stock, snodes, out);
        }
        C::If(a, b, d) => {
            run_deps(a, stock, snodes, out);
            run_deps(b, stock, snodes, out);
            run_deps(d, stock, snodes, out);
        }
        C::Pure(_, args) => args.iter().for_each(|a| run_deps(a, stock, snodes, out)),
        C::ActiveInit(a, _) => run_deps(a, stock, snodes, out),
        C::State(n) if snodes[*n].kind == SKind::SampleIfTrue => {
            run_deps(&snodes[*n].args[0], stock, snodes, out);
            run_deps(&snodes[*n].args[1], stock, snodes, out);
        }
        // outputs that read their arguments at the same step
        C::State(n) if matches!(snodes[*n].kind, SKind::Npv | SKind::Npve) => {
            run_deps(&snodes[*n].args[0], stock, snodes, out);
            if let Some(f) = snodes[*n].args.get(3) {
                run_deps(f, stock, snodes, out);
            }
        }
        C::State(n) if snodes[*n].kind == SKind::Depreciate => {
            run_deps(&snodes[*n].args[0], stock, snodes, out);
            run_deps(&snodes[*n].args[1], stock, snodes, out);
        }
        C::State(n) if matches!(snodes[*n].kind, SKind::Derivn | SKind::Irr | SKind::NpvAt | SKind::Levelised | SKind::Payback | SKind::Loan | SKind::Tax | SKind::Previous | SKind::Carry | SKind::History) => {
            snodes[*n].args.iter().for_each(|a| run_deps(a, stock, snodes, out));
        }
        _ => {}
    }
}

/// Variables needed to compute `c` at the initial time (including stateful initial values).
fn init_deps(c: &C, snodes: &[SNode], out: &mut Vec<usize>) {
    match c {
        C::Var(i) => {
            if !out.contains(i) {
                out.push(*i)
            }
        }
        C::Neg(a) | C::Not(a) | C::Table(_, a) => init_deps(a, snodes, out),
        C::TableMode(_, _, a) | C::QueueInfo(_, _, a) => a.iter().for_each(|a| init_deps(a, snodes, out)),
        // a queue starts from its initial content only: its in- and outflows may depend on the queue itself
        C::State(n) if snodes[*n].kind == SKind::Queue => {
            if let Some(i) = snodes[*n].args.get(2) {
                init_deps(i, snodes, out);
            }
        }
        C::Bin(_, a, b) => {
            init_deps(a, snodes, out);
            init_deps(b, snodes, out);
        }
        C::If(a, b, d) => {
            init_deps(a, snodes, out);
            init_deps(b, snodes, out);
            init_deps(d, snodes, out);
        }
        C::Pure(_, args) => args.iter().for_each(|a| init_deps(a, snodes, out)),
        C::ActiveInit(_, b) => init_deps(b, snodes, out),
        C::State(n) if matches!(snodes[*n].kind, SKind::Npv | SKind::Npve | SKind::Depreciate | SKind::Derivn | SKind::Irr | SKind::NpvAt | SKind::Levelised | SKind::Payback | SKind::Loan | SKind::Tax | SKind::Previous | SKind::Carry | SKind::History) => {
            snodes[*n].args.iter().for_each(|a| init_deps(a, snodes, out));
        }
        C::State(n) => {
            let s = &snodes[*n];
            if s.has_init {
                init_deps(&s.args[s.args.len() - 1], snodes, out);
            } else {
                init_deps(&s.args[0], snodes, out);
            }
            if s.args.len() > 1 && matches!(s.kind, SKind::Delay1 | SKind::Delay3) {
                init_deps(&s.args[1], snodes, out);
            }
            // TREND starts from its input and its averaging time as well as its initial trend
            if s.kind == SKind::Trend {
                s.args.iter().for_each(|a| init_deps(a, snodes, out));
            }
        }
        _ => {}
    }
}

/// Order nodes so each comes after what it needs. Returns the names caught in a loop on failure.
fn topo(n: usize, deps: &[Vec<usize>], include: &[bool]) -> Result<Vec<usize>, Vec<usize>> {
    let mut state = vec![0u8; n];
    let mut order = Vec::new();
    fn visit(
        i: usize,
        deps: &[Vec<usize>],
        state: &mut [u8],
        order: &mut Vec<usize>,
        stack: &mut Vec<usize>,
    ) -> Result<(), Vec<usize>> {
        if state[i] == 2 {
            return Ok(());
        }
        if state[i] == 1 {
            let pos = stack.iter().position(|x| *x == i).unwrap_or(0);
            return Err(stack[pos..].to_vec());
        }
        state[i] = 1;
        stack.push(i);
        for &d in &deps[i] {
            visit(d, deps, state, order, stack)?;
        }
        stack.pop();
        state[i] = 2;
        order.push(i);
        Ok(())
    }
    for i in 0..n {
        if include[i] {
            visit(i, deps, &mut state, &mut order, &mut Vec::new())?;
        }
    }
    Ok(order.into_iter().filter(|i| include[*i]).collect())
}

/// With `units_scale_values` on, a constant that is a plain number and is declared in a percent or ppm unit carries the
/// scale of that unit: "5 ~ %/Day" is 0.05 per Day. (Other units do not rescale values: MB stays a number of MB.)
/// Returns the model with those values replaced, or None when nothing changes.
fn scaled_by_units(model: &Model) -> Option<Model> {
    if !model.settings.units_scale_values {
        return None;
    }
    let ctx = crate::units::UnitCtx::of(model);
    let mut out: Option<Model> = None;
    for (i, v) in model.vars.iter().enumerate() {
        if v.kind != Kind::Constant || v.units.trim().is_empty() || (!v.units.contains('%') && !v.units.to_lowercase().contains("ppm") && !v.units.to_lowercase().contains("percent")) {
            continue;
        }
        let val = match expr::parse(&v.equation) {
            Ok(expr::Expr::Num(n)) => n,
            Ok(expr::Expr::Neg(a)) => match *a {
                expr::Expr::Num(n) => -n,
                _ => continue,
            },
            _ => continue,
        };
        let Ok(u) = crate::units::parse_unit_in(&v.units, &ctx) else { continue };
        if u.pure_scale == 1.0 {
            continue;
        }
        let m = out.get_or_insert_with(|| model.clone());
        m.vars[i].equation = format!("{}", val * u.pure_scale);
    }
    out
}

pub fn compile(model: &Model) -> Result<Compiled, Vec<Problem>> {
    match scaled_by_units(model) {
        Some(m) => compile_impl(&m),
        None => compile_impl(model),
    }
}

fn compile_impl(model: &Model) -> Result<Compiled, Vec<Problem>> {
    crate::userfn::set(&model.functions);
    let bad = crate::userfn::check(&model.functions);
    if !bad.is_empty() {
        return Err(bad.into_iter().map(|(f, message)| Problem { var: Some(f), message }).collect());
    }
    // module instances become ordinary variables first
    let instanced;
    let model = match crate::modules::flatten(model).map_err(|p| p.into_iter().map(|(var, message)| Problem { var, message }).collect::<Vec<_>>())? {
        Some(f) => {
            instanced = f;
            &instanced
        }
        None => model,
    };
    if !crate::sets::uses_sets(model) {
        return compile_flat(model);
    }
    let ex = crate::sets::expand(model).map_err(|p| p.into_iter().map(|(var, message)| Problem { var, message }).collect::<Vec<_>>())?;
    let mut c = compile_flat(&ex.model)?;
    c.rollups = ex.rollups;
    Ok(c)
}

fn compile_flat(model: &Model) -> Result<Compiled, Vec<Problem>> {
    let mut problems = Vec::new();
    let mut index = HashMap::new();
    for (i, v) in model.vars.iter().enumerate() {
        let n = expr::normalize(&v.name);
        if n.is_empty() {
            problems.push(Problem {
                var: Some(v.name.clone()),
                message: "A variable has no name".into(),
            });
        } else if expr::TIME_VARS.iter().any(|(t, _)| *t == n) {
            problems.push(Problem {
                var: Some(v.name.clone()),
                message: format!("'{}' is a reserved name", v.name),
            });
        } else if index.insert(n, i).is_some() {
            problems.push(Problem {
                var: Some(v.name.clone()),
                message: format!("Two variables are called '{}'", v.name),
            });
        }
    }
    let mut b = Builder {
        model,
        cur: String::new(),
        calls: 0,
        calendar: crate::calendar::CalCtx::of(model),
        index,
        tables: Vec::new(),
        table_of: HashMap::new(),
        snodes: Vec::new(),
        n_states: 0,
        n_hist: 0,
    };
    let n = model.vars.len();
    let mut stock_slot = vec![None; n];
    let mut slot = 0;
    for (i, v) in model.vars.iter().enumerate() {
        if v.kind == Kind::Stock {
            stock_slot[i] = Some(slot);
            slot += 1;
        }
    }
    let n_stocks = slot;
    b.n_states = n_stocks;
    let mut exprs = vec![None; n];
    let mut has_series = vec![true; n];
    for (i, v) in model.vars.iter().enumerate() {
        let lookup_fn = v.kind == Kind::Lookup && v.equation.trim().is_empty();
        if lookup_fn {
            has_series[i] = false;
            if v.table.is_empty() {
                problems.push(Problem {
                    var: Some(v.name.clone()),
                    message: "Lookup table has no points".into(),
                });
            }
            continue;
        }
        if v.equation.trim().is_empty() {
            let what = if v.kind == Kind::Stock {
                "an initial value"
            } else {
                "a formula"
            };
            problems.push(Problem {
                var: Some(v.name.clone()),
                message: format!("Needs {what}"),
            });
            continue;
        }
        b.cur = v.name.clone();
        b.calls = 0;
        match expr::parse(&v.equation).and_then(|e| b.compile(&e)) {
            Ok(mut c) => {
                if v.kind == Kind::Lookup {
                    let t = b.table_for(i);
                    c = C::Table(t, Box::new(c));
                }
                exprs[i] = Some(c);
            }
            Err(e) => problems.push(Problem {
                var: Some(v.name.clone()),
                message: e,
            }),
        }
    }
    let id_index: HashMap<u64, usize> = model
        .vars
        .iter()
        .enumerate()
        .map(|(i, v)| (v.id, i))
        .collect();
    let mut stock_flows = Vec::new();
    for (i, v) in model.vars.iter().enumerate() {
        if v.kind == Kind::Stock {
            let ins = model
                .vars
                .iter()
                .enumerate()
                .filter(|(_, f)| f.kind == Kind::Flow && f.to == Some(v.id))
                .map(|(j, _)| j)
                .collect();
            let outs = model
                .vars
                .iter()
                .enumerate()
                .filter(|(_, f)| f.kind == Kind::Flow && f.from == Some(v.id))
                .map(|(j, _)| j)
                .collect();
            stock_flows.push((i, ins, outs));
        }
    }
    let _ = id_index;
    if !problems.is_empty() {
        return Err(problems);
    }
    // run order: all non-stock variables
    let mut deps_run = vec![Vec::new(); n];
    let mut deps_init = vec![Vec::new(); n];
    for i in 0..n {
        if let Some(c) = &exprs[i] {
            if stock_slot[i].is_none() {
                run_deps(c, &stock_slot, &b.snodes, &mut deps_run[i]);
            }
            init_deps(c, &b.snodes, &mut deps_init[i]);
        }
    }
    // stateful inputs must be computable too: they're evaluated after all variables, so no ordering needed.
    let include_run: Vec<bool> = (0..n)
        .map(|i| exprs[i].is_some() && stock_slot[i].is_none())
        .collect();
    let include_init: Vec<bool> = (0..n).map(|i| exprs[i].is_some()).collect();
    let name_list = |cyc: Vec<usize>| {
        cyc.iter()
            .map(|i| model.vars[*i].name.clone())
            .collect::<Vec<_>>()
            .join(" → ")
    };
    let order_run = topo(n, &deps_run, &include_run).map_err(|c| {
        vec![Problem {
            var: Some(model.vars[c[0]].name.clone()),
            message: format!("Circular logic with no stock in between: {}", name_list(c)),
        }]
    })?;
    let order_init = topo(n, &deps_init, &include_init).map_err(|c| {
        vec![Problem {
            var: Some(model.vars[c[0]].name.clone()),
            message: format!("Initial values depend on each other: {}", name_list(c)),
        }]
    })?;
    Ok(Compiled {
        names: model.vars.iter().map(|v| v.name.clone()).collect(),
        kinds: model.vars.iter().map(|v| v.kind).collect(),
        exprs,
        stock_slot,
        stock_flows,
        non_negative: model.vars.iter().map(|v| v.non_negative).collect(),
        order_run,
        order_init,
        n_states: b.n_states,
        n_hist: b.n_hist,
        snodes: b.snodes,
        tables: b.tables,
        has_series,
        rollups: Vec::new(),
        calendar: b.calendar.clone(),
        flow_per: flow_pers(model),
    })
}

/// Calendar convention in a monthly or yearly model: flows declared per Day or per Hour.
fn flow_pers(model: &Model) -> Vec<Option<FlowPer>> {
    let cal = model.settings.time_convention == crate::model::TimeConvention::Calendar && matches!(crate::units::time_base(model).as_str(), "month" | "year");
    model
        .vars
        .iter()
        .map(|v| {
            if !cal || v.kind != Kind::Flow {
                return None;
            }
            let (_, den) = crate::doctor::units_rules::split_text(&v.units);
            match den.map(|d| d.trim().to_lowercase()).as_deref() {
                Some("day") | Some("days") => Some(FlowPer::Day),
                Some("hour") | Some("hours") | Some("hr") => Some(FlowPer::Hour),
                _ => None,
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Run {
    pub name: String,
    pub times: Vec<f64>,
    /// normalised variable name → values at each time
    pub series: HashMap<String, Vec<f64>>,
    pub time_units: String,
    /// Every step where a non-negative stock could not pay what its outflows asked for (see `Constraint`).
    pub constraints: Vec<Constraint>,
}

/// One activation of the outflow constraint of a non-negative stock: at `time` the outflows of `stock` asked for
/// `requested` per time unit, but only `allowed` was available (the stock's content spread over one step plus its inflows),
/// so every outflow was scaled by allowed / requested. Material is conserved: the stock never goes below zero by creating it.
#[derive(Clone, Debug, PartialEq)]
pub struct Constraint {
    pub time: f64,
    pub stock: String,
    pub requested: f64,
    pub allowed: f64,
}

impl Default for Run {
    fn default() -> Self {
        Run { name: String::new(), times: Vec::new(), series: HashMap::new(), time_units: String::new(), constraints: Vec::new() }
    }
}

impl Run {
    pub fn get(&self, name: &str) -> Option<&Vec<f64>> {
        self.series.get(&expr::normalize(name))
    }
}

struct Eval<'a> {
    c: &'a Compiled,
    vals: Vec<f64>,
    y: Vec<f64>,
    dy: Vec<f64>,
    t: f64,
    t0: f64,
    t1: f64,
    dt: f64,
    init: bool,
    hist: Vec<Vec<(f64, f64)>>,
    rng: u64,
    /// One random stream per call site of RANDOM UNIFORM / RANDOM NORMAL, started from its seed argument:
    /// (site, state). Sites are numbered in the order they are first used, so a run is repeatable.
    streams: Vec<(usize, u64)>,
    /// outflow-constraint log (recorded on the main pass of each step)
    constraints: Vec<Constraint>,
    /// per stock (index in `stock_flows`): was its constraint active at the previous recorded step?
    constraint_active: Vec<bool>,
}

fn rand01(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

impl Eval<'_> {
    fn ev(&mut self, c: &C) -> f64 {
        match c {
            C::Num(n) => *n,
            C::Var(i) => match self.c.stock_slot[*i] {
                Some(s) => self.y[s],
                None => self.vals[*i],
            },
            C::Time => self.t,
            C::T0 => self.t0,
            C::T1 => self.t1,
            C::Dt => self.dt,
            C::Neg(a) => -self.ev(a),
            C::Not(a) => b2f(self.ev(a) == 0.0),
            C::Bin(op, a, b) => {
                let x = self.ev(a);
                let y = self.ev(b);
                match op {
                    BinOp::Add => x + y,
                    BinOp::Sub => x - y,
                    BinOp::Mul => x * y,
                    BinOp::Div => x / y,
                    BinOp::Pow => x.powf(y),
                    BinOp::Lt => b2f(x < y),
                    BinOp::Le => b2f(x <= y),
                    BinOp::Gt => b2f(x > y),
                    BinOp::Ge => b2f(x >= y),
                    BinOp::Eq => b2f(x == y),
                    BinOp::Ne => b2f(x != y),
                    BinOp::And => b2f(x != 0.0 && y != 0.0),
                    BinOp::Or => b2f(x != 0.0 || y != 0.0),
                }
            }
            C::If(cnd, a, b) => {
                if self.ev(cnd) != 0.0 {
                    self.ev(a)
                } else {
                    self.ev(b)
                }
            }
            C::ActiveInit(a, b) => {
                if self.init {
                    self.ev(b)
                } else {
                    self.ev(a)
                }
            }
            C::QueueInfo(var, mode, a) => {
                let v: Vec<f64> = a.iter().map(|x| self.ev(x)).collect();
                self.queue_info(*var, *mode, &v)
            }
            C::TableMode(t, mode, a) => {
                let v: Vec<f64> = a.iter().map(|x| self.ev(x)).collect();
                table_mode(&self.c.tables[*t], *mode, &v)
            }
            C::Table(t, a) => {
                let x = self.ev(a);
                expr::lookup(&self.c.tables[*t], x)
            }
            C::Pure(p, args) => {
                let v: Vec<f64> = args.iter().map(|a| self.ev(a)).collect();
                let t = self.t;
                let half = self.dt / 2.0;
                match p {
                    Pure::Min => v.iter().copied().fold(f64::INFINITY, f64::min),
                    Pure::Max => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    Pure::Abs => v[0].abs(),
                    Pure::Sqrt => v[0].sqrt(),
                    Pure::Exp => v[0].exp(),
                    Pure::Ln => v[0].ln(),
                    Pure::Log => v[0].ln() / v[1].ln(),
                    Pure::Pow => v[0].powf(v[1]),
                    Pure::Int => v[0].trunc(),
                    Pure::GammaLn => ln_gamma(v[0]),
                    Pure::VecFn(k) => vec_fn(*k, &v),
                    Pure::DateFn(k) => date_fn(*k, &v),
                    Pure::Fin(k) => crate::finance::eval(*k, &v),
                    Pure::Dist(k) => match k {
                        0 => crate::stats::normal_cdf(v[0], v[1], v[2]),
                        1 => crate::stats::normal_inv(v[0], v[1], v[2]),
                        2 => crate::stats::lognormal_cdf(v[0], v[1], v[2]),
                        _ => crate::stats::lognormal_inv(v[0], v[1], v[2]),
                    },
                    Pure::Rnd(code, np) => {
                        let key = *v.last().unwrap();
                        let user = if v.len() == np + 2 { Some(v[*np]) } else { None };
                        let mut rng = crate::stats::Rng::new(self.stream_state(key, user));
                        let x = crate::stats::draw(*code, &v[..*np], &mut rng);
                        self.set_stream_state(key, rng.state);
                        x
                    }
                    Pure::Logic(k) => match k {
                        0 => b2f(v.iter().all(|x| *x != 0.0)),
                        1 => b2f(v.iter().any(|x| *x != 0.0)),
                        2 => b2f(v[0] == 0.0),
                        _ => b2f(v[1] <= v[0] && v[0] <= v[2]),
                    },
                    Pure::Seas(k) => match k {
                        0 | 1 => {
                            let n = v.len() - 1;
                            let phase = (t - self.t0).rem_euclid(v[0]);
                            let slot = v[0] / n as f64;
                            if *k == 1 {
                                v[1 + ((phase / slot).floor() as usize).min(n - 1)]
                            } else {
                                let x = phase / slot - 0.5;
                                let i = x.floor();
                                let (a, b) = (((i as i64).rem_euclid(n as i64)) as usize, (((i as i64) + 1).rem_euclid(n as i64)) as usize);
                                v[1 + a] + (v[1 + b] - v[1 + a]) * (x - i)
                            }
                        }
                        2 => v[0] + (v[1] - v[0]) * (1.0 + (std::f64::consts::TAU * (t - v[2]) / v[3]).cos()) / 2.0,
                        3 => {
                            let cal = self.c.calendar.as_ref().unwrap();
                            let h = cal.stamp_at(t, self.t0).secs / 3600.0;
                            if h >= v[0] && h <= v[1] { (std::f64::consts::PI * (h - v[0]) / (v[1] - v[0])).sin() } else { 0.0 }
                        }
                        4 => b2f((t - v[0]).abs() < self.dt / 2.0),
                        6 => v[0] * (1.0 + v[1]).powf((t - v[2]) / v[3]),
                        _ => {
                            let start = v.get(1).copied().unwrap_or(self.t0);
                            let d = t - start;
                            b2f(d >= -self.dt / 2.0 && (d - (d / v[0]).round() * v[0]).abs() < self.dt / 2.0)
                        }
                    },
                    Pure::Cal(k) => {
                        let cal = self.c.calendar.as_ref().unwrap();
                        let now = cal.stamp_at(t, self.t0);
                        let civil_now = crate::calendar::civil_from_days(now.days);
                        match k {
                            0 => cal.days_in_step(t, self.t0),
                            1 => 24.0 * cal.days_in_step(t, self.t0),
                            3 => crate::calendar::excel_serial(civil_now.0, civil_now.1, civil_now.2).map(|x| x as f64 + now.secs / 86_400.0).unwrap_or(f64::NAN),
                            4 => civil_now.0 as f64,
                            5 => civil_now.1 as f64,
                            6 => civil_now.2 as f64,
                            7 => crate::calendar::day_of_year(civil_now.0, civil_now.1, civil_now.2) as f64,
                            8 => (now.secs / 3600.0).floor(),
                            9 => crate::calendar::iso_week(civil_now.0, civil_now.1, civil_now.2) as f64,
                            10 => crate::calendar::weekday(now.days) as f64,
                            11 => b2f(cal.is_weekend(now.days)),
                            12 => b2f(cal.is_workday(now.days)),
                            13 => cal.workdays_between(cal.stamp_at(v[0], self.t0).days, cal.stamp_at(v[1], self.t0).days) as f64,
                            _ => {
                                let civil = |x: f64| {
                                    let st = cal.stamp_at(x, self.t0);
                                    crate::calendar::civil_from_days(st.days)
                                };
                                match crate::calendar::Basis::from_number(v.get(2).copied().unwrap_or(0.0)) {
                                    Some(b) => crate::calendar::year_fraction(civil(v[0]), civil(v[1]), b),
                                    None => f64::NAN,
                                }
                            }
                        }
                    }
                    Pure::RandExp => {
                        // RANDOM EXPONENTIAL(min, max, shift, stretch, seed)
                        let u = self.stream(args.as_ptr() as usize, v[4]).max(1e-300);
                        (v[2] + v[3] * -u.ln()).clamp(v[0], v[1])
                    }
                    Pure::Quantum => if v[1] <= 0.0 { v[0] } else { v[1] * (v[0] / v[1]).trunc() },
                    Pure::Ceil => v[0].ceil(),
                    Pure::SinWave | Pure::CosWave => {
                        let start = v.get(2).copied().unwrap_or(self.t0);
                        let a = (t - start) * std::f64::consts::TAU / v[1];
                        v[0] * if *p == Pure::SinWave { a.sin() } else { a.cos() }
                    }
                    Pure::Graph | Pure::GraphStep => {
                        let ys = &v[3..];
                        let pos = (v[0] - v[1]) / v[2];
                        if !(pos > 0.0) {
                            ys[0]
                        } else if pos >= (ys.len() - 1) as f64 {
                            ys[ys.len() - 1]
                        } else if *p == Pure::GraphStep {
                            ys[pos.floor() as usize]
                        } else {
                            let i = pos.floor() as usize;
                            ys[i] + (ys[i + 1] - ys[i]) * (pos - i as f64)
                        }
                    }
                    Pure::Sinh => v[0].sinh(),
                    Pure::Cosh => v[0].cosh(),
                    Pure::Tanh => v[0].tanh(),
                    Pure::Asin => v[0].asin(),
                    Pure::Acos => v[0].acos(),
                    Pure::Atan2 => v[0].atan2(v[1]),
                    Pure::Floor => v[0].floor(),
                    Pure::Sign => if v[0] == 0.0 { 0.0 } else { v[0].signum() },
                    Pure::Log10 => v[0].log10(),
                    Pure::Log2 => v[0].log2(),
                    Pure::Cbrt => v[0].cbrt(),
                    Pure::Hypot => v[0].hypot(v[1]),
                    Pure::Rad => v[0].to_radians(),
                    Pure::Deg => v[0].to_degrees(),
                    Pure::Clamp => v[0].max(v[1].min(v[2])).min(v[1].max(v[2])),
                    Pure::Mean => v.iter().sum::<f64>() / v.len() as f64,
                    Pure::Median => median(&v),
                    Pure::Pi => std::f64::consts::PI,
                    Pure::Erf => erf(v[0]),
                    Pure::Gamma => gamma(v[0]),
                    Pure::Fact => gamma(v[0].round() + 1.0).round(),
                    Pure::Round => v[0].round(),
                    Pure::Mod => v[0].rem_euclid(v[1]),
                    Pure::Sin => v[0].sin(),
                    Pure::Cos => v[0].cos(),
                    Pure::Tan => v[0].tan(),
                    Pure::Atan => v[0].atan(),
                    // Vensim: the divisor counts as zero when it is smaller than 1e-6 in magnitude
                    Pure::Xidz => {
                        if v[1].abs() < 1e-6 {
                            v[2]
                        } else {
                            v[0] / v[1]
                        }
                    }
                    Pure::Zidz => {
                        if v[1].abs() < 1e-6 {
                            0.0
                        } else {
                            v[0] / v[1]
                        }
                    }
                    Pure::Step => {
                        if t + half >= v[1] {
                            v[0]
                        } else {
                            0.0
                        }
                    }
                    Pure::Pulse => b2f(t + half >= v[0] && t + half < v[0] + v[1].max(self.dt)),
                    Pure::PulseTrain => {
                        if t + half < v[0] || t > v[3] || v[2] <= 0.0 {
                            0.0
                        } else {
                            b2f(((t + half - v[0]) % v[2]) < v[1].max(self.dt))
                        }
                    }
                    Pure::Ramp => {
                        if t <= v[1] {
                            0.0
                        } else {
                            v[0] * (t.min(v[2]) - v[1])
                        }
                    }
                    Pure::RandU => v[0] + (v[1] - v[0]) * self.stream(args.as_ptr() as usize, v[2]),
                    Pure::RandN => {
                        let site = args.as_ptr() as usize;
                        let u1 = self.stream(site, v[4]).max(1e-12);
                        let u2 = self.stream(site, v[4]);
                        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
                        (v[2] + v[3] * z).clamp(v[0], v[1])
                    }
                }
            }
            C::State(n) => self.state(*n),
        }
    }

    /// The state of the stream of a random call, made on first use: from the call's own key, or from the seed it was given.
    fn stream_state(&mut self, key: f64, user: Option<f64>) -> u64 {
        let site = key as usize;
        if let Some((_, st)) = self.streams.iter().find(|(s, _)| *s == site) {
            return *st;
        }
        let seed = match user {
            Some(u) => crate::stats::Rng::new(u.to_bits()).next_u64(),
            None => crate::stats::Rng::new(key as u64).next_u64(),
        };
        self.streams.push((site, seed));
        seed
    }
    fn set_stream_state(&mut self, key: f64, state: u64) {
        let site = key as usize;
        if let Some(e) = self.streams.iter_mut().find(|(s, _)| *s == site) {
            e.1 = state;
        }
    }

    /// The next number in [0, 1) of the stream belonging to this call site. The stream starts from
    /// the seed argument, so two runs with the same seed agree and different seeds give independent
    /// draws; two call sites with the same seed still differ (they are numbered by first use).
    fn stream(&mut self, site: usize, seed: f64) -> f64 {
        let i = match self.streams.iter().position(|(s, _)| *s == site) {
            Some(i) => i,
            None => {
                let n = self.streams.len() as u64 + 1;
                // splitmix64 over the seed bits and the site number
                let mut z = seed.to_bits() ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^= z >> 31;
                self.streams.push((site, if z == 0 { 0x9E37_79B9_7F4A_7C15 } else { z }));
                self.streams.len() - 1
            }
        };
        rand01(&mut self.streams[i].1)
    }

    /// Ages and amounts of what is waiting in the queue held by variable `var` (see `SKind::Queue`).
    fn queue_info(&mut self, var: usize, mode: u8, v: &[f64]) -> f64 {
        let c = self.c;
        let Some(C::State(n)) = c.exprs[var].as_ref() else { return NA };
        let s = &c.snodes[*n];
        if s.kind != SKind::Queue {
            return NA;
        }
        let k = s.slot;
        let (cum_in, cum_out, init) = (self.y[k], self.y[k + 1], self.y[k + 2]);
        // the groups that arrived together, oldest first, as (age, amount): the initial content (one step old
        // at the start) and what came in during each step so far
        let mut groups: Vec<(f64, f64)> = Vec::new();
        if init > 0.0 {
            groups.push((self.t - (self.t0 - self.dt), init));
        }
        let h = &self.hist[s.hist];
        for (j, (t, f)) in h.iter().enumerate() {
            let next = h.get(j + 1).map(|(_, g)| *g).unwrap_or(cum_in);
            if next > *f {
                groups.push((self.t - *t, next - *f));
            }
        }
        // the oldest groups have left already
        let mut gone = cum_out;
        let mut waiting: Vec<(f64, f64)> = Vec::new();
        for (age, amount) in groups {
            if gone >= amount {
                gone -= amount;
            } else {
                waiting.push((age, amount - gone));
                gone = 0.0;
            }
        }
        match mode {
            2 => waiting.first().map(|(a, _)| *a).unwrap_or(0.0),
            1 => {
                // the average age of the oldest `quant` (everything when quant is 0 or more than there is)
                let total: f64 = waiting.iter().map(|(_, m)| m).sum();
                if total <= 0.0 {
                    return 0.0;
                }
                let quant = v.first().copied().filter(|q| *q > 0.0 && *q < total).unwrap_or(total);
                let (mut left, mut weighted) = (quant, 0.0);
                for (age, amount) in &waiting {
                    let take = left.min(*amount);
                    weighted += age * take;
                    left -= take;
                    if left <= 0.0 {
                        break;
                    }
                }
                weighted / quant
            }
            _ => {
                // the amount at least `min` old and no older than `max` (a missing end is open)
                let (lo, hi) = (v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(f64::INFINITY));
                let (lo, hi) = (if lo == NA { 0.0 } else { lo }, if hi == NA { f64::INFINITY } else { hi });
                waiting.iter().filter(|(a, _)| *a >= lo && *a <= hi).map(|(_, m)| m).sum()
            }
        }
    }

    fn state(&mut self, n: usize) -> f64 {
        let c = self.c;
        let s = &c.snodes[n];
        let k = s.slot;
        if self.init {
            // a stage of a DELAY N / SMOOTH N chain with its own initial value: start the stages before it too
            if s.has_init {
                if let C::State(prev) = &s.args[0] {
                    self.state(*prev);
                }
            }
            let init = if s.has_init {
                self.ev(&s.args[s.args.len() - 1])
            } else {
                self.ev(&s.args[0])
            };
            let dtime = if s.args.len() > 1 {
                self.ev(&s.args[1])
            } else {
                0.0
            };
            match s.kind {
                SKind::Smooth | SKind::Initial | SKind::Trend | SKind::SampleIfTrue => self.y[k] = init,
                SKind::Smooth3 => self.y[k..k + 3].iter_mut().for_each(|x| *x = init),
                SKind::Delay1 => self.y[k] = init * dtime,
                SKind::Delay3 => self.y[k..k + 3]
                    .iter_mut()
                    .for_each(|x| *x = init * dtime / 3.0),
                SKind::DelayFixed => {
                    // the initial value is evaluated once, at the start (Vensim), and held until the delay has passed
                    self.y[k] = init;
                    return init;
                }
                SKind::Npv => {
                    self.y[k] = 1.0;
                    self.y[k + 1] = self.ev(&s.args[2]);
                }
                SKind::Npve => {
                    let r = self.ev(&s.args[1]);
                    self.y[k] = 1.0 / (1.0 + r * self.dt);
                    self.y[k + 1] = self.ev(&s.args[2]);
                }
                SKind::NpvAt => self.y[k] = self.ev(&s.args[2]),
                SKind::Levelised => {
                    self.y[k] = self.ev(&s.args[3]);
                    self.y[k + 1] = 0.0;
                }
                SKind::Payback => {
                    self.y[k] = 0.0;
                    self.y[k + 1] = 0.0;
                    self.y[k + 2] = 0.0;
                }
                SKind::Loan | SKind::Tax | SKind::History => {}
                SKind::Previous => self.y[k] = self.ev(&s.args[1]),
                SKind::Carry => self.y[k] = 0.0,
                SKind::Derivn | SKind::Irr => {}
                SKind::Queue => {
                    self.y[k] = 0.0;
                    self.y[k + 1] = 0.0;
                    self.y[k + 2] = s.args.get(2).map(|a| self.ev(a)).unwrap_or(0.0);
                }
                SKind::Depreciate => {}
            }
            if s.kind == SKind::Trend {
                // stored value is the average input; recover it from the initial trend
                let input = self.ev(&s.args[0]);
                self.y[k] = input / (1.0 + init * dtime);
                return init;
            }
        }
        match s.kind {
            SKind::NpvAt => {
                let df = self.discount(s, 1, 5, 4);
                (self.y[k] + self.ev(&s.args[0]) * self.dt * df) * self.ev(&s.args[3])
            }
            SKind::Levelised => {
                let off = [0.0, 0.5, 1.0][(self.ev(&s.args[4]) as usize).min(2)];
                let df = self.discount_at(s, 2, 5, off);
                let pvc = self.y[k] + self.ev(&s.args[0]) * self.dt * df;
                let pvq = self.y[k + 1] + self.ev(&s.args[1]) * self.dt * df;
                if pvq.abs() < 1e-300 { f64::NAN } else { pvc / pvq }
            }
            SKind::Payback => self.payback_step(s).1.unwrap_or(f64::NAN),
            SKind::Loan => self.loan_value(s),
            SKind::Tax => self.tax_value(s),
            SKind::Previous => self.y[k],
            SKind::Carry => {
                let amount = self.ev(&s.args[0]) * self.dt + self.y[k];
                (amount + 1e-9).floor() / self.dt
            }
            SKind::History => {
                let (target, now) = (self.ev(&s.args[1]), self.ev(&s.args[0]));
                let mut pts: Vec<(f64, f64)> = self.hist[s.hist].clone();
                pts.push((self.t, now));
                if target >= self.t || pts.len() == 1 {
                    now
                } else if target <= pts[0].0 {
                    pts[0].1
                } else {
                    let i = pts.iter().position(|(t, _)| *t >= target).unwrap_or(pts.len() - 1);
                    let (a, b) = (pts[i - 1], pts[i]);
                    if b.0 == a.0 { b.1 } else { a.1 + (b.1 - a.1) * (target - a.0) / (b.0 - a.0) }
                }
            }
            SKind::Queue => self.y[k + 2] + self.y[k] - self.y[k + 1],
            SKind::Irr => {
                // the cash that came in each step so far (and this one), each discounted back to the start
                let period = s.args.get(1).map(|a| self.ev(a)).filter(|p| *p > 0.0).unwrap_or(1.0);
                let init = s.args.get(2).map(|a| self.ev(a)).unwrap_or(0.0);
                let now = self.ev(&s.args[0]) * self.dt;
                let mut flows: Vec<(f64, f64)> = self.hist[s.hist].iter().filter(|(t, _)| *t < self.t - 1e-9).map(|(t, v)| ((*t - self.t0) / period, v * self.dt)).collect();
                flows.push(((self.t - self.t0) / period, now));
                // (1 + r)^-t as exp(-t ln(1 + r)): one logarithm per rate instead of a power per flow
                let npv = |r: f64| {
                    let l = r.ln_1p();
                    init + flows.iter().map(|(t, a)| a * (-t * l).exp()).sum::<f64>()
                };
                let (mut lo, mut hi) = (-0.999999, 1.0);
                let (flo, fhi) = (npv(lo), npv(hi));
                if !(flo.is_finite() && fhi.is_finite()) || flo * fhi > 0.0 {
                    return NA;
                }
                // bisect until the interval is as narrow as the numbers allow (about 50 halvings), not for a fixed 200
                for _ in 0..200 {
                    let mid = 0.5 * (lo + hi);
                    if (hi - lo) <= 4.0 * f64::EPSILON * mid.abs().max(1e-3) {
                        break;
                    }
                    if npv(mid) * flo > 0.0 { lo = mid } else { hi = mid }
                }
                0.5 * (lo + hi)
            }
            SKind::Derivn => {
                // the nth backward difference of the recorded values, divided by dt^n (0 until there is enough history)
                let order = self.ev(&s.args[1]).round().max(1.0) as usize;
                let x = self.ev(&s.args[0]);
                let h = &self.hist[s.hist];
                let mut vals: Vec<f64> = h.iter().rev().filter(|(t, _)| *t < self.t - 1e-9).take(order).map(|(_, v)| *v).collect();
                if vals.len() < order {
                    return 0.0;
                }
                vals.reverse();
                vals.push(x);
                for _ in 0..order {
                    vals = vals.windows(2).map(|w| w[1] - w[0]).collect();
                }
                vals[0] / self.dt.powi(order as i32)
            }
            SKind::Npv | SKind::Npve => {
                let stream = self.ev(&s.args[0]);
                let factor = s.args.get(3).map(|f| self.ev(f)).unwrap_or(1.0);
                (self.y[k + 1] + stream * self.dt * self.y[k]) * factor
            }
            SKind::Depreciate => {
                // this step's share of the current input plus the shares of the inputs still being written off
                let input = self.ev(&s.args[0]);
                let dtime = self.ev(&s.args[1]);
                if dtime <= 0.0 {
                    return input;
                }
                let n = ((dtime / self.dt).round() as usize).max(1);
                let since = self.t - (n as f64 - 0.5) * self.dt;
                let past: f64 = self.hist[s.hist].iter().filter(|(t, _)| *t > since && *t < self.t - 1e-9).map(|(_, x)| *x).sum();
                (input + past) / dtime
            }
            SKind::SampleIfTrue => {
                if self.init {
                    return self.y[k];
                }
                if self.ev(&s.args[0]) != 0.0 {
                    self.ev(&s.args[1])
                } else {
                    self.y[k]
                }
            }
            SKind::Smooth | SKind::Initial => self.y[k],
            SKind::Smooth3 => self.y[k + 2],
            SKind::Delay1 => {
                let d = self.ev(&s.args[1]);
                self.y[k] / d
            }
            SKind::Delay3 => {
                let d = self.ev(&s.args[1]) / 3.0;
                self.y[k + 2] / d
            }
            SKind::Trend => {
                let input = self.ev(&s.args[0]);
                let at = self.ev(&s.args[1]);
                if self.y[k] == 0.0 || at == 0.0 {
                    0.0
                } else {
                    (input - self.y[k]) / (self.y[k] * at)
                }
            }
            SKind::DelayFixed => {
                let d = self.ev(&s.args[1]);
                let target = self.t - d;
                let h = &self.hist[s.hist];
                if target < self.t0 - 1e-9 || h.is_empty() {
                    return self.y[k];
                }
                let i = h.partition_point(|(t, _)| *t <= target + 1e-9);
                if i == 0 { h[0].1 } else { h[i - 1].1 }
            }
        }
    }

    /// Derivatives of the hidden states, computed once every variable has a value.
    fn state_derivs(&mut self, record: bool) {
        let c = self.c;
        for s in &c.snodes {
            let k = s.slot;
            match s.kind {
                SKind::Initial => self.dy[k] = 0.0,
                SKind::Smooth | SKind::Trend => {
                    let input = self.ev(&s.args[0]);
                    let d = self.ev(&s.args[1]);
                    self.dy[k] = (input - self.y[k]) / d;
                }
                SKind::Smooth3 => {
                    let input = self.ev(&s.args[0]);
                    let d = self.ev(&s.args[1]) / 3.0;
                    self.dy[k] = (input - self.y[k]) / d;
                    self.dy[k + 1] = (self.y[k] - self.y[k + 1]) / d;
                    self.dy[k + 2] = (self.y[k + 1] - self.y[k + 2]) / d;
                }
                SKind::Delay1 => {
                    let input = self.ev(&s.args[0]);
                    let d = self.ev(&s.args[1]);
                    self.dy[k] = input - self.y[k] / d;
                }
                SKind::Delay3 => {
                    let input = self.ev(&s.args[0]);
                    let d = self.ev(&s.args[1]) / 3.0;
                    let (a, b, c) = (self.y[k] / d, self.y[k + 1] / d, self.y[k + 2] / d);
                    self.dy[k] = input - a;
                    self.dy[k + 1] = a - b;
                    self.dy[k + 2] = b - c;
                }
                SKind::Queue => {
                    let inflow = self.ev(&s.args[0]).max(0.0);
                    let outflow = self.ev(&s.args[1]).max(0.0);
                    // what can leave: everything waiting, and what arrives
                    let waiting = self.y[k + 2] + self.y[k] - self.y[k + 1];
                    self.dy[k] = inflow;
                    self.dy[k + 1] = outflow.min(waiting / self.dt + inflow);
                    if record {
                        let t = self.t;
                        let cum_in = self.y[k];
                        self.hist[s.hist].push((t, cum_in));
                    }
                }
                SKind::NpvAt => {
                    let df = self.discount(s, 1, 5, 4);
                    self.dy[k] = self.ev(&s.args[0]) * df;
                }
                SKind::Levelised => {
                    let off = [0.0, 0.5, 1.0][(self.ev(&s.args[4]) as usize).min(2)];
                    let df = self.discount_at(s, 2, 5, off);
                    self.dy[k] = self.ev(&s.args[0]) * df;
                    self.dy[k + 1] = self.ev(&s.args[1]) * df;
                }
                SKind::Payback => {
                    let (c, cross) = self.payback_step(s);
                    self.dy[k] = c / self.dt;
                    // slots: cumulative, the crossing time, and 1 once it is known (the state only ever moves by adding)
                    let new = cross.is_some() && self.y[k + 2] < 0.5;
                    self.dy[k + 1] = if new { cross.unwrap() / self.dt } else { 0.0 };
                    self.dy[k + 2] = if new { 1.0 / self.dt } else { 0.0 };
                }
                SKind::Previous => self.dy[k] = (self.ev(&s.args[0]) - self.y[k]) / self.dt,
                SKind::Carry => {
                    let amount = self.ev(&s.args[0]) * self.dt + self.y[k];
                    let moved = (amount + 1e-9).floor();
                    self.dy[k] = (amount - moved - self.y[k]) / self.dt;
                }
                SKind::History => {
                    if record {
                        let (x, t) = (self.ev(&s.args[0]), self.t);
                        self.hist[s.hist].push((t, x));
                    }
                }
                SKind::Loan => {
                    if record {
                        let amount = self.ev(&s.args[0]) * self.dt;
                        let t = self.t;
                        self.hist[s.hist].push((t, amount));
                    }
                }
                SKind::Tax => {
                    if record {
                        self.tax_record(s);
                    }
                }
                SKind::Derivn | SKind::Irr => {
                    if record {
                        let input = self.ev(&s.args[0]);
                        let t = self.t;
                        self.hist[s.hist].push((t, input));
                    }
                }
                SKind::Npv | SKind::Npve => {
                    let stream = self.ev(&s.args[0]);
                    let r = self.ev(&s.args[1]);
                    self.dy[k] = -self.y[k] * r / (1.0 + r * self.dt);
                    self.dy[k + 1] = stream * self.y[k];
                }
                SKind::Depreciate => {
                    if record {
                        let input = self.ev(&s.args[0]);
                        let t = self.t;
                        self.hist[s.hist].push((t, input));
                    }
                }
                SKind::SampleIfTrue => {
                    // move the held value to this step's output, so the next step starts from it
                    let v = if self.ev(&s.args[0]) != 0.0 { self.ev(&s.args[1]) } else { self.y[k] };
                    self.dy[k] = (v - self.y[k]) / self.dt;
                }
                SKind::DelayFixed => {
                    self.dy[k] = 0.0;
                    if record {
                        let input = self.ev(&s.args[0]);
                        let t = self.t;
                        self.hist[s.hist].push((t, input));
                    }
                }
            }
            let _ = s.width;
        }
    }

    /// Non-negative stocks: scale the outflows down so that the stock cannot be drained below zero within a step
    /// (Stella's rule). A = content / DT + inflows; if the outflows O exceed A, each is multiplied by A / O (proportional
    /// sharing; the model has no priority setting). The flow values stay the constrained ones, so the receiving stock
    /// gets exactly what left. Chains of non-negative stocks are settled by repeating the sweep.
    fn constrain_outflows(&mut self, record: bool) {
        let c = self.c;
        if !c.non_negative.iter().any(|b| *b) {
            return;
        }
        let mut worst: Vec<Option<(f64, f64)>> = vec![None; c.stock_flows.len()];
        for sweep in 0..8 {
            let mut changed = false;
            for (k, (st, ins, outs)) in c.stock_flows.iter().enumerate() {
                if !c.non_negative[*st] || outs.is_empty() {
                    continue;
                }
                let slot = c.stock_slot[*st].unwrap();
                let o: f64 = outs.iter().map(|f| self.vals[*f].max(0.0)).sum();
                if !(o > 0.0) {
                    continue;
                }
                let a = self.y[slot] / self.dt
                    + ins.iter().map(|f| self.vals[*f]).sum::<f64>()
                    - outs.iter().map(|f| self.vals[*f].min(0.0)).sum::<f64>();
                let a = a.max(0.0);
                if o > a * (1.0 + 1e-12) {
                    let f = a / o;
                    for j in outs {
                        if self.vals[*j] > 0.0 {
                            self.vals[*j] *= f;
                        }
                    }
                    changed = true;
                    if sweep == 0 || worst[k].is_none() {
                        worst[k] = Some((o, a));
                    }
                }
            }
            if !changed {
                break;
            }
        }
        if record && !self.init {
            // the log holds the start of each spell of constraint, not every step of it
            for (k, w) in worst.iter().enumerate() {
                if let Some((req, allowed)) = w {
                    if !self.constraint_active[k] {
                        let st = c.stock_flows[k].0;
                        self.constraints.push(Constraint { time: self.t, stock: c.names[st].clone(), requested: *req, allowed: *allowed });
                    }
                }
                self.constraint_active[k] = w.is_some();
            }
        }
    }

    /// (1 + r)^-(k + offset) with k = (t - t0) / period: the discount factor of the step that starts now. The rate is
    /// argument `rate`, the period argument `period`, the offset argument `off`.
    fn discount(&mut self, s: &SNode, rate: usize, period: usize, off: usize) -> f64 {
        let o = self.ev(&s.args[off]);
        self.discount_at(s, rate, period, o)
    }
    fn discount_at(&mut self, s: &SNode, rate: usize, period: usize, off: f64) -> f64 {
        let (r, p) = (self.ev(&s.args[rate]), self.ev(&s.args[period]));
        (1.0 + r).powf(-((self.t - self.t0) / p + off))
    }

    /// PAYBACK: this step's discounted (period end) amount, and the payback time if it is known by the end of this step.
    fn payback_step(&mut self, s: &SNode) -> (f64, Option<f64>) {
        let k = s.slot;
        let df = self.discount_at(s, 1, 2, 1.0);
        let c = self.ev(&s.args[0]) * self.dt * df;
        let (cum, found) = (self.y[k], self.y[k + 1]);
        if self.y[k + 2] > 0.5 {
            return (c, Some(found));
        }
        if cum < 0.0 && cum + c >= 0.0 && c > 0.0 {
            return (c, Some(self.t + self.dt * (-cum) / c));
        }
        (c, None)
    }

    /// LOAN: the measure of every tranche opened so far (and by this step's drawdown), summed.
    fn loan_value(&mut self, s: &SNode) -> f64 {
        let (rate, term, ty, grace, measure) = (self.ev(&s.args[1]), self.ev(&s.args[2]), self.ev(&s.args[3]) as i32, self.ev(&s.args[4]), self.ev(&s.args[5]) as i32);
        let (rp, ts, gs) = (self.ev(&s.args[6]), self.ev(&s.args[7]), self.ev(&s.args[8]));
        let r = rate * self.dt / rp;
        let n = ((term * ts / self.dt).round()).max(1.0);
        let g = (grace * gs / self.dt).round().max(0.0);
        let mut tranches: Vec<(f64, f64)> = self.hist[s.hist].clone();
        tranches.push((self.t, self.ev(&s.args[0]) * self.dt));
        let mut total = 0.0;
        for (t0, a) in tranches {
            if a == 0.0 {
                continue;
            }
            let age = ((self.t - t0) / self.dt).round();
            let (bal, interest, principal) = if age < g {
                (a, a * r, 0.0)
            } else {
                let j = age - g;
                if j >= n {
                    (0.0, 0.0, 0.0)
                } else {
                    match ty {
                        0 => {
                            let p = if r == 0.0 { a / n } else { a * r / (1.0 - (1.0 + r).powf(-n)) };
                            let bal = if r == 0.0 { a - p * j } else { a * (1.0 + r).powf(j) - p * ((1.0 + r).powf(j) - 1.0) / r };
                            (bal, bal * r, p - bal * r)
                        }
                        1 => {
                            let bal = a * (1.0 - j / n);
                            (bal, bal * r, a / n)
                        }
                        _ => (a, a * r, if j == n - 1.0 { a } else { 0.0 }),
                    }
                }
            };
            total += match measure {
                0 => bal,
                1 => (interest + principal) / self.dt,
                2 => interest / self.dt,
                _ => principal / self.dt,
            };
        }
        total
    }

    /// TAX: the usable losses at this time (not expired).
    fn usable_losses(&mut self, s: &SNode) -> f64 {
        let cf = self.ev(&s.args[2]) != 0.0;
        if !cf {
            return 0.0;
        }
        let years = self.ev(&s.args[3]) * self.ev(&s.args[4]);
        let t = self.t;
        self.hist[s.hist].iter().filter(|(tl, _)| years <= 0.0 || t - tl <= years + 1e-9).map(|(_, r)| *r).sum()
    }

    fn tax_value(&mut self, s: &SNode) -> f64 {
        let (x, rate) = (self.ev(&s.args[0]) * self.dt, self.ev(&s.args[1]));
        if x <= 0.0 {
            return 0.0;
        }
        let usable = self.usable_losses(s).min(x);
        rate * (x - usable) / self.dt
    }

    fn tax_record(&mut self, s: &SNode) {
        let x = self.ev(&s.args[0]) * self.dt;
        if self.ev(&s.args[2]) == 0.0 {
            return;
        }
        let years = self.ev(&s.args[3]) * self.ev(&s.args[4]);
        let t = self.t;
        let pool = &mut self.hist[s.hist];
        pool.retain(|(tl, r)| *r > 1e-12 && (years <= 0.0 || t - tl <= years + 1e-9));
        if x < 0.0 {
            pool.push((t, -x));
        } else {
            let mut left = x;
            for (_, r) in pool.iter_mut() {
                let take = r.min(left);
                *r -= take;
                left -= take;
                if left <= 0.0 {
                    break;
                }
            }
        }
    }

    /// A flow's value per step of the clock: in a calendar model, a flow per Day counts the real days of the step.
    fn flow_rate(&self, f: usize) -> f64 {
        let v = self.vals[f];
        match (self.c.flow_per.get(f).copied().flatten(), self.c.calendar.as_ref()) {
            (Some(FlowPer::Day), Some(cal)) => v * cal.days_in_step(self.t, self.t0),
            (Some(FlowPer::Hour), Some(cal)) => v * 24.0 * cal.days_in_step(self.t, self.t0),
            _ => v,
        }
    }

    /// Compute every variable and every derivative at the current state.
    fn pass(&mut self, record: bool) {
        let c = self.c;
        for &i in &c.order_run {
            let v = self.ev(c.exprs[i].as_ref().unwrap());
            self.vals[i] = v;
        }
        self.constrain_outflows(record);
        for (st, ins, outs) in &c.stock_flows {
            let slot = c.stock_slot[*st].unwrap();
            let r: f64 = ins.iter().map(|f| self.flow_rate(*f)).sum::<f64>()
                - outs.iter().map(|f| self.flow_rate(*f)).sum::<f64>();
            self.dy[slot] = r;
        }
        self.state_derivs(record);
    }
}

/// The middle value (the mean of the two middle ones for an even count).
fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let n = s.len();
    if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 }
}

/// The gamma function (Lanczos approximation, accurate to about 1e-13).
fn gamma(x: f64) -> f64 {
    use std::f64::consts::PI;
    if x < 0.5 {
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9, 676.520_368_121_885_1, -1_259.139_216_722_402_8, 771.323_428_777_653_1,
        -176.615_029_162_140_6, 12.507_343_278_686_905, -0.138_571_095_265_720_12, 9.984_369_578_019_572e-6, 1.505_632_735_149_311_6e-7,
    ];
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + G + 0.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    (2.0 * PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
}

/// The error function (Abramowitz & Stegun 7.1.26, about 1.5e-7).
fn erf(x: f64) -> f64 {
    let s = x.signum();
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0 - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t + 0.254_829_592) * t * (-x * x).exp();
    s * y
}

fn b2f(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

pub fn simulate(c: &Compiled, model: &Model, name: &str) -> Result<Run, String> {
    simulate_impl(c, model, name, None, None, None)
}

/// Run the model and record, at every step after the first, the score of every causal link
/// (Loops That Matter; see `loopscore.rs`). Euler integration only.
pub fn simulate_ltm(c: &Compiled, model: &Model) -> Result<(Run, Ltm), String> {
    if model.settings.method != Method::Euler {
        return Err("link scores are defined for Euler integration; set the method to Euler".into());
    }
    let mut l = Ltm::new(c);
    let r = simulate_impl(c, model, "ltm", Some(&mut l), None, None)?;
    Ok((r, l))
}

/// How a causal link is scored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinkKind {
    /// z is computed from x: change in z caused by x over the whole change in z, signed by the direction of the effect.
    Direct,
    /// A flow into (+) or out of (-) a stock: its share of the change in the stock's net flow (so that a loop's score
    /// describes what drives the *curvature* of the stock's path).
    FlowIn,
    FlowOut,
    /// Into a stage of a SMOOTH, SMOOTH3, DELAY1 or DELAY3 (a hidden stock): the input or the previous stage; and the
    /// stage's own draining term.
    StageIn,
    StageSelf,
    /// Through DELAY FIXED (and the conveyor DELAY(..., exact=True, leak=...) that compiles to it): the output is the
    /// input of one delay ago. Convention (see wiki/Find-What-Matters.md): the loop is cut at the delay, the link
    /// carries the whole output change (score 1 in absolute value, shared among the variables of the input
    /// expression, positive), at the time the output changes.
    Lagged,
    /// Through a stateful function the scores cannot follow (NPV, IRR, queue ages...): the link exists, the score does not.
    Unscored,
}

pub struct LtmLink {
    pub from: usize,
    pub to: usize,
    pub kind: LinkKind,
    pub scores: Vec<f64>,
}

/// Link scores of a run. Nodes `0..n` are the model's variables, then one node per stage of each supported
/// SMOOTH, SMOOTH3, DELAY1 or DELAY3.
pub struct Ltm {
    pub names: Vec<String>,
    pub is_stock: Vec<bool>,
    pub times: Vec<f64>,
    pub links: Vec<LtmLink>,
    /// extra node → (stateful node index, state slot of the stage, stage number)
    smooth_nodes: Vec<(usize, usize, usize)>,
    /// (from, to) of a `Lagged` link → the DELAY FIXED node it goes through
    lag_node: HashMap<(usize, usize), usize>,
    n_vars: usize,
}

fn stages_of(k: SKind) -> usize {
    match k {
        // TREND keeps an exponential average of its input (one stage); a QUEUE FIFO is a level fed by its inflow and
        // drained by its outflow (one stage)
        SKind::Smooth | SKind::Delay1 | SKind::Trend | SKind::Queue => 1,
        SKind::Smooth3 | SKind::Delay3 => 3,
        _ => 0,
    }
}

fn c_refs(c: &C, snodes: &[SNode], out: &mut Vec<(usize, bool)>) {
    // (variable, through an unsupported stateful function)
    match c {
        C::Var(i) => out.push((*i, false)),
        C::Num(_) | C::Time | C::T0 | C::T1 | C::Dt => {}
        C::Neg(a) | C::Not(a) | C::Table(_, a) => c_refs(a, snodes, out),
        C::Bin(_, a, b) | C::ActiveInit(a, b) => {
            c_refs(a, snodes, out);
            c_refs(b, snodes, out);
        }
        C::If(a, b, d) => {
            c_refs(a, snodes, out);
            c_refs(b, snodes, out);
            c_refs(d, snodes, out);
        }
        C::Pure(_, v) | C::TableMode(_, _, v) => v.iter().for_each(|x| c_refs(x, snodes, out)),
        C::QueueInfo(var, _, v) => {
            out.push((*var, true));
            v.iter().for_each(|x| c_refs(x, snodes, out));
        }
        C::State(n) => {
            let s = &snodes[*n];
            if s.kind == SKind::Trend {
                // the output reads the input and the averaging time directly (and the hidden average, a stage)
                s.args.iter().take(2).for_each(|a| c_refs(a, snodes, out));
            } else if s.kind == SKind::DelayFixed {
                // followed through a `Lagged` link, added in `Ltm::new`
            } else if stages_of(s.kind) == 0 {
                let mut inner = Vec::new();
                c_refs(&s.args[0], snodes, &mut inner);
                out.extend(inner.into_iter().map(|(i, _)| (i, true)));
            }
        }
    }
}

/// The stateful nodes an expression reads. With `deep`, also those read by the inputs of those nodes.
fn smooth_refs(c: &C, snodes: &[SNode], deep: bool, out: &mut Vec<usize>) {
    match c {
        C::State(n) => {
            out.push(*n);
            if deep {
                smooth_refs(&snodes[*n].args[0], snodes, deep, out);
                if snodes[*n].kind == SKind::Queue {
                    smooth_refs(&snodes[*n].args[1], snodes, deep, out);
                }
            }
        }
        C::Neg(a) | C::Not(a) | C::Table(_, a) => smooth_refs(a, snodes, deep, out),
        C::Bin(_, a, b) | C::ActiveInit(a, b) => {
            smooth_refs(a, snodes, deep, out);
            smooth_refs(b, snodes, deep, out);
        }
        C::If(a, b, d) => {
            smooth_refs(a, snodes, deep, out);
            smooth_refs(b, snodes, deep, out);
            smooth_refs(d, snodes, deep, out);
        }
        C::Pure(_, v) | C::TableMode(_, _, v) | C::QueueInfo(_, _, v) => v.iter().for_each(|x| smooth_refs(x, snodes, deep, out)),
        _ => {}
    }
}

/// The causal graph of a compiled model: node names and (from, to) edges (flows into stocks included).
pub fn influence_graph(c: &Compiled) -> (Vec<String>, Vec<(usize, usize)>) {
    let l = Ltm::new(c);
    (l.names.clone(), l.links.iter().map(|k| (k.from, k.to)).collect())
}

impl Ltm {
    fn new(c: &Compiled) -> Ltm {
        let n = c.names.len();
        let mut names = c.names.clone();
        let mut is_stock: Vec<bool> = (0..n).map(|i| c.stock_slot[i].is_some()).collect();
        let mut links: Vec<LtmLink> = Vec::new();
        let mut add = |from: usize, to: usize, kind: LinkKind| {
            if !links.iter().any(|l| l.from == from && l.to == to) {
                links.push(LtmLink { from, to, kind, scores: Vec::new() });
            }
        };
        // stage nodes of the stateful functions we follow: only those used by a variable (or by the input of one)
        let mut chain_first: HashMap<usize, usize> = HashMap::new();
        let mut smooth_nodes: Vec<(usize, usize, usize)> = Vec::new();
        for z in 0..n {
            if c.stock_slot[z].is_some() {
                continue;
            }
            if let Some(e) = &c.exprs[z] {
                let mut sm = Vec::new();
                smooth_refs(e, &c.snodes, true, &mut sm);
                for k in sm {
                    let st = stages_of(c.snodes[k].kind);
                    if st > 0 && !chain_first.contains_key(&k) {
                        chain_first.insert(k, n + smooth_nodes.len());
                        for j in 0..st {
                            smooth_nodes.push((k, c.snodes[k].slot + j, j));
                            names.push(format!("{:?} stage {} of {}", c.snodes[k].kind, j + 1, c.names[z]).to_lowercase());
                            is_stock.push(true);
                        }
                    }
                }
            }
        }
        let mut lag_node: HashMap<(usize, usize), usize> = HashMap::new();
        // the output node of a followed stateful function (its last stage)
        let out_node = |k: usize| chain_first.get(&k).map(|f| f + stages_of(c.snodes[k].kind) - 1);
        // links into `target` from what the expression `e` reads: variables, and outputs of followed functions
        let mut ex_links: Vec<(usize, usize, LinkKind)> = Vec::new();
        for z in 0..n {
            if c.stock_slot[z].is_some() {
                continue;
            }
            if let Some(e) = &c.exprs[z] {
                let mut r = Vec::new();
                c_refs(e, &c.snodes, &mut r);
                for (x, un) in r {
                    ex_links.push((x, z, if un { LinkKind::Unscored } else { LinkKind::Direct }));
                }
                let mut sm = Vec::new();
                smooth_refs(e, &c.snodes, false, &mut sm);
                for k in sm {
                    if let Some(o) = out_node(k) {
                        ex_links.push((o, z, LinkKind::Direct));
                    }
                    let sk = &c.snodes[k];
                    if sk.kind == SKind::Trend {
                        // the output also reads what the input expression reads, directly
                        let mut inner = Vec::new();
                        smooth_refs(&sk.args[0], &c.snodes, false, &mut inner);
                        for kk in inner {
                            if let Some(o) = out_node(kk) {
                                ex_links.push((o, z, LinkKind::Direct));
                            }
                        }
                    }
                    if sk.kind == SKind::DelayFixed {
                        let mut r = Vec::new();
                        c_refs(&sk.args[0], &c.snodes, &mut r);
                        let mut inner = Vec::new();
                        smooth_refs(&sk.args[0], &c.snodes, false, &mut inner);
                        let from: Vec<usize> = r.iter().map(|(x, _)| *x).chain(inner.iter().filter_map(|&kk| out_node(kk))).collect();
                        for x in from {
                            lag_node.insert((x, z), k);
                            ex_links.push((x, z, LinkKind::Lagged));
                        }
                    }
                }
            }
        }
        for (x, z, kind) in ex_links {
            add(x, z, kind);
        }
        for (&k, &first) in &chain_first {
            let queue = c.snodes[k].kind == SKind::Queue;
            let mut r = Vec::new();
            c_refs(&c.snodes[k].args[0], &c.snodes, &mut r);
            for (x, un) in r {
                add(x, first, if un { LinkKind::Unscored } else { LinkKind::StageIn });
            }
            // an input that is itself the output of a function we follow
            let mut inner = Vec::new();
            smooth_refs(&c.snodes[k].args[0], &c.snodes, false, &mut inner);
            for kk in inner {
                if let Some(o) = out_node(kk) {
                    add(o, first, LinkKind::StageIn);
                }
            }
            if queue {
                // the outflow of a queue drains it: what it reads is linked with a negative score
                let mut r = Vec::new();
                c_refs(&c.snodes[k].args[1], &c.snodes, &mut r);
                for (x, un) in r {
                    add(x, first, if un { LinkKind::Unscored } else { LinkKind::StageSelf });
                }
                let mut inner = Vec::new();
                smooth_refs(&c.snodes[k].args[1], &c.snodes, false, &mut inner);
                for kk in inner {
                    if let Some(o) = out_node(kk) {
                        add(o, first, LinkKind::StageSelf);
                    }
                }
                continue;
            }
            for j in 0..stages_of(c.snodes[k].kind) {
                if j > 0 {
                    add(first + j - 1, first + j, LinkKind::StageIn);
                }
                add(first + j, first + j, LinkKind::StageSelf);
            }
        }
        for (st, ins, outs) in &c.stock_flows {
            for f in ins {
                add(*f, *st, LinkKind::FlowIn);
            }
            for f in outs {
                add(*f, *st, LinkKind::FlowOut);
            }
        }
        Ltm { names, is_stock, times: Vec::new(), links, smooth_nodes, lag_node, n_vars: n }
    }

    /// Score every link at the current step from the previous step's state (Euler).
    fn record(&mut self, e: &mut Eval, prev_vals: &[f64], prev_y: &[f64], prev_t: f64) {
        let c = e.c;
        let (cur_vals, cur_y, cur_t) = (e.vals.clone(), e.y.clone(), e.t);
        let value = |vals: &[f64], y: &[f64], i: usize| match c.stock_slot[i] {
            Some(s) => y[s],
            None => vals[i],
        };
        let node_val = |me: &Ltm, vals: &[f64], y: &[f64], i: usize| {
            if i < me.n_vars {
                value(vals, y, i)
            } else {
                let (k, slot, _) = me.smooth_nodes[i - me.n_vars];
                // a queue's level is what came in less what went out, plus what it started with
                if c.snodes[k].kind == SKind::Queue { y[slot + 2] + y[slot] - y[slot + 1] } else { y[slot] }
            }
        };
        let mut out = Vec::with_capacity(self.links.len());
        for li in 0..self.links.len() {
            let (from, to, kind) = (self.links[li].from, self.links[li].to, self.links[li].kind);
            let sc = match kind {
                LinkKind::Unscored => f64::NAN,
                LinkKind::Direct => {
                    let (z0, z1) = (node_val(self, prev_vals, prev_y, to), node_val(self, &cur_vals, &cur_y, to));
                    let dz = z1 - z0;
                    let dx = node_val(self, &cur_vals, &cur_y, from) - node_val(self, prev_vals, prev_y, from);
                    if dz == 0.0 || dx == 0.0 || to >= self.n_vars {
                        0.0
                    } else {
                        // evaluate z with only x moved to its new value
                        e.vals.copy_from_slice(prev_vals);
                        e.y.copy_from_slice(prev_y);
                        if from < self.n_vars {
                            match c.stock_slot[from] {
                                Some(s) => e.y[s] = cur_y[s],
                                None => e.vals[from] = cur_vals[from],
                            }
                        } else {
                            let (k, s, _) = self.smooth_nodes[from - self.n_vars];
                            e.y[s] = cur_y[s];
                            if c.snodes[k].kind == SKind::Queue {
                                e.y[s + 1] = cur_y[s + 1];
                            }
                        }
                        e.t = prev_t;
                        let zx = e.ev(c.exprs[to].as_ref().unwrap());
                        let dxz = zx - z0;
                        let sign = if (dxz > 0.0) == (dx > 0.0) { 1.0 } else { -1.0 };
                        if dxz == 0.0 { 0.0 } else { (dxz / dz).abs() * sign }
                    }
                }
                LinkKind::Lagged => {
                    let k = self.lag_node[&(from, to)];
                    let dz = node_val(self, &cur_vals, &cur_y, to) - node_val(self, prev_vals, prev_y, to);
                    if dz == 0.0 || to >= self.n_vars {
                        0.0
                    } else {
                        // the share of the change of the delayed expression that `from` accounts for (1 when `from` is all of it)
                        let arg = &c.snodes[k].args[0];
                        let at = |e: &mut Eval, vals: &[f64], y: &[f64], t: f64| {
                            e.vals.copy_from_slice(vals);
                            e.y.copy_from_slice(y);
                            e.t = t;
                            e.ev(arg)
                        };
                        let (x1, x0) = (at(e, &cur_vals, &cur_y, cur_t), at(e, prev_vals, prev_y, prev_t));
                        let dx = x1 - x0;
                        if matches!(arg, C::Var(_)) || dx == 0.0 {
                            1.0
                        } else {
                            e.vals.copy_from_slice(prev_vals);
                            e.y.copy_from_slice(prev_y);
                            if from < self.n_vars {
                                match c.stock_slot[from] {
                                    Some(s) => e.y[s] = cur_y[s],
                                    None => e.vals[from] = cur_vals[from],
                                }
                            } else {
                                let (kk, s, _) = self.smooth_nodes[from - self.n_vars];
                                e.y[s] = cur_y[s];
                                if c.snodes[kk].kind == SKind::Queue {
                                    e.y[s + 1] = cur_y[s + 1];
                                }
                            }
                            e.t = prev_t;
                            let xf = e.ev(arg) - x0;
                            if (xf > 0.0) == (dx > 0.0) { (xf / dx).abs() } else { -(xf / dx).abs() }
                        }
                    }
                }
                LinkKind::FlowIn | LinkKind::FlowOut => {
                    // the share of the change in the stock's net flow that this flow's change accounts for
                    let (_, ins, outs) = c.stock_flows.iter().find(|(st, _, _)| *st == to).unwrap();
                    let d = |f: usize| cur_vals[f] - prev_vals[f];
                    let net: f64 = ins.iter().map(|f| d(*f)).sum::<f64>() - outs.iter().map(|f| d(*f)).sum::<f64>();
                    if net == 0.0 { 0.0 } else { (d(from) / net).abs() * if kind == LinkKind::FlowIn { 1.0 } else { -1.0 } }
                }
                LinkKind::StageIn | LinkKind::StageSelf => {
                    // the stage changes at (inflow term - outflow term); the share of the change in that net flow
                    let (k, _, stage) = self.smooth_nodes[to - self.n_vars];
                    let terms = |e: &mut Eval, vals: &[f64], y: &[f64], t: f64| -> (f64, f64) {
                        e.vals.copy_from_slice(vals);
                        e.y.copy_from_slice(y);
                        e.t = t;
                        let sn = &c.snodes[k];
                        let slot = sn.slot + stage;
                        let inp = if stage == 0 { e.ev(&sn.args[0]) } else { y[slot - 1] };
                        match sn.kind {
                            SKind::Smooth | SKind::Smooth3 | SKind::Trend => (inp, y[slot]),
                            SKind::Queue => {
                                // inflow and the outflow that actually leaves (what waits and arrives caps it)
                                let inflow = inp.max(0.0);
                                let outflow = e.ev(&sn.args[1]).max(0.0);
                                let waiting = y[slot + 2] + y[slot] - y[slot + 1];
                                (inflow, outflow.min(waiting / e.dt + inflow))
                            }
                            SKind::Delay1 => {
                                let d = e.ev(&sn.args[1]);
                                (inp, y[slot] / d)
                            }
                            _ => {
                                let d = e.ev(&sn.args[1]) / 3.0;
                                (if stage == 0 { inp } else { inp / d }, y[slot] / d)
                            }
                        }
                    };
                    let (a1, b1) = terms(e, &cur_vals, &cur_y, cur_t);
                    let (a0, b0) = terms(e, prev_vals, prev_y, prev_t);
                    let (da, db) = (a1 - a0, b1 - b0);
                    let net = da - db;
                    if net == 0.0 { 0.0 } else if kind == LinkKind::StageIn { (da / net).abs() } else { -(db / net).abs() }
                }
            };
            out.push(sc);
        }
        e.vals = cur_vals;
        e.y = cur_y;
        e.t = cur_t;
        for (l, v) in self.links.iter_mut().zip(out) {
            l.scores.push(v);
        }
        self.times.push(cur_t);
    }
}

/// The sign of a causal link at each saved time, from the partial derivative at the operating point along the base run
/// (not from the syntax of the equation): the output variable is evaluated twice from the same state, with the input
/// raised by a small relative step the second time and every other input held, and the sign of the change is taken.
pub struct Probe {
    /// (input variable index, output variable index)
    pairs: Vec<(usize, usize)>,
    /// per pair: the sign (+1, -1, or 0 = no effect at that time) at each saved time
    pub signs: Vec<Vec<i8>>,
    pub times: Vec<f64>,
}

impl Probe {
    fn record(&mut self, e: &mut Eval) {
        let c = e.c;
        let saved_streams = e.streams.clone();
        let saved_rng = e.rng;
        self.times.push(e.t);
        for (k, &(from, to)) in self.pairs.iter().enumerate() {
            let Some(expr) = c.exprs[to].as_ref() else {
                self.signs[k].push(0);
                continue;
            };
            let slot = c.stock_slot[from];
            let x0 = match slot {
                Some(s) => e.y[s],
                None => e.vals[from],
            };
            let h = if x0.abs() > 1e-12 { 1e-4 * x0.abs() } else { 1e-6 };
            let base = e.ev(expr);
            match slot {
                Some(s) => e.y[s] = x0 + h,
                None => e.vals[from] = x0 + h,
            }
            let up = e.ev(expr);
            match slot {
                Some(s) => e.y[s] = x0,
                None => e.vals[from] = x0,
            }
            let d = up - base;
            let tiny = 1e-12 * base.abs().max(up.abs()).max(1e-300);
            self.signs[k].push(if !d.is_finite() || d.abs() <= tiny { 0 } else if d > 0.0 { 1 } else { -1 });
            e.streams = saved_streams.clone();
            e.rng = saved_rng;
        }
    }
}

/// The signs over time of the links `pairs` (input name, output name), taken along the base run.
/// Returns the saved times and one series of signs per pair; a link into a stock or to an unknown name gets all zeros.
pub fn link_signs(model: &Model, pairs: &[(String, String)]) -> Result<(Vec<f64>, Vec<Vec<i8>>), String> {
    let c = compile(model).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("; "))?;
    let index: HashMap<String, usize> = c.names.iter().enumerate().map(|(i, n)| (expr::normalize(n), i)).collect();
    let mut idx = Vec::new();
    let mut known = Vec::new();
    for (a, b) in pairs {
        match (index.get(&expr::normalize(a)), index.get(&expr::normalize(b))) {
            (Some(&x), Some(&y)) if c.stock_slot[y].is_none() => {
                idx.push((x, y));
                known.push(true);
            }
            _ => known.push(false),
        }
    }
    let mut p = Probe { pairs: idx, signs: vec![Vec::new(); known.iter().filter(|k| **k).count()], times: Vec::new() };
    simulate_impl(&c, model, "probe", None, None, Some(&mut p))?;
    let n = p.times.len();
    let mut it = p.signs.into_iter();
    let signs = known.iter().map(|k| if *k { it.next().unwrap() } else { vec![0; n] }).collect();
    Ok((p.times, signs))
}

/// The state of a run at a time and the Jacobian of the state derivatives there.
pub struct Snap {
    pub at: f64,
    /// if set, the state is replaced by this one before the Jacobian is taken (to follow a parameter at a fixed state)
    pub fixed_y: Option<Vec<f64>>,
    pub time: f64,
    pub y: Vec<f64>,
    /// the state derivatives at the state (before the Jacobian is taken)
    pub dy: Vec<f64>,
    pub jac: Vec<Vec<f64>>,
}

/// The order of the model: all its state variables, the stocks and the hidden states of SMOOTH, DELAY and the like
/// (SMOOTH3 has three). None if the model does not compile.
pub fn order(model: &Model) -> Option<usize> {
    compile(model).ok().map(|c| c.n_states)
}

/// Names of the state variables in the order of the Jacobian: stocks, then hidden states of delays and smooths.
pub fn state_names(c: &Compiled) -> Vec<String> {
    let mut v = vec![String::new(); c.n_states];
    for (i, sl) in c.stock_slot.iter().enumerate() {
        if let Some(s) = sl {
            v[*s] = c.names[i].clone();
        }
    }
    for (k, x) in v.iter_mut().enumerate() {
        if x.is_empty() {
            *x = format!("hidden state {k}");
        }
    }
    v
}

/// Jacobian of the state derivatives at the first saved time not before `at` (Euler runs to there).
pub fn jacobian_at(model: &Model, at: f64, fixed_y: Option<Vec<f64>>) -> Result<Snap, String> {
    let c = compile(model).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("; "))?;
    let mut sn = Snap { at, fixed_y, time: f64::NAN, y: Vec::new(), dy: Vec::new(), jac: Vec::new() };
    simulate_impl(&c, model, "jac", None, Some(&mut sn), None)?;
    if sn.jac.is_empty() {
        return Err(format!("the run ends before time {at}"));
    }
    Ok(sn)
}

fn simulate_impl(c: &Compiled, model: &Model, name: &str, mut ltm: Option<&mut Ltm>, mut snap: Option<&mut Snap>, mut probe: Option<&mut Probe>) -> Result<Run, String> {
    let s = &model.settings;
    if !(s.time_step > 0.0) {
        return Err("Time step must be above zero".into());
    }
    if s.final_time <= s.initial_time {
        return Err("Final time must be after the initial time".into());
    }
    let steps = ((s.final_time - s.initial_time) / s.time_step).round() as usize;
    if steps > 2_000_000 {
        return Err("Too many steps: raise the time step or shorten the run".into());
    }
    let n = c.names.len();
    let mut e = Eval {
        c,
        vals: vec![f64::NAN; n],
        y: vec![0.0; c.n_states],
        dy: vec![0.0; c.n_states],
        t: s.initial_time,
        t0: s.initial_time,
        t1: s.final_time,
        dt: s.time_step,
        init: true,
        hist: vec![Vec::new(); c.n_hist],
        rng: 0x9E37_79B9_7F4A_7C15,
        streams: Vec::new(),
        constraints: Vec::new(),
        constraint_active: vec![false; c.stock_flows.len()],
    };
    // initial values
    for &i in &c.order_init {
        let v = e.ev(c.exprs[i].as_ref().unwrap());
        match c.stock_slot[i] {
            Some(slot) => e.y[slot] = v,
            None => e.vals[i] = v,
        }
    }
    e.init = false;
    let mut times = Vec::with_capacity(steps + 1);
    let mut cols: Vec<Vec<f64>> = vec![Vec::with_capacity(steps + 1); n];
let mut prev: Option<(Vec<f64>, Vec<f64>, f64)> = None;
    for step in 0..=steps {
        e.t = s.initial_time + step as f64 * s.time_step;
        e.pass(true);
        if let Some(l) = ltm.as_deref_mut() {
            if let Some((pv, py, pt)) = &prev {
                l.record(&mut e, pv, py, *pt);
            }
            prev = Some((e.vals.clone(), e.y.clone(), e.t));
        }
        if let Some(p) = probe.as_deref_mut() {
            p.record(&mut e);
        }
        if let Some(sn) = snap.as_deref_mut() {
            if e.t >= sn.at - 1e-9 {
                if let Some(fy) = &sn.fixed_y {
                    e.y.copy_from_slice(fy);
                    e.pass(false);
                }
                sn.time = e.t;
                sn.y = e.y.clone();
                sn.dy = e.dy.clone();
                let k = e.y.len();
                let base = e.y.clone();
                let mut jac = vec![vec![0.0; k]; k];
                for j in 0..k {
                    let h = 1e-6 * base[j].abs().max(1.0);
                    e.y[j] = base[j] + h;
                    e.pass(false);
                    let up = e.dy.clone();
                    e.y[j] = base[j] - h;
                    e.pass(false);
                    let dn = e.dy.clone();
                    e.y[j] = base[j];
                    for i in 0..k {
                        jac[i][j] = (up[i] - dn[i]) / (2.0 * h);
                    }
                }
                sn.jac = jac;
                return Ok(Run { name: name.into(), times: vec![e.t], time_units: s.time_units.clone(), ..Default::default() });
            }
        }
        times.push(e.t);
        for i in 0..n {
            let v = match c.stock_slot[i] {
                Some(slot) => e.y[slot],
                None => e.vals[i],
            };
            cols[i].push(v);
        }
        if step == steps {
            break;
        }
        let dt = s.time_step;
        match s.method {
            Method::Euler => {
                for k in 0..e.y.len() {
                    e.y[k] += dt * e.dy[k];
                }
            }
            Method::RungeKutta2 | Method::RungeKutta3 => {
                let y0 = e.y.clone();
                let t0 = e.t;
                let k1 = e.dy.clone();
                let stage = |e: &mut Eval, h: f64, k: &[f64]| -> Vec<f64> {
                    for j in 0..y0.len() {
                        e.y[j] = y0[j] + h * k[j];
                    }
                    e.t = t0 + h;
                    e.pass(false);
                    e.dy.clone()
                };
                if s.method == Method::RungeKutta2 {
                    // Heun: an Euler step, then the average of the slopes at both ends
                    let k2 = stage(&mut e, dt, &k1);
                    for j in 0..y0.len() {
                        e.y[j] = y0[j] + dt / 2.0 * (k1[j] + k2[j]);
                    }
                } else {
                    // Kutta 3rd order: slopes at the start, the middle, and the end (from -k1 + 2 k2)
                    let k2 = stage(&mut e, dt / 2.0, &k1);
                    let back: Vec<f64> = (0..y0.len()).map(|j| -k1[j] + 2.0 * k2[j]).collect();
                    let k3 = stage(&mut e, dt, &back);
                    for j in 0..y0.len() {
                        e.y[j] = y0[j] + dt / 6.0 * (k1[j] + 4.0 * k2[j] + k3[j]);
                    }
                }
            }
            Method::RungeKutta4 => {
                let y0 = e.y.clone();
                let t0 = e.t;
                let k1 = e.dy.clone();
                let stage = |e: &mut Eval, h: f64, k: &[f64]| -> Vec<f64> {
                    for j in 0..y0.len() {
                        e.y[j] = y0[j] + h * k[j];
                    }
                    e.t = t0 + h;
                    e.pass(false);
                    e.dy.clone()
                };
                let k2 = stage(&mut e, dt / 2.0, &k1);
                let k3 = stage(&mut e, dt / 2.0, &k2);
                let k4 = stage(&mut e, dt, &k3);
                for j in 0..y0.len() {
                    e.y[j] = y0[j] + dt / 6.0 * (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]);
                }
            }
        }
    }
    let mut series = HashMap::new();
    for (i, col) in cols.into_iter().enumerate() {
        if c.has_series[i] {
            series.insert(expr::normalize(&c.names[i]), col);
        }
    }
    let _ = &c.kinds;
    // a set variable is shown as one number: the total (or average) over its items
    for r in &c.rollups {
        let parts: Vec<&Vec<f64>> = r.parts.iter().filter_map(|p| series.get(&expr::normalize(p))).collect();
        if parts.is_empty() || parts.len() != r.parts.len() {
            continue;
        }
        let mut sum = vec![0.0; times.len()];
        for p in &parts {
            for (a, b) in sum.iter_mut().zip(p.iter()) {
                *a += *b;
            }
        }
        if r.average {
            let n = parts.len() as f64;
            sum.iter_mut().for_each(|a| *a /= n);
        }
        series.insert(expr::normalize(&r.name), sum);
    }
    Ok(Run {
        name: name.into(),
        times,
        series,
        time_units: s.time_units.clone(),
        constraints: std::mem::take(&mut e.constraints),
    })
}

#[cfg(test)]
mod seed_tests {
    use super::*;
    use crate::model::{Kind, Var};

    fn noisy(seed: &str) -> Model {
        let mut m = Model::default();
        m.settings.final_time = 50.0;
        m.settings.time_step = 1.0;
        let mut a = Var::new(1, "Noise", Kind::Aux, [0.0, 0.0]);
        a.equation = format!("RANDOM NORMAL(-100, 100, 0, 1, {seed})");
        let mut b = Var::new(2, "Other noise", Kind::Aux, [0.0, 0.0]);
        b.equation = format!("RANDOM NORMAL(-100, 100, 0, 1, {seed})");
        m.vars = vec![a, b];
        m.next_id = 10;
        m
    }

    fn series(m: &Model, name: &str) -> Vec<f64> {
        simulate(&compile(m).unwrap(), m, "x").unwrap().get(name).unwrap().clone()
    }

    #[test]
    fn the_seed_argument_picks_a_repeatable_independent_stream() {
        let (a1, a1b, a2) = (series(&noisy("1"), "Noise"), series(&noisy("1"), "Noise"), series(&noisy("2"), "Noise"));
        assert_eq!(a1, a1b, "the same seed repeats");
        assert_ne!(a1, a2, "another seed gives other numbers");
        let mean = a1.iter().sum::<f64>() / a1.len() as f64;
        assert!(a1.iter().any(|x| *x > 0.5) && a1.iter().any(|x| *x < -0.5) && mean.abs() < 0.6, "roughly standard normal: mean {mean}");
        // two call sites with the same seed are not the same stream
        assert_ne!(series(&noisy("1"), "Noise"), series(&noisy("1"), "Other noise"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn example_runs() {
        let m = Model::example();
        let c = compile(&m)
            .map_err(|p| p.iter().map(|p| p.message.clone()).collect::<Vec<_>>())
            .unwrap();
        let r = simulate(&c, &m, "t").unwrap();
        let tot = r.get("Total Population").unwrap();
        assert!((tot[tot.len() - 1] - 10000.0).abs() < 1e-6);
        let rec = r.get("Recovered").unwrap();
        assert!(rec[rec.len() - 1] > 1000.0);
    }
    #[test]
    fn math_library_matches_pythons_math_module() {
        let mut m = Model::default();
        m.settings = crate::model::Settings { initial_time: 0.0, final_time: 1.0, time_step: 1.0, time_units: "s".into(), method: Method::Euler, ..Default::default() };
        let cases: &[(&str, f64)] = &[
            ("GAMMA(5)", 24.0),
            ("FACTORIAL(6)", 720.0),
            ("ERF(0.5)", 0.520_499_877_8),
            ("MEDIAN(3, 1, 2)", 2.0),
            ("MEDIAN(4, 1, 2, 3)", 2.5),
            ("MEAN(1, 2, 6)", 3.0),
            ("CLAMP(5, 0, 3)", 3.0),
            ("CLAMP(-5, 0, 3)", 0.0),
            ("PI()", std::f64::consts::PI),
            ("ARCTAN2(1, 1)", std::f64::consts::FRAC_PI_4),
            ("SIGN(-3) + SIGN(0) * 10", -1.0),
            ("HYPOT(3, 4)", 5.0),
            ("LOG10(1000) + LOG2(8)", 6.0),
            ("TANH(0)", 0.0),
            ("FLOOR(-1.5)", -2.0),
            ("CBRT(27)", 3.0),
            ("DEGREES(RADIANS(90))", 90.0),
            ("2 * PI() * 2", 4.0 * std::f64::consts::PI),
        ];
        for (i, (eq, _)) in cases.iter().enumerate() {
            let id = m.new_id();
            let mut v = crate::model::Var::new(id, &format!("case {i}"), Kind::Aux, [0.0, 0.0]);
            v.equation = (*eq).into();
            m.vars.push(v);
        }
        let c = compile(&m).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>()).unwrap();
        let r = simulate(&c, &m, "t").unwrap();
        for (i, (eq, want)) in cases.iter().enumerate() {
            let got = r.get(&format!("case {i}")).unwrap()[0];
            assert!((got - want).abs() < 1e-6, "{eq}: got {got}, want {want}");
        }
    }

    #[test]
    fn powersim_style_functions() {
        let mut m = Model::default();
        m.settings = crate::model::Settings { initial_time: 0.0, final_time: 50.0, time_step: 0.25, time_units: "s".into(), method: Method::Euler, ..Default::default() };
        for (n, eq) in [
            ("g1", "GRAPH(5, 0, 5, 0, 10, 20)"),
            ("g2", "GRAPH(-3, 0, 5, 0, 10, 20)"),
            ("g3", "GRAPH(99, 0, 5, 0, 10, 20)"),
            ("g4", "GRAPH(2.5, 0, 5, 0, 10, 20)"),
            ("gs", "GRAPHSTEP(7, 0, 5, 1, 2, 3)"),
            ("sw", "SINWAVE(2, 4)"),
            ("cw", "COSWAVE(2, 4)"),
            ("input", "STEP(10, 5)"),
            ("inf3", "DELAYINF(input, 6, 3)"),
            ("sm3", "SMOOTH3(input, 6)"),
            ("mtr3", "DELAYMTR(input, 6, 3)"),
            ("d3", "DELAY3(input, 6)"),
        ] {
            let id = m.new_id();
            let mut v = crate::model::Var::new(id, n, Kind::Aux, [0.0, 0.0]);
            v.equation = eq.into();
            m.vars.push(v);
        }
        let c = compile(&m).map_err(|p| p.iter().map(|x| x.message.clone()).collect::<Vec<_>>()).unwrap();
        let r = simulate(&c, &m, "t").unwrap();
        let at = |n: &str, t: f64| r.get(n).unwrap()[(t / 0.25) as usize];
        assert_eq!((at("g1", 0.0), at("g2", 0.0), at("g3", 0.0), at("g4", 0.0)), (10.0, 0.0, 20.0, 5.0));
        assert_eq!(at("gs", 0.0), 2.0);
        assert!((at("sw", 1.0) - 2.0).abs() < 1e-9 && at("sw", 0.0).abs() < 1e-9);
        assert!((at("cw", 0.0) - 2.0).abs() < 1e-9);
        for t in [8.0, 12.0, 20.0, 40.0] {
            assert!((at("inf3", t) - at("sm3", t)).abs() < 1e-9, "DELAYINF(x,d,3) = SMOOTH3(x,d) at {t}");
            assert!((at("mtr3", t) - at("d3", t)).abs() < 1e-9, "DELAYMTR(x,d,3) = DELAY3(x,d) at {t}");
        }
        assert!(at("mtr3", 40.0) > 9.9);
    }

    #[test]
    fn higher_order_methods_are_more_accurate_on_exponential_growth() {
        // y' = y, y(0) = 1: the exact answer at t = 1 is e
        let run = |method: Method| {
            let mut m = Model::default();
            m.settings = crate::model::Settings { initial_time: 0.0, final_time: 1.0, time_step: 0.25, time_units: "s".into(), method, ..Default::default() };
            let mut y = crate::model::Var::new(1, "y", Kind::Stock, [0.0, 0.0]);
            y.equation = "1".into();
            let mut g = crate::model::Var::new(2, "g", Kind::Flow, [0.0, 0.0]);
            g.equation = "y".into();
            g.to = Some(1);
            m.vars.push(y);
            m.vars.push(g);
            let c = compile(&m).ok().unwrap();
            (*simulate(&c, &m, "t").unwrap().get("y").unwrap().last().unwrap() - std::f64::consts::E).abs()
        };
        let (e1, e2, e3, e4) = (run(Method::Euler), run(Method::RungeKutta2), run(Method::RungeKutta3), run(Method::RungeKutta4));
        assert!(e1 > e2 && e2 > e3 && e3 > e4, "errors {e1} {e2} {e3} {e4} should shrink with the order");
        assert!(e4 < 1e-4);
    }

    #[test]
    fn delays_and_smooth() {
        let mut m = Model::default();
        m.settings = crate::model::Settings {
            initial_time: 0.0,
            final_time: 50.0,
            time_step: 0.25,
            time_units: "s".into(),
            method: Method::Euler, ..Default::default()
        };
        for (n, eq) in [
            ("input", "STEP(10, 5)"),
            ("smoothed", "SMOOTH(input, 3)"),
            ("fixed", "DELAY FIXED(input, 4, 0)"),
            ("d3", "DELAY3(input, 6)"),
        ] {
            let id = m.new_id();
            let mut v = crate::model::Var::new(id, n, Kind::Aux, [0.0, 0.0]);
            v.equation = eq.into();
            m.vars.push(v);
        }
        let c = compile(&m).ok().unwrap();
        let r = simulate(&c, &m, "t").unwrap();
        let s = r.get("smoothed").unwrap();
        assert!(s[0] == 0.0 && (s[s.len() - 1] - 10.0).abs() < 0.01);
        let f = r.get("fixed").unwrap();
        assert_eq!(f[(8.0 / 0.25) as usize], 0.0);
        assert_eq!(f[(9.5 / 0.25) as usize], 10.0);
        let d = r.get("d3").unwrap();
        assert!((d[d.len() - 1] - 10.0).abs() < 0.1);
    }
}

/// ln Γ(x) for x > 0 (Lanczos, about 15 significant digits), as Vensim's GAMMA LN.
fn ln_gamma(x: f64) -> f64 {
    if x < 0.5 {
        // reflection: Γ(x) Γ(1 − x) = π / sin(πx)
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).abs().ln() - ln_gamma(1.0 - x);
    }
    const G: [f64; 9] = [
        0.999_999_999_999_809_9, 676.520_368_121_885_1, -1_259.139_216_722_402_8, 771.323_428_777_653_1,
        -176.615_029_162_140_6, 12.507_343_278_686_905, -0.138_571_095_265_720_12, 9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let x = x - 1.0;
    let mut a = G[0];
    let t = x + 7.5;
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Vensim's missing value, :NA:.
pub const NA: f64 = -1.298074214633707e33;

/// The slope of a lookup at x (Vensim's LOOKUP SLOPE): the segment's slope; at a point, the mean of the
/// slopes either side; outside the points 0 (m = 0), the nearest segment's slope (m = 1) or :NA: (m = −1).
fn lookup_slope(t: &[[f64; 2]], x: f64, m: f64) -> f64 {
    let n = t.len();
    if n < 2 {
        return 0.0;
    }
    let seg = |i: usize| (t[i + 1][1] - t[i][1]) / (t[i + 1][0] - t[i][0]);
    if x < t[0][0] || x > t[n - 1][0] {
        return if m < 0.0 { NA } else if m > 0.0 { if x < t[0][0] { seg(0) } else { seg(n - 2) } } else { 0.0 };
    }
    if let Some(i) = t.iter().position(|p| p[0] == x) {
        if t.iter().filter(|p| p[0] == x).count() > 1 {
            return NA;
        }
        return match (i, n) {
            (0, _) => seg(0),
            (i, n) if i == n - 1 => seg(n - 2),
            (i, _) => 0.5 * (seg(i - 1) + seg(i)),
        };
    }
    let i = t.iter().position(|p| p[0] > x).unwrap_or(n - 1);
    seg(i - 1)
}

/// The area under a lookup from a to b (Vensim's LOOKUP AREA): flat beyond the end points, or continued in
/// straight lines when `extrapolate`.
fn lookup_area(t: &[[f64; 2]], a: f64, b: f64, extrapolate: bool) -> f64 {
    if a > b {
        return -lookup_area(t, b, a, extrapolate);
    }
    let f = |x: f64| if extrapolate { table_mode(t, 4, &[x]) } else { crate::expr::lookup(t, x) };
    // break [a, b] at the points; between them the curve is a straight line, so the trapezium rule is exact
    let mut xs = vec![a];
    xs.extend(t.iter().map(|p| p[0]).filter(|x| *x > a && *x < b));
    xs.push(b);
    xs.windows(2).map(|w| 0.5 * (f(w[0]) + f(w[1])) * (w[1] - w[0])).sum()
}

/// A lookup read in another mode (see `C::TableMode`). Points are sorted by x. `v` holds the arguments after
/// the lookup's name.
fn table_mode(t: &[[f64; 2]], mode: u8, v: &[f64]) -> f64 {
    if t.is_empty() {
        return if mode >= 7 { NA } else { 0.0 };
    }
    let n = t.len();
    let x = v.first().copied().unwrap_or(0.0);
    let in_range = |a: f64, b: f64| -> Vec<f64> { t.iter().filter(|p| p[0] >= a - 1e-9 && p[0] <= b + 1e-9).map(|p| p[1]).collect() };
    match mode {
        5 => return lookup_slope(t, x, v.get(1).copied().unwrap_or(0.0)),
        6 => return lookup_area(t, x, v.get(1).copied().unwrap_or(x), false),
        // GET DATA AT TIME: the value at exactly that time, else :NA:
        7 => return t.iter().find(|p| (p[0] - x).abs() <= 1e-9).map(|p| p[1]).unwrap_or(NA),
        // GET DATA BETWEEN TIMES(data, time, mode): 0 interpolate, 1 the later point, −1 the earlier; :NA: outside
        8 => {
            if x < t[0][0] - 1e-9 || x > t[n - 1][0] + 1e-9 {
                return NA;
            }
            let m = v.get(1).copied().unwrap_or(0.0);
            return if m >= 1.0 { table_mode(t, 1, &[x]) } else if m <= -1.0 { table_mode(t, 2, &[x]) } else { crate::expr::lookup(t, x) };
        }
        9 => return t[0][0],
        10 => return t[n - 1][0],
        11..=16 => {
            let ys = in_range(x, v.get(1).copied().unwrap_or(f64::INFINITY));
            if ys.is_empty() {
                return NA;
            }
            let k = ys.len() as f64;
            let mean = ys.iter().sum::<f64>() / k;
            return match mode {
                11 => ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                12 => ys.iter().cloned().fold(f64::INFINITY, f64::min),
                13 => mean,
                14 => {
                    let mut s = ys.clone();
                    s.sort_by(|a, b| a.total_cmp(b));
                    let h = s.len() / 2;
                    if s.len() % 2 == 1 { s[h] } else { 0.5 * (s[h - 1] + s[h]) }
                }
                15 => {
                    if ys.len() < 2 { 0.0 } else { (ys.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / (k - 1.0)).sqrt() }
                }
                _ => ys.iter().sum(),
            };
        }
        _ => {}
    }
    match mode {
        1 | 2 => {
            // forward: the y of the first point at or after x; backward: the y of the last point at or before x
            if x <= t[0][0] {
                return t[0][1];
            }
            if x >= t[n - 1][0] {
                return t[n - 1][1];
            }
            let i = t.iter().position(|p| p[0] >= x).unwrap_or(n - 1);
            if t[i][0] == x || mode == 1 { t[i][1] } else { t[i - 1][1] }
        }
        3 => {
            // invert: read the table with x and y swapped (and in rising order, so a falling curve works too)
            let mut inv: Vec<[f64; 2]> = t.iter().map(|p| [p[1], p[0]]).collect();
            inv.sort_by(|a, b| a[0].total_cmp(&b[0]));
            crate::expr::lookup(&inv, x)
        }
        _ => {
            // extrapolate: straight lines through the first two and the last two points
            if n >= 2 && x < t[0][0] {
                let (a, b) = (t[0], t[1]);
                return a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0]);
            }
            if n >= 2 && x > t[n - 1][0] {
                let (a, b) = (t[n - 2], t[n - 1]);
                return b[1] + (b[1] - a[1]) * (x - b[0]) / (b[0] - a[0]);
            }
            crate::expr::lookup(t, x)
        }
    }
}

/// The array functions, each given all its elements and returning one item (0-based `k`):
/// 1 ALLOCATE AVAILABLE ITEM(k, n, request…, profile (4 per requester)…, available)
/// 2 ALLOCATE BY PRIORITY ITEM(k, n, request…, priority…, width, supply)
/// 3 VECTOR SORT ORDER ITEM(k, direction, x…)   → the 0-based index at place k of the sorted order
/// 4 VECTOR RANK ITEM(k, direction, x…)         → the 1-based rank of element k
/// 5 VECTOR REORDER ITEM(k, n, x…, order…)      → x at the (0-based) index order[k]
/// 6 VECTOR ELM MAP ITEM(start, offset, x…)     → x at start + offset (0-based)
/// 7 VECTOR SELECT ITEMS(n, selection…, value…, missing, action) → one value
/// 8 INVERT MATRIX ITEM(i, j, n, m (row by row)…) → element (i, j) of the inverse
fn vec_fn(kind: u8, v: &[f64]) -> f64 {
    let idx = |x: f64| x.round().max(0.0) as usize;
    match kind {
        1 => {
            let (k, n) = (idx(v[0]), idx(v[1]));
            if v.len() < 2 + n * 5 + 1 {
                return f64::NAN;
            }
            let req = &v[2..2 + n];
            let pp: Vec<[f64; 4]> = (0..n).map(|i| { let b = 2 + n + i * 4; [v[b], v[b + 1], v[b + 2], v[b + 3]] }).collect();
            let avail = v[2 + n + 4 * n];
            allocate_available(req, &pp, avail).get(k).copied().unwrap_or(f64::NAN)
        }
        2 => {
            let (k, n) = (idx(v[0]), idx(v[1]));
            if v.len() < 2 + 2 * n + 2 {
                return f64::NAN;
            }
            let req = &v[2..2 + n];
            let pri = &v[2 + n..2 + 2 * n];
            allocate_by_priority(req, pri, v[2 + 2 * n], v[2 + 2 * n + 1]).get(k).copied().unwrap_or(f64::NAN)
        }
        3 | 4 => {
            let (k, dir) = (idx(v[0]), v[1]);
            let x = &v[2..];
            let order = sort_order(x, dir);
            if kind == 3 {
                order.get(k).map(|i| *i as f64).unwrap_or(f64::NAN)
            } else {
                order.iter().position(|i| *i == k).map(|p| p as f64 + 1.0).unwrap_or(f64::NAN)
            }
        }
        5 => {
            let (k, n) = (idx(v[0]), idx(v[1]));
            if v.len() < 2 + n {
                return f64::NAN;
            }
            let x = &v[2..2 + n];
            let o = v.get(2 + n + k).copied().unwrap_or(f64::NAN);
            x.get(idx(o)).copied().unwrap_or(f64::NAN)
        }
        6 => {
            let i = v[0] + v[1];
            let x = &v[2..];
            if i < 0.0 || i.round() as usize >= x.len() { f64::NAN } else { x[i.round() as usize] }
        }
        7 => {
            let n = idx(v[0]);
            if v.len() < 3 + 2 * n {
                return f64::NAN;
            }
            let sel = &v[1..1 + n];
            let val = &v[1 + n..1 + 2 * n];
            let (missing, action) = (v[1 + 2 * n], v[2 + 2 * n].round() as i64);
            let picked: Vec<f64> = sel.iter().zip(val).filter(|(s, _)| **s != 0.0).map(|(s, x)| match action {
                0..=4 => s * x,
                5 => x.powf(*s),
                _ => *x,
            }).collect();
            if picked.is_empty() {
                return missing;
            }
            match action {
                0 | 6 => picked.iter().sum(),
                1 | 5 | 7 => picked.iter().product(),
                2 | 8 => picked.iter().cloned().fold(f64::INFINITY, f64::min),
                3 | 9 => picked.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                _ => picked.iter().sum::<f64>() / picked.len() as f64,
            }
        }
        9 => {
            // VECTOR LOOKUP: y values over n−1 equal steps from xmin to xmax; mode 0 lookup, 1 area, 2 backward,
            // 3 extrapolate, 4 forward, 5 invert, 6 slope, 7–9 as 1, 5, 6 with extrapolation
            let n = idx(v[0]);
            if n < 2 || v.len() < n + 5 {
                return f64::NAN;
            }
            let (ys, rest) = (&v[1..1 + n], &v[1 + n..]);
            let (x, xmin, xmax, mode) = (rest[0], rest[1], rest[2], rest[3].round() as i64);
            let step = (xmax - xmin) / (n as f64 - 1.0);
            let t: Vec<[f64; 2]> = (0..n).map(|i| [xmin + step * i as f64, ys[i]]).collect();
            match mode {
                0 => crate::expr::lookup(&t, x),
                1 => lookup_area(&t, xmin, x, false),
                2 => table_mode(&t, 2, &[x]),
                3 => table_mode(&t, 4, &[x]),
                4 => table_mode(&t, 1, &[x]),
                5 => table_mode(&t, 3, &[x]),
                6 => lookup_slope(&t, x, 0.0),
                7 => lookup_area(&t, xmin, x, true),
                8 => {
                    // invert with extrapolation
                    let mut inv: Vec<[f64; 2]> = t.iter().map(|p| [p[1], p[0]]).collect();
                    inv.sort_by(|a, b| a[0].total_cmp(&b[0]));
                    table_mode(&inv, 4, &[x])
                }
                9 => lookup_slope(&t, x, 1.0),
                _ => f64::NAN,
            }
        }
        8 => {
            let (i, j, n) = (idx(v[0]), idx(v[1]), idx(v[2]));
            if v.len() < 3 + n * n {
                return f64::NAN;
            }
            invert(&v[3..3 + n * n], n).map(|m| m[i * n + j]).unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    }
}

/// Indices that sort `x` ascending (direction > 0) or descending; ties keep their order.
fn sort_order(x: &[f64], dir: f64) -> Vec<usize> {
    let mut o: Vec<usize> = (0..x.len()).collect();
    o.sort_by(|a, b| x[*a].total_cmp(&x[*b]));
    if dir <= 0.0 {
        o.reverse();
    }
    o
}

/// Gauss-Jordan inverse with partial pivoting; None if singular.
fn invert(m: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut a = m.to_vec();
    let mut inv = vec![0.0; n * n];
    for i in 0..n {
        inv[i * n + i] = 1.0;
    }
    for c in 0..n {
        let p = (c..n).max_by(|x, y| a[x * n + c].abs().total_cmp(&a[y * n + c].abs()))?;
        if a[p * n + c].abs() < 1e-300 {
            return None;
        }
        for k in 0..n {
            a.swap(c * n + k, p * n + k);
            inv.swap(c * n + k, p * n + k);
        }
        let d = a[c * n + c];
        for k in 0..n {
            a[c * n + k] /= d;
            inv[c * n + k] /= d;
        }
        for r in 0..n {
            if r != c {
                let f = a[r * n + c];
                if f != 0.0 {
                    for k in 0..n {
                        a[r * n + k] -= f * a[c * n + k];
                        inv[r * n + k] -= f * inv[c * n + k];
                    }
                }
            }
        }
    }
    Some(inv)
}

/// How much of `q0` a requester with profile `pp` = [type, priority, width, extra] gets at priority level x
/// (Vensim's ALLOCATE AVAILABLE curves: 1 rectangular, 2 triangular, 3 normal, 4 exponential), and the range
/// of x over which it changes.
fn priority_curve(q0: f64, pp: [f64; 4], x: f64) -> f64 {
    let (pr, w) = (pp[1], pp[2]);
    match pp[0].round() as i64 {
        1 => {
            if x <= pr - w * 0.5 { q0 } else if x < pr + w * 0.5 { q0 * (1.0 - (x - pr + w * 0.5) / w) } else { 0.0 }
        }
        2 => {
            if x <= pr - w * 0.5 {
                q0
            } else if x < pr {
                q0 * (1.0 - 2.0 * (x - pr + w * 0.5).powi(2) / (w * w))
            } else if x < pr + w * 0.5 {
                2.0 * q0 * (pr + w * 0.5 - x).powi(2) / (w * w)
            } else {
                0.0
            }
        }
        3 => q0 * 0.5 * (2.0 - erfc((pr - x) / (std::f64::consts::SQRT_2 * w))),
        4 => {
            if x < pr { q0 * (1.0 - 0.5 * ((x - pr) / w).exp()) } else { q0 * 0.5 * ((pr - x) / w).exp() }
        }
        _ => f64::NAN,
    }
}

fn curve_span(pp: [f64; 4]) -> (f64, f64) {
    let (pr, w) = (pp[1], pp[2]);
    match pp[0].round() as i64 {
        3 => (pr - 8.2923611 * w, pr + 8.2923611 * w),
        4 => (pr - 36.7368005696 * w, pr + 36.7368005696 * w),
        _ => (pr - w * 0.5, pr + w * 0.5),
    }
}

/// Complementary error function to about 1e-15: a series for erf near 0, a continued fraction in the tails.
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let r = if z < 3.0 {
        // erf(z) = 2/sqrt(pi) * sum (-1)^n z^(2n+1) / (n! (2n+1))
        let mut term = z;
        let mut sum = z;
        let z2 = z * z;
        for n in 1..200 {
            term *= -z2 / n as f64;
            let add = term / (2 * n + 1) as f64;
            sum += add;
            if add.abs() < 1e-17 * sum.abs() {
                break;
            }
        }
        1.0 - 2.0 / std::f64::consts::PI.sqrt() * sum
    } else {
        // erfc(z) = exp(-z^2)/sqrt(pi) * 1/(z + 1/2/(z + 1/(z + 3/2/(z + ...)))) (modified Lentz)
        let tiny = 1e-300;
        let mut f = z;
        let mut c = z;
        let mut d = 0.0;
        for n in 1..300 {
            let a = n as f64 / 2.0;
            d = z + a * d;
            d = if d.abs() < tiny { tiny } else { d };
            c = z + a / c;
            c = if c.abs() < tiny { tiny } else { c };
            d = 1.0 / d;
            let delta = c * d;
            f *= delta;
            if (delta - 1.0).abs() < 1e-16 {
                break;
            }
        }
        (-z * z).exp() / std::f64::consts::PI.sqrt() / f
    };
    if x >= 0.0 { r } else { 2.0 - r }
}

/// ALLOCATE AVAILABLE: find the priority level at which the requesters' curves add up to what is available.
fn allocate_available(req: &[f64], pp: &[[f64; 4]], avail: f64) -> Vec<f64> {
    let total: f64 = req.iter().sum();
    if avail >= total {
        return req.to_vec();
    }
    if avail <= 0.0 {
        return vec![0.0; req.len()];
    }
    let active: Vec<usize> = (0..req.len()).filter(|i| req[*i] != 0.0).collect();
    if active.is_empty() {
        return vec![0.0; req.len()];
    }
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &i in &active {
        let (a, b) = curve_span(pp[i]);
        lo = lo.min(a);
        hi = hi.max(b);
    }
    let sum_at = |x: f64| active.iter().map(|&i| priority_curve(req[i], pp[i], x)).sum::<f64>();
    // the sum falls as x rises: bisect for sum = avail
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if sum_at(mid) > avail { lo = mid } else { hi = mid }
        if hi - lo <= 1e-13 * (1.0 + hi.abs()) {
            break;
        }
    }
    let x = 0.5 * (lo + hi);
    (0..req.len()).map(|i| if req[i] == 0.0 { 0.0 } else { priority_curve(req[i], pp[i], x) }).collect()
}

/// ALLOCATE BY PRIORITY (Vensim): the highest priority is served first; requesters whose priorities are within
/// `width` of each other share, in proportion to their requests.
fn allocate_by_priority(request: &[f64], priority: &[f64], width: f64, supply: f64) -> Vec<f64> {
    let n = request.len();
    let total: f64 = request.iter().sum();
    if supply >= total {
        return request.to_vec();
    }
    if supply <= 0.0 || width <= 0.0 {
        return vec![0.0; n];
    }
    let nz: Vec<usize> = (0..n).filter(|i| request[*i] != 0.0).collect();
    let mut sort: Vec<usize> = (0..nz.len()).collect();
    sort.sort_by(|a, b| priority[nz[*b]].total_cmp(&priority[nz[*a]]));
    let req: Vec<f64> = sort.iter().map(|s| request[nz[*s]]).collect();
    let pri: Vec<f64> = sort.iter().map(|s| priority[nz[*s]]).collect();
    let m = req.len();
    let mut out = vec![0.0; m];
    let mut dist = vec![f64::NAN; m];
    for i in 0..m.saturating_sub(1) {
        dist[i] = ((pri[i] - pri[i + 1]) / width).min(1.0) * req[i];
    }
    let mut active = vec![false; m];
    if m > 0 {
        active[0] = true;
    }
    let mut c = 0usize;
    let mut supply = supply;
    let mut guard = 0;
    while supply > 0.0 && guard < 10 * m + 10 {
        guard += 1;
        let tot: f64 = (0..m).filter(|i| active[*i]).map(|i| req[i]).sum();
        if tot <= 0.0 {
            break;
        }
        let slope: Vec<f64> = (0..m).map(|i| if active[i] { req[i] / tot } else { 0.0 }).collect();
        let top = (0..m).filter(|i| active[*i] && slope[*i] > 0.0).map(|i| (req[i] - out[i]) / slope[i]).fold(f64::INFINITY, f64::min);
        let start = if c < m && !dist[c].is_nan() && slope[c] > 0.0 { (dist[c] - out[c]) / slope[c] } else { f64::INFINITY };
        let dx = top.min(start).min(supply);
        for i in 0..m {
            out[i] += slope[i] * dx;
        }
        if (dx - start).abs() <= 1e-10 * start.abs().max(1e-16) && c + 1 < m {
            c += 1;
            active[c] = true;
        }
        if dx == top {
            for i in 0..m {
                if (out[i] - req[i]).abs() <= 1e-12 * req[i].abs().max(1.0) {
                    active[i] = false;
                }
            }
        }
        supply -= dx;
        if !active.iter().any(|a| *a) {
            break;
        }
    }
    let mut res = vec![0.0; n];
    for (p, s) in sort.iter().enumerate() {
        res[nz[*s]] = out[p];
    }
    res
}

// Date arithmetic lives in `calendar.rs` (Hinnant's algorithms; Excel's 1900 date system with its leap-year quirk).
use crate::calendar::{civil_from_days, days_from_civil};

/// 0 EXCELDATESERIAL(y, m, d) · 1 year · 2 month of year · 3 day of month · 4 day of year of a serial.
fn date_fn(kind: u8, v: &[f64]) -> f64 {
    // serial 25569 is 1970-01-01
    if kind == 0 {
        if v.len() < 3 {
            return f64::NAN;
        }
        let (y, m, d) = (v[0].floor() as i64, v[1].floor() as i64, v[2].floor() as i64);
        // months outside 1..12 roll the year
        let (y, m) = (y + (m - 1).div_euclid(12), (m - 1).rem_euclid(12) + 1);
        // Excel's serial: days since 1899-12-30 from 1900-03-01 on; one lower before that, because Excel counts a
        // 29 February 1900 that never existed
        let days = days_from_civil(y, m, 1) + (d - 1);
        let since = days + 25569;
        return if days < days_from_civil(1900, 3, 1) { since - 1 } else { since } as f64;
    }
    let serial = v.first().copied().unwrap_or(f64::NAN).floor() as i64;
    // serials 1 to 60 are 1900-01-01 to 1900-02-28 in Excel (and 60 is its fictitious 29 February): shift to days since 1899-12-30
    let serial = if (1..60).contains(&serial) { serial + 1 } else { serial };
    let (y, m, d) = civil_from_days(serial - 25569);
    match kind {
        1 => y as f64,
        2 => m as f64,
        3 => d as f64,
        _ => (days_from_civil(y, m, d) - days_from_civil(y, 1, 1) + 1) as f64,
    }
}

#[cfg(test)]
mod optional_seed_tests {
    use super::*;
    use crate::model::Var;

    fn run_of(eq: &str) -> Vec<f64> {
        let mut m = Model::default();
        m.settings.final_time = 5.0;
        let mut a = Var::new(1, "Noise", Kind::Aux, [0.0, 0.0]);
        a.equation = eq.into();
        m.vars = vec![a];
        m.next_id = 10;
        let c = compile(&m).unwrap_or_else(|e| panic!("{eq}: {:?}", e.iter().map(|p| &p.message).collect::<Vec<_>>()));
        simulate(&c, &m, "t").unwrap().get("Noise").unwrap().clone()
    }

    #[test]
    fn the_seed_is_optional_for_uniform_normal_and_exponential_as_for_the_others() {
        for (with, without) in [("RANDOM UNIFORM(0, 1, 3)", "RANDOM UNIFORM(0, 1)"), ("RANDOM NORMAL(-9, 9, 0, 1, 3)", "RANDOM NORMAL(-9, 9, 0, 1)"), ("RANDOM EXPONENTIAL(0, 99, 0, 1, 3)", "RANDOM EXPONENTIAL(0, 99, 0, 1)")] {
            let (a, b) = (run_of(without), run_of(without));
            assert_eq!(a, b, "{without} repeats");
            assert!(a.windows(2).any(|w| w[0] != w[1]), "{without} varies");
            // a model that gives its seed keeps its numbers: they are not those of the call's own stream
            assert_ne!(run_of(with), a);
        }
    }
}
