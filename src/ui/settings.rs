use crate::{app::TranslateRApp, i18n::tr, project::ThemeMode};

pub fn draw(app: &mut TranslateRApp, ui: &mut egui::Ui, id_prefix: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Interface").as_ref());
        language_selector(app, ui, id_prefix);
        ui.separator();
        ui.label(tr("Theme").as_ref());
        theme_selector(app, ui, id_prefix);
    });
}

fn language_selector(app: &mut TranslateRApp, ui: &mut egui::Ui, id_prefix: &str) {
    let mut selected_language = app.config.ui_language.clone();
    egui::ComboBox::from_id_salt(format!("{id_prefix}_ui_language"))
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
}

fn theme_selector(app: &mut TranslateRApp, ui: &mut egui::Ui, id_prefix: &str) {
    let mut selected_theme = app.config.theme;
    egui::ComboBox::from_id_salt(format!("{id_prefix}_theme"))
        .selected_text(theme_label(selected_theme))
        .show_ui(ui, |ui| {
            for theme in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
                ui.selectable_value(&mut selected_theme, theme, theme_label(theme));
            }
        });
    if selected_theme != app.config.theme
        && let Err(err) = app.set_theme_mode(selected_theme, ui.ctx())
    {
        app.last_error = Some(err.to_string());
    }
}

fn theme_label(theme: ThemeMode) -> String {
    match theme {
        ThemeMode::System => tr("System").into_owned(),
        ThemeMode::Light => tr("Light").into_owned(),
        ThemeMode::Dark => tr("Dark").into_owned(),
    }
}
