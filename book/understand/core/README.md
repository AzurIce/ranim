# 核心概念

Ranim 的动画定义保留初始状态与配方，运行实例负责按时间采样；两者通过
`build(&self)` 连接：

```text
Item + recipe                         anim / anim_ref / anim_mut
  -> AnimDef<T, R>                     retained input + recipe
  -> Paramed<A> / At<A>                playback params / placement
  -> AnimSequence / AnimStack / AnimLagged
  -> AnimDefNode { Children / Leaf }   retained definitions
  -> build(&self)
  -> AnimNode { timing shell, NodeContent }  runtime execution
  -> SealedRanimScene -> SceneEvaluator
```

- 定义期（open）：`AnimDefNode` 保留完整输入与配方；`animation::compose` 中的容器通过标准 `Into<AnimDefNode>` 转换排布子定义。Inspector 和 timeline 直接读取定义树，不展开配方产生的运行子树。
- 运行期（closed）：`AnimDefNode::build(&self)` 构建独立的 `AnimNode`，不消费定义。其 `NodeContent` 是封闭的核心语言（`Children` 或 `Leaf`，叶子载荷为动画或音频），供视觉求值与音频烘焙遍历。
- [动画系统](./anim.md) 从「初始状态 + 配方 → 运行结构」出发，单独解释 `anim` / `anim_ref` / `anim_mut` 的输入所有权与即时写回，再介绍定义树、播放配置、组合容器、运行时和 `Eval`，最后说明检查与持久化的边界。
- [完整求值帧与渲染提取](./extract.md) 描述完整输出如何保留在帧中，以及值级 `Extract` 与泛型插件如何在渲染侧递归消费它们。
- [渲染表示与物件插件](../rendering/README.md) 介绍 items 所有的 `RenderVItem`、`RenderMeshItem` 与相机插件；core 仅保留通用动画与变换。
- `RanimScene` 的根节点是一个 `AnimStack`。`r.play(animation)` 等价于向根 Stack 执行 `push`，因此多次根级 `play` 默认从 0 秒并行。

Scene 不维护可变的运行时物件表。需要独立生命周期的内容由各自的 `AnimSequence` 持有，最后通过 Stack 组合。
