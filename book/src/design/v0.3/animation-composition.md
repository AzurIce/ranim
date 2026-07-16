# 动画组合与 Play 调度

> 状态：构思中。本页记录动画组合和 Timeline 调度的职责拆分，不代表最终实现。

## 想法

`play` 不再负责通过连续方法调用表达动画之间的先后关系。动画首先通过组合器构造成一个完整的时间结构，然后由 `play` 将它安排到 TimeCursor 指定的位置。

```text
基础动画
   │
   ├─ sequence / then  ─┐
   └─ parallel / with  ─┼─► 组合动画 ─► play(cursor, animation)
                        │                    │
                        └────────────────────┘
                                             ▼
                                         Playback
```

这里包含两个独立阶段：

- 动画组合决定内部片段如何顺序或并行执行。
- `play` 决定组合动画在全局 Timeline 上从何时开始。

## 动机

当前 `timeline.play(anim).play(anim)` 同时推进时间游标并安排动画。简单场景中很方便，但它占用了 `play` 的返回值，使 `play` 无法返回被调度片段的信息，也会让并行动画、嵌套组合和依赖关系需要额外的特殊 API。

拆分后，`play` 可以返回一个稳定的 `Playback`，用于后续同步、查询、标记和 Editor 选择。

## 顺序与并行组合

函数形式可以使用异构 tuple 表达一组动画：

```rust,ignore
let anim = sequence((
    square.fade_in(),
    parallel((
        square.shift(RIGHT),
        square.rotate(PI),
    )),
    square.fade_out(),
));

let playback = r.with_cursor(main).play(anim);
```

也可以提供等价的组合方法：

```rust,ignore
let anim = square
    .fade_in()
    .then(square.shift(RIGHT).with(square.rotate(PI)))
    .then(square.fade_out());
```

两种风格可以共存：tuple 形式适合结构清晰的大组合，方法形式适合短表达式。它们应产生相同的组合类型或相同的内部 clip tree。

建议使用 `sequence` 和 `parallel` 作为正式术语。`chain` 容易同时表示迭代器链、方法调用链或顺序组合，语义不够明确。

## 组合语义

每个可播放动画应至少暴露其 duration 和目标信息。组合器据此计算内部时间范围：

```text
sequence((A, B, C))
duration = duration(A) + duration(B) + duration(C)

parallel((A, B, C))
duration = max(duration(A), duration(B), duration(C))
```

对于 duration 不同的并行动画，较短动画结束后的语义需要明确。初步选择是保持其结束状态，直到整个 parallel 结束；如果需要裁剪、循环或恢复原状态，应由显式组合器表达。

组合动画仍应保持纯采样能力。给定组合动画的局部时间，组合器将其映射到子动画的局部 alpha，再由子动画计算结果。组合本身不应依赖此前是否 tick 过。

## Play 的职责

`play` 接收 cursor 和一个已经构造好的动画，完成以下工作：

1. 读取 cursor 当前时间作为组合动画的开始时间。
2. 将组合展开或整体注册到全局 Timeline。
3. 返回代表本次调度结果的 `Playback`。
4. 默认将 cursor 移动到 Playback 的结束时间。

概念接口：

```rust,ignore
pub fn play<A>(&mut self, cursor: CursorId, animation: A) -> Playback
where
    A: IntoAnimation;

pub struct Playback {
    pub id: PlaybackId,
    pub range: TimeRange,
    pub targets: Vec<EntityId>,
}
```

返回值可以用于后续布局：

```rust,ignore
let title_intro = r.with_cursor(title).play(title.fade_in());

r.with_cursor(body)
    .sync_to(title_intro.range.start + 0.2)
    .play(body.write());
```

`Playback` 是已调度动画的 handle，不持有 World 对象的可变引用。这样它可以被 Editor、time mark、依赖系统和调试工具长期保存。

## 默认 Cursor 与简洁 API

为了避免简单场景变得冗长，可以保留默认 cursor：

```rust,ignore
let intro = r.play(square.fade_in());
let movement = r.play(parallel((
    square.shift(RIGHT),
    square.rotate(PI),
)));
```

这仍然是顺序调度：第一次 `play` 后默认 cursor 被移动到 `intro.range.end`。区别在于，动画的内部并行关系已经通过 `parallel` 显式表达，而 `play` 的返回值不再是 `&mut Timeline`。

## 类型与类型擦除

组合器可以在进入 Timeline 前保持静态类型：

```rust,ignore
Sequence<(FadeIn, Parallel<(Shift, Rotate)>, FadeOut)>
```

这样基础动画和组合器之间无需动态分派。类型擦除发生在 `play` 将组合动画加入 Timeline 的边界：

```text
静态组合类型
    │ play
    ▼
Box<dyn ScheduledClip> / 类型擦除后的 Timeline 存储
```

这可以减少当前 `Box<dyn CoreItemAnimation>` 内再次包含 `Box<dyn Eval<T>>` 的双重间接层。代价是组合类型可能很大，因此错误信息、编译时间以及是否需要显式 `.boxed()` 逃生口仍需通过原型验证。

## 同一目标的并行写入

`parallel` 可能让多个子动画同时写入同一 entity/property。不能只依赖“最后一个写入者获胜”，否则 tuple 顺序会隐式影响结果。

可以考虑以下策略：

- 默认拒绝同一属性的重叠写入，并在构造或 seal Timeline 时报告冲突。
- 对可组合属性提供显式 additive/blend 模式。
- 允许用户通过 layer/priority 明确覆盖关系。

初步建议默认拒绝，未来按属性类型增加显式组合规则。

## 待决问题

- Timeline 内保存组合树，还是在 `play` 时展开为扁平 scheduled clips？组合树更接近作者结构，扁平结构更易求值和检测冲突。
- tuple 支持到多少项，是否依赖类似 Bevy 的 tuple trait 实现，或主要使用宏？
- `.then()` 和 `.with()` 是否会因为同一对象被多次可变借用而影响动画构造 API？动画创建阶段可能需要基于 entity handle，而不是持续借用对象。
- `play` 是否总是推进 cursor？可能需要 `schedule` 或 `play_at` 作为不移动 cursor 的低层 API。
- `Playback` 的 targets/property 信息是精确记录，还是只保存调试用途的摘要？
