use glam::{Mat4, Vec2, Vec3, Vec4};

/// Camera/view data consumed by the renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewData {
    /// Projection matrix.
    pub proj_mat: Mat4,
    /// View matrix.
    pub view_mat: Mat4,
    /// Half size of the camera frame in world units.
    pub half_frame_size: Vec2,
}

impl Default for ViewData {
    fn default() -> Self {
        Self {
            proj_mat: Mat4::IDENTITY,
            view_mat: Mat4::IDENTITY,
            half_frame_size: Vec2::splat(1.0),
        }
    }
}

/// Render-side vector item data.
///
/// `points` stores xyz in world space and a close-path flag in w.
/// Style arrays are expected to be aligned to the item anchors.
#[derive(Debug, Clone, PartialEq)]
pub struct VItemRenderData {
    /// 3D points with the close-path flag in w.
    pub points: Vec<Vec4>,
    /// Optional projection plane normal.
    pub normal: Option<Vec3>,
    /// Fill colors in linear RGBA.
    pub fill_rgbas: Vec<Vec4>,
    /// Stroke colors in linear RGBA.
    pub stroke_rgbas: Vec<Vec4>,
    /// Stroke widths.
    pub stroke_widths: Vec<f32>,
}

impl VItemRenderData {
    /// Number of per-anchor style attributes required by this item.
    pub fn attr_count(&self) -> usize {
        self.points.len().div_ceil(2)
    }
}

/// Render-side mesh item data.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshRenderData {
    /// Mesh vertex positions.
    pub points: Vec<Vec3>,
    /// Triangle indices.
    pub indices: Vec<u32>,
    /// Mesh transform matrix.
    pub transform: Mat4,
    /// Per-vertex colors in linear RGBA.
    pub vertex_colors: Vec<Vec4>,
    /// Per-vertex normals. Missing normals are treated as zero by the renderer.
    pub vertex_normals: Vec<Vec3>,
}

/// A retained render input buffer.
///
/// This type is intentionally a render-side scene, not a ranim semantic scene.
/// It can be reused across frames by calling [`RenderScene::reset`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderScene {
    /// View/camera data.
    pub view: ViewData,
    /// Vector items to render.
    pub vitems: Vec<VItemRenderData>,
    /// Mesh items to render.
    pub meshes: Vec<MeshRenderData>,
}

impl RenderScene {
    /// Create an empty render scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all renderable items while preserving allocation.
    pub fn reset(&mut self) {
        self.vitems.clear();
        self.meshes.clear();
    }
}
