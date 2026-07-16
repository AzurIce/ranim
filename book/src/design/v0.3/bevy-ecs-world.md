1. 将 ranim 的 Store 底层重构为 bevy_ecs 的 World，且重构相应的渲染 extract 等逻辑。这一阶段的目标就是不改动全部外部接口的情况下更换我们底层的 store。
2. 我们的动画现在只有 eval_alpha 语义，但是我们改为 seek 和 tick，我们在这个基础之上构思如何避免现在的 Eval<T> 会导致频繁的内存分配（这块可能就需要借助 World，在 seek 时同步 world 物件（spawn despawn 之类的），然后 tick 时原地修改。
3. 然后我们再去构思模拟式的动画 API 要怎么设计。

---

# 使用 bevy_ecs 重构 World

> 状态：构思中。本页描述 v0.3 第一阶段的底层存储迁移。此阶段以替换内部实现为目标，不同时重写 Animation、Timeline 和用户编码 API。

## 阶段一：CoreItem ECS 化

阶段一只替换当前 `CoreItemStore` 的真实存储和 renderer 的读取方式，不让高层 `Square`、`Text`、`Group` 等作者 Item 直接进入 World。现有 Timeline 仍然求值并提取出 `CoreItem`，每个 CoreItem occurrence 再被物化为一个 ECS entity。

```text
Timeline::eval_at_sec(t)
        │
        ▼
Iterator<((timeline_id, animation_id), CoreItem)>
        │ reconcile
        ▼
bevy_ecs::World
  ├─ Entity(CoreItemKey, CoreItemOrder, CameraFrame)
  ├─ Entity(CoreItemKey, CoreItemOrder, VItem)
  └─ Entity(CoreItemKey, CoreItemOrder, MeshItem)
        │ extract
        ▼
Renderer
```

`(timeline_id, animation_id)` 不能单独作为 entity key：一个动画可以提取出多个 CoreItem。因此阶段一使用内部 key：

```rust,ignore
struct CoreItemKey {
    source: (usize, usize),
    part: usize,
}
```

`part` 是同一 source 在当前提取结果中的 occurrence index。连续帧中只要提取顺序稳定，同一 part 就会原地更新；新增和消失的 part 分别 spawn/despawn。

ECS 的 query/迭代顺序不构成渲染顺序保证，因此每个实体还记录当前帧的 `CoreItemOrder`。renderer 必须按该顺序提取，保持迁移前数组顺序和透明对象绘制结果不变。

阶段一完成后：

- `CoreItemStore` 内部唯一真实场景存储是 `bevy_ecs::World`；
- `CameraFrame`、`VItem` 和 `MeshItem` 是 ECS Component；
- renderer 不再直接读取 `CoreItemStore` 的公开 Vec；
- `CoreItem` enum 暂时保留为 Timeline 求值到 World reconciliation 的兼容输入；
- Main World / Render World 的双 World extraction 留到后续小阶段，不在本次迁移中强制完成。

### 目标

当前 `CoreItemStore` 按类型维护 `CameraFrame`、`VItem` 和 `MeshItem` 数组，并在每次求值后清空、重新填充。第一阶段计划使用 `bevy_ecs::World` 作为底层场景存储，利用其实体生命周期、组件查询和 change detection，为后续动画原地更新、模拟系统和 Editor 打下基础。

这一阶段应保持以下外部行为不变：

- `RanimScene`、`Timeline` 和 `AnimationCell` 的作者 API 暂不调整；
- `SealedRanimScene::eval_at_sec` 仍以当前语义求值；
- preview、离线渲染、截图与编码结果保持一致；
- `CameraFrame`、`VItem`、`MeshItem` 等用户数据类型继续直接使用；
- 不要求用户接触 `bevy_ecs::Entity`、Query 或 System。

因此这一步是存储和渲染输入边界的重构，而不是 v0.3 新动画 API 的一次性落地。

### 依赖范围

第一阶段只引入 `bevy_ecs`，不引入 `bevy_render`。

```text
ranim-core
  └─ bevy_ecs::World

ranim-render
  └─ 继续使用现有 wgpu、RenderGraph、RenderPool 和 RenderPackets
```

`bevy_ecs` 提供 World、Entity、Component、Resource、Query、Schedule 和 change tick。渲染层仍由 ranim 控制，以避免被 Bevy 的 App、Asset、RenderDevice、wgpu 版本和 RenderGraph 生命周期绑定。

### 兼容存储层

迁移初期可以保留 `CoreItemStore` 名称，让它成为 `bevy_ecs::World` 的兼容封装：

```rust,ignore
pub struct CoreItemStore {
    world: bevy_ecs::world::World,
    entities: HashMap<CoreItemKey, Entity>,
    ordered_entities: Vec<Entity>,
}
```

`CoreItemKey` 由现有 `(timeline_id, animation_id)` 加同一 source 内的 part index 组成。它不是未来最终的对象身份，只用于让现有求值结果能够在连续帧之间找到对应的 ECS entity。`ordered_entities` 只保存当前帧的显式顺序，不保存 Item 数据。

现有入口可以暂时保留：

```rust,ignore
impl CoreItemStore {
    pub fn update(
        &mut self,
        items: impl Iterator<Item = (CoreItemSourceId, CoreItem)>,
    ) {
        // reconcile entities and components
    }
}
```

但 `update` 的内部语义由“清空 Vec 后重新填充”改为 reconciliation：

