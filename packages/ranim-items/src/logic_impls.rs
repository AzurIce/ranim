//! Explicit `LogicItem` impls for the built-in item types.
//!
//! `LogicItem` deliberately has **no blanket impl** — explicit impls are what
//! let the compiler prove containers like `Vec<T>` are not `LogicItem`, so
//! group outputs can take a separate materialization path (see
//! `ranim_core::logic::MaterializeOut`). New built-in item types must add
//! their impl here; dylib item types never do (they register their own
//! materializers, E4 pattern).

use ranim_core::logic::LogicItem;

use crate::debug::VisualizeAabbItem;
use crate::mesh::{MeshItem, Sphere, Surface};
use crate::vitem::geometry::{
    Arc, ArcBetweenPoints, Circle, Ellipse, EllipticArc, Line, Parallelogram, Polygon, Rectangle,
    RegularPolygon, Square,
};
use crate::vitem::svg::SvgItem;
use crate::vitem::VItem;
#[cfg(feature = "typst")]
use crate::vitem::{text::TextItem, typst::TypstText};

impl LogicItem for MeshItem {}
impl LogicItem for Sphere {}
impl LogicItem for Surface {}
impl LogicItem for VItem {}
impl LogicItem for Arc {}
impl LogicItem for ArcBetweenPoints {}
impl LogicItem for Circle {}
impl LogicItem for Ellipse {}
impl LogicItem for EllipticArc {}
impl LogicItem for Line {}
impl LogicItem for Parallelogram {}
impl LogicItem for Polygon {}
impl LogicItem for Rectangle {}
impl LogicItem for RegularPolygon {}
impl LogicItem for Square {}
impl LogicItem for SvgItem {}

impl<T: LogicItem + ranim_core::anchor::Aabb + Clone> LogicItem for VisualizeAabbItem<T> {}

#[cfg(feature = "typst")]
impl LogicItem for TextItem {}
#[cfg(feature = "typst")]
impl LogicItem for TypstText {}
