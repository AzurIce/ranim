# 动画

## `Eval<T>`

Ranim 的叶子动画核心是一个归一化纯函数：输入进度 `alpha`，输出对应状态 `T`。

```rust,ignore
pub trait Eval<T> {
    fn eval_alpha(&self, alpha: f64) -> T;
}
```

具体 evaluator 同时保存求值所需的数据。例如 `Static<T>` 始终返回同一个值，`Morph<T>` 保存插值需要的源状态和目标状态。

## `AnimationCell<T, E>`

`AnimationCell<T, E>` 将具体 evaluator `E` 与局部时间参数组合：

```text
AnimationCell<T, E>
├─ E: Eval<T>
├─ duration
├─ rate function
├─ enabled
└─ evaluator name
```

它不保存全局 `start_sec`。动画只有被 `At` 放置，或进入 `AnimSequence`、`AnimStack` 和 Scene 时，才需要确定开始位置。

与旧实现不同，`E` 在 build 前直接内联存储，不会先擦除为 `Box<dyn Eval<T>>`。叶子动画在进入动态组合容器前始终保留具体 evaluator 类型。

## `Animation` 与 build

所有可组合动画实现：

```rust,ignore
pub trait Animation: Sized {
    fn time_range(&self) -> Range<f64>;
    fn build(self, origin_sec: f64, output: &mut Vec<BuiltAnimation>);
}
```

`origin_sec` 表示当前动画局部时间坐标的父级原点：

- 普通叶子直接在该原点生成 `BuiltAnimation`；
- `At<A>` 将自己的 offset 加到原点后继续 build 内部动画；
- `AnimSequence` 中的叶子已经位于 Sequence 局部坐标，整体 build 时统一平移；
- `AnimStack` 中的子动画共享局部 0，整体 build 时同样统一平移。

`Animation::duration_secs()` 默认使用 `time_range().end`。Sequence 用它推进 cursor，Stack 用它计算最长子动画范围。

## `BuiltAnimation`

Sequence、Stack 和 Scene 需要保存异构动画，因此 build 会生成统一的 `BuiltAnimation`：

```text
BuiltAnimation
├─ BuiltEval
│  ├─ Dynamic(Box<dyn EvalDyn>)
│  └─ Static(Vec<DynItem>)
├─ time range
├─ rate function
├─ enabled
└─ evaluator name
```

`EvalDyn` 将结果追加到扁平的 `Vec<DynItem>`。动态 evaluator 通常追加一个结果；Sequence 的 `hold` 可以将同一时刻的多个结果保存为 Static。Static 求值时直接展开其中的 items，不会把聚合 Vec 再包装进新的 `DynItem`。

类型擦除只发生在 evaluator 内部。时间范围位于 Box 外，可以在动画已经擦除后继续平移、重新放置或供 preview 查询。

## Requirement Trait 模式

用户通常不直接构造 evaluator，而是通过 Item 的动画扩展 Trait：

```rust,ignore
let animation = square
    .fade_in()
    .with_duration(2.0)
    .with_rate_func(smooth);
```

返回值仍保留具体 evaluator 类型，例如：

```text
AnimationCell<Square, FadeIn<Square>>
```

只有当它进入 `AnimSequence::push`、`AnimStack::push` 或 `RanimScene::play` 时才会被 build 和擦除。
