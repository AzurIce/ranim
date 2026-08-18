# `VItem`

```rust,ignore
pub struct VItem {
    pub normal: Option<DVec3>,        // 投影平面法向；None 时渲染时推导
    pub vpoints: VPointVec,           // 点列（二次贝塞尔）
    pub stroke_widths: PointVec<Width>,
    pub stroke_rgbas: PointVec<Rgba>,
    pub fill_rgbas: PointVec<Rgba>,
}
```

## vpoints：二次贝塞尔路径

`vpoints` 是 anchor 与 handle 交替排列的点列：`[a₀, h₀, a₁, h₁, a₂, …]`，
每三个连续点 `(aᵢ, hᵢ, aᵢ₊₁)` 构成一段二次贝塞尔。颜色与线宽数组按**段**
对齐（长度 = 点数 / 2 向上取整），因此可以给一个 item 的不同段设置不同
颜色/线宽。

```rust,ignore
// 直接用点列构造（默认：白描边 0.02、无填充）
let vitem = VItem::from_vpoints(vec![
    dvec3(0.0, 0.0, 0.0),
    dvec3(1.0, 0.0, 0.0),
    dvec3(0.5, 1.0, 0.0),
]);
```

常用方法：`close()`（闭合路径）、`shrink()`（缩到包围盒中心）、
`get_anchor(idx)`（取第 idx 个 anchor）、`extend_vpoints(...)`（追加，颜色/
线宽数组自动补齐）、`put_start_and_end_on(start, end)`（把首尾移到指定位
置）、`with_normal(...)` / `set_normal(...)`（指定投影平面法向）。

## 渲染语义：平面投影

渲染时假设 `VItem` 的所有点共面，实际渲染的是它在投影平面上的投影
（共面时投影即其本身）。语义细节见 Core Items 的
[VItem](../../understand/core/core_items/vitem.md)。

`normal` 一般保持默认的 `None` 即可：投影平面在渲染时从**当前点数据**推导，
动画中间帧的插值点总是推导出与之一致的法向，不会漂移。反之，显式
`set_normal` 之后，插值就发生在法向量本身上（`Some(a).lerp(Some(b), t)`，
普通线性插值且不重新归一化），不再跟随点数据。因此只在确有需要时才显式
设置，例如点共线/重合等自动推导存在歧义的退化情形，或故意要让非共面点
渲染成投影效果。

## 动画相关 trait

`VItem` 实现了 `Interpolatable`（逐点/逐颜色插值）与 `Alignable`（点数不同
时自动补齐对齐，`morph` 依赖它），因此可以直接：

```rust,ignore
let anim = square.morph(|sq| {
    sq.set_fill_color(manim::BLUE_C);
    sq.shift(DVec3::X * 2.0);
});
```

还实现了 `FillColor` / `StrokeColor` / `StrokeWidth` / `Opacity` /
`Partial`（`get_partial(range)` 截取路径的一段，`Create`/`Write` 动画的
基础）、`PointsFunc`（`apply_points_func` 批量变换点）、`Aabb` 与
`ShiftTransform` / `RotateTransform` / `ScaleTransform`。

`PointVec` 是分量数组的动画包装：对齐时按规则补齐长度，插值逐分量进行。
