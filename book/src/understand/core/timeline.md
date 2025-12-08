# 时间线

简单来说，时间线的本质是类型擦除后的 `AnimationCell<T>` 的容器，若干条时间线组合在一起即表示了整个场景的完整动画。

因为 `AnimationCell<T>` 带有范性参数，所以也就涉及类型擦除，满足于 `T: AnyExtractCoreItem` 的 `AnimationCell<T>` 会被擦除为 `Box<dyn CoreItemAnimation>`：

```rust,ignore
{{#include ../../../../packages/ranim-core/src/animation.rs:AnimationCell-CoreItemAnimation-eval_alpha}}
    // ...
}
```

```rust,ignore
{{#include ../../../../packages/ranim-core/src/timeline.rs:Timeline}}
```

在编写动画时的一系列操作（如 `forward`、`play` 等）最后都会转变为对 `Timeline` 内部属性的操作，最终达成的结果就是在其 `anims` 属性中完成此条时间线所有动 `AnimationCell` 的编码（即“把动画在时间上放到正确的位置”）。
