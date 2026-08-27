//! glTF 2.0 scene-graph import into trees of [`MeshItem`]s.
//!
//! This module is a proof of concept for importing a glTF scene as a
//! [`Node`](crate::hierarchy::Node) tree, validating that the generic tree
//! works for any item type — [`MeshItem`] here, exactly like
//! [`VItem`][crate::vitem::VItem] in the SVG pipeline. It is opt-in via the
//! `gltf` cargo feature and stays I/O-free: callers parse the file (e.g.
//! with `gltf::Gltf::from_slice`) and hand over a closure that resolves each
//! glTF buffer to its byte slice, mirroring `gltf::mesh::Primitive::reader`.
//!
//! # Mapping
//!
//! | glTF                       | ranim                                              |
//! |----------------------------|----------------------------------------------------|
//! | node TRS / matrix          | [`Node::transform`](crate::hierarchy::Node::transform) as a [`DAffine3`](ranim_core::glam::DAffine3) (`T * R * S`) |
//! | node name (non-empty)      | [`Node::id`](crate::hierarchy::Node::id)           |
//! | node children              | [`NodeContent::Children`](crate::hierarchy::NodeContent::Children), source order kept |
//! | primitive                  | leaf [`MeshItem`] with identity transform          |
//!
//! Per-primitive attributes: `POSITION` → [`MeshItem::points`], `indices` →
//! [`MeshItem::triangle_indices`] (a non-indexed primitive gets sequential
//! indices, because an index-less [`MeshItem`] would mean a point cloud
//! rather than triangles), `NORMAL` → [`MeshItem::vertex_normals`] (all-zero
//! when absent, matching [`MeshItem::from_indexed_vertices`], i.e. the
//! shader's flat-shading fallback), and `COLOR_0` →
//! [`MeshItem::vertex_colors`] (3-component colors get alpha 1.0; when the
//! attribute is absent the [`MeshItem`] default `Rgba::default()` is kept).
//! All f32/u8/u16 normalized attribute variants are normalized by the glTF
//! reader API itself. COLOR_0 values are taken as-is into `Rgba`'s linear
//! storage, which is spec-correct for the f32 variant but skips the
//! sRGB-to-linear conversion that normalized integer variants require — a
//! known POC simplification.
//!
//! Transforms are stored as the widest [`DAffine3`](ranim_core::glam::DAffine3)
//! because glTF matrices may carry shear; narrowing to
//! [`Rigid`](ranim_core::traits::Rigid) /
//! [`Similarity`](ranim_core::traits::Similarity) can come later via
//! [`Node::map_transform`](crate::hierarchy::Node::map_transform) and
//! `TryFrom`.
//!
//! # Known limitations (deliberate, POC scope)
//!
//! - **mesh + children**: glTF allows a node to carry both a mesh and child
//!   nodes. The tree cannot express both as one payload, so the primitives
//!   are split off as sibling leaves placed *before* the child nodes (each
//!   with an identity transform); the node's own transform still applies to
//!   both. Narrowing this split needs a multi-payload node.
//! - **single scene**: only the default scene (falling back to the first
//!   scene) is imported.
//! - **triangle meshes only**: primitives whose drawing mode is not
//!   `TRIANGLES` are imported as-is after a warning; their indices are not
//!   re-triangulated.
//! - A missing scene imports as an empty group rather than failing.
//!
//! # Examples
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ranim_items::mesh::gltf::node_tree_from_gltf;
//!
//! let bytes = std::fs::read("model.gltf")?;
//! let gltf = gltf::Gltf::from_slice(&bytes)?;
//! let blob = gltf.blob.clone();
//! // Resolve buffers the way the caller sees fit: GLB blob, data URIs,
//! // or external files — the loader only ever asks for byte slices.
//! let tree = node_tree_from_gltf(&gltf.document, |buffer| match buffer.source() {
//!     gltf::buffer::Source::Bin => blob.as_deref(),
//!     gltf::buffer::Source::Uri(_) => None, // external buffers: read the file here
//! });
//! # Ok(())
//! # }
//! ```

use crate::hierarchy::{Node, NodeContent};
use crate::mesh::MeshItem;
use ranim_core::components::rgba::Rgba;
use ranim_core::glam::{DAffine3, DMat4, DQuat, DVec3, Vec4, dvec3};

