use crate::{
    pipelines::{clipbox_2d::Clipbox2dComputeBindGroup, vitem2d::RenderBindGroup},
    utils::{WgpuBuffer, WgpuContext, WgpuVecBuffer},
};
use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use ranim_core::{
    components::{rgba::Rgba, width::Width},
    core_item::vitem_2d::VItem2d,
};

use super::{Primitive, RenderResource};

impl Primitive for VItem2d {
    type RenderPacket = VItem2dRenderInstance;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PlaneUniform {
    origin: Vec4,  // xyz, w=pad
    basis_u: Vec4, // xyz, w=pad
    basis_v: Vec4, // xyz, w=pad
}

/// [`VItem2d`]'s render instance.
pub struct VItem2dRenderInstance {
    // since the storage buffer is aligned to 16 bytes, we use vec4 for alignment
    /// COMPUTE INPUT, RENDER INPUT: (x, y, is_closed, _padding)
    pub(crate) points2d_buffer: WgpuVecBuffer<Vec4>,
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
    /// RENDER UNIFORM
    pub(crate) plane_buffer: WgpuBuffer<PlaneUniform>,

    /// COMPUTE BIND GROUP
    pub(crate) compute_bind_group: Option<Clipbox2dComputeBindGroup>,

    /// RENDER BIND GROUP
    pub(crate) render_bind_group: Option<RenderBindGroup>,
}

impl RenderResource for VItem2dRenderInstance {
    type Data = VItem2d;

    fn init(ctx: &WgpuContext, data: &Self::Data) -> Self {
        let points_data: Vec<Vec4> = data
            .points2d
            .iter()
            .map(|p| Vec4::new(p.x, p.y, p.z, 0.0))
            .collect();

        let points2d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 2d Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            &points_data,
        );

        let clip_info_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Clip Info Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
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

        let plane_data = PlaneUniform {
            origin: Vec4::from((data.origin, 0.0)),
            basis_u: Vec4::from((data.basis.0, 0.0)),
            basis_v: Vec4::from((data.basis.1, 0.0)),
        };
        let plane_buffer = WgpuBuffer::new_init(
            ctx,
            Some("Plane Uniform Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            plane_data,
        );

        let compute_bind_group = Clipbox2dComputeBindGroup::new(
            ctx,
            &points2d_buffer.buffer,
            &stroke_widths.buffer,
            &clip_info_buffer.buffer,
            point_cnt_buffer.as_ref(),
        );

        let render_bind_group = RenderBindGroup::new(
            ctx,
            &points2d_buffer.buffer,
            &fill_rgbas.buffer,
            &stroke_rgbas.buffer,
            &stroke_widths.buffer,
            &clip_info_buffer.buffer,
            plane_buffer.as_ref(),
        );

        Self {
            points2d_buffer,
            clip_info_buffer,
            point_cnt_buffer: Some(point_cnt_buffer),
            fill_rgbas,
            stroke_rgbas,
            stroke_widths,
            plane_buffer,
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

        let points_data: Vec<Vec4> = data
            .points2d
            .iter()
            .map(|p| Vec4::new(p.x, p.y, p.z, 0.0))
            .collect();

        let plane_data = PlaneUniform {
            origin: Vec4::from((data.origin, 0.0)),
            basis_u: Vec4::from((data.basis.0, 0.0)),
            basis_v: Vec4::from((data.basis.1, 0.0)),
        };
        self.plane_buffer.set(ctx, plane_data);

        // Dynamic sized
        if [
            self.points2d_buffer.set(ctx, &points_data),
            self.fill_rgbas.set(ctx, &data.fill_rgbas),
            self.stroke_rgbas.set(ctx, &data.stroke_rgbas),
            self.stroke_widths.set(ctx, &data.stroke_widths),
            self.clip_info_buffer
                .set(ctx, &[i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0]),
            self.compute_bind_group.is_none(),
        ]
        .iter()
        .any(|x| *x)
        {
            self.compute_bind_group = Some(Clipbox2dComputeBindGroup::new(
                ctx,
                &self.points2d_buffer.buffer,
                &self.stroke_widths.buffer,
                &self.clip_info_buffer.buffer,
                self.point_cnt_buffer.as_ref().unwrap().as_ref(),
            ));
            self.render_bind_group = Some(RenderBindGroup::new(
                ctx,
                &self.points2d_buffer.buffer,
                &self.fill_rgbas.buffer,
                &self.stroke_rgbas.buffer,
                &self.stroke_widths.buffer,
                &self.clip_info_buffer.buffer,
                self.plane_buffer.as_ref(),
            ));
        }
    }
}

impl VItem2dRenderInstance {
    pub fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        cpass.set_bind_group(0, self.compute_bind_group.as_ref().unwrap().as_ref(), &[]);
        cpass.dispatch_workgroups(self.points2d_buffer.len().div_ceil(256) as u32, 1, 1);
    }
    pub fn encode_depth_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        rpass.set_bind_group(1, self.render_bind_group.as_ref().unwrap().as_ref(), &[]);
        rpass.draw(0..4, 0..1);
    }
    pub fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        rpass.set_bind_group(1, self.render_bind_group.as_ref().unwrap().as_ref(), &[]);
        rpass.draw(0..4, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Renderer, ViewportUniform, resource::RenderPool, utils::WgpuContext};
    use glam::{Vec3, vec3, vec4};
    use ranim_core::{core_item::CoreItem, store::CoreItemStore, traits::With};

