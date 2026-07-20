# v0.3

## BREAKING CHANGES

- 弃用 `Timeline`，用 `AnimSequence` 和 `AnimStack` 替代
- 修改 `Eval<T>` Trait 的泛型参数为关联类型

## Composable Animation Arrangement

Ranim 动画编排的本质是构造动画数据表示并放入集合，在之前的设计中整个 `RanimScene` 通过内部的 `Vec<Timeline>` 来维护动画。

`Timeline` 的本质是 `Vec<Bod<dyn CoreItemAnimation>>` 动画序列容器，其中的每个元素都是前后相继的动画表示，同一时间一个 `Timeline` 只有一个动画激活，于是以前在动画组合代数上非常局限：
- 串行的动画必须通过 `Timeline` 的 API 手动推进/同步时间到对应位置
- 并行的动画必须通过创建新的 `Timeline` 来实现
- 整个场景的 `Vec<Timeline>` 本质是一次性并行组合多个串行编排的性质

在 Ranim v0.3 中，原本的 `Timeline` 被弃用，新增了两个可组合的基本动画容器 `AnimSequence` 和 `AnimStack`：




1. 动画集合所有权同意集中在 `RanimScene` 下，一切操作都必须经过 `RanimScene` 的 API。
2. 缺少可组合的动画表示。

以往基于 `Timeline` 的动画组织
