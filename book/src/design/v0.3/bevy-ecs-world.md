1. 将 ranim 的 Store 底层重构为 bevy_ecs 的 World，且重构相应的渲染 extract 等逻辑。这一阶段的目标就是不改动全部外部接口的情况下更换我们底层的 store。
2. 我们的动画现在只有 eval_alpha 语义，但是我们改为 seek 和 tick，我们在这个基础之上构思如何避免现在的 Eval<T> 会导致频繁的内存分配（这块可能就需要借助 World，在 seek 时同步 world 物件（spawn despawn 之类的），然后 tick 时原地修改。
3. 然后我们再去构思模拟式的动画 API 要怎么设计。

---

# 使用 `bevy_ecs` 重构 World

> 状态：第一阶段已经落地。本文从尚未引入 `bevy_ecs` 时的 ranim 出发，说明为什么引入 ECS、采用了什么模型、当前代码如何工作，以及这一阶段为后续动画模型留下了什么能力。

## 背景

引入 `bevy_ecs` 以前，ranim 的核心数据流可以简化为：

```text
Timeline 在时间 t 求值
        │
        ▼
一组带顺序的 CoreItem 值
        │
        ▼
Renderer 遍历这些值
        │
        ▼
VItemsBuffer / MeshItemsBuffer
        │
        ▼
RenderGraph
```

这种模型很适合纯 Timeline 动画：每个采样时间都可以重新计算完整场景，Renderer 只需要消费本帧结果。但它也带来几个限制：

- 场景主要表现为一组临时值，而不是具有稳定身份、可原地修改的对象；
- Timeline 是产生场景状态的唯一主要入口，用户难以直接操作一个长期存在的 World；
- 每帧完整求值容易产生大对象 clone、`Vec` 分配和重新聚合；
- 基于上一个状态递推的模拟逻辑没有自然的存储和调度位置；
- 渲染器直接面对求值结果，场景状态、渲染状态和 GPU 数据的职责容易混在一起。

我们希望以后同时支持两种动画：

```text
纯函数式动画：seek(t) 直接得到 t 时刻的状态
模拟式动画：  tick(dt) 根据上一状态递推到下一状态
```

二者最终都应修改同一个长期存在的场景。因此第一步不是立即重写 Animation API，而是先建立一个可以保存稳定对象、支持 spawn/despawn、Query 和原地修改的 World。

## 第一阶段目标

第一阶段只调整底层状态和渲染边界，不同时重写用户动画 API：

- 使用 `bevy_ecs::World` 作为场景的真实存储；
- 让 Timeline 现有的完整帧求值仍能写入这个 World；
- 让任意可渲染 Item 可以作为 Component 存在；
- 把 Main World 与 Render World 分离；
- 将渲染拆成 Extract、Queue、Prepare 和 Render 阶段；
- 保持 `Square::new()`、Timeline、preview、离线渲染和编码接口基本不变；
- 记录性能基线，为后续 `seek`/`tick` 和增量更新提供依据。

这一阶段只引入 `bevy_ecs`，不引入 `bevy_app` 或 `bevy_render`。Ranim 继续使用自己的 wgpu、RenderGraph、RenderPool、RenderPackets 和输出线程模型。

## 核心设计

### 两个 World

整个系统分为 Main World 和 Render World：

```text
Main World                         Render World
----------                         ------------
用户和动画关心的场景状态           渲染管线关心的派生状态
Square、Text、Transform             core VItem、MeshItem
Visibility、物理状态、约束          render root、primitive
由 seek/tick/System 修改             由 Extract 更新
```

Main World 是场景的事实来源。Render World 是可丢弃、可从 Main World 重新生成的渲染侧缓存。

这个边界很重要：

- 动画和模拟不需要依赖 Renderer；
- Renderer 不需要理解 Timeline；
- 同一个 Main World 将来可以被不同 renderer、导出器或检查工具消费；
- Extract 完成后，当前帧的渲染不再借用 Main World；
- Render World 可以跨帧复用实体和渲染资源，而不要求 clone 整个 World。

### crate 所有权

