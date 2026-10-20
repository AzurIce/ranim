use glam::{Mat4, Vec2};

use crate::utils::{WgpuBuffer, WgpuContext};

/// Total normalized-depth span reserved for scene-order biasing.
///
/// Fragments whose true depth differs by less than this span may be reordered
/// by scene insertion order (later items on top); anything farther apart keeps
/// true depth ordering. The per-order epsilon divides this span by the frame's
/// full scene-order span (at least one). This keeps offsets bounded even when
/// omitted render plugins leave gaps in the prepared items' scene orders.
pub const DEPTH_ORDER_SPAN: f32 = 1e-4;

/// Uniforms for the camera
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportUniform {
    proj_mat: Mat4,
    view_mat: Mat4,
    half_frame_size: Vec2,
    /// Per-order depth bias epsilon, see [`DEPTH_ORDER_SPAN`].
    bias_epsilon: f32,
    _padding: u32,
}

impl ViewportUniform {
    /// Build a view packet from projection, view and frame-size data.
    pub fn new(proj_mat: Mat4, view_mat: Mat4, half_frame_size: Vec2, bias_epsilon: f32) -> Self {
        Self {
            proj_mat,
            view_mat,
            half_frame_size,
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

impl Default for ViewportUniform {
    fn default() -> Self {
        Self::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec2::ONE, 0.0)
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
    pub fn bind_group_layout(ctx: &WgpuContext) -> wgpu::BindGroupLayout {
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
    /// Bind the current view data in an item rendering pipeline.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.uniforms_bind_group.bind_group
    }

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

    pub fn update(&mut self, ctx: &WgpuContext, data: &ViewportUniform) {
        self.uniforms_buffer.set(ctx, *data);
    }
}
