# 动画系统

Ranim 的一个场景就是一棵动画树。这棵树分成「定义期」和「运行期」两个层面：

- **定义期（开放）**：`AnimDef<T, R>` 保存输入和配方，通过标准 `Into<AnimDefNode>`
  转换组成定义树。节点内容只有 `Children` 或 `Leaf`；叶子保留完整的输入与配方。
  容器位于 `animation::compose`：`AnimSequence` / `AnimStack` / `AnimLagged`。
- **运行期（封闭核心）**：`AnimDefNode::build(&self)` 在保留定义的同时构建独立的
  `AnimNode`。`AnimNode` 的内容是封闭的 `NodeContent`：
  `Children` 或 `Leaf`（动画 / 音频）。视觉求值、seal 时音频
  烘焙遍历运行树；Inspector 与 timeline 直接读取定义树。

定义用于组织、检查和重建动画；运行节点用于求值和音频烘焙。两者通过下面的链路连接：

```text
Item + 参数 P                          anim / anim_ref / anim_mut
  ▼
struct AnimDef<T, R>                    保留 initial 与 recipe
  ▼
struct Paramed<A> / At<A>               PlaybackExt / Unplaced::at
  ▼
struct AnimSequence / AnimStack / AnimLagged    顺序 / 并行 / 交错容器
  │ Into<AnimDefNode>
  ▼
struct AnimDefNode { playback overrides, Children / Leaf }
  │ build(&self)                        不消费定义
  ▼
struct AnimNode { timing shell, NodeContent }
  NodeContent:
    Children(Vec<AnimNode>)
    Leaf(LeafContent)
      Anim(Box<dyn EvalDyn>)
      Audio(Box<AudioTrack>)
  ▼
RanimScene::seal → SealedRanimScene（定义 + 运行树）→ SceneEvaluator 采样
```

## 定义与执行的边界

**一份动画定义 = 初始状态快照 + 动画配方。**

```rust
pub struct AnimDef<T, R> {
    pub initial: T,
    pub recipe: R,
}

pub trait AnimRecipe<T> {
    fn build(&self, input: &T) -> AnimNode;
}
```

`AnimDef::new(initial, recipe)` 只要求配方能接受这个输入，不要求输入实现 `Eval`，
也不要求输入可克隆。`anim_ref` 和 `anim_mut` 为保留快照而需要的 `Clone`，是这些
绑定方法自己的约束，不是所有定义的约束。

配方负责构建完整的运行节点，包括默认播放窗口；它可以产生视觉叶子、音频叶子或
组合子树，没有 `Eval` 关联类型，也不要求运行输出仍是 `T`。例如 `FadeIn` 保留的是
输入物件和淡入配方；插值用的 `src/dst` 属于 build 出的 `FadeEval`，不是定义字段。

`build(&self)` 借用定义并构建独立执行实例，不消耗输入、不替换配方，也不把运行缓存
写回定义树。保留同一份定义可以重复 build；编辑输入或配方后，新构建的运行树采用
新值，先前构建的运行树不变。迭代动画的积分缓存也在运行实例中重新建立。

### 定义树与完整定义的类型擦除

```rust
pub enum AnimDefNodeContent {
    Children(Vec<AnimDefNode>),
    Leaf(Box<dyn DynAnimDef>),
}

pub struct AnimDefNode {
    pub content: AnimDefNodeContent,
    pub time_range: Option<Range<f64>>,
    pub internal_time_secs: Option<f64>,
    pub rate_func: Option<AnimDefRate>,
    pub enabled: Option<bool>,
    pub end_inclusive: Option<bool>,
    pub anim_name: Option<&'static str>,
}
```

`DynAnimDef` 擦除的是完整定义，而不是已经准备好的 `Eval`。它的 `build(&self)`
构建运行节点；通过 `Any` 可以 downcast 回 `AnimDef<T, R>`，同时访问 `initial` 和
`recipe`。进入这个类型擦除边界时要求定义满足 `'static`，不额外要求 `Send/Sync`。

节点的可选播放字段是 **override**，不是另一份必须与配方同步的默认值：`None` 保留
配方构建出的默认配置。`AnimDefRate::Linear` 表示显式清除配方的自定义速率；
`internal_time_secs` 则可以保留容器已经声明的内容跨度，包括末尾空白。

