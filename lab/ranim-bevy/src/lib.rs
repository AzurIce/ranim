//! Experimental Bevy integration for Ranim.
//!
//! This crate keeps Bevy as an optional host. Ranim data remains plain
//! `ranim-core` data, while Bevy drives extraction, presentation, and app
//! scheduling around it.

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        render_asset::RenderAssets,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::{RenderAdapter, RenderDevice, RenderQueue},
        texture::GpuImage,
    },
    ecs::world::FromWorld,
};
use ranim_core::{
    CameraFrame, VItem,
    glam::Vec2,
    store::CoreItemStore,
};
use ranim_render::{
    Renderer,
    resource::RenderTextures,
    scene::{RenderScene, VItemRenderData, ViewData},
    utils::WgpuContext,
};

/// A Bevy component containing a Ranim vector item.
#[derive(Component, Clone, Debug)]
pub struct RanimVItem {
    /// The vector item to render.
    pub item: VItem,
}

impl RanimVItem {
    /// Create a component from a Ranim [`VItem`].
    pub fn new(item: VItem) -> Self {
        Self { item }
    }
}

impl From<VItem> for RanimVItem {
    fn from(item: VItem) -> Self {
        Self::new(item)
    }
}

/// The offscreen image that receives the Ranim render output.
#[derive(Resource, Clone, Debug)]
pub struct RanimRenderTarget {
    /// Image sampled by Bevy after the Ranim renderer copies into it.
    pub image: Handle<Image>,
    /// Width of the render target in pixels.
    pub width: u32,
    /// Height of the render target in pixels.
    pub height: u32,
}

impl RanimRenderTarget {
    /// Create a Bevy image suitable as a GPU copy destination.
    pub fn new(images: &mut Assets<Image>, width: u32, height: u32) -> Self {
        let mut image = Image::new_fill(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage |= TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
        let image = images.add(image);

        Self {
            image,
            width,
            height,
        }
    }
}

/// Configuration for [`RanimBevyPlugin`].
#[derive(Clone, Debug)]
pub struct RanimBevyPlugin {
    /// Width of the offscreen Ranim render target.
    pub width: u32,
    /// Height of the offscreen Ranim render target.
    pub height: u32,
    /// Clear color used by Ranim's renderer.
    pub clear_color: wgpu::Color,
}

impl Default for RanimBevyPlugin {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            clear_color: wgpu::Color {
                r: 0.055,
                g: 0.058,
                b: 0.065,
                a: 1.0,
            },
        }
    }
}

impl Plugin for RanimBevyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RanimRenderSettings {
            width: self.width,
            height: self.height,
            clear_color: self.clear_color,
        })
        .init_resource::<RanimRenderTarget>();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            tracing::warn!("RanimBevyPlugin requires Bevy's RenderApp; add it after DefaultPlugins");
            return;
        };

        render_app
            .insert_resource(RanimExtractedScene::default())
            .insert_resource(RanimRenderSettings {
                width: self.width,
                height: self.height,
                clear_color: self.clear_color,
            })
            .add_systems(ExtractSchedule, extract_ranim_scene)
            .add_systems(
                Render,
                render_ranim_scene.in_set(RenderSystems::Render),
            );
    }
}

#[derive(Resource, Clone, Debug)]
struct RanimRenderSettings {
    width: u32,
    height: u32,
    clear_color: wgpu::Color,
}

#[derive(Resource, Clone, Debug, Default)]
struct RanimExtractedScene {
    scene: RenderScene,
    target: Option<Handle<Image>>,
}

#[derive(Resource)]
struct RanimGpuRenderer {
    ctx: WgpuContext,
    renderer: Renderer,
    textures: RenderTextures,
    width: u32,
    height: u32,
}

impl RanimGpuRenderer {
    fn new(
        adapter: &RenderAdapter,
        device: &RenderDevice,
        queue: &RenderQueue,
        width: u32,
        height: u32,
    ) -> Self {
        let ctx = WgpuContext::from_device(
            adapter.as_ref().clone().into_inner(),
            device.wgpu_device().clone(),
            queue.as_ref().clone().into_inner(),
        );
        let renderer = Renderer::new(&ctx, width, height, 8);
        let textures = renderer.new_render_textures(&ctx);

        Self {
            ctx,
            renderer,
            textures,
            width,
            height,
        }
    }
}

