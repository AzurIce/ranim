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
//! use ranim_core::glam::{Vec4, dvec3};
//! use ranim_core::traits::ShiftTransform;
//! use ranim_core::Extract;
//! use ranim_items::hierarchy::Node;
//!
//! let stroke = CoreVItem {
//!     points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
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
/// transform, an optional payload, and ordered children.
///
/// Every node is a *frame*: its payload (if any) and its children all live
/// in the node's local space, and the node's transform places that space in
/// the parent's. This is the same shape as a glTF node (`matrix|TRS`,
/// `mesh?`, `children?`) or a Bevy entity (components + `Children`): a node
/// may carry a payload and children at the same time, and a node without
/// either is a pure anchor frame.
///
/// # Examples
///
/// Build trees with [`Node::leaf`], [`Node::group`], [`Node::branch`], and
/// the builders, or with struct literals when every field is known:
///
/// ```
/// use ranim_core::glam::dvec3;
/// use ranim_core::traits::Translation;
/// use ranim_items::hierarchy::Node;
///
/// let node = Node {
///     id: Some("outer".into()),
///     transform: Translation(dvec3(1.0, 0.0, 0.0)),
///     item: Some("geometry".to_string()),
///     children: Vec::new(),
/// };
/// assert_eq!(node.item(), Some(&"geometry".to_string()));
/// ```
pub struct Node<I, G = DAffine3> {
    /// External identifier carried from the source format (e.g. SVG element
    /// id, glTF node name). Ignored by rendering/extraction beyond being
    /// transported: alignment preserves it, and lerping switches ids at the
    /// mid-point exactly like other front-loaded fields in ranim.
    pub id: Option<String>,
    /// The transform from this node's local space into the parent's space.
    pub transform: G,
    /// The payload carried by this node, in canonical local coordinates.
    pub item: Option<I>,
    /// The ordered child nodes, living in this node's local space. Stored in
    /// source order; extraction preserves that order depth-first so
    /// downstream consumers see painter's-algorithm ordering.
    pub children: Vec<Node<I, G>>,
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
            item: self.item.clone(),
            children: self.children.clone(),
        }
    }
}

impl<I: PartialEq, G: PartialEq> PartialEq for Node<I, G> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.transform == other.transform
            && self.item == other.item
            && self.children == other.children
    }
}

impl<I: Eq, G: Eq> Eq for Node<I, G> {}

impl<I: fmt::Debug, G: fmt::Debug> fmt::Debug for Node<I, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("transform", &self.transform)
            .field("item", &self.item)
            .field("children", &self.children)
            .finish()
    }
}

// MARK: Inherent API

impl<I, G> Node<I, G> {
    /// Pair a payload (if any) and children with a transform, without any
    /// external id.
    pub fn new(transform: G, item: Option<I>, children: Vec<Self>) -> Self {
        Self {
            id: None,
            transform,
            item,
            children,
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

    /// Whether this node is a bare leaf: a payload with no children.
    pub fn is_leaf(&self) -> bool {
        self.item.is_some() && self.children.is_empty()
    }

    /// Whether this node is a pure frame: no payload, only (possibly empty)
    /// children.
    pub fn is_group(&self) -> bool {
        self.item.is_none()
    }

    /// The payload carried by this node, if any.
    pub fn item(&self) -> Option<&I> {
        self.item.as_ref()
    }

    /// The payload mutably, if any.
    pub fn item_mut(&mut self) -> Option<&mut I> {
        self.item.as_mut()
    }

    /// The ordered children.
    pub fn children(&self) -> &[Node<I, G>] {
        &self.children
    }

    /// The ordered children mutably.
    pub fn children_mut(&mut self) -> &mut [Node<I, G>] {
        &mut self.children
    }

    /// The first leaf payload in depth-first order, without composing any
    /// transforms. This backs the color/stroke-width getters; `None` means
    /// the tree has no payloads at all.
    pub fn first_leaf(&self) -> Option<&I> {
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            if let Some(item) = &node.item {
                return Some(item);
            }
            stack.extend(node.children.iter().rev());
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
            node = node.children.get(index)?;
        }
        Some(node)
    }

    /// Mutable variant of [`Node::get`].
    pub fn get_mut(&mut self, path: &[usize]) -> Option<&mut Node<I, G>> {
        let mut node = self;
        for &index in path {
            node = node.children.get_mut(index)?;
        }
        Some(node)
    }

    /// The first node (depth-first preorder) whose id equals `id`.
    ///
    /// Ids are external labels (SVG ids, glTF node names) and are not
    /// guaranteed unique, so duplicates resolve to the preorder-first match;
    /// see [`Node::by_ids`] for every match and [`Node::by_id_path`] for a
    /// reusable address. `None` when no node carries the id.
    pub fn by_id(&self, id: &str) -> Option<&Self> {
        if self.id.as_deref() == Some(id) {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.by_id(id))
    }

    /// Mutable variant of [`Node::by_id`].
    pub fn by_id_mut(&mut self, id: &str) -> Option<&mut Self> {
        if self.id.as_deref() == Some(id) {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.by_id_mut(id))
    }

    /// Every node whose id equals `id`, in depth-first order.
    pub fn by_ids(&self, id: &str) -> Vec<&Self> {
        let mut matches = Vec::new();
        self.collect_by_id(id, &mut matches);
        matches
    }

    fn collect_by_id<'a>(&'a self, id: &str, matches: &mut Vec<&'a Self>) {
        if self.id.as_deref() == Some(id) {
            matches.push(self);
        }
        for child in &self.children {
            child.collect_by_id(id, matches);
        }
    }

