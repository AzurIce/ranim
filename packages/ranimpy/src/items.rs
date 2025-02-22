use pyo3::{
    pyclass, pymethods,
    types::{PyModule, PyModuleMethods},
    Bound, PyResult,
};
use ranim::{
    glam::vec3,
    items::{svg_item::SvgItem, vitem::VItem, Rabject},
};

pub fn items(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVItem>()?;
    m.add_class::<PySvgItem>()?;
    Ok(())
}

// MARK: SvgItem

/// SvgItem
#[pyclass]
#[pyo3(name = "SvgItem")]
pub struct PySvgItem {
    pub(crate) inner: Rabject<SvgItem>,
}

#[pymethods]
impl PySvgItem {
    #[new]
    pub fn new(svg_str: &str) -> Self {
        Self {
            inner: Rabject::new(SvgItem::from_svg(svg_str)),
        }
    }
}

// MARK: VItem

#[pyclass]
#[pyo3(name = "VItem")]
pub struct PyVItem {
    pub(crate) inner: Rabject<VItem>,
}

#[pymethods]
impl PyVItem {
    #[new]
    pub fn new(vpoints: Vec<[f32; 3]>) -> Self {
        let vpoints = vpoints.iter().map(|v| vec3(v[0], v[1], v[2])).collect();
        Self {
            inner: Rabject::new(VItem::from_vpoints(vpoints)),
        }
    }
}
// MARK: macro impl_transformable

use ranim::components::TransformAnchor;
use ranim::glam::{IVec3, Vec3};
use ranim::prelude::*;

macro_rules! impl_transformable {
    ($py_class:ident) => {
        #[pymethods]
        impl $py_class {
            // 基础变换方法
            fn shift(&mut self, shift: (f32, f32, f32)) {
                self.inner.shift(Vec3::from(shift));
            }

            fn scale(&mut self, scale: (f32, f32, f32)) {
                self.inner.scale(Vec3::from(scale));
            }

            // fn scale_by_anchor(
            //     &mut self,
            //     scale: (f32, f32, f32),
            //     anchor: TransformAnchor
            // )  {
            //     self.inner.scale_by_anchor(Vec3::from(scale), anchor);
            // }

            // 旋转方法
            fn rotate(&mut self, angle: f32, axis: (f32, f32, f32)) {
                self.inner.rotate(angle, Vec3::from(axis));
            }

            fn rotate_by_point(
                &mut self,
                angle: f32,
                axis: (f32, f32, f32),
                anchor: (f32, f32, f32),
            ) {
                self.inner.rotate_by_anchor(
                    angle,
                    Vec3::from(axis),
                    TransformAnchor::Point(Vec3::from(anchor)),
                );
            }

            fn rotate_by_edge(
                &mut self,
                angle: f32,
                axis: (f32, f32, f32),
                anchor: (i32, i32, i32),
            ) {
                self.inner.rotate_by_anchor(
                    angle,
                    Vec3::from(axis),
                    TransformAnchor::Edge(IVec3::from(anchor)),
                );
            }
            // 定位方法
            fn put_center_on(&mut self, point: (f32, f32, f32)) {
                self.inner.put_center_on(Vec3::from(point));
            }

            fn put_start_and_end_on(&mut self, start: (f32, f32, f32), end: (f32, f32, f32)) {
                self.inner
                    .put_start_and_end_on(Vec3::from(start), Vec3::from(end));
            }

            // 获取信息方法
            #[getter]
            fn start_position(&self) -> Option<(f32, f32, f32)> {
                self.inner.get_start_position().map(Into::into)
            }

            #[getter]
            fn end_position(&self) -> Option<(f32, f32, f32)> {
                self.inner.get_end_position().map(Into::into)
            }

            #[getter]
            fn bounding_box(&self) -> Vec<(f32, f32, f32)> {
                self.inner
                    .get_bounding_box()
                    .iter()
                    .map(|v| (*v).into())
                    .collect()
            }

            fn get_bounding_box_point(&self, edge: (i32, i32, i32)) -> (f32, f32, f32) {
                self.inner.get_bounding_box_point(IVec3::from(edge)).into()
            }

            fn get_bounding_box_corners(&self) -> Vec<(f32, f32, f32)> {
                self.inner
                    .get_bounding_box()
                    .iter()
                    .map(|v| (*v).into())
                    .collect()
            }
        }
    };
}

impl_transformable! {PyVItem}
impl_transformable! {PySvgItem}