定义树和运行树都只使用 `Children / Leaf` 这套结构词汇，但不要求节点一一对应。
一个定义叶子可以 build 出整棵运行子树；Inspector 和 timeline 仍把它显示为一个
定义叶子，不根据构建产物反向扩展定义层级。Sequence、Stack、Lagged 是作者侧的
排布方式，不是额外的节点种类。

这项分层不意味着定义期间禁止 build。排布需要自然窗口、`hold()` 需要当前状态，
或 Lagged 需要边缘填充时，可以临时 build 并查询或求值；原定义仍然保留。采样
结果通过现有静态定义保存，不引入延迟采样或兄弟节点构建上下文。

## `Eval`：叶子求值协议

视觉执行器实现统一的求值协议。构建出的动画内容是自身归一化进度
`alpha ∈ [0, 1]` 的纯函数；编辑定义后通过重新 build 准备新的执行器。

```rust
pub trait Eval {
    type Output;

    /// 在归一化进度 alpha 处求值。
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}
```

- 协议只有一个入口：`eval_alpha(&self, alpha)`；
- 它是 `&self` 上的纯查询：无论调用顺序和次数，同一个 `alpha` 得到同一个
  `Output`；
- evaluator 看不到秒、场景时钟或 `logic_fps`。`AnimNode` 负责把场景时间
  映射成进度后才调用它；
- 有状态（迭代）区段在内部记忆化自己的积分快照；纯区段就是闭式。

运行期的 `LeafContent::Anim` 持有 `Box<dyn EvalDyn>`，它是 `Eval` 的擦除
对应物：`Eval` 带有关联类型 `Output`，不能直接作为 trait object，因此
`animation::eval` 为所有满足 `Output: Clone + Send + Sync + 'static` 的 `E: Eval`
自动实现了 `EvalDyn`。普通采样将完整输出直接写入类型化帧；hold 捕获同样的
原始值供后续克隆，不改变其具体类型。
视觉执行器和音频是叶子载荷；组合结构由 `Children` 表示。

除显式播放适配外，`EvalExt` 还提供两个操作已准备执行器的便捷方法：

```rust
pub trait EvalExt: Eval + Sized {
    fn apply_alpha_to(self, item: &mut Self::Output, alpha: f64) -> Self;
    fn apply_to(self, item: &mut Self::Output) -> Self; // alpha = 1.0
}
```

它们用于直接操作已经准备好的执行器。普通 Item 的动画构造则使用
`anim` / `anim_ref` / `anim_mut`，见下文。

### 进度是唯一坐标

`ranim::core::time` 只有两个类型别名：

```rust
pub type Alpha = f64;       // 归一化进度
pub type DeltaAlpha = f64;  // 均匀进度步长
```

「内容即序列」：迭代动画的内容是作者声明的进度点序列 `x₀…x_N`。`N` 是定义而
不是采样精度；`rate_func`、`with_duration`、placement 都只是「哪个进度何时可
见」的采样重映射。

## 内容的两种来源：`Pure` 与 `Iterative`

两者都是 `Eval` 的实现：`Pure` 适配闭式求值，`Iterative` 适配逐步积分。它们与
由具名参数（`FadeIn`、`Morph` 等）生成的执行器共享 `Eval` 协议，只是内容的
产生方式不同。

### 纯闭包：`Pure`

闭包是匿名类型，不能按名字实现 `Eval`，所以用 `Pure` 包一层：

```rust
use ranim::core::animation::eval::pure::Pure;

let animation = Pure::new(|alpha| Square::new(alpha)).with_duration(2.0);
```

自定义的 `Eval` 不会自动得到播放窗口；可克隆的执行器可以用
`.into_animation()` 显式选择一秒默认窗口，每次构建时克隆模板。
`Pure` 的闭包也需要 `Clone`。配方内部可以用
`AnimNode::from_eval(evaluator, duration)` 直接构建运行节点，无需执行器可克隆。

### 迭代区段：`IterativeEval` + `Iterative`

物理模拟、混沌系统等没有闭式的内容，用逐步推进的方式定义：

```rust
pub trait IterativeEval {
    type Output;

    /// 推进一个内容步。alpha 是当前进度，delta_alpha = 1/N。
    fn step(&self, output: &mut Self::Output, alpha: f64, delta_alpha: f64);
}
```

