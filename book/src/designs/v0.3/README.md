# v0.3

## 新增

- 可组合动画编排系统（见 "Composable Animation Arrangement" 一节）
  - `AnimSequence`/`AnimStack` 容器与 `seq!`/`stack!` 宏，`hold`/`forward`/`extend` 等编排 API
  - `AnimLagged` 容器（stagger 排布 + 窗口外静态填充）与 `lagged!` 宏、迭代器容器收集（`collect::<AnimStack>()`/`collect::<AnimSequence>()`、`into_stack`/`into_seq`/`into_lagged`）
  - 播放参数 `Paramed<A>`（`with_duration`/`with_rate_func`/`with_enabled`）与放置 `At<A>`
- 迭代式动画区段：粒子、弹簧、物理模拟等有状态动画（见"迭代式动画区段"一节）
  - 通用求值协议 `Eval`（`sample`/`reset`/`step`）与轻量会话驱动 `SceneEvaluator`（固定逻辑网格、确定性 seek 重放）
  - `Time`/`DeltaTime` 时刻/时段时间上下文
  - 纯/迭代特化：*ranim-anims* 的 `PureEval` + `Pure<E>`、`IterativeEval` + `Iterative<E>`；闭包经 `Pure(|alpha| ...)` / `Iterative::from_fn(state, step_fn)` 成为动画

## BREAKING CHANGES

- 动画组织系统
  - 弃用 `Timeline`（迁移到 `AnimSequence`/`AnimStack`）
  - `AnimationCell<T>` 的泛型参数移除、不再直接构建（`Eval` 类型可直接作为动画使用）；原先 `AnimationCell<T>` 的 `AnimationInfo` 参数改为 `Paramed<A>` 和 `At<A>`
  - *ranim-anims* 中全部内置动画创建工具方法现在默认用 `linear` 速率函数和 `1.0` 持续秒数
- 求值与内置动画 API
  - `Eval<T>` 泛型参数改为关联类型，方法集改为 `sample`/`reset`/`step`（见"迭代式动画区段"一节）；v0.2 的 `eval_alpha` 作为纯求值能力移至 *ranim-anims* 的 `PureEval`
  - 内置动画工具方法返回类型变为 `Pure<...>`（如 `fade_in()` 返回 `Pure<FadeIn<T>>`）
  - `CameraFrame::orbit` 移到 *ranim-anims* 的 `CameraFrameAnim`
  - 删除 *ranim-anims* 的 `Lagged` 求值器与 `lagged` 模块（`LaggedAnim` 糖），stagger 排布改用 `AnimLagged` 容器

## Composable Animation Arrangement

https://github.com/AzurIce/ranim/pull/170

### `AnimSequence` 和 `AnimStack`

Ranim 动画编排的本质是构造动画数据表示并放入集合，在之前的设计中整个 `RanimScene` 通过内部的 `Vec<Timeline>` 来维护动画。

`Timeline` 的本质是 `Vec<Box<dyn CoreItemAnimation>>` 动画序列容器，其中的每个元素都是前后相继的动画表示，同一时间一个 `Timeline` 只有一个动画激活，于是以前在动画组合代数上非常局限：
- 串行的动画必须通过 `Timeline` 的 API 手动推进/同步时间到对应位置
- 并行的动画必须通过创建新的 `Timeline` 来实现
- 整个场景的 `Vec<Timeline>` 本质是一次性并行组合多个串行编排的性质

在 Ranim v0.3 中，原本的 `Timeline` 被弃用，新增了两个可组合的基本动画容器 `AnimSequence` 和 `AnimStack`。

比如对于如下的动画：

- 正方形：0.0s ~ 1.0s 淡入 | 1.0s ~ 2.0s 变成圆形 | 2.0s ~ 3.0s 淡出
- 文字：0.5s ~ 1.5s 写入 | 1.5s ~ 2.5s 擦除

在以前的 Timeline API 下要这样编写：

```rust
let r_vitem = r.insert_with(|t| {
    t.play(item.fade_in())
        .play(item.morph_to(VItem::from(Circle::default())))
        .play(item.fade_out())
});
let r_text = r.insert_with(|t| {
    t.forward(0.5)
        .play(text.write())
        .play(text.unwrite())
});
```

而使用 `AnimSequence` 和 `AnimStack` 可以这样：

