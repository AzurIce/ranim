//! Hierarchical scene-graph composition for items.
//!
//! This module lifts
//! [`Transformed`](ranim_core::core_item::transformed::Transformed)'s
//! "canonical local data + external transform" model from a flat wrapper to
//! a tree. Scene-graph composition lives in the items layer, built on top of
//! ranim-core's transform-group algebra
//! ([`TransformGroup`](ranim_core::traits::TransformGroup)): each node
//! stores canonical local data (like
//! [`Transformed::inner`](ranim_core::core_item::transformed::Transformed::inner)
//! does) plus one transform from its local space into the parent's space.
//!
//! The structural rules of the model:
//!
//! - **Extraction flattens depth-first** and composes matrices per node,
//!   `acc = acc * node_transform`, so the resulting
//!   [`CoreItem`](ranim_core::core_item::CoreItem) sequence preserves
//!   painter's-algorithm draw order (front-to-back document order).
//! - Every node's pose interpolates **independently** of its geometry:
//!   lerping moves transforms and leaf payloads only and never bakes points
//!   into vertices — the basis for future skeletal animation.
//!
//! # Examples
//!
//! ```rust
//! use ranim_core::core_item::vitem::VItem as CoreVItem;
//! use ranim_core::glam::dvec3;
//! use ranim_core::traits::{Extract, ShiftTransform};
//! use ranim_items::hierarchy::Node;
//!
//! let stroke = CoreVItem {
//!     points: vec![glam::Vec4::new(1.0, 0.0, 0.0, 0.0)],
//!     ..Default::default()
//! };
//! let mut tree = Node::<CoreVItem>::group(vec![
//!     Node::leaf(stroke.clone()).with_id("a"),
//!     Node::leaf(stroke).with_id("b"),
//! ]);
//! // Root posing is O(1) and never bakes points.
//! tree.shift(dvec3(1.0, 0.0, 0.0));
//!
//! let extracted = tree.extract();
//! assert_eq!(extracted.len(), 2);
//! ```

use std::fmt;
use std::ops::Range;

use ranim_core::components::width::Width;
use ranim_core::core_item::transformed::Transformed;
use ranim_core::{
    Extract,
    anchor::{Aabb, Centroid, Locate},
    color::{AlphaColor, Srgb, palette::css},
    core_item::CoreItem,
    glam::{DAffine3, DVec3, dvec3},
    traits::{
        Alignable, ApplyTransform, Empty, FillColor, Interpolatable, Opacity, Partial, StrokeColor,
        StrokeWidth, TransformGroup,
    },
    utils::resize_preserving_order_with_repeated_indices,
};
use tracing::warn;

/// One node of a scene-graph tree: an external id, a local-to-parent
/// transform, and either leaf content or children.
///
/// # Examples
///
/// Build trees with [`Node::leaf`], [`Node::group`], and the builders, or
/// with struct literals when every field is known:
///
/// ```
/// use ranim_core::glam::dvec3;
/// use ranim_core::traits::Translation;
/// use ranim_items::hierarchy::{Node, NodeContent};
///
/// let node = Node {
///     id: Some("outer".into()),
///     transform: Translation(dvec3(1.0, 0.0, 0.0)),
///     content: NodeContent::Leaf("geometry".to_string()),
/// };
/// assert_eq!(node.leaf_content(), Some(&"geometry".to_string()));
/// ```
pub struct Node<I, G = DAffine3> {
    /// External identifier carried from the source format (e.g. SVG element
    /// id, glTF node name). Ignored by rendering/extraction beyond being
    /// transported: alignment preserves it, and lerping switches ids at the
    /// mid-point exactly like other front-loaded fields in ranim.
    pub id: Option<String>,
    /// The transform from this node's local space into the parent's space.
    pub transform: G,
    /// Either leaf content or ordered children.
    pub content: NodeContent<I, G>,
}

/// The payload variants of a [`Node`].
///
/// Children are stored in source order; extraction preserves that order
/// depth-first so downstream consumers see painter's-algorithm ordering.
pub enum NodeContent<I, G = DAffine3> {
    /// Leaf content in canonical local coordinates.
    Leaf(I),
    /// Ordered child nodes.
    Children(Vec<Node<I, G>>),
}

// MARK: Manual basic impls
//
// Derived impls would place bounds on the defaulted type parameter `G` even
// where the fields do not require them; writing these by hand keeps the
// bounds to exactly what the fields demand.

impl<I: Clone, G: Clone> Clone for Node<I, G> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            transform: self.transform.clone(),
            content: self.content.clone(),
        }
    }
}

impl<I: PartialEq, G: PartialEq> PartialEq for Node<I, G> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.transform == other.transform && self.content == other.content
    }
}

impl<I: Eq, G: Eq> Eq for Node<I, G> {}

impl<I: fmt::Debug, G: fmt::Debug> fmt::Debug for Node<I, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("transform", &self.transform)
            .field("content", &self.content)
            .finish()
    }
}

impl<I: Clone, G: Clone> Clone for NodeContent<I, G> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(item) => Self::Leaf(item.clone()),
            Self::Children(children) => Self::Children(children.clone()),
        }
    }
}

impl<I: PartialEq, G: PartialEq> PartialEq for NodeContent<I, G> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Leaf(a), Self::Leaf(b)) => a == b,
            (Self::Children(a), Self::Children(b)) => a == b,
            _ => false,
        }
    }
}

impl<I: Eq, G: Eq> Eq for NodeContent<I, G> {}

impl<I: fmt::Debug, G: fmt::Debug> fmt::Debug for NodeContent<I, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Leaf(item) => f.debug_tuple("Leaf").field(item).finish(),
            Self::Children(children) => f.debug_tuple("Children").field(children).finish(),
        }
    }
}

// MARK: Inherent API

impl<I, G> Node<I, G> {
    /// Pair `content` with `transform`, without any external id.
    pub fn new(transform: G, content: NodeContent<I, G>) -> Self {
        Self {
            id: None,
            transform,
            content,
        }
    }

    /// Attach an external id, consuming and returning the node.
    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Replace the local-to-parent transform, consuming and returning the
    /// node.
    #[must_use]
    pub fn with_transform(mut self, transform: G) -> Self {
        self.transform = transform;
        self
    }

    /// Whether this node holds leaf content.
    pub fn is_leaf(&self) -> bool {
        matches!(self.content, NodeContent::Leaf(_))
    }

