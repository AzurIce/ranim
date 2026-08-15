# 动画

## `Eval`

Ranim 的叶子动画核心是一个统一的求值协议。动画内容一旦定义就不可变：它是自身归一化进度 `alpha ∈ [0, 1]` 的纯函数。

```rust,ignore
pub trait Eval {
    type Output;

    /// 在归一化进度 alpha 处求值。
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}
```

- 协议只有一个入口：`eval_alpha(&self, alpha)`；
- 它是 `&self` 上的纯查询：无论调用顺序和次数，同一个 `alpha` 得到同一个 `Output`；
- evaluator 看不到秒、场景时钟或 `logic_fps`。`AnimationCell` 负责把场景时间映射成进度后才调用它；
- 有状态（迭代）区段在内部记忆化自己的积分快照；纯区段就是闭式。

`EvalExt` 提供两个 build 期便捷方法：

```rust,ignore
pub trait EvalExt: Eval + Sized {
    fn apply_alpha_to(self, item: &mut Self::Output, alpha: f64) -> Self;
    fn apply_to(self, item: &mut Self::Output) -> Self; // alpha = 1.0
}
```

内置动画的工具方法（`fade_in()` 等）正是靠 `apply_to` 在创建动画的同时把 item 置为动画末态。

## 进度是唯一坐标

`Time` / `DeltaTime` / `GlobalTime` 已从协议中删除。`ranim::core::time` 只剩两个类型别名：

```rust,ignore
pub type Alpha = f64;       // 归一化进度
pub type DeltaAlpha = f64;  // 均匀进度步长
```

“内容即序列”：迭代动画的内容是作者声明的进度点序列 `x₀…x_N`。`N` 是定义而不是采样精度；`rate_func`、`with_duration`、placement 都只是“哪个进度何时可见”的采样重映射。

## 通用适配器：`PureFunc` 与 `Iterative`

两个 author-facing 适配器现在都在 `ranim_core::animation` 中。

### 纯闭包：`PureFunc`

闭包是匿名类型，不能按名字实现 `Eval`，所以用 `PureFunc` 包一层：

```rust,ignore
use ranim::core::animation::eval::pure::PureFunc;

let animation = PureFunc::new(|alpha| Square::new(alpha)).with_duration(2.0);
```

具名纯动画（`FadeIn`、`Morph`、`Create` 等）直接实现 `Eval`，不需要这个 wrapper。

### 迭代区段：`IterativeEval` + `Iterative`

```rust,ignore
pub trait IterativeEval {
    type Output;

    /// 推进一个内容步。alpha 是当前进度，delta_alpha = 1/N。
    fn step(&self, output: &mut Self::Output, alpha: f64, delta_alpha: f64);
}
```

`Iterative::new(initial, evaluator)` 持有不可变的定义（初始状态、`sim_step`、step 逻辑），把积分快照放在内部 `RefCell<Snapshot>` 中：

```rust,ignore
let animation = Iterative::from_fn(
    SpringState { x: 1.0, v: 0.0 },
    |state: &mut SpringState, _alpha, delta_alpha| {
        let dt = SIM_SECS * delta_alpha; // 内容自己的物理秒
        let acc = -K * state.x - C * state.v;
        state.v += acc * dt;
        state.x += state.v * dt;
    },
)
.with_steps(240);
```

- `with_steps(N)` 声明内容自己的步数，默认 `1/120`；
- `eval_alpha(target)` 前进时逐 `sim_step` 积分，回退时从初始状态重置重放，重复查询同一个 `alpha` 是 O(1)；
- 常量放闭包捕获或命名 step 结构体；可变状态全部住在 `Output` 里；
- 闭包的状态类型位于 `Fn` 输入位置，无法从闭包类型反推出关联 `Output`，所以 `Iterative::from_fn` 通过 `IterativeFn<S, F>` 显式绑定二者。

## `Eval` 自动成为叶子动画

只要 `Eval::Output` 可以提取为场景元素，该 evaluator 就自动获得默认 linear、1 秒、enabled 的 `Animation` 实现：

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

let animation = square.fade_in().with_duration(1.0);
```

## 内置动画家族

`ranim-anims` 现在只包含具名动画家族，不再承载通用适配器：

```text
ranim::anims
├── camera     （Orbit、CameraFrameAnim）
├── creation   （Create/UnCreate/Write/Unwrite）
├── fading     （FadeIn/FadeOut）
├── morph      （Morph）
└── rotating   （RotatingAnimation）
```

```rust,ignore
use ranim::{
    anims::fading::FadingAnim,
    prelude::*,
};
```

## `Paramed<A>` 与 `At<A>`

所有尚未固定父时间坐标的 `Placeable` 动画通过 `AnimationExt` 获得统一的播放参数 API：

```rust,ignore
animation
    .with_duration(2.0)
    .with_rate_func(smooth)
    .with_enabled(true)
```

`At<A>` 表示已经固定在父时间坐标中的 entry，不再实现 `Placeable`，因此参数必须在 placement 之前设置：

```rust,ignore
animation.with_duration(2.0).at(3.0); // At<Paramed<A>>
```

## `AnimationCell`

Sequence、Stack 和 Scene 需要保存异构动画，因此每个直接子动画会 lower 成一个 `AnimationCell`：

```text
AnimationCell
├─ Box<dyn EvalDyn>
├─ time range
├─ rate function
├─ enabled
└─ evaluator name
```

`AnimationCell::eval_at(sec, out)` 是唯一的时间管理入口：cell 先检查 enabled / active，再用自己的 `time_range` 和 `rate_func` 把 `sec` 映射成局部 `alpha`，最后调用擦除后的 `eval_dyn(alpha, out)`。

动态求值会把结果追加到 `Vec<DynItem>`，但组合树本身不会被展开。类型擦除只隐藏直接子动画的 Rust 类型，不删除组合层级。

## `SceneEvaluator`

`SceneEvaluator::sample_at(render_secs, out)` 是唯一的 session 交互：

- 对每个顶层 cell 调用 `eval_at(render_secs)`；
- 前进 / 回退 / 原地求值的判断在 `Iterative` 等 stateful 节点内部完成；
- preview 拖拽和 render 采样共用同一条路径。

`logic_fps` 参数仅为 API 兼容保留，不再驱动步进；步进尺度由每个迭代区段自己的 `sim_step` 决定。
