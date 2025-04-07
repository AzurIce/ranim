use crate::{
    components::{rgba::Rgba, width::Width},
    context::WgpuContext,
    render::{
        RenderTextures,
        pipelines::{
            Map3dTo2dPipeline, VItemPipeline, map_3d_to_2d::ComputeBindGroup,
            vitem::RenderBindGroup,
        },
    },
    utils::{
        PipelinesStorage,
        wgpu::{WgpuBuffer, WgpuVecBuffer},
    },
};
use glam::Vec4;

use super::{Primitive, Renderable};

pub struct VItemPrimitive {
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

pub struct VItemPrimitiveData {
    pub(crate) points2d: Vec<Vec4>,
    pub(crate) fill_rgbas: Vec<Rgba>,
    pub(crate) stroke_rgbas: Vec<Rgba>,
    pub(crate) stroke_widths: Vec<Width>,
}

impl Primitive for VItemPrimitive {
    type Data = VItemPrimitiveData;

    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self {
        let points3d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 3d Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.points2d,
        );
        let points2d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 2d Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &data.points2d,
        );
        let clip_info_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Clip Info Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &[i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0],
        );
        let point_cnt_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Point Cnt Buffer"),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            data.points2d.len() as u32,
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

        let compute_bind_group = ComputeBindGroup::new(
            ctx,
            points3d_buffer.buffer.as_ref().unwrap(),
            stroke_widths.buffer.as_ref().unwrap(),
            points2d_buffer.buffer.as_ref().unwrap(),
            clip_info_buffer.buffer.as_ref().unwrap(),
            point_cnt_buffer.as_ref(),
        );

        let render_bind_group = RenderBindGroup::new(
            ctx,
            points2d_buffer.buffer.as_ref().unwrap(),
            fill_rgbas.buffer.as_ref().unwrap(),
            stroke_rgbas.buffer.as_ref().unwrap(),
            stroke_widths.buffer.as_ref().unwrap(),
            clip_info_buffer.buffer.as_ref().unwrap(),
        );

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
        if self.point_cnt_buffer.is_none() {
            self.point_cnt_buffer = Some(WgpuBuffer::new_init(
                ctx,
                Some("Point Cnt Buffer"),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                data.points2d.len() as u32,
            ));
        } else {
            self.point_cnt_buffer
                .as_mut()
                .unwrap()
                .set(ctx, data.points2d.len() as u32);
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

impl Default for VItemPrimitive {
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
            clip_info_buffer,
            point_cnt_buffer: None,
            fill_rgbas,
            stroke_rgbas,
            stroke_widths,

            compute_bind_group: None,
            render_bind_group: None,
        }
    }
}

// impl VItemPrimitive {
//     pub fn update(
//         &mut self,
//         ctx: &WgpuContext,
//         render_points: &[Vec4],
//         fill_rgbas: &[Rgba],
//         stroke_rgbas: &[Rgba],
//         stroke_widths: &[Width],
//     ) {
//         // println!("update vitem primitive");
//         // println!("render_points: {:?}", render_points);
//         if self.point_cnt_buffer.is_none() {
//             self.point_cnt_buffer = Some(WgpuBuffer::new_init(
//                 ctx,
//                 Some("Point Cnt Buffer"),
//                 wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
//                 render_points.len() as u32,
//             ));
//         } else {
//             self.point_cnt_buffer
//                 .as_mut()
//                 .unwrap()
//                 .set(ctx, render_points.len() as u32);
//         }
//         // Dynamic sized
//         if [
//             self.points3d_buffer.set(ctx, render_points),
//             self.fill_rgbas.set(ctx, fill_rgbas),
//             self.stroke_rgbas.set(ctx, stroke_rgbas),
//             self.stroke_widths.set(ctx, stroke_widths),
//             self.points2d_buffer.resize(ctx, render_points.len()),
//             self.clip_info_buffer
//                 .set(ctx, &[i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0]),
//             // This two should be all none or all some
//             self.compute_bind_group.is_none(),
//             // self.render_bind_group.is_none(),
//         ]
//         .iter()
//         .any(|x| *x)
//         {
//             self.compute_bind_group = Some(ComputeBindGroup::new(
//                 ctx,
//                 self.points3d_buffer.buffer.as_ref().unwrap(),
//                 self.stroke_widths.buffer.as_ref().unwrap(),
//                 self.points2d_buffer.buffer.as_ref().unwrap(),
//                 self.clip_info_buffer.buffer.as_ref().unwrap(),
//                 self.point_cnt_buffer.as_ref().unwrap().as_ref(),
//             ));
//             self.render_bind_group = Some(RenderBindGroup::new(
//                 ctx,
//                 self.points2d_buffer.buffer.as_ref().unwrap(),
//                 self.fill_rgbas.buffer.as_ref().unwrap(),
//                 self.stroke_rgbas.buffer.as_ref().unwrap(),
//                 self.stroke_widths.buffer.as_ref().unwrap(),
//                 self.clip_info_buffer.buffer.as_ref().unwrap(),
//             ));
//         }
//     }
// }

