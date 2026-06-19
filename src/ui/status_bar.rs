use crate::app::TranslateRApp;

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::Panel::bottom("status_bar").show_inside(parent, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some(doc) = &app.doc {
                ui.label(doc.path.display().to_string());
                ui.separator();
                ui.label(if doc.dirty { "dirty" } else { "clean" });
                if let Some(lang) = app.active_language() {
                    ui.separator();
                    ui.label(format!("language: {lang}"));
                }
            }
            if let Some(package) = &app.active_package {
                ui.separator();
                ui.label(format!("package version: {}", package.pack_version));
            }
            ui.separator();
            ui.label(&app.status);
        });
    });
}
