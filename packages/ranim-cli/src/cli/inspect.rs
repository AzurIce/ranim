use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use ranim::{
    SceneConstructor,
    color::{AlphaColor, Srgb},
    core::{
        anchor::Aabb,
        animation::{AnimationInfo, AnimationInfoKind},
        components::{rgba::Rgba, vpoint::VPointVec},
        core_item::{
            CoreItem,
            camera_frame::CameraFrame,
            mesh_item::MeshItem,
            vitem::{VItem, vitem_normal_from_points},
        },
        scene_evaluator::EvaluatedFrame,
        utils::{bezier::get_subpath_closed_flag, rate_functions},
    },
    glam::{DVec3, Mat3},
};
use serde::Serialize;

use crate::{RanimUserLibrary, cli::CliArgs, load_user_library, select_scene};

/// Output format for `ranim inspect` subcommands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum FormatArg {
    /// Human-readable text output.
    #[default]
    Text,
    /// JSON output.
    Json,
}

#[derive(Debug, Subcommand)]
pub enum InspectCommand {
    /// List available scenes and their output summaries without constructing scenes.
    Scenes {
        #[arg(long, value_enum, default_value_t = FormatArg::Text)]
        format: FormatArg,
    },
    /// Build a scene and print its hierarchical animation tree.
    Tree {
        /// Scene name. Optional when the library contains exactly one scene.
        scene: Option<String>,
        #[arg(long, value_enum, default_value_t = FormatArg::Text)]
        format: FormatArg,
    },
    /// Sample a scene at a time and print the evaluated frame items.
    Frame {
        /// Scene name.
        scene: String,
        /// Sample time in seconds.
        #[arg(long)]
        at: f64,
        #[arg(long, value_enum, default_value_t = FormatArg::Text)]
        format: FormatArg,
        /// Include full geometry data (points, indices, colors, normals).
        #[arg(long)]
        verbose: bool,
    },
}

pub fn inspect_command(args: &CliArgs, command: InspectCommand) -> Result<()> {
    let lib = load_user_library(args)?;
    match command {
        InspectCommand::Scenes { format } => scenes_command(&lib, format),
        InspectCommand::Tree { scene, format } => tree_command(&lib, scene.as_deref(), format),
        InspectCommand::Frame {
            scene,
            at,
            format,
            verbose,
        } => frame_command(&lib, &scene, at, format, verbose),
    }
}

// MARK: Scenes

#[derive(Debug, Serialize)]
struct ScenesOutput {
    schema_version: u32,
    scenes: Vec<SceneDto>,
}

#[derive(Debug, Serialize)]
struct SceneDto {
    name: String,
    clear_color: String,
    outputs: Vec<OutputDto>,
}

#[derive(Debug, Serialize)]
struct OutputDto {
    width: u32,
    height: u32,
    fps: u32,
    format: String,
    dir: String,
    name_template: String,
    save_frames: bool,
}

const DEFAULT_NAME_TEMPLATE: &str = "{name}_{width}x{height}_{fps}";

fn scenes_output(lib: &RanimUserLibrary) -> ScenesOutput {
    let scenes = lib
        .scenes()
        .map(|scene| SceneDto {
            name: scene.name,
            clear_color: scene.config.clear_color,
            outputs: scene
                .outputs
                .iter()
                .map(|output| OutputDto {
                    width: output.width,
                    height: output.height,
                    fps: output.fps,
                    format: output.format.to_string(),
                    dir: output.dir.clone(),
                    name_template: output
                        .name_template
                        .clone()
                        .unwrap_or_else(|| DEFAULT_NAME_TEMPLATE.to_string()),
                    save_frames: output.save_frames,
                })
                .collect(),
        })
        .collect();
    ScenesOutput {
        schema_version: 1,
        scenes,
    }
}

