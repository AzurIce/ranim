# v0.3 设计构思

本目录记录 ranim v0.3 场景与求值架构调整期间的原始构思和独立设计提案。这里的内容仍在讨论中，不代表最终 API。

## World

在之前的版本中，*World* 的存在被极度弱化，大概只有在渲染时求值出的 `Vec<CoreItem>` 才短暂地有它的影子，之后现在为了渲染相关做了 `CoreItemStore` 其实本质上也算是属于 *World* 了。但是这些基本都仅存在于渲染时，编码逻辑时依旧只有 Timeline。

为了扩展编码逻辑时的自由度与能力（毕竟不是全部动画都能很好地表示为纯函数），以及后续 Editor 需要，以及物件缓存等等，都需要一个作为中心的 *World* 存在。

*World* 本质是场景某一时刻的快照，是物件的 Collection。它应该是是全部逻辑的中枢，一切逻辑操作将变更落实到 *World* 上，所有渲染数据也都来源于 *World* 的状态。

## Animation

我们现在用这样一个结构来描述动画：

```rust
/// A cell of an animation
pub struct AnimationCell<T> {
    inner: Box<dyn Eval<T>>,
    /// The animation info
    pub info: AnimationInfo,
    // ANCHOR_END: AnimationCell
    anim_name: String,
}
```

其中 `inner` 是一个基于 `fn eval_alpha(&self, alpha: f64) -> T` 的纯函数，而 `info` 持有包含速率函数、时间范围等等的元信息。

现在的场景求值本质是基于 `eval_alpha` 的只有 Seek 操作的函数式求值，但是后面大概会不可避免地再涉及到 Step/Tick 类操作。
对于纯函数求值动画来说，其实后者等价于前者，但是对于模拟动画来说，Seek 往往是更加耗时、成本更高的操作。

未来离线渲染循环以及在线播放操作都应该是基于 Tick 的。

在思考这里是用 Trait + Box 直接将两类动画统一，还是用不同的 Trait 和 enum？

现在的 `AnimationCell<T>` 存在 `T` 范型，因此要被放入集合又要再次类型擦除，导致两级指针存在。或许这样更好：

```rust
pub trait Animator<T> {
    fn seek(&self, t);
    fn tick(&self, t, delta) {
        self.seek(t + delta);
    }
    /// Construct an [`AnimationCell<T>`] with default [`AnimationInfo`]
    fn into_animation_cell(self) -> AnimationCell<T, Self>
    {
        AnimationCell {
            inner: self,
            info: AnimationInfo::default(),
        }
    }
}

pub struct AnimationCell<T, A: Animator<T>> {
    inner: A,
    info: AnimationInfo
}
```

不过我们是否需要区分 Pure 和非 Pure 的？（比如后续需要再 timeline 里加以区分？）还是干脆不管？

以及我想了一下，要不干脆 play 取消掉链式调用，而改为用 `anim.chain(xxx)` 或者 `(xxx, xxx)` 的方式来做（类似 bevy 的 system 支持的语法）？或者后续要同时播放两个动画实现个类似 `stack![]` 的东西？这样的话 play 可以返回一点额外的信息（比如时间，用于辅助构造后续动画）。

我还想过单独做一个 TimeCursor，这样如果用户可以 `r.with_cursor(a).play(xxx)` 来方便地做不同“时间线”的排布？以及可以对 cursor 之间做同步等操作或者一些手动 seek 操作？

## 独立提案

- [使用 bevy_ecs 重构 World](./bevy-ecs-world.md)
- [Animation 结构与求值能力](./animation-model.md)
- [全局 Timeline 与 TimeCursor](./time-cursor.md)
- [动画组合与 Play 调度](./animation-composition.md)
