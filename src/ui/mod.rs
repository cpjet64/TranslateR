pub mod display;
pub mod editor_panel;
pub mod file_panel;
pub mod fonts;
pub mod message_list;
pub mod status_bar;
pub mod top_bar;

use crate::app::TranslateRApp;

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
                    .add_filter("PO files", &["po"])
                    .pick_file()
                {
                    if let Err(err) = app.start_translator(path) {
                        app.last_error = Some(err.to_string());
                    }
                }
            }
            if ui.button("Maintainer Mode").clicked() {
                let po = rfd::FileDialog::new()
                    .add_filter("PO files", &["po"])
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
        ui.label("Choose the PO file the maintainer gave you.");
        ui.label("Translate the messages, then export a .tpatch file to send back.");
        ui.label("Translator mode does not save merged PO files.");
        ui.separator();
        ui.heading("Maintainer Mode");
        ui.label(
            "Choose the base PO file, then choose the folder containing translator .tpatch files.",
        );
        ui.label(
            "Review each diff, merge matching TPatches, and save the merged PO as a new version.",
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
        egui::Window::new("History")
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                if app.versions.is_empty() {
                    ui.label("No saved versions yet.");
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for version in &app.versions {
                            ui.label(format!(
                                "v{}  {}  {}  {}",
                                version.version_number,
                                version.created_at,
                                version.translator_name,
                                &version.content_hash[..8.min(version.content_hash.len())]
                            ));
                        }
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

    if app.ui.confirm_restore {
        egui::Window::new("Restore latest version")
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Replace the active PO file with the latest saved version?");
                ui.horizontal(|ui| {
                    if ui.button("Restore").clicked() {
                        if let Err(err) = app.restore_latest() {
                            app.last_error = Some(err.to_string());
                        }
                        app.ui.confirm_restore = false;
                    }
                    if ui.button("Cancel").clicked() {
                        app.ui.confirm_restore = false;
                    }
                });
            });
    }
}
