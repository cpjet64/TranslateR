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
        let ctx = ui.ctx().clone();
        if ctx.input(|input| input.viewport().close_requested())
            && self.has_unsaved_changes()
            && !self.ui.close_confirmed
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_close_confirmation();
        }
        self.handle_keyboard_shortcuts(&ctx);
        #[cfg(target_os = "linux")]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::IMEAllowed(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::IMEPurpose(
                egui::viewport::IMEPurpose::Normal,
            ));
        }
        self.update_tick(&ctx);
        crate::ui::draw(self, ui);
        self.ui.input_diagnostics.capture_from_context(&ctx);
    }
}
