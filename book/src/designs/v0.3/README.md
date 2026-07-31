# v0.3

## 新增

## BREAKING CHANGES

- 重构动画组织系统
  - 弃用 `Timeline`，用 `AnimSequence` 和 `AnimStack` 替代
  - 修改 `Eval<T>` Trait 的泛型参数为关联类型
  - 支持直接将 `Eval<T>` 当作动画使用（不再需要转换为 `AnimationCell`）
  - 用 `Paramed<A>` 和 `At<A>` 替代原先 `AnimationCell<T>` 的 `AnimationInfo`
  - *ranim-anims* 中全部内置动画创建工具方法现在默认用 `linear` 速率函数和 `1.0` 持续秒数。

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
    fn fade_in(&mut self) -> FadeIn<Self> {
        FadeIn::new(self.clone()).apply_to(self)
    }
    fn fade_out(&mut self) -> FadeOut<Self> {
        FadeOut::new(self.clone()).apply_to(self)
    }
}
```

`Animation` Trait 也是可组合动画的核心，`AnimSequence`、`AnimStack`、`Paramed<A>` 和 `At<A>` 也实现了该 Trait，可以当作一个动画使用。

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

之前 Ranim 的求值结果由 `CoreItemStore` 承载：

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

既用于承载并传输求值结果，又用于渲染管线查询访问，

现在拆分为了 `RenderFrame` 和 `Renderer` 内部的 ECS World：

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

前者只用于传输，而后者用于承载运行时的查询。

每帧从 `RenderFrame` 更新 `World` 并运行渲染 Schedule：

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
## 迭代式动画区段

https://github.com/AzurIce/ranim/pull/177

v0.3 之前的动画区段都是**函数式**的：`Eval::eval_alpha(alpha)` 从归一化进度闭式采样。这类区段无法表达**有状态的迭代式动画**（粒子、弹簧、物理模拟、三体），因为求值器无法保留跨帧状态、也无法按 `dt` 推进。

### 统一求值器：`Evaluator` 与 `Eval`

新增公共求值器 `Evaluator`，`Eval` 保持原名原形：

```rust
/// 统一求值器：cell 驱动的统一入口（擦除层操作它）。
/// 迭代式区段直接实现它（sample 必需，reset/step 默认）。
pub trait Evaluator {
    type Output;

    /// 采样当前状态（统一入口）。函数式由 blanket 派生；迭代式显式实现。
    fn sample(&self, time: &SegmentTime) -> Self::Output;

    /// 回到区段起点（确定性契约：不得依赖墙钟/未播种 RNG）。
    fn reset(&mut self) {}

    /// 推进一个逻辑步或 substep；`time.local_delta_secs` 是积分步长。
    /// 函数式默认空操作（免费）；采样不受 step 历史影响。
    fn step(&mut self, _time: &SegmentTime) {}
}

/// 函数式：保留现有名字与形态，现有 impl 零迁移。
/// `sample` 由 blanket 自动派生自 `eval_alpha`。
pub trait Eval {
    type Output;

    /// 闭式采样。
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}

// 关键：blanket 让函数式区段免费获得 Evaluator——作者只写一个 impl。
impl<E: Eval> Evaluator for E {
    type Output = E::Output;

    fn sample(&self, time: &SegmentTime) -> Self::Output {
        self.eval_alpha(time.alpha)
    }
}
```

作者视角：

- **函数式**：`impl Eval { type Output; eval_alpha }`——一个 impl，与之前完全一致（零迁移）；
- **迭代式**：`impl Evaluator { type Output; sample; reset; step }`——一个 impl，没有 `eval_alpha`。

cell 对擦除后的公共类型**无条件**调 `step`：函数式空步免费，且消除了"忘了标记导致 step 被跳过"的 footgun。

### `SegmentTime`：传给区段的完整时间上下文

```rust
pub struct SegmentTime {
    pub global_secs: f64,          // 全局时间 t（秒）
    pub global_delta_secs: f64,    // 逻辑步长（恒稳，= 1/logic_fps）
    pub start_secs: f64,           // 区段起点 s
    pub duration_secs: f64,        // 区段时长 D
    pub local_secs: f64,           // 局部时间 u(t) = D·r((t−s)/D)（秒）
    pub local_delta_secs: f64,     // Δu = u(t_k) − u(t_{k−1})，随 rate 变化（秒）
    pub alpha: f64,                // local_secs / D
    pub render_frame: u64,         // 当前渲染帧序号（frame-coupled 内容用）
    pub is_render_frame_boundary: bool,
}
```

- `global_delta_secs` 恒稳（逻辑网格构造保证）；`local_delta_secs` 仅在线性 rate 下等于逻辑步长——非线性 rate 下逐帧变化是 rate func 的本职（扭曲局部时钟），迭代区段按**变步长积分**编写；
- 需要物理真实时间（不被 rate 扭曲）的区段改用 `global_delta_secs`。

### `SceneEvaluator`：轻量会话驱动（非 ECS）

```rust
impl SceneEvaluator {
    /// 渲染采样时刻驱动：内部把 `render_secs` floor 到逻辑刻并推进。
    /// 唯一包含 tick 推进逻辑的入口。
    fn advance_to(&mut self, render_secs: f64);

    /// 纯采样：只读内部 clock（= floor 逻辑刻），不含 tick 逻辑。
    fn sample_into(&self, out: &mut Vec<((usize, usize), CoreItem)>);

    /// seek：全量 reset + 重放（确定性契约）。
    fn seek(&mut self, render_secs: f64);
}
```

- **逻辑帧与渲染帧分离**：固定逻辑网格（默认 120Hz，与 24/30/60/120 整除对齐）驱动模拟，渲染 fps 只决定读取哪些逻辑态；
- **确定性**：`seek` 重放与正向推进逐帧一致（preview scrub 与渲染可复现）；
- **迭代区段要求 `SceneEvaluator`**：纯 `eval_at_sec` 路径不推进其状态。

### 示例

- `iterative_spring`：阻尼弹簧（`Evaluator` 驱动）；
- `nbody`：三体引力模拟（velocity Verlet、混沌弹射终场、无边界）；
- `cloth_wrap`：零重力布料（弹簧力 + 自碰撞 + 球-布碰撞，MeshItem 曲面渲染，球穿布后布料包裹）。