    /// Whether this node holds children.
    pub fn is_group(&self) -> bool {
        matches!(self.content, NodeContent::Children(_))
    }

    /// The leaf content, if this node is a leaf.
    ///
    /// (Named `leaf_content` rather than `leaf` because [`Node::leaf`] is
    /// the leaf constructor.)
    pub fn leaf_content(&self) -> Option<&I> {
        match &self.content {
            NodeContent::Leaf(item) => Some(item),
            NodeContent::Children(_) => None,
        }
    }

    /// The leaf content mutably, if this node is a leaf.
    pub fn leaf_content_mut(&mut self) -> Option<&mut I> {
        match &mut self.content {
            NodeContent::Leaf(item) => Some(item),
            NodeContent::Children(_) => None,
        }
    }

    /// The ordered children, if this node is a group.
    pub fn children(&self) -> Option<&[Node<I, G>]> {
        match &self.content {
            NodeContent::Leaf(_) => None,
            NodeContent::Children(children) => Some(children),
        }
    }

    /// The ordered children mutably, if this node is a group.
    pub fn children_mut(&mut self) -> Option<&mut [Node<I, G>]> {
        match &mut self.content {
            NodeContent::Leaf(_) => None,
            NodeContent::Children(children) => Some(children),
        }
    }

    /// The first leaf payload in depth-first order, without composing any
    /// transforms. This backs the color/stroke-width getters; `None` means
    /// the tree has no leaves at all.
    pub fn first_leaf(&self) -> Option<&I> {
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            match &node.content {
                NodeContent::Leaf(item) => return Some(item),
                NodeContent::Children(children) => {
                    stack.extend(children.iter().rev());
                }
            }
        }
        None
    }

    /// Look up a descendant by walking child indices: an empty path returns
    /// `self`, `[i]` returns child `i`, `[i, j]` returns child `j` of child
    /// `i`, and so on. Returns `None` when an index is out of bounds or the
    /// path tries to descend into a leaf.
    pub fn get(&self, path: &[usize]) -> Option<&Node<I, G>> {
        let mut node = self;
        for &index in path {
            node = node.children()?.get(index)?;
        }
        Some(node)
    }

    /// Mutable variant of [`Node::get`].
    pub fn get_mut(&mut self, path: &[usize]) -> Option<&mut Node<I, G>> {
        let mut node = self;
        for &index in path {
            node = node.children_mut()?.get_mut(index)?;
        }
        Some(node)
    }

    /// Iterate over flattened leaves with their accumulated world affine,
    /// yielding `(world_affine, &item)` pairs in depth-first order — i.e.
    /// painter's-algorithm draw order.
    ///
    /// The world affine composes top-down, `acc = acc * node_transform`,
    /// starting from the root node's own transform, so the yielded affine
    /// includes every node on the path down to and including the leaf —
    /// the same placement [`Extract`] bakes into core items. Implemented
    /// with an explicit stack, so deep trees cannot overflow the call stack.
    pub fn leaves(&self) -> Leaves<'_, I, G>
    where
        G: Clone + Into<DAffine3>,
    {
        Leaves {
            stack: vec![(self.transform.clone().into(), self)],
        }
    }

    /// Iterate over mutable references to all leaf payloads in depth-first
    /// order. Unlike [`Node::leaves`], no transforms are composed: callers
    /// mutate canonical local data only.
    pub fn leaves_mut(&mut self) -> LeavesMut<'_, I, G> {
        LeavesMut { stack: vec![self] }
    }

    /// Map every leaf payload to a new type, keeping ids, transforms, and
    /// the tree shape unchanged. This is the recursive analog of
    /// [`Transformed::map_inner`].
    pub fn map_inner<U>(self, f: impl FnMut(I) -> U) -> Node<U, G> {
        fn map_inner_rec<I, U, G>(node: Node<I, G>, f: &mut impl FnMut(I) -> U) -> Node<U, G> {
            let Node {
                id,
                transform,
                content,
            } = node;
            let content = match content {
                NodeContent::Leaf(item) => NodeContent::Leaf(f(item)),
                NodeContent::Children(children) => NodeContent::Children(
                    children
                        .into_iter()
                        .map(|child| map_inner_rec(child, f))
                        .collect(),
                ),
            };
            Node {
                id,
                transform,
                content,
            }
        }
        let mut f = f;
        map_inner_rec(self, &mut f)
    }

    /// Map the transform storage of every node to a new type while keeping
    /// everything else unchanged. This mirrors [`Transformed::map_transform`]
    /// and is the general form of converting between transform groups —
    /// including widening, which intentionally has no blanket `From` impl on
    /// `Node`.
    pub fn map_transform<H>(self, f: impl FnMut(G) -> H) -> Node<I, H> {
        fn map_transform_rec<I, G, H>(node: Node<I, G>, f: &mut impl FnMut(G) -> H) -> Node<I, H> {
            let Node {
                id,
                transform,
                content,
            } = node;
            let content = match content {
                NodeContent::Leaf(item) => NodeContent::Leaf(item),
                NodeContent::Children(children) => NodeContent::Children(
                    children
                        .into_iter()
                        .map(|child| map_transform_rec(child, f))
                        .collect(),
                ),
            };
            Node {
                id,
                transform: f(transform),
                content,
            }
        }
        let mut f = f;
        map_transform_rec(self, &mut f)
    }
}

impl<I, G: TransformGroup> Node<I, G> {
    /// Create a leaf node with the identity transform and no id.
    pub fn leaf(item: I) -> Self {
        Self::new(G::identity(), NodeContent::Leaf(item))
    }

    /// Create a group node with the identity transform and no id.
    pub fn group(children: Vec<Node<I, G>>) -> Self {
        Self::new(G::identity(), NodeContent::Children(children))
    }
}

impl<I, G> Node<I, G>
where
    G: TransformGroup + Clone,
{
    /// Collapse a single-leaf tree back into a [`Transformed`] wrapper.
    ///
    /// Walks down chains of single-child groups composing their transforms
    /// in `G` along the way (`acc.compose(child)`), so the storage type is
    /// preserved exactly. Returns `None` when some group branches, because
    /// such a tree holds more than one leaf.
    pub fn into_transformed(self) -> Option<Transformed<I, G>> {
        let Node {
            transform, content, ..
        } = self;
        collapse_into_transformed(transform, content)
    }
}

