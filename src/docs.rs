//! The function reference: documentation for every function, operator and built-in
//! variable of the equation language, and the window that browses it.

use egui::{Color32, CornerRadius, FontId, RichText, Stroke};

use crate::app::App;
use crate::expr::function_info;

/// Full documentation for one entry.
pub struct Doc {
    pub name: &'static str,
    /// Shown in place of the function list's signature for non-functions.
    pub group: &'static str,
    pub signature: &'static str,
    pub summary: &'static str,
    pub params: &'static [(&'static str, &'static str)],
    pub returns: &'static str,
    pub details: &'static str,
    pub example: &'static str,
    pub tips: &'static [&'static str],
    pub see: &'static [&'static str],
    /// What "Insert" puts into the equation.
    pub insert: &'static str,
}

macro_rules! doc {
    ($name:expr, $group:expr, $sig:expr, $sum:expr, [$(($pn:expr, $pd:expr)),* $(,)?], $ret:expr, $det:expr, $ex:expr, [$($tip:expr),* $(,)?], [$($see:expr),* $(,)?], $ins:expr) => {
        Doc { name: $name, group: $group, signature: $sig, summary: $sum, params: &[$(($pn, $pd)),*], returns: $ret, details: $det, example: $ex, tips: &[$($tip),*], see: &[$($see),*], insert: $ins }
    };
}