impl FromWorld for RanimRenderTarget {
    fn from_world(world: &mut World) -> Self {
        let settings = world.resource::<RanimRenderSettings>().clone();
        let mut images = world.resource_mut::<Assets<Image>>();
        Self::new(&mut images, settings.width, settings.height)
    }
}

fn extract_ranim_scene(
    mut extracted: ResMut<RanimExtractedScene>,
    target: Extract<Option<Res<RanimRenderTarget>>>,
    items: Extract<Query<&RanimVItem>>,
    settings: Res<RanimRenderSettings>,
) {
    extracted.scene.reset();
    extracted.scene.view = view_data_from_camera_frame(
        &CameraFrame::default(),
        settings.width,
        settings.height,
    );
    extracted.target = target.as_ref().map(|target| target.image.clone());
    extracted
        .scene
        .vitems
        .extend(items.iter().map(|component| vitem_render_data(&component.item)));
}

fn render_ranim_scene(
    mut commands: Commands,
    adapter: Res<RenderAdapter>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    settings: Res<RanimRenderSettings>,
    extracted: Res<RanimExtractedScene>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut gpu_renderer: Option<ResMut<RanimGpuRenderer>>,
) {
    let Some(target) = extracted.target.as_ref() else {
        return;
    };
    let Some(gpu_image) = gpu_images.get(target.id()) else {
        return;
    };

    if gpu_renderer.is_none() {
        commands.insert_resource(RanimGpuRenderer::new(
            &adapter,
            &device,
            &queue,
            settings.width,
            settings.height,
        ));
        return;
    }

    let mut gpu_renderer = gpu_renderer.take().unwrap();
    if gpu_renderer.width != settings.width || gpu_renderer.height != settings.height {
        *gpu_renderer = RanimGpuRenderer::new(
            &adapter,
            &device,
            &queue,
            settings.width,
            settings.height,
        );
    }

    let RanimGpuRenderer {
        ctx,
        renderer,
        textures,
        ..
    } = &mut *gpu_renderer;
    renderer.render_scene(ctx, textures, settings.clear_color, &extracted.scene);

    let copy_size = Extent3d {
        width: settings.width,
        height: settings.height,
        depth_or_array_layers: 1,
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Ranim Bevy Copy Encoder"),
    });
    encoder.copy_texture_to_texture(
        gpu_renderer.textures.output_texture().as_image_copy(),
        gpu_image.texture.as_image_copy(),
        copy_size,
    );
    queue.submit([encoder.finish()]);
}

fn vitem_render_data(vitem: &VItem) -> VItemRenderData {
    let points = vitem.get_render_points();
    let attr_count = points.len().div_ceil(2);

    VItemRenderData {
        points,
        normal: vitem.normal.map(|n| n.as_vec3()),
        fill_rgbas: vitem
            .fill_rgbas
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|rgba| rgba.0)
            .collect(),
        stroke_rgbas: vitem
            .stroke_rgbas
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|rgba| rgba.0)
            .collect(),
        stroke_widths: vitem
            .stroke_widths
            .resize_by_sample(attr_count)
            .into_iter()
            .map(|width| width.0)
            .collect(),
    }
}

fn view_data_from_camera_frame(camera_frame: &CameraFrame, width: u32, height: u32) -> ViewData {
    let ratio = width as f64 / height as f64;
    ViewData {
        proj_mat: camera_frame.projection_matrix(ratio).as_mat4(),
        view_mat: camera_frame.view_matrix().as_mat4(),
        half_frame_size: Vec2::new(
            (camera_frame.frame_height * ratio) as f32 / 2.0,
            camera_frame.frame_height as f32 / 2.0,
        ),
    }
}

/// Fill a [`CoreItemStore`] from the Ranim VItems currently in a Bevy world.
pub fn collect_vitems_into_store(items: impl IntoIterator<Item = VItem>) -> CoreItemStore {
    let mut store = CoreItemStore::new();
    store.camera_frames.push(CameraFrame::default());
    store.vitems.extend(items);
    store
}