/// Fold `acc` (the composed ancestors' transform, outermost first) down
/// through a single-leaf spine into a [`Transformed`].
fn collapse_into_transformed<I, G>(acc: G, content: NodeContent<I, G>) -> Option<Transformed<I, G>>
where
    G: TransformGroup + Clone,
{
    match content {
        NodeContent::Leaf(item) => Some(Transformed::new(item, acc)),
        NodeContent::Children(mut children) => {
            if children.len() != 1 {
                return None;
            }
            let child = children.swap_remove(0);
            collapse_into_transformed(acc.compose(&child.transform), child.content)
        }
    }
}

// MARK: Iterators

/// Iterator over flattened leaves with accumulated world affines, produced by
/// [`Node::leaves`].
///
/// Yields `(world_affine, &item)` pairs in depth-first (painter's algorithm)
/// order.
pub struct Leaves<'a, I, G = DAffine3> {
    stack: Vec<(DAffine3, &'a Node<I, G>)>,
}

impl<'a, I, G> Iterator for Leaves<'a, I, G>
where
    G: Clone + Into<DAffine3>,
{
    type Item = (DAffine3, &'a I);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((acc, node)) = self.stack.pop() {
            match &node.content {
                NodeContent::Leaf(item) => return Some((acc, item)),
                NodeContent::Children(children) => {
                    // Pushed in reverse so popping yields children in source
                    // (painter's-algorithm) order.
                    for child in children.iter().rev() {
                        let acc_child = acc.compose(&child.transform.clone().into());
                        self.stack.push((acc_child, child));
                    }
                }
            }
        }
        None
    }
}

/// Iterator over mutable references to leaf payloads, produced by
/// [`Node::leaves_mut`]. Depth-first order; no transforms are composed.
pub struct LeavesMut<'a, I, G = DAffine3> {
    stack: Vec<&'a mut Node<I, G>>,
}

impl<'a, I, G> Iterator for LeavesMut<'a, I, G> {
    type Item = &'a mut I;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match &mut node.content {
                NodeContent::Leaf(item) => return Some(item),
                NodeContent::Children(children) => {
                    self.stack.extend(children.iter_mut().rev());
                }
            }
        }
        None
    }
}

// MARK: Extract

impl<I, G> Extract for Node<I, G>
where
    I: Extract<Target = CoreItem>,
    G: TransformGroup + Clone + Into<DAffine3>,
{
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        let start = buf.len();
        match &self.content {
            NodeContent::Leaf(item) => item.extract_into(buf),
            NodeContent::Children(children) => {
                for child in children {
                    child.extract_into(buf);
                }
            }
        }
        // Each level post-multiplies its own affine onto whatever its subtree
        // appended, so a chain composes as `t_root * ... * t_leaf * local`.
        // Recursing before applying also keeps emission depth-first: paint
        // order survives flattening.
        let transform = self.transform.clone().into();
        for item in &mut buf[start..] {
            item.apply_transform(&transform);
        }
    }
}

// MARK: ApplyTransform

impl<I, G, H> ApplyTransform<H> for Node<I, G>
where
    G: TransformGroup + From<H>,
{
    fn apply(&mut self, transform: H) -> &mut Self {
        // Root posing only: compose onto the ROOT node's transform, O(1).
        // Leaf points are never baked; descendants keep canonical local data.
        self.transform = G::from(transform).compose(&self.transform);
        self
    }
}

// MARK: Interpolatable

impl<I, G> Interpolatable for Node<I, G>
where
    I: Interpolatable,
    G: Interpolatable,
{
    /// Structural lerp: nodes interpolate positionally, leaf payloads and
    /// node poses interpolate independently, and ids switch at the mid-point
    /// like other front-loaded fields in ranim. Callers must have aligned
    /// structures first (see [`Alignable`]); misaligned kinds panic, and
    /// unequal sibling counts follow `Vec`'s truncating-zip precedent.
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            id: if t < 0.5 {
                self.id.clone()
            } else {
                target.id.clone()
            },
            transform: self.transform.lerp(&target.transform, t),
            content: lerp_contents(&self.content, &target.content, t),
        }
    }
}

/// Structural lerp over contents.
fn lerp_contents<I, G>(
    current: &NodeContent<I, G>,
    target: &NodeContent<I, G>,
    t: f64,
) -> NodeContent<I, G>
where
    I: Interpolatable,
    G: Interpolatable,
{
    match (current, target) {
        (NodeContent::Leaf(a), NodeContent::Leaf(b)) => NodeContent::Leaf(a.lerp(b, t)),
        (NodeContent::Children(a), NodeContent::Children(b)) => {
            NodeContent::Children(a.iter().zip(b).map(|(a, b)| a.lerp(b, t)).collect())
        }
        _ => panic!("interpolating unaligned hierarchies: align them with Alignable first"),
    }
}

// MARK: Alignable

impl<I, G> Alignable for Node<I, G>
where
    I: Alignable + Opacity,
    G: TransformGroup + Clone + Into<DAffine3>,
{
    /// Whether both sides are already structurally compatible for direct
    /// interpolation: kinds must match positionally (leaf with leaf, group
    /// with group), sibling counts must be equal, and every leaf pair must
    /// satisfy [`Alignable::is_aligned`]. This mirrors the `Vec<T>`
    /// blanket's pre-alignment contract; [`Alignable::align_with`]
    /// establishes this state from mismatched trees.
    fn is_aligned(&self, other: &Self) -> bool {
        nodes_are_aligned(self, other)
    }

    /// Align two trees for interpolation:
    ///
    /// 1. **Pairwise recursion** whenever kinds already match.
    /// 2. **Lift on kind mismatch**: at the same position, whichever side is
    ///    a leaf is lifted into a single-child group with an identity
    ///    transform, symmetrically, so kinds match.
    /// 3. **Pad on count mismatch**: in matched groups with unequal child
    ///    counts, both sides are resized with
    ///    `resize_preserving_order_with_repeated_indices`; transparently
    ///    repeated stand-ins get `set_opacity(0.0)` before recursing
    ///    pairwise — exactly how the `Vec<T>: Alignable` blanket treats
    ///    repeated items.
    ///
    /// Policies 2 and 3 compose uniformly: a leaf opposite a multi-child
    /// group is lifted first and then padded, so the extra positions pair
    /// against transparent repetitions of the leaf.
    fn align_with(&mut self, other: &mut Self) {
        if !same_kind(&self.content, &other.content) {
            if self.is_leaf() {
                self.lift_leaf_to_group();
            } else {
                other.lift_leaf_to_group();
            }
        }
        match (&mut self.content, &mut other.content) {
            (NodeContent::Leaf(current), NodeContent::Leaf(target)) => current.align_with(target),
            (NodeContent::Children(a), NodeContent::Children(b)) => {
                let len = a.len().max(b.len());
                expand_with_transparent_repeats(a, len);
                expand_with_transparent_repeats(b, len);
                a.iter_mut()
                    .zip(b.iter_mut())
                    .for_each(|(a, b)| a.align_with(b));
            }
            _ => unreachable!("kinds were unified above"),
        }
    }
}

