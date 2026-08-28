//! glTF 2.0 scene-graph import into trees of [`MeshItem`]s, opt-in via the
//! `gltf` cargo feature. The `gltf` crate itself is re-exported here, so
//! callers can parse documents without their own matching dependency.
//!
//! Two loaders: [`node_tree_from_path`](crate::mesh::gltf::node_tree_from_path)
//! reads a `.glb` (embedded blob) or `.gltf` (external buffer files resolved
//! relative to the file) from disk, and the I/O-free
//! [`node_tree_from_gltf`](crate::mesh::gltf::node_tree_from_gltf) takes a
//! parsed document plus a buffer resolver.
//!
//! # Mapping
//!
//! | glTF                       | ranim                                              |
//! |----------------------------|----------------------------------------------------|
//! | node TRS / matrix          | [`Node`](crate::hierarchy::Node) pose as a [`DAffine3`](ranim_core::glam::DAffine3) (`T * R * S`) |
//! | node name (non-empty)      | [`Node::id`](crate::hierarchy::Node::id) (payload nodes fall back to the mesh name) |
//! | node children              | [`Node::children`](crate::hierarchy::Node::children), source order kept |
//! | mesh (single primitive)    | the node's own payload [`MeshItem`]                |
//! | mesh (multiple primitives) | primitive leaves before the children, identity transforms |
//!
//! glTF also addresses nodes by document index (animation channels,
//! `skin.joints`) — [`GltfTree::node`](crate::mesh::gltf::GltfTree::node)
//! resolves that, [`Node::by_id`](crate::hierarchy::Node::by_id) resolves
//! names. `POSITION`/indices/`NORMAL`/`COLOR_0` map to the [`MeshItem`]
//! fields with zero-normals and default colors as the absent-attribute
//! fallbacks; `COLOR_0` is stored as-is (no sRGB conversion for normalized
//! integer variants).
//!
//! # Scope
//!
//! Only the default scene imports (missing scene → empty tree). Cameras,
//! lights, materials/textures (color comes from `COLOR_0` or whatever you
//! set after loading), `TEXCOORD`/`TANGENT` streams, animations, skins,
//! morph targets and all extensions are not interpreted — Draco-compressed
//! primitives therefore import as empty meshes with a warning.
//! Non-`TRIANGLES` modes import their indices unchanged after a warning.
//! glTF's mandated Y-up is converted to ranim's Z-up by composing
//! `Rx(π/2)` into the root pose — apply the inverse there for verbatim
//! coordinates.
//!
//! # Examples
//!
//! ```rust,no_run
//! # fn main() -> Result<(), ranim_items::mesh::gltf::GltfLoadError> {
//! use ranim_items::mesh::gltf::node_tree_from_path;
//!
//! let tree = node_tree_from_path("model.glb")?;
//! # Ok(())
//! # }
//! ```
//!
//! For custom I/O, parse yourself and hand over a buffer resolver:
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ranim_items::mesh::gltf::{gltf, node_tree_from_gltf};
//!
//! let bytes = std::fs::read("model.gltf")?;
//! let parsed = gltf::Gltf::from_slice(&bytes)?;
//! let blob = parsed.blob.clone();
//! let tree = node_tree_from_gltf(&parsed.document, |buffer| match buffer.source() {
//!     gltf::buffer::Source::Bin => blob.as_deref(),
//!     gltf::buffer::Source::Uri(_) => None, // read the file here
//! });
//! # Ok(())
//! # }
//! ```

use std::path::Path;

/// The `gltf` crate this module is built on, re-exported so callers can
/// parse documents (and match on [`GltfLoadError::Parse`]'s error type)
/// without their own matching dependency.
pub use gltf;

use ranim_core::core_item::transformed::Transformed;

use crate::hierarchy::Node;
use crate::mesh::MeshItem;
use ranim_core::components::rgba::Rgba;
use ranim_core::glam::{DAffine3, DMat4, DQuat, DVec3, Vec4, dvec3};

