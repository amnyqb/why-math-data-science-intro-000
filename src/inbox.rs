//! The Inbox window: messages between the person, the builder and assistants in other projects.
//! Everything lives in the wiki as plain Markdown (`INBOX.md`, `OUTBOX.md`), so any conversation
//! that can read and write files can take part.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use egui::{Color32, RichText};

use crate::app::App;
use crate::icons::Ic;
use crate::spreadsheet::WIKI_DIR;

const KINDS: [&str; 5] = ["model request", "question", "bug", "feature", "review notes"];

pub fn inbox_path() -> PathBuf {
    Path::new(WIKI_DIR).join("INBOX.md")
}

pub fn outbox_path() -> PathBuf {
    Path::new(WIKI_DIR).join("OUTBOX.md")
}

/// One message: its `###` heading and the lines under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Msg {
    pub title: String,
    pub lines: Vec<String>,
}

impl Msg {
    /// The value of a `**Key:** value` line, if any.
    pub fn field(&self, key: &str) -> Option<String> {
        let tag = format!("**{key}:**");
        self.lines.iter().rev().find_map(|l| l.trim().strip_prefix(&tag).map(|v| v.trim().to_string()))
    }
    pub fn status(&self) -> String {
        self.field("Status").unwrap_or_else(|| "open".into()).to_lowercase()
    }
}

