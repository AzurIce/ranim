# `SvgItem`

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

从 SVG 构造的矢量物件，定义于 `ranim_items::vitem::svg`。

```rust,ignore
pub struct SvgItem(Transformed<Node<VItem>, DAffine3>);

let svg = SvgItem::new(svg_str); // svg_str: impl AsRef<str>
```

内部按 SVG 分组保存放置树：每条路径是 `VItem` payload，组变换保存在边上的
`Transformed` 中。`tree()` / `tree_mut()` / `into_tree()` 提供树访问，
`by_id` 可按 SVG 元素 ID 寻址。

动画直接输出 `SvgItem` 时，它在类型化帧中占一项，内部放置树完整保留。
注册 `ranim_render::extract::ExtractPlugin<SvgItem>` 后，值级 `Extract` 按深度
优先顺序生成多个 `RenderVItem` 实体，并通过 `ExtractContext` 累积树内变换。
若输出是外层 `Transformed<SvgItem, G>`，需注册这个完整类型，其变换也通过
同一个上下文组合。每个实体使用同一个来源 `order` 和不同的输出子序号，再
统一解析为 `SceneOrder`。
`inspect frame` 的 `part` 仍表示这个 SVG 输入的位置，多个图元保存在
它的 CPU 投影分组中。