/// Builds the node tree of the default glTF scene as [`Node`]s of
/// [`MeshItem`]s.
///
/// `get_buffer_data` resolves a glTF buffer to its byte slice; returning
/// `None` makes the affected accessors unreadable, so primitives fall back to
/// their documented defaults. The bound is higher-ranked over the buffer's
/// lifetime (glTF's `Primitive::reader` ties its own borrow to the call);
/// the returned slice only borrows the resolver's own storage (e.g. a GLB
/// blob), never the document. See the [module docs](self) for the full
/// mapping and the POC limitations.
pub fn node_tree_from_gltf<'s, F>(doc: &gltf::Document, get_buffer_data: F) -> Node<MeshItem>
where
    F: Clone + for<'b> Fn(gltf::buffer::Buffer<'b>) -> Option<&'s [u8]>,
{
    let scene = match doc.default_scene().or_else(|| doc.scenes().next()) {
        Some(scene) => scene,
        None => {
            tracing::warn!("glTF document has no scene, importing an empty tree");
            return Node::group(Vec::new());
        }
    };
    let children = scene
        .nodes()
        .map(|node| node_tree_from_gltf_node(node, &get_buffer_data))
        .collect();
    Node::group(children)
}

/// Converts one glTF node (recursively, via [`gltf::scene::Node::children`]).
fn node_tree_from_gltf_node<'s, F>(
    node: gltf::scene::Node<'_>,
    get_buffer_data: &F,
) -> Node<MeshItem>
where
    F: Clone + for<'b> Fn(gltf::buffer::Buffer<'b>) -> Option<&'s [u8]>,
{
    let transform = node_transform(node.transform());
    let id = non_empty(node.name());

    let mut children: Vec<Node<MeshItem>> = Vec::new();
    if let Some(mesh) = node.mesh() {
        for (index, primitive) in mesh.primitives().enumerate() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                tracing::warn!(
                    "primitive {index} of mesh {:?} is not TRIANGLES, importing indices as-is",
                    mesh.name()
                );
            }
            let leaf = Node::leaf(primitive_mesh_item(primitive, get_buffer_data));
            children.push(match primitive_leaf_id(mesh.name(), node.name(), index) {
                Some(id) => leaf.with_id(id),
                None => leaf,
            });
        }
    }
    // Known debt: a glTF node may carry both a mesh and child nodes; the
    // primitives above become sibling leaves placed before the children.
    children.extend(
        node.children()
            .map(|child| node_tree_from_gltf_node(child, get_buffer_data)),
    );

    let node = Node::new(transform, NodeContent::Children(children));
    match id {
        Some(id) => node.with_id(id),
        None => node,
    }
}

/// Converts one glTF primitive to a [`MeshItem`].
fn primitive_mesh_item<'s, F>(primitive: gltf::mesh::Primitive<'_>, get_buffer_data: &F) -> MeshItem
where
    F: Clone + for<'b> Fn(gltf::buffer::Buffer<'b>) -> Option<&'s [u8]>,
{
    let reader = primitive.reader(get_buffer_data);

    let points: Vec<DVec3> = match reader.read_positions() {
        Some(positions) => positions
            .map(|p| dvec3(p[0] as f64, p[1] as f64, p[2] as f64))
            .collect(),
        None => {
            tracing::warn!("primitive without POSITION attribute, importing no vertices");
            Vec::new()
        }
    };
    // Non-indexed glTF primitives draw their vertices as consecutive
    // triangles; synthesize the identity indexing because an index-less
    // MeshItem would mean a point cloud.
    let triangle_indices: Vec<u32> = match reader.read_indices() {
        Some(indices) => indices.into_u32().collect(),
        None => (0..points.len() as u32).collect(),
    };
    // Absent normals stay all-zero: MeshItem's contract for flat shading.
    let vertex_normals: Vec<DVec3> = match reader.read_normals() {
        Some(normals) => normals
            .map(|n| dvec3(n[0] as f64, n[1] as f64, n[2] as f64))
            .collect(),
        None => vec![DVec3::ZERO; points.len()],
    };
    // Absent colors keep the MeshItem default (matching
    // MeshItem::from_indexed_vertices); 3-component colors get alpha 1.0.
    let vertex_colors: Vec<Rgba> = match reader.read_colors(0) {
        Some(colors) => colors
            .into_rgba_f32()
            .map(|rgba| Rgba(Vec4::from_array(rgba)))
            .collect(),
        None => vec![Rgba::default(); points.len()],
    };

    MeshItem {
        points: points.into(),
        triangle_indices,
        vertex_colors: vertex_colors.into(),
        vertex_normals: vertex_normals.into(),
    }
}

