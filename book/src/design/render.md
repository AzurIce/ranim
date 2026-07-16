# 渲染架构

---

下面是 Agent 生成内容

本章描述 ranim 从一帧场景状态到图像或视频文件的渲染路径。渲染层的职责是**消费一帧已求值的场景数据**；它不应知道时间线、动画或模拟如何产生这帧数据。

本文同时记录当前实现和期望保持的架构边界。代码调整时，应优先更新本文中“目标边界”和“帧路径”的描述。

## 总览

```text
World（未来）/ CoreItemStore（当前）
                │ emit / extract
                ▼
          RenderFrame
                │
                ▼
     Renderer::render_frame(...)
                │
     ┌──────────┴──────────┐
     ▼                     ▼
预览窗口纹理          离屏纹理 + 异步 readback
                              │
                              ▼
                       图片序列 / 视频编码器
```

目前 `CoreItemStore` 是实际的帧输入，按类型保存 `CameraFrame`、`VItem` 与 `MeshItem`。它由求值层在每帧重新填充。未来 `World::emit()` 应产生一个等价的、面向渲染的 `RenderFrame`；`CoreItemStore` 可以成为其内部表示或过渡实现，但不应继续承担场景状态的角色。

## 分层与职责

### 场景提取层

输入是某一时刻的场景状态，输出是仅包含渲染所需数据的帧快照。

- 将 World 中的实体和组件提取为 primitive；当前 primitive 为 `CoreItem`。
- 保留稳定的实体身份，供增量上传、调试、选择和未来的多视图使用。
- 验证渲染前提，例如每个视图需要一个相机。当前 renderer 直接使用 `store.camera_frames[0]`，因此空相机是调用方错误；未来应将这一要求变成显式的 `RenderFrame` 校验或视图描述。
- 不执行动画求值、不读取输出 FPS，也不创建 GPU 资源。

建议的边界为：

```rust,ignore
pub trait EmitRenderFrame {
    fn emit(&self, frame: &mut RenderFrame);
}

renderer.render_frame(&ctx, &mut targets, clear_color, &frame, &mut pool);
```

### Renderer

`ranim-render::Renderer` 负责 CPU 到 GPU 的上传、渲染图执行和提交 command buffer。它持有与分辨率和管线相关的长期资源：渲染图、按类型缓存的 pipeline、合并后的 VItem/MeshItem GPU buffer，以及临时 render packet 集合。

每次 `render_frame` 应仅渲染一次已给定的帧，不产生磁盘副作用。当前对应入口是 `Renderer::render_store_with_pool`。

### 输出层

输出层拥有目标尺寸、FPS、文件格式和编码策略。

- 预览：渲染到 GUI 可展示的纹理；时间变化时重新求值并渲染。
- 离线渲染：为每个采样时刻取得一帧，渲染到离屏纹理，异步读回 CPU，再交给图片写入或 ffmpeg 编码器。
- `buffer_count` 决定可并行在飞的离屏目标数量。当前 worker 在目标全部占用时读取最早提交的帧，从而让 GPU 渲染与 CPU 编码重叠。

Renderer 不应拥有“总时长”“第几帧”或“时间标记”等时间线概念；这些属于求值和输出调度。

## 当前单帧路径

当前离线路径的关键顺序如下：

```text
SealedRanimScene::eval_at_sec(t)
        │
        ▼
CoreItemStore::update(items)
        │
        ▼
Renderer::render_store_with_pool(...)
        │
        ├─ CameraFrame → ViewportUniform
        ├─ VItem       → VItemsBuffer
        └─ MeshItem    → MeshItemsBuffer
                 │
                 ▼
             RenderGraph
                 │
                 ▼
       RenderTextures（颜色 + 深度）
                 │
                 ▼
      start_readback / finish_readback → FileWriter
```

`CoreItemStore::update` 会清空并重建所有按类型的数组。因此当前路径是“每帧完整提取、完整更新”的模型；虽然 GPU buffer 自身可复用，但还未以 World entity 为单位做增量同步。

## GPU 资源与帧目标

每个 `RenderTextures` 目前包含：

- 一张 `Rgba8UnormSrgb` 的颜色目标纹理，同时支持 render attachment、texture binding、copy 和 CPU readback；
- 一张 `Depth32Float` 的深度纹理；
- 颜色纹理的 sRGB 和线性 view；
- 供 OIT resolve 读取深度的 bind group。

`ResolutionInfo` 保存与分辨率相关的 uniform/storage buffer，包括 OIT 使用的颜色与深度数据。输出尺寸变化时，应该重建这些与尺寸绑定的资源，而 scene/world 本身不应受到影响。

## 默认渲染图

默认 `Renderer` 建立的全局图为：

```text
Clear
  │
  ▼
ViewRenderGraph
  ├─ MergedVItemCompute
  ├─ MergedVItemDepth
  ├─ MergedMeshItemDepth
  ├─ MergedVItemColor
  └─ MergedMeshItemColor
  │
  ▼
OITResolve
```

主要依赖关系是：VItem compute 先于其深度阶段；VItem 与 MeshItem 的深度阶段先于相应颜色阶段；两类颜色阶段都等待另一类的深度结果。最后 `OITResolve` 将透明度累积结果解析到颜色目标。全局图节点通过声明其所需的 `RenderPackets` 类型获得输入，视图图则针对每个 viewport 执行。

当前实际渲染采用合并缓冲路径：一帧内的所有 `VItem` 更新到 `VItemsBuffer`，所有 `MeshItem` 更新到 `MeshItemsBuffer`。`ViewportUniform` 是目前由 render packet 传递的视图数据。以后若加入灯光、材质、实例或多相机，应继续使用“帧数据 → 明确 packet/buffer → 节点查询”的方式，避免节点直接访问 World。

## 离线帧采样与编码

输出层根据 `Output { width, height, fps, format, ... }` 生成采样时刻。当前规则为采样 `0, 1/fps, ...`，并在时长不能被帧间隔整除时额外采样一次精确的结束时刻；这保证最终状态不会因取整丢失。

每个采样时刻的流程是：

1. 求值器把 World 更新到该时刻。
2. World emit 一帧渲染数据。
3. worker 将帧渲染至一个可用的离屏目标并启动异步 readback。
4. 达到缓冲上限时，读取最早的目标并将像素交给编码器。
5. 所有帧提交后，排空剩余 readback 并结束编码。

必须保持“帧索引、采样时间、编码顺序”一致。异步 readback 可以改变完成时间，但不能改变提交给编码器的帧顺序。

## 设计约束

- 渲染输入是一个完整、可独立消费的帧快照；renderer 不得借用可变 World 跨越 GPU/编码异步边界。
- 同一 World 状态和同一渲染配置应产生可重复的帧。随机性、时钟和模拟步进必须在求值层被固定。
- 渲染图节点只通过 `RenderContext`、声明的 packet 和 GPU resource 协作，避免隐式的全局状态。
- 场景数据类型不应泄漏 `wgpu` 类型；GPU 资源只属于 `ranim-render`。
- `RenderFrame` 应能服务预览、离线编码、截图和测试，而不是为任一输出方式专门设计。

## 与求值模型的接口

渲染只要求“当前 World 的一帧”。因此理想调用关系是：

```rust,ignore
let mut world = runner.world_at(sample_time)?;
let frame = world.emit();
renderer.render_frame(&frame);
```

`world_at` 的实现可以是纯 Timeline 的随机求值，也可以是带 checkpoint 的模拟递推；两种差异必须停留在求值层，不能扩散到 renderer。
