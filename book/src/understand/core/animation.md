# 动画

本节将对 Ranim 中 *动画* 的实现思路进行讲解。

## `Eval<T>` Trait

一个标准化的动画其实本质上就是一个函数 $f$，它的输入是一个进度值 $a \in [0, 1]$，输出是该动画在对应进度处的结果 $x$：

$$
x = f(a)
$$

这个函数 $f$ 不仅定义了动画的 *求值*，同时其内部也包含了求值所需要的 *信息*。对应到计算机世界，其实也就是 *算法* 和 *数据*，而对应到编程语言上也就是 *方法* 和 *数据类型*。

在由 Rust 实现的 Ranim 中也就是 `Eval<T>` Trait 和实现了它的类型 `T`：

```rust,ignore
{{#include ../../../../packages/ranim-core/src/animation.rs:Eval-eval_alpha}}

    // ...
}
```

它接受自身的不可变引用和一个进度值作为输入，经过计算，输出一个自身类型的结果。

### 例 | `Static` 动画

`ranim_core::animation::Static` 动画是最基础的，也是唯一一个内置进 `ranim-core` 的动画：

```rust,ignore
{{#include ../../../../packages/ranim-core/src/animation.rs:Static}}
```

非常简单，其内置的 *信息* 就是物件本身，其 *求值* 就是简单的返回相同的物件。

### 例 | `Transform` 动画

以 `ranim_anims::transform::Transform` 动画为例，其内部包含了物件初始状态和目标状态，以及用于插值的对齐后的初始和目标状态，在 `EvalDynamic<T>` 的实现中使用内部的数据进行计算求值得到结果：

```rust,ignore
{{#include ../../../../packages/ranim-anims/src/transform.rs:Transform}}

{{#include ../../../../packages/ranim-anims/src/transform.rs:Transform-Eval}}
```

## AnimationCell

一个动画还会有很多额外的信息：

- 开始时间 $t_0$
- 持续时间 $\Delta t$
- 速率函数 $g$
- ...

在 Ranim 中，这些被信息被表示为一个 `AnimationInfo` 结构：

```rust,ignore
{{#include ../../../../packages/ranim-core/src/animation.rs:AnimationInfo}}
```

通过这些信息，我们可以将全局的秒映射到局部的 $a$，并将局部的 $a$ 映射到内部标准化的 $a$：


```rust,ignore
impl AnimationInfo {
{{#include ../../../../packages/ranim-core/src/animation.rs:AnimationInfo-map_sec_to_alpha}}
{{#include ../../../../packages/ranim-core/src/animation.rs:AnimationInfo-map_alpha}}
    // ...
}
```

如此，将以 $a \in [0, 1]$ 为输入标准化的动画函数与这些信息结合，也就构造除了一个以秒 $t \in [t_0, t_0 + \Delta t]$ 为输入的动画函数 $F(t)$：

$$
F(t) = \begin{cases}
f(g(\dfrac{t - t_0}{\Delta t})), &\Delta t \neq 0\\
1.0, &\Delta t = 0
\end{cases}
$$

在 Ranim 中，这对应着 `AnimationCell` 结构：

```rust,ignore
{{#include ../../../../packages/ranim-core/src/animation.rs:AnimationCell}}
// ...
}

{{#include ../../../../packages/ranim-core/src/animation.rs:AnimationCell-Eval}}
```

## Requirement Trait 模式

相信你注意到了，在实际的动画编写中，我们并没有手动构造任何一个动画结构，而是直接在物件身上调用一个方法来构造 `AnimationCell`：

```rust,ignore
let vitem_a = // ...;
let vitem_b = // ...;

// let anim = Transform::new(vitem_a, vitem_b).to_animation_cell();
// r.timeline_mut(t_id).play(anim);

r.timeline_mut(t_id).play(vitem_a.clone().transform_to(vitem_b));
```

```rust,ignore
let mut vitem_a = // ...;
let vitem_b = // ...;

// let anim = Transform::new(vitem_a, vitem_b).to_animation_cell();
// vitem_a = anim.eval_alpha(1.0);
// r.timeline_mut(t_id).play(anim);

r.timeline_mut(t_id).play(vitem_a.morph_to(vitem_b));
```

这是 Ranim 动画的一种编程模式，每一个动画都有一个对应的 Requirement Trait：

```rust,ignore
{{#include ../../../../packages/ranim-anims/src/morph.rs:MorphRequirement}}
```

同时还有一个对应的 Animation Trait，包含了一系列的 Helper 函数，以及为 `T: <Requirement Trait>` 的实现：

```rust,ignore
{{#include ../../../../packages/ranim-anims/src/morph.rs:MorphAnim}}

{{#include ../../../../packages/ranim-anims/src/morph.rs:MorphAnim-Impl}}
```

通过这种模式可以便捷地构造动画，并将动画的效果应用到物件状态上。
