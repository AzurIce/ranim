//! Core items of Ranim.
//!
//! [`CoreItem`]s are the fundamental items of Ranim. All other Items are built upon them.
//!
//! Currently, there are two types of [`CoreItem`]s:
//! - [`CameraFrame`]: The camera frame.
//! - [`VItem`]: The vitem primitive.
use std::any::Any;

use dyn_clone::DynClone;

use crate::{
    Extract,
    core_item::{camera_frame::CameraFrame, mesh_item::MeshItem, vitem::VItem},
    traits::Interpolatable,
};

/// Camera frame
pub mod camera_frame;
/// MeshItem
pub mod mesh_item;
/// Vitem
pub mod vitem;

/// The core ranim builtin items
#[derive(Debug, Clone, PartialEq)]
pub enum CoreItem {
    /// [`CameraFrame`]
    CameraFrame(CameraFrame),
    /// [`VItem`]
    VItem(VItem),
    /// [`MeshItem`]
    MeshItem(MeshItem),
}

/// The item that can be extracted to [`CoreItem`]s
pub trait AnyExtractCoreItem: Any + Extract<Target = CoreItem> + DynClone {}
impl<T: Extract<Target = CoreItem> + Any + DynClone> AnyExtractCoreItem for T {}

dyn_clone::clone_trait_object!(AnyExtractCoreItem);

/// A dynamic item, basically type erased [`AnyExtractCoreItem`]
#[derive(Clone)]
pub struct DynItem(pub Box<dyn AnyExtractCoreItem>);

impl Extract for DynItem {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.0.extract_into(buf);
    }
}

impl<T: Into<CoreItem> + Clone> Extract for T {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        buf.push(self.clone().into());
    }
}

impl<T: Extract<Target = CoreItem>> Extract for Vec<T> {
    type Target = CoreItem;
    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        for item in self {
            item.extract_into(buf);
        }
    }
}

impl Interpolatable for Vec<CoreItem> {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        self.iter()
            .zip(target.iter())
            .map(|(a, b)| match (a, b) {
                (CoreItem::CameraFrame(a), CoreItem::CameraFrame(b)) => {
                    CoreItem::CameraFrame(a.lerp(b, t))
                }
                (CoreItem::VItem(a), CoreItem::VItem(b)) => CoreItem::VItem(a.lerp(b, t)),
                (CoreItem::MeshItem(a), CoreItem::MeshItem(b)) => CoreItem::MeshItem(a.lerp(b, t)),
                _ => unreachable!("align_with should ensure matching variants"),
            })
            .collect()
    }

    fn is_aligned(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self.iter().zip(other.iter()).all(|(a, b)| match (a, b) {
                (CoreItem::CameraFrame(a), CoreItem::CameraFrame(b)) => a.is_aligned(b),
                (CoreItem::VItem(a), CoreItem::VItem(b)) => a.is_aligned(b),
                (CoreItem::MeshItem(a), CoreItem::MeshItem(b)) => a.is_aligned(b),
                _ => false,
            })
    }

    fn align_with(&mut self, other: &mut Self) {
        // Split both vecs by variant type
        let split = |v: &mut Vec<CoreItem>| -> (Vec<CameraFrame>, Vec<VItem>, Vec<MeshItem>) {
            let mut cameras = Vec::new();
            let mut vitems = Vec::new();
            let mut meshes = Vec::new();
            for item in v.drain(..) {
                match item {
                    CoreItem::CameraFrame(c) => cameras.push(c),
                    CoreItem::VItem(vi) => vitems.push(vi),
                    CoreItem::MeshItem(m) => meshes.push(m),
                }
            }
            (cameras, vitems, meshes)
        };

        let (mut a_cam, mut a_vi, mut a_mesh) = split(self);
        let (mut b_cam, mut b_vi, mut b_mesh) = split(other);

        // Align counts by padding with defaults
        fn pad_to_same_len<T: Default>(a: &mut Vec<T>, b: &mut Vec<T>) {
            let len = a.len().max(b.len());
            a.resize_with(len, T::default);
            b.resize_with(len, T::default);
        }
        pad_to_same_len(&mut a_cam, &mut b_cam);
        pad_to_same_len(&mut a_vi, &mut b_vi);
        pad_to_same_len(&mut a_mesh, &mut b_mesh);

        // Align each pair element-wise
        a_cam
            .iter_mut()
            .zip(b_cam.iter_mut())
            .for_each(|(x, y)| x.align_with(y));
        a_vi.iter_mut()
            .zip(b_vi.iter_mut())
            .for_each(|(x, y)| x.align_with(y));
        a_mesh
            .iter_mut()
            .zip(b_mesh.iter_mut())
            .for_each(|(x, y)| x.align_with(y));

        // Reassemble in consistent order: CameraFrame, VItem, MeshItem
        let reassemble =
            |cams: Vec<CameraFrame>, vis: Vec<VItem>, ms: Vec<MeshItem>| -> Vec<CoreItem> {
                let mut out = Vec::with_capacity(cams.len() + vis.len() + ms.len());
                out.extend(cams.into_iter().map(CoreItem::CameraFrame));
                out.extend(vis.into_iter().map(CoreItem::VItem));
                out.extend(ms.into_iter().map(CoreItem::MeshItem));
                out
            };

        *self = reassemble(a_cam, a_vi, a_mesh);
        *other = reassemble(b_cam, b_vi, b_mesh);
    }
}
