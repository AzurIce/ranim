# `RenderMeshItem`

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

3D 三角网格的渲染组件，需要 items 的 `render` feature，定义于 `ranim_items::mesh::render`。

```rust,ignore
pub struct RenderMeshItem {
    /// 顶点（局部空间）
    pub points: Vec<Vec3>,
    /// 三角形索引
    pub triangle_indices: Vec<u32>,
    /// 局部到世界的变换
    pub transform: Mat4,
    /// 每顶点颜色
    pub vertex_colors: Vec<Rgba>,
    /// 每顶点法线（用于平滑着色）
    pub vertex_normals: Vec<Vec3>,
}
```

要点：

- `points` 与 `triangle_indices` 描述局部空间几何，渲染时统一乘
  `transform`；平移/旋转/缩放任一动画都应优先作用在 `transform` 上，而不是
  逐顶点改 `points`。
- `vertex_normals` 全零或为空时，着色器回退到用 `dpdx`/`dpdy` 计算的
  **flat shading**；需要平滑着色时由用户层（如 `Surface::with_smooth_normals`）
  预计算法线。
- 几何细节（折叠的边、重合顶点）不会被渲染器清理，索引中的退化三角形由
  调用方避免。

逻辑 `ranim_items::mesh::MeshItem` 在渲染侧转换为该组件。
`ranim_items::mesh::MeshItemPlugin` 安装裸逻辑类型的提取与网格 GPU 管线；
默认渲染配置已包含它，自定义 `RenderSetup` 时经 `RanimItemsPlugin` 或单独安装。
`Transformed<MeshItem, G>` 或容器输出另注册其实际完整类型的 `ExtractPlugin<T>`。