fn scenes_command(lib: &RanimUserLibrary, format: FormatArg) -> Result<()> {
    let output = scenes_output(lib);
    match format {
        FormatArg::Json => print_json(&output),
        FormatArg::Text => {
            for scene in &output.scenes {
                println!("scene {}", scene.name);
                println!("  clear_color: {}", scene.clear_color);
                println!("  outputs:");
                if scene.outputs.is_empty() {
                    println!("    (none)");
                }
                for output in &scene.outputs {
                    println!(
                        "    - {}x{} @{}fps {} -> {} save_frames={} name_template=\"{}\"",
                        output.width,
                        output.height,
                        output.fps,
                        output.format,
                        output.dir,
                        output.save_frames,
                        output.name_template
                    );
                }
            }
            Ok(())
        }
    }
}

// MARK: Tree

#[derive(Debug, Serialize)]
struct TreeOutput {
    schema_version: u32,
    scene: String,
    total_secs: f64,
    time_marks: Vec<TimeMarkDto>,
    tree: Vec<TreeNodeDto>,
}

#[derive(Debug, Serialize)]
struct TimeMarkDto {
    sec: f64,
    kind: String,
    label: String,
}

#[derive(Debug, Serialize)]
struct TreeNodeDto {
    path: Vec<usize>,
    kind: String,
    anim_name: String,
    range: [f64; 2],
    content_duration_secs: f64,
    rate_func: String,
    enabled: bool,
    /// Content step of iterative segments (`1/N` progress per step); absent
    /// for other nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    sim_step: Option<f64>,
    children: Vec<TreeNodeDto>,
}

fn tree_output(
    scene_name: String,
    total_secs: f64,
    time_marks: Vec<TimeMarkDto>,
    infos: Vec<AnimationInfo>,
) -> TreeOutput {
    TreeOutput {
        schema_version: 1,
        scene: scene_name,
        total_secs,
        time_marks,
        tree: infos
            .iter()
            .enumerate()
            .map(|(idx, info)| animation_info_to_tree_node(info, vec![idx]))
            .collect(),
    }
}

fn animation_info_to_tree_node(info: &AnimationInfo, path: Vec<usize>) -> TreeNodeDto {
    TreeNodeDto {
        path: path.clone(),
        kind: animation_kind_str(info.kind).to_string(),
        anim_name: info.anim_name.clone(),
        range: range_to_array(&info.range),
        content_duration_secs: info.content_duration_secs,
        rate_func: rate_func_name(info.rate_func).to_string(),
        enabled: info.enabled,
        sim_step: info.sim_step,
        children: info
            .children
            .iter()
            .enumerate()
            .map(|(idx, child)| {
                let mut child_path = path.clone();
                child_path.push(idx);
                animation_info_to_tree_node(child, child_path)
            })
            .collect(),
    }
}

fn range_to_array(range: &std::ops::Range<f64>) -> [f64; 2] {
    [range.start, range.end]
}

fn animation_kind_str(kind: AnimationInfoKind) -> &'static str {
    match kind {
        AnimationInfoKind::Eval => "eval",
        AnimationInfoKind::Sequence => "sequence",
        AnimationInfoKind::Stack => "stack",
        AnimationInfoKind::Lagged => "lagged",
        AnimationInfoKind::Static => "static",
        AnimationInfoKind::Sound => "sound",
    }
}

fn rate_func_name(rate_func: fn(f64) -> f64) -> &'static str {
    // Function pointers are opaque in Rust and addresses are not guaranteed to
    // be comparable across codegen units, so identify the known rate functions
    // by their values on a small probe set instead.
    const PROBE_POINTS: [f64; 6] = [0.0, 0.1, 0.25, 0.5, 0.75, 1.0];
    type KnownRateFunc = (&'static str, fn(f64) -> f64);
    const KNOWN: &[KnownRateFunc] = &[
        ("linear", rate_functions::linear),
        ("smooth", rate_functions::smooth),
        ("ease_in_quad", rate_functions::ease_in_quad),
        ("ease_out_quad", rate_functions::ease_out_quad),
        ("ease_in_out_quad", rate_functions::ease_in_out_quad),
        ("ease_in_cubic", rate_functions::ease_in_cubic),
        ("ease_out_cubic", rate_functions::ease_out_cubic),
        ("ease_in_out_cubic", rate_functions::ease_in_out_cubic),
    ];

    KNOWN
        .iter()
        .find(|(_, known)| {
            PROBE_POINTS
                .iter()
                .all(|&point| (known(point) - rate_func(point)).abs() < 1e-12)
        })
        .map(|(name, _)| *name)
        .unwrap_or("custom")
}

