# Items

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

`ranim-items` 提供**用户层 item**：编写场景时直接构造和做动画的类型。与
渲染表示（见渲染表示大节）相比，它们用 f64（`DVec3` / `DAffine3`）描述、
携带动画所需的辅助结构（如 `PointVec` 对齐包装），并实现了一批动画/变换
trait（`Interpolatable`、`Alignable`、`FillColor`、`ShiftTransform` 等），可以
直接配合 `morph`、`fade_in` 等动画使用。完整的动画输出原样保存在类型化帧中。
启用 items 的 `render` feature 后，各物件实现值级 `Extract`；默认渲染配置
已包含内置物件插件（`RanimItemsPlugin`），渲染裸内置类型无需装配。渲染未被
覆盖的完整输出（容器、变换 wrapper、自定义类型）时，项目注册自定义
`RenderSetup`，为实际完整输出安装 `ExtractPlugin<T>`；容器与变换 wrapper
在渲染侧递归解释（见 [完整求值帧与渲染提取](../understand/core/extract.md)）。

可见几何分两类：

- [VItem 类](./vitem/README.md) — `vitem` 模块：矢量物件。核心是 `VItem`，
  外加几何构造器（`geometry`）、`SvgItem`、以及 `typst` feature 提供的文字
  物件。
- [MeshItem 类](./mesh/README.md) — `mesh` 模块：三维网格物件。核心是
  `MeshItem`，外加参数曲面 `Surface` 和球体 `Sphere`。

[CameraFrame](./camera.md) 定义相机数据与投影参数，`CameraFramePlugin` 负责其
渲染提取与 viewport 准备（默认渲染配置已包含）。

另有 `debug` 模块提供调试辅助（如 `VisualizeAabbItem<T>`：把任意实现了
`Aabb` 的 item 的包围盒可视化为线框矩形）。
