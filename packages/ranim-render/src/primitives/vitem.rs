#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Renderer, resource::RenderPool, utils::WgpuContext};
    use glam::{DVec3, Vec3, Vec4, vec4};
    use ranim_core::{
        core_item::{CoreItem, camera_frame::CameraFrame, vitem::Basis2d},
        store::CoreItemStore,
    };

    #[test]
    fn foo_render_vitem2d_primitive() {
        let ctx = pollster::block_on(WgpuContext::new());
        let mut renderer = Renderer::new(&ctx, 1280, 720, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let clear_color = wgpu::Color {
            r: 0.8,
            g: 0.8,
            b: 0.8,
            a: 1.0,
        };

        let mut camera = CameraFrame::new();
        camera.pos = DVec3::new(3.0, 3.0, 3.0);
        camera.facing = DVec3::new(-1.0, -1.0, -1.0).normalize();
        camera.up = DVec3::Y;
        camera.perspective_blend = 1.0; // Use perspective

        // Set z=1.0 to enable fill (is_closed=true)
        let scale = 2.0;
        let mut points = vec![
            Vec4::new(-1.0, -1.0, 0.0, 1.0),
            Vec4::new(-1.0, 0.0, 0.0, 1.0),
            Vec4::new(-1.0, 1.0, 0.0, 1.0),
            Vec4::new(0.0, 1.0, 0.0, 1.0),
            Vec4::new(1.0, 1.0, 0.0, 1.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            Vec4::new(1.0, -1.0, 0.0, 1.0),
            Vec4::new(0.0, -1.0, 0.0, 1.0),
            Vec4::new(-1.0, -1.0, 0.0, 1.0),
        ];
        let n = points.len().div_ceil(2);
        points.iter_mut().for_each(|p| {
            p.x *= scale;
            p.y *= scale;
        });

        let make_items = |origin: Vec3, alpha: f32| {
            // Red on XY plane
            let item1 = VItem {
                origin,
                basis: Basis2d::XY,
                points: points.clone(),
                fill_rgbas: vec![Rgba(vec4(1.0, 0.0, 0.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.5, 0.0, 0.0, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };

            // Green on YZ
            let item2 = VItem {
                origin,
                basis: Basis2d::YZ,
                points: points.clone(),
                fill_rgbas: vec![Rgba(vec4(0.0, 1.0, 0.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.0, 0.5, 0.0, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };

            // Blue on XZ
            let item3 = VItem {
                origin,
                basis: Basis2d::XZ,
                points: points.clone(),
                fill_rgbas: vec![Rgba(vec4(0.0, 0.0, 1.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.0, 0.0, 0.5, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };
            std::iter::once(item1)
                .chain(std::iter::once(item2))
                .chain(std::iter::once(item3))
        };

        let mut pool = RenderPool::new();
        let mut store = CoreItemStore::new();
        let center = Vec3::ZERO;
        let dir = (Vec3::X + Vec3::NEG_Z).normalize();
        store.update(
            make_items(-scale * 1.5 * dir + center, 1.0)
                .chain(make_items(scale * 1.5 * dir + center, 0.5))
                .map(CoreItem::VItem)
                .chain(std::iter::once(CoreItem::CameraFrame(camera)))
                .enumerate()
                .map(|(id, x)| ((id, id), x)),
        );

        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &store, &mut pool);
        let img = render_textures.get_rendered_texture_img_buffer(&ctx);
        img.save("../../output/vitem2d_intersecting_perspective.png")
            .unwrap();
        let depth_img = render_textures.get_depth_texture_img_buffer(&ctx);
        depth_img
            .save("../../output/vitem2d_intersecting_perspective_depth.png")
            .unwrap();
    }

    /// Render the same scene with the merged buffer path for visual comparison.
    #[test]
    fn render_merged_vitem2d_primitive() {
        let ctx = pollster::block_on(WgpuContext::new());
        let mut renderer = Renderer::new(&ctx, 1280, 720, 8);
        let mut render_textures = renderer.new_render_textures(&ctx);
        let clear_color = wgpu::Color {
            r: 0.8,
            g: 0.8,
            b: 0.8,
            a: 1.0,
        };

        let mut camera = CameraFrame::new();
        camera.pos = DVec3::new(3.0, 3.0, 3.0);
        camera.facing = DVec3::new(-1.0, -1.0, -1.0).normalize();
        camera.up = DVec3::Y;
        camera.perspective_blend = 1.0;

        let scale = 2.0;
        let mut points = vec![
            Vec4::new(-1.0, -1.0, 0.0, 1.0),
            Vec4::new(-1.0, 0.0, 0.0, 1.0),
            Vec4::new(-1.0, 1.0, 0.0, 1.0),
            Vec4::new(0.0, 1.0, 0.0, 1.0),
            Vec4::new(1.0, 1.0, 0.0, 1.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            Vec4::new(1.0, -1.0, 0.0, 1.0),
            Vec4::new(0.0, -1.0, 0.0, 1.0),
            Vec4::new(-1.0, -1.0, 0.0, 1.0),
        ];
        let n = points.len().div_ceil(2);
        points.iter_mut().for_each(|p| {
            p.x *= scale;
            p.y *= scale;
        });

        let make_items = |origin: Vec3, alpha: f32| {
            let item1 = VItem {
                origin,
                basis: Basis2d::XY,
                points: points.clone(),
                fill_rgbas: vec![Rgba(vec4(1.0, 0.0, 0.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.5, 0.0, 0.0, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };
            let item2 = VItem {
                origin,
                basis: Basis2d::YZ,
                points: points.clone(),
                fill_rgbas: vec![Rgba(vec4(0.0, 1.0, 0.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.0, 0.5, 0.0, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };
            let item3 = VItem {
                origin,
                basis: Basis2d::XZ,
                points: points.clone(),
                fill_rgbas: vec![Rgba(vec4(0.0, 0.0, 1.0, alpha)); n],
                stroke_rgbas: vec![Rgba(vec4(0.0, 0.0, 0.5, 1.0)); n],
                stroke_widths: vec![Width(0.02); n],
            };
            std::iter::once(item1)
                .chain(std::iter::once(item2))
                .chain(std::iter::once(item3))
        };

        let mut pool = RenderPool::new();
        let mut store = CoreItemStore::new();
        let center = Vec3::ZERO;
        let dir = (Vec3::X + Vec3::NEG_Z).normalize();
        store.update(
            make_items(-scale * 1.5 * dir + center, 1.0)
                .chain(make_items(scale * 1.5 * dir + center, 0.5))
                .map(CoreItem::VItem)
                .chain(std::iter::once(CoreItem::CameraFrame(camera)))
                .enumerate()
                .map(|(id, x)| ((id, id), x)),
        );

        renderer.render_store_with_pool(&ctx, &mut render_textures, clear_color, &store, &mut pool);
        ctx.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        let img = render_textures.get_rendered_texture_img_buffer(&ctx);
        img.save("../../output/merged_vitem2d_intersecting_perspective.png")
            .unwrap();
    }
}
