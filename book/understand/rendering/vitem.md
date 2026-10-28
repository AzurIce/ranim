# `RenderVItem`

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

矢量图元的渲染组件，需要 items 的 `render` feature，定义于 `ranim_items::vitem::render`。注意点是三维的
（局部空间），`transform` 给出局部到世界的变换。「共面」是渲染时的假设
（见下文「平面投影渲染」）。

```rust,ignore
pub struct RenderVItem {
    /// 投影目标平面的法向；None 时由渲染器从点推导
    pub normal: Option<Vec3>,
    /// 局部空间点列；(x, y, z, is_closed)
    pub points: Vec<Vec4>,
    /// 局部到世界的变换
    pub transform: Mat4,
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

通常由逻辑 `VItem` 的值级 `Extract` 生成该组件，并将
`ExtractContext` 累积的父变换与已有 `transform` 组合，写入 render world；GPU
在平面基重建之后应用矩阵。`ranim_items::vitem::VItemPlugin` 安装裸逻辑
`VItem`（及 `Vec<VItem>`、几何、SVG、文字输入）的提取与 GPU 管线；默认渲染
配置已包含它，自定义 `RenderSetup` 时经 `RanimItemsPlugin` 或单独安装。变换
wrapper 或未被覆盖的容器另注册实际完整类型的
`ExtractPlugin<T>`。同一输入重复渲染不会修改帧中的点或矩阵。

## 平面投影渲染

渲染 `RenderVItem` 时，Ranim 假设所有点共面以计算深度，实际渲染的是它在某个
平面上的**投影**：

- 投影平面的初始基为 `(X, Y)`、法向为 `Z`，且包含点列的第一个点；
- `normal` 为 `Some` 时使用指定的投影平面；
- `normal` 为 `None` 时由 `vitem_normal_from_points` 在渲染时推导：先对 anchor
  点做 Newell 法（鞋带公式的 3D 形式）求面积法向；面积退化（如单段曲线）时
  扫描全部点寻找非共线三元组；点共线时取一个包含该直线的确定性平面；所有点
  重合时回退到 `Z` 轴。

因此正常使用应保证一个 `RenderVItem` 的点共面（此时投影即其本身）；故意打破
共面则得到的是投影效果。
