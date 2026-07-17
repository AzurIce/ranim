# 渲染架构

---

下面是 Agent 生成内容

本章描述 ranim 从一帧场景状态到图像或视频文件的渲染路径。渲染层的职责是**消费一帧已求值的场景数据**；它不应知道时间线、动画或模拟如何产生这帧数据。

本文同时记录当前实现和期望保持的架构边界。代码调整时，应优先更新本文中“目标边界”和“帧路径”的描述。

## 总览

```text
Main World / CoreItemStore（兼容期）
                │ sync + extract
                ▼
     Render World（由渲染侧持有）
                │ queue / sort / prepare
                ▼
       RenderPackets + GPU resources
                │
                ▼
           RenderGraph
                │
     ┌──────────┴──────────┐
     ▼                     ▼
预览窗口纹理          离屏纹理 + 异步 readback
                              │
                              ▼
                       图片序列 / 视频编码器
```

阶段 1C 已让 `CoreItemStore` 只保留 Main World。Render World、实体映射和 ExtractSchedule 已移入 `ranim-render`，由 `Renderer` 持有并跨帧复用。

## 分层与职责

### 场景提取层

输入是某一时刻的 Main World，输出是渲染侧独立持有的 Render World 当前状态。

- 先同步需要渲染的 Main Entity 与稳定 render root 的生命周期；
- 再通过注册的只读 Query 将场景组件提取为 render component 或独立 primitive entity；
- 保留稳定的来源和 part identity，供增量上传、调试、选择和未来的多视图使用；
- 验证渲染前提，例如每个视图需要一个相机。当前 renderer 直接使用第一个 `CameraFrame`，因此空相机是调用方错误；未来应将这一要求变成 Render World 中显式的视图描述与校验。
- 不执行动画求值、不读取输出 FPS，也不创建 GPU 资源。

目标边界为：

```rust,ignore
renderer.render_store_with_pool(
    &ctx,
    &mut targets,
    clear_color,
    &mut store,
    &mut pool,
);
```

该入口内部先把 Main World Sync/Extract 到 renderer-owned Render World，再执行 Queue、Prepare 和 Render。`extract` 完成后不再借用 Main World，因此后续 GPU 渲染、readback 与编码可以独立进行。Render World 是跨帧复用的 renderer-side scene，不是要求每帧 clone 的一次性完整快照。

### Renderer

`ranim-render::Renderer` 负责 Render World 的 extraction 编排、CPU 到 GPU 的上传、渲染图执行和 command buffer 提交。它或更高一级的 render runner 持有 Render World，以及与分辨率和管线相关的长期资源：渲染图、按类型缓存的 pipeline、合并后的 VItem/MeshItem GPU buffer 和临时 render packet 集合。

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
        └─ reconcile Main World
        ▼
Renderer::render_store_with_pool(...)
        │
        ├─ Sync render roots
        ├─ Extract registered components
        ├─ Queue CameraFrame → ViewportUniform
        ├─ Prepare VItem     → VItemsBuffer
        └─ Prepare MeshItem  → MeshItemsBuffer
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

阶段 1C 已修正 Render World 所有权，并落地了 Queue 工作集（`QueuedFrame`）与帧级上传去重：extraction 仍每帧完整执行，但内容未变化的合并缓冲不再重复上传。Render root 与 primitive entity 会跨帧复用，下一步再做 change detection 驱动的增量 extraction、逐 item 稳定 buffer range 和脏区间上传。

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

当前实际渲染采用合并缓冲路径：一帧内的所有 `VItem` 更新到 `VItemsBuffer`，所有 `MeshItem` 更新到 `MeshItemsBuffer`。`ViewportUniform` 是目前由 render packet 传递的视图数据。以后若加入灯光、材质、实例或多相机，应继续使用“Render World → Queue/Sort → prepared packet/buffer → 节点查询”的方式，避免 RenderGraph 节点直接访问 Main World 或 Render World。

## 离线帧采样与编码

输出层根据 `Output { width, height, fps, format, ... }` 生成采样时刻。当前规则为采样 `0, 1/fps, ...`，并在时长不能被帧间隔整除时额外采样一次精确的结束时刻；这保证最终状态不会因取整丢失。

每个采样时刻的流程是：

1. 求值器把 World 更新到该时刻。
2. render worker 将 Main World 同步、提取到自己持有的 Render World。
3. worker Queue/Prepare 当前渲染工作集，将帧渲染至一个可用的离屏目标并启动异步 readback。
4. 达到缓冲上限时，读取最早的目标并将像素交给编码器。
5. 所有帧提交后，排空剩余 readback 并结束编码。

必须保持“帧索引、采样时间、编码顺序”一致。异步 readback 可以改变完成时间，但不能改变提交给编码器的帧顺序。

## 设计约束

- Sync/Extract 结束后，渲染侧必须拥有独立于 Main World 的当前帧状态；renderer 不得借用 Main World 跨越 GPU/编码异步边界。
- 同一 World 状态和同一渲染配置应产生可重复的帧。随机性、时钟和模拟步进必须在求值层被固定。
- 渲染图节点只通过 `RenderContext`、声明的 packet 和 GPU resource 协作，避免隐式的全局状态。
- 场景数据类型不应泄漏 `wgpu` 类型；GPU 资源只属于 `ranim-render`。
- 同一套 Render World extraction 和单帧渲染入口应服务预览、离线编码、截图和测试，而不是为任一输出方式维护专用场景表示。

## 与求值模型的接口

渲染只要求“当前 Main World 提取出的 Render World 状态”。因此目标调用关系是：

```rust,ignore
runner.seek_or_tick(sample_time)?;
renderer.render_store_with_pool(
    &ctx,
    &mut target,
    clear_color,
    runner.store_mut(),
    &mut pool,
);
```

`seek_or_tick` 可以由纯 Timeline 的随机求值实现，也可以由带 checkpoint 的模拟递推实现；两种差异必须停留在求值层，不能扩散到 renderer。
