# 核心概念

Ranim 将动画定义为可按任意时间采样的值，并通过顺序和并行容器组织场景：

```text
Eval<Output = T>
  -> default Animation
  -> Paramed<A>
  -> AnimSequence / AnimStack / AnimLagged
  -> AnimationCell
  -> SealedRanimScene
```

- [`Eval`、`Animation` 与 `Paramed`](./animation.md) 描述叶子动画如何根据局部进度产生状态并附加播放参数。
- [`AnimSequence`、`AnimStack` 与 `AnimLagged`](./timeline.md) 分别描述顺序状态、并行动画层和 stagger 排布。
- `RanimScene` 的根节点是一个 `AnimStack`。`r.play(animation)` 等价于向根 Stack 执行 `push`，因此多次根级 `play` 默认从 0 秒并行。

新模型不维护 Scene 内可变的 `TimelineId` 或运行时物件表。需要独立生命周期的内容由各自的 `AnimSequence` 持有，最后通过 Stack 组合。
