# `Surface`

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

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

顶点是调用者提供的一般 local data，不会被自动中心化或平移；顶点按行主序存储：
`points[i * nv + j]`。需要把曲面放到场景中时，用外层
`Transformed<Surface, G>` 保存 placement，而不是修改采样坐标。

- `with_vertex_colors(colors)` 直接指定每顶点颜色；
- `From<Surface> for MeshItem` 完成到 `MeshItem` 的转换；`Surface` 自身也实现
  了 `Extract`，可直接作为动画输出类型。

从 `Sphere` 转成 `Surface` 时，球面的 canonical local 原点已经由 Sphere 的
采样函数确定；`Surface` 不会再次执行 center，因此不会发生重复 center。若
需要把参数曲面变成可直接吸收某种变换的物件，可以在明确边界处调用 `bake`；
否则保留 wrapper，将 placement 留在外层。