```text
ranim-core
├─ Main World 的兼容封装 CoreItemStore
├─ Timeline 求值到 Main World 的 reconciliation
├─ renderer-neutral 的场景 Component
└─ ExtractComponent / ExtractMany 协议

ranim-render
├─ RenderWorld
├─ Main Entity 到 Render Entity 的映射
├─ ExtractSchedule 和 extraction systems
├─ Queue / Prepare
├─ prepared GPU resources
├─ RenderPackets
└─ Renderer / RenderGraph
```

`CameraFrame`、core `VItem` 和 `MeshItem` 暂时仍在 `ranim-core`。它们是不包含 wgpu 类型的 CPU primitive，也可以被 bounds、测试和导出逻辑使用。Render World、提取状态和 GPU 聚合资源则属于 `ranim-render`。

### 单帧数据流

```text
Timeline eval / 未来的 seek、tick、System
                    │
                    ▼
          Main World<Item Components>
                    │
                    │ Sync + Extract
                    ▼
       Render World<Render Components>
                    │
                    │ Queue
                    ▼
          当前 View 的逻辑工作集
                    │
                    │ Prepare / Batch
                    ▼
      VItemsBuffer / MeshItemsBuffer
                    │
                    │ packets
                    ▼
               RenderGraph
```

各阶段职责如下：

- Sync：维护 Main Entity 与 Render Entity 的生命周期对应；
- Extract：把场景 Component 转换为渲染 Component；
- Queue：确定当前 view 实际参与渲染的逻辑工作集和顺序；
- Prepare/Batch：把 queued entity 聚合为 GPU buffer 和 bind group；
- Render：RenderGraph 只消费 prepared resource 和 RenderPackets，编码 GPU 命令。

## Main World

### `CoreItemStore` 的兼容职责

当前仍保留 `CoreItemStore` 这个名字，但其真实存储已经是 `bevy_ecs::World`：

```rust,ignore
pub struct CoreItemStore {
    world: bevy_ecs::world::World,
    entities: HashMap<CoreItemKey, Entity>,
    evaluated_entities: Vec<Entity>,
    item_entities: Vec<Entity>,
}
```

它同时兼容两类来源：

1. 现有 Timeline 求值产生的 `CoreItem`；
2. 通过 `insert_item` 直接放入 Main World 的 Item Component。

因此现阶段可以同时存在：

```rust,ignore
store.update(timeline.eval_at_sec(t));
store.insert_item(Square::new(2.0));
```

`CoreItemStore` 只是迁移期 wrapper。长期顶级 API 可以改成更直接的 `World` 或 `Ranim`，但第一阶段不需要先决定这个名字。

### Timeline reconciliation

Timeline 仍会在每个采样时间产生完整的 `CoreItem` 序列。`CoreItemStore::update` 不再清空并重建数组，而是把这些值 reconcile 到稳定 Entity：

```rust,ignore
struct CoreItemKey {
    source: (usize, usize),
    part: usize,
}
```

`source` 是现有的 `(timeline_id, animation_id)`。一个动画可能产生多个 CoreItem，因此还需要 `part` 表示同一 source 的第几个输出。

每次更新执行：

1. key 已存在时，复用原 Entity 并更新组件；
2. key 新出现时，spawn Entity；
3. key 消失时，despawn Entity；
4. CoreItem 类型变化时，在同一 Entity 上移除旧组件并插入新组件；
5. 值未变化时不重复 insert，使 Bevy change tick 只在真实变化时推进。

这种 key 不是未来最终的用户对象身份，只是把旧 Timeline 的值序列接入稳定 World 的兼容桥梁。

### 场景顺序

ECS Query 的迭代顺序不是渲染顺序。为了保持迁移前的输出结果，Main World 额外维护显式场景顺序：

```text
直接插入的 item entities
        +
当前 Timeline 求值产生的 entities
        │
        ▼
CoreItemStore::scene_entities()
```

Timeline entity 还带有 `CoreItemOrder`。Extract 会把场景序列中的位置转换为 Render World 的 `RenderRootOrder`。