```rust
let anim = stack![
    seq![
        item.fade_in(),
        item.morph_to(VItem::from(Circle::default())),
        item.fade_out(),
    ],
    seq![
        text.write(),
        text.unwrite()
    ].at(0.5)
];
r.play(anim);
```

其中的 `seq!` 和 `stack!`（类似 `vec!`），会构造 `AnimSequence` 和 `AnimStack` 并将动画插入其中（类似 `Vec`）。

如果要把这段动画播放两遍，原来的 Timeline API 会非常繁琐，或许需要将相关时间线操作封装为闭包，而对于新的可组合 API 很简单：

```rust
r.play(seq![anim.clone(), anim]);
```

更能够表现新系统的可组合与复用能力的例子见 `composable_choreaography` example。

### `AnimationCell`、`Eval` 与 `Animation` Trait

`Eval<T>` 的泛型参数被移除并改成了关联类型（一个求值器类型的求值结果类型是唯一的）。

以前 `AnimationCell<T>` 被当作动画的组织单元，所有动画必须被表示为 `AnimationCell<T>` 才能够被插入时间线。现在这个行为被抽象为了一个 `Animation` Trait：

```rust
/// A statically typed animation definition that can be lowered into a runtime animation.
pub trait Animation: Sized {
    /// Lower this definition into its local runtime representation.
    fn build(self) -> AnimationCell;
}
```

同时泛型参数被从 `AnimationCell` 移除，其内部变成类型擦除的 `Box<dyn EvalDyn>`。

所有的 `E: Eval where E::Output: AnyExtractCoreItem` 都自动实现了 `Animation`，于是所有的动画创建都不必返回 `AnimationCell`，可以直接返回自己就可以使用。

```rust
// previous
impl<T: FadingRequirement + Sized + 'static> FadingAnim for T {
    fn fade_in(&mut self) -> AnimationCell<Self> {
        FadeIn::new(self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
    fn fade_out(&mut self) -> AnimationCell<Self> {
        FadeOut::new(self.clone())
            .into_animation_cell()
            .with_rate_func(smooth)
            .apply_to(self)
    }
}
```

```rust
impl<T: FadingRequirement + Sized + 'static> FadingAnim for T {
    fn fade_in(&mut self) -> Pure<FadeIn<Self>> {
        Pure(FadeIn::new(self.clone())).apply_to(self)
    }
    fn fade_out(&mut self) -> Pure<FadeOut<Self>> {
        Pure(FadeOut::new(self.clone())).apply_to(self)
    }
}
```

`Animation` Trait 也是可组合动画的核心，`AnimSequence`、`AnimStack`、`Paramed<A>` 和 `At<A>` 也实现了该 Trait，可以当作一个动画使用。

### `AnimLagged` 与迭代器收集

stagger 排布由 `AnimLagged` 容器表达：

```rust
let animation = lagged![0.2; a.fade_in(), b.fade_in(), c.write()];
```

- 子动画要求 `Placeable`（和 `AnimSequence` 一样），放置由容器计算：`start_i = start_{i-1} + lag_ratio · d_{i-1}`。`lag_ratio` 因此是 `AnimStack`（0.0，同时）与 `AnimSequence`（1.0，相继）之间的插值；
- 窗口外时间由 `with_leading`/`with_trailing` 配置（`LaggedFill::{Hold, Empty}`，默认都 `Hold`）：每个元素在 build 时被物化为一条 `[前填充][动画][后填充]` 的 per-item `AnimSequence` 轨道（前=初态、后=末态，采样自窗口边缘；空填充跳过；零时长子项跳过前填充）——preview 时间线所见即所得，没有隐藏的钳制规则。想让元素窗口后消失，让它的动画以 `hide` 结尾（`seq![item.fade_in(), item.hide()]`）；
- 由于填充在 build 时采样，子动画应当是纯（闭式）动画；
- 子动画是完整的 `Animation`：可以自带 `with_rate_func`/`with_duration` 等播放参数，容器在各自 cell 上施加速率；
- 配套迭代器 API：`collect::<AnimStack>()`/`collect::<AnimSequence>()`（`FromIterator`）与 `AnimIterExt::{into_stack, into_seq, into_lagged}`。

### `Paramed<A>`、`At<A>`

