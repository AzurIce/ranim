# VItem 类

`ranim_items::vitem` 模块：矢量物件。这类物件的点本来就是三维点，可以任意
摆放、旋转在 3D 空间中；只是渲染时假设单个 item 的所有点共面，实际渲染的
是它在投影平面上的投影（共面时投影即其本身），语义细节见 Core Items 的
[VItem](../../understand/core/core_items/vitem.md)。

成员：

- [VItem](./vitem.md) — 核心类型：二次贝塞尔路径 + 描边/填充，所有同类物件
  最终都转化为它。
- [几何构造器](./geometry.md) — `Circle`、`Square`、`Arc` 等数据 struct，
  可直接 `VItem::from(...)`。
- [SvgItem](./svg.md) — 从 SVG 构造。
- [文字物件](./text.md) — `TextItem` / `TypstText`（`typst` feature）。
