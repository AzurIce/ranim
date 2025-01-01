pub mod bez_path;
pub mod vmobject;

#[cfg(test)]
mod test {
    use crate::{
        prelude::Alignable,
        rabject::{
            rabject2d::vmobject::{geometry::Arc, svg::Svg},
            Blueprint,
        },
    };

    #[test]
    fn test_align_svg() {
        let mut svg = Svg::from_file("assets/Ghostscript_Tiger.svg").build();
        let mut arc = Arc::new(2.0 * std::f32::consts::PI)
            .with_radius(10.0)
            .build();
        assert!(!svg.is_aligned(&arc));
        svg.align_with(&mut arc);
        assert!(svg.is_aligned(&arc));
    }
}