/// A glTF scene imported as a [`Node`] tree, plus the mapping from glTF node
/// indices to index paths in the tree.
///
/// glTF addresses nodes structurally by index (animation channels and
/// `skin.joints` reference the document's node array), while names are
/// optional display labels that are neither unique nor guaranteed present.
/// This type carries both views: [`GltfTree::node`] resolves a document
/// index, and [`Node::by_id`](crate::hierarchy::Node::by_id) resolves a
/// label.
pub struct GltfTree {
    /// The default scene (or the first scene) as a node tree; the scene's
    /// root nodes are direct children of this synthetic root group.
    pub tree: Node<MeshItem>,
    /// glTF node index → index path into [`GltfTree::tree`] (see
    /// [`Node::get`](crate::hierarchy::Node::get)). `None` for document
    /// nodes that are not part of the imported scene.
    pub node_paths: Vec<Option<Vec<usize>>>,
}

impl GltfTree {
    /// The tree node for glTF node `index`, or `None` when the index is out
    /// of range or the node is not part of the imported scene.
    pub fn node(&self, index: usize) -> Option<&Transformed<Node<MeshItem>, DAffine3>> {
        self.tree.get(self.node_paths.get(index)?.as_deref()?)
    }

    /// Mutable variant of [`GltfTree::node`].
    pub fn node_mut(&mut self, index: usize) -> Option<&mut Transformed<Node<MeshItem>, DAffine3>> {
        self.tree.get_mut(self.node_paths.get(index)?.as_deref()?)
    }
}

impl std::ops::Deref for GltfTree {
    type Target = Node<MeshItem>;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

/// glTF mandates a right-handed Y-up; ranim is Z-up. The conversion is a
/// fixed, spec-mandated convention translation, composed into the root
/// pose so a loaded model stands upright and no vertex data moves.
fn y_up_to_z_up() -> DAffine3 {
    DAffine3::from_rotation_x(std::f64::consts::FRAC_PI_2)
}

/// Errors from loading a glTF/GLB file via [`node_tree_from_path`].
#[derive(Debug)]
pub enum GltfLoadError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The file is not valid glTF/GLB.
    Parse(gltf::Error),
}

impl std::fmt::Display for GltfLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GltfLoadError::Io(error) => write!(f, "failed to read glTF file: {error}"),
            GltfLoadError::Parse(error) => write!(f, "failed to parse glTF file: {error}"),
        }
    }
}

impl std::error::Error for GltfLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GltfLoadError::Io(error) => Some(error),
            GltfLoadError::Parse(error) => Some(error),
        }
    }
}

/// Loads a `.glb` or `.gltf` file from disk into a [`GltfTree`].
///
/// A GLB's embedded blob is used directly. For a `.gltf`, external buffer
/// files (`"uri"` fields) are read relative to the file's directory, joined
/// as raw paths (no percent-decoding yet); `data:` URIs are not decoded and
/// their buffers are skipped with a warning — the affected primitives fall
/// back to their documented defaults.
pub fn node_tree_from_path(path: impl AsRef<Path>) -> Result<GltfTree, GltfLoadError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(GltfLoadError::Io)?;
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(GltfLoadError::Parse)?;
    let blob = gltf.blob.clone();
    let base = path.parent().unwrap_or_else(|| Path::new("."));

    // External buffer files are read up-front so the returned slices
    // outlive the resolver closure handed to [`node_tree_from_gltf`].
    let external: Vec<Option<Vec<u8>>> = gltf
        .document
        .buffers()
        .map(|buffer| match buffer.source() {
            gltf::buffer::Source::Bin => Ok(None),
            gltf::buffer::Source::Uri(uri) if uri.starts_with("data:") => {
                tracing::warn!("data: buffer URIs are not supported yet, skipping buffer {uri}");
                Ok(None)
            }
            gltf::buffer::Source::Uri(uri) => std::fs::read(base.join(uri))
                .map(Some)
                .map_err(GltfLoadError::Io),
        })
        .collect::<Result<_, _>>()?;

    Ok(node_tree_from_gltf(&gltf.document, |buffer| {
        match buffer.source() {
            gltf::buffer::Source::Bin => blob.as_deref(),
            gltf::buffer::Source::Uri(_) => external
                .get(buffer.index())
                .and_then(|data| data.as_deref()),
        }
    }))
}

