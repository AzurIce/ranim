# 动画

## `Eval`

Ranim 的叶子动画核心是一个统一的求值协议——运行时对一个动画区段能做的全部事情：在时刻上采样输出、把内部状态推进一个逻辑步、回到初始状态。

```rust,ignore
pub trait Eval {
    type Output;

    /// 在时刻 time 采样输出(采样不推进状态,也不携带任何 delta)。
    fn sample(&self, time: &Time) -> Self::Output;
    /// 回到区段初始状态(确定性契约:不依赖墙钟/未播种 RNG)。
    fn reset(&mut self);
    /// 推进一个逻辑步。
    fn step(&mut self, time: &Time, delta_time: &DeltaTime);
}
```

三个方法都是 required：有状态的区段忘了 `reset` 会在编译期报错，而不是悄悄破坏 seek 重放的确定性。无状态区段把 `reset`/`step` 留空即可。

`Eval` 还内置两个经 `sample` 定义的通用便捷方法：`apply_alpha_to(item, alpha)` 和 `apply_to(item)`（即 `alpha = 1.0`)。它们把动画在某个进度处的状态写入对象并返回动画自身——内置动画的工具方法（`fade_in()` 等）正是靠它在创建动画的同时把 item 置为动画末态。

## 时刻与时段：`Time` 与 `DeltaTime`

求值方法接收的时间上下文按"时刻 / 时段"分为两个类型（`ranim::core::time`):

```rust,ignore
pub struct Time {
    pub alpha: f64,        // rate 扭曲后的进度 r((t-s)/D),由 cell 算好
    pub global_secs: f64,  // 真实全局时刻
}

pub struct DeltaTime {
    pub alpha: f64,        // Δalpha:本逻辑步的进度增量(积分步长)
    pub global_secs: f64,  // Δt = 1/logic_fps,恒稳
}
```

- `sample` 只收 `&Time`——采样在类型层面就拿不到任何 delta;
- `step` 同时收 `&Time` 和 `&DeltaTime`——时变力读时刻，积分读时段；
- `alpha`/`Δalpha` 由包裹区段的 `AnimationCell` 用自己的起点、时长、速率函数算好后传入。**动画逻辑只能拿到时间读数，拿不到时间配置**:`with_duration`/`with_rate_func` 对所有区段都只是纯播放变换；
- 需要墙钟真实时间（不被 rate 扭曲）的区段读 `global_secs`/`global_delta_secs`，它们从会话时钟出发、沿任意嵌套深度的容器原样下传。

## 两种特化：`Pure` 与 `Iterative`

用户通常不直接实现 `Eval`，而是实现 *ranim-anims* 里两个能力 trait 之一，再用对应的适配结构体包成动画：

**纯（闭式、无状态）——`PureEval` + `Pure`:**

```rust,ignore
pub trait PureEval {
    type Output;
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}
```

只有一个方法：`Pure` 适配器把 `sample` 实现为 `eval_alpha(time.alpha)`,`reset`/`step` 为空。`Fn(f64) -> T` 闭包自动实现 `PureEval`，于是闭包动画写作：

```rust,ignore
let animation = Pure(|alpha| Square::new(alpha)).with_duration(2.0);
```

具体 evaluator 同时保存求值所需的数据。例如 `Static<T>` 始终返回同一个值，`Morph<T>` 保存插值需要的源状态和目标状态。

**迭代（有状态、逐步推进）——`IterativeEval` + `Iterative`:**

```rust,ignore
pub trait IterativeEval<S> {
    /// 推进一个逻辑步(状态由适配器持有,以 &mut 传入)。
    fn step(&self, output: &mut S, time: &Time, delta_time: &DeltaTime);
}
```

只有一个方法：状态类型 `S` 同时就是区段的输出；`Iterative::new(initial, step_fn)` 持有初始状态与当前状态——`sample` 是克隆当前状态，`reset` 是恢复初始状态，两者都是结构性的，不需要也无法写错。`Fn(&mut S, &Time, &DeltaTime)` 闭包自动实现 `IterativeEval<S>`：

```rust,ignore
let animation = Iterative::new(
    SpringState { x: 1.0, v: 0.0 },
    |state: &mut SpringState, _time: &Time, dt: &DeltaTime| {
        let dt = SIM_SECS * dt.alpha; // 逻辑秒
        let acc = -K * state.x - C * state.v;
        state.v += acc * dt;
        state.x += state.v * dt;
    },
)
.with_duration(4.0);
```

