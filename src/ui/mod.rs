mod app_shell;

pub mod display;
pub mod editor_panel;
pub mod file_panel;
pub mod fonts;
pub mod input_diagnostics;
pub mod message_list;
pub mod settings;
pub mod status_bar;
pub mod top_bar;

use crate::{
    app::{ConfirmedAction, FileOperation, TranslateRApp},
    i18n::{tr, tr_format},
    workflow::VersionLogEntry,
};

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
        ui.heading(tr("TranslateR").as_ref());
        settings::draw(app, ui, "startup_settings");
        input_diagnostics::draw_button(&mut app.ui.input_diagnostics, ui);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(tr("Translator Mode").as_ref()).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr("TranslateR files").as_ref(), &["trpack", "trdraft", "po"])
                    .pick_file()
                && let Err(err) = app.start_translator(path)
            {
                app.last_error = Some(err.to_string());
            }
            if ui.button(tr("Maintainer Mode").as_ref()).clicked() {
                let po = rfd::FileDialog::new()
                    .add_filter(tr("TranslateR package or PO").as_ref(), &["trpack", "po"])
                    .pick_file();
                if let Some(po) = po
                    && let Some(folder) = rfd::FileDialog::new().pick_folder()
                    && let Err(err) = app.start_maintainer(po, folder)
                {
                    app.last_error = Some(err.to_string());
                }
            }
        });
        ui.separator();
        ui.heading(tr("Translator Mode").as_ref());
        ui.label(tr("Choose the .trpack file the maintainer gave you, or reopen your .trdraft.").as_ref());
        ui.label(tr("Translate the entries, save a .trdraft if unfinished, then export a .tpatch file to send back.").as_ref());
        ui.label(tr("Translator mode exports TPatches and drafts, not merged PO files.").as_ref());
        ui.separator();
        ui.heading(tr("Maintainer Mode").as_ref());
        ui.label(
            tr("Choose the base .trpack or PO file, then choose the folder containing translator .tpatch files.").as_ref(),
        );
        ui.label(
            tr("Export .trpack files, review returned TPatches, merge matches, and save the merged PO as a new version.").as_ref(),
        );
    });
}