1. 输入中存在且 World 中已有的 ID，原地更新相应组件；
2. 输入中新增的 ID，spawn 新 entity；
3. 上一帧存在但本帧消失的 ID，despawn 对应 entity；
4. 记录组件变化，供渲染 extract 决定是否需要重新准备数据。

这样即使第一阶段仍由 `eval_at_sec` 产生完整帧，底层也已经具备稳定的帧间存储位置。

### ECS 组件表示

第一版可以直接将现有核心类型作为组件：

```rust,ignore
world.spawn((key, CoreItemOrder(order), CameraFrame::default()));
world.spawn((key, CoreItemOrder(order), vitem));
world.spawn((key, CoreItemOrder(order), mesh_item));
```

必要的内部组件包括：

```rust,ignore
struct CoreItemKey {
    source: (usize, usize),
    part: usize,
}

struct CoreItemOrder(usize);
```

初期不必立即把 `VItem` 和 `MeshItem` 拆成细粒度 ECS 组件。先以完整对象组件完成等价迁移，可以减少改动范围；等动画 seek/tick 与原地修改模型明确后，再评估是否拆分 Transform、Geometry、Style、Visibility 等组件。

### 渲染提取

迁移后的渲染数据路径为：

```text
SealedRanimScene::eval_at_sec(t)
        │
        ▼
CoreItemStore::update(...)
        │ reconcile
        ▼
bevy_ecs::World
        │ extract queries
        ▼
ExtractedFrame / RenderWorld
        │ prepare
        ▼
RenderPool + merged buffers
        │ queue
        ▼
RenderPackets
        │
        ▼
RenderGraph
```

第一版 extractor 通过 `CoreItemStore` 的有序迭代器借用 World 中的组件，并直接更新 renderer 的合并缓冲：

```rust,ignore
let camera = store.camera_frames().next();
vitems_buffer.update(ctx, store.vitems());
mesh_items_buffer.update(ctx, store.mesh_items());
```

这一版不会为了适配 ECS 再 clone 一份 VItem/MeshItem；renderer 直接借用组件内容完成 GPU buffer packing。之后再利用 `Changed<T>`、资源 handle 和可复用 buffer，把 extraction 改为增量同步。

长期可以让 RenderWorld 同样使用另一个 `bevy_ecs::World`：

```text
Main World (bevy_ecs)
    │ Extract Schedule
    ▼
Render World (bevy_ecs)
```

但这不是第一版迁移的强制条件。第一版应优先验证 Main World 存储替换和现有 renderer 的兼容性。

### RenderPackets 的位置

`RenderPackets` 继续表示当前帧供 RenderGraph 查询的 packet handle 集合，不承担 World 的职责。

```text
World / RenderWorld：场景和渲染实体状态
RenderPool：准备并复用具体 RenderResource
RenderPackets：本帧参与渲染的 packet 索引
RenderGraph：消费 packet 并编码 GPU 命令
```

现有 `VItemsBuffer` 和 `MeshItemsBuffer` 继续作为跨帧复用的 prepared GPU resources。ECS migration 不要求重写现有 OIT 和合并缓冲管线。

### 与下一阶段的连接

Store 迁移后，Animation 的 `seek` 和 `tick` 可以直接面向 World 中已物化的对象工作：

```text
seek(t)
  ├─ 确定当前有效 Animation
  ├─ spawn 新出现的对象
  ├─ despawn 已离开有效区间的对象
  └─ 将对象同步到 t 时刻的完整状态

tick(dt)
  ├─ 复用已有 Entity 和组件存储
  └─ 原地修改当前对象，尽量复用 Vec/GPU buffer 容量
```

这一阶段暂不决定最终的 `Animation` trait，只保证底层 World 能支持这两种访问模式。模拟式动画 API 则在纯动画 seek/tick 的行为、对象生命周期和内存模型验证后再设计。

### 第一阶段验收条件

- 现有 examples 无需修改即可构建和渲染；
- 相同采样时间产生与迁移前一致的 `CameraFrame`、`VItem` 和 `MeshItem`；
- `CoreItemStore` 不再以多个公开 Vec 作为真实存储；
- 连续帧可以按 ID 更新、spawn 和 despawn ECS entity；
- renderer 通过 extract 结果工作，而不直接依赖 Timeline；
- native preview、离线输出和 WASM 路径保持可用；
- 为 entity reconciliation、组件更新和删除补充测试；
- 记录迁移前后的求值时间、分配次数和渲染上传量，作为 seek/tick 重构的基线。

### 当前实现状态

- 已引入最小 `bevy_ecs` 依赖，不包含 `bevy_render`；
- `CoreItemStore` 已以 `bevy_ecs::World` 作为真实存储；
- `CameraFrame`、`VItem` 和 `MeshItem` 已实现 Component；
- 已实现 `{ source, part }` reconciliation、跨帧 Entity 复用和 stale entity despawn；
- 已通过显式顺序保持 renderer 输入次序；
- renderer 已通过有序借用迭代器读取 ECS component，不再依赖公开 Vec；
- 已覆盖一对多 part、类型切换、顺序、despawn 和 Store clone 测试；
- 尚未实现独立 RenderWorld、Changed-based 增量 GPU prepare 和性能基准对比。

### 暂不处理

- 不在这一阶段公开 `bevy_ecs` System API；
- 不把 `RanimScene` 改成新的 `Ranim` 顶级结构；
- 不重新设计 `play`、TimeCursor 或动画组合；
- 不引入 Simulation；
- 不要求任意 ECS World 可以 clone 或 snapshot；
- 不引入 `bevy_render`；
- 不立即将全部 item 拆成细粒度组件或 asset handle。
