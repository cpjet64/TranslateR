use crate::app::TranslateRApp;

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
            ui.heading("Active PO");
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
                ui.label(format!("{} translation units", file.stats.entries));
                ui.label(format!("{} untranslated", file.stats.untranslated));
                ui.label(format!("{} warnings", file.stats.warnings));
            } else {
                ui.label("No PO file open.");
            }
            ui.separator();
            ui.heading("How to use");
            match app.mode {
                crate::app::AppMode::Translator => {
                    ui.label("1. Translate empty fields.");
                    ui.label("2. Export a .tpatch file.");
                    ui.label("3. Send the .tpatch file to the maintainer.");
                    ui.label("Translator mode does not save merged PO files.");
                    ui.separator();
                    draw_status_legend(ui);
                }
                crate::app::AppMode::Maintainer => {
                    ui.label("1. Review each TPatch diff.");
                    ui.label("2. Apply matching TPatches.");
                    ui.label("3. Save the merged PO as a new version.");
                    ui.separator();
                    draw_status_legend(ui);
                    ui.separator();
                    ui.label("Apply Selected merges one TPatch.");
                    ui.label("Apply All merges TPatches in filename order.");
                    ui.label("TPatch context must match the active PO.");
                }
                crate::app::AppMode::Startup => {}
            }
            ui.separator();
            if app.mode == crate::app::AppMode::Maintainer {
                ui.heading("TPatches");
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
                if let Some(idx) = view_patch {
                    if let Err(err) = app.view_patch(idx) {
                        app.last_error = Some(err.to_string());
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("Apply Selected").clicked() {
                        if let Err(err) = app.apply_selected_patch() {
                            app.last_error = Some(err.to_string());
                        }
                    }
                    if ui.button("Apply All").clicked() {
                        if let Err(err) = app.apply_all_patches() {
                            app.last_error = Some(err.to_string());
                        }
                    }
                });
                ui.separator();
            }
            ui.heading("History");
            if let Some(state) = &app.history_state {
                ui.label(format!(
                    "latest version: {}",
                    state
                        .latest_version
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "none".to_string())
                ));
                ui.label(format!("versions: {}", app.versions.len()));
            } else {
                ui.label("no saved versions");
            }
        });
}

fn draw_status_legend(ui: &mut egui::Ui) {
    ui.label("[U] untranslated");
    ui.label("[F] fuzzy, needs review");
    ui.label("[P] plural forms");
    ui.label("[C] context matters");
    ui.label("[%] placeholder/format text");
    ui.label("[!] warning to fix or review");
}