/// The messages between heading `start` and the next `## ` heading.
pub fn section(doc: &str, start: &str) -> Vec<Msg> {
    let mut out: Vec<Msg> = Vec::new();
    let mut inside = false;
    for line in doc.lines() {
        if line.starts_with("## ") {
            inside = line.trim() == start;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(t) = line.strip_prefix("### ") {
            out.push(Msg { title: t.trim().to_string(), lines: Vec::new() });
        } else if let Some(m) = out.last_mut() {
            if line.trim() != "---" {
                m.lines.push(line.to_string());
            }
        }
    }
    for m in &mut out {
        while m.lines.last().is_some_and(|l| l.trim().is_empty()) {
            m.lines.pop();
        }
    }
    out
}

/// Every message in the outbox (newest first).
pub fn outbox_messages(doc: &str) -> Vec<Msg> {
    let mut out: Vec<Msg> = Vec::new();
    for line in doc.lines() {
        if let Some(t) = line.strip_prefix("### ") {
            out.push(Msg { title: t.trim().to_string(), lines: Vec::new() });
        } else if let Some(m) = out.last_mut() {
            if line.trim() != "---" {
                m.lines.push(line.to_string());
            }
        }
    }
    for m in &mut out {
        while m.lines.last().is_some_and(|l| l.trim().is_empty()) {
            m.lines.pop();
        }
    }
    out
}

/// Add `block` at the end of *Open messages* (before the `---` that closes the section).
pub fn add_to_inbox(doc: &str, block: &str) -> String {
    let doc = doc.replacen("## Open messages\n\n*(none yet)*\n", "## Open messages\n", 1);
    let Some(open) = doc.find("## Open messages") else {
        return format!("{}\n\n## Open messages\n\n{block}\n", doc.trim_end());
    };
    let rest = &doc[open..];
    let end = rest.find("\n## ").map(|i| open + i).unwrap_or(doc.len());
    // step back over the closing `---` and blank lines
    let mut cut = end;
    let head = doc[..end].trim_end();
    if head.ends_with("---") {
        cut = head.len() - 3;
    }
    let before = doc[..cut].trim_end();
    let after = &doc[cut..];
    format!("{before}\n\n{block}\n\n{}", after.trim_start_matches('\n'))
}

/// Add `block` at the top of the outbox (newest first).
pub fn add_to_outbox(doc: &str, block: &str) -> String {
    let doc = doc.replacen("*(none yet)*\n", "", 1);
    match doc.find("\n---\n") {
        Some(i) => format!("{}\n---\n\n{block}\n\n---\n\n{}", &doc[..i], doc[i + 5..].trim_start_matches('\n').trim_start_matches("---\n").trim_start_matches('\n')),
        None => format!("{}\n\n---\n\n{block}\n", doc.trim_end()),
    }
}

/// Today as YYYY-MM-DD (UTC).
pub fn today() -> String {
    let days = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    // days → civil date (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

/// The note to paste into another conversation so it can join in.
pub fn note_for_other_conversation() -> String {
    let w = WIKI_DIR;
    format!(
        "Dynamist keeps a shared wiki at `{w}`.\n\
         Read `README.md` and `Spreadsheet-Template.md` there first.\n\
         To ask the Dynamist builder for something, add a message at the end of *Open messages* in `{w}/INBOX.md` \
         (copy the block shown at the top of that page) and put any files in `{w}/inbox/`.\n\
         Read the builder's answers under your message in INBOX.md and in `{w}/OUTBOX.md`.\n\
         Don't edit the app's source; only the wiki."
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Open,
    Handled,
    Outbox,
    Write,
}

pub struct Inbox {
    pub tab: Tab,
    pub open_msgs: Vec<Msg>,
    pub handled: Vec<Msg>,
    pub outbox: Vec<Msg>,
    pub title: String,
    pub from: String,
    pub kind: usize,
    pub files: String,
    pub text: String,
    /// true: to the builder (INBOX.md); false: to other projects (OUTBOX.md).
    pub to_builder: bool,
    pub note: Option<(String, bool)>,
    stamp: Option<(SystemTime, SystemTime)>,
    checked: Option<Instant>,
}

impl Default for Inbox {
    fn default() -> Self {
        Inbox {
            tab: Tab::Open,
            open_msgs: Vec::new(),
            handled: Vec::new(),
            outbox: Vec::new(),
            title: String::new(),
            from: "Amin".into(),
            kind: 1,
            files: String::new(),
            text: String::new(),
            to_builder: true,
            note: None,
            stamp: None,
            checked: None,
        }
    }
}

fn modified(p: &Path) -> SystemTime {
    std::fs::metadata(p).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)
}

impl Inbox {
    /// Re-read the files when they changed (checked at most every two seconds).
    pub fn refresh(&mut self, force: bool) {
        if !force && self.checked.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            return;
        }
        self.checked = Some(Instant::now());
        let stamp = (modified(&inbox_path()), modified(&outbox_path()));
        if !force && self.stamp == Some(stamp) {
            return;
        }
        self.stamp = Some(stamp);
        let inbox = std::fs::read_to_string(inbox_path()).unwrap_or_default();
        self.open_msgs = section(&inbox, "## Open messages");
        self.handled = section(&inbox, "## Handled");
        self.outbox = outbox_messages(&std::fs::read_to_string(outbox_path()).unwrap_or_default());
    }

    /// Messages that still want something from someone.
    pub fn waiting(&self) -> usize {
        self.open_msgs.iter().filter(|m| m.status() != "done").count()
    }

    fn send(&mut self) -> Result<String, String> {
        let title = self.title.trim();
        let text = self.text.trim();
        if title.is_empty() || text.is_empty() {
            return Err("Give the message a title and some text.".into());
        }
        let mut block = format!("### {} — {title}\n**From:** {}\n", today(), self.from.trim());
        if self.to_builder {
            block.push_str(&format!("**Kind:** {}\n", KINDS[self.kind]));
        }
        if !self.files.trim().is_empty() {
            block.push_str(&format!("**Files:** {}\n", self.files.trim()));
        }
        block.push_str(&format!("**Message:**\n{text}\n\n**Status:** open"));
        let (path, add): (PathBuf, fn(&str, &str) -> String) = if self.to_builder { (inbox_path(), add_to_inbox) } else { (outbox_path(), add_to_outbox) };
        let doc = std::fs::read_to_string(&path).unwrap_or_else(|_| if self.to_builder { "# Inbox\n\n## Open messages\n\n---\n".into() } else { "# Outbox\n\n---\n".into() });
        std::fs::write(&path, add(&doc, &block)).map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
        self.title.clear();
        self.text.clear();
        self.files.clear();
        self.refresh(true);
        Ok(if self.to_builder {
            "Sent to the builder. In the builder's conversation, say “check the inbox”.".into()
        } else {
            "Put in the outbox. In the other conversation, say “check the Dynamist outbox”.".into()
        })
    }
}

/// Markdown → plain text for reading: drop `**` and backticks, show links by their text.
fn plain(t: &str) -> String {
    let t = t.replace("**", "").replace('`', "");
    let mut out = String::new();
    let mut rest = t.as_str();
    while let Some(i) = rest.find('[') {
        let after = &rest[i + 1..];
        match after.find("](").and_then(|j| after[j..].find(')').map(|k| (j, j + k))) {
            Some((j, k)) => {
                out.push_str(&rest[..i]);
                out.push_str(&after[..j]);
                rest = &after[k + 1..];
            }
            None => {
                out.push_str(&rest[..=i]);
                rest = after;
            }
        }
    }
    out + rest
}

fn status_color(status: &str, pal: &crate::theme::Palette) -> Color32 {
    match status {
        "done" => pal.ok,
        "needs answer" => pal.aux,
        "in progress" => pal.accent,
        _ => pal.stock,
    }
}

fn message_card(ui: &mut egui::Ui, m: &Msg, pal: &crate::theme::Palette) {
    let st = m.status();
    egui::Frame::new().fill(pal.surface).stroke(egui::Stroke::new(1.0, pal.border)).corner_radius(8).inner_margin(10).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(&m.title).strong().size(14.5).color(pal.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!(" {st} ")).small().strong().color(Color32::WHITE).background_color(status_color(&st, pal)));
            });
        });
        ui.add_space(2.0);
        for l in &m.lines {
            let t = l.trim();
            if t.is_empty() {
                ui.add_space(3.0);
            } else if let Some((k, v)) = t.strip_prefix("**").and_then(|r| r.split_once(":**")) {
                if k == "Status" {
                    continue;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(RichText::new(format!("{k}:")).strong().color(if k.starts_with("Reply") { pal.accent } else { pal.muted }));
                    ui.label(RichText::new(plain(v.trim())).color(pal.text));
                });
            } else {
                ui.label(RichText::new(plain(t)).color(pal.text));
            }
        }
    });
    ui.add_space(6.0);
}

