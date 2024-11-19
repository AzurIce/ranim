use std::path::Path;

use image::{ImageBuffer, Rgba};

use crate::{camera::Camera, mobject::Mobject, pipeline::simple, RanimContext, WgpuContext};

pub struct Scene {
    pub camera: Camera,
    pub mobjects: Vec<Mobject<simple::Vertex>>,
}

impl Scene {
    pub fn new(ctx: &WgpuContext) -> Self {
        Self {
            camera: Camera::new(ctx, 1920, 1080),
            mobjects: Vec::new(),
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
        for mobject in &mut self.mobjects {
            self.camera.render(ctx, mobject);
        }
        let size = self.camera.frame.size;
        let texture_data = self.camera.get_rendered_texture(&ctx.wgpu_ctx);
        let buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(size.0 as u32, size.1 as u32, texture_data)
                .unwrap();
        buffer.save(path).unwrap();
    }
}