impl Renderable for VItemPrimitive {
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

#[cfg(test)]
mod test {
    use glam::{Vec4, dvec2};
    use image::Rgba;

    use crate::{
        context::WgpuContext,
        items::{Blueprint, camera_frame::CameraFrame, vitem::Square},
        render::{
            CameraUniforms, CameraUniformsBindGroup, RenderTextures,
            primitives::{Extract, Primitive, Renderable},
        },
        utils::{PipelinesStorage, get_texture_data, wgpu::WgpuBuffer},
    };

    use super::VItemPrimitive;

    #[test]
    fn test() {
        let ctx = pollster::block_on(WgpuContext::new());

        #[cfg(feature = "profiling")]
        let mut wgpu_profiler = wgpu_profiler::GpuProfiler::new(
            &ctx.device,
            wgpu_profiler::GpuProfilerSettings::default(),
        )
        .unwrap();
        let vitem = Square(8.0).build();

        let mut pipelines = PipelinesStorage::default();
        let vitem_primitive = VItemPrimitive::init(&ctx, &vitem.extract());

        let (width, height) = (1920, 1080);
        let frame_size = dvec2(8.0 * width as f64 / height as f64, 8.0);

        let camera = CameraFrame::new();
        let uniforms = CameraUniforms {
            proj_mat: camera
                .projection_matrix(frame_size.y, width as f64 / height as f64)
                .as_mat4(),
            view_mat: camera.view_matrix().as_mat4(),
            half_frame_size: (frame_size / 2.0).as_vec2(),
            _padding: [0.0; 2],
        };
        let uniforms_buffer = WgpuBuffer::new_init(
            &ctx,
            Some("Uniforms Buffer"),
            wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            uniforms,
        );
        let uniforms = uniforms_buffer.read_buffer(&ctx);
        let uniforms: &[CameraUniforms] = bytemuck::cast_slice(&uniforms);
        println!("uniforms: {:?}", uniforms);
        let camera_bind_group = CameraUniformsBindGroup::new(&ctx, &uniforms_buffer);
        let render_textures = RenderTextures::new(&ctx, width, height);

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // Clear
        {
            let RenderTextures {
                render_view,
                // multisample_view,
                // depth_stencil_view,
                ..
            } = &render_textures;
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("VMobject Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // view: multisample_view,
                    // resolve_target: Some(render_view),
                    view: render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                //     view: depth_stencil_view,
                //     depth_ops: Some(wgpu::Operations {
                //         load: wgpu::LoadOp::Clear(1.0),
                //         store: wgpu::StoreOp::Store,
                //     }),
                //     stencil_ops: Some(wgpu::Operations {
                //         load: wgpu::LoadOp::Clear(0),
                //         store: wgpu::StoreOp::Store,
                //     }),
                // }),
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }
        ctx.queue.submit(Some(encoder.finish()));

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        vitem_primitive.encode_render_command(
            &ctx,
            &mut pipelines,
            &mut encoder,
            &camera_bind_group.bind_group,
            &render_textures,
            #[cfg(feature = "profiling")]
            &mut wgpu_profiler,
        );
        ctx.queue.submit(Some(encoder.finish()));
        let points3d = vitem_primitive.points3d_buffer.read_buffer(&ctx).unwrap();
        let points3d = bytemuck::cast_slice::<_, Vec4>(&points3d);
        println!("points3d: {:?}", points3d);
        let points2d = vitem_primitive.points2d_buffer.read_buffer(&ctx).unwrap();
        let points2d = bytemuck::cast_slice::<_, Vec4>(&points2d);
        println!("points2d: {:?}", points2d);

        let res = vitem_primitive.clip_info_buffer.read_buffer(&ctx).unwrap();
        let res: &[i32] = bytemuck::cast_slice(&res);
        println!("{:?}", res);

        let texture_data = get_texture_data(&ctx, &render_textures.render_texture);
        let img_buffer = image::ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(
            width as u32,
            height as u32,
            &texture_data,
        )
        .unwrap();
        img_buffer.save("test.png").unwrap();
    }
}
