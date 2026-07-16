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

## 阶段 1B：Main World、Render World 与分阶段渲染

> 阶段结论：1B 是用于验证“任意 Item Component、1:N extraction、独立 Render Entity 和 Prepare/Queue 分层”是否可行的纵向原型。原型已经证明这些机制可以接入现有 renderer，但其中“`CoreItemStore` 同时持有 Main World 与 Render World”的所有权设计不再继续沿用。正式边界由阶段 1C 修正。

阶段 1A 只把 CoreItem 数组换成 ECS 存储。阶段 1B 引入真正独立的 Main World 和 Render World，并将渲染路径拆成 Extract、Prepare、Queue、Render 四个阶段。

```text
Main World<Item Components>
        │ Extract
        ▼
Render World<Primitive Components>
        │ Prepare
        ▼
VItemsBuffer / MeshItemsBuffer / GPU resources
        │ Queue
        ▼
RenderPackets
        │ Render
        ▼
RenderGraph
```

### Item 与 Component

可渲染 Item 是 Main World 中的 Component。World 可以插入任意注册了渲染提取逻辑的组件类型，而不是只能保存 `CoreItem`：

```rust,ignore
world.insert_item(Square::new());
world.insert_item(Text::new(...));
world.insert_item(custom_item);
```

不是每个 Component 都必须可渲染；速度、约束、标签和模拟状态等组件可以只参与 Main World 逻辑。

当前 `Vec<T>` 不能直接实现外部 crate 的 `Component`，后续需要使用 ranim 自己的 `Group<T>`，或由插入 API 在内部包装。阶段 1B 的纵向切片先覆盖单个 Item Component。

### 新 Extract 边界

旧 `Extract<Target = CoreItem>` 暂时保留给 Timeline 兼容求值。阶段 1B 新增面向 ECS 的提取 trait：

```rust,ignore
pub trait ExtractToRenderWorld: Component {
    type RenderItem: Component;

    fn extract_to_render_world(
        &self,
        output: &mut Vec<Self::RenderItem>,
    );
}
```

一个 Main Item 可以输出零到多个同类 RenderItem。不同 Item 类型可以输出不同 RenderItem 类型，ECS 本身负责异构存储，因此不再需要统一包进 `CoreItem` enum。

长期目标是让这一 trait 取代旧 `Extract<Target = CoreItem>`。过渡期保留两个名字，避免在 Store 架构尚未验证前同时重写全部 Animation 和 Item 实现。

### 独立 Render Entity

每个提取结果在 Render World 中保留独立 Entity：

```text
Main Entity<Text>
  ├─ Render Entity<VItemPrimitive> part 0
  ├─ Render Entity<VItemPrimitive> part 1
  └─ Render Entity<VItemPrimitive> part 2
```

映射键为：

```rust,ignore
struct RenderItemKey {
    source: Entity,
    extractor: ExtractorId,
    part: usize,
}
```

Render Entity 同时记录 `ExtractedFrom` 和 `RenderItemOrder`。连续 extraction 会原地更新已有 part、spawn 新 part、despawn 消失 part。

独立 Render Entity 必须保持到 Prepare 之前，不能在 Extract 阶段直接跳到 `VItemsBuffer`。否则会过早丢失来源映射、可见性、bounds、picking、change tick、多视图过滤和增量更新所需的信息。

### 注册 Extractor

ECS 无法自动发现全部实现了提取 trait 的 Component 类型。Ranim World wrapper 在 `insert_item<T>()` 时按 `TypeId` 自动注册泛型 extractor，并提供显式注册入口：

```rust,ignore
world.register_item::<CustomItem>();
```

直接绕过 wrapper 操作内部 `bevy_ecs::World` 的高级用户，需要自行确保对应 extractor 已注册。

### Prepare 与 Queue

Prepare 查询独立 Render Entity，并按 primitive 类型构造或更新聚合 GPU buffer：

```text
Render Entity<VItemPrimitive> × N
        │ Prepare
        ▼
Prepared VItemsBuffer
```

第一版仍可完整 packing，但阶段边界必须独立。后续再利用 `Changed<T>`、稳定 buffer range 和资源 handle 做增量上传。

