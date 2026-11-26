//! Names of the five kinds of variable, for menus.

use crate::model::Kind;

pub fn kind_options() -> [Kind; 5] {
    [Kind::Stock, Kind::Flow, Kind::Aux, Kind::Constant, Kind::Lookup]
}

pub fn kind_label(k: Kind) -> &'static str {
    k.label()
}
