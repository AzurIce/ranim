//! Built-in animation operation parameters.
//!
//! `item.anim(params)` retains an owned input; `anim_ref` clones without
//! changing it, and `anim_mut` also writes the terminal state back. Parameters
//! implement `AnimRecipe<T>`; prepared evaluators implement `Eval` separately.
//! Write-back operations additionally implement `AnimWriteBack<T>`.
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

    fn sample<T, R>(definition: &AnimDef<T, R>, alpha: f64) -> T
    where
        T: Clone + Send + Sync + 'static,
        R: AnimRecipe<T> + Clone + 'static,
    {
        let mut scene = RanimScene::new();
        scene.play(definition.clone());
        let mut frame = ranim_core::frame::EvaluatedFrame::new();
        scene.seal().eval_at_alpha(alpha, &mut frame);
        frame.items::<T>()[0].value.clone()
    }

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
        assert_eq!(definition.initial.0, 0.5);
        assert_eq!(sample(&definition, 0.5).0, 0.25);
        let mut fade = definition.initial.anim(FadeIn);
        assert_eq!(sample(&fade, 0.0).0, 0.0);
        fade.initial.0 = 0.75;
        assert_eq!(sample(&fade, 1.0).0, 0.75);
    }

    #[test]
    fn morph_keeps_destination_and_closure_edits_become_plain_data() {
        let mut input = DVec3::X;
        let params = Morph::by(&input, |target| *target = DVec3::Y);
        let mut definition = input.anim_mut(params);
        assert_eq!(input, DVec3::Y);
        assert_eq!(definition.initial, DVec3::X);
        assert_eq!(definition.recipe.target, DVec3::Y);
        assert_eq!(sample(&definition, 0.5), (DVec3::X + DVec3::Y) * 0.5);
        definition.recipe.target = DVec3::Z;
        assert_eq!(sample(&definition, 1.0), DVec3::Z);
    }

    #[test]
    fn rotation_prepares_pivot_and_uses_edited_parameters() {
        let mut definition = DVec3::X.anim(Rotate::at(PI, DVec3::Z, DVec3::ZERO));
        assert!(sample(&definition, 1.0).distance(-DVec3::X) < 1e-10);
        definition.recipe.angle = PI / 2.0;
        assert!(sample(&definition, 1.0).distance(DVec3::Y) < 1e-10);
        let mut item = DVec3::X;
        let definition = item.anim_mut(Rotate::at(PI, DVec3::Z, DVec3::ZERO));
        assert!(item.distance(-DVec3::X) < 1e-10);
        assert_eq!(definition.initial, DVec3::X);
    }

    #[test]
    fn camera_orbit_derives_coordinates_from_each_input() {
        let input = CameraFrame::from_spherical(PI / 2.0, 0.0, 3.0);
        let mut definition = input.anim(Orbit {
            target: DVec3::ZERO,
            angle: PI / 2.0,
        });
        assert!(sample(&definition, 1.0).pos.distance(DVec3::Y * 3.0) < 1e-9);
        definition.initial.pos = DVec3::X * 4.0;
        assert!(sample(&definition, 1.0).pos.distance(DVec3::Y * 4.0) < 1e-9);
    }

    #[test]
    fn creation_and_writing_endpoints_and_write_back_are_preserved() {
        let original = VItem::from(Square::new(2.0));
        let empty = VItem::empty();
        let create = original.anim_ref(Create);
        assert_eq!(sample(&create, 0.0).vpoints.0, empty.vpoints.0);
        assert_eq!(sample(&create, 1.0).vpoints.0, original.vpoints.0);
        let uncreate = original.anim_ref(UnCreate);
        assert_eq!(sample(&uncreate, 0.0).vpoints.0, original.vpoints.0);
        assert_eq!(sample(&uncreate, 1.0).vpoints.0, empty.vpoints.0);
        let write = original.anim_ref(Write);
        assert_eq!(sample(&write, 1.0).vpoints.0, original.vpoints.0);
        let mut item = original.clone();
        let unwrite = item.anim_mut(Unwrite);
        assert_eq!(item.vpoints.0, empty.vpoints.0);
        assert_eq!(unwrite.initial.vpoints.0, original.vpoints.0);
        assert_eq!(sample(&unwrite, 0.0).vpoints.0, original.vpoints.0);
        assert_eq!(sample(&unwrite, 1.0).vpoints.0, empty.vpoints.0);
    }

    #[test]
    fn retained_definition_identifies_the_recipe_type() {
        let mut scene = RanimScene::new();
        scene.play(OpacityValue(1.0).anim(FadeIn));
        let scene = scene.seal();
        assert_eq!(
            scene.definition().definitions()[0].anim_name(),
            std::any::type_name::<FadeIn>()
        );
        assert_eq!(FadeIn::DESCRIPTOR.package, "ranim-anims");
    }
}
