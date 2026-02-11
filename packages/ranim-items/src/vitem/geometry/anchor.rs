use ranim_core::{
    core_item::vitem::Basis2d,
    glam::{DVec2, DVec3},
    traits::Locate,
};

use crate::vitem::geometry::{Arc, ArcBetweenPoints, Circle, Ellipse, EllipticArc};

/// `Origin` anchor for shapes with an origin point.
#[derive(Debug, Clone, Copy)]
pub struct Origin;

/// Focus of an ellipse.
#[derive(Debug, Clone, Copy)]
pub struct Focus {
    pos: bool,
}

impl Focus {
    /// Focus on the positive semi-major axis.
    pub const POS: Self = Focus { pos: true };
    /// Focus on the negative semi-major axis.
    pub const NEG: Self = Focus { pos: false };
}

impl Locate<Arc> for Origin {
    fn locate(&self, target: &Arc) -> DVec3 {
        target.center
    }
}

impl Locate<Arc> for Focus {
    fn locate(&self, target: &Arc) -> DVec3 {
        target.center
    }
}

impl Locate<ArcBetweenPoints> for Origin {
    fn locate(&self, target: &ArcBetweenPoints) -> DVec3 {
        // TODO: make this better
        Arc::from(target.clone()).center
    }
}

impl Locate<ArcBetweenPoints> for Focus {
    fn locate(&self, target: &ArcBetweenPoints) -> DVec3 {
        // TODO: make this better
        Arc::from(target.clone()).center
    }
}

impl Locate<Circle> for Origin {
    fn locate(&self, target: &Circle) -> DVec3 {
        target.center
    }
}

impl Locate<Circle> for Focus {
    fn locate(&self, target: &Circle) -> DVec3 {
        target.center
    }
}

fn ellipse_focus(basis: Basis2d, radius: DVec2) -> DVec3 {
    let DVec2 { x: rx, y: ry } = radius;
    let c = (rx * rx - ry * ry).abs().sqrt();
    (if rx > ry { basis.u() } else { basis.v() }) * c
}

impl Locate<EllipticArc> for Origin {
    fn locate(&self, target: &EllipticArc) -> DVec3 {
        target.center
    }
}

impl Locate<EllipticArc> for Focus {
    fn locate(&self, target: &EllipticArc) -> DVec3 {
        let &EllipticArc {
            basis,
            center,
            radius,
            ..
        } = target;
        let focus = ellipse_focus(basis, radius);
        if self.pos {
            center + focus
        } else {
            center - focus
        }
    }
}

impl Locate<Ellipse> for Origin {
    fn locate(&self, target: &Ellipse) -> DVec3 {
        target.center
    }
}

impl Locate<Ellipse> for Focus {
    fn locate(&self, target: &Ellipse) -> DVec3 {
        let &Ellipse {
            basis,
            center,
            radius,
            ..
        } = target;
        let focus = ellipse_focus(basis, radius);
        if self.pos {
            center + focus
        } else {
            center - focus
        }
    }
}