/// Converts a glTF node transform to a [`DAffine3`]: the 4x4 matrix as-is, or
/// the decomposed `T * R * S` (glTF semantics; the glTF crate's
/// [`gltf::scene::Transform::matrix`] uses the same equation).
fn node_transform(transform: gltf::scene::Transform) -> DAffine3 {
    match transform {
        gltf::scene::Transform::Matrix { matrix } => {
            let matrix = matrix.map(|col| col.map(|v| v as f64));
            DAffine3::from_mat4(DMat4::from_cols_array_2d(&matrix))
        }
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => {
            let rotation = DQuat::from_xyzw(
                rotation[0] as f64,
                rotation[1] as f64,
                rotation[2] as f64,
                rotation[3] as f64,
            );
            let translation = dvec3(
                translation[0] as f64,
                translation[1] as f64,
                translation[2] as f64,
            );
            let scale = dvec3(scale[0] as f64, scale[1] as f64, scale[2] as f64);
            DAffine3::from_rotation_translation(rotation, translation) * DAffine3::from_scale(scale)
        }
    }
}

/// The id of the leaf holding primitive `index` of a mesh on a node: the mesh
/// name when present (disambiguated by index), else the node name, else none.
fn primitive_leaf_id(
    mesh_name: Option<&str>,
    node_name: Option<&str>,
    index: usize,
) -> Option<String> {
    match non_empty(mesh_name) {
        Some(mesh_name) => Some(format!("{mesh_name}.{index}")),
        None => non_empty(node_name).map(|node_name| format!("{node_name}.primitive{index}")),
    }
}