Queue 不构造几何 buffer，只根据 View、可见性、顺序和 prepared resource 生成本帧 `RenderPackets`。RenderGraph 继续只消费 queued packet 和 prepared resource。

### 阶段 1B 纵向切片

第一版实现范围：

- `CoreItemStore` 内部分为 Main World 与 Render World；
- 新增可注册的 `ExtractToRenderWorld`；
- `CameraFrame`、core `VItem`、core `MeshItem` 作为首批 Main Item 和 RenderItem；
- 现有 `CoreItemStore::update` 先 materialize 到 Main World，再统一 extract 到 Render World；
- renderer 只读取 Render World；
- `VItemsBuffer`、`MeshItemsBuffer` 的更新归入明确的 Prepare 函数；
- viewport packet 和本帧工作集生成归入 Queue 函数；
- 保持现有 Timeline、Animation 和用户 examples 不变。

纵向切片验证后，再逐步让 `Square`、`Text`、`Group` 和自定义 Item 直接成为 Main World Component，并删除 `CoreItem` 兼容层。

### 阶段 1B 当前实现状态

- `CoreItemStore` 已内含相互独立的 Main World 与 Render World；
- 已新增 `ExtractToRenderWorld`，支持任意注册的 Component 输出零到多个同类 RenderItem；
- `insert_item<T>()` 会自动注册 extractor、spawn Main Entity 并执行 extraction；
- Render Entity 使用 `{ source, extractor, part }` 做跨帧 reconciliation；
- Render World 保留独立 Entity、`ExtractedFrom` 和显式顺序；
- renderer 已提供直接消费 Render World 的入口；
- VItem/MeshItem 聚合已放入明确的 Prepare 函数，viewport packet 生成已放入 Queue 函数；
- core `CameraFrame`、core `VItem`、core `MeshItem` 已接入新 extraction；
- 用户侧 `Square` 和高层 `ranim_items::VItem` 已可直接作为 Main World Component，并提取为 core VItem Render Entity；
- 旧 `CoreItemStore::update`、Timeline 和 `Extract<Target = CoreItem>` 仍作为兼容路径存在；
- 尚未使用 Bevy Schedule/Query 驱动 extraction，也尚未实现 Changed-based 增量 extraction、Group、异构单 Item 输出和多 View Queue。

## 阶段 1C：Renderer-owned Render World

阶段 1C 是阶段一的下一个正式推进内容。它不继续扩展 `CoreItemStore` 内部的 Render World，而是先修正 World 的所有权和 extraction 边界，再在新的边界上恢复 1B 已验证的能力。

本阶段仍然只依赖 `bevy_ecs`，不直接引入 `bevy_render`。Bevy 的实现用于参考 World 分离、实体同步、提取注册和渲染阶段划分，ranim 保留自己的 wgpu、RenderGraph、RenderPool 与离线输出架构。

### Bevy 0.19 调研结论

Bevy 的渲染架构中有以下几项值得直接借鉴：

- Render World 是渲染子应用持有的独立 ECS World，不属于 Main World 的 Store；
- entity sync 只维护 Main Entity 与 Render Entity 的生命周期和映射，不复制组件数据；
- component extraction 在 sync 之后独立执行，并通过只读 Query 获取 Main World 中真正需要的数据；
- `ExtractComponent` 可以声明 `QueryData`、`QueryFilter` 和输出 `Bundle`，并不局限于 `&Self -> Self`；
- asset extraction 与 entity/component extraction 分开，CPU asset 到 GPU resource 的转换发生在 PrepareAssets；
- extraction 之后先确定视图工作集并 Queue/Sort，再根据最终工作集 Prepare/Batch 帧级 GPU 数据；
- extraction command 可以延迟应用，从而让下一帧 Main World 更新与当前帧渲染并行。

以下部分不应直接复制：

- 不为了这一阶段引入 `bevy_app::SubApp`、完整 Plugin 栈和 `bevy_render`；
- 不立即实现独立渲染线程、延迟 Commands 或完整 pipelined rendering；
- 不把 Bevy 标准的 1:1 `ExtractComponent` 当作 ranim 唯一的 extraction 形式；
- 不在尚无 asset handle 模型时照搬完整 `RenderAsset` 系统。

