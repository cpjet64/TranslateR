#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([1180.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "TranslateR",
        options,
        Box::new(|cc| Ok(Box::new(translater::app::TranslateRApp::new(cc)))),
    )
}
