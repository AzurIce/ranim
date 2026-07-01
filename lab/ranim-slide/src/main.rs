mod app;
mod model;
mod object;

fn main() -> eframe::Result {
    init_tracing();

    let mut native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 620.0]),
        ..Default::default()
    };
    if std::env::var_os("RANIM_SLIDE_NO_VSYNC").is_some() {
        native_options.wgpu_options.present_mode = wgpu::PresentMode::AutoNoVsync;
        native_options.wgpu_options.desired_maximum_frame_latency = Some(1);
        tracing::info!(
            present_mode = ?native_options.wgpu_options.present_mode,
            desired_maximum_frame_latency = ?native_options.wgpu_options.desired_maximum_frame_latency,
            "using experimental wgpu present settings"
        );
    }

    eframe::run_native(
        "Ranim Slide",
        native_options,
        Box::new(|cc| Ok(Box::new(app::RanimSlideApp::new(cc)))),
    )
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("warn,ranim_slide=info,ranim_render=info,eframe=warn,wgpu=warn")
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .try_init();
}
