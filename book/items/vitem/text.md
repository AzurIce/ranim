# 文字物件

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

`vitem::text` 与 `vitem::typst` 提供文字物件，需要启用 `typst` feature。

## `TextItem`

简单文字（`vitem::text`）：

```rust
let text = TextItem::new("Hello Ranim", 1.0); // canonical local 文本与 em 字号
let placed = text.transformed(Translation(dvec3(1.0, 2.0, 0.0)));
```

字体通过 `TextFont` 配置：

```rust
let font = TextFont::new(["Noto Sans CJK SC", "serif"]); // 按序回退的字体族
```

## `TypstText`

Typst 排版（`vitem::typst`），支持行内/多行代码与数学公式：

```rust
let formula = TypstText::new("$ integral_0^1 x^2 dif x $");
let code = TypstText::new_inline_code("let x = 1;");
let block = TypstText::new_multiline_code("fn main() {}", Some("rust"));
```

两类文字物件都经 Typst 排版为矢量轮廓。它们的值级 `Extract` 在渲染侧生成
多个 `RenderVItem`，输出子序号保留字形与图层顺序。直接以文字类型作为动画
输出时，整段文字仍只占一个帧条目；`vitems()` 可以借用其矢量轮廓切片。

项目需开启 `typst` 与 `render`。`ranim_items::vitem::VItemPlugin` 同时注册
裸 `TextItem` 与 `TypstText` 的提取，默认渲染配置（`RanimItemsPlugin`）已包含
它，直接输出裸文字类型无需装配；上例的 `Transformed<TextItem, Translation>` 等
完整类型仍需自定义 `RenderSetup`，注册精确类型的
`ranim_render::extract::ExtractPlugin<T>`。
项目插件将这份 setup 提供给 preview / output，在每次创建 renderer 时重放。