    #[test]
    fn foo_clear_screen() {
        let ctx = pollster::block_on(WgpuContext::new());
        let mut renderer = Renderer::new(&ctx, 1280, 720);
        let clear_color = wgpu::Color {
            r: 0.8,
            g: 0.8,
            b: 0.8,
            a: 1.0,
        };
        renderer.clear_screen(&ctx, clear_color);

        let img = renderer.get_rendered_texture_img_buffer(&ctx);
        img.save("../../output/clear_screen.png").unwrap();
        let depth_img = renderer.get_depth_texture_img_buffer(&ctx);
        depth_img
            .save("../../output/clear_screen_depth.png")
            .unwrap();
    }

    #[test]
    fn foo_render_vitem2d_primitive() {
        let ctx = pollster::block_on(WgpuContext::new());
        let mut renderer = Renderer::new(&ctx, 1280, 720);
        let clear_color = wgpu::Color {
            r: 0.8,
            g: 0.8,
            b: 0.8,
            a: 1.0,
        };

        // Setup Camera for Perspective View
        use glam::DVec3;
        use ranim_core::core_item::camera_frame::CameraFrame;

        let mut camera = CameraFrame::new();
        camera.pos = DVec3::new(3.0, 3.0, 3.0);
        camera.facing = DVec3::new(-1.0, -1.0, -1.0).normalize();
        camera.up = DVec3::Y;
        camera.perspective_blend = 1.0; // Use perspective

        // A simple "leaf" shape points (local 2D)
        // Set z=1.0 to enable fill (is_closed=true)
        let scale = 2.0;
        let mut points = vec![
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 0.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(0.0, -1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ];
        let n = (points.len() + 1) / 2;
        points.iter_mut().for_each(|p| {
            p.x *= scale;
            p.y *= scale;
        });

        let make_items = |origin: Vec3, alpha: f32| {
            // Red on XY plane
            let item1 = VItem2d {
                origin,
                basis: (Vec3::X, Vec3::Y),
                points2d: points.clone(),
                fill_rgbas: vec![Rgba(vec4(1.0, 0.0, 0.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.5, 0.0, 0.0, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };

            // Green on YZ
            let item2 = VItem2d {
                origin,
                basis: (Vec3::Z, Vec3::Y), // Z is "X", Y is "Y"
                points2d: points.clone(),
                fill_rgbas: vec![Rgba(vec4(0.0, 1.0, 0.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.0, 0.5, 0.0, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };

            // Blue on XZ
            let item3 = VItem2d {
                origin,
                basis: (Vec3::X, Vec3::Z), // X is "X", Z is "Y"
                points2d: points.clone(),
                fill_rgbas: vec![Rgba(vec4(0.0, 0.0, 1.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.0, 0.0, 0.5, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };
            std::iter::once(item1)
                .chain(std::iter::once(item2))
                .chain(std::iter::once(item3))
        };

        renderer.viewport.update(
            &ctx,
            &ViewportUniform::from_camera_frame(&camera, 1280, 720),
        );
        let mut pool = RenderPool::new();
        let mut store = CoreItemStore::new();
        let center = Vec3::ZERO;
        let dir = (Vec3::X + Vec3::NEG_Z).normalize();
        store.update(
            make_items(-scale * 1.5 * dir + center, 1.0)
                .chain(make_items(scale * 1.5 * dir + center, 0.5))
                .map(CoreItem::VItem2D)
                .chain(std::iter::once(CoreItem::CameraFrame(camera)))
                .enumerate()
                .map(|(id, x)| ((id, id), x)),
        );

        renderer.render_store_with_pool(&ctx, clear_color, &store, &mut pool);
        let img = renderer.get_rendered_texture_img_buffer(&ctx);
        img.save("../../output/vitem2d_intersecting_perspective.png")
            .unwrap();
        let depth_img = renderer.get_depth_texture_img_buffer(&ctx);
        depth_img
            .save("../../output/vitem2d_intersecting_perspective_depth.png")
            .unwrap();
    }
}
