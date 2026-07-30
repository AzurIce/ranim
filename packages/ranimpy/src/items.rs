//! Item bindings: `VItem`, geometry (`Square`, `Circle`), `SvgItem`,
//! `CameraFrame`, colors and palettes.
//!
//! Item methods mutate in place and return `self` so Python calls can chain,
//! mirroring the Rust traits' `&mut self -> &mut Self` style.

use pyo3::prelude::*;
use pyo3::types::PyModule;

use ranim::anims::creation::{CreationAnim, WritingAnim};
use ranim::anims::fading::FadingAnim;
use ranim::anims::morph::MorphAnim;
use ranim::color::palettes::manim;
use ranim::color::{AlphaColor, Srgb};
use ranim::core::anchor::{AabbPoint, Locate};
use ranim::core::core_item::camera_frame::CameraFrame;
use ranim::core::glam::DVec3;
use ranim::core::prelude::*;
use ranim::items::vitem::geometry::{Circle, Square};
use ranim::items::vitem::svg::SvgItem;
use ranim::items::vitem::VItem;

use crate::anims::PyAnim;

// MARK: Color

/// An RGBA color.
#[pyclass(name = "Color", frozen)]
#[derive(Clone, Copy)]
pub struct PyColor(pub(crate) AlphaColor<Srgb>);

#[pymethods]
impl PyColor {
    #[new]
    #[pyo3(signature = (r, g, b, a=1.0))]
    fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self(AlphaColor::new([r, g, b, a]))
    }

    /// Return a copy with a different alpha.
    fn with_alpha(&self, alpha: f32) -> Self {
        Self(self.0.with_alpha(alpha))
    }
}

/// The `ranimpy.palettes` submodule (currently only `manim`).
pub fn palettes_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let palettes = PyModule::new(py, "palettes")?;
    let manim_m = PyModule::new(py, "manim")?;
    manim_m.add("BLUE_E", PyColor(manim::BLUE_E))?;
    manim_m.add("BLUE_D", PyColor(manim::BLUE_D))?;
    manim_m.add("BLUE_C", PyColor(manim::BLUE_C))?;
    manim_m.add("BLUE_B", PyColor(manim::BLUE_B))?;
    manim_m.add("BLUE_A", PyColor(manim::BLUE_A))?;
    manim_m.add("TEAL_E", PyColor(manim::TEAL_E))?;
    manim_m.add("TEAL_D", PyColor(manim::TEAL_D))?;
    manim_m.add("TEAL_C", PyColor(manim::TEAL_C))?;
    manim_m.add("TEAL_B", PyColor(manim::TEAL_B))?;
    manim_m.add("TEAL_A", PyColor(manim::TEAL_A))?;
    manim_m.add("GREEN_E", PyColor(manim::GREEN_E))?;
    manim_m.add("GREEN_D", PyColor(manim::GREEN_D))?;
    manim_m.add("GREEN_C", PyColor(manim::GREEN_C))?;
    manim_m.add("GREEN_B", PyColor(manim::GREEN_B))?;
    manim_m.add("GREEN_A", PyColor(manim::GREEN_A))?;
    manim_m.add("YELLOW_E", PyColor(manim::YELLOW_E))?;
    manim_m.add("YELLOW_D", PyColor(manim::YELLOW_D))?;
    manim_m.add("YELLOW_C", PyColor(manim::YELLOW_C))?;
    manim_m.add("YELLOW_B", PyColor(manim::YELLOW_B))?;
    manim_m.add("YELLOW_A", PyColor(manim::YELLOW_A))?;
    manim_m.add("GOLD_E", PyColor(manim::GOLD_E))?;
    manim_m.add("GOLD_D", PyColor(manim::GOLD_D))?;
    manim_m.add("GOLD_C", PyColor(manim::GOLD_C))?;
    manim_m.add("GOLD_B", PyColor(manim::GOLD_B))?;
    manim_m.add("GOLD_A", PyColor(manim::GOLD_A))?;
    manim_m.add("RED_E", PyColor(manim::RED_E))?;
    manim_m.add("RED_D", PyColor(manim::RED_D))?;
    manim_m.add("RED_C", PyColor(manim::RED_C))?;
    manim_m.add("RED_B", PyColor(manim::RED_B))?;
    manim_m.add("RED_A", PyColor(manim::RED_A))?;
    manim_m.add("MAROON_E", PyColor(manim::MAROON_E))?;
    manim_m.add("MAROON_D", PyColor(manim::MAROON_D))?;
    manim_m.add("MAROON_C", PyColor(manim::MAROON_C))?;
    manim_m.add("MAROON_B", PyColor(manim::MAROON_B))?;
    manim_m.add("MAROON_A", PyColor(manim::MAROON_A))?;
    manim_m.add("PURPLE_E", PyColor(manim::PURPLE_E))?;
    manim_m.add("PURPLE_D", PyColor(manim::PURPLE_D))?;
    manim_m.add("PURPLE_C", PyColor(manim::PURPLE_C))?;
    manim_m.add("PURPLE_B", PyColor(manim::PURPLE_B))?;
    manim_m.add("PURPLE_A", PyColor(manim::PURPLE_A))?;
    manim_m.add("GREY_E", PyColor(manim::GREY_E))?;
    manim_m.add("GREY_D", PyColor(manim::GREY_D))?;
    manim_m.add("GREY_C", PyColor(manim::GREY_C))?;
    manim_m.add("GREY_B", PyColor(manim::GREY_B))?;
    manim_m.add("GREY_A", PyColor(manim::GREY_A))?;
    manim_m.add("WHITE", PyColor(manim::WHITE))?;
    manim_m.add("BLACK", PyColor(manim::BLACK))?;
    manim_m.add("GREEN_SCREEN", PyColor(manim::GREEN_SCREEN))?;
    manim_m.add("GREY_BROWN", PyColor(manim::GREY_BROWN))?;
    manim_m.add("LIGHT_BROWN", PyColor(manim::LIGHT_BROWN))?;
    manim_m.add("PINK", PyColor(manim::PINK))?;
    manim_m.add("LIGHT_PINK", PyColor(manim::LIGHT_PINK))?;
    manim_m.add("ORANGE", PyColor(manim::ORANGE))?;
    palettes.add_submodule(&manim_m)?;
    Ok(palettes)
}

