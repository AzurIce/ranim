use glam::{Mat4, Vec2};
use ranim_core::prelude::CameraFrame;

use crate::utils::{WgpuBuffer, WgpuContext};

/// Total normalized-depth span reserved for scene-order biasing.
///
/// Fragments whose true depth differs by less than this span may be reordered
/// by scene insertion order (later items on top); anything farther apart keeps
/// true depth ordering. The span is split evenly across all items of a frame
/// (`epsilon = DEPTH_ORDER_SPAN / item_count`) so the total offset does not
/// grow with scene size.
pub const DEPTH_ORDER_SPAN: f32 = 1e-4;

/// Lower bound for the per-order bias epsilon, in ulps of the f32 depth
/// buffer at mid-depth.
///
/// `DEPTH_ORDER_SPAN / item_count` degenerates below one depth-ulp once a
/// frame holds enough items (a text-heavy scene easily reaches thousands of
/// per-glyph items): adjacent orders then round to the same stored depth and
/// the bias no longer breaks ties. The floor keeps adjacent orders
/// distinguishable at any scene size; the trade-off is that once the floor is
/// active the effective "counts as coplanar" threshold grows linearly with
/// item count.
pub const MIN_ORDER_BIAS_ULPS: f32 = 16.0;

/// f32 spacing at normalized depth 0.5; depths in `[0.25, 1]` share it.
const DEPTH_ULP: f32 = f32::EPSILON / 2.0;

/// Per-order depth bias epsilon for a frame holding `item_count` items.
///
/// Evenly splits [`DEPTH_ORDER_SPAN`] across the items, clamped from below at
/// [`MIN_ORDER_BIAS_ULPS`] depth-ulps so the bias keeps breaking ties in
/// dense scenes.
pub fn depth_bias_epsilon(item_count: u32) -> f32 {
    (DEPTH_ORDER_SPAN / item_count.max(1) as f32).max(MIN_ORDER_BIAS_ULPS * DEPTH_ULP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epsilon_splits_span_for_small_scenes() {
        let eps = depth_bias_epsilon(10);
        assert!((eps - DEPTH_ORDER_SPAN / 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn epsilon_never_drops_below_the_ulp_floor() {
        let eps = depth_bias_epsilon(1839);
        assert_eq!(eps, MIN_ORDER_BIAS_ULPS * DEPTH_ULP);
        // adjacent orders must stay at least one depth-ulp apart
        assert!((eps * 1839.0) > MIN_ORDER_BIAS_ULPS * DEPTH_ULP);
    }
}

/// Uniforms for the camera
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportUniform {
    proj_mat: Mat4,
    view_mat: Mat4,
    half_frame_size: Vec2,
    /// Per-order depth bias epsilon, see [`depth_bias_epsilon`].
    bias_epsilon: f32,
    _padding: u32,
}
impl ViewportUniform {
    pub fn from_camera_frame(
        camera_frame: &CameraFrame,
        width: u32,
        height: u32,
        bias_epsilon: f32,
    ) -> Self {
        let ratio = width as f64 / height as f64;
        Self {
            proj_mat: camera_frame.projection_matrix(ratio).as_mat4(),
            view_mat: camera_frame.view_matrix().as_mat4(),
            half_frame_size: Vec2::new(
                (camera_frame.frame_height * ratio) as f32 / 2.0,
                camera_frame.frame_height as f32 / 2.0,
            ),
            bias_epsilon,
            _padding: 0,
        }
    }
    pub(crate) fn as_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}

pub struct ViewportBindGroup {
    pub bind_group: wgpu::BindGroup,
}

impl AsRef<wgpu::BindGroup> for ViewportBindGroup {
    fn as_ref(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl ViewportBindGroup {
    pub(crate) fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
        ctx.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Viewport Bind Group Layout"),
                entries: &[ViewportUniform::as_bind_group_layout_entry(0)],
            })
    }

    pub(crate) fn new(ctx: &WgpuContext, uniforms_buffer: &WgpuBuffer<ViewportUniform>) -> Self {
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Uniforms"),
            layout: &Self::bind_group_layout(ctx),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    uniforms_buffer.as_ref().as_entire_buffer_binding(),
                ),
            }],
        });
        Self { bind_group }
    }
}

#[derive(bevy_ecs::prelude::Resource)]
pub struct ViewportGpuPacket {
    pub(crate) uniforms_buffer: WgpuBuffer<ViewportUniform>,
    pub(crate) uniforms_bind_group: ViewportBindGroup,
}

impl ViewportGpuPacket {
    pub(crate) fn new(ctx: &WgpuContext, data: &ViewportUniform) -> Self {
        let uniforms_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Uniforms Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            *data,
        );
        let uniforms_bind_group = ViewportBindGroup::new(ctx, &uniforms_buffer);

        Self {
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub(crate) fn update(&mut self, ctx: &WgpuContext, data: &ViewportUniform) {
        self.uniforms_buffer.set(ctx, *data);
    }
}
