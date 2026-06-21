use crate::{
    app::TranslateRApp,
    i18n::{tr, tr_format},
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    let panel_width = match app.mode {
        crate::app::AppMode::Maintainer => 280.0,
        crate::app::AppMode::Translator | crate::app::AppMode::Startup => 250.0,
    };

    egui::Panel::left("file_panel")
        .resizable(false)
        .default_size(panel_width)
        .show_inside(parent, |ui| {
            ui.set_width(panel_width);
            ui.heading(tr("Active PO").as_ref());
            ui.separator();
            if let Some(file) = app.project.files.first() {
                let name = file.language.clone().unwrap_or_else(|| {
                    file.path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });
                ui.label(name);
                ui.label(file.path.display().to_string());
                ui.label(tr_format(
                    "{count} translation units",
                    &[("count", file.stats.entries.to_string())],
                ));
                ui.label(tr_format(
                    "{count} untranslated",
                    &[("count", file.stats.untranslated.to_string())],
                ));
                ui.label(tr_format(
                    "{count} warnings",
                    &[("count", file.stats.warnings.to_string())],
                ));
            } else {
                ui.label(tr("No PO file open.").as_ref());
            }
            if let Some(package) = &app.active_package {
                ui.separator();
                ui.heading(
                    (if package.is_draft {
                        tr("Active Draft")
                    } else {
                        tr("Active Package")
                    })
                    .as_ref(),
                );
                ui.label(tr_format(
                    "project: {project}",
                    &[("project", package.project_id.clone())],
                ));
                ui.label(tr_format(
                    "version: {version}",
                    &[("version", package.pack_version.clone())],
                ));
                if let Some(language) = &package.language {
                    ui.label(tr_format(
                        "language: {language}",
                        &[("language", language.clone())],
                    ));
                }
                ui.label(tr_format(
                    "source: {source}",
                    &[("source", package.po_filename.clone())],
                ));
                ui.label(tr_format(
                    "base: {hash}",
                    &[(
                        "hash",
                        package.base_hash[..12.min(package.base_hash.len())].to_string(),
                    )],
                ));
                ui.label(package.source_path.display().to_string());
            }
            ui.separator();
            ui.heading(tr("How to use").as_ref());
            match app.mode {
                crate::app::AppMode::Translator => {
                    ui.label(tr("1. Open the maintainer .trpack.").as_ref());
                    ui.label(tr("2. Translate entries.").as_ref());
                    ui.label(tr("3. Save a .trdraft if unfinished.").as_ref());
                    ui.label(tr("4. Export a .tpatch to send back.").as_ref());
                    ui.label(tr("Translator mode does not save merged PO files.").as_ref());
                    ui.separator();
                    draw_status_legend(ui);
                }
                crate::app::AppMode::Maintainer => {
                    ui.label(tr("1. Export a .trpack for translators.").as_ref());
                    ui.label(tr("2. Review returned TPatches.").as_ref());
                    ui.label(tr("3. Apply matching TPatches.").as_ref());
                    ui.label(tr("4. Save the merged PO as a new version.").as_ref());
                    ui.separator();
                    draw_status_legend(ui);
                    ui.separator();
                    ui.label(tr("Apply Selected merges one TPatch.").as_ref());
                    ui.label(tr("Apply All merges TPatches in filename order.").as_ref());
                    ui.label(tr("TPatch context must match the active PO.").as_ref());
                }
                crate::app::AppMode::Startup => {}
            }
            ui.separator();
            if app.mode == crate::app::AppMode::Maintainer {
                ui.heading(tr("TPatches").as_ref());
                let mut view_patch = None;
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for (idx, patch) in app.ui.patch_files.iter().enumerate() {
                            let selected = app.ui.selected_patch == Some(idx);
                            let label = patch.file_name().unwrap_or_default().to_string_lossy();
                            if ui.selectable_label(selected, label).clicked() {
                                view_patch = Some(idx);
                            }
                        }
                    });
                if let Some(idx) = view_patch
                    && let Err(err) = app.view_patch(idx)
                {
                    app.last_error = Some(err.to_string());
                }
                ui.horizontal(|ui| {
                    if ui.button(tr("Apply Selected").as_ref()).clicked()
                        && let Err(err) = app.apply_selected_patch()
                    {
                        app.last_error = Some(err.to_string());
                    }
                    if ui.button(tr("Apply All").as_ref()).clicked()
                        && let Err(err) = app.apply_all_patches()
                    {
                        app.last_error = Some(err.to_string());
                    }
                });
                ui.separator();
            }
            ui.heading(tr("History").as_ref());
            if let Some(package) = &app.active_package {
                ui.label(tr_format(
                    "latest version: {version}",
                    &[("version", package.pack_version.clone())],
                ));
                ui.label(tr_format(
                    "versions: {count}",
                    &[("count", package.history.len().to_string())],
                ));
            } else {
                ui.label(tr("Open or export a .trpack to use portable version history.").as_ref());
            }
        });
}

fn draw_status_legend(ui: &mut egui::Ui) {
    ui.label(tr("[U] untranslated").as_ref());
    ui.label(tr("[F] fuzzy, needs review").as_ref());
    ui.label(tr("[P] plural forms").as_ref());
    ui.label(tr("[C] context matters").as_ref());
    ui.label(tr("[%] placeholder/format text").as_ref());
    ui.label(tr("[!] warning to fix or review").as_ref());
}