    /// The index path (see [`Node::get`]) of the first node in depth-first
    /// order whose id equals `id`; the root's path is empty. Useful to reuse
    /// an address across frames without re-searching.
    pub fn by_id_path(&self, id: &str) -> Option<Vec<usize>> {
        if self.id.as_deref() == Some(id) {
            return Some(Vec::new());
        }
        for (index, child) in self.children.iter().enumerate() {
            if let Some(mut path) = child.by_id_path(id) {
                path.insert(0, index);
                return Some(path);
            }
        }
        None
    }

    /// The first leaf payload in depth-first order, mutably.
    pub fn first_leaf_mut(&mut self) -> Option<&mut I> {
        if let Some(item) = self.item.as_mut() {
            return Some(item);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.first_leaf_mut())
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
        fn map_inner_rec<I, U, G>(node: Node<I, G>, mut f: &mut impl FnMut(I) -> U) -> Node<U, G> {
            let Node {
                id,
                transform,
                item,
                children,
            } = node;
            Node {
                id,
                transform,
                item: item.map(&mut f),
                children: children
                    .into_iter()
                    .map(|child| map_inner_rec(child, f))
                    .collect(),
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
                item,
                children,
            } = node;
            Node {
                id,
                transform: f(transform),
                item,
                children: children
                    .into_iter()
                    .map(|child| map_transform_rec(child, f))
                    .collect(),
            }
        }
        let mut f = f;
        map_transform_rec(self, &mut f)
    }
}

impl<I, G: TransformGroup> Node<I, G> {
    /// Create a bare leaf — a payload with no children — with the identity
    /// transform and no id.
    pub fn leaf(item: I) -> Self {
        Self {
            id: None,
            transform: G::identity(),
            item: Some(item),
            children: Vec::new(),
        }
    }

    /// Create a pure frame node with the identity transform and no id; the
    /// children live in the frame's local space.
    pub fn group(children: Vec<Self>) -> Self {
        Self {
            id: None,
            transform: G::identity(),
            item: None,
            children,
        }
    }

    /// Create a branch node carrying both a payload and children, with the
    /// identity transform and no id. The payload paints before the children.
    pub fn branch(item: I, children: Vec<Self>) -> Self {
        Self {
            id: None,
            transform: G::identity(),
            item: Some(item),
            children,
        }
    }
}

impl<I, G> Node<I, G>
where
    G: TransformGroup + Clone,
{
    /// Collapse a single-payload spine back into a [`Transformed`] wrapper.
    ///
    /// Walks down chains of pure frames composing their transforms in `G`
    /// along the way (`acc.compose(child)`), so the storage type is preserved
    /// exactly. Returns `None` when the spine branches or ends without a
    /// payload, because such a tree holds zero or several leaves.
    pub fn into_transformed(self) -> Option<Transformed<I, G>> {
        let Node {
            transform,
            item,
            children,
            ..
        } = self;
        collapse_into_transformed(transform, item, children)
    }
}

