//! Rules that are raised by the unit checks (`units_rules`) and the base checks (`base`) rather than by a function of their
//! own. They are listed here so that the rule table is the whole catalogue (and the documentation is generated from it).

use super::{Ctx, Finding, FnRule, Rule, Stage};

fn raised_elsewhere(_: &Ctx) -> Vec<Finding> {
    Vec::new()
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    let row = |id: &'static str, stage: Stage, about: &'static str| -> Box<dyn Rule> { Box::new(FnRule { id, stage, about, f: raised_elsewhere }) };
    vec![
        row("U1", Stage::C3, "Dimension: quantities of different kinds added or compared, or an equation that gives other units than declared (code unit-dimension-mismatch, unit-declared)."),
        row("U2", Stage::C3, "Scale: the same kind of quantity in different scales, with the exact conversion factor as the fix (code unit-scale-mismatch)."),
        row("U3", Stage::C3, "Percent and ppm: how a constant declared in % or ppm is read, or what units_scale_values would change (code unit-percent, unit-scale-change)."),
        row("U4", Stage::C3, "A flow times the time unit that is not its stock's unit (code unit-flow)."),
        row("U5", Stage::C3, "Real and nominal money, or two price years, added together (code money-real-nominal)."),
        row("U6", Stage::C3, "An ambiguous unit symbol (MB, ton, gallon): which meaning is meant (code unit-ambiguous)."),
        row("U7", Stage::C3, "A unit that cannot be read, with the closest known unit as a suggestion (code unit-parse)."),
        row("U8", Stage::C3, "A variable with no units whose equation gives them (code units-inferred)."),
        row("F4", Stage::C4, "A number other than 0 and 1 written into an equation (code magic-number)."),
        row("N3", Stage::C4, "Halving the time step changes a stock by more than 1% (code dt-sensitivity)."),
        row("N5", Stage::C4, "A time constant below the time step: the fastest loop of the model is faster than the clock (raised together with N2)."),
        row("N6", Stage::C4, "Time compared for equality with a time step that is not a power of 1/2 (code time-equality)."),
    ]
}
