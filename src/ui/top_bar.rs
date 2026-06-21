use rfd::FileDialog;

use crate::{
    app::{AppMode, TranslateRApp},
    i18n::tr,
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::Panel::top("top_bar").show_inside(parent, |ui| {
        ui.horizontal(|ui| {
            if ui.button(tr("Open PO").as_ref()).clicked() {
                let mut dialog = FileDialog::new();
                dialog = if app.mode == AppMode::Translator {
                    dialog.add_filter(
                        tr("TranslateR files").as_ref(),
                        &["trpack", "trdraft", "po"],
                    )
                } else {
                    dialog.add_filter(tr("TranslateR package or PO").as_ref(), &["trpack", "po"])
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
            if app.mode == AppMode::Maintainer
                && ui.button(tr("Load TPatch Folder").as_ref()).clicked()
                && let Some(path) = FileDialog::new().pick_folder()
                && let Err(err) = app.load_patch_folder(path)
            {
                app.last_error = Some(err.to_string());
            }
            if app.mode == AppMode::Maintainer {
                if ui.button(tr("Export TRPack").as_ref()).clicked()
                    && let Some(path) = FileDialog::new()
                        .add_filter(tr("TranslateR package").as_ref(), &["trpack"])
                        .set_file_name("translation.trpack")
                        .save_file()
                    && let Err(err) = app.export_trpack(path)
                {
                    app.last_error = Some(err.to_string());
                }
                if ui.button(tr("Save PO").as_ref()).clicked()
                    && let Err(err) = app.save_active()
                {
                    app.last_error = Some(err.to_string());
                }
                if ui.button(tr("History").as_ref()).clicked() {
                    app.ui.show_history = true;
                }
            }
            if app.mode == AppMode::Translator && ui.button(tr("Save TRDraft").as_ref()).clicked() {
                let path = app.active_draft_path.clone().or_else(|| {
                    FileDialog::new()
                        .add_filter(tr("TranslateR draft").as_ref(), &["trdraft"])
                        .set_file_name("translation.trdraft")
                        .save_file()
                });
                if let Some(path) = path
                    && let Err(err) = app.save_draft(path)
                {
                    app.last_error = Some(err.to_string());
                }
            }
            if ui.button(tr("Export TPatch").as_ref()).clicked()
                && let Some(path) = FileDialog::new()
                    .add_filter(tr("TranslateR patch").as_ref(), &["tpatch"])
                    .set_file_name("translation.tpatch")
                    .save_file()
                && let Err(err) = app.export_patch(path)
            {
                app.last_error = Some(err.to_string());
            }
            if app.mode == AppMode::Maintainer
                && ui.button(tr("Import TPatch").as_ref()).clicked()
                && let Some(path) = FileDialog::new()
                    .add_filter(tr("TranslateR patch").as_ref(), &["tpatch"])
                    .pick_file()
                && let Err(err) = app.import_diff(path)
            {
                app.last_error = Some(err.to_string());
            }
            if ui.button(tr("Mode").as_ref()).clicked() {
                app.mode = AppMode::Startup;
            }
            if ui.button(tr("Check for Updates").as_ref()).clicked() {
                app.check_for_updates(ui.ctx());
            }
            ui.separator();
            ui.label(tr("Interface").as_ref());
            let mut selected_language = app.config.ui_language.clone();
            egui::ComboBox::from_id_salt("ui_language")
                .selected_text(selected_language.clone())
                .show_ui(ui, |ui| {
                    for language in crate::i18n::available_languages() {
                        ui.selectable_value(&mut selected_language, language.clone(), language);
                    }
                });
            if selected_language != app.config.ui_language
                && let Err(err) = app.set_ui_language(selected_language)
            {
                app.last_error = Some(err.to_string());
            }
        });
    });
}
