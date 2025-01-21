use glam::{Vec2, Vec4};
use log::trace;

use crate::{
    context::WgpuContext,
    items::vitem::ExtractedVItem,
    rabject,
    render::{
        pipelines::{
            map_3d_to_2d::ComputeBindGroup, vitem::RenderBindGroup, Map3dTo2dPipeline,
            VItemPipeline,
        },
        WgpuBuffer,
    },
};

use super::Primitive;

pub struct VItemPrimitive {
    /// COMPUTE INPUT: (x, y, z, is_closed)
    pub(crate) points3d_buffer: WgpuBuffer<Vec4>,
    /// COMPUTE OUTPUT, RENDER INPUT: (x, y, is_closed, 0)
    pub(crate) points2d_buffer: WgpuBuffer<Vec4>, // Use vec4 for alignment

    /// RENDER VERTEX INPUT
    pub(crate) clip_box_buffer: WgpuBuffer<Vec2>,
    /// RENDER INPUT
    pub(crate) fill_rgbas: WgpuBuffer<Vec4>,
    /// RENDER INPUT
    pub(crate) stroke_rgbas: WgpuBuffer<Vec4>,
    /// RENDER INPUT
    pub(crate) stroke_widths: WgpuBuffer<f32>,

    /// COMPUTE BIND GROUP 1: 0-points3d, 1-points2d
    pub(crate) compute_bind_group: ComputeBindGroup,

    /// RENDER BIND GROUP 1: 0-points, 1-fill_rgbas, 2-stroke_rgbas, 3-stroke_widths
    pub(crate) render_bind_group: RenderBindGroup,
}

impl rabject::Primitive for VItemPrimitive {
    type Data = ExtractedVItem;
    fn init(wgpu_ctx: &WgpuContext, data: &Self::Data) -> Self {
        Primitive::init(wgpu_ctx, data)
    }
    fn update(&mut self, wgpu_ctx: &WgpuContext, data: &Self::Data) {
        Primitive::update(self, wgpu_ctx, data)
    }
    fn render(
        &self,
        wgpu_ctx: &WgpuContext,
        pipelines: &mut crate::utils::RenderResourceStorage,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        depth_stencil_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    ) {
        Primitive::render(
            self,
            wgpu_ctx,
            pipelines,
            multisample_view,
            target_view,
            depth_stencil_view,
            uniforms_bind_group,
        );
    }
}

impl Primitive for VItemPrimitive {
    type Data = ExtractedVItem;
    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self {
        trace!("init");
        let points3d_buffer = WgpuBuffer::new_init(
            ctx,
            &data.points,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let points2d_buffer = WgpuBuffer::new(
            ctx,
            (std::mem::size_of::<Vec4>() * points3d_buffer.len()) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let clip_box_buffer = WgpuBuffer::new_init(
            ctx,
            &[
                Vec2::new(-1.0, -1.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
            ],
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let fill_rgbas = WgpuBuffer::new_init(
            ctx,
            &data.fill_rgbas,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let stroke_rgbas = WgpuBuffer::new_init(
            ctx,
            &data.stroke_rgbas,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let stroke_widths = WgpuBuffer::new_init(
            ctx,
            &data.stroke_widths,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let compute_bind_group = ComputeBindGroup::new(ctx, &points3d_buffer, &points2d_buffer);
        let render_bind_group = RenderBindGroup::new(
            ctx,
            &points2d_buffer,
            &fill_rgbas,
            &stroke_rgbas,
            &stroke_widths,
        );
        Self {
            points3d_buffer,
            points2d_buffer,
            clip_box_buffer,
            fill_rgbas,
            stroke_rgbas,
            stroke_widths,

            compute_bind_group,
            render_bind_group,
        }
    }
    fn update(&mut self, wgpu_ctx: &crate::context::WgpuContext, data: &Self::Data) {
        trace!("update, data: {:?}", data.points);
        trace!("points3d len: {}", self.points3d_buffer.len());
        trace!("points2d len: {}", self.points3d_buffer.len());
        self.points3d_buffer
            .prepare_from_slice(wgpu_ctx, &data.points);
        self.fill_rgbas
            .prepare_from_slice(wgpu_ctx, &data.fill_rgbas);
        self.stroke_rgbas
            .prepare_from_slice(wgpu_ctx, &data.stroke_rgbas);
        self.stroke_widths
            .prepare_from_slice(wgpu_ctx, &data.stroke_widths);
        if self.points2d_buffer.len() < data.points.len() {
            self.points2d_buffer = WgpuBuffer::new(
                wgpu_ctx,
                (std::mem::size_of::<Vec4>() * data.points.len()) as u64,
                wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::VERTEX,
            );
            self.compute_bind_group
                .update(&wgpu_ctx, &self.points3d_buffer, &self.points2d_buffer);
            self.render_bind_group.update(
                &wgpu_ctx,
                &self.points2d_buffer,
                &self.fill_rgbas,
                &self.stroke_rgbas,
                &self.stroke_widths,
            );
        }
        self.points2d_buffer.set_len(data.points.len());
    }
    fn render(
        &self,
        wgpu_ctx: &crate::context::WgpuContext,
        pipelines: &mut crate::utils::RenderResourceStorage,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        _depth_stencil_view: &wgpu::TextureView,
        uniforms_bind_group: &wgpu::BindGroup,
    ) {
        trace!("render");
        let mut encoder = wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipelines.get_or_init::<Map3dTo2dPipeline>(&wgpu_ctx));
            cpass.set_bind_group(0, uniforms_bind_group, &[]);
            cpass.set_bind_group(1, &*self.compute_bind_group, &[]);
            cpass.dispatch_workgroups(((self.points3d_buffer.len() + 255) / 256) as u32, 1, 1);
        }
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VItem Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: multisample_view,
                    resolve_target: Some(target_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(pipelines.get_or_init::<VItemPipeline>(&wgpu_ctx));
            rpass.set_bind_group(0, uniforms_bind_group, &[]);
            rpass.set_bind_group(1, &*self.render_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.clip_box_buffer.slice(..));
            rpass.draw(0..self.clip_box_buffer.len() as u32, 0..1);
        }
        wgpu_ctx.queue.submit(Some(encoder.finish()));
    }
}