`Iterative::new(initial, evaluator)` 持有不可变的定义（初始状态、`sim_step`、
step 逻辑），把积分快照放在内部 `RefCell<Snapshot>` 中：

```rust
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
- `with_steps(N)` 声明内容自己的步数，默认进度步长为 `1/120`；
- `eval_alpha(target)` 前进时逐 `sim_step` 积分，回退时从初始状态重置重放，
  重复查询同一个 `alpha` 是 O(1)；
- 可变状态全部住在 `Output` 里；
- 闭包的状态类型位于 `Fn` 输入位置，无法从闭包类型反推出关联 `Output`，所以
  `Iterative::from_fn` 通过 `IterativeFn<S, F>` 显式绑定二者。

`Iterative` 实现的 `Eval::sim_step()` 返回 `Some(1/N)`，供 `inspect tree`
等工具内省；它不影响求值本身。
每次由定义构建运行节点时，积分快照从原始输入重新建立，不复用先前运行的缓存。

## 从输入和配方到可播放定义

```rust
pub trait AnimWriteBack<T>: AnimRecipe<T> {
    fn end_value(&self, input: &T) -> T;
}
```

`AnimWriteBack` 是独立的终态写回能力。因为 `AnimRecipe` 不限制运行输出类型，不能
普遍从一个 `AnimNode` 中猜测可写回的 `T`；只有显式实现 `end_value` 的配方才能用于
`anim_mut`。它可以直接计算终态，也可以在内部通过具体的执行器求值。

动画操作参数与输入 Item 分开定义：

```rust
let owned = square.clone().anim(FadeIn);
let borrowed = square.anim_ref(FadeIn);
let write_back = square.anim_mut(FadeOut).with_duration(2.0);
```

- `anim(params)` 接管输入；`anim_ref(params)` 克隆输入，不修改原值。
- `anim_mut(params)` 克隆原始输入，再通过 `AnimWriteBack::end_value` 把终态写回；
  未实现写回能力的配方会在编译期被拒绝。`anim` 和 `anim_ref` 不受此限制。
- 返回的 `AnimDef<T, R>` 保留公开的 `initial` 和 `recipe`，可以修改后重新 `build()`。
- 淡入的 `src/dst`、Morph 的对齐结果、Orbit 的球坐标等只存在于构建出的执行器中。
- `AnimDefNode` 擦除完整定义而非执行器，支持 downcast；`build(&self)` 不消费定义。
  排布和边缘采样需要时可临时 build，普通逐帧求值只读取已构建的运行节点。

例如旋转参数与目标形状分开：

```rust
let animation = square.anim_mut(Rotate {
    angle: std::f64::consts::PI,
    axis: DVec3::Z,
    point: AabbPoint::CENTER,
});
```

Morph 保存明确的目标值。闭包辅助构造只执行一次，不将闭包保存在参数中：

```rust
let params = Morph::by(&square, |target| { target.shift(DVec3::X); });
let animation = square.anim_mut(params);
```

保留的输入和配方可以直接编辑，构建出的实例与后续编辑相互独立：

```rust
let mut definition = square.anim_ref(Morph::to(target));
let first = definition.build();
definition.recipe.target.shift(DVec3::X);
let second = definition.build();
// first 使用原目标；second 使用编辑后的目标。
```

配方构建出的运行节点给出默认播放窗口：内置物件操作为一秒，`Sound` 采用
裁剪后音频长度，组合容器采用子节点跨度。`.with_duration()` 等方法通过
`Paramed<A>` 只覆盖指定字段，不会重置其他默认值。

`ranim-anims` 只包含这类具名叶子动画家族，通用适配器（`Pure` / `Iterative`
/ `Static`）、定义树和运行节点在 `ranim_core::animation` 中：

```text
ranim::anims
├── camera     （Orbit）
├── creation   （Create/UnCreate/Write/Unwrite）
├── fading     （FadeIn/FadeOut）
├── morph      （Morph）
└── rotating   （Rotate）
```

### `Unplaced`、`PlaybackExt`、`Paramed` 与 `At`

所有尚未固定父时间坐标的 `Unplaced` 动画通过 `PlaybackExt` 获得统一的播放
参数 API：

```rust
animation
    .with_duration(2.0)
    .with_rate_func(smooth)
    .with_enabled(true)
