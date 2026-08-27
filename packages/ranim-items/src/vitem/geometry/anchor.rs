use ranim_core::{
    glam::{DAffine3, DVec2, DVec3},
    prelude::Transformed,
    traits::Locate,
};

use crate::vitem::geometry::{Arc, ArcBetweenPoints, Circle, Ellipse, EllipticArc};

/// `Origin` anchor for shapes with an origin point.
#[derive(Debug, Clone, Copy)]
pub struct Origin;

/// Focus of an ellipse.
#[derive(Debug, Clone, Copy)]
pub enum Focus {
    /// Focus on the positive semi-major axis.
    Pos,
    /// Focus on the negative semi-major axis.
    Neg,
}

impl<T, G> Locate<Transformed<T, G>> for Origin
where
    Origin: Locate<T>,
    G: Clone + Into<DAffine3>,
{
    fn locate(&self, target: &Transformed<T, G>) -> DVec3 {
        target
            .transform
            .clone()
            .into()
            .transform_point3(self.locate(&target.inner))
    }
}

impl<T, G> Locate<Transformed<T, G>> for Focus
where
    Focus: Locate<T>,
    G: Clone + Into<DAffine3>,
{
    fn locate(&self, target: &Transformed<T, G>) -> DVec3 {
        target
            .transform
            .clone()
            .into()
            .transform_point3(self.locate(&target.inner))
    }
}

impl Locate<Arc> for Origin {
    fn locate(&self, _target: &Arc) -> DVec3 {
        DVec3::ZERO
    }
}

impl Locate<Arc> for Focus {
    fn locate(&self, _target: &Arc) -> DVec3 {
        DVec3::ZERO
    }
}

impl Locate<ArcBetweenPoints> for Origin {
    fn locate(&self, target: &ArcBetweenPoints) -> DVec3 {
        target.center()
    }
}

impl Locate<ArcBetweenPoints> for Focus {
    fn locate(&self, target: &ArcBetweenPoints) -> DVec3 {
        target.center()
    }
}

impl Locate<Circle> for Origin {
    fn locate(&self, _target: &Circle) -> DVec3 {
        DVec3::ZERO
    }
}

impl Locate<Circle> for Focus {
    fn locate(&self, _target: &Circle) -> DVec3 {
        DVec3::ZERO
    }
}

fn ellipse_focus(radius: DVec2) -> DVec3 {
    let DVec2 { x: rx, y: ry } = radius;
    let c = (rx * rx - ry * ry).abs().sqrt();
    if rx > ry { DVec3::X * c } else { DVec3::Y * c }
}

impl Locate<EllipticArc> for Origin {
    fn locate(&self, _target: &EllipticArc) -> DVec3 {
        DVec3::ZERO
    }
}

impl Locate<EllipticArc> for Focus {
    fn locate(&self, target: &EllipticArc) -> DVec3 {
        let focus = ellipse_focus(target.radius);
        if matches!(self, Focus::Pos) {
            focus
        } else {
            -focus
        }
    }
}

impl Locate<Ellipse> for Origin {
    fn locate(&self, _target: &Ellipse) -> DVec3 {
        DVec3::ZERO
    }
}

impl Locate<Ellipse> for Focus {
    fn locate(&self, target: &Ellipse) -> DVec3 {
        let focus = ellipse_focus(target.radius);
        if matches!(self, Focus::Pos) {
            focus
        } else {
            -focus
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::{
        glam::{DAffine3, dvec2, dvec3},
        prelude::TransformedExt,
    };

    #[test]
    fn transformed_focus_is_located_in_local_space_then_moved() {
        let ellipse = Ellipse::new(dvec2(5.0, 3.0));
        let placed = ellipse.transformed(DAffine3::from_translation(dvec3(2.0, 0.0, 0.0)));
        assert_eq!(Focus::Pos.locate(&placed), dvec3(6.0, 0.0, 0.0));
    }
}
