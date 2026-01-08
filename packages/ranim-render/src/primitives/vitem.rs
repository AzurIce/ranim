use crate::{
    pipelines::{map_3d_to_2d::ComputeBindGroup, vitem::RenderBindGroup},
    utils::{WgpuBuffer, WgpuContext, WgpuVecBuffer},
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
        let points3d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 3d Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            &data.points2d,
        );
        let points2d_buffer = WgpuVecBuffer::new_init(
            ctx,
            Some("Points 2d Buffer"),
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            &data.points2d,
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

impl VItemRenderInstance {
    pub fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {
        cpass.set_bind_group(1, self.compute_bind_group.as_ref().unwrap().as_ref(), &[]);
        cpass.dispatch_workgroups(self.points3d_buffer.len().div_ceil(256) as u32, 1, 1);
    }
    pub fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {
        rpass.set_bind_group(1, self.render_bind_group.as_ref().unwrap().as_ref(), &[]);
        rpass.draw(0..4, 0..1);
    }
}

impl RenderCommand for VItemRenderInstance {
    fn encode_compute_pass_command(&self, cpass: &mut wgpu::ComputePass) {}
    fn encode_depth_render_pass_command(&self, _rpass: &mut wgpu::RenderPass) {}
    fn encode_render_pass_command(&self, rpass: &mut wgpu::RenderPass) {}
    fn debug(&self, _ctx: &WgpuContext) {
        // let points2d = self.points2d_buffer.read_buffer(ctx).unwrap();
        // let points2d = bytemuck::cast_slice::<_, Vec4>(&points2d);
        // println!("points2d: {:?}", points2d);
    }
}

#[cfg(test)]
mod tests {
    use glam::vec4;
    use ranim_core::{core_item::CoreItem, prelude::CameraFrame, store::CoreItemStore};

    use super::*;
    use crate::{Renderer, primitives::RenderPool, utils::WgpuContext};

    #[test]
    fn test_render_vitem_primitive() {
        let ctx = pollster::block_on(WgpuContext::new());
        let mut renderer = Renderer::new(&ctx, 8.0, 1920, 720);
        let clear_color = wgpu::Color::BLACK;

        let vitem_primitive_data = VItemPrimitive {
            points2d: vec![
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                Vec4::new(0.5, 1.0, 0.0, 1.0),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
                Vec4::new(0.5, 0.0, 0.0, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
            ],
            fill_rgbas: vec![Rgba(vec4(1.0, 0.0, 0.0, 1.0)); 3],
            stroke_rgbas: vec![Rgba(vec4(0.0, 1.0, 0.0, 1.0)); 3],
            stroke_widths: vec![Width(0.02); 3],
        };
        let mut pool = RenderPool::new();
        let mut store = CoreItemStore::new();
        store.update(
            std::iter::once(CoreItem::VItemPrimitive(vitem_primitive_data)).chain(std::iter::once(
                CoreItem::CameraFrame(CameraFrame::default()),
            )),
        );
        renderer.render_store_with_pool(&ctx, clear_color, &store, &mut pool);
        let img = renderer.get_rendered_texture_img_buffer(&ctx);
        img.save("../../output/vitem_primitive.png").unwrap();
    }
}
