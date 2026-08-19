# 动画系统

Ranim 的一个场景就是一棵动画树。谈论这棵树时要区分两个层面：

- **定义期（具体类型）**：叶子是任何实现了 `Eval` 的类型——`FadeIn`、
  `Morph`、`Pure`、`Iterative` 或自定义 struct，`Eval::Output` 是该动画产出的
  item 类型 `T`；容器是 `AnimSequence` / `AnimStack` / `AnimLagged` 三个
  struct；树根是 `RanimScene` 自带的根 `AnimStack`。
- **运行期（类型擦除）**：每个节点统一 lower 为 struct `AnimationCell`，叶子
  在 cell 内部是擦除自 `Eval<Output = T>` 的 trait object（`Box<dyn EvalDyn>`，
  `T` 需可提取为场景元素）。擦除只隐藏直接子节点的 Rust 类型，组合层级本身
  保留。

本章自底向上整理这条链路：

```text
trait Eval<Output = T>                  叶子协议：alpha -> T 的纯函数
  │ 具体叶子 struct：FadeIn / Morph / Pure / Iterative / 自定义 …
  │ 自动实现 trait Animation + Placeable（默认 linear、1 秒、enabled）
  ▼
struct Paramed<A> / At<A>               with_duration / with_rate_func / with_enabled / at
  ▼
struct AnimSequence / AnimStack / AnimLagged   顺序 / 并行 / 交错容器（自身也实现 Animation）
  │ Animation::build（类型擦除，保留组合层级）
  ▼
struct AnimationCell                    运行时节点：Box<dyn EvalDyn> + 时间区间 + rate_func + enabled
  ▼
RanimScene 根 AnimStack  →  SceneEvaluator 采样
```

## `Eval`：叶子求值协议

Ranim 的叶子动画核心是一个统一的求值协议。动画内容一旦定义就不可变：它是自身
归一化进度 `alpha ∈ [0, 1]` 的纯函数。

```rust,ignore
pub trait Eval {
    type Output;

    /// 在归一化进度 alpha 处求值。
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}
```

- 协议只有一个入口：`eval_alpha(&self, alpha)`；
- 它是 `&self` 上的纯查询：无论调用顺序和次数，同一个 `alpha` 得到同一个
  `Output`；
- evaluator 看不到秒、场景时钟或 `logic_fps`。`AnimationCell` 负责把场景时间
  映射成进度后才调用它；
- 有状态（迭代）区段在内部记忆化自己的积分快照；纯区段就是闭式。

`EvalExt` 提供两个 build 期便捷方法：

```rust,ignore
pub trait EvalExt: Eval + Sized {
    fn apply_alpha_to(self, item: &mut Self::Output, alpha: f64) -> Self;
    fn apply_to(self, item: &mut Self::Output) -> Self; // alpha = 1.0
}
```

内置动画的工具方法（`fade_in()` 等）正是靠 `apply_to` 在创建动画的同时把 item
置为动画末态。

### 进度是唯一坐标

`ranim::core::time` 只有两个类型别名：

```rust,ignore
pub type Alpha = f64;       // 归一化进度
pub type DeltaAlpha = f64;  // 均匀进度步长
```

「内容即序列」：迭代动画的内容是作者声明的进度点序列 `x₀…x_N`。`N` 是定义而
不是采样精度；`rate_func`、`with_duration`、placement 都只是「哪个进度何时可
见」的采样重映射。

## 内容的两种来源：`Pure` 与 `Iterative`

两者都是 `Eval` 的实现：`Pure` 适配闭式求值，`Iterative` 适配逐步积分。它们与
具名动画（`FadeIn`、`Morph` 等直接 `impl Eval` 的类型）地位相同，只是内容的
产生方式不同。

### 纯闭包：`Pure`

闭包是匿名类型，不能按名字实现 `Eval`，所以用 `Pure` 包一层：

```rust,ignore
use ranim::core::animation::eval::pure::Pure;

let animation = Pure::new(|alpha| Square::new(alpha)).with_duration(2.0);
```

具名纯动画（`FadeIn`、`Morph`、`Create` 等）直接实现 `Eval`，不需要这个
wrapper。

### 迭代区段：`IterativeEval` + `Iterative`

物理模拟、混沌系统等没有闭式的内容，用逐步推进的方式定义：

```rust,ignore
pub trait IterativeEval {
    type Output;

    /// 推进一个内容步。alpha 是当前进度，delta_alpha = 1/N。
    fn step(&self, output: &mut Self::Output, alpha: f64, delta_alpha: f64);
}
```