ranim 当前的关键差异是，一个作者 Item 可能提取成多个独立 primitive。例如 `Text`、`Group` 和复合图形天然需要 1:N extraction。因此阶段 1C 采用“稳定 render root + 可选 primitive children”的扩展模型。

### 所有权边界

目标 crate 边界为：

```text
ranim-core
├─ Main World / World
├─ Timeline 兼容求值
├─ Item 与场景侧 Component
└─ renderer 无关的场景 primitive 数据

ranim-render
├─ RenderWorld
├─ Main/Render entity mapping
├─ ExtractRegistry
├─ Queue / Sort / Prepare schedules
├─ prepared GPU resources
├─ RenderPackets
└─ Renderer / RenderGraph
```

这里的 renderer 无关 primitive 是指当前 `CameraFrame`、core `VItem` 和 `MeshItem` 一类 CPU 场景数据。它们可以暂时继续位于 `ranim-core`，因为不包含 `wgpu` 类型，也可以被 bounds、测试、导出等非渲染逻辑使用。`RenderWorld`、映射、提取状态和 GPU 聚合资源则必须位于 `ranim-render`。

`CoreItemStore` 在兼容期只持有 Main World：

```rust,ignore
pub struct CoreItemStore {
    world: bevy_ecs::world::World,
    // Timeline -> Main World reconciliation state
}

pub struct RenderWorld {
    world: bevy_ecs::world::World,
    // renderer-side entity mapping and extraction state
}
```

`RenderWorld` 是渲染侧的长生命周期对象。实现时可以先由 `Renderer` 直接持有；如果后续需要无 GPU extraction 测试、多个 renderer backend 或独立 render runner，再在 `Renderer` 外增加一个同时持有 RenderWorld 和 GPU renderer 的 `RenderApp`/`RenderRunner`，而不是把 RenderWorld 放回 core。

### 单帧数据流

阶段 1C 的目标数据流为：

```text
Timeline seek / World tick
          │
          ▼
Main World<Item Components>
          │
          ├─ Sync render roots
          └─ Extract registered components
                     │
                     ▼
Render World<Render Components + Primitive Entities>
          │
          ├─ Prepare assets
          ├─ Queue per-view work
          ├─ Sort logical render items
          └─ Prepare/Batch frame buffers
                     │
                     ▼
          RenderPackets + GPU resources
                     │
                     ▼
                 RenderGraph
```

Main World 只在 Sync/Extract 期间被读取；完成 extraction 后，当前帧渲染不再借用 Main World。这样 preview、离线 worker 和未来的模拟/渲染并行都能共享同一边界。

### 实体同步

每个参与渲染的 Main Entity 在 Render World 中有一个稳定的 root entity：

```text
Main Entity<Square, Transform, Visibility>
                    │
                    ▼
Render Root<MainEntity, RenderTransform, RenderVisibility, ...>
```

阶段 1C 首版把映射保存在 render 侧：

```rust,ignore
struct MainEntity(Entity);

struct RenderEntityMap {
    roots: HashMap<Entity, Entity>,
}
```

这样 Main World 不需要插入 renderer 定义的 `RenderEntity` 组件，也不要求 `ranim-core` 依赖 `ranim-render`。每次 sync 负责：

1. 为新增的可渲染 Main Entity 创建 render root；
2. 复用仍然存在的 root；
3. Main Entity 消失或不再包含任何已注册渲染来源时，清理 root 及其 primitive children。

首版可以完整 reconcile 已注册 query 的实体集合。等正确性稳定后，再使用 added/removed component 事件或 change tick 将 sync 改为增量路径。只有当双向查询成为明确需求时，才考虑像 Bevy 一样把 `RenderEntity` 写回 Main World。

### 1:1 与 1:N extraction

对简单 Item，提取结果直接作为 component 插入稳定 root：

```text
Main Entity<core::VItem>
        │ extract
        ▼
Render Root<RenderVItem>
```

对复合 Item，root 保留共同状态，每个 primitive 使用独立 child entity：

```text
Main Entity<Text>
        │
        ▼
Render Root<MainEntity, RenderTransform, ...>
  ├─ Primitive<ExtractedFrom, PartKey, RenderVItem>
  ├─ Primitive<ExtractedFrom, PartKey, RenderVItem>
  └─ Primitive<ExtractedFrom, PartKey, RenderVItem>
```

