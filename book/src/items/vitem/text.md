# 文字物件

`vitem::text` 与 `vitem::typst` 提供文字物件，需要启用 `typst` feature。

## `TextItem`

简单文字（`vitem::text`）：

```rust,ignore
let text = TextItem::new("Hello Ranim", 1.0); // 文本与 em 字号
```

字体通过 `TextFont` 配置：

```rust,ignore
let font = TextFont::new(["Noto Sans CJK SC", "serif"]); // 按序回退的字体族
```

## `TypstText`

Typst 排版（`vitem::typst`），支持行内/多行代码与数学公式：

```rust,ignore
let formula = TypstText::new("$ integral_0^1 x^2 dif x $");
let code = TypstText::new_inline_code("let x = 1;");
let block = TypstText::new_multiline_code("fn main() {}", Some("rust"));
```

两类文字物件都经 Typst 排版为矢量轮廓，extract 时 1→N 展开为多个 core
`VItem`。
