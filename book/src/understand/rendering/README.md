# 渲染表示与物件插件

`ranim-core` 负责动画求值、类型化帧与通用变换，独立于具体物件及 renderer。
`ranim-items` 持有内置物件：逻辑 `VItem` / `MeshItem` 使用 f64 数据与动画辅助
结构，启用 `render` feature 后，分别转换到同模块下的渲染组件。组件、GPU
缓冲、管线和系统归对应物件模块所有。

- [RenderVItem](./vitem.md)：`ranim_items::vitem::render::RenderVItem`，使用 f32
  局部几何与 `Mat4` 渲染变换，供矢量路径管线使用。
- [RenderMeshItem](./mesh_item.md)：`ranim_items::mesh::render::RenderMeshItem`，
  使用 f32 顶点、法线、矩阵与三角形索引，供网格管线使用。
- [CameraFrame](../../items/camera.md)：`ranim_items::camera::CameraFrame`，保留 f64
  相机参数；渲染插件把它用于视图和投影配置。

默认渲染配置已包含三个内置物件插件——`ranim_items::RanimItemsPlugin` 捆绑
`CameraFramePlugin`、`VItemPlugin`、`MeshItemPlugin`：相机插件注册裸相机提取与
viewport 准备，矢量/网格插件注册各自族内裸逻辑类型的值级提取和 GPU 渲染系统。
项目未注册 `RenderSetup` 时，output / preview 回退到该默认配置。需要渲染容器、
变换 wrapper 或自定义类型时，项目把自定义装配保存为 App 的
`ranim_render::RenderSetup` resource：回调中安装 `RanimItemsPlugin`（或按需单个
安装三个物件插件——两种写法互斥，Bevy 插件唯一，重复安装会 panic），并为完整
类型另加 `ExtractPlugin<T>`。output / preview 从项目 App 取得该 setup，经
`ranim_render::install_render_app` 应用后以 `RanimRenderApp` 标签挂为项目 App
的渲染 SubApp，每个 GPU runtime 独立重放；提取靠 bevy SubApp 的提取钩子——
每次 `App::update()` 主 schedule 结束后，主世界的 `EvaluatedFrame` 被交换进渲染
世界。`ranim-render` 本体不再持有 `Renderer` 结构体，只提供通用提取上下文、
schedule、渲染目标和提交生命周期（`build_render_app` / `install_render_app` /
`RenderAppExt`），不依赖这些具体组件。

完整输入类型的注册与递归提取见[完整求值帧与渲染提取](../core/extract.md)。
逻辑物件只需 CPU features 即可用于求值；渲染组件及插件通过 items 的 `render`
feature 提供。`RenderVItem` / `RenderMeshItem` 是 render world 的数据组件，不
实现动画插值、样式操作或值级 `Extract`；普通动画输出使用逻辑物件。