`Iterative::new(initial, evaluator)` 持有不可变的定义（初始状态、`sim_step`、
step 逻辑），把积分快照放在内部 `RefCell<Snapshot>` 中：

```rust,ignore
let sim_secs = 4.0;

let animation = Iterative::from_fn(
    SpringState { x: 1.0, v: 0.0 },
    move |state: &mut SpringState, _alpha, delta_alpha| {
        let dt = sim_secs * delta_alpha; // 内容自己的物理秒
        let acc = -K * state.x - C * state.v;
        state.v += acc * dt;
        state.x += state.v * dt;
    },
)
.with_steps(240)
.with_duration(sim_secs);
```

- 逻辑时长用过程中的局部变量（例如 `sim_secs`）捕获，并同时传给
  `with_duration`，不要使用全局 `const`；
- 迭代逻辑较复杂时，实现命名 `IterativeEval` 结构体，把 `sim_secs` 等参数放在
  `self` 上；
- `with_steps(N)` 声明内容自己的步数，默认 `1/120`；
- `eval_alpha(target)` 前进时逐 `sim_step` 积分，回退时从初始状态重置重放，
  重复查询同一个 `alpha` 是 O(1)；
- 可变状态全部住在 `Output` 里；
- 闭包的状态类型位于 `Fn` 输入位置，无法从闭包类型反推出关联 `Output`，所以
  `Iterative::from_fn` 通过 `IterativeFn<S, F>` 显式绑定二者。

`Iterative` 实现的 `Eval::sim_step()` 返回 `Some(1/N)`，供 `ranim inspect tree`
等工具内省；它不影响求值本身。

## 从 `Eval` 到可播放的动画

以一行最常见的代码为例，自顶向下拆开它经过的每一层：

```rust,ignore
let animation = square.fade_in().with_duration(1.0);
```

**第 1 层：`fade_in()`。** 它来自 `ranim-anims` 的 `FadingAnim` trait（对满足
`Opacity + Interpolatable + Clone` 的类型自动实现）。它做两件事：构造具名
evaluator `FadeIn<T>`，并通过 `EvalExt::apply_to` 把 `square` 就地置为动画末
态——所以动画创建完成时，item 本身已经是「播完」的样子，后续 build 出的新
状态都从这个末态出发：

```rust,ignore
fn fade_in(&mut self) -> FadeIn<Self> {
    FadeIn::new(self.clone()).apply_to(self)
}
```

**第 2 层：`FadeIn<T>`。** 它就是一个普通的 `Eval` 实现——持有初末两个状态，
按 `alpha` 插值：

```rust,ignore
pub struct FadeIn<T: FadingRequirement> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> Eval for FadeIn<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> Self::Output {
        self.src.lerp(&self.dst, alpha)
    }
}
```

**第 3 层：blanket impl。** 任何 `Eval` 实现，只要 `Output` 可提取为场景元素
（`AnyExtractCoreItem`），就自动实现 `Animation` 与 `Placeable`，默认参数为
linear、时长 1 秒、enabled：

```rust,ignore
impl<E> Animation for E
where
    E: Eval + 'static,
    E::Output: AnyExtractCoreItem,
{
    fn build(self) -> AnimationCell {
        // rate_func = linear, time_range = 0.0..1.0, enabled = true
        ...
    }
}
```

**第 4 层：`with_duration(1.0)`。** 来自 `AnimationExt`，把动画包成
`Paramed<A>` 携带播放参数（见下节）。

`ranim-anims` 只包含这类具名动画家族，通用适配器（`Pure` / `Iterative`）在
`ranim_core::animation` 中：

```text
ranim::anims
├── camera     （Orbit、CameraFrameAnim）
├── creation   （Create/UnCreate/Write/Unwrite）
├── fading     （FadeIn/FadeOut）
├── morph      （Morph）
└── rotating   （RotatingAnimation）
```

### `Paramed<A>` 与 `At<A>`

所有尚未固定父时间坐标的 `Placeable` 动画通过 `AnimationExt` 获得统一的播放
参数 API：

```rust,ignore
animation
    .with_duration(2.0)
    .with_rate_func(smooth)
    .with_enabled(true)
```

`At<A>` 表示已经固定在父时间坐标中的 entry，不再实现 `Placeable`，因此参数
必须在 placement 之前设置：

```rust,ignore
animation.with_duration(2.0).at(3.0); // At<Paramed<A>>
```

