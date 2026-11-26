//! What a conversion could not carry over. Every format reader and writer returns one, so a
//! round trip never silently drops or bends anything: an empty report means nothing was lost.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LossKind {
    /// Left out: the target has no place for it.
    Dropped,
    /// Carried over in a different form (flattened, renamed, rewritten).
    Approximated,
    /// Kept as a note or an opaque block so that writing the same format again restores it.
    Preserved,
    /// Written out one piece at a time because the target cannot express the structure (a set
    /// with parents written as one variable per item): the numbers agree, the structure is gone.
    Flattened,
    /// Kept in the file so it survives, but the engine cannot run it (a vendor-only function).
    PreservedNotSimulable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LossItem {
    pub kind: LossKind,
    /// What it was: "dimension mapping", "function ALLOCATE BY PRIORITY", "view 2" …
    pub construct: String,
    /// Where it was and what happened to it, in words a modeller can act on.
    pub detail: String,
    /// The variable, set or place it concerns, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LossReport {
    /// The format read from or the model written from ("mdl", "sdm" …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The format written to or read into.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub items: Vec<LossItem>,
}

impl LossReport {
    pub fn add(&mut self, kind: LossKind, construct: impl Into<String>, detail: impl Into<String>) {
        self.items.push(LossItem { kind, construct: construct.into(), detail: detail.into(), location: None });
    }
    pub fn flattened(&mut self, construct: impl Into<String>, detail: impl Into<String>) {
        self.add(LossKind::Flattened, construct, detail);
    }
    pub fn preserved_not_simulable(&mut self, construct: impl Into<String>, detail: impl Into<String>) {
        self.add(LossKind::PreservedNotSimulable, construct, detail);
    }
    /// Say where the last item applies.
    pub fn at(&mut self, location: impl Into<String>) {
        if let Some(i) = self.items.last_mut() {
            i.location = Some(location.into());
        }
    }
    pub fn dropped(&mut self, construct: impl Into<String>, detail: impl Into<String>) {
        self.add(LossKind::Dropped, construct, detail);
    }
    pub fn approximated(&mut self, construct: impl Into<String>, detail: impl Into<String>) {
        self.add(LossKind::Approximated, construct, detail);
    }
    pub fn preserved(&mut self, construct: impl Into<String>, detail: impl Into<String>) {
        self.add(LossKind::Preserved, construct, detail);
    }
    /// True when nothing was dropped, approximated, flattened or left unrunnable (a preserved
    /// note does not count as loss).
    pub fn is_lossless(&self) -> bool {
        self.items.iter().all(|i| i.kind == LossKind::Preserved)
    }
    pub fn summary(&self) -> String {
        if self.items.is_empty() {
            return "No loss.".into();
        }
        self.items.iter().map(|i| format!("{:?}: {}{}: {}", i.kind, i.construct, i.location.as_ref().map(|l| format!(" at {l}")).unwrap_or_default(), i.detail)).collect::<Vec<_>>().join("\n")
    }
}
