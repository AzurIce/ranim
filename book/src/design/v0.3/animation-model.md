# Animation 结构与求值能力

> 状态：构思中。本页讨论 Animation 本身的结构、求值语义和类型擦除边界，不代表最终实现。

## 核心判断

Animation 应表示一个**可按任意局部时间采样的纯动画**。依赖上一状态递推的 Simulation/System 不属于 Animation；两者只在更高层的 Runner 中统一为“更新 World 的逻辑”。

```text
Animation::sample(alpha) ─┐
                          ├─► Runner ─► World
Simulation::step(dt) ─────┘
```

区分二者不是为了限制实现，而是为了保留不同的能力保证：

- Animation 可以随机访问、重复求值和并行求值。
- Simulation 依赖历史，需要 reset、固定步长和可能的 checkpoint。
- Editor 拖动时间轴时可以直接采样 Animation，但必须恢复或重放 Simulation。

因此不建议让同一个 `Animator` trait 同时提供 `seek` 和 `tick`，也不建议用 enum 把 Pure/Stateful 作为 Animation 的两个变体。它们的状态模型和调用约束不同，强行统一会让调用方仍需检查具体能力。

## 当前结构

当前 `AnimationCell<T>` 包含一个类型擦除的 evaluator：

```rust,ignore
pub struct AnimationCell<T> {
    inner: Box<dyn Eval<T>>,
    pub info: AnimationInfo,
    anim_name: String,
}
```

加入 Timeline 后，`AnimationCell<T>` 又被擦除为 `Box<dyn CoreItemAnimation>`：

```text
Box<dyn CoreItemAnimation>
  └─ AnimationCell<T>
       └─ Box<dyn Eval<T>>
```

这形成两次堆分配和两层动态分派。第一层用于隐藏具体动画类型，第二层用于让不同输出类型的动画进入同一个 Timeline。两层擦除各有原因，但不一定需要同时存在。

## 静态 Animation

可以让 Animation 保持具体类型，并使用关联类型声明输出：

```rust,ignore
pub trait Animation {
    type Output;

    fn sample(&self, alpha: f64) -> Self::Output;
}
```

这里使用 `sample` 而不是 `seek`：Animation 本身没有当前时间，也不改变内部播放位置，只计算给定 `alpha` 的结果。

时间范围、rate function、启用状态和调试名称属于 clip/cell 元数据：

```rust,ignore
pub struct AnimationCell<A: Animation> {
    inner: A,
    info: AnimationInfo,
    anim_name: String,
}

impl<A: Animation> AnimationCell<A> {
    pub fn sample_at(&self, local_time: Time) -> A::Output {
        let alpha = self.info.map_time_to_alpha(local_time);
        self.inner.sample(alpha)
    }
}
```

这样 `AnimationCell<A>` 在进入 Timeline 前不需要 `Box<dyn Eval<T>>`，具体动画可以直接内联存储。

## Animation、Cell 与 Scheduled Clip

建议区分三个层次：

| 层次 | 职责 |
| --- | --- |
| `Animation` | 将局部 `alpha` 纯函数式地映射为一个值或变更结果 |
| `AnimationCell<A>` | 为 Animation 添加 duration、rate function、名称等局部时间信息 |
| `ScheduledClip` | 将 cell 绑定到全局时间范围和 World 中的目标 |

概念结构为：

```rust,ignore
pub struct ScheduledAnimation<A: Animation> {
    target: AnimationTarget,
    range: TimeRange,
    cell: AnimationCell<A>,
}
```

Timeline 中真正需要类型擦除的是 `ScheduledAnimation<A>`，因为只有到这里才需要把不同动画类型、输出类型和目标放入同一个集合。

```rust,ignore
pub trait DynScheduledAnimation {
    fn range(&self) -> TimeRange;
    fn target(&self) -> AnimationTarget;
    fn apply_at(&self, time: Time, world: &mut World);
}

pub struct Timeline {
    clips: Vec<Box<dyn DynScheduledAnimation>>,
}
```