impl<I, G> Node<I, G>
where
    G: TransformGroup,
{
    /// Convert a leaf in place into a single-child group whose child keeps
    /// the leaf content under the identity transform. A temporary empty
    /// group parks in `content` during the swap, so no bounds on `I` are
    /// needed to move the payload out.
    fn lift_leaf_to_group(&mut self) {
        if let NodeContent::Leaf(_) = self.content {
            let content = std::mem::replace(&mut self.content, NodeContent::Children(Vec::new()));
            let NodeContent::Leaf(item) = content else {
                unreachable!("the content was a leaf")
            };
            self.content = NodeContent::Children(vec![Node::leaf(item)]);
        }
    }
}

/// Expand `nodes` in place to `len` entries, preserving order; repeated
/// stand-ins become fully transparent via `set_opacity(0.0)`, matching the
/// `Vec<T>` align blanket in ranim-core.
fn expand_with_transparent_repeats<I, G>(nodes: &mut Vec<Node<I, G>>, len: usize)
where
    I: Opacity,
    Node<I, G>: Clone,
{
    if nodes.len() != len {
        let (mut expanded, repeated_idxs) =
            resize_preserving_order_with_repeated_indices(nodes, len);
        for idx in repeated_idxs {
            expanded[idx].set_opacity(0.0);
        }
        *nodes = expanded;
    }
}

fn same_kind<I, G>(current: &NodeContent<I, G>, target: &NodeContent<I, G>) -> bool {
    matches!(
        (current, target),
        (NodeContent::Leaf(_), NodeContent::Leaf(_))
            | (NodeContent::Children(_), NodeContent::Children(_))
    )
}

/// Structural alignment check mirroring [`Alignable::is_aligned`]: kind and
/// sibling-count equality at every position, with leaf payloads compared by
/// their own alignment.
fn nodes_are_aligned<I, G>(current: &Node<I, G>, target: &Node<I, G>) -> bool
where
    I: Alignable,
{
    match (&current.content, &target.content) {
        (NodeContent::Leaf(a), NodeContent::Leaf(b)) => a.is_aligned(b),
        (NodeContent::Children(a), NodeContent::Children(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| nodes_are_aligned(a, b))
        }
        _ => false,
    }
}

// MARK: Partial

impl<I, G> Partial for Node<I, G>
where
    I: Partial,
    G: Clone,
{
    fn get_partial(&self, range: Range<f64>) -> Self {
        Self {
            id: self.id.clone(),
            transform: self.transform.clone(),
            content: partial_content(&self.content, range, false),
        }
    }

    fn get_partial_closed(&self, range: Range<f64>) -> Self {
        Self {
            id: self.id.clone(),
            transform: self.transform.clone(),
            content: partial_content(&self.content, range, true),
        }
    }
}

/// Forward the same range down every branch, keeping poses fixed: partial
/// display shows sub-geometry but never moves nodes.
fn partial_content<I, G>(
    content: &NodeContent<I, G>,
    range: Range<f64>,
    closed: bool,
) -> NodeContent<I, G>
where
    I: Partial,
    G: Clone,
{
    match content {
        NodeContent::Leaf(item) => NodeContent::Leaf(if closed {
            item.get_partial_closed(range)
        } else {
            item.get_partial(range)
        }),
        NodeContent::Children(children) => NodeContent::Children(
            children
                .iter()
                .map(|child| Node {
                    id: child.id.clone(),
                    transform: child.transform.clone(),
                    content: partial_content(&child.content, range.clone(), closed),
                })
                .collect(),
        ),
    }
}

// MARK: Empty

impl<I, G> Empty for Node<I, G>
where
    I: Empty,
    G: TransformGroup,
{
    fn empty() -> Self {
        Node {
            id: None,
            transform: G::identity(),
            content: NodeContent::Leaf(I::empty()),
        }
    }
}

// MARK: Aabb

impl<I, G> Aabb for Node<I, G>
where
    I: Aabb,
    G: Clone + Into<DAffine3>,
{
    /// Union of descendant AABBs, transforming each child's box corners by
    /// this node's affine before folding, then applying this node's own
    /// transform last — the recursive generalization of `Transformed::aabb`'s
    /// 8-corner loop. An empty group warns and reports a degenerate box,
    /// mirroring the slice impl in ranim-core.
    fn aabb(&self) -> [DVec3; 2] {
        let inner_box = match &self.content {
            NodeContent::Leaf(item) => item.aabb(),
            NodeContent::Children(children) => children
                .iter()
                .map(Node::aabb)
                .reduce(|[acc_lo, acc_hi], [lo, hi]| [acc_lo.min(lo), acc_hi.max(hi)])
                .unwrap_or_else(|| {
                    warn!("Empty bounding box, is the tree empty?");
                    [DVec3::ZERO, DVec3::ZERO]
                }),
        };
        transformed_aabb(inner_box, self.transform.clone().into())
    }
}

/// Apply `affine` to the eight corners of a box and re-tighten the result,
/// replicating `Transformed::aabb`'s corner loop.
fn transformed_aabb([min, max]: [DVec3; 2], affine: DAffine3) -> [DVec3; 2] {
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for i in 0..8 {
        let corner = dvec3(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        );
        let point = affine.transform_point3(corner);
        lo = lo.min(point);
        hi = hi.max(point);
    }
    [lo, hi]
}

// MARK: Locate

/// The centroid of a tree weights every flattened leaf equally: sum each
/// leaf's centroid mapped through its accumulated world affine and divide by
/// the leaf count — NOT per-child weighting. An empty tree warns and returns
/// zero instead of producing NaNs.
impl<I, G> Locate<Node<I, G>> for Centroid
where
    Centroid: Locate<I>,
    G: Clone + Into<DAffine3>,
{
    fn locate(&self, target: &Node<I, G>) -> DVec3 {
        let mut sum = DVec3::ZERO;
        let mut count = 0usize;
        for (affine, leaf) in target.leaves() {
            sum += affine.transform_point3(self.locate(leaf));
            count += 1;
        }
        if count == 0 {
            warn!("Locating the centroid of an empty tree, returning zero");
            return DVec3::ZERO;
        }
        sum / count as f64
    }
}

