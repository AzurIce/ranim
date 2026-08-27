# `Sphere`

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

球体便捷构造，定义于 `ranim_items::mesh`。

```rust,ignore
let sphere = Sphere::new(0.6)                    // 半径，默认分辨率 (101, 51)
    .with_resolution((31, 16))
    .with_fill_color(manim::YELLOW_C);
let mesh = MeshItem::from(sphere);               // Sphere → Surface → MeshItem
let placed = mesh.transformed(DAffine3::from_translation(dvec3(1.0, 0.0, 0.0)));
```

`Sphere` 是 canonical local primitive：球心固定在 local 原点，半径定义
其 local 尺寸。需要场景 placement 时，应像上面的示例一样使用外层
`Transformed`，而不是给 `Sphere` 增加中心字段。球面按 `u ∈ [0, 2π]`、
`v ∈ [0, π]` 参数化。
`From<Sphere> for Surface` 默认 flat shading；需要平滑效果时先转 `Surface`
再 `with_smooth_normals()`：

```rust,ignore
let mesh = MeshItem::from(Surface::from(sphere).with_smooth_normals());
```