目前只有通过 `insert_item` 或 Timeline reconciliation 登记的 Entity 会进入场景顺序。直接调用底层 `world_mut().spawn(...)` 创建的 Entity 尚不会自动参与 extraction。这是迁移期 wrapper 的限制，不是最终 World API 的目标语义。

## RenderWorld & Extract

### RenderWorld

`RenderWorld` 由 `Renderer` 持有，是一个跨帧存在的独立 `bevy_ecs::World`：

```rust,ignore
pub struct Renderer {
    render_world: RenderWorld,
    // GPU resources and render graph
}
```

Main World 中的 Entity 和 Render World 中的 Entity 属于不同 World，即使数值碰巧相同也没有身份关系。RenderWorld 使用显式映射维护二者的对应。

#### render root

一个参与渲染的 Main Entity 对应一个稳定的 render root：

```text
Main World                         Render World

Entity A<Square>                   Entity R
                                   ├─ MainEntity(A)
                                   ├─ RenderRootOrder
                                   └─ extracted components
```

这里的 root 不是场景树根节点，也不表示 ECS hierarchy。它只是 Main Entity 在 Render World 中的稳定身份代理。

映射当前只保存在 renderer 侧：

```rust,ignore
struct RenderEntities {
    roots: HashMap<MainEntity, RenderEntity>,
    // primitive mappings and frame order
}
```

与 Bevy 不同，ranim 当前不会把 `RenderEntity(R)` 反向插入 Main Entity A。这样可以保持 Main World 不依赖 `ranim-render`。代价是同步阶段需要在 renderer 内维护映射。

当传入另一个 Main World 时，`WorldId` 会变化。RenderWorld 会清空绑定旧 World 的实体和映射，并为新的 World 重建 `QueryState`，避免把两个 World 中碰巧相同的 Entity ID 当成同一对象。

#### 1:1 extraction

简单 Item 通常只产生一个渲染对象。`ExtractComponent` 的输出直接插入 render root：

```text
Main A<Square>
        │
        ▼
Render R<MainEntity(A), core::VItem>
```

这样 1:1 情况不会额外创建一层 primitive Entity。

如果 extractor 返回 `None` 或不再匹配 query，它之前写入的 Bundle 会从 root 上移除。只要还有其他 extractor 匹配同一个 Main Entity，root 就继续存在。

#### 1:N extraction

Text、Group 和复合图形可能产生多个独立 primitive。`ExtractMany` 为每个输出创建独立 Render Entity：

```text
Main A<Text>
        │
        ▼
Render Root R<MainEntity(A)>
        ├─ P0<ExtractedFrom(R), RenderVItem>
        ├─ P1<ExtractedFrom(R), RenderVItem>
        └─ P2<ExtractedFrom(R), RenderVItem>
```

这些 Entity 不使用 ECS hierarchy，而是通过组件表达归属和身份：

```rust,ignore
struct RenderItemKey {
    root: Entity,
    extractor: usize,
    part: usize,
}

struct ExtractedFrom(Entity);
```

稳定 key 为 `(render_root, extractor_id, part_key)`：

- key 已存在时原地更新原 Entity；
- 新 key 会 spawn；
- 本帧不再出现的旧 key 会 despawn。

顺序 `push(bundle)` 使用当前输出下标作为 part key。动态拓扑 Item 应使用 `emit(part_key, bundle)` 提供语义稳定的 key，否则在中间插入一个 part 会改变后续全部 Entity 的身份。

#### 渲染顺序

`RenderRootOrder` 来源于 Main World 的场景序列，不是 z 坐标或 z-index。

当前帧的输出按以下 key 排序：

```text
(render_root_order, extractor_registration_order, output_order)
```

排序结果写入 `ordered_items`，并为输出 Entity 写入 `RenderItemOrder`。同一个 render root 被多个 1:1 extractor 命中时，当前实现只在 `ordered_items` 中保留一次该 root。

这是为了保持迁移前的确定性顺序。未来正式的 Queue/Sort 仍需要结合 view、visibility、render phase、透明深度和 pipeline key 生成真正的排序结果。

