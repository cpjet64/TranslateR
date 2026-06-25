use rfd::FileDialog;

use crate::{
    app::{AppMode, ConfirmedAction, FileOperation, TranslateRApp, force_extension},
    i18n::tr,
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::Panel::top("top_bar").show_inside(parent, |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(app.can_undo(), egui::Button::new(tr("Undo").as_ref()))
                .clicked()
            {
                app.undo();
            }
            if ui
                .add_enabled(app.can_redo(), egui::Button::new(tr("Redo").as_ref()))
                .clicked()
            {
                app.redo();
            }
            ui.separator();
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
                    && let Some(path) = save_path(
                        tr("TranslateR package").as_ref(),
                        "trpack",
                        default_file_name(app, "trpack", "translation.trpack"),
                    )
                {
                    app.request_confirmation(
                        FileOperation::ExportTRPack,
                        ConfirmedAction::ExportTrpack(path),
                    );
                }
                if ui.button(tr("Save PO").as_ref()).clicked() {
                    app.request_confirmation(FileOperation::SavePo, ConfirmedAction::SaveActive);
                }
                if ui.button(tr("Save PO As...").as_ref()).clicked()
                    && let Some(path) = save_path(
                        tr("PO file").as_ref(),
                        "po",
                        default_file_name(app, "po", "translation.po"),
                    )
                {
                    app.request_confirmation(
                        FileOperation::SavePoAs,
                        ConfirmedAction::SaveActiveAs(path),
                    );
                }
                if ui.button(tr("History").as_ref()).clicked() {
                    app.ui.show_history = true;
                }
            }
            if app.mode == AppMode::Translator && ui.button(tr("Save TRDraft").as_ref()).clicked() {
                let path = app.active_draft_path.clone().or_else(|| {
                    save_path(
                        tr("TranslateR draft").as_ref(),
                        "trdraft",
                        default_file_name(app, "trdraft", "translation.trdraft"),
                    )
                });
                if let Some(path) = path {
                    app.request_confirmation(
                        FileOperation::SaveTrDraft,
                        ConfirmedAction::SaveDraft(path),
                    );
                }
            }
            if app.mode == AppMode::Translator
                && ui.button(tr("Save TRDraft As...").as_ref()).clicked()
                && let Some(path) = save_path(
                    tr("TranslateR draft").as_ref(),
                    "trdraft",
                    default_file_name(app, "trdraft", "translation.trdraft"),
                )
            {
                app.request_confirmation(
                    FileOperation::SaveTrDraftAs,
                    ConfirmedAction::SaveDraft(path),
                );
            }
            if ui.button(tr("Export TPatch").as_ref()).clicked()
                && let Some(path) = save_path(
                    tr("TranslateR patch").as_ref(),
                    "tpatch",
                    default_file_name(app, "tpatch", "translation.tpatch"),
                )
            {
                app.request_confirmation(
                    FileOperation::ExportTPatch,
                    ConfirmedAction::ExportPatch(path),
                );
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
            crate::ui::input_diagnostics::draw_button(&mut app.ui.input_diagnostics, ui);
        });
        ui.horizontal_wrapped(|ui| {
            crate::ui::settings::draw(app, ui, "top_bar_settings");
        });
    });
}

fn save_path(label: &str, extension: &str, file_name: String) -> Option<std::path::PathBuf> {
    FileDialog::new()
        .add_filter(label, &[extension])
        .set_file_name(&file_name)
        .save_file()
        .map(|path| force_extension(path, extension))
}

fn default_file_name(app: &TranslateRApp, extension: &str, fallback: &str) -> String {
    let Some(stem) = app
        .doc
        .as_ref()
        .and_then(|doc| doc.path.file_stem())
        .map(|stem| stem.to_string_lossy().to_string())
    else {
        return fallback.to_string();
    };
    format!("{stem}.{extension}")
}