/// `None` for empty strings, so blank glTF names do not become ids.
fn non_empty(name: Option<&str>) -> Option<&str> {
    name.filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use ranim_core::core_item::CoreItem;
    use ranim_core::{Extract, glam::Vec3};

    use super::*;

    fn push_f32(buf: &mut Vec<u8>, value: f32) {
        buf.extend_from_slice(&value.to_le_bytes());
    }

    /// A minimal GLB: scene → "parent" (T(1,2,3)·Rz(90°)) → "child" (scale 2)
    /// carrying one triangle mesh with POSITION, NORMAL, COLOR_0, indices.
    fn triangle_glb() -> Vec<u8> {
        let mut bin = Vec::new();
        for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for c in p {
                push_f32(&mut bin, c);
            }
        }
        for _ in 0..3 {
            for c in [0.0f32, 0.0, 1.0] {
                push_f32(&mut bin, c);
            }
        }
        for rgba in [
            [1.0f32, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
        ] {
            for c in rgba {
                push_f32(&mut bin, c);
            }
        }
        for i in [0u32, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        assert_eq!(bin.len(), 132);

        let json = r#"
        {
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [
                {"name": "parent", "translation": [1.0, 2.0, 3.0],
                 "rotation": [0.0, 0.0, 0.7071067811865476, 0.7071067811865476],
                 "children": [1]},
                {"name": "child", "scale": [2.0, 2.0, 2.0], "mesh": 0}
            ],
            "meshes": [{"name": "tri", "primitives": [{
                "attributes": {"POSITION": 0, "NORMAL": 1, "COLOR_0": 2},
                "indices": 3
            }]}],
            "buffers": [{"byteLength": 132}],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 36},
                {"buffer": 0, "byteOffset": 72, "byteLength": 48},
                {"buffer": 0, "byteOffset": 120, "byteLength": 12}
            ],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3,
                 "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]},
                {"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4"},
                {"bufferView": 3, "componentType": 5125, "count": 3, "type": "SCALAR"}
            ]
        }"#;
        let mut json_chunk = json.as_bytes().to_vec();
        while !json_chunk.len().is_multiple_of(4) {
            json_chunk.push(b' ');
        }
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }

        let total = 12 + 8 + json_chunk.len() + 8 + bin.len();
        let mut glb = Vec::with_capacity(total);
        glb.extend_from_slice(&0x46546C67u32.to_le_bytes()); // "glTF"
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
        glb.extend_from_slice(&json_chunk);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
        glb.extend_from_slice(&bin);
        glb
    }

    fn triangle_tree() -> Node<MeshItem> {
        let glb = triangle_glb();
        let gltf = gltf::Gltf::from_slice(&glb).unwrap();
        let blob = gltf.blob.clone();
        node_tree_from_gltf(&gltf.document, |buffer| match buffer.source() {
            gltf::buffer::Source::Bin => blob.as_deref(),
            gltf::buffer::Source::Uri(_) => None,
        })
    }

    #[test]
    fn tree_shape_mirrors_the_gltf_node_graph() {
        let tree = triangle_tree();
        assert!(tree.is_group());
        let roots = tree.children().unwrap();
        assert_eq!(roots.len(), 1);

        let parent = &roots[0];
        assert_eq!(parent.id.as_deref(), Some("parent"));
        assert!(parent.is_group());
        let parent_children = parent.children().unwrap();
        assert_eq!(parent_children.len(), 1);

        let child = &parent_children[0];
        assert_eq!(child.id.as_deref(), Some("child"));
        assert!(child.is_group());
        let leaves = child.children().unwrap();
        assert_eq!(leaves.len(), 1);
        // The primitive leaf inherits the mesh name, disambiguated by index.
        assert_eq!(leaves[0].id.as_deref(), Some("tri.0"));
        assert!(leaves[0].is_leaf());
    }

    #[test]
    fn node_transforms_match_trs_semantics() {
        let tree = triangle_tree();
        let parent = &tree.children().unwrap()[0];
        let child = &parent.children().unwrap()[0];

        // Parent: T(1,2,3) composed with Rz(+90deg), applied after rotation.
        // Rotation-derived values carry ~1e-7 f32 noise from the glTF source,
        // so their tolerance is 1e-6 (translations/scales stay exact).
        let parent_t = &parent.transform;
        assert!(
            parent_t
                .transform_point3(DVec3::ZERO)
                .abs_diff_eq(dvec3(1.0, 2.0, 3.0), 1e-9)
        );
        assert!((parent_t.matrix3 * DVec3::X).abs_diff_eq(DVec3::Y, 1e-6));

        // Child: pure uniform scale of 2.
        assert!(
            child
                .transform
                .transform_point3(dvec3(1.0, 1.0, 1.0))
                .abs_diff_eq(dvec3(2.0, 2.0, 2.0), 1e-9)
        );
    }

    #[test]
    fn leaf_mesh_round_trips_primitive_data() {
        let tree = triangle_tree();
        let leaf = tree.children().unwrap()[0].children().unwrap()[0]
            .children()
            .unwrap()[0]
            .leaf_content()
            .unwrap();

        let points: Vec<DVec3> = leaf.points.iter().cloned().collect();
        assert_eq!(
            points,
            vec![
                dvec3(0.0, 0.0, 0.0),
                dvec3(1.0, 0.0, 0.0),
                dvec3(0.0, 1.0, 0.0)
            ]
        );
        assert_eq!(leaf.triangle_indices, vec![0, 1, 2]);
        assert!(
            leaf.vertex_normals
                .iter()
                .all(|n| n.abs_diff_eq(DVec3::Z, 1e-6))
        );
        let first_color = leaf.vertex_colors[0].0;
        assert!((first_color - Vec4::new(1.0, 0.0, 0.0, 1.0)).abs_diff_eq(Vec4::ZERO, 1e-6));
    }

    #[test]
    fn extraction_composes_the_world_transform_for_meshes() {
        let tree = triangle_tree();

        // leaves() yields the full f64 chain: parent(T·R) * child(S2).
        // f32 source noise puts the 1e-6 tolerance on rotation contributions.
        let (world, _) = tree.leaves().next().unwrap();
        assert!(
            world
                .transform_point3(dvec3(1.0, 0.0, 0.0))
                .abs_diff_eq(dvec3(1.0, 4.0, 3.0), 1e-6)
        );

        // Extraction bakes the same chain into the core item's transform.
        let extracted = tree.extract();
        assert_eq!(extracted.len(), 1);
        match &extracted[0] {
            CoreItem::MeshItem(mesh) => {
                let local = Vec3::new(1.0, 0.0, 0.0);
                let world = mesh.transform.transform_point3(local);
                assert!((world - Vec3::new(1.0, 4.0, 3.0)).length() < 1e-4);
                // Vertex data stays local.
                assert_eq!(mesh.points.len(), 3);
            }
            _ => panic!("expected a MeshItem"),
        }
    }
}
