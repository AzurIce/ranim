# `MeshItem`

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

```rust,ignore
pub struct MeshItem {
    pub points: PointVec<DVec3>,         // 顶点（局部空间）
    pub triangle_indices: Vec<u32>,      // 三角形索引
    pub vertex_colors: PointVec<Rgba>,   // 每顶点颜色
    pub vertex_normals: PointVec<DVec3>, // 每顶点法线；全零 → flat shading
}
```

与 core `MeshItem` 一一对应，但使用 f64 类型，且顶点/颜色/法线包在
`PointVec` 里以支持对齐与插值，因此可以直接参与 `morph` 等动画。

构造与常用操作：

```rust,ignore
// 仅顶点（无索引，适合点云）或 顶点+索引
let mesh = MeshItem::from_vertices(points);
let mesh = MeshItem::from_indexed_vertices(points, triangle_indices);

let mesh = mesh.with_color(manim::BLUE_C); // 统一每顶点颜色
mesh.vertex_colors = colors.into();        // 或逐顶点自定义
```

## 变换：`Transformed<T, G>`

`MeshItem` 自身不持有变换矩阵——顶点始终处于局部空间。需要摆放、移动、
旋转、缩放时，用 [`Transformed<T, G>`](../../understand/core/transformed.md)
包裹。现有 mesh/surface 场景通常以 `DAffine3` 作为存储上界：

```rust,ignore
let mesh: Transformed<_, DAffine3> = Transformed::new(
    mesh,
    DAffine3::from_translation(...),
);
```

也可以用 prelude 中的 extension trait：

```rust,ignore
let mesh = mesh.transformed::<DAffine3>(DAffine3::IDENTITY);
```

`Transformed<T, G>` 实现了 `Interpolatable`（`transform` 与内部数据分别插值）、
`Aabb`、`Alignable`，并在 `G: From<H>` 时通过 `ApplyTransform<H>` 做 outer
composition。它不会自动 widening；需要更一般的存储时显式转换为
`Transformed<_, DAffine3>`。extract 时才把 `G` 转为 `DAffine3` 并展平进
CoreItem：`MeshItem` 左乘其渲染用 `transform` 矩阵，`VItem` 则逐点烘焙。
层旋转、整体移动这类动画应优先用 wrapper（或像
`examples/tetrahedron_spheres` 那样在自定义 Eval 中直接更新公开的
`transform` 字段），而不是逐顶点改 `points`。

两个辅助函数：

- `generate_grid_indices(nu, nv)`：生成 `nu × nv` 行主序网格的三角形索引；
- `compute_smooth_normals(points, triangle_indices)`：按顶角加权的平滑法线
  （退化三角形自动跳过）。
