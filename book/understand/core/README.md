# 核心概念

Ranim 将动画定义为可按任意时间采样的值，并通过 closed core + open leaves 的方式组织场景：

```text
AnimDef<T, R>                          retained input + recipe
  -> Paramed<A> / At<A>                playback params / placement
  -> AnimSequence / AnimStack / AnimLagged
  -> AnimDefNode { Children / Leaf }   retained definitions
  -> build(&self)
  -> AnimNode { timing shell, NodeContent }  runtime execution
  -> SealedRanimScene -> SceneEvaluator
```

- 定义期（open）：`AnimDefNode` 保留完整输入与配方；`animation::compose` 中的容器通过标准 `Into<AnimDefNode>` 转换排布子定义。Inspector 和 timeline 直接读取定义树，不展开配方产生的运行子树。
- 运行期（closed）：`AnimDefNode::build(&self)` 构建独立的 `AnimNode`，不消费定义。其 `NodeContent` 是封闭的核心语言（`Children` 或 `Leaf`，叶子载荷为动画或音频），供视觉求值与音频烘焙遍历。
- [动画定义、`Eval`、容器与运行时](./anim.md) 描述初始快照与配方、完整定义的类型擦除、可重复构建，以及定义树和运行树各自的职责。还说明播放 override、固定排布和检查/持久化能力的边界。
- [完整求值帧与渲染提取](./extract.md) 描述完整输出如何保留在帧中，以及值级 `Extract` 与泛型插件如何在渲染侧递归消费它们。
- [渲染表示与物件插件](../rendering/README.md) 介绍 items 所有的 `RenderVItem`、`RenderMeshItem` 与相机插件；core 仅保留通用动画与变换。
- `RanimScene` 的根节点是一个 `AnimStack`。`r.play(animation)` 等价于向根 Stack 执行 `push`，因此多次根级 `play` 默认从 0 秒并行。

Scene 不维护可变的运行时物件表。需要独立生命周期的内容由各自的 `AnimSequence` 持有，最后通过 Stack 组合。