// MARK: Opacity

impl<I, G> Opacity for Node<I, G>
where
    I: Opacity,
{
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        for leaf in self.leaves_mut() {
            leaf.set_opacity(opacity);
        }
        self
    }
}

// MARK: FillColor

impl<I, G> FillColor for Node<I, G>
where
    I: FillColor,
{
    /// The fill color of the first leaf in DFS order; an empty tree warns
    /// and reports white.
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.first_leaf()
            .map(FillColor::fill_color)
            .unwrap_or_else(|| {
                warn!("Accessing the fill color of an empty tree, returning white");
                css::WHITE
            })
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        for leaf in self.leaves_mut() {
            leaf.set_fill_color(color);
        }
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        for leaf in self.leaves_mut() {
            leaf.set_fill_opacity(opacity);
        }
        self
    }
}

// MARK: StrokeColor

impl<I, G> StrokeColor for Node<I, G>
where
    I: StrokeColor,
{
    /// The stroke color of the first leaf in DFS order; an empty tree warns
    /// and reports white.
    fn stroke_color(&self) -> AlphaColor<Srgb> {
        self.first_leaf()
            .map(StrokeColor::stroke_color)
            .unwrap_or_else(|| {
                warn!("Accessing the stroke color of an empty tree, returning white");
                css::WHITE
            })
    }

    fn set_stroke_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        for leaf in self.leaves_mut() {
            leaf.set_stroke_color(color);
        }
        self
    }

    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        for leaf in self.leaves_mut() {
            leaf.set_stroke_opacity(opacity);
        }
        self
    }
}

// MARK: StrokeWidth

impl<I, G> StrokeWidth for Node<I, G>
where
    I: StrokeWidth,
{
    /// The stroke width of the first leaf in DFS order; an empty tree warns
    /// and reports `0.0`.
    fn stroke_width(&self) -> f32 {
        self.first_leaf()
            .map(StrokeWidth::stroke_width)
            .unwrap_or_else(|| {
                warn!("Accessing the stroke width of an empty tree, returning 0");
                0.0
            })
    }

    /// Forward the stroke-width function to every leaf independently.
    fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [Width])) -> &mut Self {
        for leaf in self.leaves_mut() {
            leaf.apply_stroke_func(&f);
        }
        self
    }
}

// MARK: Conversions