// MARK: macros

/// Transforms and styles shared by all item wrappers.
macro_rules! impl_item_transforms {
    ($py_class:ident) => {
        #[pymethods]
        impl $py_class {
            /// Shift the item by an `(x, y, z)` offset.
            fn shift<'py>(
                mut slf: PyRefMut<'py, Self>,
                offset: (f64, f64, f64),
            ) -> PyRefMut<'py, Self> {
                slf.inner.shift(DVec3::from(offset));
                slf
            }

            /// Rotate the item around an `(x, y, z)` axis by `angle` radians.
            fn rotate<'py>(
                mut slf: PyRefMut<'py, Self>,
                angle: f64,
                axis: (f64, f64, f64),
            ) -> PyRefMut<'py, Self> {
                slf.inner.rotate_on_axis(DVec3::from(axis), angle);
                slf
            }

            /// Rotate the item around an axis through an arbitrary `point`.
            fn rotate_about<'py>(
                mut slf: PyRefMut<'py, Self>,
                angle: f64,
                axis: (f64, f64, f64),
                point: (f64, f64, f64),
            ) -> PyRefMut<'py, Self> {
                let point = DVec3::from(point);
                let axis = DVec3::from(axis);
                slf.inner.with_origin(point, |item| {
                    item.rotate_on_axis(axis, angle);
                });
                slf
            }

            /// The center of the item's bounding box.
            #[getter]
            fn center(&self) -> (f64, f64, f64) {
                AabbPoint::CENTER.locate(&self.inner).into()
            }

            /// Move the item's anchor to a point.
            fn move_to<'py>(
                mut slf: PyRefMut<'py, Self>,
                point: (f64, f64, f64),
            ) -> PyRefMut<'py, Self> {
                slf.inner.move_to(DVec3::from(point));
                slf
            }

            /// Set both fill and stroke color.
            fn set_color<'py>(mut slf: PyRefMut<'py, Self>, color: PyColor) -> PyRefMut<'py, Self> {
                slf.inner.set_color(color.0);
                slf
            }

            /// Set the fill color.
            fn set_fill_color<'py>(
                mut slf: PyRefMut<'py, Self>,
                color: PyColor,
            ) -> PyRefMut<'py, Self> {
                slf.inner.set_fill_color(color.0);
                slf
            }

            /// Set the stroke color.
            fn set_stroke_color<'py>(
                mut slf: PyRefMut<'py, Self>,
                color: PyColor,
            ) -> PyRefMut<'py, Self> {
                slf.inner.set_stroke_color(color.0);
                slf
            }

            /// Set the fill opacity.
            fn set_fill_opacity<'py>(
                mut slf: PyRefMut<'py, Self>,
                opacity: f32,
            ) -> PyRefMut<'py, Self> {
                slf.inner.set_fill_opacity(opacity);
                slf
            }

            /// Set the stroke opacity.
            fn set_stroke_opacity<'py>(
                mut slf: PyRefMut<'py, Self>,
                opacity: f32,
            ) -> PyRefMut<'py, Self> {
                slf.inner.set_stroke_opacity(opacity);
                slf
            }
        }
    };
}