fn draw_dialogs(app: &mut TranslateRApp, ctx: &egui::Context) {
    if let Some(status) = input_diagnostics::draw_window(&mut app.ui.input_diagnostics, ctx) {
        app.status = status;
    }

    if app.updates.show_dialog {
        egui::Window::new(tr("TranslateR Update").as_ref())
            .collapsible(false)
            .resizable(true)
            .default_width(640.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                ui.label(tr_format(
                    "Installed version: {version}",
                    &[("version", env!("CARGO_PKG_VERSION").to_string())],
                ));
                if let Some(release) = app.updates.latest.clone() {
                    ui.label(tr_format(
                        "Latest version: {version}",
                        &[("version", release.tag_name.clone())],
                    ));
                    ui.label(tr_format(
                        "Package: {name}",
                        &[("name", release.asset.name.clone())],
                    ));
                    ui.separator();
                    ui.heading(tr("Release Notes").as_ref());
                    let mut notes = release.body.clone();
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut notes)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(8)
                                    .interactive(false),
                            );
                        });
                }
                ui.separator();
                ui.label(app.updates.message.clone());
                ui.horizontal(|ui| {
                    if matches!(
                        app.updates.status,
                        crate::update::UpdateStatus::UpdateAvailable
                    ) && ui.button(tr("Download Update").as_ref()).clicked()
                    {
                        app.download_update(ctx);
                    }
                    if matches!(
                        app.updates.status,
                        crate::update::UpdateStatus::ReadyToApply
                    ) && ui.button(tr("Apply Update and Restart").as_ref()).clicked()
                    {
                        app.request_confirmation(
                            FileOperation::ApplyUpdate,
                            ConfirmedAction::ApplyDownloadedUpdate,
                        );
                    }
                    if let Some(release) = app.updates.latest.clone()
                        && ui.button(tr("Open Release Page").as_ref()).clicked()
                        && let Err(err) = crate::update::open_url(&release.html_url)
                    {
                        app.last_error = Some(err.to_string());
                    }
                    if ui.button(tr("Close").as_ref()).clicked() {
                        app.updates.show_dialog = false;
                    }
                });
            });
    }

    draw_confirmation_dialog(app, ctx);

    if let Some(err) = app.last_error.clone() {
        egui::Window::new(tr("Error").as_ref())
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(err);
                if ui.button(tr("OK").as_ref()).clicked() {
                    app.last_error = None;
                }
            });
    }

    if app.ui.show_history {
        egui::Window::new(tr("Version History").as_ref())
            .collapsible(false)
            .resizable(true)
            .default_width(820.0)
            .default_height(520.0)
            .show(ctx, |ui| {
                if app.versions.is_empty() {
                    ui.label(tr("No saved versions yet.").as_ref());
                } else {
                    if app.ui.selected_history_version.is_none() {
                        app.ui.selected_history_version =
                            app.versions.last().map(|version| version.version.clone());
                    }
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading(tr("Saved Versions").as_ref());
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .max_width(260.0)
                                .show(ui, |ui| {
                                    for version in app.versions.iter().rev() {
                                        let selected = app.ui.selected_history_version.as_deref()
                                            == Some(version.version.as_str());
                                        let label = tr_format(
                                            "v{version}  {created_at}",
                                            &[
                                                ("version", version.version.clone()),
                                                ("created_at", version.created_at.clone()),
                                            ],
                                        );
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
                                ui.heading(tr_format(
                                    "Version {version}",
                                    &[("version", version.version.clone())],
                                ));
                                ui.label(tr_format(
                                    "Saved: {created_at}",
                                    &[("created_at", version.created_at.clone())],
                                ));
                                ui.label(tr_format(
                                    "Author: {author}",
                                    &[("author", version.author.clone())],
                                ));
                                if !version.note.trim().is_empty() {
                                    ui.label(tr_format(
                                        "Reason: {reason}",
                                        &[("reason", version.note.clone())],
                                    ));
                                }
                                ui.label(tr_format(
                                    "Hash: {hash}",
                                    &[(
                                        "hash",
                                        version.content_hash[..12.min(version.content_hash.len())]
                                            .to_string(),
                                    )],
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
                if ui.button(tr("Close").as_ref()).clicked() {
                    app.ui.show_history = false;
                }
            });
    }

    if let Some(diff) = app.ui.diff_text.clone() {
        egui::Window::new(tr("TPatch Diff").as_ref())
            .collapsible(false)
            .resizable(true)
            .default_width(760.0)
            .default_height(520.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr("Apply TPatch").as_ref()).clicked() {
                        app.request_confirmation(
                            FileOperation::ApplyTPatch,
                            ConfirmedAction::ApplySelectedPatch,
                        );
                    }
                    if ui.button(tr("Close").as_ref()).clicked() {
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

fn draw_confirmation_dialog(app: &mut TranslateRApp, ctx: &egui::Context) {
    let Some(pending) = app.ui.pending_confirmation.clone() else {
        return;
    };
    let target_path = pending
        .action
        .target_path()
        .map(|path| path.display().to_string())
        .or_else(|| active_po_target_for_confirmation(app, pending.operation));

    egui::Window::new(tr("Confirm file change").as_ref())
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .show(ctx, |ui| {
            ui.heading(file_operation_label(pending.operation));
            ui.label(file_operation_message(pending.operation));
            if let Some(path) = target_path.as_ref() {
                ui.separator();
                ui.label(tr_format("Target: {path}", &[("path", path.clone())]));
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(tr("Cancel").as_ref()).clicked() {
                    app.cancel_pending_confirmation();
                }
                if ui.button(tr("Continue").as_ref()).clicked() {
                    app.confirm_pending_confirmation();
                }
            });
        });
}

fn active_po_target_for_confirmation(
    app: &TranslateRApp,
    operation: FileOperation,
) -> Option<String> {
    match operation {
        FileOperation::SavePo | FileOperation::ApplyTPatch | FileOperation::ApplyAllTPatches => {
            app.doc.as_ref().map(|doc| doc.path.display().to_string())
        }
        FileOperation::SavePoAs
        | FileOperation::SaveTrDraft
        | FileOperation::SaveTrDraftAs
        | FileOperation::ExportTPatch
        | FileOperation::ExportTRPack
        | FileOperation::ApplyUpdate => None,
    }
}

fn file_operation_label(operation: FileOperation) -> String {
    match operation {
        FileOperation::SavePo => tr("Save PO").into_owned(),
        FileOperation::SavePoAs => tr("Save PO As...").into_owned(),
        FileOperation::SaveTrDraft => tr("Save TRDraft").into_owned(),
        FileOperation::SaveTrDraftAs => tr("Save TRDraft As...").into_owned(),
        FileOperation::ExportTPatch => tr("Export TPatch").into_owned(),
        FileOperation::ExportTRPack => tr("Export TRPack").into_owned(),
        FileOperation::ApplyTPatch => tr("Apply TPatch").into_owned(),
        FileOperation::ApplyAllTPatches => tr("Apply All TPatches").into_owned(),
        FileOperation::ApplyUpdate => tr("Apply Update and Restart").into_owned(),
    }
}

fn file_operation_message(operation: FileOperation) -> String {
    match operation {
        FileOperation::SavePo => tr("This will write changes to the active PO file.").into_owned(),
        FileOperation::SavePoAs => {
            tr("This will write a PO copy to the selected location and make it the active PO.")
                .into_owned()
        }
        FileOperation::SaveTrDraft => tr("This will write a translator draft file.").into_owned(),
        FileOperation::SaveTrDraftAs => {
            tr("This will write a translator draft file to the selected location.").into_owned()
        }
        FileOperation::ExportTPatch => {
            tr("This will write a TPatch file containing your changes and questions.").into_owned()
        }
        FileOperation::ExportTRPack => {
            tr("This will write a TRPack package for translators.").into_owned()
        }
        FileOperation::ApplyTPatch => {
            tr("This will modify the active PO by merging the selected TPatch.").into_owned()
        }
        FileOperation::ApplyAllTPatches => {
            tr("This will modify the active PO by merging every TPatch in filename order.")
                .into_owned()
        }
        FileOperation::ApplyUpdate => {
            tr("This will replace the portable app files in this folder and restart TranslateR.")
                .into_owned()
        }
    }
}

fn version_history_log(version: &VersionLogEntry) -> String {
    let summary = &version.change_summary;
    let mut out = String::new();
    out.push_str(tr("Change Summary").as_ref());
    out.push('\n');
    out.push_str(&tr_format(
        "Line additions: {count}",
        &[("count", summary.line_additions.to_string())],
    ));
    out.push('\n');
    out.push_str(&tr_format(
        "Line deletions: {count}",
        &[("count", summary.line_deletions.to_string())],
    ));
    out.push_str("\n\n");

    if summary.changed_translations.is_empty() {
        out.push_str(tr("Translation changes:").as_ref());
        out.push('\n');
        out.push_str(tr("No translation field changes detected.").as_ref());
        out.push_str("\n\n");
    } else {
        out.push_str(tr("Translation changes:").as_ref());
        out.push('\n');
        for change in &summary.changed_translations {
            out.push_str("- ");
            out.push_str(change);
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str(tr("Hashes").as_ref());
    out.push('\n');
    if !version.base_hash.is_empty() {
        out.push_str(&tr_format(
            "Base: {hash}",
            &[("hash", version.base_hash.clone())],
        ));
        out.push('\n');
    }
    out.push_str(&tr_format(
        "Content: {hash}",
        &[("hash", version.content_hash.clone())],
    ));
    out
}
