use crate::{
    components::{rgba::Rgba, width::Width},
    context::WgpuContext,
    render::pipelines::{
        map_3d_to_2d::ComputeBindGroup, vitem::RenderBindGroup, Map3dTo2dPipeline, VItemPipeline,
    },
    utils::{wgpu::WgpuVecBuffer, RenderResourceStorage},
};
use glam::{Vec2, Vec4};

use super::RenderInstance;

pub struct VItemPrimitive {
    /// COMPUTE INPUT: (x, y, z, is_closed)
    pub(crate) points3d_buffer: WgpuVecBuffer<Vec4>,
    /// COMPUTE OUTPUT, RENDER INPUT: (x, y, is_closed, 0)
    pub(crate) points2d_buffer: WgpuVecBuffer<Vec4>, // Use vec4 for alignment

    /// RENDER VERTEX INPUT
    pub(crate) clip_box_buffer: WgpuVecBuffer<Vec2>,
    /// RENDER INPUT
    pub(crate) fill_rgbas: WgpuVecBuffer<Rgba>,
    /// RENDER INPUT
    pub(crate) stroke_rgbas: WgpuVecBuffer<Rgba>,
    /// RENDER INPUT
    pub(crate) stroke_widths: WgpuVecBuffer<Width>,

    /// COMPUTE BIND GROUP 1: 0-points3d, 1-points2d
    pub(crate) compute_bind_group: Option<ComputeBindGroup>,

    /// RENDER BIND GROUP 1: 0-points, 1-fill_rgbas, 2-stroke_rgbas, 3-stroke_widths
    pub(crate) render_bind_group: Option<RenderBindGroup>,
}

impl Default for VItemPrimitive {
    fn default() -> Self {
        let points3d_buffer = WgpuVecBuffer::new(
            Some("Points 3d Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let points2d_buffer = WgpuVecBuffer::new(
            Some("Points 2d Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let clip_box_buffer = WgpuVecBuffer::new(
            Some("Clip Box Buffer"),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let fill_rgbas = WgpuVecBuffer::new(
            Some("Fill Rgbas Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let stroke_rgbas = WgpuVecBuffer::new(
            Some("Stroke Rgbas Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let stroke_widths = WgpuVecBuffer::new(
            Some("Stroke Widths Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );

        Self {
            points3d_buffer,
            points2d_buffer,
            clip_box_buffer,
            fill_rgbas,
            stroke_rgbas,
            stroke_widths,

            compute_bind_group: None,
            render_bind_group: None,
        }
    }
}

impl VItemPrimitive {
    pub fn update(
        &mut self,
        ctx: &WgpuContext,
        // clip_box: &[Vec2; 4],
        render_points: &[Vec4],
        fill_rgbas: &[Rgba],
        stroke_rgbas: &[Rgba],
        stroke_widths: &[Width],
    ) {
        // trace!(
        //     "VItemPrimitive update: {} {} {} {}",
        //     render_points.len(),
        //     fill_rgbas.len(),
        //     stroke_rgbas.len(),
        //     stroke_widths.len()
        // );
        // // Fixed sized
        // self.clip_box_buffer.set(ctx, clip_box);

        // Dynamic sized
        if [
            self.points3d_buffer.set(ctx, render_points),
            self.fill_rgbas.set(ctx, fill_rgbas),
            self.stroke_rgbas.set(ctx, stroke_rgbas),
            self.stroke_widths.set(ctx, stroke_widths),
            self.points2d_buffer.resize(ctx, render_points.len()),
            // This two should be all none or all some
            self.compute_bind_group.is_none(),
            // self.render_bind_group.is_none(),
        ]
        .iter()
        .any(|x| *x)
        {
            self.compute_bind_group = Some(ComputeBindGroup::new(
                ctx,
                self.points3d_buffer.buffer.as_ref().unwrap(),
                self.points2d_buffer.buffer.as_ref().unwrap(),
            ));
            self.render_bind_group = Some(RenderBindGroup::new(
                ctx,
                self.points2d_buffer.buffer.as_ref().unwrap(),
                self.fill_rgbas.buffer.as_ref().unwrap(),
                self.stroke_rgbas.buffer.as_ref().unwrap(),
                self.stroke_widths.buffer.as_ref().unwrap(),
            ));
        }
    }
}

impl RenderInstance for VItemPrimitive {
    fn update_clip_box(&mut self, ctx: &WgpuContext, clip_box: &[Vec2; 4]) {
        // trace!("VItemPrimitive update_clip_box: {:?}", clip_box);
        self.clip_box_buffer.set(ctx, clip_box);
    }
    fn encode_render_command(
        &mut self,
        ctx: &WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipelines.get_or_init::<Map3dTo2dPipeline>(ctx));
            cpass.set_bind_group(0, uniforms_bind_group, &[]);

            cpass.set_bind_group(1, self.compute_bind_group.as_ref().unwrap().as_ref(), &[]);
            cpass.dispatch_workgroups(
                ((self.points3d_buffer.get().len() + 255) / 256) as u32,
                1,
                1,
            );
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
            rpass.set_pipeline(pipelines.get_or_init::<VItemPipeline>(ctx));
            rpass.set_bind_group(0, uniforms_bind_group, &[]);

            rpass.set_bind_group(1, self.render_bind_group.as_ref().unwrap().as_ref(), &[]);
            rpass.set_vertex_buffer(0, self.clip_box_buffer.buffer.as_ref().unwrap().slice(..));
            rpass.draw(0..self.clip_box_buffer.get().len() as u32, 0..1);
        }
    }
}