/// Animation constructors shared by `VItem`-backed wrappers.
macro_rules! impl_item_anims {
    ($py_class:ident) => {
        #[pymethods]
        impl $py_class {
            /// Fade the item in.
            fn fade_in(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.fade_in())
            }

            /// Fade the item out.
            fn fade_out(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.fade_out())
            }

            /// Create (draw) the item.
            fn create(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.create())
            }

            /// Un-create (erase) the item.
            fn uncreate(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.uncreate())
            }

            /// Write the item (create with a writing style).
            fn write(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.write())
            }

            /// Unwrite the item.
            fn unwrite(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.unwrite())
            }

            /// Show the item statically.
            fn show(&self) -> PyAnim {
                PyAnim::from_anim(self.inner.show())
            }

            /// Hide the item statically.
            fn hide(&self) -> PyAnim {
                PyAnim::from_anim(self.inner.hide())
            }
        }
    };
}

// MARK: VItem

/// A vector item (the fundamental renderable shape).
#[pyclass(name = "VItem")]
#[derive(Clone)]
pub struct PyVItem {
    pub(crate) inner: VItem,
}

#[pymethods]
impl PyVItem {
    /// Create a `VItem` from a list of `(x, y, z)` anchor points.
    #[new]
    fn new(points: Vec<(f64, f64, f64)>) -> Self {
        let points = points.into_iter().map(DVec3::from).collect::<Vec<_>>();
        Self {
            inner: VItem::from_vpoints(points),
        }
    }

    /// Scale the item uniformly by `factor`.
    fn scale<'py>(mut slf: PyRefMut<'py, Self>, factor: f64) -> PyRefMut<'py, Self> {
        slf.inner.scale(DVec3::splat(factor));
        slf
    }

    /// Set the stroke width.
    fn set_stroke_width<'py>(mut slf: PyRefMut<'py, Self>, width: f32) -> PyRefMut<'py, Self> {
        slf.inner.set_stroke_width(width);
        slf
    }

    /// Morph the item into another `VItem`.
    fn morph_to(&mut self, target: PyVItem) -> PyAnim {
        PyAnim::from_anim(self.inner.morph_to(target.inner))
    }

    /// Clone the item.
    fn clone(&self) -> Self {
        Clone::clone(self)
    }
}

impl_item_transforms!(PyVItem);
impl_item_anims!(PyVItem);

// MARK: Square / Circle

