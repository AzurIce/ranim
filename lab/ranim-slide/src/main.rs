mod app;
mod model;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Ranim Slide",
        native_options,
        Box::new(|cc| Ok(Box::new(app::RanimSlideApp::new(cc)))),
    )
}
