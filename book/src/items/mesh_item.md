# MeshItem 类

`ranim_items::mesh` 模块：三维网格物件。核心是 `MeshItem`，外加参数曲面
`Surface` 与球体 `Sphere` 两个便捷构造类型。

## `MeshItem`

```rust,ignore
pub struct MeshItem {
    pub points: PointVec<DVec3>,         // 顶点（局部空间）
    pub triangle_indices: Vec<u32>,      // 三角形索引
    pub transform: DMat4,                // 局部到世界
    pub vertex_colors: PointVec<Rgba>,   // 每顶点颜色
    pub vertex_normals: PointVec<DVec3>, // 每顶点法线；全零 → flat shading
}
```

与 core `MeshItem` 一一对应，但顶点/颜色/法线包在 `PointVec` 里以支持
对齐与插值，因此可以直接参与 `morph` 等动画。

构造与常用操作：

```rust,ignore
// 仅顶点（无索引，适合点云）或 顶点+索引
let mesh = MeshItem::from_vertices(points);
let mesh = MeshItem::from_indexed_vertices(points, triangle_indices);

let mesh = mesh
    .with_transform(DMat4::from_translation(...))  // 设置变换
    .with_color(manim::BLUE_C);                    // 统一每顶点颜色
mesh.vertex_colors = colors.into();               // 或逐顶点自定义
```

实现了 `FillColor` / `Opacity` / `Aabb` 与 `ShiftTransform` /
`RotateTransform` / `ScaleTransform`——变换都作用在 `transform` 矩阵上，
不改顶点数据。层旋转、整体移动这类动画应优先变换 `transform`（或像
`examples/tetrahedron_spheres` 那样自定义 `Eval` 左乘旋转矩阵），而不是
逐顶点改 `points`。

两个辅助函数：

- `generate_grid_indices(nu, nv)`：生成 `nu × nv` 行主序网格的三角形索引；
- `compute_smooth_normals(points, triangle_indices)`：按顶角加权的平滑法线
  （退化三角形自动跳过）。

## `Surface`

参数曲面：在 `(u, v)` 网格上采样生成 `MeshItem` 数据。

```rust,ignore
let surface = Surface::from_uv_func(
    |u, v| dvec3(u, v, (u * u + v * v).sin()),
    (0.0, 1.0),   // u 范围
    (0.0, 1.0),   // v 范围
    (64, 64),     // 分辨率 (nu, nv)，各自 >= 2
)
.with_fill_by_z(&[(manim::BLUE_C, -1.0), (manim::RED_C, 1.0)]) // 按 z 上色
.with_smooth_normals();  // 预计算平滑法线；不调用则 flat shading
```

`From<Surface> for MeshItem` 完成转换；`Surface` 自身也实现了 `Extract`，
可直接作为动画输出类型。

## `Sphere`

```rust,ignore
let sphere = Sphere::new(0.6)                    // 半径，默认分辨率 (101, 51)
    .with_center(dvec3(1.0, 0.0, 0.0))
    .with_resolution((31, 16))
    .with_fill_color(manim::YELLOW_C);
let mesh = MeshItem::from(sphere);               // Sphere → Surface → MeshItem
```

球面按 `u ∈ [0, 2π]`、`v ∈ [0, π]` 参数化；`From<Sphere> for Surface` 默认
flat shading，需要平滑效果时先转 `Surface` 再 `with_smooth_normals()`。

## 选择建议

- 规则几何体（球、参数曲面）：用 `Sphere` / `Surface` 构造；
- 任意几何（自定义多面体、模型）：直接拼 `MeshItem` 的顶点与索引；
- 平滑曲面记得 `with_smooth_normals()`；硬边物体保持法线全零走 flat
  shading 即可。