/// Builds the node tree of the default glTF scene as [`Node`]s of
/// [`MeshItem`]s, wrapped in a [`GltfTree`] with the document-index →
/// tree-path mapping.
///
/// `get_buffer_data` resolves a glTF buffer to its byte slice; returning
/// `None` makes the affected accessors unreadable, so primitives fall back to
/// their documented defaults. The bound is higher-ranked over the buffer's
/// lifetime (glTF's `Primitive::reader` ties its own borrow to the call);
/// the returned slice only borrows the resolver's own storage (e.g. a GLB
/// blob), never the document. See the [module docs](self) for the full
/// mapping and the POC limitations.
pub fn node_tree_from_gltf<'s, F>(doc: &gltf::Document, get_buffer_data: F) -> GltfTree
where
    F: Clone + for<'b> Fn(gltf::buffer::Buffer<'b>) -> Option<&'s [u8]>,
{
    let mut node_paths = vec![None; doc.nodes().count()];
    let scene = match doc.default_scene().or_else(|| doc.scenes().next()) {
        Some(scene) => scene,
        None => {
            tracing::warn!("glTF document has no scene, importing an empty tree");
            return GltfTree {
                tree: Node::frame(),
                node_paths,
            };
        }
    };
    let children = scene
        .nodes()
        .enumerate()
        .map(|(slot, node)| {
            let mut wrapper =
                node_tree_from_gltf_node(node, &[slot], &mut node_paths, &get_buffer_data);
            wrapper.compose_outer(y_up_to_z_up());
            wrapper
        })
        .collect::<Vec<_>>();
    GltfTree {
        tree: Node::group(children),
        node_paths,
    }
}

/// Converts one glTF node (recursively, via [`gltf::scene::Node::children`]).
///
/// `path` is the index path of the node being converted; every visited
/// node's document index (see [`gltf::scene::Node::index`]) is recorded in
/// `node_paths`.
fn node_tree_from_gltf_node<'s, F>(
    node: gltf::scene::Node<'_>,
    path: &[usize],
    node_paths: &mut Vec<Option<Vec<usize>>>,
    get_buffer_data: &F,
) -> Transformed<Node<MeshItem>, DAffine3>
where
    F: Clone + for<'b> Fn(gltf::buffer::Buffer<'b>) -> Option<&'s [u8]>,
{
    let transform = node_transform(node.transform());
    let mut id = non_empty(node.name()).map(str::to_string);

    let mut children: Vec<Transformed<Node<MeshItem>, DAffine3>> = Vec::new();
    let mut item: Option<MeshItem> = None;
    if let Some(mesh) = node.mesh() {
        let primitive_count = mesh.primitives().count();
        if primitive_count == 1 {
            let (index, primitive) = mesh.primitives().enumerate().next().unwrap();
            // The common case maps natively: the node carries its single
            // primitive as the payload, exactly like the glTF node carries
            // its mesh. A node without a name of its own inherits the
            // mesh's.
            warn_if_not_triangles(&primitive, mesh.name(), index);
            item = Some(primitive_mesh_item(primitive, get_buffer_data));
            if id.is_none() {
                id = non_empty(mesh.name()).map(str::to_string);
            }
        } else {
            // Known debt (narrowed to multi-primitive meshes): the
            // primitives become sibling leaves placed before the children.
            for (index, primitive) in mesh.primitives().enumerate() {
                warn_if_not_triangles(&primitive, mesh.name(), index);
                let leaf = Node::leaf(primitive_mesh_item(primitive, get_buffer_data));
                children.push(
                    match primitive_leaf_id(mesh.name(), node.name(), index) {
                        Some(id) => leaf.with_id(id),
                        None => leaf,
                    }
                    .into(),
                );
            }
        }
    }
    let payload_slots = children.len();
    children.extend(node.children().enumerate().map(|(slot, child)| {
        let mut child_path = path.to_vec();
        child_path.push(payload_slots + slot);
        node_tree_from_gltf_node(child, &child_path, node_paths, get_buffer_data)
    }));

    node_paths[node.index()] = Some(path.to_vec());
    let node = Node::new(item, children);
    match id {
        Some(id) => Transformed::new(node.with_id(id), transform),
        None => Transformed::new(node, transform),
    }
}

