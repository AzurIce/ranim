# 动画

## `Eval`

Ranim 的叶子动画核心是一个归一化纯函数：输入进度 `alpha`，输出对应状态 `T`。

```rust,ignore
pub trait Eval {
    type Output;
    fn eval_alpha(&self, alpha: f64) -> Self::Output;
}
```

具体 evaluator 同时保存求值所需的数据。例如 `Static<T>` 始终返回同一个值，`Morph<T>` 保存插值需要的源状态和目标状态。

## Eval 自动成为叶子动画

只要 `Eval::Output` 可以提取为场景元素，该 evaluator 就自动获得默认 linear、1 秒、enabled 的 `Animation` 实现：

```rust,ignore
pub struct FadeIn<T: FadingRequirement> {
    src: T,
    dst: T,
}

impl<T: FadingRequirement> Eval for FadeIn<T> {
    type Output = T;
    fn eval_alpha(&self, alpha: f64) -> Self::Output { /* ... */ }
}
```

因此 `FadeIn<T>` 本身就是一个可组合动画，不需要 marker 或宏。`Fn(f64) -> T` 闭包也自动实现 `Eval<Output = T>`，可以直接设置播放参数：

```rust,ignore
let animation = (|alpha| Square::new(alpha)).with_duration(2.0);
```

具名 evaluator 和闭包都不会在进入动态容器前擦除类型。`AnimSequence::push`、`AnimStack::push` 或 Scene build 时会将直接子节点转换为保留层级的运行时节点。

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

动态求值会将结果追加到 `Vec<DynItem>`，但组合树本身不会被展开。类型擦除只隐藏直接子动画的 Rust 类型，不删除组合层级。时间范围位于 Box 外，供父动画调度和 preview 查询。

## Requirement Trait 模式

用户通常不直接构造 evaluator，而是通过 Item 的动画扩展 Trait：

```rust,ignore
let animation = square
    .fade_in()
    .with_duration(2.0)
    .with_rate_func(smooth);
```

返回值就是具体 evaluator 类型，例如：

```text
FadeIn<Square>
```

只有当它进入 `AnimSequence::push`、`AnimStack::push` 或 `RanimScene::play` 时才会被 build 和擦除。