动画本身在时间轴上“长什么样子”并不依赖于其起始时间，只有在要 **放置** 在某种时间坐标上的时候起始时间才存在作用。对于 `AnimSequence` 和 `AnimStack` 来说，前者反而要求动画没有被指定起始时间，因为动画要被相继紧接着放置进序列中。

原先统一在 `AnimationInfo` 内的动画参数现在拆分到了 `Paramed<A>` 和 `At<A>` 两个泛型结构体内：

```rust
/// An animation definition with overridden playback parameters.
pub struct Paramed<A> {
    inner: A,
    param: AnimationParam,
}

/// An animation fixed at an offset in its parent's time coordinates.
///
/// This is a terminal placement entry: it implements [`Animation`] but not
/// [`Placeable`], so playback parameters must be configured before calling
/// [`Placeable::at`].
pub struct At<A> {
    inner: A,
    offset_sec: f64,
}
```

使用 `.with_duration`、`with_rate_func`、`with_enabled` 会自动修改或包裹 `Paramed<A>`，使用 `.at` 会自动包裹 `At<A>`。

## Preview App 时间轴控件重构

在新的动画组织系统下，Preview App 的时间轴控件也对应做了大幅重构：

![composable_choreography.png](./composable_choreography.png)

## ECS Schedule 取代 RenderGraph

https://github.com/AzurIce/ranim/pull/175

渲染侧的 ECS 化：渲染原语进入内部 `RenderWorld`，渲染准备与 GPU pass 由 schedule 组织；用户级 item、动画求值仍停留在 World 之外。

### 之前：`CoreItemStore` 兼任传输与查询

旧实现里，求值结果由 `CoreItemStore` 承载：

```rust
/// A store of [`CoreItem`]s.
#[derive(Default, Clone)]
pub struct CoreItemStore {
    /// Id of [`CameraFrame`]s
    pub camera_frame_ids: Vec<(usize, usize)>,
    /// [`CameraFrame`]s
    pub camera_frames: Vec<CameraFrame>,

    /// Id of [`VItem`]s
    pub vitem_ids: Vec<(usize, usize)>,
    /// [`VItem`]s
    pub vitems: Vec<VItem>,

    /// Id of [`MeshItem`]s
    pub mesh_item_ids: Vec<(usize, usize)>,
    /// [`MeshItem`]s
    pub mesh_items: Vec<MeshItem>,
}
```

它既用于承载并传输求值结果，又用于渲染管线查询访问——两种职责混在一起。

### 现在：`RenderFrame` 传输 + `RenderWorld` 查询

拆分为了 `RenderFrame`（帧级传输缓冲）和 `Renderer` 内部的 ECS World：

```rust
/// A reusable, frame-local transport buffer between evaluation and rendering.
#[derive(Default)]
pub struct RenderFrame {
    items: Vec<(CoreItemId, CoreItem)>,
}
```

```rust
pub struct Renderer {
    width: u32,
    height: u32,
    world: World,
}
```

前者只用于传输（求值线程 → 渲染线程），后者用于承载运行时的查询、变更检测与 schedule。

### Reconcile：按身份增量更新实体

每帧从 `RenderFrame` 更新 `World`，再运行渲染 Schedule：

```rust
/// Reconcile and render one evaluated frame.
pub fn render_frame(
    &mut self,
    render_textures: &mut RenderTextures,
    clear_color: wgpu::Color,
    frame: &RenderFrame,
) {
    reconcile(&mut self.world, frame);
    self.world
        .insert_resource(FrameTarget::new(render_textures, clear_color));
    self.world.run_schedule(RenderPrepare);
    self.world.run_schedule(RenderGraph);
}
```

reconcile 以 `CoreItemIdentity(animation_id, part)` 为跨帧 key，从 `RenderFrame` 更新渲染 `World`：

- 每个实体携带 `CoreItemIdentity` 与 `SceneOrder`；值相同则不写组件（保留 `Changed<T>`），值变化才替换，本帧消失的 key 对应实体被移除；
- **身份与顺序是两件事**：`CoreItemIdentity` 回答"是否是上一帧的同一项"，`SceneOrder` 回答"本帧按什么顺序消费"——ECS query 顺序不构成绘制顺序，prepare 阶段显式按 `SceneOrder` 排序分桶；

### Schedule 组织渲染阶段

