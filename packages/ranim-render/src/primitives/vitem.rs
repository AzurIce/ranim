use crate::{
    RenderTextures,
    pipelines::{
        Map3dTo2dPipeline, VItemPipeline, map_3d_to_2d::ComputeBindGroup, vitem::RenderBindGroup,
    },
    utils::{PipelinesStorage, WgpuBuffer, WgpuContext, WgpuVecBuffer},
};
use glam::Vec4;
use ranim_core::{
    components::{rgba::Rgba, width::Width},
    core_item::vitem::VItemPrimitive,
};

use super::{Primitive, RenderCommand, RenderResource};

impl Primitive for VItemPrimitive {
    type RenderInstance = VItemRenderInstance;
}

/// [`VItemPrimitive`]'s render instance.
pub struct VItemRenderInstance {
    /// COMPUTE INPUT: (x, y, z, is_closed)
    pub(crate) points3d_buffer: WgpuVecBuffer<Vec4>,
    /// COMPUTE OUTPUT, RENDER INPUT: (x, y, is_closed, 0)
    pub(crate) points2d_buffer: WgpuVecBuffer<Vec4>, // Use vec4 for alignment
    /// COMPUTE OUTPUT, RENDER INPUT: (min_x, max_x, min_y, max_y, max_w)
    pub(crate) clip_info_buffer: WgpuVecBuffer<i32>,
    /// COMPUTE INPUT: point_cnt
    pub(crate) point_cnt_buffer: Option<WgpuBuffer<u32>>,

    /// RENDER INPUT
    pub(crate) fill_rgbas: WgpuVecBuffer<Rgba>,
    /// RENDER INPUT
    pub(crate) stroke_rgbas: WgpuVecBuffer<Rgba>,
    /// RENDER INPUT, COMPUTE INPUT
    pub(crate) stroke_widths: WgpuVecBuffer<Width>,

    /// COMPUTE BIND GROUP 1: 0-points3d, 1-points2d
    pub(crate) compute_bind_group: Option<ComputeBindGroup>,

    /// RENDER BIND GROUP 1: 0-points, 1-fill_rgbas, 2-stroke_rgbas, 3-stroke_widths
    pub(crate) render_bind_group: Option<RenderBindGroup>,
}

impl RenderResource for VItemRenderInstance {
    type Data = VItemPrimitive;

    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self {
        // info!("init");
        // info!("3d: {}", data.points2d.len());
        let points3d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 3d Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            &data.points2d,
        );
        // info!("2d: {}", data.points2d.len());
        let points2d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 2d Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            &data.points2d,
        );
        // info!("clip");
        let clip_info_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Clip Info Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            &[i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0],
        );
        // info!("point_cnt");
        let point_cnt_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Point Cnt Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            data.points2d.len() as u32,
        );
        // info!("fill: {}",data.fill_rgbas.len());
        let fill_rgbas = WgpuVecBuffer::new_init(
            ctx,
            Some("Fill Rgbas Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.fill_rgbas,
        );
        // info!("stroke: {}",data.stroke_rgbas.len());
        let stroke_rgbas = WgpuVecBuffer::new_init(
            ctx,
            Some("Stroke Rgbas Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.stroke_rgbas,
        );
        // info!("stroke_widths: {}",data.stroke_widths.len());
        let stroke_widths = WgpuVecBuffer::new_init(
            ctx,
            Some("Stroke Widths Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.stroke_widths,
        );

        // info!("compute_bind_group");
        let compute_bind_group = ComputeBindGroup::new(
            ctx,
            points3d_buffer.buffer.as_ref().unwrap(),
            stroke_widths.buffer.as_ref().unwrap(),
            points2d_buffer.buffer.as_ref().unwrap(),
            clip_info_buffer.buffer.as_ref().unwrap(),
            point_cnt_buffer.as_ref(),
        );

        // info!("render_bind_group");
        let render_bind_group = RenderBindGroup::new(
            ctx,
            points2d_buffer.buffer.as_ref().unwrap(),
            fill_rgbas.buffer.as_ref().unwrap(),
            stroke_rgbas.buffer.as_ref().unwrap(),
            stroke_widths.buffer.as_ref().unwrap(),
            clip_info_buffer.buffer.as_ref().unwrap(),
        );
        // info!("init done");

        Self {
            points3d_buffer,
            points2d_buffer,
            clip_info_buffer,
            point_cnt_buffer: Some(point_cnt_buffer),
            fill_rgbas,
            stroke_rgbas,
            stroke_widths,
            compute_bind_group: Some(compute_bind_group),
            render_bind_group: Some(render_bind_group),
        }
    }
    fn update(&mut self, ctx: &WgpuContext, data: &Self::Data) {
        if let Some(point_cnt_buffer) = self.point_cnt_buffer.as_mut() {
            point_cnt_buffer.set(ctx, data.points2d.len() as u32);
        } else {
            self.point_cnt_buffer = Some(WgpuBuffer::new_init(
                ctx,
                Some("Point Cnt Buffer"),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                data.points2d.len() as u32,
            ));
        }
        // Dynamic sized
        if [
            self.points3d_buffer.set(ctx, &data.points2d),
            self.fill_rgbas.set(ctx, &data.fill_rgbas),
            self.stroke_rgbas.set(ctx, &data.stroke_rgbas),
            self.stroke_widths.set(ctx, &data.stroke_widths),
            self.points2d_buffer.resize(ctx, data.points2d.len()),
            self.clip_info_buffer
                .set(ctx, &[i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0]),
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
                self.stroke_widths.buffer.as_ref().unwrap(),
                self.points2d_buffer.buffer.as_ref().unwrap(),
                self.clip_info_buffer.buffer.as_ref().unwrap(),
                self.point_cnt_buffer.as_ref().unwrap().as_ref(),
            ));
            self.render_bind_group = Some(RenderBindGroup::new(
                ctx,
                self.points2d_buffer.buffer.as_ref().unwrap(),
                self.fill_rgbas.buffer.as_ref().unwrap(),
                self.stroke_rgbas.buffer.as_ref().unwrap(),
                self.stroke_widths.buffer.as_ref().unwrap(),
                self.clip_info_buffer.buffer.as_ref().unwrap(),
            ));
        }
    }
}

