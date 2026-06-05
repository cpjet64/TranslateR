#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    let options = native_options();
    eframe::run_native(
        "TranslateR",
        options,
        Box::new(|cc| Ok(Box::new(translater::app::TranslateRApp::new(cc)))),
    )
}

fn native_options() -> eframe::NativeOptions {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([1180.0, 700.0]),
        ..Default::default()
    };
    configure_native_options(options)
}

#[cfg(target_os = "macos")]
fn configure_native_options(mut options: eframe::NativeOptions) -> eframe::NativeOptions {
    options.renderer = eframe::Renderer::Glow;
    options
}

#[cfg(not(target_os = "macos"))]
fn configure_native_options(options: eframe::NativeOptions) -> eframe::NativeOptions {
    options
}