```text
RenderPrepare:  Collect → PrepareResources → Upload → PrepareBindGroups
RenderGraph:    Begin → Render → Submit → Finish
  └─ ViewRender: Clear → Compute → Depth → Color → OITResolve
```

- `RenderPrepare` 把组件展开为 GPU 输入（storage/index/uniform 数据、上传、绑定组）；
- `RenderGraph` 驱动整个画面生命周期：`Begin` 创建 frame encoder，`Render` 运行逐 view 子 schedule，`Submit` 提交 command buffer，`Finish` 结束 profiling frame；
- 单相机也走完整的 `ViewRender` 子 schedule（clear、VItem compute、depth、color、OIT resolve），避免单 view 成为以后多 view 的特殊路径；
- 自制的 `Graph<NodeKey, Box<dyn RenderNode>>` 节点图被移除——节点 trait、拓扑容器和查询都在重复 ECS schedule 已提供的能力。

## 迭代式动画区段

https://github.com/AzurIce/ranim/pull/177

v0.2 的动画区段都是**函数式**的：从归一化进度闭式采样。这类区段无法表达**有状态的迭代式动画**（粒子、弹簧、物理模拟、三体），因为求值器无法保留跨帧状态、也无法按 `dt` 推进。v0.3 用一套通用求值协议统一了两类区段。

### 通用求值协议：单一 `Eval` trait

纯（闭式）与迭代（有状态）区段底层是同一个协议——运行时对一个区段能做的全部事情，三个方法全部 required：

```rust
pub trait Eval {
    type Output;

    /// 在时刻采样输出（不携带任何 delta；采样不推进状态）。
    fn sample(&self, time: &Time) -> Self::Output;
    /// 回到区段初始状态（确定性契约：不依赖墙钟/未播种 RNG）。
    fn reset(&mut self);
    /// 推进一个逻辑步。
    fn step(&mut self, time: &Time, delta_time: &DeltaTime);
}
```

外层操作全部由这三个动词组合而成：render 逐帧采样与纯查询（`SealedRanimScene::eval_at_sec`）走 `sample`（纯区段的闭式就是它的 `sample`）；`SceneEvaluator::advance_to` 逐 tick 驱动 `step`；preview 的 seek（scrub）= 全员 `reset` + 会话层重放 `step`——seek 不是区段级原语（逻辑网格归会话所有，区段无法自己重放，因此协议里是 `reset` 而非 `seek`）。

`apply_to`/`apply_alpha_to` 是经 `sample` 定义的通用便捷方法（把动画在 alpha 处的状态写入对象，构建动画的同时把 item 置为末态），对纯、迭代区段语义一致（迭代区段应用的是当前投影状态）。

### 特化在 ranim-anims：`Pure` 与 `Iterative`

`Eval` 是**通用**协议（有状态机形态），纯区段是它的无状态特化。作者不直接实现 `Eval`（除非异形），而是实现两个能力 trait 之一，再由对应的适配结构体特化成完整协议：

```rust
// ranim-anims
pub trait PureEval {              // 纯（闭式）能力：一个方法，无默认实现
    type Output;
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}
pub trait IterativeEval {         // 迭代能力：关联输出 + 一个方法，无默认实现
    type Output;
    fn step(&self, output: &mut Self::Output, time: &Time, delta_time: &DeltaTime);
}

pub struct Pure<E>(pub E);            // sample = eval_alpha(time.alpha)；reset/step 平凡
pub struct IterativeFn<S, F> { ... }  // 把闭包 F 的可变输入 S 绑定为唯一 Output
pub struct Iterative<E> { ... }       // 持有 E::Output 的初始/当前状态;sample = 克隆当前状态,reset = 恢复初始状态
```

- `Fn(f64) -> T` 闭包自动实现 `PureEval`；迭代闭包的 `S` 位于 `Fn` 输入位置，stable Rust 无法从闭包类型反推出关联 `Output`，所以 `Iterative::from_fn(initial, step_fn)` 用 `IterativeFn<S, F>` 显式绑定二者；
- 能力 trait 没有任何默认实现；迭代侧连 `reset` 都不需要——适配器持有初始状态值，恢复是结构性的；
- stable Rust 不允许对两个能力家族各写一个 `Animation` blanket impl（overlap），适配结构体同时解决了这个 coherence 问题：`Animation`/`Placeable` 是对 `E: Eval` 的单一 blanket impl，两个结构体免费获得 `play`/`with_duration`/`at`；
- `Eval` 公开且可直接实现，留给异形区段（M2 world-dependent 将走这条路径）；
- `AnimationCell` 内部是 `Box<dyn EvalDyn>`，`EvalDyn` 的方法集就是协议本身（`sample_dyn`/`reset_dyn`/`step_dyn`）；纯查询对任何区段都有定义——迭代区段返回当前已推进状态。
- 迭代区段的状态与要渲染的内容不同时，为状态类型实现 `Extract`（投影点，每帧一次），如 `nbody` 的 bodies+trails → `VItem`。