```

`At<A>` 表示已经固定在父时间坐标中的 entry，不再实现 `Unplaced`，因此参数
必须在 placement 之前设置：

```rust
animation.with_duration(2.0).at(3.0); // At<Paramed<A>>
```

这些方法定义在 `animation::build`，返回的 `Paramed<A>` / `At<A>` 都支持
标准 `Into<AnimDefNode>`，所以可以继续被容器或 Scene 接纳。

它们配置的是定义节点，而不是提前把定义替换为运行节点。容器排布得到的窗口已经
写在各子定义上；编辑某个子定义不会自动重排兄弟节点，也不会重新执行 Sequence 或
Lagged 的排布规则。需要重新排布时，应重新构造对应容器。

## `animation::compose`：顺序、并行、交错

### 顺序容器：`AnimSequence`

`AnimSequence::push` 将动画转换为局部 `AnimDefNode`，再把它移动到当前
cursor，并按定义窗口时长推进 cursor；未指定的自然窗口通过临时 build 取得：

```rust
let mut intro = AnimSequence::new();
intro
    .push(square.anim_ref(FadeIn))
    .hold(1.0)
    .push(square.anim_mut(FadeOut));

r.play(intro);
```

Sequence 是动态类型擦除边界，但不会展开传入动画的组合树。每次 `push` 只将
直接子动画转换为一个 `AnimDefNode`；如果子动画是 Stack 或 Sequence，其内部层级
会继续保留。Sequence 转换后产生 `AnimDefNodeContent::Children`，构建运行树后
对应 `NodeContent::Children(Vec<AnimNode>)`。

Sequence 自己通过 cursor 决定子动画的位置，因此 `push` 只接受尚未显式放置的
`Unplaced`。`At<A>` 已经固定父时间坐标，不能进入 Sequence。

Sequence 本身实现 `Into<AnimDefNode>` 与 `Unplaced`，所以可以先独立构造，再整体
使用 `at` 放置或加入另一个组合：

```rust
r.play(intro.at(2.0));
```

#### `forward` 与 `hold`

两者都会推进 Sequence cursor，但输出语义不同：

- `forward(secs)` 只推进 cursor，产生的空白区间没有输出。
- `hold(secs)` 当场临时 build 命中的定义，在 cursor 处求值，将快照保存为持续
  `secs` 的静态定义。保存的是可克隆的原始 `Eval::Output`，播放时按原类型写回帧；
  之后修改源定义不会自动更新这份快照。
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

#### `show`、`hide` 与最终求值

`show()` 和 `hide()` 都是普通的零时长动画：

- `show()` 是 enabled 的静态动画，求值时输出对应物件；
- `hide()` 是 disabled 的静态动画，求值时不输出内容。

它们不需要 `hold` 特判。因为 Sequence 在边界上选择最后一个适用的直接子动画，
末尾的 `show()` 会成为最终求值结果，末尾的 `hide()` 则自然得到空结果；`hold`
只负责把这个结果保存为静态动画。

```rust
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

```rust
r.play(square_sequence);
r.play(circle_sequence);
```

如果两个物件需要在同一时刻一起求值，应直接 push 一个 `stack![...]` 组合。

### 并行容器：`AnimStack` 与根场景

`AnimStack::push` 不推进其他子动画；Stack duration 是所有子动画窗口终点的最大值：

```rust
let animation = stack![
    background.show().with_duration(5.0),
    content.at(1.0),
    camera.show().with_duration(5.0),
];

r.play(animation);
```

Stack 接受普通 `Unplaced` 动画和已经放置的 `At<A>`。普通动画从 Stack 局部 0
开始，`At<A>` 使用自己的显式 offset。参数必须在调用 `at` 之前设置。

`RanimScene` 自带一个根 `AnimStack`：

```rust
pub fn play<A: Into<AnimDefNode>>(&mut self, animation: A) -> &mut Self {
    self.root.push(animation);
    self
}
```

因此，多次根级 `play` 默认都从 0 秒开始。它们是并行动画，不存在后一次调用
覆盖前一次调用的隐含对象语义。

运行时数量不固定时可以直接构造 `AnimStack`：

