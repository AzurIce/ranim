use glam::{Mat4, Vec2};
use ranim_core::prelude::CameraFrame;

use crate::{
    primitives::{Primitive, RenderResource},
    utils::{WgpuBuffer, WgpuContext},
};

/// Uniforms for the camera
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportUniform {
    proj_mat: Mat4,
    view_mat: Mat4,
    half_frame_size: Vec2,
    _padding: [u32; 2],
}
impl Primitive for ViewportUniform {
    type RenderPacket = ViewportGpuPacket;
}

impl ViewportUniform {
    pub fn from_camera_frame(camera_frame: &CameraFrame, width: u32, height: u32) -> Self {
        let ratio = width as f64 / height as f64;
        Self {
            proj_mat: camera_frame.projection_matrix(ratio).as_mat4(),
            view_mat: camera_frame.view_matrix().as_mat4(),
            half_frame_size: Vec2::new(
                (camera_frame.frame_height * ratio) as f32 / 2.0,
                camera_frame.frame_height as f32 / 2.0,
            ),
            _padding: [0; 2],
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

pub struct ViewportGpuPacket {
    pub(crate) uniforms_buffer: WgpuBuffer<ViewportUniform>,
    pub(crate) uniforms_bind_group: ViewportBindGroup,
}

impl RenderResource for ViewportGpuPacket {
    type Data = ViewportUniform;

    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self {
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

    fn update(&mut self, ctx: &WgpuContext, data: &Self::Data) {
        self.uniforms_buffer.set(ctx, *data);
    }
}