fn short_anim_name(anim_name: &str) -> &str {
    let without_generics = anim_name.split('<').next().unwrap_or(anim_name);
    without_generics
        .rsplit("::")
        .next()
        .unwrap_or(without_generics)
}

fn tree_command(lib: &RanimUserLibrary, scene_name: Option<&str>, format: FormatArg) -> Result<()> {
    let scene = select_scene(lib, scene_name)?;
    let sealed = scene.constructor.build_scene();
    let time_marks = sealed
        .time_marks()
        .iter()
        .map(|(sec, mark)| TimeMarkDto {
            sec: *sec,
            kind: match mark {
                ranim::core::TimeMark::Capture(_) => "capture".to_string(),
            },
            label: match mark {
                ranim::core::TimeMark::Capture(label) => label.clone(),
            },
        })
        .collect();
    let output = tree_output(
        scene.name.clone(),
        sealed.total_secs(),
        time_marks,
        sealed.get_animation_infos(),
    );
    match format {
        FormatArg::Json => print_json(&output),
        FormatArg::Text => {
            println!("scene {}", output.scene);
            println!("total_secs: {}", output.total_secs);
            println!("time_marks:");
            for mark in &output.time_marks {
                println!("  - {} {} {:?}", mark.sec, mark.kind, mark.label);
            }
            println!("tree:");
            for node in &output.tree {
                print_tree_node(node, 0);
            }
            Ok(())
        }
    }
}

