# 场景与求值模型

ranim 的核心应从“时间线直接产出可渲染物件”转为“在给定时间得到一个 World，再从 World 渲染一帧”。这样，编码动画、物理/数值模拟、交互编辑和离线渲染可以共享同一份场景状态与渲染接口。

## 核心模型

```text
作者定义
  Base World + Timeline + Simulation/Systems
                    │
                    ▼
              Runner / Player
             seek(t) 或 step(dt)
                    │
                    ▼
             Runtime World at t
                    │
                    ▼
                 World::emit
                    │
                    ▼
               RenderFrame
```

这里的关键不是把所有逻辑强行做成 Timeline，而是让所有逻辑的结果都落在 `World` 上。

## World

`World` 是某一时刻场景的可变状态，也是用户可以直接交互的顶级对象。它应拥有：

- 实体及稳定的 `EntityId`/handle；
- 实体组件和场景资源，例如几何、变换、可见性、相机和用户定义状态；
- 用于渲染提取的只读查询；
- 生成、删除和直接修改对象的 API。

World 不应拥有输出 FPS、视频编码器或 GPU resource，也不应把“当前播放时间”作为其唯一身份的一部分。一个 World 可以是作者编辑的 base world，也可以是 runner 在某时刻建立的 runtime world。

稳定实体 ID 很重要。当前渲染数据的身份来自 `(timeline_id, animation_id)`；这适用于“时间线求值后立即丢弃”的模式，却无法自然表示一个脱离 Timeline 仍持续存在、可被模拟和交互共同修改的对象。Timeline 应引用 entity handle，而不是反过来成为实体的所有者。

## Timeline：纯时间描述

Timeline 是对“在时间 `t` 应向 World 施加什么变化”的可编排、可随机访问描述。典型内容是属性轨道、区间动画和离散事件。

它应满足以下语义：

- 在相同 base world、相同时间和相同输入下，结果确定；
- `seek(t)` 不依赖此前是否访问过 `t - dt`；
- 可被预览器任意拖动，也可被离线渲染并行或重复求值；
- 轨道目标是 World entity/component/property，而不是 renderer primitive。

推荐的概念接口为：

```rust,ignore
trait TimelineEvaluator {
    fn apply_at(&self, time: Time, world: &mut World);
}
```

`apply_at` 的含义必须是“把由 Timeline 控制的字段写成该时刻的值”，而不是在现有值上无条件累加。这样从 base world 重建后求值才是幂等的。

当前实现中，`RanimScene` 保存多条 `Timeline`；`Timeline` 保存类型擦除后的 `AnimationCell<T>`。`SealedRanimScene::eval_at_sec(t)` 逐条 timeline 求值、提取 `CoreItem`，而不是修改一个持久 World。这是本模型迁移时需要替换的核心边界。

## Simulation / Systems：有历史的时间演进

模拟同样在特定时间修改 World，但它通常依赖上一状态：

```rust,ignore
trait Simulation {
    fn reset(&mut self, world: &mut World);
    fn step(&mut self, dt: Duration, world: &mut World);
}
```

例如粒子、刚体、流体、迭代布局或实时输入驱动的逻辑均属于这一类。它们不能仅靠 `apply_at(t)` 无历史地重算，因此不能假装与纯 Timeline 具有相同的随机访问成本。

要支持预览拖动和离线渲染，应选择并明确一种策略：

- 从初始 World 重置后，以固定 `dt` 递进到目标时间；最简单、最确定，但远距离 seek 较慢。
- 定期保存 World + simulation 内部状态的 checkpoint，从最近快照递进；这是推荐的通用策略。
- 要求具体模拟实现高层的解析式 `seek(t)`；只适用于少数系统。

固定步长是可重复性的前提。输出帧间隔 `1/fps` 不必等于模拟步长；runner 可在两个输出时刻之间执行零到多次固定 simulation step，必要时采用已定义的插值策略。

## Runner / Player

Runner 负责把多个时间来源调度成一个 runtime world。它拥有当前时间、base world、副本/快照，以及 Timeline 和 Simulation 的执行顺序。

建议对外提供两种不同但互补的操作：

```rust,ignore
runner.seek_to(t, &mut world); // 为预览、截图、离线采样建立 t 时刻状态
runner.step(dt, &mut world);   // 为实时播放/模拟从当前状态前进
```