```rust
let mut layers = AnimStack::new();
for animation in animations {
    layers.push(animation);
}
r.play(layers);
```

#### 场景时长与显式生命周期

Scene 总时长是根 Stack 中子动画窗口终点的最大值。seal 不会自动把静态物件和
相机延长到 Scene 结束。

需要全程存在的内容应显式指定生命周期：

```rust
let total_secs = content.cursor_sec();

let mut camera = AnimSequence::new();
camera
    .push(CameraFrame::default().show())
    .hold_to(total_secs);

r.play(camera);
r.play(content);
```

这种写法使空白和保持区间成为动画定义的一部分。

### 交错容器：`AnimLagged`

`AnimLagged` 把一组**未放置**（`Unplaced`）的子动画按 stagger 规则相继排布：
第 `i` 个子动画的起点是 `start_{i-1} + lag_ratio · d_{i-1}`。`lag_ratio` 插值
在两种容器语义之间：

- `0.0` —— 所有子动画同时开始（类似 `AnimStack`）；
- `1.0` —— 首尾相接（类似 `AnimSequence`）；
- 中间值 —— 重叠相继。

```rust
let animation = lagged![0.2;
    square.anim_ref(FadeIn),
    circle.anim_ref(FadeIn),
    text.anim_ref(Write),
];
r.play(animation);
```

子动画窗口之外的时间默认由**真实的静态动画定义**填充：每个元素在转换为 Def 节点时被物化
为一条 `[前填充][动画][后填充]` 的 per-item `AnimSequence` 轨道（前=初态，
后=末态，采样自窗口边缘，空的填充会被跳过），因此 preview 时间线看到的就是
实际渲染的内容，没有隐藏的求值规则。每端的行为可以用
`with_leading`/`with_trailing` 配置（`LaggedFill::{Hold, Empty}`，默认都是
`Hold`）；若希望元素在窗口结束后消失，让它的动画以 `hide` 结尾即可（如
`seq![item.anim_ref(FadeIn), item.hide()]`）。

填充直接临时 build 子定义并在窗口边缘求值，再保存快照；没有延迟采样节点。

对一组元素施加同一个动画时，用迭代器收集（core 的 `AnimIterExt`）：

```rust
let animation = group
    .iter_mut()
    .map(|item| item.anim_mut(FadeIn).with_rate_func(smooth))
    .into_lagged(0.2);
```

迭代器还可以收集为另外两个容器：`into_stack()`/`into_seq()`，或直接
`collect::<AnimStack>()`/`collect::<AnimSequence>()`。

### `seq!`、`stack!` 与 `lagged!`

固定写法可以使用宏简化：

```rust
let intro = seq![
    square.anim_ref(FadeIn),
    square.anim(FadeOut),
];

let scene = stack![intro, camera];
r.play(scene);
```

`seq!` 返回 `AnimSequence`，`stack!` 返回 `AnimStack`。二者都只是构造辅助，
最终构成保留子节点层级的定义树。`lagged![0.2; a, b, c]` 以 0.2 的
stagger ratio 返回 `AnimLagged`（见上文）。

## 运行期：`AnimNode`、`NodeContent` 与 `SceneEvaluator`

Sequence、Stack 和 Scene 保存异构的 `AnimDefNode`。seal 保留这些定义并构建独立的
`AnimNode` 运行树：

```text
AnimNode
├─ content: NodeContent
├─ time_range: Range<f64>         在父坐标中的窗口
├─ internal_time_secs: f64        content 轴长度
├─ rate_func: Option<fn(f64) -> f64>
├─ enabled: bool
├─ end_inclusive: bool           是否允许包含父容器末尾的右端点
└─ anim_name: &'static str
```

`NodeContent` 是封闭的运行时语言：

```text
NodeContent
├─ Children(Vec<AnimNode>)   所有命中的子节点按插入顺序叠加求值
└─ Leaf(LeafContent)
   ├─ Anim(Box<dyn EvalDyn>) 用户 Eval 叶子的类型擦除
   └─ Audio(Box<AudioTrack>) seal 时烘焙的音频叶子
```

`AnimNode::eval_at(sec, out)` 是唯一的时间管理入口：node 先检查
enabled / active，再用自己的 `time_range` 和 `rate_func` 把 `sec` 映射成局部
`alpha`，最后交给 `NodeContent`：