/// Lift a wrapper into a leaf node, keeping the transform external.
///
/// Deliberately no widening cross-parameter `From` impls exist beyond this
/// and [`Node::into_transformed`] (to avoid future coherence conflicts);
/// widening is expressible through [`Node::map_transform`] instead.
impl<I, G> From<Transformed<I, G>> for Node<I, G> {
    fn from(value: Transformed<I, G>) -> Self {
        Node {
            id: None,
            transform: value.transform,
            content: NodeContent::Leaf(value.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ranim_core::core_item::vitem::VItem as CoreVItem;
    use ranim_core::glam::{DQuat, Mat4, Quat, Vec3, Vec4, dvec3};
    use ranim_core::traits::{
        Diag, Rigid, RotateTransform, ShiftTransform, Similarity, Translation,
        UniformScaleTransform,
    };

    type CoreNode<G = DAffine3> = Node<CoreVItem, G>;
    type HierarchyVItem = crate::vitem::VItem;

    /// A core VItem whose single anchor carries a marker on the x axis, so
    /// tests can identify which leaf was emitted.
    fn marked_core_vitem(marker: f32) -> CoreVItem {
        CoreVItem {
            points: vec![Vec4::new(marker, 0.0, 0.0, 0.0)],
            ..Default::default()
        }
    }

    /// A high-level VItem with opaque strokes, used for opacity inspection.
    fn stroked_vitem(marker: f64) -> HierarchyVItem {
        let mut vitem = HierarchyVItem::from_vpoints(vec![
            dvec3(marker, 0.0, 0.0),
            dvec3(marker + 0.5, 0.0, 0.0),
            dvec3(marker + 1.0, 0.0, 0.0),
        ]);
        vitem.set_stroke_width(0.04);
        vitem
    }

    fn assert_affine_eq(actual: DAffine3, expected: DAffine3) {
        assert!(
            actual
                .transform_point3(dvec3(0.3, -0.7, 1.1))
                .abs_diff_eq(expected.transform_point3(dvec3(0.3, -0.7, 1.1)), 1e-9)
        );
        for i in 0..3 {
            assert!(
                actual
                    .matrix3
                    .col(i)
                    .abs_diff_eq(expected.matrix3.col(i), 1e-9),
                "matrix3 column {i} diverges"
            );
        }
        assert!(actual.translation.abs_diff_eq(expected.translation, 1e-9));
    }

    #[test]
    fn nested_extract_composes_matrices_and_keeps_points_local() {
        // Leaf -> Translation((3,4,5)) -> Similarity(scale 2, t (10,20,30)).
        // World = S * T: translation becomes (16,28,40).
        let inner_similarity = Similarity {
            scale: 2.0,
            rotation: DQuat::IDENTITY,
            translation: dvec3(10.0, 20.0, 30.0),
        };
        let tree = CoreNode::new(
            DAffine3::from(inner_similarity),
            NodeContent::Children(vec![CoreNode::new(
                DAffine3::from(Translation(dvec3(3.0, 4.0, 5.0))),
                NodeContent::Leaf(CoreVItem {
                    points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
                    ..Default::default()
                }),
            )]),
        );

        let expected_world =
            DAffine3::from(inner_similarity) * DAffine3::from(Translation(dvec3(3.0, 4.0, 5.0)));
        assert_affine_eq(tree.leaves().next().unwrap().0, expected_world);

        match &tree.extract()[0] {
            CoreItem::VItem(extracted) => {
                // Local data stays byte-identical; the world placement moves.
                assert_eq!(extracted.points[0], Vec4::new(1.0, 0.0, 0.0, 0.0));
                assert_eq!(
                    extracted.transform,
                    Mat4::from_scale_rotation_translation(
                        Vec3::splat(2.0),
                        Quat::IDENTITY,
                        Vec3::new(16.0, 28.0, 40.0),
                    )
                );
            }
            _ => panic!("expected a VItem"),
        }
    }

    #[test]
    fn dfs_emission_matches_painters_order_on_three_levels() {
        //        root
        //        /   \
        //      g1    leaf(2)
        //      |
        //      g2
        //      |
        //    leaf(1), leaf(3)
        let tree = CoreNode::<DAffine3>::group(vec![
            CoreNode::<DAffine3>::group(vec![CoreNode::<DAffine3>::group(vec![
                CoreNode::leaf(marked_core_vitem(1.0)),
                CoreNode::leaf(marked_core_vitem(3.0)),
            ])]),
            CoreNode::leaf(marked_core_vitem(2.0)),
        ]);

        let markers: Vec<f32> = tree
            .extract()
            .into_iter()
            .map(|item| match item {
                CoreItem::VItem(vitem) => vitem.points[0].x,
                _ => panic!("expected a VItem"),
            })
            .collect();
        // Depth-first: the deeply nested branch paints entirely first.
        assert_eq!(markers, [1.0, 3.0, 2.0]);
    }

    #[test]
    fn aabb_follows_node_rotation_and_empty_groups_degenerate() {
        // Axis-aligned box x in [-1, 3], y in [-1, 1]: anchors along a
        // rectangular loop with colinear midpoint handles.
        let rect = HierarchyVItem::from_vpoints(vec![
            dvec3(-1.0, -1.0, 0.0),
            dvec3(1.0, -1.0, 0.0),
            dvec3(3.0, -1.0, 0.0),
            dvec3(3.0, 0.0, 0.0),
            dvec3(3.0, 1.0, 0.0),
            dvec3(1.0, 1.0, 0.0),
            dvec3(-1.0, 1.0, 0.0),
            dvec3(-1.0, 0.0, 0.0),
            dvec3(-1.0, -1.0, 0.0),
        ]);
        let rotated = Node::leaf(rect).with_transform(DAffine3::from_rotation_translation(
            DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2),
            DVec3::ZERO,
        ));

        let [lo, hi] = rotated.aabb();
        // Rotating (+90deg about Z, x' = -y, y' = x) maps the box onto
        // x in [-1, 1], y in [-1, 3].
        assert!(lo.abs_diff_eq(dvec3(-1.0, -1.0, 0.0), 1e-9), "lo is {lo:?}");
        assert!(hi.abs_diff_eq(dvec3(1.0, 3.0, 0.0), 1e-9), "hi is {hi:?}");

        // An empty group degenerates to the zero box.
        let empty = Node::<HierarchyVItem>::group(Vec::new());
        assert_eq!(empty.aabb(), [DVec3::ZERO, DVec3::ZERO]);
    }

    #[test]
    fn centroid_weights_each_flattened_leaf_equally() {
        // One side has 3 leaves at x = 3, 4, 5; the other a single leaf at
        // x = -36. Equal weighting gives (3+4+5-36)/4 = -6, while per-child
        // weighting (subtree average 4 mixed half-and-half) would give -16.
        let asymmetric = Node::<DVec3>::group(vec![
            Node::group(vec![
                Node::leaf(dvec3(3.0, 0.0, 0.0)),
                Node::leaf(dvec3(4.0, 0.0, 0.0)),
                Node::leaf(dvec3(5.0, 0.0, 0.0)),
            ]),
            Node::leaf(dvec3(-36.0, 0.0, 0.0)),
        ]);

        let centroid = Centroid.locate(&asymmetric);
        assert!(
            centroid.abs_diff_eq(dvec3(-6.0, 0.0, 0.0), 1e-9),
            "centroid is {centroid:?}"
        );
        assert_ne!(centroid, dvec3(-16.0, 0.0, 0.0));

        let empty = Node::<DVec3>::group(Vec::new());
        assert_eq!(Centroid.locate(&empty), DVec3::ZERO);
    }

    #[test]
    fn align_lifts_single_leaf_opposite_group_and_lerps() {
        let left = Node::leaf(stroked_vitem(0.0));
        let right = Node::<HierarchyVItem>::group(vec![
            Node::leaf(stroked_vitem(2.0)).with_transform(DAffine3::from(Translation(DVec3::Y))),
        ]);

        assert!(!left.is_aligned(&right));
        let mut left = left;
        let mut right = right;
        left.align_with(&mut right);

        // Both sides are now single-child groups with aligned leaves.
        assert!(left.is_aligned(&right));
        assert!(right.is_group());
        assert_eq!(left.children().unwrap().len(), 1);

        // The lift and the subsequent lerp both succeed; the geometry moves
        // to the midpoint while the lifted identity pose chases the
        // translated one.
        let mid = left.lerp(&right, 0.5);
        assert_eq!(mid.children().unwrap().len(), 1);
        let (mid_world, mid_leaf) = mid.leaves().next().unwrap();
        assert!(
            (mid_leaf.vpoints[0].x - 1.0).abs() < 1e-6,
            "marker lerps from 0 toward 2"
        );
        // Halfway between the lifted identity and Translation(Y).
        assert_affine_eq(mid_world, DAffine3::from_translation(dvec3(0.0, 0.5, 0.0)));
    }

    #[test]
    fn align_pads_shorter_groups_with_transparent_stand_ins() {
        let big = Node::<HierarchyVItem>::group(vec![
            Node::leaf(stroked_vitem(0.0)),
            Node::leaf(stroked_vitem(10.0)),
        ]);
        let small = Node::<HierarchyVItem>::group(vec![Node::leaf(stroked_vitem(20.0))]);

        assert!(!big.is_aligned(&small));
        let mut big = big;
        let mut small = small;
        small.align_with(&mut big);

        assert!(big.is_aligned(&small));
        assert_eq!(small.children().unwrap().len(), 2);
        // The repeated stand-in became fully transparent while the original
        // kept its opacity.
        let stand_in = &small.children().unwrap()[1];
        let stand_in = stand_in.leaf_content().unwrap();
        assert_eq!(stand_in.stroke_rgbas[0].0.w, 0.0);
        assert_eq!(stand_in.fill_rgbas[0].0.w, 0.0);
        let original = small.children().unwrap()[0].leaf_content().unwrap();
        assert_eq!(original.stroke_rgbas[0].0.w, 1.0);
    }

    #[test]
    fn root_apply_transform_poses_without_baking_points() {
        let mut tree = CoreNode::leaf(CoreVItem {
            points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
            ..Default::default()
        })
        .with_transform(DAffine3::from(Translation(DVec3::X)));

        tree.apply(Rigid::from_translation(DVec3::Y));

        match &tree.extract()[0] {
            CoreItem::VItem(extracted) => {
                // Points are byte-identical, only the matrix updated.
                assert_eq!(extracted.points[0], Vec4::new(1.0, 0.0, 0.0, 0.0));
                assert_eq!(
                    extracted.transform,
                    Mat4::from_translation(Vec3::new(1.0, 1.0, 0.0))
                );
            }
            _ => panic!("expected a VItem"),
        }
    }

    #[test]
    fn subgroup_operations_keep_the_root_storage_type() {
        // The shift/scale blankets derive from ApplyTransform and can never
        // widen the root's storage group.
        let mut tree = Node::<(), Similarity> {
            id: None,
            transform: Similarity::IDENTITY,
            content: NodeContent::Leaf(()),
        };
        tree.shift(DVec3::X).scale_uniform(2.0);
        assert_eq!(tree.transform.scale, 2.0);
        assert_eq!(tree.transform.translation, dvec3(2.0, 0.0, 0.0));

        let mut rigid_tree = Node::<(), Rigid> {
            id: None,
            transform: Rigid::IDENTITY,
            content: NodeContent::Leaf(()),
        };
        rigid_tree.rotate_on_axis(DVec3::Z, 0.5);
        assert!(
            rigid_tree
                .transform
                .rotation
                .abs_diff_eq(DQuat::from_axis_angle(DVec3::Z, 0.5), 1e-9)
        );
    }

    #[test]
    fn interpolation_moves_poses_only() {
        let geometry =
            || HierarchyVItem::from_vpoints(vec![dvec3(0.0, 0.0, 0.0), dvec3(1.0, 0.0, 0.0)]);
        let start = Node::leaf(geometry())
            .with_id("start")
            .with_transform(DAffine3::from(Translation(dvec3(1.0, 0.0, 0.0))));
        let end = Node::leaf(geometry())
            .with_id("end")
            .with_transform(DAffine3::from(Translation(dvec3(3.0, 2.0, 0.0))));

        assert!(start.is_aligned(&end));
        let mid = start.lerp(&end, 0.5);
        let (_, mid_leaf) = mid.leaves().next().unwrap();
        assert_eq!(mid_leaf.vpoints.0, vec![DVec3::ZERO, DVec3::X]);
        assert_affine_eq(
            mid.transform,
            DAffine3::from_translation(dvec3(2.0, 1.0, 0.0)),
        );
        assert_eq!(mid.id.as_deref(), Some("end"));
    }

    #[test]
    #[should_panic(expected = "aligned")]
    fn unaligned_kind_lerp_panics_with_guidance() {
        let leaf: Node<u32> = Node::leaf(1);
        let group: Node<u32> = Node::group(vec![Node::leaf(2)]);
        drop(leaf.lerp(&group, 0.5));
    }

    #[test]
    fn unequal_sibling_counts_follow_vecs_truncating_zip() {
        let few = Node::<u32>::group(vec![Node::leaf(1)]);
        let many = Node::<u32>::group(vec![Node::leaf(1), Node::leaf(2), Node::leaf(3)]);
        let mid = many.lerp(&few, 0.5);
        assert_eq!(mid.children().unwrap().len(), 1);
    }

    #[test]
    fn partial_forwards_ranges_down_recursively() {
        let base = stroked_vitem(0.0);
        let tree = Node::leaf(base.clone()).with_transform(DAffine3::from(Translation(DVec3::X)));

        let partial = tree.get_partial(0.25..0.75);
        assert_eq!(partial.transform, DAffine3::from(Translation(DVec3::X)));
        assert_eq!(
            partial.leaf_content().unwrap(),
            &base.get_partial(0.25..0.75),
            "the range must be forwarded verbatim"
        );

        let closed = tree.get_partial_closed(0.25..0.75);
        assert_eq!(
            closed.leaf_content().unwrap(),
            &base.get_partial_closed(0.25..0.75)
        );

        let grouped = Node::<HierarchyVItem>::group(vec![Node::leaf(base.clone()); 3]);
        let partial = grouped.get_partial(0.0..0.5);
        assert_eq!(partial.children().unwrap().len(), 3);
    }

    #[test]
    fn empty_forwarding_spans_all_storage_groups() {
        let translation_empty = Node::<HierarchyVItem, Translation>::empty();
        assert_eq!(translation_empty.transform, Translation(DVec3::ZERO));
        let rigid_empty = Node::<HierarchyVItem, Rigid>::empty();
        assert_eq!(rigid_empty.transform, Rigid::IDENTITY);
        let similarity_empty = Node::<HierarchyVItem, Similarity>::empty();
        assert_eq!(similarity_empty.transform, Similarity::IDENTITY);
        let diag_empty = Node::<HierarchyVItem, Diag>::empty();
        assert_eq!(diag_empty.transform, Diag(DVec3::ONE));
        let affine_empty = Node::<HierarchyVItem>::empty();
        assert_eq!(affine_empty.transform, DAffine3::IDENTITY);

        // All five are single leaves carrying the `Empty` leaf payload.
        for (is_leaf, name) in [
            (translation_empty.is_leaf(), "Translation"),
            (rigid_empty.is_leaf(), "Rigid"),
            (similarity_empty.is_leaf(), "Similarity"),
            (diag_empty.is_leaf(), "Diag"),
            (affine_empty.is_leaf(), "DAffine3"),
        ] {
            assert!(is_leaf, "{name} empties must be leaves");
        }
        let leaf = affine_empty.leaf_content().unwrap();
        assert_eq!(leaf.stroke_widths[0].0, 0.0);
        assert!(leaf.fill_rgbas.iter().all(|rgba| rgba.0 == Vec4::ZERO));
    }

    #[test]
    fn index_paths_walk_children_and_fail_closed() {
        let tree = Node::<u32>::group(vec![
            Node::group(vec![Node::leaf(1), Node::leaf(2)]),
            Node::leaf(3),
        ]);

        assert_eq!(tree.get(&[]).unwrap().children().unwrap().len(), 2);
        assert_eq!(tree.get(&[0, 1]).and_then(Node::leaf_content), Some(&2));
        assert_eq!(tree.get(&[1]).and_then(Node::leaf_content), Some(&3));
        assert!(tree.get(&[2]).is_none());
        // Cannot descend into a leaf.
        assert!(tree.get(&[1, 0]).is_none());

        let mut tree = tree;
        let target = tree.get_mut(&[0, 0]).unwrap();
        assert_eq!(target.leaf_content(), Some(&1));
        target.transform = DAffine3::from(Translation(DVec3::X));
        assert_eq!(
            tree.get(&[0, 0]).unwrap().transform,
            DAffine3::from(Translation(DVec3::X))
        );
    }

    #[test]
    fn leaves_accumulate_correctly_over_skewed_affine_chains() {
        let scale = DAffine3::from_scale(dvec3(2.0, 3.0, 4.0));
        let rotate = DAffine3::from_rotation_translation(
            DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2),
            DVec3::ZERO,
        );
        let translate = DAffine3::from_translation(DVec3::X);

        // root -> child -> grandchild -> leaf, each carrying one factor.
        let tree = CoreNode::new(
            scale,
            NodeContent::Children(vec![CoreNode::new(
                rotate,
                NodeContent::Children(vec![CoreNode::new(
                    translate,
                    NodeContent::Leaf(CoreVItem {
                        points: vec![Vec4::ZERO],
                        ..Default::default()
                    }),
                )]),
            )]),
        );

        let expected = scale * rotate * translate;
        let (world, _) = tree.leaves().next().unwrap();

        // Hand-check the skewed composition: X is scaled to 2 then rotated
        // onto Y and scaled by 3, so translating by X lands at (0, 3, 0).
        assert!(
            world
                .transform_point3(DVec3::ZERO)
                .abs_diff_eq(dvec3(0.0, 3.0, 0.0), 1e-9)
        );
        assert_affine_eq(world, expected);
    }

    #[test]
    fn transformed_lift_roundtrips_through_nodes() {
        let similarity = Similarity {
            scale: 2.0,
            rotation: DQuat::IDENTITY,
            translation: dvec3(1.0, 2.0, 3.0),
        };
        let wrapped = Transformed::new(stroked_vitem(0.0), similarity);

        // The lift keeps the wrapper's exact storage type (no widening).
        let node: Node<HierarchyVItem, Similarity> = wrapped.clone().into();
        assert!(node.is_leaf());
        assert_eq!(node.id, None);
        assert_eq!(node.transform, similarity);

        let roundtrip = node.into_transformed().unwrap();
        assert_eq!(roundtrip.inner, wrapped.inner);
        assert_eq!(roundtrip.transform, wrapped.transform);

        // A branching tree cannot collapse back into a single wrapper.
        let branching = Node::<HierarchyVItem>::group(vec![
            Node::leaf(HierarchyVItem::empty()).with_transform(DAffine3::from(similarity)),
            Node::leaf(HierarchyVItem::empty()),
        ]);
        assert!(branching.into_transformed().is_none());

        // ...while a pure chain of single-child groups collapses,
        // composing transforms inside the SAME storage family.
        let rigid_chain = Node::<HierarchyVItem, Rigid>::group(vec![
            Node::group(vec![
                Node::leaf(HierarchyVItem::empty())
                    .with_transform(Rigid::from_translation(DVec3::X)),
            ])
            .with_transform(Rigid::from_axis_angle(
                DVec3::Z,
                std::f64::consts::FRAC_PI_2,
            )),
        ])
        .with_transform(Rigid::from_translation(DVec3::Y));
        let collapsed = rigid_chain.into_transformed().unwrap();
        assert_eq!(collapsed.inner, HierarchyVItem::empty());
        let rigid = collapsed.transform;
        let expected = (Rigid::from_translation(DVec3::Y).compose(&Rigid::from_axis_angle(
            DVec3::Z,
            std::f64::consts::FRAC_PI_2,
        )))
        .compose(&Rigid::from_translation(DVec3::X));
        assert!(
            DAffine3::from(rigid)
                .transform_point3(DVec3::ZERO)
                .abs_diff_eq(DAffine3::from(expected).transform_point3(DVec3::ZERO), 1e-9)
        );
    }

    #[test]
    fn map_inner_maps_payloads_and_keeps_the_shape() {
        let tree = Node::<u32, Translation>::group(vec![
            Node::leaf(1)
                .with_id("one")
                .with_transform(Translation(DVec3::X)),
            Node::group(vec![Node::leaf(2)]),
        ]);
        let mapped = tree.map_inner(|payload| payload * 10);

        assert_eq!(
            mapped.get(&[0]).unwrap().leaf_content(),
            Some(&10),
            "ids and shape survive mapping"
        );
        assert_eq!(mapped.get(&[0]).unwrap().id.as_deref(), Some("one"));
        assert_eq!(mapped.get(&[0]).unwrap().transform, Translation(DVec3::X));
        assert_eq!(mapped.get(&[1, 0]).unwrap().leaf_content(), Some(&20));
    }

    #[test]
    fn derived_impls_behave_like_plain_data() {
        let make = |id: &str| {
            Node::<u32>::leaf(7)
                .with_id(id)
                .with_transform(Translation(DVec3::X).into())
        };
        let a = make("a");
        let b = make("b");

        assert_eq!(a, a.clone());
        assert_ne!(a, b, "external ids participate in equality");

        let mut cloned = a.clone();
        *cloned.leaf_content_mut().unwrap() = 8;
        assert_ne!(a, cloned, "clones must be independent");

        let rendered = format!("{:?}", make("dbg"));
        assert!(rendered.contains("\"dbg\""), "debug output is {rendered}");
    }

    #[test]
    fn styling_traits_read_first_leaf_and_write_every_leaf() {
        let mut tree = Node::<HierarchyVItem>::group(vec![
            Node::leaf(stroked_vitem(0.0)),
            Node::group(vec![
                Node::leaf(stroked_vitem(5.0)),
                Node::leaf(stroked_vitem(9.0)),
            ]),
        ]);

        // Getters report the first leaf encountered in DFS order.
        assert_eq!(tree.stroke_width(), 0.04);
        tree.set_opacity(0.5);
        let opacities: Vec<f32> = tree
            .leaves_mut()
            .map(|leaf| leaf.stroke_rgbas[0].0.w)
            .collect();
        assert_eq!(opacities, [0.5, 0.5, 0.5]);

        // Stroke functions forward to each leaf independently.
        tree.set_stroke_width(1.0);
        let widths: Vec<f32> = tree
            .leaves_mut()
            .map(|leaf| leaf.stroke_widths[0].0)
            .collect();
        assert_eq!(widths, [1.0, 1.0, 1.0]);

        // An empty tree warns and reports neutral values.
        let empty = Node::<HierarchyVItem>::group(Vec::new());
        assert_eq!(empty.stroke_width(), 0.0);
        assert_eq!(empty.fill_color(), css::WHITE);
    }
}
