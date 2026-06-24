use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::i18n::{tr, tr_format};

const DEFAULT_MAX_ENTRIES: usize = 500;
const MAX_LOG_TEXT_CHARS: usize = 500;

#[derive(Debug, Clone)]
pub struct InputDiagnosticsState {
    pub show_window: bool,
    pub capture_enabled: bool,
    pub entries: Vec<InputDiagnosticEntry>,
    pub last_saved_path: Option<PathBuf>,
    pub last_error: Option<String>,
    pub max_entries: usize,
    next_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDiagnosticEntry {
    pub sequence: u64,
    pub timestamp: String,
    pub focused_widget: String,
    pub event: String,
}

impl Default for InputDiagnosticsState {
    fn default() -> Self {
        Self {
            show_window: false,
            capture_enabled: false,
            entries: Vec::new(),
            last_saved_path: None,
            last_error: None,
            max_entries: DEFAULT_MAX_ENTRIES,
            next_sequence: 1,
        }
    }
}

impl InputDiagnosticsState {
    pub fn capture_from_context(&mut self, ctx: &egui::Context) {
        if !self.capture_enabled {
            return;
        }

        let focused_widget = focused_widget_label(ctx);
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            if let Some(description) = describe_event(&event) {
                self.push_event(focused_widget.clone(), description);
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_error = None;
        self.last_saved_path = None;
        self.next_sequence = 1;
    }

    pub fn save_to_executable_dir(&mut self) -> Result<PathBuf> {
        self.save_to_dir(&executable_directory())
    }

    pub fn save_to_dir(&mut self, directory: &Path) -> Result<PathBuf> {
        fs::create_dir_all(directory).with_context(|| {
            format!(
                "Could not create diagnostic log folder {}",
                directory.display()
            )
        })?;
        let path = directory.join(format!(
            "translater-input-diagnostics-{}.log",
            filename_timestamp()
        ));
        fs::write(&path, self.render_log())
            .with_context(|| format!("Could not write diagnostic log to {}", path.display()))?;
        self.last_error = None;
        self.last_saved_path = Some(path.clone());
        Ok(path)
    }

    fn push_event(&mut self, focused_widget: String, event: String) {
        let entry = InputDiagnosticEntry {
            sequence: self.next_sequence,
            timestamp: timestamp(),
            focused_widget,
            event,
        };
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            let excess = self.entries.len() - self.max_entries;
            self.entries.drain(0..excess);
        }
    }

    fn render_log(&self) -> String {
        let mut out = String::new();
        out.push_str("TranslateR input diagnostics\n");
        out.push_str(&format!("app_version={}\n", env!("CARGO_PKG_VERSION")));
        out.push_str(&format!("os={}\n", std::env::consts::OS));
        out.push_str(&format!("arch={}\n", std::env::consts::ARCH));
        out.push_str(&format!(
            "current_exe={}\n",
            std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|err| format!("unknown ({err})"))
        ));
        out.push_str(&format!("captured_at={}\n", timestamp()));
        out.push_str(&format!("entries={}\n", self.entries.len()));
        out.push_str("note=This log can contain typed or pasted text.\n\n");
        out.push_str("sequence\ttimestamp\tfocused_widget\tevent\n");
        for entry in &self.entries {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                entry.sequence, entry.timestamp, entry.focused_widget, entry.event
            ));
        }
        out
    }
}

pub fn draw_button(state: &mut InputDiagnosticsState, ui: &mut egui::Ui) {
    if ui.button(tr("Input Diagnostics").as_ref()).clicked() {
        state.show_window = true;
    }
}

pub fn draw_window(state: &mut InputDiagnosticsState, ctx: &egui::Context) -> Option<String> {
    if !state.show_window {
        return None;
    }

    let mut status = None;
    let mut open = state.show_window;
    egui::Window::new(tr("Input Diagnostics").as_ref())
        .open(&mut open)
        .resizable(true)
        .default_width(760.0)
        .default_height(520.0)
        .show(ctx, |ui| {
            ui.label(
                tr("Use this when keyboard, accent, IME, or punctuation input behaves incorrectly.")
                    .as_ref(),
            );
            ui.label(
                tr("Diagnostics can include typed and pasted text. Turn capture off after reproducing the issue.")
                    .as_ref(),
            );
            ui.label(tr_format(
                "Logs are saved beside the running TranslateR binary: {path}",
                &[("path", executable_directory().display().to_string())],
            ));
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut state.capture_enabled,
                    tr("Capture input events").as_ref(),
                );
                if ui.button(tr("Clear").as_ref()).clicked() {
                    state.clear();
                }
                if ui.button(tr("Save diagnostic log").as_ref()).clicked() {
                    match state.save_to_executable_dir() {
                        Ok(path) => {
                            status = Some(tr_format(
                                "Saved diagnostic log to {path}",
                                &[("path", path.display().to_string())],
                            ));
                        }
                        Err(err) => {
                            state.last_error = Some(err.to_string());
                        }
                    }
                }
            });
            ui.label(tr_format(
                "Captured events: {count}",
                &[("count", state.entries.len().to_string())],
            ));
            if let Some(path) = &state.last_saved_path {
                ui.label(tr_format(
                    "Last saved: {path}",
                    &[("path", path.display().to_string())],
                ));
            }
            if let Some(err) = &state.last_error {
                ui.colored_label(egui::Color32::RED, err);
            }
            ui.separator();
            let mut text = state.render_log();
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(22)
                        .interactive(false),
                );
            });
        });
    state.show_window = open;
    status
}