常量（物理参数、调色板）放闭包捕获或命名 step 结构体；一切可变状态都必须住在状态值里。区段模拟多久是它自己的参数（如 `SIM_SECS`)——`with_duration` 只是播放抻拉。当状态与要渲染的内容不同，为状态类型实现 `Extract`（投影点，每帧一次）。

**异形区段**——两个适配器覆盖不了的需求（比如以后要查询外部世界的区段），直接在类型上实现 `Eval`。

## `Eval` 自动成为叶子动画

只要 `Eval::Output` 可以提取为场景元素，该 evaluator 就自动获得默认 linear、1 秒、enabled 的 `Animation` 实现：

```rust,ignore
pub struct FadeIn<T: FadingRequirement> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> PureEval for FadeIn<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> Self::Output { /* ... */ }
}

// Pure<FadeIn<T>> 本身就是一个可组合动画,不需要 marker 或宏:
let animation = Pure(FadeIn::new(square)).with_duration(1.0);
```

具名 evaluator 和 `Pure(闭包)` 都不会在进入动态容器前擦除类型。`AnimSequence::push`、`AnimStack::push` 或 Scene build 时会将直接子节点转换为保留层级的运行时节点。

## `Paramed<A>`

所有尚未固定父时间坐标的 `Placeable` 动画通过 `AnimationExt` 获得统一的播放参数 API：

```rust,ignore
animation
    .with_duration(2.0)
    .with_rate_func(smooth)
    .with_enabled(true)
```

第一次调用会生成 `Paramed<A>`。它只属于 Animation 层，负责 duration override、rate function 和 enabled，不再实现 `Eval`。裸动画的默认值是 linear、1 秒和 enabled。Sequence 或 Stack 被包装时，rate function 重映射整个组合的局部时间轴。

`At<A>` 表示已经固定在父时间坐标中的 entry，不再实现 `Placeable`，因此参数必须在 placement 之前设置：

```rust,ignore
animation.with_duration(2.0).at(3.0); // At<Paramed<A>>
```

## `Animation` 与 build

所有可组合动画实现：

```rust,ignore
pub trait Animation: Sized {
    fn build(self) -> AnimationCell;
}
```

`Animation` 不再提前暴露 time range 或 duration，它只负责将静态定义 lower 为局部坐标中的 `AnimationCell`：

- 普通叶子 build 为 `0.0..1.0`；
- `Paramed<A>` build 内层后，在外层应用 duration override、rate function 和 enabled；
- `At<A>` build 内层后移动根 time range；
- Sequence push 时先 build 子动画，再将它移动到 cursor；
- Stack push 时先 build 子动画，再根据 built range 更新整体 duration。

`AnimSequence` 和 `AnimStack` 仍提供自己的 `duration_secs()` 查询，但通用 `Animation` trait 不再要求每个静态类型重复提供时间信息。

## `AnimationCell`

Sequence、Stack 和 Scene 需要保存异构动画，因此每个直接子动画会生成一个 `AnimationCell`：

```text
AnimationCell
├─ Box<dyn EvalDyn>
├─ time range
├─ rate function
├─ enabled
└─ evaluator name
```

`EvalDyn` 是私有的 object-safe 求值接口：所有 `E: Eval` 通过 blanket impl 进入类型擦除，`AnimSequence` 和 `AnimStack` 也直接实现该接口。Paramed 直接修改内层 build 出来的 cell，不再额外嵌套一个 `AnimationCell`。`hold` 保存的已求值结果直接使用 `Static<Vec<DynItem>>`。

动态求值会将结果追加到 `Vec<DynItem>`，但组合树本身不会被展开。类型擦除只隐藏直接子动画的 Rust 类型，不删除组合层级。时间范围位于 Box 外，供父动画调度和 preview 查询；cell 也负责把时间配置（time range、rate function）换算成 `Time`/`DeltaTime` 读数后，再调用擦除后的求值器。

## Requirement Trait 模式

用户通常不直接构造 evaluator，而是通过 Item 的动画扩展 Trait：

```rust,ignore
let animation = square
    .fade_in()
    .with_duration(2.0)
    .with_rate_func(smooth);
```

返回值就是适配后的具体类型，例如：

```text
Pure<FadeIn<Square>>
```

只有当它进入 `AnimSequence::push`、`AnimStack::push` 或 `RanimScene::play` 时才会被 build 和擦除。