impl App {
    pub fn open_inbox(&mut self, tab: Tab) {
        let mut b = self.inbox.take().unwrap_or_default();
        b.tab = tab;
        b.refresh(true);
        self.inbox = Some(b);
    }

    /// Number of open messages, for the badge in the top bar.
    pub fn inbox_waiting(&mut self) -> usize {
        self.inbox_badge.refresh(false);
        self.inbox_badge.waiting()
    }

    pub fn inbox_window(&mut self, ctx: &egui::Context) {
        let Some(mut b) = self.inbox.take() else { return };
        b.refresh(false);
        let pal = self.pal;
        let mut open = true;
        let mut status: Option<String> = None;
        egui::Window::new("Inbox").open(&mut open).collapsible(false).default_size([640.0, 560.0]).default_pos(ctx.content_rect().center() - egui::vec2(320.0, 290.0)).show(ctx, |ui| {
            ui.label(RichText::new("Messages between you, the builder and conversations in other projects. They are kept in the wiki, so any assistant that can read files can take part.").color(pal.muted));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if crate::icons::button(ui, Ic::Copy, Some("Copy note for another conversation"), false, pal.text, pal.accent).clicked() {
                    ui.ctx().copy_text(note_for_other_conversation());
                    b.note = Some(("Copied. Paste it into the other conversation so it knows where the inbox is.".into(), true));
                }
                if crate::icons::button(ui, Ic::Open, Some("Show inbox folder"), false, pal.text, pal.accent).clicked() {
                    status = Some(reveal(&Path::new(WIKI_DIR).join("inbox")));
                }
                if crate::icons::button(ui, Ic::Replay, Some("Reload"), false, pal.text, pal.accent).clicked() {
                    b.refresh(true);
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let waiting = b.waiting();
                ui.selectable_value(&mut b.tab, Tab::Open, format!("Open ({waiting})"));
                ui.selectable_value(&mut b.tab, Tab::Handled, format!("Handled ({})", b.handled.len()));
                ui.selectable_value(&mut b.tab, Tab::Outbox, format!("Outbox ({})", b.outbox.len()));
                ui.selectable_value(&mut b.tab, Tab::Write, "✏  Write a message");
            });
            ui.separator();
            if let Some((n, good)) = &b.note {
                ui.label(RichText::new(n).color(if *good { pal.ok } else { pal.error }));
                ui.add_space(4.0);
            }
            match b.tab {
                Tab::Open | Tab::Handled | Tab::Outbox => {
                    let (list, empty) = match b.tab {
                        Tab::Open => (&b.open_msgs, "No open messages."),
                        Tab::Handled => (&b.handled, "Nothing handled yet."),
                        _ => (&b.outbox, "The builder hasn't written to other projects yet."),
                    };
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        if list.is_empty() {
                            ui.label(RichText::new(empty).color(pal.muted));
                        }
                        for m in list {
                            message_card(ui, m, &pal);
                        }
                    });
                }
                Tab::Write => {
                    egui::Grid::new("inbox_form").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                        ui.label("To");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut b.to_builder, true, "The builder (Inbox)");
                            ui.radio_value(&mut b.to_builder, false, "Other projects (Outbox)");
                        });
                        ui.end_row();
                        ui.label("Title");
                        ui.add(egui::TextEdit::singleline(&mut b.title).hint_text("Short title, e.g. Fuel plan for 2027").desired_width(f32::INFINITY));
                        ui.end_row();
                        ui.label("From");
                        ui.add(egui::TextEdit::singleline(&mut b.from).desired_width(f32::INFINITY));
                        ui.end_row();
                        if b.to_builder {
                            ui.label("Kind");
                            egui::ComboBox::from_id_salt("inbox_kind").selected_text(KINDS[b.kind]).show_ui(ui, |ui| {
                                for (i, k) in KINDS.iter().enumerate() {
                                    ui.selectable_value(&mut b.kind, i, *k);
                                }
                            });
                            ui.end_row();
                        }
                        ui.label("Files");
                        ui.add(egui::TextEdit::singleline(&mut b.files).hint_text("Optional: files you put in the inbox folder").desired_width(f32::INFINITY));
                        ui.end_row();
                    });
                    ui.add_space(6.0);
                    ui.label("Message");
                    ui.add(egui::TextEdit::multiline(&mut b.text).hint_text("What you want, in plain words.").desired_rows(8).desired_width(f32::INFINITY));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new("  Send  ").color(Color32::WHITE).strong()).fill(pal.accent)).clicked() {
                            b.note = Some(match b.send() {
                                Ok(n) => {
                                    b.tab = if b.to_builder { Tab::Open } else { Tab::Outbox };
                                    (n, true)
                                }
                                Err(e) => (e, false),
                            });
                        }
                        let target = if b.to_builder { inbox_path() } else { outbox_path() };
                        ui.label(RichText::new(format!("Saved to {}", target.display())).small().color(pal.muted));
                    });
                }
            }
        });
        if let Some(s) = status {
            self.status = s;
        }
        if open {
            self.inbox = Some(b);
        }
        self.inbox_badge.refresh(false);
    }
}

