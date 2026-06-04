use rfd::FileDialog;

use crate::app::{AppMode, TranslateRApp};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::Panel::top("top_bar").show_inside(parent, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Open PO").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("PO files", &["po"])
                    .pick_file()
                {
                    if let Err(err) = app.open_file(path) {
                        app.last_error = Some(err.to_string());
                    }
                }
            }
            if app.mode == AppMode::Maintainer {
                if ui.button("Load TPatch Folder").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        if let Err(err) = app.load_patch_folder(path) {
                            app.last_error = Some(err.to_string());
                        }
                    }
                }
            }
            if app.mode == AppMode::Maintainer {
                if ui.button("Save PO").clicked() {
                    if let Err(err) = app.save_active() {
                        app.last_error = Some(err.to_string());
                    }
                }
                if ui.button("Save Version").clicked() {
                    if let Err(err) = app.record_version("Manual version") {
                        app.last_error = Some(err.to_string());
                    }
                }
                if ui.button("History").clicked() {
                    app.ui.show_history = true;
                }
                if ui.button("Restore").clicked() {
                    app.ui.confirm_restore = true;
                }
            }
            if ui.button("Export TPatch").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("TranslateR patch", &["tpatch"])
                    .set_file_name("translation.tpatch")
                    .save_file()
                {
                    if let Err(err) = app.export_patch(path) {
                        app.last_error = Some(err.to_string());
                    }
                }
            }
            if app.mode == AppMode::Maintainer && ui.button("Import TPatch").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("TranslateR patch", &["tpatch"])
                    .pick_file()
                {
                    if let Err(err) = app.import_diff(path) {
                        app.last_error = Some(err.to_string());
                    }
                }
            }
            if ui.button("Mode").clicked() {
                app.mode = AppMode::Startup;
            }
        });
    });
}
