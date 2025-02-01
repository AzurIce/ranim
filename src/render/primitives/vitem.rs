use glam::{ivec3, vec2, Vec2, Vec4, Vec4Swizzles};
use itertools::Itertools;
use log::trace;

use crate::{
    components::{rgba::Rgba, width::Width},
    context::WgpuContext,
    items::vitem::VItem,
    render::{
        pipelines::{
            map_3d_to_2d::ComputeBindGroup, vitem::RenderBindGroup, Map3dTo2dPipeline,
            VItemPipeline,
        },
        CameraFrame,
    },
    utils::{wgpu::WgpuVecBuffer, RenderResourceStorage},
};

use super::Primitive;

pub struct VItemPrimitive {
    vitem: VItem,
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
    pub(crate) compute_bind_group: ComputeBindGroup,

    /// RENDER BIND GROUP 1: 0-points, 1-fill_rgbas, 2-stroke_rgbas, 3-stroke_widths
    pub(crate) render_bind_group: RenderBindGroup,
}

impl Primitive for VItemPrimitive {
    type Entity = VItem;
    fn init(ctx: &WgpuContext, data: &Self::Entity) -> Self {
        trace!("init");
        let points3d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 3d Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.get_render_points(),
        );
        let points2d_buffer = WgpuVecBuffer::new(
            ctx,
            Some("Points 2d Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            data.vpoints.len(),
        );
        let clip_box_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Clip Box Buffer"),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &[
                Vec2::new(-1.0, -1.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
            ],
        );
        let fill_rgbas = WgpuVecBuffer::new_init(
            ctx,
            Some("Fill Rgbas Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.fill_rgbas,
        );
        let stroke_rgbas = WgpuVecBuffer::new_init(
            ctx,
            Some("Stroke Rgbas Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.stroke_rgbas,
        );
        let stroke_widths = WgpuVecBuffer::new_init(
            ctx,
            Some("Stroke Widths Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.stroke_widths,
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
            vitem: data.clone(),
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
    fn update(&mut self, wgpu_ctx: &crate::context::WgpuContext, data: &Self::Entity) {
        // trace!("update, data.vpoints: {:?}", data.vpoints);
        // trace!("update, data.fill_rgbas: {:?}", data.fill_rgbas);
        // trace!("points3d len: {}", self.points3d_buffer.len());
        // trace!("points2d len: {}", self.points3d_buffer.len());
        self.vitem = data.clone();
        if self.vitem.vpoints.is_empty() {
            return;
        }
        self.points3d_buffer
            .set(wgpu_ctx, &data.get_render_points());
        // trace!("set fill_rgbas");
        self.fill_rgbas.set(wgpu_ctx, &data.fill_rgbas);
        // trace!("set stroke_rgbas");
        self.stroke_rgbas.set(wgpu_ctx, &data.stroke_rgbas);
        // trace!("set stroke_widths");
        self.stroke_widths.set(wgpu_ctx, &data.stroke_widths);
        // trace!("resize points2d");
        if self.points2d_buffer.resize(wgpu_ctx, data.vpoints.len()) {
            // trace!("resized points2d, updating bind groups");
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
    }

    fn update_clip_info(&mut self, ctx: &WgpuContext, camera: &CameraFrame) {
        let corners = [-1, 1]
            .into_iter()
            .cartesian_product([-1, 1])
            .cartesian_product([-1, 1])
            .map(|((x, y), z)| {
                // trace!("{x} {y} {z}");
                self.vitem.vpoints.get_bounding_box_point(ivec3(x, y, z))
            })
            .map(|p| {
                let mut p = camera.view_projection_matrix() * p.extend(1.0);
                p /= p.w;
                p.xy()
            })
            .collect::<Vec<Vec2>>();
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (1.0f32, -1.0f32, 1.0f32, -1.0f32);
        for p in corners {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let max_width = self
            .vitem
            .stroke_widths
            .iter()
            .cloned()
            .reduce(|acc, w| acc.max(w))
            .map(|w| w.0)
            .unwrap_or(0.0);
        let radii = Vec2::splat(max_width) / camera.half_frame_size();
        min_x -= radii.x;
        min_y -= radii.y;
        max_x += radii.x;
        max_y += radii.y;

        let clip_box = [
            vec2(min_x, min_y),
            vec2(min_x, max_y),
            vec2(max_x, min_y),
            vec2(max_x, max_y),
        ];
        trace!("updated clip_box: {:?}", clip_box);
        self.clip_box_buffer.set(ctx, &clip_box);
    }
    fn encode_render_command(
        &mut self,
        ctx: &crate::context::WgpuContext,
        pipelines: &mut RenderResourceStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        multisample_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
    ) {
        if self.vitem.vpoints.is_empty() {
            return;
        }
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipelines.get_or_init::<Map3dTo2dPipeline>(&ctx));
            cpass.set_bind_group(0, uniforms_bind_group, &[]);

            cpass.set_bind_group(1, &*self.compute_bind_group, &[]);
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
            rpass.set_pipeline(pipelines.get_or_init::<VItemPipeline>(&ctx));
            rpass.set_bind_group(0, uniforms_bind_group, &[]);

            rpass.set_bind_group(1, &*self.render_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.clip_box_buffer.slice(..));
            rpass.draw(0..self.clip_box_buffer.get().len() as u32, 0..1);
        }
    }
}
