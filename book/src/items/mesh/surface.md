# `Surface`

参数曲面：在 `(u, v)` 网格上采样生成网格数据。

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

- 顶点按行主序存储：`points[i * nv + j]`；
- `with_vertex_colors(colors)` 直接指定每顶点颜色；
- `From<Surface> for MeshItem` 完成到 `MeshItem` 的转换；`Surface` 自身也实现
  了 `Extract`，可直接作为动画输出类型。