## 顺序容器：`AnimSequence`

`AnimSequence::push` 先将动画 build 为局部 `AnimationCell`，再把它移动到当前
cursor，并按 cell duration 推进 cursor：

```rust,ignore
let mut intro = AnimSequence::new();
intro
    .push(square.clone().fade_in())
    .hold(1.0)
    .push(square.fade_out());

r.play(intro);
```

Sequence 是动态类型擦除边界，但不会展开传入动画的组合树。每次 `push` 只将
直接子动画转换为一个 `AnimationCell`；如果子动画是 Stack 或 Sequence，其内部
层级会继续保留。

Sequence 自己通过 cursor 决定子动画的位置，因此 `push` 只接受尚未显式放置的
`Placeable`。`At<A>` 已经固定父时间坐标，不能进入 Sequence。

Sequence 本身仍实现 `Animation`，所以可以先独立构造，再整体使用 `at` 放置或
加入另一个组合：

```rust,ignore
r.play(intro.at(2.0));
```

### `forward` 与 `hold`

两者都会推进 Sequence cursor，但输出语义不同：

- `forward(secs)` 只推进 cursor，产生的空白区间没有输出。
- `hold(secs)` 取得 cursor 处的 Sequence 状态，将它保存为持续 `secs` 的静态
  运行时节点。
- `forward_to(target)` 和 `hold_to(target)` 是对应的绝对 cursor 版本。

`hold` 没有额外的状态协议，它直接采用 Sequence 在 cursor 处的正常求值结果。
Sequence 在同一时刻只求值最后一个适用的直接子动画；如果这个子动画是 Stack，
则由 Stack 求值其中所有仍然适用的子动画。已经提前结束的 Stack 子动画不会被
自动延长。

```text
child A: [0, 1)
child B: [0, 2)
cursor:        2

hold at 2 -> 只保持 B 的左侧终态
```

连续 `hold` 会分别保存每次调用时的求值结果，形成相邻的静态区间。

### `show`、`hide` 与最终求值

`show()` 和 `hide()` 都是普通的零时长动画：

- `show()` 是 enabled 的静态动画，求值时输出对应物件；
- `hide()` 是 disabled 的静态动画，求值时不输出内容。

它们不需要 `hold` 特判。因为 Sequence 在边界上选择最后一个适用的直接子动画，
末尾的 `show()` 会成为最终求值结果，末尾的 `hide()` 则自然得到空结果；`hold`
只负责把这个结果保存为静态动画。

```rust,ignore
let mut content = AnimSequence::new();
content
    .push(square.show())
    .hold(1.0)
    .push(square.hide())
    .hold(1.0);
```

这里 `hide` 只改变 `content` 这条 Sequence 的状态。它不会查找或影响根 Stack
中另一个独立动画。

如果两个物件需要独立生命周期，应分别使用两个 Sequence：

```rust,ignore
r.play(square_sequence);
r.play(circle_sequence);
```

如果两个物件需要在同一时刻一起求值，应直接 push 一个 `stack![...]` 组合。

## 并行容器：`AnimStack` 与根场景

`AnimStack::push` 不推进其他子动画；Stack duration 是所有子动画 duration 的
最大值：

```rust,ignore
let animation = stack![
    background.show().with_duration(5.0),
    content.at(1.0),
    camera.show().with_duration(5.0),
];

r.play(animation);
```

Stack 接受普通 `Placeable` 动画和已经放置的 `At<A>`。普通动画从 Stack 局部 0
开始，`At<A>` 使用自己的显式 offset。参数必须在调用 `at` 之前设置。

`RanimScene` 自带一个根 `AnimStack`：

```rust,ignore
pub fn play<A: Animation + 'static>(&mut self, animation: A) -> &mut Self {
    self.root.push(animation);
    self
}
```

因此，多次根级 `play` 默认都从 0 秒开始。它们是并行动画，不存在后一次调用
覆盖前一次调用的隐含对象语义。

运行时数量不固定时可以直接构造 `AnimStack`：

```rust,ignore
let mut layers = AnimStack::new();
for animation in animations {
    layers.push(animation);
}
r.play(layers);
```

### 场景时长与显式生命周期

Scene 总时长是根 Stack 中最长子动画的 duration。新模型不会像旧 Timeline 那样
在 seal 时自动把静态物件和相机延长到 Scene 结束。

需要全程存在的内容应显式指定生命周期：