fn focused_widget_label(ctx: &egui::Context) -> String {
    ctx.memory(|memory| {
        memory
            .focused()
            .map(|id| format!("{id:?}"))
            .unwrap_or_else(|| "none".to_string())
    })
}

fn describe_event(event: &egui::Event) -> Option<String> {
    match event {
        egui::Event::Copy => Some("Copy".to_string()),
        egui::Event::Cut => Some("Cut".to_string()),
        egui::Event::Paste(text) => Some(format!("Paste text={}", quoted_text(text))),
        egui::Event::Text(text) => Some(format!("Text text={}", quoted_text(text))),
        egui::Event::Key {
            key,
            physical_key,
            pressed,
            repeat,
            modifiers,
        } => Some(format!(
            "Key key={key:?} physical_key={physical_key:?} pressed={pressed} repeat={repeat} modifiers={modifiers:?}"
        )),
        egui::Event::Ime(egui::ImeEvent::Enabled) => Some("Ime::Enabled".to_string()),
        egui::Event::Ime(egui::ImeEvent::Disabled) => Some("Ime::Disabled".to_string()),
        egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
            Some(format!("Ime::Preedit text={}", quoted_text(text)))
        }
        egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
            Some(format!("Ime::Commit text={}", quoted_text(text)))
        }
        egui::Event::WindowFocused(focused) => Some(format!("WindowFocused focused={focused}")),
        _ => None,
    }
}

fn quoted_text(text: &str) -> String {
    let mut out = String::from("\"");
    let mut truncated = false;
    for (index, ch) in text.chars().enumerate() {
        if index >= MAX_LOG_TEXT_CHARS {
            truncated = true;
            break;
        }
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    if truncated {
        out.push_str("...[truncated]");
    }
    out.push('"');
    out
}

fn executable_directory() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn timestamp() -> String {
    let now = now();
    now.format(&Rfc3339)
        .unwrap_or_else(|_| now.unix_timestamp().to_string())
}

fn filename_timestamp() -> String {
    let now = now();
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_text_ime_key_and_focus_events() {
        assert_eq!(
            describe_event(&egui::Event::Text("ê?".to_string())).unwrap(),
            "Text text=\"ê?\""
        );
        assert_eq!(
            describe_event(&egui::Event::Paste("line\n二".to_string())).unwrap(),
            "Paste text=\"line\\n二\""
        );
        assert_eq!(
            describe_event(&egui::Event::Ime(egui::ImeEvent::Commit(
                "中文".to_string()
            )))
            .unwrap(),
            "Ime::Commit text=\"中文\""
        );
        let key = describe_event(&egui::Event::Key {
            key: egui::Key::Questionmark,
            physical_key: Some(egui::Key::Slash),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                shift: true,
                ..Default::default()
            },
        })
        .unwrap();
        assert!(key.contains("key=Questionmark"));
        assert!(key.contains("physical_key=Some(Slash)"));
        assert!(key.contains("pressed=true"));
        assert!(describe_event(&egui::Event::PointerGone).is_none());
    }

    #[test]
    fn caps_events_to_configured_max_entries() {
        let mut state = InputDiagnosticsState {
            max_entries: 2,
            ..Default::default()
        };
        state.push_event("field".to_string(), "first".to_string());
        state.push_event("field".to_string(), "second".to_string());
        state.push_event("field".to_string(), "third".to_string());

        assert_eq!(state.entries.len(), 2);
        assert_eq!(state.entries[0].event, "second");
        assert_eq!(state.entries[1].event, "third");
        assert_eq!(state.entries[0].sequence, 2);
        assert_eq!(state.entries[1].sequence, 3);
    }

    #[test]
    fn saves_log_to_requested_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = InputDiagnosticsState::default();
        state.push_event("translation".to_string(), "Text text=\"ê!\"".to_string());

        let path = state.save_to_dir(dir.path()).unwrap();
        assert_eq!(path.parent(), Some(dir.path()));
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("translater-input-diagnostics-")
        );
        let log = fs::read_to_string(path).unwrap();
        assert!(log.contains("TranslateR input diagnostics"));
        assert!(log.contains("app_version="));
        assert!(log.contains("os="));
        assert!(log.contains("translation"));
        assert!(log.contains("Text text=\"ê!\""));
    }

    #[test]
    fn clear_resets_capture_log_state() {
        let mut state = InputDiagnosticsState {
            last_error: Some("error".to_string()),
            last_saved_path: Some(PathBuf::from("log.txt")),
            ..Default::default()
        };
        state.push_event("translation".to_string(), "Text text=\"a\"".to_string());

        state.clear();

        assert!(state.entries.is_empty());
        assert!(state.last_error.is_none());
        assert!(state.last_saved_path.is_none());
        state.push_event("translation".to_string(), "Text text=\"b\"".to_string());
        assert_eq!(state.entries[0].sequence, 1);
    }
}
