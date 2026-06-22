#![allow(dead_code)]

mod timeline;
pub mod fox_halo;

use std::f64::consts::TAU;
#[cfg(all(feature = "video", not(target_family = "wasm")))]
use std::time::Duration;

use bevy::{
    asset::AssetPlugin,
    core_pipeline::oit::OrderIndependentTransparencySettings,
    prelude::*,
};
#[cfg(all(feature = "video", not(target_family = "wasm")))]
use bevy::{log::LogPlugin, render::RenderPlugin, window::ExitCondition, winit::WinitPlugin};
use ranim_bevy::RanimBevyPlugin;
#[cfg(all(feature = "video", not(target_family = "wasm")))]
use ranim_bevy::video::{BevyOutput, VideoExportConfig, VideoExportPlugin};
use ranim_core::{
    VItem,
    components::{rgba::Rgba, width::Width},
    glam::{DVec3, dvec3, vec4},
};

#[allow(unused_imports)]
pub use timeline::{
    RateFunc, VItemAnimState, VItemTimelineBuilder, animate_vitem_timelines, spawn_timeline,
};

#[derive(Debug, Clone, Copy)]
pub struct ExampleConfig {
    pub title: &'static str,
    pub scene_name: &'static str,
    pub asset_path: &'static str,
    pub duration_secs: f32,
    pub output_name: Option<&'static str>,
}

impl ExampleConfig {
    pub fn new(title: &'static str, scene_name: &'static str, duration_secs: f32) -> Self {
        Self {
            title,
            scene_name,
            asset_path: "assets",
            duration_secs,
            output_name: None,
        }
    }

    pub fn with_asset_path(mut self, asset_path: &'static str) -> Self {
        self.asset_path = asset_path;
        self
    }

    pub fn with_output_name(mut self, output_name: &'static str) -> Self {
        self.output_name = Some(output_name);
        self
    }
}

pub fn app(title: &'static str) -> App {
    app_with_asset_path(title, "assets")
}

pub fn app_with_asset_path(title: &'static str, asset_path: impl Into<String>) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: title.to_string(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: asset_path.into(),
                ..default()
            }),
    )
    .add_plugins(RanimBevyPlugin::default());
    app
}

#[cfg(all(feature = "video", not(target_family = "wasm")))]
pub fn headless_app_with_asset_path(asset_path: impl Into<String>) -> App {
    init_video_tracing();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<LogPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: asset_path.into(),
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<WinitPlugin>(),
    )
    .add_plugins(RanimBevyPlugin::default());
    app
}

#[cfg(all(feature = "video", not(target_family = "wasm")))]
fn init_video_tracing() {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    fn build_filter() -> EnvFilter {
        let mut filter = EnvFilter::from_default_env();
        let env = std::env::var("RUST_LOG").unwrap_or_default();
        for (name, level) in [
            ("ranim_bevy", LevelFilter::INFO),
            ("bevy_time", LevelFilter::WARN),
        ]
        .iter()
        .filter(|(name, _)| !env.contains(name))
        {
            filter = filter.add_directive(format!("{name}={level}").parse().unwrap());
        }
        filter
    }

    let indicatif_layer = tracing_indicatif::IndicatifLayer::new();
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_writer(indicatif_layer.get_stderr_writer()))
        .with(indicatif_layer)
        .with(build_filter())
        .try_init();
}

#[cfg(any(not(feature = "video"), target_family = "wasm"))]
pub fn run_example(config: ExampleConfig, build_scene: impl FnOnce(&mut App)) {
    let mut app = app_with_asset_path(config.title, config.asset_path);
    build_scene(&mut app);
    app.run();
}

#[cfg(all(feature = "video", not(target_family = "wasm")))]
pub fn run_example(config: ExampleConfig, build_scene: impl FnOnce(&mut App)) {
    let mut app = headless_app_with_asset_path(config.asset_path);
    build_scene(&mut app);

    let mut output = BevyOutput {
        name: config.output_name.map(ToString::to_string),
        ..default()
    };
    output.apply_env_overrides("RANIM_BEVY_VIDEO_");

    let mut export_config = VideoExportConfig::new(
        config.scene_name,
        Duration::from_secs_f32(config.duration_secs),
    )
    .with_output(output);
    export_config.apply_env_overrides("RANIM_BEVY_VIDEO_");

    app.add_plugins(VideoExportPlugin::new(export_config));
    app.run();
}

pub fn camera(commands: &mut Commands, transform: Transform) {
    commands.spawn((
        Camera3d::default(),
        transform,
        OrderIndependentTransparencySettings::default(),
        Msaa::Off,
    ));
}

pub fn light(commands: &mut Commands) {
    commands.spawn((
        PointLight {
            intensity: 1_600.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 5.0, 8.0),
    ));
}

