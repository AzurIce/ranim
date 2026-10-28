# 几何构造器

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

`vitem::geometry` 子模块提供常用平面图形的构造器。它们都是「数据 struct +
`From<...> for VItem`」：字段公开可直接改，也实现了常用的定位/变换 trait。

| 类型 | 说明 |
|---|---|
| `Circle` | 圆（半径） |
| `Ellipse` | 椭圆 |
| `Arc` / `ArcBetweenPoints` | 圆弧 / 过两点与半径的圆弧 |
| `EllipticArc` | 椭圆弧 |
| `Line` | 线段 |
| `Square` / `Rectangle` | 正方形 / 矩形（canonical local 尺寸） |
| `Polygon` / `RegularPolygon` | 任意多边形 / 正多边形 |
| `Parallelogram` | 平行四边形 |

这些构造器的 canonical local primitive 都有明确的局部坐标约定：通常以原点为
中心，`Rectangle` 的尺寸沿其 intrinsic/canonical X/Y axes 解释，`ArcBetweenPoints`
则保留由输入点决定的 local center。构造器不会因为输入数据“看起来偏了”就自动
中心化；一般 local data（例如 `VItem` 点集或 `Surface` 顶点）也同样保持调用者
提供的坐标。

需要把物件放到场景中的位置时，优先把 placement 放在
`Transformed<_, G>` 的外层；anchor 若有 forwarding，会先在 inner/local item
上计算，再应用外部变换。`Origin` 表示 primitive 的 local origin，`Focus`
仍只表示椭圆自身的焦点语义，不会被 wrapper 重新解释。`AabbPoint` 的通用
实现按目标的 AABB 工作，不能假定它会按任意 anchor 的 local 语义穿过 wrapper；
需要明确的 local anchor 时，应先对 `inner` 定位再手动应用 `transform`。

`Rectangle::scale_axes` 是 intrinsic shape-data 编辑：它改变尺寸参数，而不是
给 wrapper 做矩阵组合，也不会改变“外部 placement”的职责。

```rust,ignore
let vitem = VItem::from(
    Square::new(2.0).with(|sq| {
        sq.set_color(manim::RED_C);
    })
);
```
