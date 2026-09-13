# 核心概念

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

Ranim 将动画定义为可按任意时间采样的值，并通过 closed core + open leaves 的方式组织场景：

```text
Eval<Output = T>                        visual leaf content
  -> IntoAnimNode                       default lowering (linear / 1s / enabled)
  -> Paramed<A> / At<A>                 playback params / placement
  -> AnimSequence / AnimStack / AnimLagged
  -> AnimNode { timing shell, NodeContent }
  -> SealedRanimScene -> SceneEvaluator
```

- 定义期（open）：叶子实现 `Eval`；容器/Sugar 位于 `animation::compose`，通过 `IntoAnimNode` lower 到运行时。
- 运行期（closed）：所有定义都 lower 成 `AnimNode`，其 `NodeContent` 是封闭的核心语言（`Sequence`、`Stack`、`Leaf`、`Static`、`Audio`）。视觉求值、音频烘焙、preview introspection 都是这棵树上的 interpreter。
- [`Eval`、`IntoAnimNode`、容器与运行时](./anim.md) 描述叶子动画如何根据局部进度产生状态、附加播放参数，以及顺序 / 并行 / 交错容器如何把场景组织成动画树。
- [`CoreItem` 与 `Extract`](./core_item.md) 描述动画求值结果如何经 `Extract` 展开为渲染器消费的 core item。
- [Core Items](./core_items/README.md) 逐个介绍三种 core item（`CameraFrame`、`VItem`、`MeshItem`）的字段与渲染语义。
- `RanimScene` 的根节点是一个 `AnimStack`。`r.play(animation)` 等价于向根 Stack 执行 `push`，因此多次根级 `play` 默认从 0 秒并行。

新模型不维护 Scene 内可变的 `TimelineId` 或运行时物件表。需要独立生命周期的内容由各自的 `AnimSequence` 持有，最后通过 Stack 组合。