### Extract

Extract 的职责是读取 Main World，但只修改 Render World。协议定义在 `ranim-core`，执行系统和注册入口位于 `ranim-render`。

#### `ExtractComponent`

```rust,ignore
pub trait ExtractComponent: Send + Sync + 'static {
    type QueryData: ReadOnlyQueryData;
    type QueryFilter: QueryFilter;
    type Out: Bundle;

    fn extract_component(
        item: QueryItem<Self::QueryData>,
    ) -> Option<Self::Out>;
}
```

它可以读取同一 Main Entity 上的多个 Component，而不局限于 `&Self -> Self`：

```rust,ignore
type QueryData = (&'static Shape, &'static Transform, &'static Visibility);
type Out = RenderShape;
```

#### `ExtractMany`

```rust,ignore
pub trait ExtractMany: Send + Sync + 'static {
    type QueryData: ReadOnlyQueryData;
    type QueryFilter: QueryFilter;
    type Out: Bundle;

    fn extract_many(
        item: QueryItem<Self::QueryData>,
        output: &mut ExtractOutput<Self::Out>,
    );
}
```

`ExtractOutput` 是 system-local 的可复用缓冲，不会为每个 source 每帧创建新的输出 Vec。

#### 注册

ECS 不能自动发现所有实现了 extraction trait 的 Rust 类型，因此 renderer 必须注册相应 system：

```rust,ignore
renderer.register_component::<Square>();
renderer.register_many::<TextExtractor>();
```

内部等价于：

```text
register_component
    → ExtractSchedule.add_systems(extract_component_system)

register_many
    → ExtractSchedule.add_systems(extract_many_system)
```

同一种 extractor 重复注册是幂等的。core `CameraFrame`、core `VItem` 和 `MeshItem` 由 RenderWorld 默认注册；高层 `Square` 和 `ranim_items::VItem` 由当前 renderer 构造路径显式注册。

这里借鉴了 Bevy 的核心思路：extraction 是 ECS Schedule 中的普通泛型 system，而不是 `Vec<Box<dyn ErasedExtractor>>` 形式的独立运行时注册表。

#### 每帧执行

当前 `RenderWorld::extract` 的流程是：

```text
1. 从 CoreItemStore 构造 MainSceneOrder
2. 检查 Main WorldId
3. 将 Main World 临时作为 Resource 移入 Render World
4. RenderEntities::begin_frame
5. ExtractSchedule.run
6. 应用 Commands
7. 清理 stale roots、components 和 primitive entities
8. 生成 ordered_items
9. 将 Main World 原样放回 CoreItemStore
```

每个 extraction system 使用自己的 `Local<QueryState>` 查询 Main World，并通过 `Commands` 更新 Render World。`QueryState` 会跨帧复用，在 `WorldId` 改变时重建。

当前尚未实现 Bevy 的 `PendingSyncEntity` 增量同步队列。各 extraction system 也共享可变 reconciliation 状态，因此目前仍会串行执行并全量扫描匹配实体。

## Queue、Prepare 与 Render

Extract 完成后，Render World 仍保存独立 Render Entity，不会立即压平为 GPU buffer。

### Queue

当前 `queue_default_view` 产生一个单视图 `QueuedFrame`：

```rust,ignore
struct QueuedFrame {
    camera: Option<Entity>,
    items: Vec<Entity>,
}
```

目前没有 visibility、layer、多 view 和独立 render phase，Queue 只是把 `ordered_items` 复制为默认工作集，并选择其中的 CameraFrame。

### Prepare

Prepare 只消费 queued entity：

```text
queued VItem entities
        │
        ▼
VItemsBuffer::update

queued MeshItem entities
        │
        ▼
MeshItemsBuffer::update
```

当前仍然每帧完整 packing。`WgpuVecBuffer::set` 会将 packed 内容与上一帧缓存做字节比较，内容完全相同时跳过 GPU upload，但本帧的 CPU 遍历、clone 和临时 Vec 构造仍然存在。

### Render

Queue 同时准备 viewport packet，Prepare 更新长期 GPU resource。RenderGraph 只消费这些结果，不查询 Main World 或 Render World：

