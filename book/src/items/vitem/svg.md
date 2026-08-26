# `SvgItem`

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

从 SVG 构造的矢量物件，定义于 `ranim_items::vitem::svg`。

```rust,ignore
pub struct SvgItem(Vec<VItem>);

let svg = SvgItem::new(svg_str); // svg_str: impl AsRef<str>
```

内部就是一组 `VItem`：SVG 的每个路径解析为一个 `VItem`。因此 extract 时一个
`SvgItem` 会展开为**多个** core `VItem`（1→N），在 `ranim inspect frame` 的
输出里体现为同一个 `animation_id` 下递增的 `part` 序号。