child 不强制使用 ECS hierarchy；只要它记录来源 root、extractor 和 part identity 即可：

```rust,ignore
struct ExtractedFrom(Entity); // render root
struct ExtractorId(u32);
struct PartKey(u64);
```

映射的稳定键为 `(render_root, extractor_id, part_key)`。固定拓扑的 Item 可以用顺序 index 生成 `PartKey`；Text、Group 或动态拓扑 Item 应优先提供语义稳定的 key。这样在插入、删除或重排一个 part 时，不必让后续全部 primitive 改变身份。

每个 extractor 在一轮 extraction 结束时对自己的输出做 reconciliation：已有 key 原地更新，新增 key spawn，本轮未出现的旧 key despawn。一个 Main Entity 可以注册多个 extractor，从而分别输出 VItem、MeshItem 或其他不同 render bundle，不要求一个 trait 调用返回异构 Vec。

### Extract 协议与注册

阶段 1B 的 `&self -> Vec<RenderItem>` 只能读取单一 component，并且每次调用新建 Vec。正式协议应借鉴 Bevy 的 query-based extraction：

```rust,ignore
pub trait ExtractComponent {
    type QueryData: ReadOnlyQueryData;
    type QueryFilter: QueryFilter;
    type Out: Bundle;

    fn extract_component(
        item: QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out>;
}
```

1:N extraction 使用独立协议或等价的注册函数：

```rust,ignore
pub trait ExtractMany {
    type QueryData: ReadOnlyQueryData;
    type QueryFilter: QueryFilter;
    type Out: Bundle;

    fn extract_many(
        item: QueryItem<'_, '_, Self::QueryData>,
        output: &mut ExtractOutput<Self::Out>,
    );
}
```

`ExtractOutput` 由 renderer 复用内部容量，并提供 `emit(part_key, bundle)`；它不是用户动画 API 中的 World scope，只是 extraction 系统的输出写入器。

提取逻辑由 `ranim-render::ExtractRegistry` 注册和执行。首版不必复刻 Bevy 的跨 World `SystemParam`：每个 typed extractor 可以缓存 `QueryState`，显式接收 `&MainWorld` 和 `&mut RenderWorld`。当 extraction 的并行调度确实成为瓶颈时，再将 registry 升级为 `Schedule`。

注册策略遵循以下原则：

- 内置 core primitive 的 extractor 由 renderer 默认注册，用户没有额外负担；
- `ranim-items` 的内置高层 Item 由 ranim 的默认 render 集成统一注册；
- 自定义 Item 只在需要自定义渲染提取时注册一次 extractor；
- 普通模拟状态 Component 无需实现或注册任何渲染 trait；
- `World::spawn(square)` 与 Entity 操作不触碰 extractor registry，World 本身保持 renderer 无关。

初期可以为现有 `T: Extract<Target = CoreItem> + Component` 提供兼容注册器，把旧 Item extraction 接入新的 Render World。长期再让内置 Item 使用 query-based 协议，避免这一步阻塞阶段一落地。

### Queue、Sort 与 Prepare

当前原型先完整构造 `VItemsBuffer`/`MeshItemsBuffer`，之后 Queue 只生成 viewport packet。这个顺序足以保持现有单视图行为，但不适合未来的可见性、多视图和按材质/管线 batching。

目标阶段顺序调整为：

```text
Sync
→ Extract
→ PrepareAssets
→ Queue
→ Sort
→ Prepare/Batch
→ Render
→ Cleanup
```

各阶段职责如下：

- Queue：根据 camera/view、visibility、layer 和 primitive 类型确定逻辑工作集；
- Sort：确定透明对象顺序、pipeline/batch key 和稳定的帧内次序；
- PrepareAssets：把发生变化的长期 CPU asset 转换或上传为可复用 GPU resource；
- Prepare/Batch：只对 queued work 按最终顺序构造本帧 `VItemsBuffer`、`MeshItemsBuffer` 和 bind group；
- Render：RenderGraph 只消费 prepared resource 和 `RenderPackets`，不查询 Main World；
- Cleanup：清理本帧临时 render entity、packet 和 staging data，保留长期 Render World 状态。