fn print_tree_node(node: &TreeNodeDto, depth: usize) {
    let path = node
        .path
        .iter()
        .map(|idx| idx.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let sim_step = node
        .sim_step
        .map(|step| format!(" sim_step={step}"))
        .unwrap_or_default();
    println!(
        "{}[{}] {} {} [{}..{}] content={} rate_func={} enabled={}{}",
        "  ".repeat(depth),
        path,
        short_anim_name(&node.anim_name),
        node.kind,
        node.range[0],
        node.range[1],
        node.content_duration_secs,
        node.rate_func,
        node.enabled,
        sim_step
    );
    for child in &node.children {
        print_tree_node(child, depth + 1);
    }
}

// MARK: Frame

#[derive(Debug, Serialize)]
struct FrameOutput {
    schema_version: u32,
    scene: String,
    at: f64,
    total_secs: f64,
    items: Vec<FrameItemDto>,
}

#[derive(Debug, Serialize)]
struct FrameItemDto {
    z_order: usize,
    id: [usize; 2],
    animation_id: usize,
    part: usize,
    kind: String,
    source: SourceDto,
    data: FrameItemDataDto,
}

#[derive(Debug, Serialize)]
struct SourceDto {
    path: Vec<usize>,
    kind: String,
    anim_name: String,
    range: [f64; 2],
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum FrameItemDataDto {
    VItem(VItemDataDto),
    Mesh(MeshDataDto),
    Camera(CameraDataDto),
}

#[derive(Debug, Serialize)]
struct BoundsDto {
    min: [f64; 3],
    max: [f64; 3],
}

#[derive(Debug, Serialize)]
struct VItemDataDto {
    point_count: usize,
    subpath_count: usize,
    closed: Vec<bool>,
    fill_colors: Vec<String>,
    stroke_colors: Vec<String>,
    stroke_widths: Vec<f32>,
    normal: [f32; 3],
    bounds: BoundsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    points: Option<Vec<[f32; 4]>>,
}

#[derive(Debug, Serialize)]
struct MeshDataDto {
    point_count: usize,
    triangle_count: usize,
    transform: [f32; 16],
    bounds: BoundsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    points: Option<Vec<[f32; 3]>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    triangle_indices: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vertex_colors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vertex_normals: Option<Vec<[f32; 3]>>,
}

#[derive(Debug, Serialize)]
struct CameraDataDto {
    pos: [f64; 3],
    facing: [f64; 3],
    up: [f64; 3],
    near: f64,
    far: f64,
    perspective_blend: f64,
    frame_height: f64,
    scale: f64,
    fovy: f64,
}

fn frame_command(
    lib: &RanimUserLibrary,
    scene_name: &str,
    at: f64,
    format: FormatArg,
    verbose: bool,
) -> Result<()> {
    let scene = select_scene(lib, Some(scene_name))?;
    let sealed = scene.constructor.build_scene();
    let total_secs = sealed.total_secs();
    let mut evaluator = sealed.into_evaluator(120.0);
    let infos = evaluator.animation_infos();
    let mut frame = EvaluatedFrame::new();
    evaluator.sample_at(at, &mut frame);

    let items = frame
        .iter()
        .enumerate()
        .map(|(z_order, ((animation_id, part), item))| {
            FrameItemDto::from_core_item(*animation_id, *part, item, &infos, z_order, verbose)
        })
        .collect();

    let output = FrameOutput {
        schema_version: 1,
        scene: scene.name.clone(),
        at,
        total_secs,
        items,
    };

    match format {
        FormatArg::Json => print_json(&output),
        FormatArg::Text => {
            println!(
                "scene {} @{} total_secs={}",
                output.scene, output.at, output.total_secs
            );
            for item in &output.items {
                print_frame_item(item);
            }
            Ok(())
        }
    }
}

impl FrameItemDto {
    fn from_core_item(
        animation_id: usize,
        part: usize,
        item: &CoreItem,
        infos: &[AnimationInfo],
        z_order: usize,
        verbose: bool,
    ) -> Self {
        let (kind, data) = match item {
            CoreItem::VItem(item) => ("vitem", FrameItemDataDto::from_vitem(item, verbose)),
            CoreItem::MeshItem(item) => ("mesh", FrameItemDataDto::from_mesh(item, verbose)),
            CoreItem::CameraFrame(item) => ("camera", FrameItemDataDto::from_camera(item)),
        };

        Self {
            z_order,
            id: [animation_id, part],
            animation_id,
            part,
            kind: kind.to_string(),
            source: source_from_infos(infos, animation_id),
            data,
        }
    }
}

impl FrameItemDataDto {
    fn from_vitem(item: &VItem, verbose: bool) -> Self {
        // Core VItem data lives in the item's local space; the inspect dump
        // reports world-space values, so apply the stored transform here
        // (in the same f32 precision used by the renderer).
        let normal_matrix = Mat3::from_mat4(item.transform).inverse().transpose();
        let vpoints = VPointVec(
            item.points
                .iter()
                .map(|point| item.transform.transform_point3(point.truncate()).as_dvec3())
                .collect(),
        );
        let subpaths = if vpoints.is_empty() {
            Vec::new()
        } else {
            vpoints.get_subpaths()
        };
        let closed = subpaths
            .iter()
            .map(|subpath| {
                get_subpath_closed_flag(subpath)
                    .map(|(_, closed)| closed)
                    .unwrap_or(false)
            })
            .collect();
        let normal_local = item
            .normal
            .unwrap_or_else(|| vitem_normal_from_points(&item.points));
        let normal = (normal_matrix * normal_local).normalize_or_zero();
        let bounds = dvec3_bounds(vpoints.aabb());

        Self::VItem(VItemDataDto {
            point_count: item.points.len(),
            subpath_count: subpaths.len(),
            closed,
            fill_colors: item.fill_rgbas.iter().map(rgba_to_hex).collect(),
            stroke_colors: item.stroke_rgbas.iter().map(rgba_to_hex).collect(),
            stroke_widths: item.stroke_widths.iter().map(|width| width.0).collect(),
            normal: normal.to_array(),
            bounds,
            points: verbose.then(|| {
                item.points
                    .iter()
                    .map(|point| {
                        let world = item.transform.transform_point3(point.truncate());
                        [world.x, world.y, world.z, point.w]
                    })
                    .collect()
            }),
        })
    }

    fn from_mesh(item: &MeshItem, verbose: bool) -> Self {
        let transformed_points = item
            .points
            .iter()
            .map(|point| item.transform.transform_point3(*point))
            .collect::<Vec<_>>();
        let bounds = vec3_bounds(&transformed_points);

        Self::Mesh(MeshDataDto {
            point_count: item.points.len(),
            triangle_count: item.triangle_indices.len() / 3,
            transform: item.transform.to_cols_array(),
            bounds,
            points: verbose.then(|| {
                transformed_points
                    .iter()
                    .map(|point| point.to_array())
                    .collect()
            }),
            triangle_indices: verbose.then(|| item.triangle_indices.clone()),
            vertex_colors: verbose.then(|| item.vertex_colors.iter().map(rgba_to_hex).collect()),
            vertex_normals: verbose.then(|| {
                item.vertex_normals
                    .iter()
                    .map(|normal| normal.to_array())
                    .collect()
            }),
        })
    }

    fn from_camera(item: &CameraFrame) -> Self {
        Self::Camera(CameraDataDto {
            pos: dvec3_to_array(item.pos),
            facing: dvec3_to_array(item.facing),
            up: dvec3_to_array(item.up),
            near: item.near,
            far: item.far,
            perspective_blend: item.perspective_blend,
            frame_height: item.frame_height,
            scale: item.scale,
            fovy: item.fovy,
        })
    }
}

fn source_from_infos(infos: &[AnimationInfo], animation_id: usize) -> SourceDto {
    infos
        .get(animation_id)
        .map(|info| SourceDto {
            path: vec![animation_id],
            kind: animation_kind_str(info.kind).to_string(),
            anim_name: info.anim_name.clone(),
            range: range_to_array(&info.range),
        })
        .unwrap_or(SourceDto {
            path: vec![animation_id],
            kind: "unknown".to_string(),
            anim_name: "<unknown>".to_string(),
            range: [0.0, 0.0],
        })
}

fn dvec3_bounds(bounds: [DVec3; 2]) -> BoundsDto {
    BoundsDto {
        min: dvec3_to_array(bounds[0]),
        max: dvec3_to_array(bounds[1]),
    }
}

fn vec3_bounds(points: &[ranim::glam::Vec3]) -> BoundsDto {
    if points.is_empty() {
        return BoundsDto {
            min: [0.0; 3],
            max: [0.0; 3],
        };
    }
    let mut min = points[0];
    let mut max = points[0];
    for point in &points[1..] {
        min = min.min(*point);
        max = max.max(*point);
    }
    BoundsDto {
        min: [min.x as f64, min.y as f64, min.z as f64],
        max: [max.x as f64, max.y as f64, max.z as f64],
    }
}

fn dvec3_to_array(v: DVec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn rgba_to_hex(rgba: &Rgba) -> String {
    let srgb: AlphaColor<Srgb> = (*rgba).into();
    let [r, g, b, a] = srgb.to_rgba8().to_u8_array();
    format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
}

fn print_frame_item(item: &FrameItemDto) {
    println!(
        "[{}] id=[{}, {}] {} source=[{}] {} {} range=[{}..{}]",
        item.z_order,
        item.id[0],
        item.id[1],
        item.kind,
        item.source
            .path
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        item.source.kind,
        short_anim_name(&item.source.anim_name),
        item.source.range[0],
        item.source.range[1],
    );
    match &item.data {
        FrameItemDataDto::VItem(data) => {
            println!(
                "  point_count={} subpath_count={} closed={:?}",
                data.point_count, data.subpath_count, data.closed
            );
            println!(
                "  fill={:?} stroke={:?} stroke_widths={:?}",
                data.fill_colors, data.stroke_colors, data.stroke_widths
            );
            println!(
                "  normal={:?} bounds=min:{:?} max:{:?}",
                data.normal, data.bounds.min, data.bounds.max
            );
            if let Some(points) = &data.points {
                println!("  points={points:?}");
            }
        }
        FrameItemDataDto::Mesh(data) => {
            println!(
                "  point_count={} triangle_count={} bounds=min:{:?} max:{:?}",
                data.point_count, data.triangle_count, data.bounds.min, data.bounds.max
            );
            if let Some(points) = &data.points {
                println!("  points={points:?}");
            }
            if let Some(triangle_indices) = &data.triangle_indices {
                println!("  triangle_indices={triangle_indices:?}");
            }
            if let Some(vertex_colors) = &data.vertex_colors {
                println!("  vertex_colors={vertex_colors:?}");
            }
            if let Some(vertex_normals) = &data.vertex_normals {
                println!("  vertex_normals={vertex_normals:?}");
            }
        }
        FrameItemDataDto::Camera(data) => {
            println!(
                "  pos={:?} facing={:?} up={:?} near={} far={} perspective_blend={} frame_height={} scale={} fovy={}",
                data.pos,
                data.facing,
                data.up,
                data.near,
                data.far,
                data.perspective_blend,
                data.frame_height,
                data.scale,
                data.fovy
            );
        }
    }
}

fn print_json<T: Serialize>(output: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use ranim::{
        core::{animation::AnimationInfo, utils::rate_functions},
        glam::{DVec3, Vec4},
    };

    use super::*;

    fn sample_animation_info() -> AnimationInfo {
        AnimationInfo {
            anim_name: "AnimSequence".to_string(),
            kind: AnimationInfoKind::Sequence,
            range: 0.0..8.0,
            content_duration_secs: 8.0,
            rate_func: rate_functions::linear,
            enabled: true,
            sim_step: None,
            sound: None,
            children: vec![AnimationInfo {
                anim_name: "FadeIn<VItem>".to_string(),
                kind: AnimationInfoKind::Eval,
                range: 0.0..3.0,
                content_duration_secs: 3.0,
                rate_func: rate_functions::smooth,
                enabled: true,
                sim_step: None,
                sound: None,
                children: vec![],
            }],
        }
    }

    #[test]
    fn maps_animation_info_to_tree_dto() {
        let node = animation_info_to_tree_node(&sample_animation_info(), vec![0]);
        assert_eq!(node.path, [0]);
        assert_eq!(node.kind, "sequence");
        assert_eq!(node.anim_name, "AnimSequence");
        assert_eq!(node.range, [0.0, 8.0]);
        assert_eq!(node.rate_func, "linear");
        assert!(node.enabled);
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].path, [0, 0]);
        assert_eq!(node.children[0].kind, "eval");
        assert_eq!(node.children[0].rate_func, "smooth");
    }

    #[test]
    fn maps_unknown_rate_func_to_custom() {
        fn custom(_t: f64) -> f64 {
            0.0
        }
        assert_eq!(rate_func_name(custom as fn(f64) -> f64), "custom");
    }

    #[test]
    fn maps_vitem_to_frame_dto() {
        let vitem = VItem {
            points: vec![
                Vec4::new(-1.0, -1.0, 0.0, 1.0),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
                Vec4::new(-1.0, -1.0, 0.0, 1.0),
            ],
            fill_rgbas: vec![Vec4::new(1.0, 1.0, 1.0, 1.0).into()],
            stroke_rgbas: vec![Vec4::new(0.0, 0.0, 0.0, 1.0).into()],
            stroke_widths: vec![0.0.into()],
            ..Default::default()
        };

        let dto = FrameItemDto::from_core_item(
            0,
            0,
            &CoreItem::VItem(vitem),
            &[sample_animation_info()],
            0,
            false,
        );

        assert_eq!(dto.id, [0, 0]);
        assert_eq!(dto.kind, "vitem");
        assert_eq!(dto.source.path, [0]);
        assert_eq!(dto.source.anim_name, "AnimSequence");

        let FrameItemDataDto::VItem(data) = &dto.data else {
            panic!("expected vitem data");
        };
        assert_eq!(data.point_count, 3);
        assert_eq!(data.subpath_count, 1);
        assert_eq!(data.closed, [true]);
        assert_eq!(data.fill_colors, ["#ffffffff"]);
        assert_eq!(data.stroke_colors, ["#000000ff"]);
        assert!(data.points.is_none());
    }

    #[test]
    fn maps_dvec3_bounds() {
        let bounds = dvec3_bounds([DVec3::new(-1.0, -2.0, -3.0), DVec3::new(1.0, 2.0, 3.0)]);
        assert_eq!(bounds.min, [-1.0, -2.0, -3.0]);
        assert_eq!(bounds.max, [1.0, 2.0, 3.0]);
    }
}
