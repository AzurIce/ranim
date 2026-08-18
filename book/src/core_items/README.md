# Core Items

Core item 是渲染器直接消费的三种 primitive，定义在 `ranim-core` 的
`core_item` 模块，即 [`CoreItem`](../understand/core/core_item.md) 枚举的三个
变体。与用户层 item（见 Items 大节）相比，它们：

- 数据为 f32（`Vec3` / `Vec4` / `Mat4`），位于世界空间，可直接进入渲染管线；
- 不携带动画辅助结构（如 `PointVec` 对齐包装）；
- 每种对应一条渲染路径：2D 矢量、3D 网格、相机。

用户通常不直接构造 core item，而是使用 `ranim-items` 中的用户层 item，由
`Extract` 自动转换。

- [CameraFrame](./camera_frame.md) — 相机：视图/投影参数与正交-透视混合。
- [VItem](./vitem.md) — 2D 矢量图元：二次贝塞尔路径 + 描边/填充，按投影
  平面渲染。
- [MeshItem](./mesh_item.md) — 3D 三角网格：顶点、索引、变换与每顶点
  颜色/法线。
