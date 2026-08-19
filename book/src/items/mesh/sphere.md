# `Sphere`

球体便捷构造，定义于 `ranim_items::mesh`。

```rust,ignore
let sphere = Sphere::new(0.6)                    // 半径，默认分辨率 (101, 51)
    .with_center(dvec3(1.0, 0.0, 0.0))
    .with_resolution((31, 16))
    .with_fill_color(manim::YELLOW_C);
let mesh = MeshItem::from(sphere);               // Sphere → Surface → MeshItem
```

球面按 `u ∈ [0, 2π]`、`v ∈ [0, π]` 参数化。`From<Sphere> for Surface` 默认
flat shading；需要平滑效果时先转 `Surface` 再 `with_smooth_normals()`：

```rust,ignore
let mesh = MeshItem::from(Surface::from(sphere).with_smooth_normals());
```
