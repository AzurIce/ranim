# Items

`ranim-items` 提供**用户层 item**：编写场景时直接构造和做动画的类型。与
core item（见 Core Items 大节）相比，它们用 f64（`DVec3` / `DMat4`）描述、
携带动画所需的辅助结构（如 `PointVec` 对齐包装），并实现了一批动画/变换
trait（`Interpolatable`、`Alignable`、`FillColor`、`ShiftTransform` 等），可以
直接配合 `morph`、`fade_in` 等动画使用。渲染前由 `Extract` 转为 core item
（见 [CoreItem 与 Extract](../understand/core/core_item.md)）。

当前分两类：

- [VItem 类](./vitem/README.md) — `vitem` 模块：二维矢量物件。核心是 `VItem`，
  外加几何构造器（`geometry`）、`SvgItem`、以及 `typst` feature 提供的文字
  物件。
- [MeshItem 类](./mesh/README.md) — `mesh` 模块：三维网格物件。核心是
  `MeshItem`，外加参数曲面 `Surface` 和球体 `Sphere`。

另有 `debug` 模块提供调试辅助（如 `VisualizeAabbItem<T>`：把任意实现了
`Aabb` 的 item 的包围盒可视化为线框矩形）。