/// Fold `acc` (the composed ancestors' transform, outermost first) down
/// through a single-payload spine into a [`Transformed`].
fn collapse_into_transformed<I, G>(
    acc: G,
    item: Option<I>,
    children: Vec<Node<I, G>>,
) -> Option<Transformed<I, G>>
where
    G: TransformGroup + Clone,
{
    match (item, children.len()) {
        (Some(item), 0) => Some(Transformed::new(item, acc)),
        (None, 1) => {
            let mut children = children;
            let child = children.swap_remove(0);
            collapse_into_transformed(acc.compose(&child.transform), child.item, child.children)
        }
        _ => None,
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
            // A node may carry a payload *and* children: queue the children
            // first (in reverse, so popping yields source order), then yield
            // the payload — painter's-algorithm order with the payload
            // painting before its descendants.
            for child in node.children.iter().rev() {
                let acc_child = acc.compose(&child.transform.clone().into());
                self.stack.push((acc_child, child));
            }
            if let Some(item) = &node.item {
                return Some((acc, item));
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
            self.stack.extend(node.children.iter_mut().rev());
            if let Some(item) = node.item.as_mut() {
                return Some(item);
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
        if let Some(item) = &self.item {
            item.extract_into(buf);
        }
        for child in &self.children {
            child.extract_into(buf);
        }
        // Each level post-multiplies its own affine onto whatever its payload
        // and subtree appended, so a chain composes as
        // `t_root * ... * t_leaf * local`. Recursing before applying also
        // keeps emission depth-first: paint order survives flattening, and a
        // node's own payload paints before its descendants.
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
    /// Structural lerp: nodes interpolate positionally, payloads and node
    /// poses interpolate independently, and ids switch at the mid-point like
    /// other front-loaded fields in ranim. Callers must have aligned
    /// structures first (see [`Alignable`]): payload-presence mismatches
    /// panic, and unequal sibling counts follow `Vec`'s truncating-zip
    /// precedent.
    fn lerp(&self, target: &Self, t: f64) -> Self {
        Self {
            id: if t < 0.5 {
                self.id.clone()
            } else {
                target.id.clone()
            },
            transform: self.transform.lerp(&target.transform, t),
            item: lerp_items(&self.item, &target.item, t),
            children: self
                .children
                .iter()
                .zip(target.children.iter())
                .map(|(a, b)| a.lerp(b, t))
                .collect(),
        }
    }
}

/// Structural lerp over payloads. `None` must pair with `None`: a presence
/// mismatch is filled with transparent clones by [`Alignable`] first.
fn lerp_items<I: Interpolatable>(current: &Option<I>, target: &Option<I>, t: f64) -> Option<I> {
    match (current, target) {
        (Some(current), Some(target)) => Some(current.lerp(target, t)),
        (None, None) => None,
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
    /// interpolation: payload presence must match positionally, sibling
    /// counts must be equal, and every payload and child pair must satisfy
    /// [`Alignable::is_aligned`]. This mirrors the `Vec<T>` blanket's
    /// pre-alignment contract; [`Alignable::align_with`] establishes this
    /// state from mismatched trees.
    fn is_aligned(&self, other: &Self) -> bool {
        let items_aligned = match (&self.item, &other.item) {
            (Some(current), Some(target)) => current.is_aligned(target),
            (None, None) => true,
            _ => false,
        };
        items_aligned
            && self.children.len() == other.children.len()
            && self
                .children
                .iter()
                .zip(other.children.iter())
                .all(|(current, target)| nodes_are_aligned(current, target))
    }

    /// Align two trees for interpolation under one uniform rule: **absence
    /// is filled with a transparent clone of the present side**.
    ///
    /// 1. **Payload presence**: when only one side carries an item, the
    ///    other side receives a transparent (`set_opacity(0.0)`) clone of
    ///    it, so lerping fades the payload in or out smoothly instead of
    ///    jumping.
    /// 2. **Payload pairs**: when both sides carry items, they align with
    ///    each other (vertex-level padding for point data).
    /// 3. **Children**: unequal child counts are padded on both sides. A
    ///    non-empty list grows by repeating its own entries
    ///    (`resize_preserving_order_with_repeated_indices`, matching the
    ///    `Vec<T>: Alignable` blanket); an *empty* list has nothing of its
    ///    own to repeat, so it grows with transparent clones of the other
    ///    side's children — the same absence rule as payloads. Pairs
    ///    recurse.
    fn align_with(&mut self, other: &mut Self) {
        match (self.item.as_mut(), other.item.as_mut()) {
            (None, Some(target_item)) => {
                let mut fill = target_item.clone();
                fill.set_opacity(0.0);
                self.item = Some(fill);
            }
            (Some(current_item), None) => {
                let mut fill = current_item.clone();
                fill.set_opacity(0.0);
                other.item = Some(fill);
            }
            _ => {}
        }
        if let (Some(current), Some(target)) = (&mut self.item, &mut other.item) {
            current.align_with(target);
        }
        let len = self.children.len().max(other.children.len());
        match (self.children.len(), other.children.len()) {
            (0, 0) => {}
            (0, _) => self.children = transparent_clones(&other.children, len),
            (_, 0) => other.children = transparent_clones(&self.children, len),
            _ => {
                expand_with_transparent_repeats(&mut self.children, len);
                expand_with_transparent_repeats(&mut other.children, len);
            }
        }
        self.children
            .iter_mut()
            .zip(other.children.iter_mut())
            .for_each(|(current, target)| current.align_with(target));
    }
}

/// Grow `source` to `len` entries by cloning entries and marking every
/// clone transparent — the absence rule of [`Alignable::align_with`] for
/// child lists that have nothing of their own to repeat.
fn transparent_clones<I, G>(source: &[Node<I, G>], len: usize) -> Vec<Node<I, G>>
where
    I: Opacity,
    Node<I, G>: Clone,
{
    source
        .iter()
        .cycle()
        .take(len)
        .map(|node| {
            let mut fill = node.clone();
            fill.set_opacity(0.0);
            fill
        })
        .collect()
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

/// Structural alignment check mirroring [`Alignable::is_aligned`]: payload
/// presence, sibling counts, and pairwise payload/child alignment.
fn nodes_are_aligned<I, G>(current: &Node<I, G>, target: &Node<I, G>) -> bool
where
    I: Alignable,
{
    let items_aligned = match (&current.item, &target.item) {
        (Some(current), Some(target)) => current.is_aligned(target),
        (None, None) => true,
        _ => false,
    };
    items_aligned
        && current.children.len() == target.children.len()
        && current
            .children
            .iter()
            .zip(target.children.iter())
            .all(|(current, target)| nodes_are_aligned(current, target))
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
            item: self
                .item
                .as_ref()
                .map(|item| item.get_partial(range.clone())),
            children: self
                .children
                .iter()
                .map(|child| child.get_partial(range.clone()))
                .collect(),
        }
    }

    fn get_partial_closed(&self, range: Range<f64>) -> Self {
        Self {
            id: self.id.clone(),
            transform: self.transform.clone(),
            item: self
                .item
                .as_ref()
                .map(|item| item.get_partial_closed(range.clone())),
            children: self
                .children
                .iter()
                .map(|child| child.get_partial_closed(range.clone()))
                .collect(),
        }
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
            // A payload of empty geometry (not `None`), so an `Empty`-seeded
            // interpolation has a payload position to fade through — parity
            // with the old leaf-only shape.
            item: Some(I::empty()),
            children: Vec::new(),
        }
    }
}

// MARK: Aabb

impl<I, G> Aabb for Node<I, G>
where
    I: Aabb,
    G: Clone + Into<DAffine3>,
{
    /// Union of the payload's and descendants' AABBs, then applying this
    /// node's own transform last — the recursive generalization of
    /// `Transformed::aabb`'s 8-corner loop. A node with no payload and no
    /// children warns and reports a degenerate box, mirroring the slice impl
    /// in ranim-core.
    fn aabb(&self) -> [DVec3; 2] {
        let mut inner_box: Option<[DVec3; 2]> = self.item.as_ref().map(Aabb::aabb);
        for child in &self.children {
            let [lo, hi] = child.aabb();
            inner_box = Some(match inner_box {
                Some([acc_lo, acc_hi]) => [acc_lo.min(lo), acc_hi.max(hi)],
                None => [lo, hi],
            });
        }
        let inner_box = inner_box.unwrap_or_else(|| {
            warn!("Empty bounding box, is the tree empty?");
            [DVec3::ZERO, DVec3::ZERO]
        });
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
            item: Some(value.inner),
            children: Vec::new(),
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
            None,
            vec![CoreNode::new(
                DAffine3::from(Translation(dvec3(3.0, 4.0, 5.0))),
                Some(CoreVItem {
                    points: vec![Vec4::new(1.0, 0.0, 0.0, 0.0)],
                    ..Default::default()
                }),
                Vec::new(),
            )],
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
    fn align_fills_absent_payloads_and_children_with_transparent_clones() {
        let left = Node::leaf(stroked_vitem(0.0));
        let right = Node::<HierarchyVItem>::group(vec![
            Node::leaf(stroked_vitem(2.0)).with_transform(DAffine3::from(Translation(DVec3::Y))),
        ]);

        assert!(!left.is_aligned(&right));
        let mut left = left;
        let mut right = right;
        left.align_with(&mut right);

        // Both sides now share the same shape — a payload plus one child —
        // and each absence was filled with a transparent clone of the
        // present side: the group gained a transparent payload, the leaf
        // gained a transparent clone of the child.
        assert!(left.is_aligned(&right));
        assert_eq!(left.children().len(), 1);
        assert_eq!(right.children().len(), 1);
        // The group side's filled payload is transparent; the leaf side's
        // payload keeps its opacity.
        assert_eq!(right.item().unwrap().stroke_rgbas[0].0.w, 0.0);
        assert_eq!(left.item().unwrap().stroke_rgbas[0].0.w, 1.0);

        // Lerping fades both positions: the payload pair holds marker 0
        // while its opacity goes 0 -> 1, and the child pair holds marker 2
        // at Translation(Y) on both sides (identical geometry, so the pose
        // is static) while its opacity fades on the leaf side.
        let mid = left.lerp(&right, 0.5);
        assert_eq!(mid.children().len(), 1);

        let mid_item = mid.item().unwrap();
        assert!((mid_item.vpoints[0].x - 0.0).abs() < 1e-6);
        assert!((mid_item.stroke_rgbas[0].0.w - 0.5).abs() < 1e-6);

        let (mid_world, mid_leaf) = mid.leaves().nth(1).unwrap();
        assert!((mid_leaf.vpoints[0].x - 2.0).abs() < 1e-6);
        assert!((mid_leaf.stroke_rgbas[0].0.w - 0.5).abs() < 1e-6);
        assert_affine_eq(mid_world, DAffine3::from_translation(DVec3::Y));
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
        assert_eq!(small.children().len(), 2);
        // The repeated stand-in became fully transparent while the original
        // kept its opacity.
        let stand_in = &small.children()[1];
        let stand_in = stand_in.item().unwrap();
        assert_eq!(stand_in.stroke_rgbas[0].0.w, 0.0);
        assert_eq!(stand_in.fill_rgbas[0].0.w, 0.0);
        let original = small.children()[0].item().unwrap();
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
            item: Some(()),
            children: Vec::new(),
        };
        tree.shift(DVec3::X).scale_uniform(2.0);
        assert_eq!(tree.transform.scale, 2.0);
        assert_eq!(tree.transform.translation, dvec3(2.0, 0.0, 0.0));

        let mut rigid_tree = Node::<(), Rigid> {
            id: None,
            transform: Rigid::IDENTITY,
            item: Some(()),
            children: Vec::new(),
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
        assert_eq!(mid.children().len(), 1);
    }

    #[test]
    fn partial_forwards_ranges_down_recursively() {
        let base = stroked_vitem(0.0);
        let tree = Node::leaf(base.clone()).with_transform(DAffine3::from(Translation(DVec3::X)));

        let partial = tree.get_partial(0.25..0.75);
        assert_eq!(partial.transform, DAffine3::from(Translation(DVec3::X)));
        assert_eq!(
            partial.item().unwrap(),
            &base.get_partial(0.25..0.75),
            "the range must be forwarded verbatim"
        );

        let closed = tree.get_partial_closed(0.25..0.75);
        assert_eq!(closed.item().unwrap(), &base.get_partial_closed(0.25..0.75));

        let grouped = Node::<HierarchyVItem>::group(vec![Node::leaf(base.clone()); 3]);
        let partial = grouped.get_partial(0.0..0.5);
        assert_eq!(partial.children().len(), 3);
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
        let leaf = affine_empty.item().unwrap();
        assert_eq!(leaf.stroke_widths[0].0, 0.0);
        assert!(leaf.fill_rgbas.iter().all(|rgba| rgba.0 == Vec4::ZERO));
    }

    #[test]
    fn index_paths_walk_children_and_fail_closed() {
        let tree = Node::<u32>::group(vec![
            Node::group(vec![Node::leaf(1), Node::leaf(2)]),
            Node::leaf(3),
        ]);

        assert_eq!(tree.get(&[]).unwrap().children().len(), 2);
        assert_eq!(tree.get(&[0, 1]).and_then(Node::item), Some(&2));
        assert_eq!(tree.get(&[1]).and_then(Node::item), Some(&3));
        assert!(tree.get(&[2]).is_none());
        // Cannot descend into a leaf.
        assert!(tree.get(&[1, 0]).is_none());

        let mut tree = tree;
        let target = tree.get_mut(&[0, 0]).unwrap();
        assert_eq!(target.item(), Some(&1));
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
            None,
            vec![CoreNode::new(
                rotate,
                None,
                vec![CoreNode::new(
                    translate,
                    Some(CoreVItem {
                        points: vec![Vec4::ZERO],
                        ..Default::default()
                    }),
                    Vec::new(),
                )],
            )],
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
            mapped.get(&[0]).unwrap().item(),
            Some(&10),
            "ids and shape survive mapping"
        );
        assert_eq!(mapped.get(&[0]).unwrap().id.as_deref(), Some("one"));
        assert_eq!(mapped.get(&[0]).unwrap().transform, Translation(DVec3::X));
        assert_eq!(mapped.get(&[1, 0]).unwrap().item(), Some(&20));
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
        *cloned.item_mut().unwrap() = 8;
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

    #[test]
    fn by_id_addresses_nodes_by_their_external_label() {
        let tree = Node::<(), DAffine3>::group(vec![
            Node::leaf(()).with_id("dup"),
            Node::group(vec![
                Node::leaf(()).with_id("dup"),
                Node::leaf(()).with_id("other"),
            ]),
        ]);

        // Duplicates resolve to the preorder-first match.
        assert!(tree.by_id("dup").unwrap().is_leaf());
        assert_eq!(tree.by_ids("dup").len(), 2);
        assert_eq!(tree.by_id_path("dup"), Some(vec![0]));
        assert_eq!(tree.by_id_path("other"), Some(vec![1, 1]));
        assert!(tree.by_id("missing").is_none());

        // Mutation reaches exactly the addressed node.
        let mut tree = tree;
        tree.by_id_mut("other").unwrap().id = Some("renamed".into());
        assert!(tree.by_id("other").is_none());
        assert!(tree.by_id("renamed").is_some());

        // An id on the root resolves to the empty path.
        let root = Node::<(), DAffine3>::leaf(()).with_id("root");
        assert_eq!(root.by_id_path("root"), Some(Vec::new()));
        assert!(root.by_id("root").is_some());
    }

    #[test]
    fn first_leaf_mut_reaches_the_first_leaf_in_depth_first_order() {
        let mut tree = Node::<HierarchyVItem>::group(vec![
            Node::group(vec![Node::leaf(stroked_vitem(0.0))]),
            Node::leaf(stroked_vitem(1.0)),
        ]);

        let first = tree.first_leaf_mut().unwrap();
        first.set_stroke_width(7.0);

        let widths: Vec<f32> = tree
            .leaves_mut()
            .map(|leaf| leaf.stroke_widths[0].0)
            .collect();
        assert_eq!(widths, [7.0, 0.04]);
    }
}