impl RenderCommand for VItemRenderInstance {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        cpass.set_bind_group(1, self.compute_bind_group.as_ref().unwrap().as_ref(), &[]);
        cpass.dispatch_workgroups(self.points3d_buffer.len().div_ceil(256) as u32, 1, 1);
    }
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        rpass.set_bind_group(1, self.render_bind_group.as_ref().unwrap().as_ref(), &[]);
        rpass.draw(0..4, 0..1);
    }
    fn encode_render_command(
        &self,
        ctx: &WgpuContext,
        pipelines: &mut PipelinesStorage,
        encoder: &mut wgpu::CommandEncoder,
        uniforms_bind_group: &wgpu::BindGroup,
        render_textures: &RenderTextures,
        #[cfg(feature = "profiling")] profiler: &mut wgpu_profiler::GpuProfiler,
    ) {
        #[cfg(feature = "profiling")]
        let mut scope = profiler.scope("vitem rendering", encoder);
        {
            #[cfg(feature = "profiling")]
            let mut cpass = scope.scoped_compute_pass("VItem Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("VItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipelines.get_or_init::<Map3dTo2dPipeline>(ctx));
            cpass.set_bind_group(0, uniforms_bind_group, &[]);

            cpass.set_bind_group(1, self.compute_bind_group.as_ref().unwrap().as_ref(), &[]);
            cpass.dispatch_workgroups(self.points3d_buffer.len().div_ceil(256) as u32, 1, 1);
        }
        {
            let RenderTextures {
                // multisample_view,
                render_view,
                ..
            } = render_textures;
            let rpass_desc = wgpu::RenderPassDescriptor {
                label: Some("VItem Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // view: multisample_view,
                    // resolve_target: Some(render_view),
                    depth_slice: None,
                    view: render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            #[cfg(feature = "profiling")]
            let mut rpass = scope.scoped_render_pass("VItem Render Pass", rpass_desc);
            #[cfg(not(feature = "profiling"))]
            let mut rpass = encoder.begin_render_pass(&rpass_desc);
            rpass.set_pipeline(pipelines.get_or_init::<VItemPipeline>(ctx));
            rpass.set_bind_group(0, uniforms_bind_group, &[]);

            rpass.set_bind_group(1, self.render_bind_group.as_ref().unwrap().as_ref(), &[]);
            rpass.draw(0..4, 0..1);
        }
    }
    fn debug(&self, _ctx: &WgpuContext) {
        // let points2d = self.points2d_buffer.read_buffer(ctx).unwrap();
        // let points2d = bytemuck::cast_slice::<_, Vec4>(&points2d);
        // println!("points2d: {:?}", points2d);
    }
}