- `Children` 求值所有包含 content 时间的子节点；
- 动画叶子调用擦除后的 `EvalDyn::eval_frame(alpha, ctx)`，将
  `eval_alpha(alpha)` 的完整输出按实际类型写入帧；
- 音频叶子由 seal 时的音频 interpreter 处理。

子片段使用左闭右开区间，父容器末尾允许包含子片段的右端点。
Sequence 在转换为 Def 节点时移除被覆盖的零时长片段，并关闭被末尾
`show()` / `hide()` 覆盖的前一片段的右端点。这个标记是固定的区间元数据，
求值时无需维护可变的显示状态。普通 stack 的重叠片段仍然全部求值。
`hold` 和 lagged 的边缘填充通过 `eval_capture` 保存原始值，内部用
`HeldOutput` 叶子克隆重放；每个被捕获的完整输出仍写回原来的类型批次，连续
hold 不会嵌套捕获。普通叶子无需先产生 `Vec<DynOutput>`。`Vec<T>`、元组、树与
变换 wrapper 等完整输出一直保留到[渲染提取](./extract.md)阶段，再由值级
`Extract` 递归解释。

这里的关键是：运行时核心是封闭的，因此不同 consumer 可以各自遍历同一棵树，
互不污染：

- `eval_at`：逐帧视觉求值；
- `bake_audio`：seal 时对音频叶子做一次性混音；
- `has_audio`：判断树里是否存在音频叶子。

叶子仍然通过 `Eval` 保持开放；如果一个组合子能用核心构造子和窗口 placement
表达，就在 build 时 desugar，而不是增加新的 runtime variant。`AnimLagged`
就是这种 sugar：它最终 lower 成嵌套的 `Children`。

`SceneEvaluator::sample_at(render_secs, out)` 是 session 的逐帧采样入口：

- 对每个顶层 `AnimNode` 调用 `eval_at(render_secs)`；
- 前进 / 回退 / 原地求值的判断在 `Iterative` 等 stateful 节点内部完成；
- preview 拖拽和 render 采样共用同一条路径。

`logic_fps` 参数仅为 API 兼容保留，不再驱动步进；步进尺度由每个迭代区段自己
的 `sim_step` 决定。

## 检查与持久化的边界

`RanimScene` 的根 `AnimStack` 保存定义；seal 后，`SealedRanimScene` 和
`SceneEvaluator` 也继续持有它。Inspector、App 和 preview timeline 直接借用
`definition().definitions()`，只维护选中路径、展开状态和显示布局，不复制另一棵
动画信息树。App 在 UI 线程按需构造定义，因此不必为了跨线程传输 UI 快照而对所有
输入或配方施加 `Send/Sync`。

定义保留了未来 Editor 所需的输入、配方和播放配置，但这不等于已经具备通用的字段
反射或场景持久化能力：

- `Any` 提供的是已知具体类型的 downcast，不会自动枚举未知类型的字段。
- `Animation` derive 和 App 中的注册表描述定义类型家族及其包名、版本、类型身份，
  不会为输入、泛型参数、闭包或函数指针生成序列化 codec。
- `inspect tree` 的 JSON 和 WASM timeline 数据只在输出边界从定义树生成可传输的
  属性快照；它们不是能够还原任意 `AnimDef<T, R>` 的完整场景文件。

## 音频叶子

`Sound` 是音频平面的叶子 atom，使用方式与视觉动画一致（`from_file` 需要
`audio-decode` feature）：

```rust
let mut scene = RanimScene::new();
scene.play(seq![
    square.anim_ref(FadeIn),
    Sound::new(AudioClip::from_file("narration.wav")?)
        .with_fade_in(0.25),
    square.anim_ref(Write),
]);
```

`Sound` 转换为保留音轨配置的定义叶子，build 时产生
`NodeContent::Leaf(LeafContent::Audio(Box<AudioTrack>))`。它同样参与
`seq!` / `stack!` / `lagged!`、`.at()`、`.with_duration()`、
`.with_rate_func()` 和 `.with_enabled()`。音频不进入逐帧视觉管线，而是在
`RanimScene::seal` 时通过 `bake_audio` 一次性混音；见 `ranim-core` 的音频模块
文档。