pub const DOCS: &[Doc] = &[
    // ---------------------------------------------------------------- logic
    doc!("IF THEN ELSE", "Logic", "IF THEN ELSE(condition, a, b)", "Chooses between two values depending on a condition.",
        [("condition", "Anything that is true (not zero) or false (zero), e.g. Stock > 100"), ("a", "Value when the condition is true"), ("b", "Value when the condition is false")],
        "a when the condition holds, otherwise b.",
        "The condition is checked again at every time step, so the result can switch back and forth during a run. Combine conditions with :AND: and :OR:.",
        "IF THEN ELSE(Inventory < Reorder Point, Order Size, 0)",
        ["Comparisons give 1 (true) or 0 (false), so you can also multiply by them: (Time > 10) * 5.", "Avoid switching on exact equality of decimal numbers; use < or > instead."],
        ["MIN", "MAX", "STEP"], "IF THEN ELSE(condition, a, b)"),
    // ---------------------------------------------------------------- math
    doc!("MIN", "Math", "MIN(a, b, …)", "The smallest of two or more values.",
        [("a, b, …", "Two to sixteen values or expressions")], "The smallest value.",
        "Useful to cap a flow so it can never be larger than what is available, e.g. shipments limited by inventory.",
        "MIN(Desired Shipments, Inventory / Minimum Delivery Time)",
        ["MIN(x, 0) keeps only the negative part of x."], ["MAX", "IF THEN ELSE"], "MIN(a, b)"),
    doc!("MAX", "Math", "MAX(a, b, …)", "The largest of two or more values.",
        [("a, b, …", "Two to sixteen values or expressions")], "The largest value.",
        "Often used to keep a value from going below a floor, such as zero.",
        "MAX(0, Desired Production - Capacity)",
        ["MAX(x, 0) keeps a flow from becoming negative."], ["MIN", "ABS"], "MAX(a, b)"),
    doc!("ABS", "Math", "ABS(x)", "The size of a number, without its sign.",
        [("x", "Any value")], "x if x is positive, −x if it is negative.", "",
        "ABS(Target - Actual)", [], ["MAX", "MIN"], "ABS(x)"),
    doc!("SQRT", "Math", "SQRT(x)", "Square root.",
        [("x", "A value of zero or more")], "The number that multiplied by itself gives x.",
        "Negative inputs give an invalid result (shown as –).", "SQRT(Area)", [], ["POWER", "EXP"], "SQRT(x)"),
    doc!("EXP", "Math", "EXP(x)", "e (≈2.718) raised to the power x.",
        [("x", "The exponent")], "eˣ.",
        "Natural growth: a quantity growing at a constant fractional rate r follows Initial × EXP(r × Time).",
        "Initial Value * EXP(Growth Rate * Time)", [], ["LN", "POWER"], "EXP(x)"),
    doc!("LN", "Math", "LN(x)", "Natural logarithm (the inverse of EXP).",
        [("x", "A value greater than zero")], "The power to which e must be raised to give x.", "",
        "LN(2) / Growth Rate", ["LN(2) / rate is the doubling time of exponential growth."], ["EXP", "LOG"], "LN(x)"),
    doc!("LOG", "Math", "LOG(x, base)", "Logarithm of x in any base.",
        [("x", "A value greater than zero"), ("base", "The base, e.g. 10 or 2")], "The power to which base must be raised to give x.", "",
        "LOG(Population, 10)", [], ["LN", "POWER"], "LOG(x, base)"),
    doc!("POWER", "Math", "POWER(x, y)", "x raised to the power y.",
        [("x", "The base"), ("y", "The exponent")], "xʸ — the same as writing x ^ y.", "",
        "POWER(1 + Interest Rate, Years)", [], ["EXP", "SQRT"], "POWER(x, y)"),
    doc!("INTEGER", "Math", "INTEGER(x)", "The whole-number part of x.",
        [("x", "Any value")], "x with its fraction removed, towards zero (2.7 → 2, −2.7 → −2).", "",
        "INTEGER(Time / 7)", ["Use ROUND for the nearest whole number instead."], ["ROUND", "MODULO"], "INTEGER(x)"),
    doc!("ROUND", "Math", "ROUND(x)", "x rounded to the nearest whole number.",
        [("x", "Any value")], "The nearest whole number; halves round away from zero.", "",
        "ROUND(Staff Needed)", [], ["INTEGER"], "ROUND(x)"),
    doc!("MODULO", "Math", "MODULO(x, y)", "The remainder after dividing x by y.",
        [("x", "The number divided"), ("y", "The divisor")], "A value from 0 up to (not including) y.",
        "Handy for repeating patterns such as days of the week or seasons.",
        "IF THEN ELSE(MODULO(Time, 7) >= 5, Weekend Demand, Weekday Demand)", [], ["INTEGER", "PULSE TRAIN"], "MODULO(x, y)"),
    doc!("SIN", "Math", "SIN(x)", "Sine of an angle in radians.",
        [("x", "Angle in radians (a full circle is 2π ≈ 6.283)")], "A value between −1 and 1.",
        "Good for seasonal cycles: one full wave every Period time units.",
        "Average Demand * (1 + 0.2 * SIN(2 * 3.14159 * Time / 12))", [], ["COS", "TAN"], "SIN(x)"),
    doc!("COS", "Math", "COS(x)", "Cosine of an angle in radians.", [("x", "Angle in radians")], "A value between −1 and 1.", "",
        "COS(2 * 3.14159 * Time / Period)", [], ["SIN", "TAN"], "COS(x)"),
    doc!("TAN", "Math", "TAN(x)", "Tangent of an angle in radians.", [("x", "Angle in radians")], "sin(x) / cos(x).", "", "TAN(Slope Angle)", [], ["ARCTAN"], "TAN(x)"),
    doc!("ARCTAN", "Math", "ARCTAN(x)", "The angle whose tangent is x.", [("x", "Any value")], "An angle in radians between −π/2 and π/2.", "", "ARCTAN(Rise / Run)", [], ["TAN"], "ARCTAN(x)"),
    doc!("XIDZ", "Math", "XIDZ(a, b, x)", "a divided by b, or x when b is zero.",
        [("a", "The numerator"), ("b", "The divisor"), ("x", "What to use instead when b is zero")], "a / b, or x if b = 0.",
        "Protects a ratio from dividing by zero, for instance when a stock empties.",
        "XIDZ(Orders, Capacity, 1)", [], ["ZIDZ"], "XIDZ(a, b, x)"),
    doc!("ZIDZ", "Math", "ZIDZ(a, b)", "a divided by b, or 0 when b is zero.",
        [("a", "The numerator"), ("b", "The divisor")], "a / b, or 0 if b = 0.", "", "ZIDZ(Infected, Total Population)", [], ["XIDZ"], "ZIDZ(a, b)"),
    doc!("SINH", "Math", "SINH(x)", "Hyperbolic sine.",
        [("x", "A value or expression")], "The result.", "", "SINH(Growth Rate * Time)", [], ["CEILING"], "SINH(x)"),
    doc!("COSH", "Math", "COSH(x)", "Hyperbolic cosine.",
        [("x", "A value or expression")], "The result.", "", "COSH(x)", [], ["CEILING"], "COSH(x)"),
    doc!("TANH", "Math", "TANH(x)", "Hyperbolic tangent: an S-shaped curve from −1 to 1, handy as a smooth switch.",
        [("x", "A value or expression")], "The result.", "", "TANH((Stock - Target) / Range)", [], ["CEILING"], "TANH(x)"),
    doc!("ARCSIN", "Math", "ARCSIN(x)", "The angle (radians) whose sine is x; x must be between −1 and 1.",
        [("x", "A value or expression")], "The result.", "", "ARCSIN(Ratio)", [], ["CEILING"], "ARCSIN(x)"),
    doc!("ARCCOS", "Math", "ARCCOS(x)", "The angle (radians) whose cosine is x; x must be between −1 and 1.",
        [("x", "A value or expression")], "The result.", "", "ARCCOS(Ratio)", [], ["CEILING"], "ARCCOS(x)"),
    doc!("ARCTAN2", "Math", "ARCTAN2(y, x)", "The angle (radians) of the point (x, y), in the right quadrant.",
        [("y", "A value or expression"), ("x", "A value or expression")], "The result.", "", "ARCTAN2(Rise, Run)", [], ["CEILING"], "ARCTAN2(y, x)"),
    doc!("FLOOR", "Math", "FLOOR(x)", "Rounds down to the whole number below.",
        [("x", "A value or expression")], "The result.", "", "FLOOR(Days / 7)", [], ["CEILING"], "FLOOR(x)"),
    doc!("SIGN", "Math", "SIGN(x)", "−1, 0 or 1 according to the sign of x.",
        [("x", "A value or expression")], "The result.", "", "SIGN(Gap)", [], ["CEILING"], "SIGN(x)"),
    doc!("LOG10", "Math", "LOG10(x)", "Logarithm to base 10.",
        [("x", "A value or expression")], "The result.", "", "LOG10(Population)", [], ["CEILING"], "LOG10(x)"),
    doc!("LOG2", "Math", "LOG2(x)", "Logarithm to base 2.",
        [("x", "A value or expression")], "The result.", "", "LOG2(Size)", [], ["CEILING"], "LOG2(x)"),
    doc!("CBRT", "Math", "CBRT(x)", "Cube root.",
        [("x", "A value or expression")], "The result.", "", "CBRT(Volume)", [], ["CEILING"], "CBRT(x)"),
    doc!("HYPOT", "Math", "HYPOT(a, b)", "√(a² + b²): the length of the hypotenuse.",
        [("a", "A value or expression"), ("b", "A value or expression")], "The result.", "", "HYPOT(Dx, Dy)", [], ["CEILING"], "HYPOT(a, b)"),
    doc!("RADIANS", "Math", "RADIANS(degrees)", "Converts degrees to radians.",
        [("degrees", "A value or expression")], "The result.", "", "SIN(RADIANS(Angle))", [], ["CEILING"], "RADIANS(degrees)"),
    doc!("DEGREES", "Math", "DEGREES(radians)", "Converts radians to degrees.",
        [("radians", "A value or expression")], "The result.", "", "DEGREES(ARCTAN2(y, x))", [], ["CEILING"], "DEGREES(radians)"),
    doc!("CLAMP", "Math", "CLAMP(x, low, high)", "Keeps x between a lower and an upper limit.",
        [("x", "A value or expression"), ("low", "A value or expression"), ("high", "A value or expression")], "The result.", "", "CLAMP(Demand, 0, Capacity)", [], ["CEILING"], "CLAMP(x, low, high)"),
    doc!("MEAN", "Math", "MEAN(a, b, …)", "The average of two or more values.",
        [("a", "A value or expression"), ("b", "A value or expression"), ("…", "A value or expression")], "The result.", "", "MEAN(Plan A, Plan B, Plan C)", [], ["CEILING"], "MEAN(a, b, …)"),
    doc!("MEDIAN", "Math", "MEDIAN(a, b, …)", "The middle value of two or more values.",
        [("a", "A value or expression"), ("b", "A value or expression"), ("…", "A value or expression")], "The result.", "", "MEDIAN(Quote 1, Quote 2, Quote 3)", [], ["CEILING"], "MEDIAN(a, b, …)"),
    doc!("PI", "Math", "PI()", "The number π (3.14159…). Write it with brackets: PI().",
        [], "The result.", "", "2 * PI() * Radius", [], ["CEILING"], "PI()"),
    doc!("ERF", "Math", "ERF(x)", "The error function (accuracy about 1e-7); NormalCDF(x) = (1 + ERF(x / SQRT(2))) / 2.",
        [("x", "A value or expression")], "The result.", "", "(1 + ERF(Z / SQRT(2))) / 2", [], ["CEILING"], "ERF(x)"),
    doc!("GAMMA", "Math", "GAMMA(x)", "The gamma function (a continuous factorial: GAMMA(n + 1) = n!).",
        [("x", "A value or expression")], "The result.", "", "GAMMA(Shape)", [], ["CEILING"], "GAMMA(x)"),
    doc!("FACTORIAL", "Math", "FACTORIAL(n)", "n! for a whole number n (up to 170).",
        [("n", "A value or expression")], "The result.", "", "FACTORIAL(Choices)", [], ["CEILING"], "FACTORIAL(n)"),
    doc!("SINWAVE", "Test inputs", "SINWAVE(amplitude, period, start)", "A repeating sine wave.",
        [("amplitude", "Height of the wave"), ("period", "Time for one full cycle"), ("start", "Optional: where the wave is at zero and rising (default: the start of the run)")], "amplitude × sin(2π (TIME − start) / period).",
        "Seasonal demand, daily cycles.", "Base * (1 + SINWAVE(0.2, 365))", [], ["COSWAVE", "SIN"], "SINWAVE(amplitude, period)"),
    doc!("COSWAVE", "Test inputs", "COSWAVE(amplitude, period, start)", "A repeating cosine wave.",
        [("amplitude", "Height of the wave"), ("period", "Time for one full cycle"), ("start", "Optional: where the wave peaks (default: the start of the run)")], "amplitude × cos(2π (TIME − start) / period).",
        "Peaks at the start time unless you give another.", "Base * (1 + COSWAVE(0.2, 12, 6))", [], ["SINWAVE", "COS"], "COSWAVE(amplitude, period)"),
    doc!("GRAPH", "Lookups", "GRAPH(x, x0, dx, y1, y2, …)", "A curve written inline from evenly spaced values.",
        [("x", "The input"), ("x0", "The x of the first value"), ("dx", "The distance between values"), ("y1, y2, …", "The values")], "Linear between values; the first value below x0 and the last beyond the end.",
        "Handy for a short table without making a lookup variable. From Powersim.", "GRAPH(Price, 0, 10, 100, 80, 55, 30, 10)", [], ["GRAPHSTEP", "WITH LOOKUP"], "GRAPH(x, x0, dx, y1, y2, y3)"),
    doc!("GRAPHSTEP", "Lookups", "GRAPHSTEP(x, x0, dx, y1, y2, …)", "Like GRAPH, but stepwise.",
        [("x", "The input"), ("x0", "The x of the first value"), ("dx", "The distance between values"), ("y1, y2, …", "The values")], "y1 for x in [x0, x0+dx), y2 for the next stretch, and so on.",
        "Use for tariffs, tax bands, price breaks.", "GRAPHSTEP(Volume, 0, 100, 5, 4.5, 4, 3.5)", [], ["GRAPH"], "GRAPHSTEP(x, x0, dx, y1, y2, y3)"),
    doc!("DELAYINF", "Delays", "DELAYINF(input, delay, order, [initial])", "Information delay: the input, smoothed by a chain of stages.",
        [("input", "What is delayed"), ("delay", "Average delay time"), ("order", "How many stages (a whole number, 1–20; default 1). More stages = a sharper delay"), ("initial", "Optional: the value every stage starts at. Without it the output starts at the input")], "The output of the last stage; each stage has delay ÷ order.",
        "For perceptions and decisions: no material is conserved. DELAYINF(x, d, 3) equals SMOOTH3(x, d); with an initial value it is Vensim's SMOOTH N(x, d, initial, n).", "DELAYINF(Orders, 5, 3)", [], ["DELAYMTR", "SMOOTH"], "DELAYINF(input, delay, 3)"),
    doc!("DELAYMTR", "Delays", "DELAYMTR(input, delay, order, [initial])", "Material delay: what goes in comes out later, nothing is lost.",
        [("input", "The inflow"), ("delay", "Average delay time"), ("order", "How many stages (a whole number, 1–20; default 1)"), ("initial", "Optional: the outflow at the start; every stage starts full enough to give it. Without it the outflow starts at the input")], "The outflow of the last stage; each stage has delay ÷ order.",
        "For things in transit: shipments, construction. DELAYMTR(x, d, 3) equals DELAY3(x, d); with an initial value it is Vensim's DELAY N(x, d, initial, n).", "DELAYMTR(Shipments, 6, 3)", [], ["DELAYINF", "DELAY3"], "DELAYMTR(input, delay, 3)"),
    doc!("CEILING", "Math", "CEILING(x)", "Rounds up to the next whole number.",
        [("x", "Any value")], "The smallest whole number that is not below x.",
        "Useful when things come in whole units: trucks, crews, batches.",
        "CEILING(Required MB * 1000 / Barrels per truck)", ["CEILING(2.1) is 3 and CEILING(3) is 3."], ["INTEGER", "ROUND"], "CEILING(x)"),
    doc!("TOTAL", "Sets", "TOTAL(name) or TOTAL(name, group)", "Adds up a variable over all the items of its set.",
        [("name", "A variable that is per a set (for example per Tank)"), ("group", "Optional: only the items that belong to this group, e.g. a region name")],
        "One number: the sum over the items.",
        "In a formula that is itself per Region, TOTAL(Stock) adds up only the tanks of that region. Anywhere else it adds up every item.",
        "TOTAL(Stock, COA)", ["TOTAL(Stock) in a per-Region variable gives each region its own tanks.", "Stock[Hail 2 - Crude] picks one item."], ["AVERAGE"], "TOTAL(name, group)"),
    doc!("AVERAGE", "Sets", "AVERAGE(name) or AVERAGE(name, group)", "The average of a variable over the items of its set.",
        [("name", "A variable that is per a set"), ("group", "Optional: only the items that belong to this group")],
        "One number: the mean over the items.", "Works like TOTAL, but divides by the number of items.",
        "AVERAGE(Stock Percent, WOA)", [], ["TOTAL"], "AVERAGE(name, group)"),
    // ---------------------------------------------------------------- test inputs
    doc!("STEP", "Test inputs", "STEP(height, start)", "Jumps from 0 to a new level at a given time and stays there.",
        [("height", "The size of the jump"), ("start", "When the jump happens")], "0 before start, height from start onwards.",
        "The classic test input: see how the system responds to a sudden, permanent change.",
        "Base Demand + STEP(20, 10)", ["Add it to a constant to model a policy change from a certain date."], ["PULSE", "RAMP"], "STEP(height, start)"),
    doc!("PULSE", "Test inputs", "PULSE(start, width)", "A one-off burst: 1 for a while, otherwise 0.",
        [("start", "When the pulse begins"), ("width", "How long it lasts (at least one time step)")], "1 from start until start + width, otherwise 0.",
        "Multiply by an amount to add a one-time shock, such as a single large order.",
        "Normal Orders + 500 * PULSE(12, 1)", [], ["PULSE TRAIN", "STEP"], "PULSE(start, width)"),
    doc!("PULSE TRAIN", "Test inputs", "PULSE TRAIN(start, width, interval, end)", "Repeating pulses at regular intervals.",
        [("start", "When the first pulse begins"), ("width", "How long each pulse lasts"), ("interval", "Time between the starts of pulses"), ("end", "No pulses after this time")],
        "1 during each pulse, otherwise 0.", "", "Shipment Size * PULSE TRAIN(0, 1, 4, 100)", [], ["PULSE", "MODULO"], "PULSE TRAIN(start, width, interval, end)"),
    doc!("RAMP", "Test inputs", "RAMP(slope, start, end)", "Rises in a straight line between two times.",
        [("slope", "Increase per time unit"), ("start", "When it starts rising"), ("end", "When it stops rising and levels off")],
        "0 before start, slope × (time − start) while rising, then flat.", "", "Base Price + RAMP(0.5, 10, 30)", [], ["STEP"], "RAMP(slope, start, end)"),
    doc!("RANDOM UNIFORM", "Test inputs", "RANDOM UNIFORM(min, max, seed)", "A new random number every time step, evenly spread between min and max. The seed picks the stream: the same seed repeats, another seed gives other numbers.",
        [("min", "Lowest possible value"), ("max", "Highest possible value"), ("seed", "Kept for compatibility; every run uses the same sequence so results are repeatable")],
        "A random value in [min, max).", "Use it to add noise to demand or test how robust a policy is.",
        "Average Demand * RANDOM UNIFORM(0.9, 1.1, 1)", ["Smoothing noisy inputs with SMOOTH makes them more realistic."], ["RANDOM NORMAL", "SMOOTH"], "RANDOM UNIFORM(min, max, seed)"),
    doc!("RANDOM NORMAL", "Test inputs", "RANDOM NORMAL(min, max, mean, sd, seed)", "Bell-curve random numbers, kept between min and max. The seed picks the stream: the same seed repeats, another seed gives other numbers.",
        [("min", "Lowest value allowed"), ("max", "Highest value allowed"), ("mean", "Average value"), ("sd", "Standard deviation (spread)"), ("seed", "Kept for compatibility; runs are repeatable")],
        "A normally distributed random value, clipped to [min, max].", "", "RANDOM NORMAL(0, 200, 100, 15, 1)", [], ["RANDOM UNIFORM"], "RANDOM NORMAL(min, max, mean, sd, seed)"),
    // ---------------------------------------------------------------- delays
    doc!("SMOOTH", "Delays", "SMOOTH(input, delay time)", "An average that catches up with its input gradually (first-order exponential smoothing).",
        [("input", "The value being averaged"), ("delay time", "How long it takes to adjust, on average")],
        "The smoothed value; it starts equal to the input.",
        "Models perceptions and expectations: people notice change only gradually. After one delay time it has closed about 63% of a gap.",
        "SMOOTH(Actual Sales, 3)", ["Use SMOOTHI to start from a different value.", "SMOOTH3 responds with an S-shape rather than an immediate start."], ["SMOOTHI", "SMOOTH3", "DELAY1"], "SMOOTH(input, delay time)"),
    doc!("SMOOTHI", "Delays", "SMOOTHI(input, delay time, initial)", "SMOOTH with a chosen starting value.",
        [("input", "The value being averaged"), ("delay time", "Adjustment time"), ("initial", "Value at the start of the run")], "The smoothed value.", "",
        "SMOOTHI(Actual Sales, 3, 100)", [], ["SMOOTH"], "SMOOTHI(input, delay time, initial)"),
    doc!("SMOOTH3", "Delays", "SMOOTH3(input, delay time)", "Third-order smoothing: a slower start, then a quicker catch-up.",
        [("input", "The value being averaged"), ("delay time", "Total adjustment time")], "The smoothed value; it starts equal to the input.",
        "Built from three SMOOTHs in a row, each a third of the delay time.", "SMOOTH3(Price, 6)", [], ["SMOOTH", "SMOOTH3I"], "SMOOTH3(input, delay time)"),
    doc!("SMOOTH3I", "Delays", "SMOOTH3I(input, delay time, initial)", "SMOOTH3 with a chosen starting value.",
        [("input", "The value being averaged"), ("delay time", "Total adjustment time"), ("initial", "Value at the start")], "The smoothed value.", "",
        "SMOOTH3I(Price, 6, 10)", [], ["SMOOTH3"], "SMOOTH3I(input, delay time, initial)"),
    doc!("DELAY1", "Delays", "DELAY1(input, delay time)", "A first-order material delay: what goes in comes out later, spread out over time.",
        [("input", "The flow going into the delay"), ("delay time", "Average time spent inside")],
        "The outflow of the delay; it starts equal to the input.",
        "Unlike SMOOTH, nothing is lost: everything that enters eventually leaves. Typical for shipping, construction or ageing.",
        "DELAY1(Orders Placed, Lead Time)", ["To see how much is inside the delay, model it with a stock and flows instead."], ["DELAY3", "DELAY FIXED", "SMOOTH"], "DELAY1(input, delay time)"),
    doc!("DELAY1I", "Delays", "DELAY1I(input, delay time, initial)", "DELAY1 with a chosen starting outflow.",
        [("input", "The inflow"), ("delay time", "Average time inside"), ("initial", "Outflow at the start")], "The outflow.", "",
        "DELAY1I(Orders Placed, Lead Time, 50)", [], ["DELAY1"], "DELAY1I(input, delay time, initial)"),
    doc!("DELAY3", "Delays", "DELAY3(input, delay time)", "A third-order material delay: outflow rises in an S-shape after the input changes.",
        [("input", "The inflow"), ("delay time", "Average time inside")], "The outflow; it starts equal to the input.",
        "More realistic than DELAY1 when most items take roughly the delay time, like a pipeline with some spread.",
        "DELAY3(Construction Starts, 18)", [], ["DELAY1", "DELAY FIXED"], "DELAY3(input, delay time)"),
    doc!("DELAY3I", "Delays", "DELAY3I(input, delay time, initial)", "DELAY3 with a chosen starting outflow.",
        [("input", "The inflow"), ("delay time", "Average time inside"), ("initial", "Outflow at the start")], "The outflow.", "",
        "DELAY3I(Construction Starts, 18, 10)", [], ["DELAY3"], "DELAY3I(input, delay time, initial)"),
    doc!("DELAY FIXED", "Delays", "DELAY FIXED(input, delay time, initial)", "An exact pipeline: the output is the input from exactly delay time ago.",
        [("input", "The value fed in"), ("delay time", "How far back to look"), ("initial", "Output before anything has made it through")],
        "The input from delay time ago, or initial early in the run.",
        "Use for processes with a fixed duration, such as a 9-month gestation or a fixed-term contract.",
        "DELAY FIXED(Hires, 3, 0)", [], ["DELAY3", "DELAY1"], "DELAY FIXED(input, delay time, initial)"),
    doc!("TREND", "Delays", "TREND(input, average time, initial trend)", "The fractional rate at which a value has been changing.",
        [("input", "The value to watch"), ("average time", "Over how long the trend is judged"), ("initial trend", "Trend assumed at the start (fraction per time unit)")],
        "Fractional change per time unit, e.g. 0.05 = growing 5% per period.",
        "Models how people extrapolate growth from recent experience.", "TREND(Sales, 6, 0)", [], ["SMOOTH"], "TREND(input, average time, initial trend)"),
    doc!("ALLOCATE AVAILABLE ITEM", "Sets", "ALLOCATE AVAILABLE ITEM(k, n, request…, profile…, available)", "One requester's share of what is available, when supply is short.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "One requester's share of what is available, when supply is short.",
        "Vensim's ALLOCATE AVAILABLE: each requester has a priority curve (rectangular, triangular, normal or exponential); the priority level is found where the curves add up to what is available. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "ALLOCATE AVAILABLE(Demand[region], Profile[region, ptype], Supply)", [], ["TOTAL"], "ALLOCATE AVAILABLE ITEM(k, n, request…, profile…, available)"),
    doc!("ALLOCATE BY PRIORITY ITEM", "Sets", "ALLOCATE BY PRIORITY ITEM(k, n, request…, priority…, width, supply)", "One requester's share when the highest priority is served first.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "One requester's share when the highest priority is served first.",
        "Vensim's ALLOCATE BY PRIORITY: requesters whose priorities are within the width of each other share in proportion to their requests. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "ALLOCATE BY PRIORITY(Demand[region], Priority[region], ELMCOUNT(region), 1, Supply)", [], ["TOTAL"], "ALLOCATE BY PRIORITY ITEM(k, n, request…, priority…, width, supply)"),
    doc!("VECTOR SORT ORDER ITEM", "Sets", "VECTOR SORT ORDER ITEM(k, direction, x…)", "The index (from 0) of the element at place k once the values are sorted.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "The index (from 0) of the element at place k once the values are sorted.",
        "Vensim's VECTOR SORT ORDER; direction 1 sorts up, 0 or less down. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "VECTOR SORT ORDER(Sales[store], 1)", [], ["TOTAL"], "VECTOR SORT ORDER ITEM(k, direction, x…)"),
    doc!("VECTOR RANK ITEM", "Sets", "VECTOR RANK ITEM(k, direction, x…)", "The rank (from 1) of element k.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "The rank (from 1) of element k.",
        "Vensim's VECTOR RANK. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "VECTOR RANK(Sales[store], 1)", [], ["TOTAL"], "VECTOR RANK ITEM(k, direction, x…)"),
    doc!("VECTOR REORDER ITEM", "Sets", "VECTOR REORDER ITEM(k, n, x…, order…)", "x at the index the order gives for place k.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "x at the index the order gives for place k.",
        "Vensim's VECTOR REORDER, usually with an order from VECTOR SORT ORDER. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "VECTOR REORDER(Sales[store], Order[store])", [], ["TOTAL"], "VECTOR REORDER ITEM(k, n, x…, order…)"),
    doc!("VECTOR ELM MAP ITEM", "Sets", "VECTOR ELM MAP ITEM(start, offset, x…)", "The element offset places after start.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "The element offset places after start.",
        "Vensim's VECTOR ELM MAP. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "VECTOR ELM MAP(Price[first], Shift)", [], ["TOTAL"], "VECTOR ELM MAP ITEM(start, offset, x…)"),
    doc!("VECTOR SELECT ITEMS", "Sets", "VECTOR SELECT ITEMS(n, selection…, value…, missing, action)", "Sums, multiplies, or takes the minimum, maximum or mean of the selected values.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "Sums, multiplies, or takes the minimum, maximum or mean of the selected values.",
        "Vensim's VECTOR SELECT; action 0 sum, 1 product, 2 min, 3 max, 4 mean (5–10 as Vensim). Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "VECTOR SELECT(Chosen[store!], Sales[store!], 0, 0, 0)", [], ["TOTAL"], "VECTOR SELECT ITEMS(n, selection…, value…, missing, action)"),
    doc!("INVERT MATRIX ITEM", "Sets", "INVERT MATRIX ITEM(i, j, n, m…)", "One element of the inverse of a square matrix.",
        [("…", "Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each")],
        "One element of the inverse of a square matrix.",
        "Vensim's INVERT MATRIX. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.",
        "INVERT MATRIX(A[row, col], 3)", [], ["TOTAL"], "INVERT MATRIX ITEM(i, j, n, m…)"),
    doc!("LOOKUP SLOPE", "Lookups", "LOOKUP SLOPE(lookup, x, m)", "The slope of a lookup at x.",
        [("lookup", "The name of a lookup"), ("x", "Where to read"), ("m", "Outside the points: 0 gives 0, 1 the nearest segment's slope, −1 gives :NA:")], "Change in y over change in x for the segment; at a point, the mean of the slopes either side.",
        "How steeply an effect changes, for example to see where a curve flattens.", "LOOKUP SLOPE(Effect Curve, Fraction, 0)", [], ["LOOKUP AREA"], "LOOKUP SLOPE(lookup, x, m)"),
    doc!("LOOKUP AREA", "Lookups", "LOOKUP AREA(lookup, start, end)", "The area under a lookup between two inputs.",
        [("lookup", "The name of a lookup"), ("start", "Where to start"), ("end", "Where to stop")], "The area between the curve and the x axis, flat beyond the end points.",
        "Normalising an effort profile so that its total over the whole range is a known amount.", "LOOKUP AREA(Effort Profile, 0, Fraction Done)", [], ["LOOKUP SLOPE"], "LOOKUP AREA(lookup, start, end)"),
    doc!("GET DATA AT TIME", "Lookups", "GET DATA AT TIME(data, time)", "The data value at exactly that time.",
        [("data", "The name of a data variable (a lookup over Time)"), ("time", "When")], "The value, or :NA: when there is no point at that time.",
        "Reading a series at one chosen moment.", "GET DATA AT TIME(Sales Data, 20)", [], ["GET DATA BETWEEN TIMES"], "GET DATA AT TIME(data, time)"),
    doc!("GET DATA BETWEEN TIMES", "Lookups", "GET DATA BETWEEN TIMES(data, time, mode)", "The data value at a time that may fall between points.",
        [("data", "The name of a data variable"), ("time", "When"), ("mode", "0 interpolates, 1 takes the later point, −1 the earlier")], "The value, or :NA: before the first or after the last point.",
        "Reading a series at times that are not on its grid.", "GET DATA BETWEEN TIMES(Sales Data, Time + 1, 0)", [], ["GET DATA AT TIME"], "GET DATA BETWEEN TIMES(data, time, mode)"),
    doc!("GET DATA FIRST TIME", "Lookups", "GET DATA FIRST TIME(data)", "The time of the first data point.",
        [("data", "The name of a data variable")], "The first time in the series.",
        "Finding where a series starts.", "GET DATA FIRST TIME(Sales Data)", [], ["GET DATA LAST TIME"], "GET DATA FIRST TIME(data)"),
    doc!("GET DATA LAST TIME", "Lookups", "GET DATA LAST TIME(data)", "The time of the last data point.",
        [("data", "The name of a data variable")], "The last time in the series.",
        "Finding where a series ends.", "GET DATA LAST TIME(Sales Data)", [], ["GET DATA FIRST TIME"], "GET DATA LAST TIME(data)"),
    doc!("GET DATA MAX", "Lookups", "GET DATA MAX(data, start, end)", "The largest data value between two times.",
        [("data", "The name of a data variable"), ("start", "From"), ("end", "To")], "The maximum, or :NA: when there are no points in the range.",
        "Summaries of a series, usually inside INITIAL.", "INITIAL(GET DATA MAX(Sales Data, INITIAL TIME, FINAL TIME))", [], ["GET DATA MIN", "GET DATA MEAN"], "GET DATA MAX(data, start, end)"),
    doc!("GET DATA MIN", "Lookups", "GET DATA MIN(data, start, end)", "The smallest data value between two times.",
        [("data", "The name of a data variable"), ("start", "From"), ("end", "To")], "The minimum, or :NA: when there are no points in the range.",
        "Summaries of a series.", "GET DATA MIN(Sales Data, 0, 12)", [], ["GET DATA MAX"], "GET DATA MIN(data, start, end)"),
    doc!("GET DATA MEAN", "Lookups", "GET DATA MEAN(data, start, end)", "The mean of the data values between two times.",
        [("data", "The name of a data variable"), ("start", "From"), ("end", "To")], "The average of the points in the range.",
        "Summaries of a series.", "GET DATA MEAN(Sales Data, 0, 12)", [], ["GET DATA MEDIAN"], "GET DATA MEAN(data, start, end)"),
    doc!("GET DATA MEDIAN", "Lookups", "GET DATA MEDIAN(data, start, end)", "The median of the data values between two times.",
        [("data", "The name of a data variable"), ("start", "From"), ("end", "To")], "The middle value of the points in the range.",
        "Summaries of a series that ignore outliers.", "GET DATA MEDIAN(Sales Data, 0, 12)", [], ["GET DATA MEAN"], "GET DATA MEDIAN(data, start, end)"),
    doc!("GET DATA STDV", "Lookups", "GET DATA STDV(data, start, end)", "The standard deviation of the data values between two times.",
        [("data", "The name of a data variable"), ("start", "From"), ("end", "To")], "The sample standard deviation of the points in the range.",
        "How much a series varies.", "GET DATA STDV(Sales Data, 0, 12)", [], ["GET DATA MEAN"], "GET DATA STDV(data, start, end)"),
    doc!("GET DATA TOTAL", "Lookups", "GET DATA TOTAL(data, start, end)", "The sum of the data values between two times.",
        [("data", "The name of a data variable"), ("start", "From"), ("end", "To")], "The sum of the points in the range.",
        "Totals of a series.", "GET DATA TOTAL(Sales Data, 0, 12)", [], ["GET DATA MEAN"], "GET DATA TOTAL(data, start, end)"),
    doc!("NPVE", "Delays", "NPVE(stream, discount rate, initial value, factor)", "Net present value with a discrete period rate, valued at the end of the period.",
        [("stream", "Money per time unit"), ("discount rate", "Per period"), ("initial value", "The value at the start"), ("factor", "Multiplies the result")], "The discounted sum, as Excel computes it.",
        "Comparing with a spreadsheet. For time steps below 1, NPV is the better choice.", "NPVE(Cash Flow, 0.01, 0, 1)", [], ["NPV"], "NPVE(stream, discount rate, initial value, factor)"),
    doc!("DERIVN", "Delays", "DERIVN(x, order)", "The nth time derivative of x, from its past values.",
        [("x", "The value to differentiate"), ("order", "1 for the rate of change, 2 for its rate of change, …")], "The nth backward difference divided by the time step to the nth; 0 until there is enough history.",
        "Speed and acceleration of a variable.", "DERIVN(Position, 1)", [], ["TREND"], "DERIVN(x, order)"),
    doc!("EXCELDATESERIAL", "Math", "EXCELDATESERIAL(year, month, day)", "The Excel serial number of a date.",
        [("year", "The year"), ("month", "1–12"), ("day", "1–31")], "Days counted the way Excel does: 39448 is 1 January 2008.",
        "Time spans between dates, or adding a span to a date.", "EXCELDATESERIAL(2008, 1, 1)", [], ["EXCELDATESERIAL YEAR"], "EXCELDATESERIAL(year, month, day)"),
    doc!("EXCELDATESERIAL YEAR", "Math", "EXCELDATESERIAL YEAR(serial)", "The year of an Excel serial date.",
        [("serial", "An Excel date number")], "The year.",
        "Taking a date apart.", "EXCELDATESERIAL YEAR(39448)", [], ["EXCELDATESERIAL"], "EXCELDATESERIAL YEAR(serial)"),
    doc!("EXCELDATESERIAL MONTH OF YEAR", "Math", "EXCELDATESERIAL MONTH OF YEAR(serial)", "The month of an Excel serial date.",
        [("serial", "An Excel date number")], "1 to 12.",
        "Taking a date apart.", "EXCELDATESERIAL MONTH OF YEAR(39448)", [], ["EXCELDATESERIAL"], "EXCELDATESERIAL MONTH OF YEAR(serial)"),
    doc!("EXCELDATESERIAL DAY OF MONTH", "Math", "EXCELDATESERIAL DAY OF MONTH(serial)", "The day of the month of an Excel serial date.",
        [("serial", "An Excel date number")], "1 to 31.",
        "Taking a date apart.", "EXCELDATESERIAL DAY OF MONTH(39448)", [], ["EXCELDATESERIAL"], "EXCELDATESERIAL DAY OF MONTH(serial)"),
    doc!("EXCELDATESERIAL DAY OF YEAR", "Math", "EXCELDATESERIAL DAY OF YEAR(serial)", "The day of the year of an Excel serial date.",
        [("serial", "An Excel date number")], "1 to 366.",
        "Taking a date apart.", "EXCELDATESERIAL DAY OF YEAR(39448)", [], ["EXCELDATESERIAL"], "EXCELDATESERIAL DAY OF YEAR(serial)"),
    doc!("REAL RATE", "Finance", "REAL RATE(nominal, inflation)", "The real rate from a nominal rate and inflation.",
        [("nominal", "Nominal rate per period"),("inflation", "Inflation per period")], "(1 + nominal) / (1 + inflation) - 1.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: REALRATE (same arguments). Example value: 0.0439.", "REAL RATE(0.07, 0.025)", [], [], "REAL RATE(nominal, inflation)"),
    doc!("NOMINAL RATE", "Finance", "NOMINAL RATE(real, inflation)", "The nominal rate from a real rate and inflation.",
        [("real", "Real rate per period"),("inflation", "Inflation per period")], "(1 + real) * (1 + inflation) - 1.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: NOMINALRATE (same arguments). Example value: 0.0608.", "NOMINAL RATE(0.04, 0.02)", [], [], "NOMINAL RATE(real, inflation)"),
    doc!("EFFECTIVE RATE", "Finance", "EFFECTIVE RATE(nominal, m)", "The effective rate of a nominal rate compounded m times (alias EFFECT).",
        [("nominal", "Nominal rate"),("m", "Compounding periods")], "(1 + nominal / m)^m - 1.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: EFFECTIVERATE (same arguments). Example value: 0.126825.", "EFFECTIVE RATE(0.12, 12)", [], [], "EFFECTIVE RATE(nominal, m)"),
    doc!("WACC", "Finance", "WACC(D, kd, ke, T)", "Weighted average cost of capital.",
        [("D", "Debt share"),("kd", "Cost of debt"),("ke", "Cost of equity"),("T", "Tax rate, default 0")], "D*kd*(1-T) + (1-D)*ke.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: WACC (same arguments). Example value: 0.061907.", "WACC(0.7, 0.07, 0.085, 0.257)", [], [], "WACC(D, kd, ke, T)"),
    doc!("ANNUITY FACTOR", "Finance", "ANNUITY FACTOR(r, n)", "The capital recovery factor (alias CRF).",
        [("r", "Rate per period"),("n", "Periods")], "r / (1 - (1+r)^-n), or 1/n at r = 0.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: ANNUITYFACTOR (same arguments). Example value: 0.101852.", "ANNUITY FACTOR(0.08, 20)", [], [], "ANNUITY FACTOR(r, n)"),
    doc!("PMT", "Finance", "PMT(r, n, pv, fv, type)", "The level payment of a loan.",
        [("r", "Rate per period"),("n", "Periods"),("pv", "Present value"),("fv", "Future value, default 0"),("type", "0 end, 1 start of period")], "Payment, negative when paid out.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: PMT (same arguments). Example value: -1073.643246.", "PMT(0.05/12, 360, 200000)", [], [], "PMT(r, n, pv, fv, type)"),
    doc!("FV", "Finance", "FV(r, n, pmt, pv, type)", "The future value of payments.",
        [("r", "Rate per period"),("n", "Periods"),("pmt", "Payment"),("pv", "Present value, default 0"),("type", "0 or 1")], "Future value.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: FV (same arguments). Example value: 18207.33.", "FV(0.06/12, 120, -100, -1000)", [], [], "FV(r, n, pmt, pv, type)"),
    doc!("PV", "Finance", "PV(r, n, pmt, fv, type)", "The present value of payments.",
        [("r", "Rate per period"),("n", "Periods"),("pmt", "Payment"),("fv", "Future value, default 0"),("type", "0 or 1")], "Present value.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: PV (same arguments). Example value: 4694.525.", "PV(0.08, 20, -500, 1000)", [], [], "PV(r, n, pmt, fv, type)"),
    doc!("NPER", "Finance", "NPER(r, pmt, pv, fv, type)", "The number of periods.",
        [("r", "Rate per period"),("pmt", "Payment"),("pv", "Present value"),("fv", "Default 0"),("type", "0 or 1")], "Periods.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: NPER (same arguments). Example value: 27.1008.", "NPER(0.07/12, -200, 5000)", [], [], "NPER(r, pmt, pv, fv, type)"),
    doc!("IPMT", "Finance", "IPMT(r, per, n, pv, fv, type)", "The interest part of payment number per.",
        [("r", "Rate per period"),("per", "Payment number"),("n", "Periods"),("pv", "Present value"),("fv", "Default 0"),("type", "0 or 1")], "Interest.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: IPMT (same arguments). Example value: -833.3333.", "IPMT(0.05/12, 1, 360, 200000)", [], [], "IPMT(r, per, n, pv, fv, type)"),
    doc!("PPMT", "Finance", "PPMT(r, per, n, pv, fv, type)", "The principal part of payment number per.",
        [("r", "Rate per period"),("per", "Payment number"),("n", "Periods"),("pv", "Present value"),("fv", "Default 0"),("type", "0 or 1")], "Principal.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: PPMT (same arguments). Example value: -240.3099.", "PPMT(0.05/12, 1, 360, 200000)", [], [], "PPMT(r, per, n, pv, fv, type)"),
    doc!("RATE", "Finance", "RATE(n, pmt, pv, fv, type, guess)", "The rate per period that solves the loan equation.",
        [("n", "Periods"),("pmt", "Payment"),("pv", "Present value"),("fv", "Default 0"),("type", "0 or 1"),("guess", "Starting guess, default 0.1")], "Rate; missing if there is none.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: RATE (same arguments). Example value: 0.0077015.", "RATE(48, -200, 8000)", [], [], "RATE(n, pmt, pv, fv, type, guess)"),
    doc!("DSCR", "Finance", "DSCR(cash, debt service)", "Debt service cover ratio.",
        [("cash", "Cash available"),("debt service", "Payments due")], "cash / debt service, or 1e9 with no debt service.",
        "Rates are per period, as in Excel. Money follows Excel's sign convention: paid out is negative. In Excel: DSCR (same arguments). Example value: 1.2.", "DSCR(120, 100)", [], [], "DSCR(cash, debt service)"),
    doc!("MAX OF", "Sets", "MAX OF(name, group)", "The largest value of a variable that is per a set.",
        [("name", "A variable per a set"), ("group", "Optional: only the items of this group")], "One number.",
        "Vensim's VMAX.", "MAX OF(Level)", [], ["TOTAL"], "MAX OF(name)"),
    doc!("MIN OF", "Sets", "MIN OF(name, group)", "The smallest value of a variable that is per a set.",
        [("name", "A variable per a set"), ("group", "Optional: only the items of this group")], "One number.",
        "Vensim's VMIN.", "MIN OF(Level)", [], ["TOTAL"], "MIN OF(name)"),
    doc!("COUNT OF", "Sets", "COUNT OF(name, group)", "The number of items of value of a variable that is per a set.",
        [("name", "A variable per a set"), ("group", "Optional: only the items of this group")], "One number.",
        "Vensim's ELMCOUNT and SIZE.", "COUNT OF(Level)", [], ["TOTAL"], "COUNT OF(name)"),
    doc!("AND", "Logic", "AND(a, b, …)", "1 when every argument is true.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "AND(Open, Staffed)", [], [], "AND(a, b, …)"),
    doc!("OR", "Logic", "OR(a, b, …)", "1 when any argument is true.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "OR(Open, Staffed)", [], [], "OR(a, b, …)"),
    doc!("NOT", "Logic", "NOT(x)", "1 when x is zero.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "NOT(Open)", [], [], "NOT(x)"),
    doc!("BETWEEN", "Logic", "BETWEEN(x, low, high)", "1 if low <= x <= high.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "BETWEEN(Level, 10, 20)", [], [], "BETWEEN(x, low, high)"),
    doc!("DATE", "Calendar", "DATE()", "The Excel serial date of the current time.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "DATE()", [], [], "DATE()"),
    doc!("YEAR", "Calendar", "YEAR()", "The calendar year of the current time.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "YEAR()", [], [], "YEAR()"),
    doc!("MONTH", "Calendar", "MONTH()", "The month of the current time.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "MONTH()", [], [], "MONTH()"),
    doc!("DAY OF MONTH", "Calendar", "DAY OF MONTH()", "The day of the month.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "DAY OF MONTH()", [], [], "DAY OF MONTH()"),
    doc!("DAY OF YEAR", "Calendar", "DAY OF YEAR()", "The day of the year.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "DAY OF YEAR()", [], [], "DAY OF YEAR()"),
    doc!("HOUR OF DAY", "Calendar", "HOUR OF DAY()", "The hour of the day.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "HOUR OF DAY()", [], [], "HOUR OF DAY()"),
    doc!("WEEK NUMBER", "Calendar", "WEEK NUMBER()", "The ISO week number.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "WEEK NUMBER()", [], [], "WEEK NUMBER()"),
    doc!("DAY OF WEEK", "Calendar", "DAY OF WEEK()", "1 Monday to 7 Sunday.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "DAY OF WEEK()", [], [], "DAY OF WEEK()"),
    doc!("IS WEEKEND", "Calendar", "IS WEEKEND()", "1 on a weekend day.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "IS WEEKEND()", [], [], "IS WEEKEND()"),
    doc!("IS WORKDAY", "Calendar", "IS WORKDAY()", "1 when not a weekend and not a holiday.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "IS WORKDAY()", [], [], "IS WORKDAY()"),
    doc!("WORKDAYS BETWEEN", "Calendar", "WORKDAYS BETWEEN(t1, t2)", "Workdays from t1 up to t2.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "WORKDAYS BETWEEN(0, 30)", [], [], "WORKDAYS BETWEEN(t1, t2)"),
    doc!("PROFILE", "Calendar", "PROFILE(period, v1, …, vn)", "A repeating profile, interpolated between slot centres.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "PROFILE(24, 1, 2, 3, 2)", [], [], "PROFILE(period, v1, …, vn)"),
    doc!("PROFILE STEP", "Calendar", "PROFILE STEP(period, v1, …, vn)", "A repeating profile, constant in each slot.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "PROFILE STEP(7, 1,1,1,1,1,1.3,1.3)", [], [], "PROFILE STEP(period, v1, …, vn)"),
    doc!("SEASON", "Calendar", "SEASON(low, high, peak time, period)", "A cosine season between low and high.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "SEASON(2, 5, 3, 12)", [], [], "SEASON(low, high, peak time, period)"),
    doc!("DAYLIGHT", "Calendar", "DAYLIGHT(sunrise, sunset)", "A half-sine between sunrise and sunset hours.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "DAYLIGHT(6, 18)", [], [], "DAYLIGHT(sunrise, sunset)"),
    doc!("AT TIME", "Calendar", "AT TIME(t)", "1 when Time is within half a step of t.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "AT TIME(13)", [], [], "AT TIME(t)"),
    doc!("EVERY", "Calendar", "EVERY(period, start)", "1 every period, from start.",
        [], "A number.",
        "Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.", "EVERY(7)", [], [], "EVERY(period, start)"),
    doc!("NPV AT", "Delays", "NPV AT(stream, rate, init, factor, offset)", "Present value with period timing: the engine form of VALUE(timing=period_start or mid_period).",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "NPV AT(Cash, 0.1, 0, 1, 0)", [], [], "NPV AT(stream, rate, init, factor, offset)"),
    doc!("LEVELISED STATE", "Finance", "LEVELISED STATE(cost, quantity, rate, init, timing)", "The engine form of LEVELISED.",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "LEVELISED STATE(OM, Output, 0.04, Capex, 2)", [], [], "LEVELISED STATE(cost, quantity, rate, init, timing)"),
    doc!("LOAN STATE", "Finance", "LOAN STATE(drawdown, rate, term, type, grace, measure)", "The engine form of LOAN.",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "LOAN STATE(Draw, 0.05, 30, 0, 0, 1)", [], [], "LOAN STATE(drawdown, rate, term, type, grace, measure)"),
    doc!("TAX STATE", "Finance", "TAX STATE(income, rate, carryforward, years)", "The engine form of TAX.",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "TAX STATE(Income, 0.2, 1, 0)", [], [], "TAX STATE(income, rate, carryforward, years)"),
    doc!("PAYBACK", "Finance", "PAYBACK(stream, rate)", "The payback time of a cash stream.",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "PAYBACK(Cash Flow)", [], [], "PAYBACK(stream, rate)"),
    doc!("ESCALATE", "Finance", "ESCALATE(base, rate, from)", "A price escalated at a rate per period of the rate.",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "ESCALATE(100, 0.03)", [], [], "ESCALATE(base, rate, from)"),
    doc!("LEVELISED", "Finance", "LEVELISED(cost stream, quantity stream, rate, initial cost, timing=period_end)", "Levelised cost: discounted costs over discounted quantity (alias LCOE).",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "LEVELISED(OM, Output, Real WACC, Capex)", [], [], "LEVELISED(cost stream, quantity stream, rate, initial cost, timing=period_end)"),
    doc!("LOAN", "Finance", "LOAN(drawdown, rate, term, type=annuity, grace=0, measure=balance)", "A loan with tranches: balance, payment, interest or principal.",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "LOAN(Drawdown, Loan Rate, Term, measure=payment)", [], [], "LOAN(drawdown, rate, term, type=annuity, grace=0, measure=balance)"),
    doc!("TAX", "Finance", "TAX(taxable income, rate, carryforward=yes, years=0)", "Tax with losses carried forward.",
        [], "A number.",
        "A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.", "TAX(Income, 0.2)", [], [], "TAX(taxable income, rate, carryforward=yes, years=0)"),
    doc!("RANDOM TRIANGULAR", "Random", "RANDOM TRIANGULAR(min, max, mode, [seed])", "Triangular random numbers.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM TRIANGULAR", [], [], "RANDOM TRIANGULAR(min, max, mode, [seed])"),
    doc!("RANDOM LOGNORMAL", "Random", "RANDOM LOGNORMAL(min, max, mean, sd, [seed])", "Log-normal random numbers with the mean and standard deviation of the variable itself.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM LOGNORMAL", [], [], "RANDOM LOGNORMAL(min, max, mean, sd, [seed])"),
    doc!("RANDOM GAMMA", "Random", "RANDOM GAMMA(min, max, shape, scale, [seed])", "Gamma random numbers.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM GAMMA", [], [], "RANDOM GAMMA(min, max, shape, scale, [seed])"),
    doc!("RANDOM BETA", "Random", "RANDOM BETA(min, max, a, b, [seed])", "Beta random numbers stretched over min to max.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM BETA", [], [], "RANDOM BETA(min, max, a, b, [seed])"),
    doc!("RANDOM WEIBULL", "Random", "RANDOM WEIBULL(min, max, shape, scale, [seed])", "Weibull random numbers.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM WEIBULL", [], [], "RANDOM WEIBULL(min, max, shape, scale, [seed])"),
    doc!("RANDOM POISSON", "Random", "RANDOM POISSON(min, max, mean, [seed])", "Poisson random counts.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM POISSON", [], [], "RANDOM POISSON(min, max, mean, [seed])"),
    doc!("RANDOM BINOMIAL", "Random", "RANDOM BINOMIAL(min, max, n, p, [seed])", "Binomial random counts.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM BINOMIAL", [], [], "RANDOM BINOMIAL(min, max, n, p, [seed])"),
    doc!("RANDOM NEGATIVE BINOMIAL", "Random", "RANDOM NEGATIVE BINOMIAL(min, max, r, p, [seed])", "Failures before the r-th success.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM NEGATIVE BINOMIAL", [], [], "RANDOM NEGATIVE BINOMIAL(min, max, r, p, [seed])"),
    doc!("RANDOM GEOMETRIC", "Random", "RANDOM GEOMETRIC(min, max, p, [seed])", "Failures before the first success.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM GEOMETRIC", [], [], "RANDOM GEOMETRIC(min, max, p, [seed])"),
    doc!("RANDOM PARETO", "Random", "RANDOM PARETO(min, max, shape, scale, [seed])", "Pareto random numbers.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM PARETO", [], [], "RANDOM PARETO(min, max, shape, scale, [seed])"),
    doc!("RANDOM PERT", "Random", "RANDOM PERT(min, max, mode, shape, [seed])", "PERT random numbers (a stretched beta around the mode; shape 4 is usual).",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM PERT", [], [], "RANDOM PERT(min, max, mode, shape, [seed])"),
    doc!("RANDOM 0 1", "Random", "RANDOM 0 1([seed])", "A uniform random number between 0 and 1.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "RANDOM 0 1", [], [], "RANDOM 0 1([seed])"),
    doc!("TRIANGULAR", "Random", "TRIANGULAR(low, mode, high, [seed])", "Triangular random numbers.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "TRIANGULAR", [], [], "TRIANGULAR(low, mode, high, [seed])"),
    doc!("LOGNORMAL", "Random", "LOGNORMAL(mean, sd, [seed])", "Log-normal random numbers with the mean and standard deviation of the variable itself.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "LOGNORMAL", [], [], "LOGNORMAL(mean, sd, [seed])"),
    doc!("WEIBULL", "Random", "WEIBULL(shape, scale, [seed])", "Weibull random numbers.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "WEIBULL", [], [], "WEIBULL(shape, scale, [seed])"),
    doc!("POISSON", "Random", "POISSON(mean, [seed])", "Poisson random counts.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "POISSON", [], [], "POISSON(mean, [seed])"),
    doc!("NORMAL", "Random", "NORMAL(mean, sd, [seed])", "Normal random numbers (Stella).",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "NORMAL", [], [], "NORMAL(mean, sd, [seed])"),
    doc!("UNIFORM", "Random", "UNIFORM(min, max, [seed])", "Uniform random numbers (Stella).",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "UNIFORM", [], [], "UNIFORM(min, max, [seed])"),
    doc!("EXPRND", "Random", "EXPRND(mean, [seed])", "Exponential random numbers (Stella).",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "EXPRND", [], [], "EXPRND(mean, [seed])"),
    doc!("NORMAL CDF", "Statistics", "NORMAL CDF(x, mean, sd)", "The cumulative normal distribution.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "NORMAL CDF", [], [], "NORMAL CDF(x, mean, sd)"),
    doc!("NORMAL INV", "Statistics", "NORMAL INV(p, mean, sd)", "The inverse of the cumulative normal distribution.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "NORMAL INV", [], [], "NORMAL INV(p, mean, sd)"),
    doc!("LOGNORMAL CDF", "Statistics", "LOGNORMAL CDF(x, mu, sigma)", "The cumulative log-normal distribution (mu and sigma of the log).",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "LOGNORMAL CDF", [], [], "LOGNORMAL CDF(x, mu, sigma)"),
    doc!("LOGNORMAL INV", "Statistics", "LOGNORMAL INV(p, mu, sigma)", "The inverse of the cumulative log-normal distribution.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "LOGNORMAL INV", [], [], "LOGNORMAL INV(p, mu, sigma)"),
    doc!("PREVIOUS", "Statistics", "PREVIOUS(x, initial)", "The value x had at the previous step (initial at the start).",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "PREVIOUS", [], [], "PREVIOUS(x, initial)"),
    doc!("HISTORY", "Statistics", "HISTORY(x, time)", "The value x had at an earlier time.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "HISTORY", [], [], "HISTORY(x, time)"),
    doc!("ROUND WITH CARRY", "Statistics", "ROUND WITH CARRY(flow)", "A flow moved in whole units: the remainder carries to the next step, so the long-run average equals the input.",
        [], "A number.",
        "A call without a seed has its own stream, derived from the model seed (setting `seed`, default 1, `dyn run --seed N`), the variable and the call, so adding a variable never changes another variable's draws. Write seed=3 or a last argument to give the call a seed of its own. The argument order of the RANDOM family is min, max, then the parameters of the distribution.", "ROUND WITH CARRY", [], [], "ROUND WITH CARRY(flow)"),
    doc!("DAYS IN STEP", "Calendar", "DAYS IN STEP()", "The number of calendar days in the current step.",
        [], "The real length of the step in days: 31 for a January month step, 29 for February 2028.",
        "Needs a start date in the model settings. With the Calendar time convention each step of a Month or Year model is the real calendar month or year, and a flow declared per Day counts these days when it enters its stock.",
        "DAYS IN STEP()", [], ["HOURS IN STEP", "YEAR FRACTION"], "DAYS IN STEP()"),
    doc!("HOURS IN STEP", "Calendar", "HOURS IN STEP()", "The number of hours in the current step.",
        [], "24 times the days in the step (8,784 hours in the year 2028).",
        "Needs a start date in the model settings.", "HOURS IN STEP()", [], ["DAYS IN STEP"], "HOURS IN STEP()"),
    doc!("YEAR FRACTION", "Calendar", "YEAR FRACTION(t1, t2, basis)", "The fraction of a year between two times, as Excel's YEARFRAC.",
        [("t1", "A time of the model"), ("t2", "A later time of the model"), ("basis", "0 = 30/360 US (default), 1 = actual/actual, 2 = actual/360, 3 = actual/365, 4 = 30E/360")], "Years between the two dates the times stand for.",
        "Needs a start date in the model settings. The 30/360 US basis follows Microsoft's February rules.", "YEAR FRACTION(0, 6, 3)", [], ["DAYS IN STEP", "EXCELDATESERIAL"], "YEAR FRACTION(t1, t2, basis)"),
    doc!("VECTOR LOOKUP ITEMS", "Sets", "VECTOR LOOKUP ITEMS(n, y…, x, xmin, xmax, mode)", "A lookup whose y values are the elements of an array.",
        [("…", "Written out by Dynamist when it reads Vensim's VECTOR LOOKUP: every element, then x, xmin, xmax and the mode")], "The lookup read in the mode asked for (0 normal, 1 area, 2 backward, 3 extrapolate, 4 forward, 5 invert, 6 slope, 7–9 with extrapolation).",
        "A curve given as an array of values over equal steps.", "VECTOR LOOKUP(Curve[first], X, 0, 10, 0)", [], ["LOOKUP SLOPE"], "VECTOR LOOKUP ITEMS(n, y…, x, xmin, xmax, mode)"),
    doc!("EXPECT", "Delays", "EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)", "What people believe about x: how they perceive it, the trend they see, or where they expect it to be.",
        [("x", "The real thing"), ("time", "How long people take to notice a change"), ("ahead=", "How far ahead they expect. Gives a forecast: the level now plus the trend they see times this horizon"), ("init=", "What they believe at the start (default: the true value)"), ("measure=", "level (default): the belief about now. trend: the growth they think they see, per time unit. forecast: where they expect it to be (the default when ahead= is given)")], "A belief: the true value, noticed late, extrapolated if you ask.",
        "Replaces SMOOTH, SMOOTHI, TREND and FORECAST. People rarely act on facts; they act on what they believe the facts are. EXPECT(Demand, 6) is demand as noticed over about six weeks; EXPECT(Demand, 6, ahead=8) is where they expect it to be in eight weeks. A trend extrapolation overshoots at turning points.", "EXPECT(Demand, 6, ahead=8)", [], ["DELAY", "TREND"], "EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)"),
    doc!("LOOK", "Lookups", "LOOK(curve, x, mode=lookup|forward|backward|inverse|extrapolate|slope|area, start=, outside=)", "Reads a curve (a lookup) in the way you ask for.",
        [("curve", "The name of a lookup"), ("x", "Where to read it (for mode=inverse, the value to find)"), ("mode=", "lookup (default): normal reading. forward / backward: as steps, the next or previous point. inverse: the input that gives this value. extrapolate: continue the end slopes beyond the ends. slope: how steep it is here. area: the area under it"), ("start=", "With mode=area: where to start (default 0); the area runs from start to x"), ("outside=", "With mode=slope: outside the points, 0 (default), 1 the nearest slope, −1 not available")], "The value, the input, the slope or the area, depending on the mode.",
        "Replaces the lookup functions LOOKUP FORWARD, BACKWARD, INVERT, EXTRAPOLATE, SLOPE and AREA. LOOK(Effect, Price) is the same as Effect(Price).", "LOOK(Effect of price, 0.7, mode=inverse)", [], ["LOOKUP SLOPE", "WITH LOOKUP"], "LOOK(curve, x, mode=lookup|forward|backward|inverse|extrapolate|slope|area, start=, outside=)"),
    doc!("INTERNAL RATE OF RETURN", "Delays", "INTERNAL RATE OF RETURN(stream, period, initial)", "The rate of return that makes a stream of money worth nothing at the start.",
        [("stream", "Money per time unit, spending negative, income positive"), ("period", "The length of the period the rate is per (default 1 time unit)"), ("initial", "Money at the start (default 0)")], "The rate per period, from the money so far; not available until the stream has both spending and income.",
        "Vensim's name for VALUE(stream, measure=irr). The rate that turns the net present value of the stream to zero.", "INTERNAL RATE OF RETURN(Cash Flow, 12, 0)", [], ["VALUE"], "INTERNAL RATE OF RETURN(stream, period, initial)"),
    doc!("AGE", "Sets", "AGE(stock, every=, entrants=, open_ended=True|False)", "Moves people up through age classes. Gives one class its net change from ageing: the class before it moves in, and its own people move on.",
        [("stock", "A variable with one value per age class, in order from the youngest"), ("every=", "How often the classes move up, in time units (default 1). The whole class moves at once, at the end of each period"), ("entrants=", "People joining the youngest class each time unit; the youngest class is refilled with a period's worth at each move (default none)"), ("open_ended=", "True (default): the oldest class keeps its people, like '65 and over'. False: they leave the model when the classes move up")],
        "The change in this class from ageing: people arriving from the class before, minus its own people moving on.",
        "Write it in a formula that is per the age set, add deaths and anything else, and use the sum as the stock's change: Change[age] = AGE(Population, every=5, entrants=Births) - Deaths[age]. Classes move together, so the stock rises and falls in steps of one period; read it at the moves (every 5 years here). Replaces SHIFT IF TRUE and hand-built ageing chains. Use Euler integration.",
        "AGE(Population, every=5, entrants=Births per year)", [], ["QUEUE", "DELAY"], "AGE(stock, every=1)"),
    doc!("QUEUE", "Delays", "QUEUE(arrivals, served, init=)", "A waiting line: a level that remembers how long each has waited. The oldest leaves first.",
        [("arrivals", "How many join each time unit"), ("served", "How many leave each time unit; never more than are waiting"), ("init=", "How many are waiting at the start (default 0); they count as one time step old")], "The number waiting.",
        "Use it when who waits longest matters: patients, orders, support tickets. Read how long they have waited with WAIT. Replaces QUEUE FIFO.", "Waiting = QUEUE(Arrivals, Treated)", [], ["WAIT"], "QUEUE(arrivals, served, init=)"),
    doc!("WAIT", "Delays", "WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)", "How long the people in a queue have waited.",
        [("queue", "The name of a variable defined with QUEUE"), ("measure=", "average (default): the mean wait of those waiting. oldest: the longest wait. count: how many have waited within a range"), ("among=", "With average: only the n who have waited longest"), ("older_than=", "With count: waited at least this long"), ("younger_than=", "With count: waited no longer than this")], "A time (average, oldest) or a number of people (count).",
        "Replaces QUEUE AGE AVERAGE, QUEUE AGE OLDEST and QUEUE AGE IN RANGE. WAIT(Waiting, measure=count, older_than=2) is how many have waited more than 2 hours.", "WAIT(Waiting, measure=oldest)", [], ["QUEUE"], "WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)"),
    doc!("QUEUE FIFO", "Delays", "QUEUE FIFO(inflow, outflow, initial)", "The queue function Vensim writes. Dynamist writes QUEUE(...) in its place.",
        [("inflow", "How many join"), ("outflow", "How many leave"), ("initial", "How many are waiting at the start")], "The number waiting.",
        "In Vensim the call also gives a profile that spreads the starting crowd over ages; that part is not read.", "QUEUE FIFO(Arriving, Servicing, 0)", [], ["QUEUE"], "QUEUE FIFO(inflow, outflow, initial)"),
    doc!("QUEUE AGE AVERAGE", "Delays", "QUEUE AGE AVERAGE(queue, [n])", "The average time those waiting have waited.",
        [("queue", "A variable defined with QUEUE FIFO"), ("n", "Only the n who have waited longest (default everyone)")], "A time.",
        "Vensim's name for WAIT(queue).", "QUEUE AGE AVERAGE(Waiting)", [], ["WAIT"], "QUEUE AGE AVERAGE(queue, [n])"),
    doc!("QUEUE AGE OLDEST", "Delays", "QUEUE AGE OLDEST(queue)", "How long the one who has waited longest has waited.",
        [("queue", "A variable defined with QUEUE FIFO")], "A time.",
        "Vensim's name for WAIT(queue, measure=oldest).", "QUEUE AGE OLDEST(Waiting)", [], ["WAIT"], "QUEUE AGE OLDEST(queue)"),
    doc!("QUEUE AGE IN RANGE", "Delays", "QUEUE AGE IN RANGE(queue, min age, max age)", "How many waiting have waited between two times.",
        [("queue", "A variable defined with QUEUE FIFO"), ("min age", "At least this long"), ("max age", "No longer than this")], "A number of people.",
        "Vensim's name for WAIT(queue, measure=count, …).", "QUEUE AGE IN RANGE(Waiting, 2, 1000)", [], ["WAIT"], "QUEUE AGE IN RANGE(queue, min age, max age)"),
    doc!("VALUE", "Delays", "VALUE(stream, rate, init=, factor=, timing=continuous|period_end) · VALUE(stream, measure=irr) · VALUE(stream, writeoff=time)", "The present value of a stream of money: what it is worth at the start of the run, discounted at a rate, accumulated up to now.",
        [("stream", "Money per time unit: income positive, spending negative"), ("rate", "The discount rate per time unit, for example 0.01 per month"), ("init=", "A value to start from (default 0)"), ("factor=", "Multiplies the result (default 1)"), ("timing=", "continuous (default, small steps) or period_end (the way a spreadsheet's NPV counts, with each period's money at its end)")],
        "The discounted sum so far.",
        "Use it to compare investments: put the spending in the stream as a negative and the income as positive, and read VALUE at the end of the run. VALUE(stream, measure=irr) gives the rate of return that makes the stream worth nothing; VALUE(stream, writeoff=time) writes each amount off evenly over that time. Replaces NPV, NPVE, INTERNAL RATE OF RETURN and DEPRECIATE STRAIGHTLINE.",
        "VALUE(Revenue - Costs, 0.01)", [], ["DELAY"], "VALUE(stream, rate)"),
    doc!("ALLOCATE", "Sets", "ALLOCATE(requests, supply, by=, overlap=, curve=rectangular|triangular|normal|exponential, spread=)", "Shares a limited supply among the items of a set when it is short. Each item gets its own share.",
        [("requests", "What each item asks for (a variable with one value per item)"), ("supply", "What there is to share (one number)"), ("by=", "Priorities (a variable per the same set). The highest is served first. Without it, everyone is served in proportion to what they ask"), ("overlap=", "Items whose priorities are this close share between them (default: none, strict order)"), ("curve=", "Instead of a strict order, a smooth fall-off around each priority: rectangular, triangular, normal or exponential. Needs by="), ("spread=", "How wide the curve is (default 1)")],
        "This item's share. When the supply covers all requests, everyone gets what they asked.",
        "Use it in a formula that is per the same set as the requests. Replaces Vensim's ALLOCATE AVAILABLE and ALLOCATE BY PRIORITY: ALLOCATE(Demand, Water) shares in proportion; ALLOCATE(Demand, Water, by=Priority) serves the highest priority first; ALLOCATE(Demand, Water, by=Priority, curve=normal, spread=2) lets the lower priorities get a little before the higher ones are full.",
        "Delivered[Region] = ALLOCATE(Demand, Supply, by=Priority)", [], ["TOTAL", "DELAY"], "ALLOCATE(requests, supply)"),
    doc!("DELAY", "Delays", "DELAY(x, time, [order], init=, kind=material|information, exact=True|False, leak=)", "Delays x by a time. One function for every kind of delay: say what you mean with an option.",
        [("x", "What is delayed"), ("time", "The average delay time (the transit time when exact=True)"), ("order", "How many stages, 1 to 20 (default 1). More stages = a sharper, more bunched delay"), ("init=", "What comes out at the start (default: the input at the start, a steady state)"), ("kind=", "material (default): what goes in comes out later, nothing is lost. information: a smoothed perception; nothing is conserved"), ("exact=", "True: a pipeline. Everything arrives exactly `time` after it entered (default False: smooth)"), ("leak=", "With exact=True: a conveyor that loses this fraction of what is on it each time step, so less arrives than entered")],
        "The delayed value: smooth by default, or exactly `time` late with exact=True.",
        "Replaces DELAY1, DELAY3, DELAYMTR, DELAYINF, SMOOTH, SMOOTH3 and DELAY FIXED. In words: DELAY(x, 6) is material in transit for about 6 time units, spread out. DELAY(x, 6, order=3) is the same, more bunched around 6. DELAY(x, 6, exact=True) is a pipeline of exactly 6. DELAY(x, 6, kind=information) is how a belief catches up with a fact.",
        "DELAY(Shipments, 6, order=3)", [], ["SMOOTH", "DELAY FIXED"], "DELAY(x, time)"),
    doc!("NPV", "Delays", "NPV(stream, discount rate, initial value, factor)", "Net present value of a stream of money, accumulated over the run.",
        [("stream", "Money per time unit (negative for spending)"), ("discount rate", "Per time unit, e.g. 0.01 per month"), ("initial value", "The value at the start"), ("factor", "Multiplies the result (1 for none)")],
        "The discounted sum of the stream so far, times the factor.",
        "Compare investments: a pulse of spending and a stream of revenue, discounted to the start of the run. Same definition as Vensim.",
        "NPV(Revenue - Costs, 0.01, 0, 1)", [], ["SMOOTH"], "NPV(stream, discount rate, 0, 1)"),
    doc!("DEPRECIATE STRAIGHTLINE", "Delays", "DEPRECIATE STRAIGHTLINE(input, depreciation time, [fiscal], [initial])", "Writes each input off evenly over the depreciation time.",
        [("input", "What is added this step (an investment, say)"), ("depreciation time", "How long each input takes to write off"), ("fiscal", "Optional; read but not used"), ("initial", "Optional; read but not used")],
        "The amount written off this step: each recent input divided by the depreciation time.",
        "Straight-line depreciation of investments, as Vensim's DEPRECIATE STRAIGHTLINE.",
        "DEPRECIATE STRAIGHTLINE(Investment, 10)", [], ["DELAY FIXED"], "DEPRECIATE STRAIGHTLINE(input, depreciation time)"),
    doc!("GAMMA LN", "Math", "GAMMA LN(x)", "The natural logarithm of the gamma function.",
        [("x", "A positive number")], "ln Γ(x); Γ(n) = (n − 1)! for whole numbers.",
        "Factorials and binomial coefficients of large numbers without overflow.", "EXP(GAMMA LN(n + 1))", [], ["EXP"], "GAMMA LN(x)"),
    doc!("RANDOM EXPONENTIAL", "Test inputs", "RANDOM EXPONENTIAL(min, max, shift, stretch, seed)", "A new exponentially distributed random number every time step.",
        [("min", "Lowest value kept"), ("max", "Highest value kept"), ("shift", "Added to the draw"), ("stretch", "Multiplies the draw (the mean of the unshifted draw)"), ("seed", "Picks the stream")],
        "shift + stretch × an exponential draw with mean 1, kept within min and max.",
        "Times between arrivals. The numbers differ from Vensim's; the distribution is the same.", "RANDOM EXPONENTIAL(0, 100, 0, 5, 1)", [], ["RANDOM UNIFORM"], "RANDOM EXPONENTIAL(0, 100, 0, 1, 1)"),
    doc!("LOOKUP FORWARD", "Lookups", "LOOKUP FORWARD(lookup, x)", "Reads a lookup as steps: the y of the next point at or after x.",
        [("lookup", "The name of a lookup"), ("x", "Where to read")], "The y of the first point whose x is at or above x; no interpolation.",
        "For values that hold until the next point, such as a price list.", "LOOKUP FORWARD(Price Table, Time)", [], ["LOOKUP BACKWARD", "WITH LOOKUP"], "LOOKUP FORWARD(lookup, x)"),
    doc!("LOOKUP BACKWARD", "Lookups", "LOOKUP BACKWARD(lookup, x)", "Reads a lookup as steps: the y of the last point at or before x.",
        [("lookup", "The name of a lookup"), ("x", "Where to read")], "The y of the last point whose x is at or below x; no interpolation.",
        "For values that hold from one point until the next.", "LOOKUP BACKWARD(Tax Bands, Income)", [], ["LOOKUP FORWARD"], "LOOKUP BACKWARD(lookup, x)"),
    doc!("LOOKUP INVERT", "Lookups", "LOOKUP INVERT(lookup, y)", "Reads a lookup the other way: the x at which it reaches y.",
        [("lookup", "The name of a lookup that only rises or only falls"), ("y", "The value to find")], "The x, interpolated, where the lookup equals y.",
        "Turning a target value back into the input that gives it.", "LOOKUP INVERT(Yield Curve, Target Yield)", [], ["LOOKUP FORWARD"], "LOOKUP INVERT(lookup, y)"),
    doc!("LOOKUP EXTRAPOLATE", "Lookups", "LOOKUP EXTRAPOLATE(lookup, x)", "Like the lookup, but continues in straight lines beyond the first and last points.",
        [("lookup", "The name of a lookup"), ("x", "Where to read")], "The interpolated y; beyond the ends, the line through the two end points continued.",
        "When the trend at the edge of the data should carry on instead of staying flat.", "LOOKUP EXTRAPOLATE(Demand Curve, Price)", [], ["LOOKUP FORWARD"], "LOOKUP EXTRAPOLATE(lookup, x)"),
    doc!("SAMPLE IF TRUE", "Delays", "SAMPLE IF TRUE(condition, input, initial)", "Takes the input while the condition holds, and keeps the last value taken otherwise.",
        [("condition", "Non-zero means take the input now"), ("input", "The value sampled"), ("initial", "The value held until the condition first holds")],
        "The input when the condition holds; otherwise the value last taken.",
        "Models a value read only at certain moments: a quarterly report, a reading taken when a switch is on.",
        "SAMPLE IF TRUE(MODULO(Time, 3) = 0, Sales, 0)", [], ["INITIAL"], "SAMPLE IF TRUE(condition, input, initial)"),
    doc!("QUANTUM", "Math", "QUANTUM(x, q)", "x cut down toward zero to a whole multiple of q.",
        [("x", "The value"), ("q", "The step it is cut to; 0 or less leaves x unchanged")],
        "q × INTEGER(x / q), or x when q is 0 or less.",
        "Orders placed in whole batches or containers.", "QUANTUM(Desired Order, 12)", [], ["INTEGER"], "QUANTUM(x, q)"),
    doc!("ACTIVE INITIAL", "Delays", "ACTIVE INITIAL(x, initial value)", "x during the run; a separate value used only while initial values are computed.",
        [("x", "The formula used during the run"), ("initial value", "The value used while the model's starting values are worked out")],
        "x at every step of the run; the initial value only at initialisation.",
        "Breaks a loop among starting values, as World3-03 does: food per capita needs food, which needs land yield, which needs food per capita.",
        "ACTIVE INITIAL(food / population, 230)", [], ["INITIAL"], "ACTIVE INITIAL(x, initial value)"),
    doc!("INITIAL", "Delays", "INITIAL(x)", "The value x had at the start of the run, held constant.",
        [("x", "Any value or expression")], "x evaluated once, at the initial time.",
        "Handy for comparing against the starting point: e.g. growth relative to the first year.",
        "Population / INITIAL(Population)", [], ["SMOOTH"], "INITIAL(x)"),
    // ---------------------------------------------------------------- lookups
    doc!("WITH LOOKUP", "Lookups", "WITH LOOKUP(input, ((x1,y1), (x2,y2), …))", "A graphical function written directly into the equation.",
        [("input", "The value looked up along the x axis"), ("((x,y), …)", "Points of the curve, in increasing x")],
        "The y value on the line through the points at the input's x; flat beyond the first and last points.",
        "Most of the time a Lookup variable is easier: draw the curve with the mouse and call it like Effect(x).",
        "WITH LOOKUP(Time, ((0,0), (10,5), (20,8)))", [], ["Calling a lookup"], "WITH LOOKUP(input, ((0,0), (1,1)))"),
    doc!("Calling a lookup", "Lookups", "Lookup Name(x)", "Use a Lookup variable like a function.",
        [("x", "The input value, read along the curve's x axis")],
        "The curve's y value at x, joining points with straight lines and staying flat outside the first and last points.",
        "Create a Lookup with the curve tool, leave its Input empty, and write its name followed by the input in brackets in any equation.",
        "Effect of Awareness(Fraction Infected)", ["Lookups express effects that aren't straight lines: saturation, thresholds, diminishing returns."], ["WITH LOOKUP"], ""),
    // ---------------------------------------------------------------- time
    doc!("TIME", "Time", "TIME", "The current simulation time.", [], "Starts at the initial time and goes up by the time step.",
        "", "IF THEN ELSE(TIME > 20, New Policy, Old Policy)", [], ["INITIAL TIME", "FINAL TIME", "TIME STEP", "STEP"], "TIME"),
    doc!("INITIAL TIME", "Time", "INITIAL TIME", "When the run starts.", [], "The value set in the toolbar's “Time … to …” fields.", "", "TIME - INITIAL TIME", [], ["TIME", "FINAL TIME"], "INITIAL TIME"),
    doc!("FINAL TIME", "Time", "FINAL TIME", "When the run ends.", [], "The end of the run.", "", "STEP(10, FINAL TIME / 2)", [], ["TIME", "INITIAL TIME"], "FINAL TIME"),
    doc!("TIME STEP", "Time", "TIME STEP", "The integration step (dt).", [], "How far time moves each calculation step.",
        "Smaller steps are more accurate but slower. A good rule: at most a quarter of the shortest delay in the model.", "Stock / TIME STEP",
        ["Halve the time step: if results barely change, it was small enough."], ["TIME"], "TIME STEP"),
    // ---------------------------------------------------------------- operators
    doc!("+  −  ×  ÷", "Operators", "a + b   a - b   a * b   a / b", "Add, subtract, multiply and divide.", [], "The usual arithmetic; * and / come before + and −.",
        "Use brackets to change the order: (a + b) * c.", "(Births - Deaths) / Population", ["Dividing by zero gives an invalid value; use XIDZ or ZIDZ to guard against it."], ["^", "XIDZ"], "+"),
    doc!("^", "Operators", "a ^ b", "Raise to a power.", [], "a to the power b; −x^2 means −(x²).", "", "Radius ^ 2 * 3.14159", [], ["POWER"], "^"),
    doc!("Comparisons", "Operators", "a < b   a <= b   a > b   a >= b   a = b   a <> b", "Compare two values.", [], "1 when true, 0 when false.",
        "Mostly used inside IF THEN ELSE.", "IF THEN ELSE(Stock >= Capacity, 0, Inflow)", [], ["IF THEN ELSE", ":AND:  :OR:  :NOT:"], ">"),
    doc!(":AND:  :OR:  :NOT:", "Operators", "a :AND: b   a :OR: b   :NOT: a", "Combine conditions (also written && || !).", [], "1 when true, 0 when false.", "",
        "IF THEN ELSE(Time > 10 :AND: Stock < 50, Order, 0)", [], ["Comparisons", "IF THEN ELSE"], ":AND:"),
];

pub const GROUPS: &[&str] = &["Logic", "Math", "Sets", "Test inputs", "Delays", "Lookups", "Time", "Operators"];

pub fn find(name: &str) -> Option<&'static Doc> {
    DOCS.iter().find(|d| d.name.eq_ignore_ascii_case(name))
}

/// State of the reference window.
pub struct Browser {
    pub query: String,
    pub group: Option<&'static str>,
    pub selected: &'static str,
    /// The variable whose equation "Insert" writes into.
    pub target: Option<u64>,
}

impl App {
    pub fn open_functions(&mut self, target: Option<u64>) {
        self.functions = Some(Browser { query: String::new(), group: None, selected: "IF THEN ELSE", target });
    }

    pub fn function_window(&mut self, ctx: &egui::Context) {
        let Some(mut b) = self.functions.take() else { return };
        let pal = self.pal;
        let mut open = true;
        let mut insert: Option<String> = None;
        let target_name = b.target.and_then(|t| self.model.var(t)).map(|v| v.name.clone());
        egui::Window::new("Function reference")
            .open(&mut open)
            .default_size([980.0, 640.0])
            .min_size([720.0, 420.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let r = ui.add(egui::TextEdit::singleline(&mut b.query).hint_text("Search functions, e.g. delay, random, average…").font(FontId::proportional(17.0)).desired_width(380.0));
                    if r.changed() {
                        b.group = None;
                    }
                    ui.add_space(8.0);
                    for g in std::iter::once(None).chain(GROUPS.iter().map(|g| Some(*g))) {
                        let on = b.group == g;
                        if ui.selectable_label(on, RichText::new(g.unwrap_or("All")).size(14.0)).clicked() {
                            b.group = g;
                        }
                    }
                });
                ui.separator();
                let q = b.query.to_lowercase();
                let hits: Vec<&Doc> = DOCS
                    .iter()
                    .filter(|d| b.group.is_none_or(|g| d.group == g))
                    .filter(|d| q.is_empty() || q.split_whitespace().all(|w| d.name.to_lowercase().contains(w) || d.summary.to_lowercase().contains(w) || d.details.to_lowercase().contains(w) || d.group.to_lowercase().contains(w)))
                    .collect();
                if !hits.iter().any(|d| d.name == b.selected) {
                    if let Some(first) = hits.first() {
                        b.selected = first.name;
                    }
                }
                let h = ui.available_height() - 52.0;
                ui.horizontal_top(|ui| {
                    // list
                    ui.vertical(|ui| {
                        ui.set_width(290.0);
                        egui::ScrollArea::vertical().id_salt("fn-list").max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                            let mut group = "";
                            for d in &hits {
                                if d.group != group {
                                    group = d.group;
                                    ui.add_space(6.0);
                                    ui.label(RichText::new(group.to_uppercase()).small().strong().color(pal.muted));
                                }
                                let on = d.name == b.selected;
                                let resp = egui::Frame::new()
                                    .fill(if on { pal.accent_soft } else { Color32::TRANSPARENT })
                                    .corner_radius(CornerRadius::same(7))
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.label(RichText::new(d.name).size(15.0).strong().color(if on { pal.accent } else { pal.text }));
                                        ui.label(RichText::new(d.summary).size(12.5).color(pal.muted));
                                    })
                                    .response
                                    .interact(egui::Sense::click());
                                if resp.clicked() {
                                    b.selected = d.name;
                                }
                                if resp.double_clicked() && b.target.is_some() && !d.insert.is_empty() {
                                    insert = Some(d.insert.to_string());
                                }
                            }
                            if hits.is_empty() {
                                ui.label(RichText::new("Nothing matches").color(pal.muted));
                            }
                        });
                    });
                    ui.separator();
                    // details
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical().id_salt("fn-doc").max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                            let Some(d) = find(b.selected) else { return };
                            ui.label(RichText::new(d.group.to_uppercase()).small().strong().color(pal.accent));
                            ui.label(RichText::new(d.name).size(24.0).strong());
                            ui.add_space(2.0);
                            ui.label(RichText::new(d.summary).size(16.0).color(pal.text));
                            ui.add_space(10.0);
                            code_block(ui, pal, d.signature, 16.0);
                            if !d.params.is_empty() {
                                heading(ui, pal, "Parameters");
                                egui::Grid::new("params").num_columns(2).spacing([18.0, 8.0]).show(ui, |ui| {
                                    for (n, desc) in d.params {
                                        ui.label(RichText::new(*n).monospace().size(14.5).color(pal.accent));
                                        ui.label(RichText::new(*desc).size(14.5));
                                        ui.end_row();
                                    }
                                });
                            }
                            heading(ui, pal, "Result");
                            ui.label(RichText::new(d.returns).size(14.5));
                            if !d.details.is_empty() {
                                heading(ui, pal, "How it works");
                                ui.label(RichText::new(d.details).size(14.5));
                            }
                            equivalents_block(ui, pal, d.name);
                            heading(ui, pal, "Example");
                            code_block(ui, pal, d.example, 14.5);
                            if b.target.is_some() && !d.example.is_empty() && ui.small_button("Insert this example").clicked() {
                                insert = Some(d.example.to_string());
                            }
                            if !d.tips.is_empty() {
                                heading(ui, pal, "Tips");
                                for t in d.tips {
                                    ui.label(RichText::new(format!("•  {t}")).size(14.5));
                                }
                            }
                            if !d.see.is_empty() {
                                heading(ui, pal, "See also");
                                ui.horizontal_wrapped(|ui| {
                                    for s in d.see {
                                        if find(s).is_some() && ui.link(RichText::new(*s).size(14.5)).clicked() {
                                            b.selected = find(s).unwrap().name;
                                        }
                                    }
                                });
                            }
                            if function_info(d.name).is_none() && d.group != "Lookups" && d.group != "Time" && d.group != "Operators" {
                                ui.label(RichText::new("(not a function)").small().color(pal.muted));
                            }
                        });
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let d = find(b.selected);
                    match (&target_name, d) {
                        (Some(name), Some(d)) if !d.insert.is_empty() => {
                            let btn = egui::Button::new(RichText::new(format!("Insert {} into “{name}”", d.name)).strong().color(Color32::WHITE)).fill(pal.accent).corner_radius(CornerRadius::same(8));
                            if ui.add(btn).on_hover_text("Puts a template at the cursor; replace the placeholder words with your variables").clicked() {
                                insert = Some(d.insert.to_string());
                            }
                            ui.label(RichText::new("Double-click a function to insert it").small().color(pal.muted));
                        }
                        _ => {
                            ui.label(RichText::new("Select a variable on the diagram to insert functions into its equation.").color(pal.muted));
                        }
                    }
                });
            });
        if let (Some(text), Some(t)) = (insert, b.target) {
            self.pending_insert = Some((t, text));
            self.status = "Inserted — replace the placeholder words with your own variables".into();
            return; // close after inserting
        }
        if open {
            self.functions = Some(b);
        }
    }
}

/// What a function is called in Vensim and in Stella, from the vendors' documentation (see the
/// wiki page "Vensim and Stella side by side" for sources and what could not be confirmed).
/// (name, Vensim, Stella, what differs)
pub const EQUIVALENTS: &[(&str, &str, &str, &str)] = &[
    ("IF THEN ELSE", "IF THEN ELSE(c, a, b)", "IF c THEN a ELSE b", ""),
    ("MIN", "MIN(a, b)", "MIN(a, b)", ""),
    ("MAX", "MAX(a, b)", "MAX(a, b)", ""),
    ("ABS", "ABS(x)", "ABS(x)", ""),
    ("SQRT", "SQRT(x)", "SQRT(x)", ""),
    ("EXP", "EXP(x)", "EXP(x)", ""),
    ("LN", "LN(x)", "LN(x)", ""),
    ("LOG", "LOG(x, base)", "LOG10(x), or LN(x) / LN(base)", ""),
    ("POWER", "POWER(x, y)", "x ^ y", ""),
    ("INTEGER", "INTEGER(x): truncates toward zero", "INT(x): rounds down, so INT(-8.9) = -9", "Differs for negative numbers. Dynamist truncates, like Vensim."),
    ("ROUND", "not found in the pages read", "ROUND(x)", ""),
    ("MODULO", "MODULO(x, y)", "x MOD y", "Neither says what a negative operand gives."),
    ("SIN", "SIN(x), radians", "SIN(x), radians", ""),
    ("COS", "COS(x)", "COS(x), radians", ""),
    ("TAN", "TAN(x), radians", "TAN(x), radians", ""),
    ("ARCSIN", "ARCSIN(x)", "ARCSIN(x)", ""),
    ("ARCCOS", "ARCCOS(x)", "ARCCOS(x)", ""),
    ("ARCTAN", "ARCTAN(x)", "ARCTAN(x), one argument", ""),
    ("SINH", "SINH(x)", "not found", ""),
    ("COSH", "COSH(x)", "not found", ""),
    ("TANH", "TANH(x)", "not found", ""),
    ("XIDZ", "XIDZ(a, b, x): the test is ABS(b) < 1E-6", "SAFEDIV(a, b, onzero)", "Vensim and Stella treat a tiny divisor as zero; Dynamist only an exact zero."),
    ("ZIDZ", "ZIDZ(a, b): the test is ABS(b) < 1E-6", "SAFEDIV(a, b)", "As XIDZ."),
    ("CEILING", "not found", "not found (INT rounds down)", ""),
    ("FLOOR", "not found", "not found (INT rounds down)", ""),
    ("SIGN", "not found", "not found", ""),
    ("PI", "not found as a function", "PI (no brackets)", ""),
    ("GAMMA", "GAMMA LN(x): only the logarithm", "GAMMALN(x). GAMMA(...) there is a random-number generator", "Write EXP(GAMMALN(x)) in Stella."),
    ("FACTORIAL", "not found", "FACTORIAL(n)", ""),
    ("ERF", "not found", "not found", ""),
    ("CLAMP", "not found", "not found", "MIN(MAX(x, low), high) in both."),
    ("MEDIAN", "not found", "not found", ""),
    ("SINWAVE", "not found", "SINWAVE(amplitude, period): no start; amplitude taken as absolute", "The phase differs when start is not 0."),
    ("COSWAVE", "not found", "COSWAVE(amplitude, period): no start", "As SINWAVE."),
    ("GRAPH", "WITH LOOKUP(x, ((x1, y1), …))", "a graphical variable, read with LOOKUP(graph, x)", ""),
    ("GRAPHSTEP", "WITH LOOKUP (no stepwise form found)", "a graphical variable, read with LOOKUP", ""),
    ("WITH LOOKUP", "WITH LOOKUP(x, ((x1, y1), …))", "LOOKUP(graphical variable, x)", ""),
    ("STEP", "STEP(height, time)", "STEP(height, time), optional duration and interval in newer versions", ""),
    ("PULSE", "PULSE(start, width); a width of 0 counts as one TIME STEP", "PULSE(volume, first, interval): a volume spread over one DT", "Different meaning in Stella: its first argument is a volume, not a start time."),
    ("PULSE TRAIN", "PULSE TRAIN(start, width, interval, end)", "not found", ""),
    ("RAMP", "RAMP(slope, start, end): constant after the end", "RAMP(slope, [time]) in newer versions; older ones also take a stop time", "Check what happens after the end in the version you use."),
    ("RANDOM UNIFORM", "RANDOM UNIFORM(min, max, stream): third argument is a stream number, usually 0", "UNIFORM(min, max, [seed])", "Numbers differ between tools; runs are not reproducible across them."),
    ("RANDOM NORMAL", "RANDOM NORMAL(min, max, mean, sd, stream): values outside the range are redrawn", "NORMAL(mean, stddev, [seed], [min], [max])", "Different argument order in Stella; Dynamist clips where Vensim redraws."),
    ("SMOOTH", "SMOOTH(x, t)", "SMTH1(x, t, [initial])", ""),
    ("SMOOTHI", "SMOOTHI(x, t, initial)", "SMTH1 with the third argument", ""),
    ("SMOOTH3", "SMOOTH3(x, t)", "SMTH3(x, t, [initial])", ""),
    ("SMOOTH3I", "SMOOTH3I(x, t, initial)", "SMTH3 with the third argument", ""),
    ("DELAY1", "DELAY1(x, t)", "DELAY1(x, t, [initial])", ""),
    ("DELAY1I", "DELAY1I (signature not read)", "DELAY1 with the third argument", ""),
    ("DELAY3", "DELAY3(x, t)", "DELAY3(x, t, [initial])", ""),
    ("DELAY3I", "DELAY3I(x, t, initial)", "DELAY3 with the third argument", ""),
    ("DELAY FIXED", "DELAY FIXED(x, t, initial)", "DELAY(x, t, [initial])", "Only the initial value of the delay time is used."),
    ("DELAYINF", "no equivalent: nearest are DELAY INFORMATION(x, t, i) and DELAY N(x, t, i, n)", "DELAYN(x, t, n, [initial]) is the nearest", "Order and meaning need checking."),
    ("DELAYMTR", "no equivalent: nearest is DELAY MATERIAL(x, t, i, m)", "DELAYN(x, t, n, [initial]) is the nearest", "As DELAYINF."),
    ("TREND", "TREND(x, t, initial trend): no default", "TREND(x, t, [initial]): default 0", ""),
    ("ALLOCATE AVAILABLE ITEM", "ALLOCATE AVAILABLE(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("ALLOCATE BY PRIORITY ITEM", "ALLOCATE BY PRIORITY(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("VECTOR SORT ORDER ITEM", "VECTOR SORT ORDER(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("VECTOR RANK ITEM", "VECTOR RANK(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("VECTOR REORDER ITEM", "VECTOR REORDER(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("VECTOR ELM MAP ITEM", "VECTOR ELM MAP(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("VECTOR SELECT ITEMS", "VECTOR SELECT(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("INVERT MATRIX ITEM", "INVERT MATRIX(…)", "no direct equivalent", "written out element by element when a Vensim model is read"),
    ("LOOKUP SLOPE", "LOOKUP SLOPE(lookup, x, m)", "no direct equivalent", ""),
    ("LOOKUP AREA", "LOOKUP AREA(lookup, start, end)", "no direct equivalent", ""),
    ("GET DATA AT TIME", "GET DATA AT TIME(data, time)", "no direct equivalent", ""),
    ("GET DATA BETWEEN TIMES", "GET DATA BETWEEN TIMES(data, time, mode)", "no direct equivalent", ""),
    ("GET DATA FIRST TIME", "GET DATA FIRST TIME(data)", "no direct equivalent", ""),
    ("GET DATA LAST TIME", "GET DATA LAST TIME(data)", "no direct equivalent", ""),
    ("GET DATA MAX", "GET DATA MAX(data, start, end)", "no direct equivalent", ""),
    ("GET DATA MIN", "GET DATA MIN(data, start, end)", "no direct equivalent", ""),
    ("GET DATA MEAN", "GET DATA MEAN(data, start, end)", "no direct equivalent", ""),
    ("GET DATA MEDIAN", "GET DATA MEDIAN(data, start, end)", "no direct equivalent", ""),
    ("GET DATA STDV", "GET DATA STDV(data, start, end)", "no direct equivalent", ""),
    ("GET DATA TOTAL", "GET DATA TOTAL(data, start, end)", "no direct equivalent", ""),
    ("NPVE", "NPVE(stream, discount rate, initial value, factor)", "no direct equivalent", ""),
    ("DERIVN", "DERIVN(x, order)", "no direct equivalent", ""),
    ("EXCELDATESERIAL", "EXCELDATESERIAL(year, month, day)", "no direct equivalent", ""),
    ("EXCELDATESERIAL YEAR", "EXCELDATESERIAL YEAR(serial)", "no direct equivalent", ""),
    ("EXCELDATESERIAL MONTH OF YEAR", "EXCELDATESERIAL MONTH OF YEAR(serial)", "no direct equivalent", ""),
    ("EXCELDATESERIAL DAY OF MONTH", "EXCELDATESERIAL DAY OF MONTH(serial)", "no direct equivalent", ""),
    ("EXCELDATESERIAL DAY OF YEAR", "EXCELDATESERIAL DAY OF YEAR(serial)", "no direct equivalent", ""),
    ("VECTOR LOOKUP ITEMS", "VECTOR LOOKUP ITEMS(n, y…, x, xmin, xmax, mode)", "no direct equivalent", ""),
    ("EXPECT", "EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)", "no direct equivalent", ""),
    ("LOOK", "LOOK(curve, x, mode=lookup|forward|backward|inverse|extrapolate|slope|area, start=, outside=)", "no direct equivalent", ""),
    ("INTERNAL RATE OF RETURN", "INTERNAL RATE OF RETURN(stream, period, initial)", "no direct equivalent", ""),
    ("AGE", "AGE(stock, every=, entrants=, open_ended=)", "SHIFT IF TRUE in Vensim (changes the stock itself); no direct equivalent in Stella (a conveyor or a chain)", "Dynamist writes the class's ageing flow as a formula when a model is exported."),
    ("QUEUE", "QUEUE(arrivals, served, init=)", "no direct equivalent", ""),
    ("WAIT", "WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)", "no direct equivalent", ""),
    ("QUEUE FIFO", "QUEUE FIFO(inflow, outflow, initial)", "no direct equivalent", ""),
    ("QUEUE AGE AVERAGE", "QUEUE AGE AVERAGE(queue, [n])", "no direct equivalent", ""),
    ("QUEUE AGE OLDEST", "QUEUE AGE OLDEST(queue)", "no direct equivalent", ""),
    ("QUEUE AGE IN RANGE", "QUEUE AGE IN RANGE(queue, min age, max age)", "no direct equivalent", ""),
    ("VALUE", "VALUE(stream, rate, init=, factor=, timing=)", "NPV(stream, rate, init, factor) and NPVE for period_end", "Dynamist writes the matching function when a model is exported."),
    ("ALLOCATE", "ALLOCATE(requests, supply, by=, overlap=, curve=, spread=)", "ALLOCATE BY PRIORITY (strict order) and ALLOCATE AVAILABLE (curves) in Vensim; no direct equivalent in Stella", "Dynamist writes the Vensim function when a model is exported."),
    ("DELAY", "DELAY(x, time, [order], init=, kind=, exact=)", "DELAY1 / DELAY3 / DELAYN / DELAY FIXED (material), SMOOTH / SMOOTH3 / SMOOTH N (information)", "Dynamist writes the matching function when a model is exported."),
    ("NPV", "NPV(stream, discount rate, initial value, factor)", "NPV(stream, discount rate, [initial], [factor])", "Stella's NPV discounts per period; check the timing."),
    ("DEPRECIATE STRAIGHTLINE", "DEPRECIATE STRAIGHTLINE(input, time, fiscal, initial)", "no direct equivalent", ""),
    ("GAMMA LN", "GAMMA LN(x)", "GAMMALN(x)", ""),
    ("RANDOM EXPONENTIAL", "RANDOM EXPONENTIAL(min, max, shift, stretch, seed)", "EXPRND(mean, [seed])", "Different streams; same distribution."),
    ("LOOKUP FORWARD", "LOOKUP FORWARD(lookup, x)", "a discrete graphical function", ""),
    ("LOOKUP BACKWARD", "LOOKUP BACKWARD(lookup, x)", "a discrete graphical function", ""),
    ("LOOKUP INVERT", "LOOKUP INVERT(lookup, y)", "LOOKUPINV(gf, y)", ""),
    ("LOOKUP EXTRAPOLATE", "LOOKUP EXTRAPOLATE(lookup, x)", "an extrapolating graphical function", ""),
    ("SAMPLE IF TRUE", "SAMPLE IF TRUE(condition, input, initial)", "no direct equivalent", ""),
    ("QUANTUM", "QUANTUM(x, q)", "no direct equivalent: q * INT(x / q)", ""),
    ("ACTIVE INITIAL", "ACTIVE INITIAL(x, initial value)", "no equivalent: x with a separate initial value", "Stella has no direct counterpart."),
    ("INITIAL", "INITIAL(x)", "INIT(variable): a single variable only", ""),
    ("TOTAL", "SUM(x[range!])", "SUM(x[*])", ""),
    ("AVERAGE", "SUM(x[range!]) / ELMCOUNT(range)", "MEAN(x[*])", ""),
    ("PMT", "not found", "not found", "Excel's PMT, with Excel's sign convention (paid out is negative)."),
    ("IPMT", "not found", "not found", "Excel's IPMT."),
    ("PPMT", "not found", "not found", "Excel's PPMT."),
    ("FV", "not found", "not found", "Excel's FV."),
    ("PV", "not found", "not found", "Excel's PV."),
    ("NPER", "not found", "not found", "Excel's NPER."),
    ("RATE", "not found", "not found", "Excel's RATE."),
    ("LEVELISED", "no direct equivalent: NPV of the costs divided by NPV of the quantities", "no direct equivalent: NPV of the costs divided by NPV of the quantities", "Alias LCOE."),
    ("LOAN", "no direct equivalent", "no direct equivalent", "Balance, payment, interest and principal of a loan with tranches, a grace period and a term."),
    ("TAX", "no direct equivalent", "no direct equivalent", "Tax on a taxable income with losses carried forward."),
    ("PAYBACK", "no direct equivalent", "no direct equivalent", ""),
    ("NPV AT", "no direct equivalent: NPV with an offset in periods", "no direct equivalent", ""),
    ("WACC", "not found", "not found", ""),
    ("REAL RATE", "not found", "not found", "(1 + nominal) / (1 + inflation) - 1."),
    ("NOMINAL RATE", "not found", "not found", "(1 + real) * (1 + inflation) - 1."),
    ("EFFECTIVE RATE", "not found", "not found", "(1 + nominal / m) ^ m - 1. Alias EFFECT."),
    ("ANNUITY FACTOR", "not found", "not found", "Alias CRF."),
    ("YEAR FRACTION", "not found", "not found", ""),
    ("DAYS IN STEP", "not found", "not found", ""),
    ("NORMAL INV", "not found", "not found", "The inverse of the cumulative normal distribution; not a random draw."),
    ("NORMAL CDF", "not found", "not found", "The cumulative normal distribution; not a random draw."),
    ("RANDOM TRIANGULAR", "not confirmed", "not confirmed", "Draws are not reproducible across tools."),
];

/// What the same function is called in Excel (from Excel's documentation), for the functions that have one.
/// Only functions that exist in Excel are listed; a text starting "no single function" says how to build it.
pub const EXCEL: &[(&str, &str)] = &[
    ("IF THEN ELSE", "IF(c, a, b)"),
    ("MIN", "MIN(a, b)"),
    ("MAX", "MAX(a, b)"),
    ("ABS", "ABS(x)"),
    ("SQRT", "SQRT(x)"),
    ("EXP", "EXP(x)"),
    ("LN", "LN(x)"),
    ("LOG", "LOG(x, base)"),
    ("LOG10", "LOG10(x)"),
    ("POWER", "POWER(x, y), or x ^ y"),
    ("INTEGER", "TRUNC(x): toward zero (INT rounds down)"),
    ("ROUND", "ROUND(x, digits)"),
    ("MODULO", "MOD(x, y): the sign follows the divisor"),
    ("SIN", "SIN(x), radians"),
    ("COS", "COS(x), radians"),
    ("TAN", "TAN(x), radians"),
    ("ARCSIN", "ASIN(x)"),
    ("ARCCOS", "ACOS(x)"),
    ("ARCTAN", "ATAN(x)"),
    ("ARCTAN2", "ATAN2(x, y): Excel takes x first"),
    ("SINH", "SINH(x)"),
    ("COSH", "COSH(x)"),
    ("TANH", "TANH(x)"),
    ("FLOOR", "FLOOR.MATH(x)"),
    ("CEILING", "CEILING.MATH(x)"),
    ("SIGN", "SIGN(x)"),
    ("PI", "PI()"),
    ("ERF", "ERF(x)"),
    ("FACTORIAL", "FACT(n)"),
    ("GAMMA", "EXP(GAMMALN(x))"),
    ("GAMMA LN", "GAMMALN(x)"),
    ("MEAN", "AVERAGE(...)"),
    ("MEDIAN", "MEDIAN(...)"),
    ("RADIANS", "RADIANS(x)"),
    ("DEGREES", "DEGREES(x)"),
    ("AND", "AND(a, b)"),
    ("OR", "OR(a, b)"),
    ("NOT", "NOT(a)"),
    ("PMT", "PMT(rate, nper, pv, [fv], [type])"),
    ("IPMT", "IPMT(rate, per, nper, pv, [fv], [type])"),
    ("PPMT", "PPMT(rate, per, nper, pv, [fv], [type])"),
    ("FV", "FV(rate, nper, pmt, [pv], [type])"),
    ("PV", "PV(rate, nper, pmt, [fv], [type])"),
    ("NPER", "NPER(rate, pmt, pv, [fv], [type])"),
    ("RATE", "RATE(nper, pmt, pv, [fv], [type], [guess])"),
    ("NPV", "NPV(rate, values): discounts the first value by one period"),
    ("NPVE", "NPV(rate, values)"),
    ("INTERNAL RATE OF RETURN", "IRR(values)"),
    ("EFFECTIVE RATE", "EFFECT(nominal, m)"),
    ("NOMINAL RATE", "NOMINAL(effective, m)"),
    ("LEVELISED", "no single function: NPV of the costs divided by NPV of the quantities"),
    ("LOAN", "no single function: PMT, IPMT and PPMT for each tranche"),
    ("PAYBACK", "no single function: cumulate the cash flows and look for the sign change"),
    ("ANNUITY FACTOR", "no single function: -PMT(rate, n, 1)"),
    ("NORMAL CDF", "NORM.DIST(x, mean, sd, TRUE)"),
    ("NORMAL INV", "NORM.INV(p, mean, sd)"),
    ("LOGNORMAL CDF", "LOGNORM.DIST(x, mu, sigma, TRUE)"),
    ("LOGNORMAL INV", "LOGNORM.INV(p, mu, sigma)"),
    ("POISSON", "POISSON.DIST(k, mean, FALSE) gives the probability; Dynamist draws a random number"),
    ("EXCELDATESERIAL", "DATE(year, month, day)"),
    ("EXCELDATESERIAL YEAR", "YEAR(serial)"),
    ("EXCELDATESERIAL MONTH OF YEAR", "MONTH(serial)"),
    ("EXCELDATESERIAL DAY OF MONTH", "DAY(serial)"),
    ("DATE", "DATE(year, month, day)"),
    ("YEAR", "YEAR(serial)"),
    ("MONTH", "MONTH(serial)"),
    ("DAY OF MONTH", "DAY(serial)"),
    ("DAY OF WEEK", "WEEKDAY(serial)"),
    ("WEEK NUMBER", "WEEKNUM(serial)"),
    ("WORKDAYS BETWEEN", "NETWORKDAYS(start, end)"),
    ("YEAR FRACTION", "YEARFRAC(start, end, [basis])"),
    ("RANDOM UNIFORM", "RANDBETWEEN(min, max) for whole numbers; min + (max - min) * RAND()"),
    ("RANDOM NORMAL", "NORM.INV(RAND(), mean, sd)"),
    ("TRIANGULAR", "no single function: invert the triangular distribution on RAND()"),
];

/// The Excel equivalent of a function, if it has one.
pub fn excel_equivalent(name: &str) -> Option<&'static str> {
    EXCEL.iter().find(|e| e.0.eq_ignore_ascii_case(name)).map(|e| e.1)
}

pub fn equivalents(name: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    EQUIVALENTS.iter().find(|e| e.0.eq_ignore_ascii_case(name))
}

/// The rows "In Excel", "In Vensim", "In Stella" of a function page (only the tools that have an entry).
pub fn equivalents_block(ui: &mut egui::Ui, pal: crate::theme::Palette, name: &str) {
    let excel = excel_equivalent(name);
    let other = equivalents(name);
    if excel.is_none() && other.is_none() {
        return;
    }
    heading(ui, pal, "In Excel, Vensim and Stella");
    let mut row = |ui: &mut egui::Ui, tool: &str, text: &str| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tool).size(14.5).strong());
            ui.label(RichText::new(text).monospace().size(14.0));
        });
    };
    row(ui, "In Excel", excel.unwrap_or("not found"));
    if let Some((_, vensim, stella, note)) = other {
        row(ui, "In Vensim", vensim);
        row(ui, "In Stella", stella);
        if !note.is_empty() {
            ui.label(RichText::new(*note).size(14.0).color(pal.aux));
        }
    }
}

fn heading(ui: &mut egui::Ui, pal: crate::theme::Palette, t: &str) {
    ui.add_space(12.0);
    ui.label(RichText::new(t).size(13.0).strong().color(pal.muted));
    ui.add_space(2.0);
}

fn code_block(ui: &mut egui::Ui, pal: crate::theme::Palette, code: &str, size: f32) {
    egui::Frame::new()
        .fill(pal.surface)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(code).monospace().size(size).color(pal.text));
        });
}

/// Every built-in function has documentation.
#[cfg(test)]
mod tests {
    #[test]
    fn every_function_is_documented() {
        for f in crate::expr::FUNCTIONS {
            assert!(super::find(f.name).is_some(), "{} has no documentation", f.name);
        }
        for d in super::DOCS {
            for s in d.see {
                assert!(super::find(s).is_some(), "{} refers to unknown {}", d.name, s);
            }
        }
    }
}

#[cfg(test)]
mod equivalents_tests {
    use super::*;

    #[test]
    fn every_equivalent_belongs_to_a_documented_name() {
        for (name, _, _, _) in EQUIVALENTS {
            assert!(DOCS.iter().any(|d| d.name.eq_ignore_ascii_case(name)), "“{name}” is in the Vensim/Stella table but has no documentation entry");
        }
    }

    #[test]
    fn the_traps_are_flagged() {
        for name in ["PULSE", "GAMMA", "INTEGER", "RANDOM NORMAL"] {
            let e = equivalents(name).unwrap();
            assert!(!e.3.is_empty(), "{name} differs between the products and must say so");
        }
    }
}
