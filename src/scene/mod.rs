use std::{fs, path::Path};

use image::{ImageBuffer, Rgba};

use crate::{
    animation::Animation, camera::Camera, mobject::Mobject, pipeline::simple, RanimContext,
    WgpuContext,
};

pub struct Scene {
    pub camera: Camera,
    pub mobjects: Vec<Mobject<simple::Vertex>>,
    pub time: f32,
    pub frame_count: usize,
}

impl Scene {
    pub fn new(ctx: &WgpuContext) -> Self {
        Self {
            camera: Camera::new(ctx, 1920, 1080),
            mobjects: Vec::new(),
            time: 0.0,
            frame_count: 0,
        }
    }

    pub fn add_mobject(&mut self, mobject: &Mobject<simple::Vertex>) {
        self.mobjects.retain(|m| m.id != mobject.id);
        self.mobjects.push(mobject.clone());
    }

    pub fn add_mobjects(&mut self, mobjects: Vec<Mobject<simple::Vertex>>) {
        self.mobjects
            .retain(|m| !mobjects.iter().any(|m2| m2.id == m.id));
        self.mobjects.extend(mobjects);
    }

    pub fn render_to_image(&mut self, ctx: &mut RanimContext, path: impl AsRef<Path>) {
        self.camera.render(ctx, &mut self.mobjects);
        self.save_frame_to_image(ctx, path);
    }

    pub fn update_frame(&mut self, ctx: &mut RanimContext, dt: f32) {
        self.time += dt;
        // self.update_mobjects(dt);
        self.camera.render(ctx, &mut self.mobjects);
    }

    pub fn save_frame_to_image(&mut self, ctx: &mut RanimContext, path: impl AsRef<Path>) {
        let size = self.camera.frame.size;
        let texture_data = self.camera.get_rendered_texture(&ctx.wgpu_ctx);
        let buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(size.0 as u32, size.1 as u32, texture_data)
                .unwrap();
        buffer.save(path).unwrap();
    }

    /// Play an animation
    ///
    /// See [`Animation`].
    pub fn play(
        &mut self,
        ctx: &mut RanimContext,
        mut animation: Animation,
    ) -> Option<Mobject<simple::Vertex>> {
        // TODO: handle the precision problem
        let frames = animation.config.calc_frames(self.camera.fps as f32);

        let dt = animation.config.run_time.as_secs_f32() / (frames - 1) as f32;
        for t in (0..frames).map(|x| x as f32 * dt) {
            // TODO: implement mobject's updaters
            // animation.update_mobjects(dt);
            let alpha = t / animation.config.run_time.as_secs_f32();
            let alpha = (animation.config.rate_func)(alpha);
            animation.interpolate(alpha);
            self.add_mobject(&animation.mobject);
            self.update_frame(ctx, dt);
            self.frame_count += 1;
            let path = format!("output/image-{:04}.png", self.frame_count);
            let dir = Path::new(&path).parent().unwrap();
            if !dir.exists() {
                fs::create_dir_all(dir).unwrap();
            }
            self.save_frame_to_image(ctx, path);
        }
        if animation.should_remove() {
            self.mobjects.retain(|m| m.id != animation.mobject.id);
            None
        } else {
            Some(animation.mobject)
        }
    }
}