理想的分配结构变为：

```text
Box<dyn DynScheduledAnimation>
  └─ ScheduledAnimation<AnimationCell<A>>
       └─ A
```

类型擦除只发生一次，而且边界与“进入异构 Timeline”这一需求一致。

## Animation 的输出

`Animation::Output` 最初可以继续是完整物件 `T`，与当前 `Eval<T>` 语义接近：

```rust,ignore
impl Animation for Morph<T> {
    type Output = T;

    fn sample(&self, alpha: f64) -> T {
        // interpolate source and destination
    }
}
```

`ScheduledAnimation` 负责将结果写回指定 entity。这样迁移成本较低，但整个物件动画同时写入 World 时可能覆盖其他系统对该物件不同字段的修改。

长期可以考虑让 target 精确到 component/property，或者让输出成为显式 patch：

```text
Entity target       → 替换整个物件
Component target    → 替换某个组件
Property target     → 写入单个可动画属性
Patch output        → 对多个字段应用定义明确的变更
```

这一选择会直接影响并行动画的冲突检测，应与动画组合提案共同验证。v0.3 初期可以先保持完整 `T` 输出，但内部 target 不应继续使用 `(timeline_id, animation_id)` 作为物件身份。

## Animation 不提供 Tick

纯 Animation 不需要 `tick(delta)`。给定当前绝对时间，Timeline 将其映射到 clip 的局部时间，然后直接调用 `sample(alpha)`。

离线输出和在线播放即使按帧迭代，也不代表 Animation 必须递推：

```rust,ignore
for sample_time in output.frame_times() {
    timeline.apply_at(sample_time, &mut world);
    simulation.advance_to(sample_time, &mut world);
    renderer.render(world.emit());
}
```

这里输出循环是顺序的，但 Animation 求值仍是随机访问的。Simulation 的固定步长也不应直接等于输出的 `1 / fps`，否则改变输出 FPS 会改变模拟结果。

## 与 World 的初始状态

Animation 的 source value 从哪里取得，需要形成明确规则：

- 在构造动画时从 authoring World 捕获 source；与当前 `&mut T` 创建动画的方式接近。
- 在 Timeline seal 时读取 base World；允许先声明动画、后完成场景构造。
- 在 clip 开始时从 runtime World 读取；最灵活，但会使 Animation 结果依赖此前执行历史，不再是完全纯粹的随机访问。

初步倾向是在构造或 seal 阶段固定 source，并将其存入具体 Animation。若确实需要“从运行到该处的状态开始”，应由 Timeline 编译阶段解析，或明确标记为具有依赖关系的 clip，而不是在 `sample()` 时隐式读取 World。

## 对象安全与命名

带有关联类型的 `Animation` 不直接作为 Timeline trait object，因此不要求它在所有用法中对象安全。对象安全边界是 `DynScheduledAnimation`。

`AnimationCell` 是否继续使用 “Cell” 命名仍可讨论。如果它只是 Animation 加局部 timing 的包装，`AnimationClip` 可能更准确；但 Timeline 中已经调度到全局时间的对象也常被称为 clip，需要避免两个层次同名。

一种候选命名是：

```text
Animation<A> / Animation trait   纯采样逻辑
TimedAnimation<A>                duration + rate function
ScheduledClip                    target + global time range
```

## 待决问题

- `Animation::Output` 是完整对象、组件值、属性值，还是 patch？
- source value 在构造、seal 还是 clip start 时捕获？
- Timeline 类型擦除后，如何保留精确的 target/property 信息用于冲突检测？
- 是否需要为编译时间和复杂组合类型提供显式 `.boxed()`？
- `AnimationInfo` 中哪些字段属于局部 Animation，哪些字段属于全局 ScheduledClip？
- Animation 的纯函数保证是否需要在 trait 文档中作为语义契约，而不是由类型系统强制？
