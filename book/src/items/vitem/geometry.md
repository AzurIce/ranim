# 几何构造器

`vitem::geometry` 子模块提供常用二维图形的构造器。它们都是「数据 struct +
`From<...> for VItem`」：字段公开可直接改，也实现了常用的定位/变换 trait。

| 类型 | 说明 |
|---|---|
| `Circle` | 圆（半径） |
| `Ellipse` | 椭圆 |
| `Arc` / `ArcBetweenPoints` | 圆弧 / 过两点与半径的圆弧 |
| `EllipticArc` | 椭圆弧 |
| `Line` | 线段 |
| `Square` / `Rectangle` | 正方形 / 矩形（`axes` 可改朝向） |
| `Polygon` / `RegularPolygon` | 任意多边形 / 正多边形 |
| `Parallelogram` | 平行四边形 |

```rust,ignore
let vitem = VItem::from(
    Square::new(2.0).with(|sq| {
        sq.set_color(manim::RED_C);
    })
);
```
