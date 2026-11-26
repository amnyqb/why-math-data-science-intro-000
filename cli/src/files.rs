//! Opening and saving model files for the command line: any format in, any format out.

use dynamist_core::io::{self, Format};
use dynamist_core::loss::LossReport;
use dynamist_core::model::Model;
use dynamist_core::ops::LogEntry;

pub fn open(path: &str) -> Result<(Model, LossReport), String> {
    let text = io::read_text(std::path::Path::new(path)).map_err(|e| format!("{path}: {e}"))?;
    let format = io::detect(path, &text).ok_or_else(|| format!("{path}: cannot tell the format (.sdm, .stmx/.xmile or .mdl)"))?;
    io::read_str(&text, format)
}

/// Write a model; the format comes from `format` or the file's extension. Returns what was lost.
pub fn save(m: &Model, path: &str, format: Option<&str>) -> Result<LossReport, String> {
    let f = match format {
        Some(n) => Format::from_name(n).ok_or_else(|| format!("unknown format “{n}”"))?,
        None => io::detect(path, "").unwrap_or(Format::Sdm),
    };
    let (text, loss) = io::write_str(m, f)?;
    std::fs::write(path, text).map_err(|e| format!("{path}: {e}"))?;
    Ok(loss)
}

/// Append a committed batch to `FILE.oplog.jsonl` next to the model.
pub fn append_log(path: &str, entry: &LogEntry) -> Result<(), String> {
    use std::io::Write;
    let log = format!("{path}.oplog.jsonl");
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&log).map_err(|e| format!("{log}: {e}"))?;
    writeln!(f, "{}", serde_json::to_string(entry).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
