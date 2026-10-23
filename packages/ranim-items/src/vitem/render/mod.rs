//! Render-world data and GPU implementation for vitem items.

mod buffer;
mod pipeline;

use ranim_core::{
    components::{rgba::Rgba, width::Width},
    glam::{Mat4, Vec3, Vec4},
};

pub(super) fn install(app: &mut bevy_app::App) {
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use ranim_render::{
        extract::ExtractPlugin,
        schedule::{RenderSchedule, ViewPhase},
        utils::WgpuContext,
    };

    use super::{VItem, geometry::*, svg::SvgItem};

    let buffer = buffer::VItemsBuffer::new(app.world().resource::<WgpuContext>());
    app.insert_resource(buffer).add_plugins((
        ExtractPlugin::<VItem>::default(),
        ExtractPlugin::<Vec<VItem>>::default(),
        ExtractPlugin::<Arc>::default(),
        ExtractPlugin::<ArcBetweenPoints>::default(),
        ExtractPlugin::<Circle>::default(),
        ExtractPlugin::<Ellipse>::default(),
        ExtractPlugin::<EllipticArc>::default(),
        ExtractPlugin::<Line>::default(),
        ExtractPlugin::<Parallelogram>::default(),
        ExtractPlugin::<Polygon>::default(),
        ExtractPlugin::<Rectangle>::default(),
        ExtractPlugin::<RegularPolygon>::default(),
        ExtractPlugin::<Square>::default(),
        ExtractPlugin::<SvgItem>::default(),
    ));
    #[cfg(feature = "typst")]
    app.add_plugins((
        ExtractPlugin::<super::text::TextItem>::default(),
        ExtractPlugin::<super::typst::TypstText>::default(),
    ));

    app.add_systems(RenderSchedule::Prepare, pipeline::prepare_vitems)
        .add_systems(
            RenderSchedule::View,
            (
                pipeline::compute.in_set(ViewPhase::Compute),
                pipeline::depth.in_set(ViewPhase::Depth),
                pipeline::color.in_set(ViewPhase::Color),
            ),
        );
}

/// Compute a normal vector from the ordered VPoints of a RenderVItem.
///
/// The primary path uses the 3D equivalent of the shoelace formula on anchor
/// points. If the accumulated area is degenerate (for example, a single
/// curved segment), all VPoints are scanned for a non-collinear triple.
/// A collinear item uses a deterministic plane containing its line, while an
/// item whose points all coincide falls back to the Z axis.
pub fn vitem_normal_from_points(points: &[Vec4]) -> Vec3 {
    if points.len() < 3 {
        return Vec3::Z;
    }

    let point3 = |point: &Vec4| point.truncate();
    let origin = point3(&points[0]);

    // VPoints alternate anchors and handles, so triangulating the ordered
    // even-indexed anchors is Newell's method in fan form.
    let mut area_normal = Vec3::ZERO;
    let mut previous = origin;
    let mut scale_squared = 0.0_f32;
    for point in points.iter().step_by(2).skip(1) {
        let current = point3(point);
        let previous_offset = previous - origin;
        let current_offset = current - origin;
        area_normal += previous_offset.cross(current_offset);
        scale_squared = scale_squared
            .max(previous_offset.length_squared())
            .max(current_offset.length_squared());
        previous = current;
    }
    if area_normal.length_squared() > f32::EPSILON * scale_squared * scale_squared {
        return area_normal.normalize();
    }

    // Two-anchor curves have zero shoelace area, but a control point can
    // still determine their plane. Preserve point order so the sign remains
    // deterministic across animation frames.
    let mut direction: Option<Vec3> = None;
    for point in &points[1..] {
        let candidate = point3(point) - origin;
        let candidate_length_squared = candidate.length_squared();
        if candidate_length_squared <= f32::EPSILON {
            continue;
        }
        if let Some(direction) = direction {
            let normal = direction.cross(candidate);
            if normal.length_squared()
                > f32::EPSILON * direction.length_squared() * candidate_length_squared
            {
                return normal.normalize();
            }
        } else {
            direction = Some(candidate);
        }
    }

    if let Some(direction) = direction {
        let direction = direction.normalize();
        let reference = if direction.dot(Vec3::Z).abs() < 0.99 {
            Vec3::Z
        } else {
            Vec3::X
        };
        return (reference - direction * direction.dot(reference)).normalize();
    }

    Vec3::Z
}

#[derive(bevy_ecs::component::Component, Debug, Clone, PartialEq)]
/// A primitive for rendering a vitem.
pub struct RenderVItem {
    /// The normal vector of the projection target plane in local space.
    /// If `None`, the normal will be derived from the points at render time
    /// and converted to world space along with the item transform.
    pub normal: Option<Vec3>,
    /// The points of the item in local space.
    /// (x, y, z, is_closed)
    pub points: Vec<Vec4>,
    /// The local-to-world transform applied when flattening onto the plane.
    pub transform: Mat4,
    /// Fill rgbas, see [`Rgba`].
    pub fill_rgbas: Vec<Rgba>,
    /// Stroke rgbs, see [`Rgba`].
    pub stroke_rgbas: Vec<Rgba>,
    /// Stroke widths, see [`Width`].
    pub stroke_widths: Vec<Width>,
}

impl Default for RenderVItem {
    fn default() -> Self {
        Self {
            normal: None,
            points: vec![Vec4::ZERO; 3],
            transform: Mat4::IDENTITY,
            stroke_widths: vec![Width::default(); 2],
            stroke_rgbas: vec![Rgba::default(); 2],
            fill_rgbas: vec![Rgba::default(); 2],
        }
    }
}

#[cfg(test)]
mod tests {
    use ranim_core::glam::{Vec3, Vec4};

    use super::vitem_normal_from_points;

    fn point(point: Vec3) -> Vec4 {
        point.extend(0.0)
    }

    #[test]
    fn inferred_normal_matches_the_polygon_plane() {
        let anchors = [
            Vec3::new(2.0, -1.0, -1.0),
            Vec3::new(2.0, 1.0, -1.0),
            Vec3::new(2.0, 1.0, 1.0),
            Vec3::new(2.0, -1.0, 1.0),
            Vec3::new(2.0, -1.0, -1.0),
        ];
        let mut points = Vec::new();
        for edge in anchors.windows(2) {
            points.push(point(edge[0]));
            points.push(point((edge[0] + edge[1]) * 0.5));
        }
        points.push(point(*anchors.last().unwrap()));

        let normal = vitem_normal_from_points(&points);
        assert!(normal.abs_diff_eq(Vec3::X, 1e-6));
    }

    #[test]
    fn normal_inference_falls_back_for_degenerate_inputs() {
        // A single curved segment has no polygon loop: use its control points.
        let curved = [point(Vec3::ZERO), point(Vec3::Y), point(Vec3::X)];
        assert!(vitem_normal_from_points(&curved).abs_diff_eq(Vec3::NEG_Z, 1e-6));

        // Collinear geometry has no unique plane: use the default normal.
        let collinear = [point(Vec3::ZERO), point(Vec3::X), point(Vec3::X * 2.0)];
        assert_eq!(vitem_normal_from_points(&collinear), Vec3::Z);

        // A vertical line picks a plane that contains the line.
        let vertical = [point(Vec3::ZERO), point(Vec3::Z), point(Vec3::Z * 2.0)];
        let normal = vitem_normal_from_points(&vertical);
        assert!(normal.abs_diff_eq(Vec3::X, 1e-6));
        assert!(normal.dot(Vec3::Z).abs() < 1e-6);
    }
}
