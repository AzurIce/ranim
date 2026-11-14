//! Built-in animation operation parameters.
//!
//! `item.anim(params)` retains an owned input; `anim_ref` clones without
//! changing it, and `anim_mut` also writes the terminal state back. Parameters
//! implement `AnimRecipe<T>`; prepared evaluators implement `Eval` separately.
//!
//! ```
//! use ranim_core::prelude::*;
//! use ranim_core::glam::DVec3;
//! use ranim_anims::{fading::FadeIn, morph::Morph};
//! use ranim_items::vitem::{VItem, geometry::Square};
//!
//! let mut item = VItem::from(Square::new(2.0));
//! let fade = item.anim_ref(FadeIn).with_duration(2.0);
//! let target = item.clone().with(|item| { item.shift(DVec3::X); });
//! let movement = item.anim_mut(Morph { target });
//! let mut scene = RanimScene::new();
//! scene.play(ranim_core::seq![fade, movement]);
//! ```
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg",
    html_favicon_url = "https://raw.githubusercontent.com/AzurIce/ranim/refs/heads/main/assets/ranim.svg"
)]

/// Camera orbit parameters.
pub mod camera;
/// Creation and writing parameters.
pub mod creation;
/// Opacity parameters.
pub mod fading;
/// Destination-based morph parameters.
pub mod morph;
/// Rotation parameters.
pub mod rotating;

#[cfg(test)]
mod tests {
    use super::{
        camera::Orbit,
        creation::{Create, UnCreate, Unwrite, Write},
        fading::{FadeIn, FadeOut},
        morph::Morph,
        rotating::Rotate,
    };
    use ranim_core::{animation::registry::Animation, glam::DVec3, prelude::*};
    use ranim_items::{
        camera::CameraFrame,
        vitem::{VItem, geometry::Square},
    };
    use std::f64::consts::PI;

    #[derive(Clone)]
    struct OpacityValue(f64);
    impl Opacity for OpacityValue {
        fn set_opacity(&mut self, opacity: f32) -> &mut Self {
            self.0 = f64::from(opacity);
            self
        }
    }
    impl Interpolatable for OpacityValue {
        fn lerp(&self, target: &Self, alpha: f64) -> Self {
            Self(self.0 + (target.0 - self.0) * alpha)
        }
    }

    #[test]
    fn fades_retain_original_input_and_rebuild_after_edits() {
        let mut item = OpacityValue(0.5);
        let definition = item.anim_mut(FadeOut);
        assert_eq!(item.0, 0.0);
        assert_eq!(definition.input.0, 0.5);
        assert_eq!(definition.build().eval_alpha(0.5).0, 0.25);
        let mut fade = definition.input.anim(FadeIn);
        assert_eq!(fade.build().eval_alpha(0.0).0, 0.0);
        fade.input.0 = 0.75;
        assert_eq!(fade.build().eval_alpha(1.0).0, 0.75);
    }

    #[test]
    fn morph_keeps_destination_and_closure_edits_become_plain_data() {
        let mut input = DVec3::X;
        let params = Morph::by(&input, |target| *target = DVec3::Y);
        let mut definition = input.anim_mut(params);
        assert_eq!(input, DVec3::Y);
        assert_eq!(definition.input, DVec3::X);
        assert_eq!(definition.params.target, DVec3::Y);
        assert_eq!(
            definition.build().eval_alpha(0.5),
            (DVec3::X + DVec3::Y) * 0.5
        );
        definition.params.target = DVec3::Z;
        assert_eq!(definition.build().eval_alpha(1.0), DVec3::Z);
    }

    #[test]
    fn rotation_prepares_pivot_and_uses_edited_parameters() {
        let mut definition = DVec3::X.anim(Rotate::at(PI, DVec3::Z, DVec3::ZERO));
        assert!(definition.build().eval_alpha(1.0).distance(-DVec3::X) < 1e-10);
        definition.params.angle = PI / 2.0;
        assert!(definition.build().eval_alpha(1.0).distance(DVec3::Y) < 1e-10);
        let mut item = DVec3::X;
        let definition = item.anim_mut(Rotate::at(PI, DVec3::Z, DVec3::ZERO));
        assert!(item.distance(-DVec3::X) < 1e-10);
        assert_eq!(definition.input, DVec3::X);
    }

    #[test]
    fn camera_orbit_derives_coordinates_from_each_input() {
        let input = CameraFrame::from_spherical(PI / 2.0, 0.0, 3.0);
        let mut definition = input.anim(Orbit {
            target: DVec3::ZERO,
            angle: PI / 2.0,
        });
        assert!(
            definition
                .build()
                .eval_alpha(1.0)
                .pos
                .distance(DVec3::Y * 3.0)
                < 1e-9
        );
        definition.input.pos = DVec3::X * 4.0;
        assert!(
            definition
                .build()
                .eval_alpha(1.0)
                .pos
                .distance(DVec3::Y * 4.0)
                < 1e-9
        );
    }

    #[test]
    fn creation_and_writing_endpoints_and_write_back_are_preserved() {
        let original = VItem::from(Square::new(2.0));
        let empty = VItem::empty();
        let create = original.anim_ref(Create).build();
        assert_eq!(create.eval_alpha(0.0).vpoints.0, empty.vpoints.0);
        assert_eq!(create.eval_alpha(1.0).vpoints.0, original.vpoints.0);
        let uncreate = original.anim_ref(UnCreate).build();
        assert_eq!(uncreate.eval_alpha(0.0).vpoints.0, original.vpoints.0);
        assert_eq!(uncreate.eval_alpha(1.0).vpoints.0, empty.vpoints.0);
        let write = original.anim_ref(Write).build();
        assert_eq!(write.eval_alpha(1.0).vpoints.0, original.vpoints.0);
        let mut item = original.clone();
        let unwrite = item.anim_mut(Unwrite);
        assert_eq!(item.vpoints.0, empty.vpoints.0);
        assert_eq!(unwrite.input.vpoints.0, original.vpoints.0);
        assert_eq!(
            unwrite.build().eval_alpha(0.0).vpoints.0,
            original.vpoints.0
        );
        assert_eq!(unwrite.build().eval_alpha(1.0).vpoints.0, empty.vpoints.0);
    }

    #[test]
    fn runtime_introspection_resolves_the_parameter_type() {
        let mut scene = RanimScene::new();
        scene.play(OpacityValue(1.0).anim(FadeIn));
        let info = scene.seal().get_animation_infos().remove(0);
        assert_eq!(info.anim_name, std::any::type_name::<FadeIn>());
        assert_eq!(FadeIn::DESCRIPTOR.package, "ranim-anims");
    }
}