不要以 `seek_to` 伪装递推模拟的所有行为；当需要回退时，runner 必须 reset 或恢复 checkpoint。反过来，也不要让纯 Timeline 只能逐帧播放，否则会失去它最有价值的随机访问能力。

### 阶段顺序

当 Timeline、模拟和用户系统共同写 World 时，必须定义稳定且可见的阶段顺序。一个可作为初始方案的顺序是：

```text
1. 从 base world 或 checkpoint 建立 runtime world
2. Timeline 在 t 写入它控制的属性
3. Simulation/System 以固定步长推进到 t
4. 后处理 / 约束 / 用户系统
5. World emit 为 RenderFrame
```

这不是唯一可行顺序。例如“模拟计算目标位置，Timeline 负责视觉缓动”可能需要 simulation 在前、Timeline 在后。无论选择何种顺序，都应把它作为 runner 的显式 phase，而不能依赖注册顺序或偶然的可变借用顺序。

还需要定义同一字段的写入规则：推荐每个字段有明确 owner，或使用显式 layer/priority；不要默默采取“最后写入者获胜”，否则 timeline、交互和模拟组合时会难以调试。

## 作者阶段与运行阶段

直接暴露 `World` 不意味着所有 `world_mut()` 都有相同含义。至少应区分：

| 操作 | 修改对象 | 用途 |
| --- | --- | --- |
| `ranim.world_mut()` | base/authoring World | 创建实体、设置初始状态、编辑静态场景 |
| `timeline.track(entity)` | Timeline 描述 | 编码未来的时间变化 |
| `runner.world_mut()` | runtime World | 交互、实时系统和调试中的当前状态 |

这一区分避免一个常见歧义：用户在播放到 2 秒时直接改了对象，究竟是在修改初始状态、在 2 秒写入一个 keyframe，还是只修改本次运行中的模拟状态？这三种需求应该有不同 API，而不是由调用位置隐式决定。

一个可能的顶层组合是：

```rust,ignore
pub struct Ranim {
    pub world: World,       // authoring/base world
    pub timeline: Timeline,
    pub systems: Systems,
}
```

`Scene` 可以继续承担宏注册、名称、输出配置和 CLI 的边界；它不必等同于 runtime world。构建 Scene 时创建 `Ranim`，输出端再按需创建 runner，是对现有 API 较平滑的演进路径。

## 当前模型与迁移

当前路径为：

```text
RanimScene
  └─ Vec<Timeline>
       └─ Vec<AnimationCell>
              │ eval_at_sec(t)
              ▼
         Iterator<CoreItem>
              ▼
         CoreItemStore
              ▼
          Renderer
```

建议按以下垂直切片迁移，保证每一步都仍可预览和离线渲染：

1. 引入 `World` 与 `World::emit`，先让它产生当前等价的 `CoreItemStore`/`RenderFrame`。
2. 让 renderer 只接收 emit 结果，切断它对 `SealedRanimScene` 的任何认识。
3. 将现有 `SealedRanimScene::eval_at_sec` 封装为“在 `t` 填充 World”的过渡 evaluator。
4. 为 World 引入稳定 entity ID，并将 Timeline 轨道目标从 timeline 下标迁移到实体/属性。
5. 引入 Runner、固定步长 simulation 与 checkpoint；最后再调整作者 API 和宏生成的入口。

在过渡期，旧 Timeline 仍可作为一种 evaluator 存在。目标不是立即删除 `AnimationCell`，而是改变它的输出位置：从“直接产出 renderer primitive”变为“更新 World 中被 Timeline 拥有的状态”。

## 不变量与测试

以下行为应成为测试和 API 设计的依据：

- 纯 Timeline：对相同 base world，`seek(t)` 的结果与先 `seek(t1)` 再 `seek(t)` 的结果相同。
- 模拟：相同初始状态、固定步长、相同输入序列得到相同结果；从 checkpoint 恢复后继续运行与未中断运行一致。
- 渲染：同一 runtime world 的 emit 结果不依赖上一次渲染或输出格式。
- 生命周期：被 Timeline、System 或交互引用的实体删除时，有定义明确的行为（报错、忽略、停止轨道或保留 tombstone），不能静默引用错误对象。

这些不变量比具体类型名称更重要；未来即使 `Ranim`、`World` 或 `Timeline` 的 Rust API 调整，也应保持其语义。