```text
RenderWorld
    → QueuedFrame
    → prepared GPU buffers + RenderPackets
    → RenderGraph
```

这保留了 ranim 现有的 RenderPool、RenderPackets、OIT 和异步输出结构，不需要采用 `bevy_render`。

## 一个 `Square` 的完整路径

下面用一个直接放入 World 的 Square 串起全部概念：

```rust,ignore
let entity = store.insert_item(Square::new(2.0));
renderer.register_component::<Square>();
renderer.render_store_with_pool(..., &mut store, ...);
```

1. `insert_item` 在 Main World spawn `Entity A<Square>`，并把 A 加入场景顺序；
2. `register_component` 将 `extract_component_system::<Square>` 加入 ExtractSchedule；
3. RenderWorld 第一次见到 A 时创建稳定 root R；
4. `Square::extract_component` 将作者 Item 转换为 core `VItem`；
5. core `VItem` 直接插入 R；
6. Queue 把 R 放入默认 view 的工作集；
7. Prepare 从 R 读取 core `VItem` 并写入 `VItemsBuffer`；
8. RenderGraph 使用 prepared buffer 完成渲染；
9. 下一帧仍使用 A 和 R，只更新它们的组件；
10. A 或 Square 消失时，Extract 清理 R 上的输出；没有其他 extractor 匹配 A 时，R 被 despawn。

```text
Main World                 Render World               GPU

A<Square>  ── Extract ──►  R<core::VItem>  ──►  VItemsBuffer
   stable                       stable              frame-packed
```

## 引入前后的区别

| 方面 | 引入前 | 当前实现 |
|---|---|---|
| 场景存储 | 每帧产生的一组值 | 长期存在的 ECS World |
| 对象身份 | source 和数组位置 | Main Entity / Render Entity |
| Timeline | 直接产生 renderer 输入 | reconcile 到 Main World |
| 用户 Item | 通常先降级为 CoreItem | 可以直接作为 Component |
| Renderer 输入 | 直接读取求值结果 | 从 RenderWorld Queue/Prepare |
| 1:N Item | 展开为值序列 | 稳定 primitive entities |
| 删除 | 下一帧值中不再出现 | stale entity/component cleanup |
| 顺序 | Vec 顺序 | 显式 order component 和排序 key |
| 模拟支持 | 没有自然状态容器 | 可以在 Main World 中原地 tick |
| GPU 数据 | 每帧聚合 | 仍每帧聚合，但可复用 buffer 并跳过相同上传 |

## 迁移过程

实现按以下顺序推进：

1. `CoreItemStore` 改用 `bevy_ecs::World`，以 reconciliation 保持 Timeline 行为；
2. 建立独立 Render World，并让 Renderer 持有它；
3. 引入稳定 render root 和 1:N primitive identity；
4. 将 extraction 改为 query-based `ExtractComponent`/`ExtractMany`；
5. 使用 `ExtractSchedule` 注册泛型 systems；
6. 将 Queue 与 Prepare 从 extraction 中分离；
7. 记录 clone、分配和 GPU upload 基线；
8. 后续再实现 changed-based extraction、稳定 GPU range 和 `seek`/`tick`。

这几步的原则是先保证所有权、身份和生命周期语义正确，再做增量性能优化。否则缓存很容易建立在不稳定的对象身份上。

## 当前实现状态

已经完成：

- `CoreItemStore` 的真实存储改为 `bevy_ecs::World`；
- Timeline CoreItem 可以跨帧复用 Entity，并正确 spawn/despawn；
- Item 可以通过 `insert_item` 直接作为 Component 存在；
- Renderer 持有独立、长生命周期 RenderWorld；
- Main/Render Entity 使用稳定 root 映射；
- 1:1 输出直接写 root，1:N 输出使用独立 primitive Entity；
- query-based extraction 和 ExtractSchedule 注册；
- component removal、`None`、1:N 缩减和 Main Entity 消失的清理；
- Main World 切换时根据 `WorldId` 重建状态；
- Queue 产生默认单 view 工作集；
- Prepare 只消费 queued Render Entity；
- 静态 packed 内容不再重复上传 GPU；
- preview、离线渲染、现有 Timeline 和主要 examples 保持工作。