/// Shorthand for geometry wrappers backed by a dedicated Rust item type
/// that converts into `VItem`.
macro_rules! impl_geometry_item {
    ($py_class:ident, $inner:ty) => {
        #[pymethods]
        impl $py_class {
            /// Scale the item uniformly by `factor`.
            fn scale<'py>(mut slf: PyRefMut<'py, Self>, factor: f64) -> PyRefMut<'py, Self> {
                slf.inner.scale(factor);
                slf
            }

            /// Set the stroke width.
            fn set_stroke_width<'py>(
                mut slf: PyRefMut<'py, Self>,
                width: f32,
            ) -> PyRefMut<'py, Self> {
                slf.inner.stroke_width = width;
                slf
            }

            /// Convert into a `VItem` (e.g. for `morph_to`).
            fn to_vitem(&self) -> PyVItem {
                PyVItem {
                    inner: VItem::from(self.inner.clone()),
                }
            }

            /// Clone the item.
            fn clone(&self) -> Self {
                Clone::clone(self)
            }
        }
    };
}

/// A square.
#[pyclass(name = "Square")]
#[derive(Clone)]
pub struct PySquare {
    inner: Square,
}

#[pymethods]
impl PySquare {
    /// Create a square with the given side length.
    #[new]
    fn new(size: f64) -> Self {
        Self {
            inner: Square::new(size),
        }
    }
}

impl_geometry_item!(PySquare, Square);
impl_item_transforms!(PySquare);

/// Fading and static anims for geometry items.
macro_rules! impl_geometry_anims {
    ($py_class:ident) => {
        #[pymethods]
        impl $py_class {
            /// Fade the item in.
            fn fade_in(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.fade_in())
            }

            /// Fade the item out.
            fn fade_out(&mut self) -> PyAnim {
                PyAnim::from_anim(self.inner.fade_out())
            }

            /// Show the item statically.
            fn show(&self) -> PyAnim {
                PyAnim::from_anim(self.inner.show())
            }

            /// Hide the item statically.
            fn hide(&self) -> PyAnim {
                PyAnim::from_anim(self.inner.hide())
            }
        }
    };
}

impl_geometry_anims!(PySquare);

/// A circle.
#[pyclass(name = "Circle")]
#[derive(Clone)]
pub struct PyCircle {
    inner: Circle,
}

#[pymethods]
impl PyCircle {
    /// Create a circle with the given radius.
    #[new]
    fn new(radius: f64) -> Self {
        Self {
            inner: Circle::new(radius),
        }
    }
}

impl_geometry_item!(PyCircle, Circle);
impl_item_transforms!(PyCircle);
impl_geometry_anims!(PyCircle);

// MARK: SvgItem

/// An SVG picture, as a group of `VItem`s.
#[pyclass(name = "SvgItem")]
#[derive(Clone)]
pub struct PySvgItem {
    inner: Vec<VItem>,
}

#[pymethods]
impl PySvgItem {
    /// Create an `SvgItem` from an SVG string.
    #[new]
    fn new(svg: &str) -> Self {
        Self {
            inner: Vec::<VItem>::from(SvgItem::new(svg)),
        }
    }

    /// Scale the item uniformly by `factor`.
    fn scale<'py>(mut slf: PyRefMut<'py, Self>, factor: f64) -> PyRefMut<'py, Self> {
        slf.inner.scale(DVec3::splat(factor));
        slf
    }

    /// Set the stroke width.
    fn set_stroke_width<'py>(mut slf: PyRefMut<'py, Self>, width: f32) -> PyRefMut<'py, Self> {
        slf.inner.set_stroke_width(width);
        slf
    }

    /// Clone the item.
    fn clone(&self) -> Self {
        Clone::clone(self)
    }
}

impl_item_transforms!(PySvgItem);
impl_geometry_anims!(PySvgItem);

// MARK: CameraFrame

/// The camera frame. Every scene should `show()` one for the content's
/// duration, otherwise nothing is rendered.
#[pyclass(name = "CameraFrame")]
#[derive(Clone, Default)]
pub struct PyCameraFrame {
    inner: CameraFrame,
}

#[pymethods]
impl PyCameraFrame {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Show the camera frame statically.
    fn show(&self) -> PyAnim {
        PyAnim::from_anim(self.inner.show())
    }

    /// Hide the camera frame statically.
    fn hide(&self) -> PyAnim {
        PyAnim::from_anim(self.inner.hide())
    }
}
