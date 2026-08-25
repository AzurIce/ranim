# MeshItem 类

`ranim_items::mesh` 模块：三维网格物件。这一类物件渲染的是真正的 3D 三角
网格（每顶点颜色/法线，法线全零时 flat shading），语义细节见 Core Items 的
[MeshItem](../../understand/core/core_items/mesh_item.md)。

成员：

- [MeshItem](./mesh_item.md) — 核心类型：顶点 + 索引 + 每顶点数据；外部变换通常使用 `Transformed<MeshItem, DAffine3>`。
- [Surface](./surface.md) — 参数曲面：`(u, v)` 网格采样生成网格。
- [Sphere](./sphere.md) — 球体便捷构造。

选择建议：

- 规则几何体（球、参数曲面）：用 `Sphere` / `Surface` 构造；
- 任意几何（自定义多面体、模型）：直接拼 `MeshItem` 的顶点与索引；
- 平滑曲面记得 `with_smooth_normals()`；硬边物体保持法线全零走 flat
  shading 即可。