fn reveal(path: &Path) -> String {
    let _ = std::fs::create_dir_all(path);
    let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(target_os = "windows") { "explorer" } else { "xdg-open" };
    match std::process::Command::new(opener).arg(path).spawn() {
        Ok(_) => format!("Opened {}", path.display()),
        Err(e) => format!("Couldn't open {}: {e}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Inbox\n\nintro\n\n---\n\n## Open messages\n\n*(none yet)*\n\n---\n\n## Handled\n\n### 2026-11-18 — Wiki\n**From:** Amin\n**Status:** done\n";

    #[test]
    fn adds_and_reads_messages() {
        let d = add_to_inbox(DOC, "### 2026-11-19 — First\n**From:** A\n**Message:**\nhello\n\n**Status:** open");
        let d = add_to_inbox(&d, "### 2026-11-19 — Second\n**From:** B\n**Message:**\nbye\n\n**Status:** needs answer");
        let open = section(&d, "## Open messages");
        assert_eq!(open.iter().map(|m| m.title.as_str()).collect::<Vec<_>>(), ["2026-11-19 — First", "2026-11-19 — Second"]);
        assert_eq!(open[1].status(), "needs answer");
        assert!(!d.contains("none yet"));
        let handled = section(&d, "## Handled");
        assert_eq!(handled.len(), 1);
        assert_eq!(handled[0].status(), "done");
        assert!(d.contains("hello\n\n**Status:** open\n\n### 2026-11-19 — Second"));
        assert!(d.contains("needs answer\n\n---\n\n## Handled"));
    }

    #[test]
    fn outbox_newest_first() {
        let doc = "# Outbox\n\nintro\n\n---\n\n*(none yet)*\n";
        let d = add_to_outbox(doc, "### one\n**Status:** open");
        let d = add_to_outbox(&d, "### two\n**Status:** open");
        let m = outbox_messages(&d);
        assert_eq!(m.iter().map(|m| m.title.as_str()).collect::<Vec<_>>(), ["two", "one"]);
        assert!(d.starts_with("# Outbox\n\nintro\n"));
    }

    #[test]
    fn plain_text() {
        assert_eq!(plain("see [Sets](Sets.md) and `inbox/` **now**"), "see Sets and inbox/ now");
    }

    #[test]
    fn date_looks_right() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
    }
}