在尚未实现 visibility 和 render phases 时，Queue 可以简单地把全部 RenderItem 按 `RenderItemOrder` 加入一个默认 view；Prepare 仍执行当前完整 packing。这样先修正边界，不同时重写 GPU 管线。

### 与现有输出线程的关系

当前离线渲染把 `CoreItemStore` 作为拥有所有权的帧容器发送给 render worker。阶段 1C 可以继续沿用这一模型：worker 顺序接收 Main World 帧，在自己持有的 Render World 中执行 Sync/Extract，然后渲染并把 Main World 容器送回对象池。

```text
evaluation thread                  render worker
-----------------                  -------------
seek/tick Main World
send owned frame  ───────────────► Sync/Extract into persistent Render World
                                    Queue/Prepare/Render
recycle Main World ◄────────────── send frame container back
```

因此 Render World 移出 `CoreItemStore` 不会破坏现有多 buffer 输出结构，也不要求每个在飞 readback 持有一份 Render World clone。异步边界要求的是 extraction 后不再借用 Main World，而不是把全部 renderer 状态复制成一次性快照。

### 推进顺序

阶段 1C 按以下纵向切片实施：

1. 在 `ranim-render` 定义 `RenderWorld`，移动 render entity、映射和有序查询逻辑；`CoreItemStore` 只保留 Main World。
2. 让 `Renderer` 持有或显式管理长生命周期 Render World，渲染入口改为接收 Main World 并先执行 extraction。
3. 将当前 core 中的 type-erased extractor registry 移入 `ranim-render`，先保持现有 `&T -> Vec<RenderItem>` 兼容行为，避免一次性改动 Item API。
4. 引入稳定 render root，分离 entity sync 与 component extraction；补齐 Main Entity despawn、component removal 和 extractor output 缩减的清理测试。
5. 引入 query-based `ExtractComponent`/`ExtractMany` 协议和可复用输出缓冲，迁移 core VItem、MeshItem、CameraFrame，再迁移 Square、Text、Group。
6. 新增 Queue 工作集与 Sort 阶段，让 Prepare 根据 queued order 构造合并缓冲；首版仍允许全量 packing。
7. 最后再利用 `Changed<T>`、asset handle、稳定 buffer range 和脏区间上传做增量优化，并记录相对阶段 1A 的分配和上传基线。

第 1 至第 4 步只修正所有权和生命周期，不改变 Timeline、Animation、`Square::new()`、`play(square.show())` 或现有输出 API。第 5 步开始扩展直接 World 编码能力，但仍不要求一般用户接触 Entity ID。

### 阶段 1C 验收条件

- `ranim-core` 不再定义或持有 Render World、RenderItemKey、ExtractedFrom 和 extractor registry；
- `ranim-render` 独立拥有并跨帧复用 Render World；
- Main World 的 entity spawn/despawn 与 render root 生命周期能够正确同步；
- component 被移除、extractor 返回 `None` 或 1:N 输出缩减时，不残留旧 render component/entity；
- 一个 Main Item 可以从多个 Main World component 中提取，并可以输出零到多个稳定 primitive；
- Render World 中的 primitive 在 Queue/Prepare 前保持独立 Entity；
- VItemsBuffer/MeshItemsBuffer 只在 Prepare/Batch 阶段构造，RenderGraph 不直接访问 Main World 或 Render World；
- preview、离线渲染、截图和现有 examples 的输出保持一致；
- 不新增 `bevy_render`、`bevy_app` 依赖；
- 文档、测试和性能基线与实现同步更新。

### 暂不决定

- 最终顶级对象命名为 `Ranim`、`RenderApp` 还是 `RenderRunner`；
- 是否把 CPU render primitive 从 `ranim-core` 拆到单独的 scene/render-types crate；
- 是否使用 ECS hierarchy 表达 render root 与 primitive children；
- 是否把 `RenderEntity` 反向组件写入 Main World；
- 是否在 v0.3 启用独立 render thread 和一帧流水并行；
- 最终 asset handle、材质和纹理资源模型。

这些问题不会阻塞阶段 1C 的前四个切片。先建立正确所有权、同步和 extraction 语义，再根据真实使用压力决定更重的抽象。
