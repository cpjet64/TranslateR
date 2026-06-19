use rfd::FileDialog;

use crate::app::{AppMode, TranslateRApp};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::Panel::top("top_bar").show_inside(parent, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Open PO").clicked() {
                let mut dialog = FileDialog::new();
                dialog = if app.mode == AppMode::Translator {
                    dialog.add_filter("TranslateR files", &["trpack", "trdraft", "po"])
                } else {
                    dialog.add_filter("TranslateR package or PO", &["trpack", "po"])
                };
                if let Some(path) = dialog.pick_file() {
                    let result = if app.mode == AppMode::Translator {
                        app.start_translator(path)
                    } else if let Some(folder) = app.ui.patch_folder.clone() {
                        app.start_maintainer(path, folder)
                    } else {
                        app.open_file(path)
                    };
                    if let Err(err) = result {
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
                if ui.button("Export TRPack").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("TranslateR package", &["trpack"])
                        .set_file_name("translation.trpack")
                        .save_file()
                    {
                        if let Err(err) = app.export_trpack(path) {
                            app.last_error = Some(err.to_string());
                        }
                    }
                }
                if ui.button("Save PO").clicked() {
                    if let Err(err) = app.save_active() {
                        app.last_error = Some(err.to_string());
                    }
                }
                if ui.button("History").clicked() {
                    app.ui.show_history = true;
                }
            }
            if app.mode == AppMode::Translator && ui.button("Save TRDraft").clicked() {
                let path = app.active_draft_path.clone().or_else(|| {
                    FileDialog::new()
                        .add_filter("TranslateR draft", &["trdraft"])
                        .set_file_name("translation.trdraft")
                        .save_file()
                });
                if let Some(path) = path {
                    if let Err(err) = app.save_draft(path) {
                        app.last_error = Some(err.to_string());
                    }
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
