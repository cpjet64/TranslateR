use crate::{
    app::TranslateRApp,
    i18n::{tr, tr_format},
};

pub fn draw(app: &mut TranslateRApp, parent: &mut egui::Ui) {
    egui::Panel::bottom("status_bar").show_inside(parent, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some(doc) = &app.doc {
                ui.label(doc.path.display().to_string());
                ui.separator();
                ui.label((if doc.dirty { tr("dirty") } else { tr("clean") }).as_ref());
                if let Some(lang) = app.active_language() {
                    ui.separator();
                    ui.label(tr_format("language: {language}", &[("language", lang)]));
                }
            }
            if let Some(package) = &app.active_package {
                ui.separator();
                ui.label(tr_format(
                    "package version: {version}",
                    &[("version", package.pack_version.clone())],
                ));
            }
            if let Some(release) = &app.updates.latest {
                ui.separator();
                ui.label(tr_format(
                    "update: {version}",
                    &[("version", release.tag_name.clone())],
                ));
            }
            ui.separator();
            ui.label(&app.status);
        });
    });
}
