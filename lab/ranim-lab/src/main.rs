mod app;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Ranim Lab",
        native_options,
        Box::new(|cc| Ok(Box::new(app::RanimLabApp::new(cc)))),
    )
}