```rust,ignore
let total_secs = content.cursor_sec();

let mut camera = AnimSequence::new();
camera
    .push(CameraFrame::default().show())
    .hold_to(total_secs);

r.play(camera);
r.play(content);
```

这种写法使空白和保持区间成为动画定义的一部分。后续可以增加默认相机或
`through_scene_end` 等辅助 API，但它们不改变 Sequence/Stack 的组合语义。

## 交错容器：`AnimLagged`

`AnimLagged` 把一组**未放置**（`Placeable`）的子动画按 stagger 规则相继排布：
第 `i` 个子动画的起点是 `start_{i-1} + lag_ratio · d_{i-1}`。`lag_ratio` 插值
在两种容器语义之间：

- `0.0` —— 所有子动画同时开始（类似 `AnimStack`）；
- `1.0` —— 首尾相接（类似 `AnimSequence`）；
- 中间值 —— 重叠相继。

```rust,ignore
let animation = lagged![0.2;
    square.fade_in(),
    circle.fade_in(),
    text.write(),
];
r.play(animation);
```

子动画窗口之外的时间默认由**真实的静态动画**填充：每个元素在 build 时被物化
为一条 `[前填充][动画][后填充]` 的 per-item `AnimSequence` 轨道（前=初态，
后=末态，采样自窗口边缘，空的填充会被跳过），因此 preview 时间线看到的就是
实际渲染的内容，没有隐藏的求值规则。每端的行为可以用
`with_leading`/`with_trailing` 配置（`LaggedFill::{Hold, Empty}`，默认都是
`Hold`）；若希望元素在窗口结束后消失，让它的动画以 `hide` 结尾即可（如
`seq![item.fade_in(), item.hide()]`）。

填充在 build 时采样，因此子动画应当是纯（闭式）动画——迭代式子动画的末态
填充会得到其初态。

对一组元素施加同一个动画时，用迭代器收集（core 的 `AnimIterExt`）：

```rust,ignore
let animation = group
    .iter_mut()
    .map(|item| item.fade_in().with_rate_func(smooth))
    .into_lagged(0.2);
```

迭代器还可以收集为另外两个容器：`into_stack()`/`into_seq()`，或直接
`collect::<AnimStack>()`/`collect::<AnimSequence>()`。

### `seq!`、`stack!` 与 `lagged!`

固定写法可以使用宏简化：

```rust,ignore
let intro = seq![
    square.clone().fade_in(),
    square.fade_out(),
];

let scene = stack![intro, camera];
r.play(scene);
```

`seq!` 返回 `AnimSequence`，`stack!` 返回 `AnimStack`。二者都只是构造辅助，
最终 build 为保留子节点层级的运行时动画树。`lagged![0.2; a, b, c]` 以 0.2 的
stagger ratio 返回 `AnimLagged`（见上文）。

## 运行时：`AnimationCell` 与 `SceneEvaluator`

Sequence、Stack 和 Scene 需要保存异构动画，因此每个直接子动画会 lower 成一个
`AnimationCell`：

```text
AnimationCell
├─ Box<dyn EvalDyn>
├─ time range
├─ rate function
├─ enabled
└─ evaluator name
```

`AnimationCell::eval_at(sec, out)` 是唯一的时间管理入口：cell 先检查
enabled / active，再用自己的 `time_range` 和 `rate_func` 把 `sec` 映射成局部
`alpha`，最后调用擦除后的 `eval_dyn(alpha, out)`。

`EvalDyn` 是 `Eval` 的擦除对应物：`Eval` 带有关联类型 `Output`，不能直接作为
trait object，因此 crate 为所有满足 `Output` 可提取为场景元素的 `E: Eval`
自动实现了 `EvalDyn`。容器（Sequence/Stack/Lagged）同样实现它，所以叶子和容器
都能装进同一个 `Box<dyn EvalDyn>`——动态求值会把结果追加到 `Vec<DynItem>`，
但组合树本身不会被展开。类型擦除只隐藏直接子动画的 Rust 类型，不删除组合
层级。

`SceneEvaluator::sample_at(render_secs, out)` 是唯一的 session 交互：

- 对每个顶层 cell 调用 `eval_at(render_secs)`；
- 前进 / 回退 / 原地求值的判断在 `Iterative` 等 stateful 节点内部完成；
- preview 拖拽和 render 采样共用同一条路径。

`logic_fps` 参数仅为 API 兼容保留，不再驱动步进；步进尺度由每个迭代区段自己
的 `sim_step` 决定。
