# 动画

本节将对 Ranim 中 *动画* 的实现思路进行讲解。

## `EvalDynamic<T>` Trait

一个标准化的动画其实本质上就是一个函数 $f$，它的输入是一个进度值 $a \in [0, 1]$，输出是该动画在对应进度处的结果 $x$：

$$
x = f(a)
$$

这个函数 $f$ 不仅定义了动画的 *求值*，同时其内部也包含了求值所需要的 *信息*。对应到计算机世界，其实也就是 *算法* 和 *数据*，而对应到编程语言上也就是 *方法* 和 *数据类型*。

在由 Rust 实现的 Ranim 中也就是 `EvalDynamic<T>` Trait 和实现了它的类型 `T`：

```rust,ignore
{{#include ../../../src/animation.rs:EvalDynamic}}
```

它接受自身的不可变引用和一个进度值作为输入，经过计算，输出一个自身类型的结果。

以 `Transform` 动画为例，其内部包含了物件初始状态和目标状态，以及用于插值的对齐后的初始和目标状态，在 `EvalDynamic<T>` 的实现中使用内部的数据进行计算求值得到结果：

```rust,ignore
{{#include ../../../src/animation/transform.rs:Transform}}

{{#include ../../../src/animation/transform.rs:Transform-EvalDynamic}}
```

## AnimationSpan

有了以进度 $a \in [0, 1]$ 为输入标准化的动画函数后，加上持续秒数 $\Delta t$、速率函数 $g$，就可以构造一个以秒 $t \in [0, \Delta t]$ 为输入的动画函数 $F(t)$：

$$
F(t) = f(g(\frac{t}{\Delta t}))
$$

在 Ranim 中，这对应着 `AnimationSpan` 结构：

```rust,ignore
{{#include ../../../src/animation.rs:AnimationSpan}}

{{#include ../../../src/animation.rs:AnimationSpan-eval}}
```

其中的 `evaluator: Evaluator<T>` 其实就是对 `Box<dyn EvalDynamic<T>>` 的封装。