pub fn regular_polygon(
    sides: usize,
    radius: f64,
    fill: [f32; 4],
    stroke: [f32; 4],
    width: f32,
) -> VItem {
    let mut points = Vec::with_capacity(sides * 2 + 1);

    for idx in 0..sides {
        let a = idx as f64 / sides as f64 * TAU;
        let next = (idx + 1) as f64 / sides as f64 * TAU;
        points.push(dvec3(radius * a.cos(), radius * a.sin(), 0.0));
        points.push(dvec3(
            radius * ((a + next) * 0.5).cos(),
            radius * ((a + next) * 0.5).sin(),
            0.0,
        ));
        if idx == sides - 1 {
            points.push(dvec3(radius * next.cos(), radius * next.sin(), 0.0));
        }
    }

    let mut item = VItem::from_vpoints(points);
    item.close();
    item.fill_rgbas = vec![rgba(fill)].into();
    item.stroke_rgbas = vec![rgba(stroke)].into();
    item.stroke_widths = vec![Width(width)].into();
    item
}

pub fn star(
    points_count: usize,
    outer_radius: f64,
    inner_radius: f64,
    phase: f64,
    fill: [f32; 4],
    stroke: [f32; 4],
    width: f32,
) -> VItem {
    let anchors = points_count * 2;
    let mut points = Vec::with_capacity(anchors * 2 + 1);

    for idx in 0..anchors {
        let a = idx as f64 / anchors as f64 * TAU + phase;
        let next = (idx + 1) as f64 / anchors as f64 * TAU + phase;
        let r = if idx % 2 == 0 {
            outer_radius
        } else {
            inner_radius
        };
        let next_r = if (idx + 1) % 2 == 0 {
            outer_radius
        } else {
            inner_radius
        };
        points.push(dvec3(r * a.cos(), r * a.sin(), 0.0));
        points.push(dvec3(
            (r + next_r) * 0.5 * ((a + next) * 0.5).cos(),
            (r + next_r) * 0.5 * ((a + next) * 0.5).sin(),
            0.0,
        ));
        if idx == anchors - 1 {
            points.push(dvec3(next_r * next.cos(), next_r * next.sin(), 0.0));
        }
    }

    let mut item = VItem::from_vpoints(points);
    item.close();
    item.fill_rgbas = vec![
        rgba(fill),
        rgba([
            (fill[0] + 0.24).min(1.0),
            (fill[1] + 0.12).min(1.0),
            (fill[2] + 0.10).min(1.0),
            fill[3],
        ]),
    ]
    .into();
    item.stroke_rgbas = vec![rgba(stroke)].into();
    item.stroke_widths = vec![Width(width)].into();
    item
}

pub fn ring_segment(
    start: f64,
    end: f64,
    radius: f64,
    thickness: f64,
    fill: [f32; 4],
    stroke: [f32; 4],
) -> VItem {
    ring_segment_with_stroke_width(start, end, radius, thickness, fill, stroke, 0.02)
}

pub fn ring_segment_with_stroke_width(
    start: f64,
    end: f64,
    radius: f64,
    thickness: f64,
    fill: [f32; 4],
    stroke: [f32; 4],
    stroke_width: f32,
) -> VItem {
    let steps = 10;
    let outer = radius + thickness * 0.5;
    let inner = radius - thickness * 0.5;
    let mut anchors = Vec::with_capacity(steps * 2 + 2);

    for idx in 0..=steps {
        let a = start + (end - start) * idx as f64 / steps as f64;
        anchors.push(dvec3(outer * a.cos(), outer * a.sin(), 0.0));
    }
    for idx in (0..=steps).rev() {
        let a = start + (end - start) * idx as f64 / steps as f64;
        anchors.push(dvec3(inner * a.cos(), inner * a.sin(), 0.0));
    }

    from_anchor_polyline(&anchors, fill, stroke, stroke_width)
}

pub fn from_anchor_polyline(
    anchors: &[DVec3],
    fill: [f32; 4],
    stroke: [f32; 4],
    width: f32,
) -> VItem {
    let mut points = Vec::with_capacity(anchors.len() * 2 + 1);
    for idx in 0..anchors.len() {
        let current = anchors[idx];
        let next = anchors[(idx + 1) % anchors.len()];
        points.push(current);
        points.push((current + next) * 0.5);
        if idx == anchors.len() - 1 {
            points.push(next);
        }
    }

    let mut item = VItem::from_vpoints(points);
    item.close();
    item.fill_rgbas = vec![rgba(fill)].into();
    item.stroke_rgbas = vec![rgba(stroke)].into();
    item.stroke_widths = vec![Width(width)].into();
    item
}

pub fn bottom_centered_rect(
    width: f64,
    height: f64,
    fill: [f32; 4],
    stroke: [f32; 4],
    stroke_width: f32,
) -> VItem {
    let half_width = width * 0.5;
    let anchors = [
        dvec3(-half_width, 0.0, 0.0),
        dvec3(half_width, 0.0, 0.0),
        dvec3(half_width, height, 0.0),
        dvec3(-half_width, height, 0.0),
    ];
    from_anchor_polyline(&anchors, fill, stroke, stroke_width)
}

pub fn rgba([r, g, b, a]: [f32; 4]) -> Rgba {
    Rgba(vec4(r, g, b, a))
}

pub fn shuffled_heights(num: usize, seed: u64) -> Vec<usize> {
    let mut values = (1..=num).collect::<Vec<_>>();
    let mut rng = SplitMix64(seed);

    for idx in (1..values.len()).rev() {
        let swap_with = (rng.next() as usize) % (idx + 1);
        values.swap(idx, swap_with);
    }

    values
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}