尚未完成：

- 最终公开的 World/Ranim 顶级 API；
- `seek`/`tick` 动画语义；
- 模拟 System API；
- `PendingSyncEntity` 或等价的增量实体同步；
- `Changed` 驱动的增量 extraction；
- 稳定 GPU buffer range 和脏区间上传；
- visibility、layer、多 view、render phase 和正式 Sort；
- Text、Group 等全部高层 Item 的 direct-world extraction；
- 独立 render thread 或一帧流水并行。

## 当前限制

这些限制应在继续扩展 Item 和动画模型时处理：

1. 两个 1:1 extractor 如果向同一 root 输出重叠组件，会发生覆盖；其中一个输出变 stale 时还可能移除另一个 extractor 刚写入的组件。需要建立明确的 render component ownership 规则。
2. 场景顺序仍由 `CoreItemStore` 外部 Vec 维护。直接操作底层 World spawn 的 Entity 不会自动进入 extraction，长期 spawn/despawn 还会留下无效 Entity 记录。最终应让 World 自身表达场景成员和顺序。
3. `extract_schedule_mut` 虽然允许添加 system，但 root mapping、reconciliation 和 ordered output 仍是私有机制，尚不足以构成完整的第三方 Render Plugin API。
4. `QueuedFrame` 当前只保存 Entity。一个 root 上存在多个逻辑渲染输出时，未来需要独立的 phase item，而不能继续把 Entity 本身等同于一次 draw。
5. Extract 会临时把 Main World 移入 Render World；当前恢复过程不是 panic-safe。
6. 每个 extractor 仍会全量 query、排序并 clone 输出。GPU upload 去重没有消除 CPU packing 和临时分配。

这些问题不否定双 World 和 ExtractSchedule 的方向，但会决定下一阶段 API 和性能优化应落在哪一层。

## 性能基线

阶段一的主要结果不是立即降低全部 CPU 成本，而是建立可测量、可继续优化的边界。

400 个 VItem 的测量结果：

| 路径 | 时间 | 分配次数 | 分配字节 | GPU 上传 |
|---|---:|---:|---:|---:|
| `eval_at_alpha` | 262 us | 9211 | 1.17 MB | - |
| eval + `store.update` | 311 us | 9227 | 1.05 MB | - |
| 静态帧 cold render | 15.4 ms | - | - | 214,400 B |
| 静态帧 steady render | 835 us | 1761 | 0.41 MB | 144 B |
| 动画帧 eval + update + render | 1.12 ms | 9421 | 2.06 MB | 全量 |

静态 packed 内容通过 memcmp 避免重复 GPU upload，因此 steady frame 只剩 viewport uniform。大场景的主要回归来自 Main World 到 Render World 的全量 clone，以及 Prepare 的完整 packing。这正是后续 changed-based extraction 和稳定 GPU range 要解决的问题。

这些数字是当前实现的回归基线，不是最终性能目标。

## 与下一阶段的连接

完成 World 和渲染边界后，Animation 不必再以“每次返回一个新的大值”为唯一实现方式。

```text
seek(t)
├─ 确定 t 时刻应该存在的对象
├─ spawn 新进入有效区间的对象
├─ despawn 已离开有效区间的对象
└─ 将已有 Entity 同步到 t 时刻的完整状态

tick(dt)
├─ 保留已有 Entity 和组件容量
├─ 根据上一状态原地递推
└─ 只标记真实发生变化的组件
```

纯函数式动画和模拟式动画的共同结果，都是在特定时间对 Main World 产生修改。Renderer 不需要知道修改来自 Timeline、seek、tick、用户交互还是模拟 System；它只需要 Extract 当前 World。

这就是第一阶段引入 `bevy_ecs` 的核心意义：不是把现有 Vec 机械地换成 ECS，而是先建立一个稳定的状态层，使动画模型和渲染模型可以分别演进。