/// Warns when a primitive's drawing mode is not `TRIANGLES` (its indices
/// are imported as-is; no re-triangulation happens in the POC).
fn warn_if_not_triangles(
    primitive: &gltf::mesh::Primitive<'_>,
    mesh_name: Option<&str>,
    index: usize,
) {
    if primitive.mode() != gltf::mesh::Mode::Triangles {
        tracing::warn!(
            "primitive {index} of mesh {mesh_name:?} is not TRIANGLES, importing indices as-is"
        );
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

    /// The raw 132-byte buffer payload: positions, normals, colors, indices.
    fn triangle_bin() -> Vec<u8> {
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
        bin
    }

    /// The document for the triangle above; `uri` switches the buffer from
    /// GLB-embedded (`None`) to an external file reference (`Some`).
    fn triangle_json(uri: Option<&str>) -> String {
        let buffers = match uri {
            Some(uri) => format!(r#"{{"uri": "{uri}", "byteLength": 132}}"#),
            None => r#"{"byteLength": 132}"#.to_string(),
        };
        r#"
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
            "buffers": [[BUFFERS]],
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
        }"#
        .replace("[BUFFERS]", &buffers)
    }

    /// Packs a JSON document and buffer payload into a GLB container.
    fn pack_glb(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut json_chunk = json.as_bytes().to_vec();
        while !json_chunk.len().is_multiple_of(4) {
            json_chunk.push(b' ');
        }
        let mut bin_chunk = bin.to_vec();
        while !bin_chunk.len().is_multiple_of(4) {
            bin_chunk.push(0);
        }

        let total = 12 + 8 + json_chunk.len() + 8 + bin_chunk.len();
        let mut glb = Vec::with_capacity(total);
        glb.extend_from_slice(&0x46546C67u32.to_le_bytes()); // "glTF"
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
        glb.extend_from_slice(&json_chunk);
        glb.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
        glb.extend_from_slice(&bin_chunk);
        glb
    }

    /// A minimal GLB: scene → "parent" (T(1,2,3)·Rz(90°)) → "child" (scale 2)
    /// carrying one triangle mesh with POSITION, NORMAL, COLOR_0, indices.
    fn triangle_glb() -> Vec<u8> {
        pack_glb(&triangle_json(None), &triangle_bin())
    }

    fn triangle_tree() -> GltfTree {
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
        let roots = tree.children();
        assert_eq!(roots.len(), 1);

        let parent = &roots[0];
        assert_eq!(parent.inner.id.as_deref(), Some("parent"));
        assert!(parent.inner.is_group());
        let parent_children = parent.inner.children();
        assert_eq!(parent_children.len(), 1);

        let child = &parent_children[0];
        // A single-primitive mesh maps natively to the node's payload.
        assert_eq!(child.inner.id.as_deref(), Some("child"));
        assert!(child.inner.is_leaf());
        assert_eq!(child.inner.item().unwrap().triangle_indices, vec![0, 1, 2]);
    }

    #[test]
    fn node_transforms_match_trs_semantics() {
        let tree = triangle_tree();
        let parent = &tree.children()[0];
        let child = &parent.inner.children()[0];

        // Parent: the scene-root wrapper's pose is the loader's Y-up → Z-up
        // flip composed outside the node's own T(1,2,3)·Rz(+90deg), so ZERO
        // lands at flip(1,2,3) = (1,-3,4) and the node's X axis maps
        // X -> Y -> Z. Rotation-derived values carry ~1e-7 f32 noise from
        // the glTF source, so their tolerance is 1e-6 (translations/scales
        // stay exact).
        let parent_t = &parent.transform;
        assert!(
            parent_t
                .transform_point3(DVec3::ZERO)
                .abs_diff_eq(dvec3(1.0, -3.0, 2.0), 1e-6)
        );
        assert!((parent_t.matrix3 * DVec3::X).abs_diff_eq(DVec3::Z, 1e-6));

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
        let leaf = tree.children()[0].inner.children()[0].inner.item().unwrap();

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

        // leaves() yields the full f64 chain:
        // flip(Y-up->Z-up) * parent(T·R) * child(S2).
        // (1,0,0) -> S2 (2,0,0) -> Rz90 (0,2,0) -> T (1,4,3) -> flip (1,-3,4).
        // f32 source noise puts the 1e-6 tolerance on rotation contributions.
        let (world, _) = tree.leaves().next().unwrap();
        assert!(
            world
                .transform_point3(dvec3(1.0, 0.0, 0.0))
                .abs_diff_eq(dvec3(1.0, -3.0, 4.0), 1e-6)
        );

        // Extraction bakes the same chain into the core item's transform.
        let extracted = tree.extract();
        assert_eq!(extracted.len(), 1);
        match &extracted[0] {
            CoreItem::MeshItem(mesh) => {
                let local = Vec3::new(1.0, 0.0, 0.0);
                let world = mesh.transform.transform_point3(local);
                assert!((world - Vec3::new(1.0, -3.0, 4.0)).length() < 1e-4);
                // Vertex data stays local.
                assert_eq!(mesh.points.len(), 3);
            }
            _ => panic!("expected a MeshItem"),
        }
    }

    #[test]
    fn loads_a_glb_file_from_disk() {
        let path = std::env::temp_dir().join(format!("ranim_gltf_test_{}.glb", std::process::id()));
        std::fs::write(&path, triangle_glb()).unwrap();
        let loaded = node_tree_from_path(&path);
        let _ = std::fs::remove_file(&path);

        let tree = loaded.unwrap();
        assert_eq!(tree.leaves().count(), 1);
        let leaf = tree.children()[0].inner.children()[0].inner.item().unwrap();
        assert_eq!(leaf.triangle_indices, vec![0, 1, 2]);
    }

    #[test]
    fn resolves_external_bin_buffers_next_to_a_gltf() {
        let dir = std::env::temp_dir();
        let stem = format!("ranim_gltf_ext_{}", std::process::id());
        let gltf_path = dir.join(format!("{stem}.gltf"));
        let bin_path = dir.join(format!("{stem}.bin"));
        std::fs::write(&gltf_path, triangle_json(Some(&format!("{stem}.bin")))).unwrap();
        std::fs::write(&bin_path, triangle_bin()).unwrap();
        let loaded = node_tree_from_path(&gltf_path);
        let _ = std::fs::remove_file(&gltf_path);
        let _ = std::fs::remove_file(&bin_path);

        // The vertex data round-trips, proving the external buffer was
        // resolved relative to the .gltf and read.
        let tree = loaded.unwrap();
        let leaf = tree.children()[0].inner.children()[0].inner.item().unwrap();
        assert_eq!(leaf.triangle_indices, vec![0, 1, 2]);
        assert_eq!(leaf.points.len(), 3);
    }

    #[test]
    fn node_paths_map_document_indices_to_tree_positions() {
        let tree = triangle_tree();

        // Doc node 0 ("parent") is the scene's only root; doc node 1
        // ("child") hangs below it. The parent carries no mesh, so the
        // child sits at slot 0.
        let parent = tree.node(0).unwrap();
        assert_eq!(parent.inner.id.as_deref(), Some("parent"));
        let child = tree.node(1).unwrap();
        assert_eq!(child.inner.id.as_deref(), Some("child"));
        assert_eq!(tree.node_paths[0], Some(vec![0]));
        assert_eq!(tree.node_paths[1], Some(vec![0, 0]));

        // Out-of-range indices resolve to None; node() agrees with get().
        assert!(tree.node(7).is_none());
        assert_eq!(
            tree.node(1).unwrap().inner.id,
            tree.get(&tree.node_paths[1].clone().unwrap())
                .unwrap()
                .inner
                .id
        );

        // Mutable access through the same map.
        let mut tree = triangle_tree();
        tree.node_mut(1).unwrap().inner.id = Some("renamed".into());
        assert_eq!(tree.node(1).unwrap().inner.id.as_deref(), Some("renamed"));
    }
}
