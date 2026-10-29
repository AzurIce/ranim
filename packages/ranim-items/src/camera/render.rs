use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use ranim_core::{glam::Vec2, traits::ApplyTransform};
use ranim_render::{
    extract::{Extract, ExtractContext, ExtractPlugin},
    primitives::viewport::{DEPTH_ORDER_SPAN, ViewportGpuPacket, ViewportUniform},
    schedule::{RenderDimensions, RenderSchedule},
    utils::WgpuContext,
    world::SceneOrder,
};

use super::CameraFrame;

/// Installs camera extraction and uploads the active view before rendering.
///
/// Each rendered frame must contain exactly one extracted [`CameraFrame`].
/// Composite frame payloads require an [`ExtractPlugin`] for their exact type.
pub struct CameraFramePlugin;

impl Plugin for CameraFramePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractPlugin::<CameraFrame>::default())
            .add_systems(RenderSchedule::Prepare, prepare_camera);
    }
}

impl Extract for CameraFrame {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        let mut value = self.clone();
        value.apply(ctx.transform());
        let mut entity = ctx.entity::<CameraFrame>();
        if entity.get::<CameraFrame>() != Some(&value) {
            entity.insert(value);
        }
    }
}

fn prepare_camera(
    ctx: Res<WgpuContext>,
    dimensions: Res<RenderDimensions>,
    cameras: Query<(&SceneOrder, &CameraFrame)>,
    orders: Query<&SceneOrder>,
    mut viewport: ResMut<ViewportGpuPacket>,
) {
    let camera = single_camera(cameras.iter().map(|(_, camera)| camera));
    let aspect_ratio = dimensions.width as f64 / dimensions.height as f64;
    let order_span = orders.iter().map(|order| order.0).max().unwrap_or(0);
    let uniform = ViewportUniform::new(
        camera.projection_matrix(aspect_ratio).as_mat4(),
        camera.view_matrix().as_mat4(),
        Vec2::new(
            (camera.frame_height * aspect_ratio) as f32 / 2.0,
            camera.frame_height as f32 / 2.0,
        ),
        DEPTH_ORDER_SPAN / order_span.max(1) as f32,
    );
    viewport.update(&ctx, &uniform);
}

fn single_camera<'a>(mut cameras: impl Iterator<Item = &'a CameraFrame>) -> &'a CameraFrame {
    let camera = cameras
        .next()
        .expect("D0002 requires exactly one active CameraFrame per rendered frame");
    assert!(
        cameras.next().is_none(),
        "D0002 requires exactly one active CameraFrame per rendered frame"
    );
    camera
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_camera_is_accepted() {
        let camera = CameraFrame::default();
        assert_eq!(single_camera([&camera].into_iter()), &camera);
    }

    #[test]
    #[should_panic(expected = "D0002 requires exactly one active CameraFrame")]
    fn missing_camera_is_rejected() {
        single_camera(std::iter::empty());
    }

    #[test]
    #[should_panic(expected = "D0002 requires exactly one active CameraFrame")]
    fn multiple_cameras_are_rejected() {
        let camera = CameraFrame::default();
        single_camera([&camera, &camera].into_iter());
    }
}
