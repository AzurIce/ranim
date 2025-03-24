use crate::{
    components::{rgba::Rgba, width::Width},
    context::WgpuContext,
    render::{
        pipelines::{
            nvitem::RenderBindGroup, nvitem_map_points::ComputeBindGroup, NVItemMapPointsPipeline, NVItemPipeline
        }, RenderTextures
    },
    utils::{wgpu::{WgpuBuffer, WgpuVecBuffer}, PipelinesStorage},
};
use glam::Vec4;

use super::RenderInstance;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NVPoint {
    pub prev_handle: Vec4,
    pub anchor: Vec4,
    pub next_handle: Vec4,
    pub closepath: f32,
    pub _padding: [f32; 3],
}

pub struct NVItemPrimitive {
    /// COMPUTE STORAGE: NVPoint
    pub(crate) points3d_buffer: WgpuVecBuffer<NVPoint>,
    /// COMPUTE INPUT AND OUTPUT, RENDER INPUT: NVPoint
    pub(crate) points2d_buffer: WgpuVecBuffer<NVPoint>,
    /// COMPUTE INPUT
    pub(crate) points_len: Option<WgpuBuffer<u32>>,
    /// COMPUTE OUTPUT, RENDER INPUT: (min_x, max_x, min_y, max_y, max_w)
    pub(crate) clip_info_buffer: WgpuVecBuffer<i32>,

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

impl Default for NVItemPrimitive {
    fn default() -> Self {
        let points3d_buffer = WgpuVecBuffer::new(
            Some("Points 3d Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        );
        let points2d_buffer = WgpuVecBuffer::new(
            Some("Points 2d Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        );
        let clip_info_buffer = WgpuVecBuffer::new(
            Some("Clip Info Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
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
            points_len: None,
            clip_info_buffer,
            fill_rgbas,
            stroke_rgbas,
            stroke_widths,

            compute_bind_group: None,
            render_bind_group: None,
        }
    }
}

impl NVItemPrimitive {
    pub fn update(
        &mut self,
        ctx: &WgpuContext,
        render_points: &[NVPoint],
        fill_rgbas: &[Rgba],
        stroke_rgbas: &[Rgba],
        stroke_widths: &[Width],
    ) {
        let len = render_points.len();
        if self.points_len.is_none() {
            self.points_len = Some(WgpuBuffer::new_init(
                ctx,
                Some("Points Len Buffer"),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                len as u32,
            ));
        } else {
            self.points_len.as_mut().unwrap().set(ctx, len as u32);
        }
        // println!("render_points: {:?}", render_points);
        // println!("fill_rgbas: {:?}", fill_rgbas);
        // println!("stroke_rgbas: {:?}", stroke_rgbas);
        // println!("stroke_widths: {:?}", stroke_widths);
        // Dynamic sized
        if [
            self.points3d_buffer.set(ctx, render_points),
            self.fill_rgbas.set(ctx, fill_rgbas),
            self.stroke_rgbas.set(ctx, stroke_rgbas),
            self.stroke_widths.set(ctx, stroke_widths),
            self.points2d_buffer.resize(ctx, render_points.len()),
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
                self.points_len.as_ref().unwrap().as_ref(),
                self.clip_info_buffer.buffer.as_ref().unwrap(),
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

impl RenderInstance for NVItemPrimitive {
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
        let mut scope = profiler.scope("nvitem rendering", encoder);
        {
            #[cfg(feature = "profiling")]
            let mut cpass = scope.scoped_compute_pass("NVItem Map Points Compute Pass");
            #[cfg(not(feature = "profiling"))]
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("NVItem Map Points Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipelines.get_or_init::<NVItemMapPointsPipeline>(ctx));
            cpass.set_bind_group(0, uniforms_bind_group, &[]);

            cpass.set_bind_group(1, self.compute_bind_group.as_ref().unwrap().as_ref(), &[]);
            cpass.dispatch_workgroups(
                ((self.points3d_buffer.get().len() + 255) / 256) as u32,
                1,
                1,
            );
        }

        {
            let RenderTextures {
                // multisample_view,
                render_view,
                ..
            } = render_textures;
            let rpass_desc = wgpu::RenderPassDescriptor {
                label: Some("NVItem Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // view: multisample_view,
                    // resolve_target: Some(render_view),
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
            let mut rpass = scope.scoped_render_pass("NVItem Render Pass", rpass_desc);
            #[cfg(not(feature = "profiling"))]
            let mut rpass = encoder.begin_render_pass(&rpass_desc);
            rpass.set_pipeline(pipelines.get_or_init::<NVItemPipeline>(ctx));
            rpass.set_bind_group(0, uniforms_bind_group, &[]);

            rpass.set_bind_group(1, self.render_bind_group.as_ref().unwrap().as_ref(), &[]);
            rpass.draw(0..4, 0..1);
        }
    }
    fn debug(&self, ctx: &WgpuContext) {
        let res = self.points2d_buffer.read_buffer(ctx).unwrap();
        let res: &[NVPoint] = bytemuck::cast_slice(&res);
        println!("points2d: {:?}", res);
    }
}

#[cfg(test)]
mod test {
    use glam::{vec2, vec3, Vec2, Vec3};
    use image::Rgba;

    use crate::{
        components::ScaleHint, context::WgpuContext, items::{
            camera_frame::CameraFrame, nvitem::{NVItem, NVItemBuilder}, Blueprint
        }, prelude::Transformable, render::{
            primitives::{nvitem::NVPoint, ExtractFrom, RenderInstance}, CameraUniforms, CameraUniformsBindGroup, RenderTextures
        }, utils::{get_texture_data, wgpu::WgpuBuffer, PipelinesStorage}
    };

    use super::NVItemPrimitive;

    #[test]
    fn test() {
        let ctx = pollster::block_on(WgpuContext::new());
        let mut nvitem = NVItemBuilder::new();
        nvitem.move_to(vec3(-3.4890716, 2.2969427, 0.0));
        nvitem.cubic_to(
            vec3(-3.5152762, 2.2969427, 0.0),
            vec3(-3.5327399, 2.2794755, 0.0),
            vec3(-3.5327399, 2.2445414, 0.0),
        );
        // nvitem.close_path();
        let mut nvitem = nvitem.build();

        nvitem
            .scale_to(ScaleHint::PorportionalHeight(8.0))
            .put_center_on(Vec3::ZERO);
        let mut nvitem_primitive = NVItemPrimitive::default();
        let mut pipelines = PipelinesStorage::default();
        let (width, height) = (1920, 1080);
        let frame_size = vec2(8.0 * 16.0 / 9.0, 8.0);
        let camera = CameraFrame::new();
        let uniforms = CameraUniforms {
            proj_mat: camera.projection_matrix(frame_size.y, 16.0 / 9.0),
            view_mat: camera.view_matrix(),
            half_frame_size: vec2(width as f32 / 2.0, height as f32 / 2.0),
            _padding: [0.0; 2],
        };
        let uniforms_buffer = WgpuBuffer::new_init(
            &ctx,
            Some("Uniforms Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            uniforms,
        );
        let camera_bind_group = CameraUniformsBindGroup::new(&ctx, &uniforms_buffer);
        let render_textures = RenderTextures::new(&ctx, width, height);

        let mut wgpu_profiler = wgpu_profiler::GpuProfiler::new(
            &ctx.device,
            wgpu_profiler::GpuProfilerSettings::default(),
        )
        .unwrap();

        nvitem_primitive.update_from(&ctx, &nvitem);
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        nvitem_primitive.encode_render_command(
            &ctx,
            &mut pipelines,
            &mut encoder,
            &camera_bind_group.bind_group,
            &render_textures,
            &mut wgpu_profiler
        );
        ctx.queue.submit(Some(encoder.finish()));

        // let texture_data = get_texture_data(&ctx, &render_textures.render_texture);
        // let texture_data: &[u8] = bytemuck::cast_slice(&texture_data);
        // let img_buffer = image::ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(
        //     width as u32,
        //     height as u32,
        //     texture_data,
        // )
        // .unwrap();
        // img_buffer.save("test.png").unwrap();

        let res = nvitem_primitive.points3d_buffer.read_buffer(&ctx).unwrap();
        let res: &[NVPoint] = bytemuck::cast_slice(&res);
        println!("points3d: {:?}", res);

        let res = nvitem_primitive.points2d_buffer.read_buffer(&ctx).unwrap();
        let res: &[NVPoint] = bytemuck::cast_slice(&res);
        println!("points2d: {:?}", res);
        // ctx.queue.submit(Some(encoder.finish()));
        let res = nvitem_primitive.clip_info_buffer.read_buffer(&ctx).unwrap();
        let res: &[i32] = bytemuck::cast_slice(&res);
        println!("clip_info: {:?}", res);
    }
}
