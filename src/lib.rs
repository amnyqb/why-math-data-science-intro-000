//! The Dynamist application as a library: the egui drawing code and `App`.
//!
//! With the default `gui` feature the `dynamist` binary (eframe window) is built from it. Without it
//! (`default-features = false`) nothing here needs a window, a display or a GUI backend: `headless`
//! drives `App` through an `egui::Context` fed a fake `RawInput`, and `svg` turns the shapes it
//! paints into SVG. That is what `dyn render` and `dyn chart` use.

pub mod app;
pub use dynamist_core::{checks, expr, mdl, model, sets, sim, units, userfn, validate};
pub mod checks_ui;
pub mod architecture;
pub mod layers_ui;
pub mod health_ui;
pub mod convert_ui;
pub mod history_ui;
pub mod modules_ui;
#[cfg(test)]
pub mod corpus_tests;
#[cfg(test)]
pub mod xmile_corpus_tests;
#[cfg(test)]
pub mod mdl_corpus_tests;
pub mod sets_ui;
pub mod userfn_ui;
pub mod arrange;
pub mod canvas;
pub mod docs;
pub mod fnjson;
pub mod entities;
pub mod charts;
pub mod chartstyle;
pub mod controls_ui;
pub mod cld;
pub mod export;
pub mod forecast;
pub mod icons;
pub mod inbox;
pub mod inspector;
pub mod levers;
pub mod modfile;
pub mod optimize;
pub mod outline;
pub mod palette;
pub mod pictures;
pub mod ribbon;
pub mod shapes_ui;
pub mod pyfn;
pub mod results;
pub mod scenarios;
pub mod sens;
pub mod sheets;
pub mod spreadsheet;
pub mod theme;

pub mod headless;
pub mod svg;

/// Stand-in for `rfd` in the headless build: no dialog is ever shown, every pick is "cancelled".
#[cfg(not(feature = "gui"))]
pub mod rfd {
    use std::path::PathBuf;
    pub struct FileDialog;
    impl FileDialog {
        pub fn new() -> Self {
            FileDialog
        }
        pub fn add_filter(self, _name: impl Into<String>, _ext: &[impl ToString]) -> Self {
            self
        }
        pub fn set_file_name(self, _name: impl Into<String>) -> Self {
            self
        }
        pub fn pick_file(self) -> Option<PathBuf> {
            None
        }
        pub fn save_file(self) -> Option<PathBuf> {
            None
        }
    }
}
