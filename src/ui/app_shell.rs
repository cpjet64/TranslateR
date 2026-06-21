use crate::{
    app::{AppMode, TranslateRApp, UiState},
    i18n::tr,
    project::{AppConfig, ProjectState},
};

impl TranslateRApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::fonts::install_fonts(&cc.egui_ctx);
        let config = AppConfig::load();
        cc.egui_ctx.set_theme(config.theme.egui_preference());
        crate::i18n::init(&config.ui_language);
        Self {
            mode: AppMode::Startup,
            project: ProjectState::default(),
            doc: None,
            config,
            versions: Vec::new(),
            ui: UiState::default(),
            active_package: None,
            active_draft_path: None,
            patch_base_text: None,
            updates: Default::default(),
            last_error: None,
            status: tr("Ready").into_owned(),
        }
    }
}

impl eframe::App for TranslateRApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.update_tick(ui.ctx());
        crate::ui::draw(self, ui);
    }
}