### 时刻与时段：`Time` / `DeltaTime`

时间上下文按"时刻 / 时段"分为两个类型（`ranim-core::time`）——点与段是不同代数（借 std `Instant`/`Duration` 的原则，但不借其类型：`Instant` 无法表达逻辑时刻，`Duration` 无符号且 `alpha` 无量纲）：

```rust
pub struct Time {
    pub alpha: f64,        // r((t-s)/D)，由 cell 算好
    pub global_secs: f64,  // 真实全局时刻
}

pub struct DeltaTime {
    pub alpha: f64,        // Δalpha，随 rate 逐帧变化
    pub global_secs: f64,  // Δt = 1/logic_fps，恒稳
}
```

- `sample` 只收 `&Time`——采样在类型层面拿不到 delta；`step` 两者都收（时变力读时刻，积分读时段）；
- **动画逻辑只见时间读数、不见时间配置**：起点、时长、rate 属于 `AnimationCell`，由它算出 `alpha`/`Δalpha` 后传入（零时长 cell 的 `alpha = 1.0` 特判也收在 cell）；
- `with_duration`/`with_rate_func` 对纯、迭代**统一为纯播放变换**（抻拉/扭曲局部时钟，不改变内容）；
- 迭代区段**模拟多久是它自己的参数**（如 `nbody` 的场景常量 `SIM_SECS`），`step` 里用 `SIM_SECS · delta_time.alpha` 换算回逻辑秒，物理参数保持量纲；于是 `with_duration(16.0)` 就是"32s 物理 2 倍速播放"；
- 需要墙钟真实时间的区段读 `global_*`：从会话时钟出发、沿任意嵌套深度的容器原样下传，任何嵌套深度下都是真实全局时间；
- 迭代区段按**变步长积分**编写：非线性 rate 下 `Δalpha` 逐帧变化，非单调 rate 下可以为负。

### `SceneEvaluator`：轻量会话驱动（非 ECS）

```rust
impl SceneEvaluator {
    /// 渲染采样时刻驱动：内部把 `render_secs` floor 到逻辑刻并推进。
    /// 唯一包含 tick 推进逻辑的入口；全局时间通道在这里产生并沿 step 递归下传。
    fn advance_to(&mut self, render_secs: f64);

    /// 纯采样：只读内部 clock（= floor 逻辑刻），不含 tick 逻辑。
    fn sample_into(&self, out: &mut Vec<((usize, usize), CoreItem)>);

    /// seek：全量 reset + 重放（确定性契约）。
    fn seek(&mut self, render_secs: f64);
}
```

- **逻辑帧与渲染帧分离**：固定逻辑网格（默认 120Hz，与 24/30/60/120 整除对齐）驱动模拟，渲染 fps 只决定读取哪些逻辑态；
- **确定性**：`seek` 重放与正向推进逐帧一致（preview scrub 与渲染可复现）；
- **迭代区段要求 `SceneEvaluator`**：纯 `eval_at_sec` 路径不推进其状态（它返回的是当前已推进状态，通常是初始状态）。

### 示例

- `iterative_spring`：阻尼弹簧（`Iterative::from_fn(SpringState { x: 1.0, v: 0.0 }, |state, _t, dt| ...)`，逻辑时长为场景常量 `SIM_SECS = 4.0`）；
- `nbody`：三体引力模拟（`NBodyState` + 闭包步进，32s 物理；velocity Verlet、混沌弹射终场、无边界）；
- `cloth_wrap`：零重力布料（弹簧力 + 自碰撞 + 球-布碰撞，MeshItem 曲面渲染；球的 kinematic 状态由 `step` 依墙钟 `global_secs` 驱动并保存，`Extract` 投影）。
