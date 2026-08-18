# Core `VItem`

2D 矢量图元的渲染表示，定义于 `ranim_core::core_item::vitem`。

```rust,ignore
pub struct VItem {
    /// 投影目标平面的法向；None 时由渲染器从点推导
    pub normal: Option<Vec3>,
    /// 世界空间点列；(x, y, z, is_closed)
    pub points: Vec<Vec4>,
    pub fill_rgbas: Vec<Rgba>,
    pub stroke_rgbas: Vec<Rgba>,
    pub stroke_widths: Vec<Width>,
}
```

## 点列语义

`points` 由用户层 `VItem` 的 vpoints 展开而来：二次贝塞尔路径的 anchor 与
handle 交替排列，每个 `Vec4` 的 `w` 分量是该点是否闭合路径（closepath）的
标记。颜色与线宽数组按**路径段**对齐（段数 = 点数 / 2 向上取整），默认描边
宽度为 `DEFAULT_STROKE_WIDTH = 0.02`。

## 平面投影渲染

渲染 core `VItem` 时，Ranim 假设所有点共面以计算深度，实际渲染的是它在某个
平面上的**投影**：

- 投影平面的初始基为 `(X, Y)`、法向为 `Z`，且包含点列的第一个点；
- `normal` 为 `Some` 时使用指定的投影平面；
- `normal` 为 `None` 时由 `vitem_normal_from_points` 在渲染时推导：先对 anchor
  点做 Newell 法（鞋带公式的 3D 形式）求面积法向；面积退化（如单段曲线）时
  扫描全部点寻找非共线三元组；点共线时取一个包含该直线的确定性平面；所有点
  重合时回退到 `Z` 轴。

因此正常使用应保证一个 core `VItem` 的点共面（此时投影即其本身）；故意打破
共面则得到的是投影效果。
