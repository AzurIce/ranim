// use std::time::Duration;

// use itertools::Itertools;

// use super::{AnimationParams, Animator};

// pub trait Composition {
//     fn chain(self, other: impl Animator + 'static) -> Chain;
// }

// impl<T: Animator + 'static> Composition for T {
//     fn chain(self, other: impl Animator + 'static) -> Chain {
//         Chain::new(vec![Box::new(self), Box::new(other)])
//     }
// }

// pub struct Chain {
//     animations: Vec<Box<dyn Animator>>,
//     alphas: Vec<f32>,

//     animating_idx: usize,

//     param: AnimationParams,
// }

// impl Chain {
//     pub fn new(animations: Vec<Box<dyn Animator>>) -> Self {
//         assert!(animations.len() > 0);
//         let duration = animations.iter().map(|a| a.duration()).sum::<Duration>();

//         let mut sum_duration = Duration::from_secs_f32(0.0);
//         let mut alphas = vec![0.0];
//         alphas.extend(animations.iter().map(|a| {
//             sum_duration += a.duration();
//             sum_duration.as_secs_f32() / duration.as_secs_f32()
//         }));

//         Self {
//             animations,
//             alphas,
//             animating_idx: 0,
//             param: AnimationParams {
//                 duration,
//                 ..Default::default()
//             },
//         }
//     }
//     pub fn append(&mut self, other: impl Animator + 'static) {
//         let duration = self.param.duration + other.duration();
//         self.animations.push(Box::new(other));

//         let mut sum_duration = Duration::from_secs_f32(0.0);
//         // TODO: optimize it
//         let mut alphas = vec![0.0];
//         alphas.extend(self.animations.iter().map(|a| {
//             sum_duration += a.duration();
//             sum_duration.as_secs_f32() / duration.as_secs_f32()
//         }));

//         self.param.duration = duration;
//         self.alphas = alphas;
//     }
//     pub fn chain(mut self, other: impl Animator + 'static) -> Self {
//         self.append(other);
//         self
//     }
// }

// impl Animator for Chain {
//     fn update_clip_info(
//         &self,
//         ctx: &crate::context::WgpuContext,
//         camera: &crate::render::CameraFrame,
//     ) {
//         self.animations[self.animating_idx].update_clip_info(ctx, camera);
//     }
//     fn update_alpha(&mut self, alpha: f32) {
//         let alpha = (self.param.rate_func)(alpha);

//         for (i, (&start, &end)) in self.alphas.iter().tuple_windows().enumerate() {
//             if start <= alpha && alpha < end {
//                 self.animating_idx = i;
//                 self.animations[i].update_alpha((alpha - start) / (end - start));
//                 break;
//             }
//         }
//     }
//     fn render(
//         &self,
//         ctx: &crate::context::WgpuContext,
//         pipelines: &mut crate::utils::RenderResourceStorage,
//         encoder: &mut wgpu::CommandEncoder,
//         uniforms_bind_group: &wgpu::BindGroup,
//         multisample_view: &wgpu::TextureView,
//         target_view: &wgpu::TextureView,
//     ) {
//         self.animations[self.animating_idx].render(
//             ctx,
//             pipelines,
//             encoder,
//             uniforms_bind_group,
//             multisample_view,
//             target_view,
//         );
//     }
// }
