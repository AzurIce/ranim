//! Browser canvas hosting for the same render sub-app used by native sessions.

use ranim_core::{
    SceneEvaluator,
    animation::node::AnimationInfo,
    color::{self, LinearSrgb},
    frame::EvaluatedFrame,
};
use ranim_render::{
    RanimRenderApp, RenderAppExt, install_render_app, resource::RenderTextures, utils::WgpuContext,
};
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use crate::{RanimApp, Scene, SceneConstructor, app::render_setup_of};

pub(super) struct CanvasRenderer {
    app: RanimApp,
    evaluator: SceneEvaluator,
    canvas: HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    ctx: WgpuContext,
    textures: RenderTextures,
    blitter: wgpu::util::TextureBlitter,
    clear_color: wgpu::Color,
}

impl CanvasRenderer {
    pub async fn new(
        mut app: RanimApp,
        scene: Scene,
        canvas: HtmlCanvasElement,
    ) -> Result<Self, JsValue> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        // The vector pipelines require more storage buffers than WebGPU's
        // default device limits. Match the adapter, as native preview does.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ranim canvas device"),
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        let ctx = WgpuContext {
            instance,
            adapter,
            device,
            queue,
        };
        let (width, height) = (canvas.width().max(1), canvas.height().max(1));
        let layers = oit_layers(&ctx, width, height)?;
        let mut config = surface
            .get_default_config(&ctx.adapter, width, height)
            .ok_or_else(|| JsValue::from_str("no compatible canvas surface format"))?;
        config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        surface.configure(&ctx.device, &config);

        let clear = color::try_color(&scene.config.clear_color)
            .unwrap_or(color::color("#333333ff"))
            .convert::<LinearSrgb>();
        let [r, g, b, a] = clear.components.map(f64::from);
        let evaluator = scene.build_scene().into_evaluator(60.0);
        app.init_resource::<EvaluatedFrame>();
        let setup = render_setup_of(&app);
        install_render_app(&mut app, &ctx, width, height, layers, setup);
        let textures = RenderTextures::new(&ctx, width, height);
        let blitter = wgpu::util::TextureBlitter::new(&ctx.device, config.format);
        Ok(Self {
            app,
            evaluator,
            canvas,
            surface,
            config,
            ctx,
            textures,
            blitter,
            clear_color: wgpu::Color { r, g, b, a },
        })
    }

    pub fn duration(&self) -> f64 {
        self.evaluator.total_secs()
    }

    pub fn animation_infos(&self) -> Vec<AnimationInfo> {
        self.evaluator.animation_infos()
    }

    pub fn audio(&self) -> &[f32] {
        self.evaluator.audio()
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        let layers = oit_layers(&self.ctx, width, height)?;
        if (width, height) == (self.config.width, self.config.height) {
            return Ok(());
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.ctx.device, &self.config);
        let setup = render_setup_of(&self.app);
        self.app.remove_sub_app(RanimRenderApp);
        install_render_app(&mut self.app, &self.ctx, width, height, layers, setup);
        self.textures = RenderTextures::new(&self.ctx, width, height);
        Ok(())
    }

    pub fn render_at(&mut self, seconds: f64) -> Result<bool, JsValue> {
        if !seconds.is_finite() {
            return Err(JsValue::from_str("preview time must be finite"));
        }
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.ctx.device, &self.config);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .ctx
                    .instance
                    .create_surface(wgpu::SurfaceTarget::Canvas(self.canvas.clone()))
                    .map_err(js_error)?;
                self.surface.configure(&self.ctx.device, &self.config);
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(JsValue::from_str("canvas surface validation failed"));
            }
        };
        let seconds = seconds.clamp(0.0, self.duration());
        self.evaluator.sample_at(
            seconds,
            &mut self.app.world_mut().resource_mut::<EvaluatedFrame>(),
        );
        self.app
            .sub_app_mut(RanimRenderApp)
            .set_frame_target(&self.textures, self.clear_color);
        self.app.update();

        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.ctx.device.create_command_encoder(&Default::default());
        // RenderTextures stores sRGB-encoded pixels. A non-sRGB canvas target
        // needs the raw view, avoiding an extra decode that would darken it.
        let source = if self.config.format.is_srgb() {
            &self.textures.render_view
        } else {
            &self.textures.linear_render_view
        };
        self.blitter
            .copy(&self.ctx.device, &mut encoder, source, &view);
        self.ctx.queue.submit([encoder.finish()]);
        self.ctx.queue.present(output);
        Ok(true)
    }
}

impl Drop for CanvasRenderer {
    fn drop(&mut self) {
        self.ctx.device.destroy();
    }
}

fn oit_layers(ctx: &WgpuContext, width: u32, height: u32) -> Result<usize, JsValue> {
    let limits = ctx.device.limits();
    if width == 0
        || height == 0
        || width > limits.max_texture_dimension_2d
        || height > limits.max_texture_dimension_2d
    {
        return Err(JsValue::from_str("invalid canvas dimensions"));
    }
    let bytes_per_layer = u64::from(width) * u64::from(height) * 8;
    let layers = limits.max_storage_buffer_binding_size / bytes_per_layer;
    if layers == 0 {
        return Err(JsValue::from_str(
            "canvas exceeds the GPU storage buffer limit",
        ));
    }
    Ok(layers.min(8) as usize)
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
