pub mod display;
pub mod editor_panel;
pub mod file_panel;
pub mod fonts;
pub mod message_list;
pub mod status_bar;
pub mod top_bar;

use crate::{app::TranslateRApp, workflow::VersionLogEntry};

pub fn draw(app: &mut TranslateRApp, ui: &mut egui::Ui) {
    if app.mode == crate::app::AppMode::Startup {
        startup(app, ui);
        let ctx = ui.ctx().clone();
        draw_dialogs(app, &ctx);
        return;
    }
    top_bar::draw(app, ui);
    file_panel::draw(app, ui);
    message_list::draw(app, ui);
    editor_panel::draw(app, ui);
    status_bar::draw(app, ui);
    let ctx = ui.ctx().clone();
    draw_dialogs(app, &ctx);
}

fn startup(app: &mut TranslateRApp, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show_inside(ui, |ui| {
        ui.heading("TranslateR");
        ui.horizontal(|ui| {
            if ui.button("Translator Mode").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("TranslateR files", &["trpack", "trdraft", "po"])
                    .pick_file()
                {
                    if let Err(err) = app.start_translator(path) {
                        app.last_error = Some(err.to_string());
                    }
                }
            }
            if ui.button("Maintainer Mode").clicked() {
                let po = rfd::FileDialog::new()
                    .add_filter("TranslateR package or PO", &["trpack", "po"])
                    .pick_file();
                if let Some(po) = po {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        if let Err(err) = app.start_maintainer(po, folder) {
                            app.last_error = Some(err.to_string());
                        }
                    }
                }
            }
        });
        ui.separator();
        ui.heading("Translator Mode");
        ui.label("Choose the .trpack file the maintainer gave you, or reopen your .trdraft.");
        ui.label("Translate the entries, save a .trdraft if unfinished, then export a .tpatch file to send back.");
        ui.label("Translator mode exports TPatches and drafts, not merged PO files.");
        ui.separator();
        ui.heading("Maintainer Mode");
        ui.label(
            "Choose the base .trpack or PO file, then choose the folder containing translator .tpatch files.",
        );
        ui.label(
            "Export .trpack files, review returned TPatches, merge matches, and save the merged PO as a new version.",
        );
    });
}

fn draw_dialogs(app: &mut TranslateRApp, ctx: &egui::Context) {
    if let Some(err) = app.last_error.clone() {
        egui::Window::new("Error")
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(err);
                if ui.button("OK").clicked() {
                    app.last_error = None;
                }
            });
    }

    if app.ui.show_history {
        egui::Window::new("Version History")
            .collapsible(false)
            .resizable(true)
            .default_width(820.0)
            .default_height(520.0)
            .show(ctx, |ui| {
                if app.versions.is_empty() {
                    ui.label("No saved versions yet.");
                } else {
                    if app.ui.selected_history_version.is_none() {
                        app.ui.selected_history_version =
                            app.versions.last().map(|version| version.version.clone());
                    }
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading("Saved Versions");
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .max_width(260.0)
                                .show(ui, |ui| {
                                    for version in app.versions.iter().rev() {
                                        let selected = app.ui.selected_history_version.as_deref()
                                            == Some(version.version.as_str());
                                        let label =
                                            format!("v{}  {}", version.version, version.created_at);
                                        if ui.selectable_label(selected, label).clicked() {
                                            app.ui.selected_history_version =
                                                Some(version.version.clone());
                                        }
                                    }
                                });
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            let selected = app
                                .ui
                                .selected_history_version
                                .as_ref()
                                .and_then(|number| {
                                    app.versions
                                        .iter()
                                        .find(|version| &version.version == number)
                                })
                                .or_else(|| app.versions.last());
                            if let Some(version) = selected {
                                ui.heading(format!("Version {}", version.version));
                                ui.label(format!("Saved: {}", version.created_at));
                                ui.label(format!("Author: {}", version.author));
                                if !version.note.trim().is_empty() {
                                    ui.label(format!("Reason: {}", version.note));
                                }
                                ui.label(format!(
                                    "Hash: {}",
                                    &version.content_hash[..12.min(version.content_hash.len())]
                                ));
                                ui.separator();
                                let mut text = version_history_log(version);
                                egui::ScrollArea::both().show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut text)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(22)
                                            .interactive(false),
                                    );
                                });
                            }
                        });
                    });
                }
                if ui.button("Close").clicked() {
                    app.ui.show_history = false;
                }
            });
    }

    if let Some(diff) = app.ui.diff_text.clone() {
        egui::Window::new("TPatch Diff")
            .collapsible(false)
            .resizable(true)
            .default_width(760.0)
            .default_height(520.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Apply TPatch").clicked() {
                        if let Err(err) = app.apply_selected_patch() {
                            app.last_error = Some(err.to_string());
                        }
                    }
                    if ui.button("Close").clicked() {
                        app.ui.diff_text = None;
                        app.ui.pending_patch = None;
                    }
                });
                ui.separator();
                let mut text = diff;
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut text)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(30)
                            .interactive(false),
                    );
                });
            });
    }
}

fn version_history_log(version: &VersionLogEntry) -> String {
    let summary = &version.change_summary;
    let mut out = String::new();
    out.push_str("Change Summary\n");
    out.push_str(&format!("Line additions: {}\n", summary.line_additions));
    out.push_str(&format!("Line deletions: {}\n\n", summary.line_deletions));

    if summary.changed_translations.is_empty() {
        out.push_str("Translation changes:\nNo translation field changes detected.\n\n");
    } else {
        out.push_str("Translation changes:\n");
        for change in &summary.changed_translations {
            out.push_str("- ");
            out.push_str(change);
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("Hashes\n");
    if !version.base_hash.is_empty() {
        out.push_str(&format!("Base: {}\n", version.base_hash));
    }
    out.push_str(&format!("Content: {}\n", version.content_hash));
    out
}
