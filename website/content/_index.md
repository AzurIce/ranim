+++
title = "Getting Started"
insert_anchor_links = "right"
+++

<p>
<span style="font-size: 36px; font-weight: bold;">Ranim</span>
是一个使用 <span style="color: rgb(183, 65, 14); font-weight: bold;">Rust</span> 编写的程序化动画引擎，
受 <a href="https://github.com/3b1b/manim">3b1b/manim</a> 和 <a href="https://github.com/jkjkil4/JAnim">jkjkil4/JAnim</a> 启发
</p>

- 矢量图形基于二阶贝塞尔曲线表示，使用 SDF 渲染
- 使用 wgpu，兼容多种后端图形 API

## Getting Started

在 Ranim 中，定义并渲染一段动画的方式基本长成下面这个样子：

```rust
use ranim::prelude::*;

#[timeline]
fn timeline_name(ranim: Ranim) {
    let Ranim(timeline, camera) = ranim;

    // ...
}

fn main() {
    render_timeline!(getting_started_0);
}
```

参数为一个 `Ranim` 的函数可以被一个 `#[timeline]` attribute 标记为一段 Ranim 动画，使用 `render_timeline!` 宏可以对其进行渲染，渲染结果将被输出到 `output/<timeline_name>/` 目录下。

`Ranim` 是一个简单 Wrapper（为了避免手写生命周期参数），其中是：
- `timeline: &'t RanimTimeline`：Ranim API 的主要入口，几乎全部对动画的编码操作都发生在这个结构上
- `camera: &'r Rabject<'t, CameraFrame>`：默认的相机 Rabject，也是 RanimTimeline 中被插入的第一个 Rabject

`RanimTimeline` 和 `Rabject` 非常重要，将贯穿整个 Ranim 动画的编码。

### 1. RanimTimeline 和 Rabject

Ranim 使用一个 `RanimTimeline` 结构来编码动画，首先介绍两个最基本的操作：
- 使用 `timeline.forward(duration_secs)` 来使时间线推进一段时间
- 使用 `timeline.insert(item)` 来将一个 `item: T` 插入时间线，返回一个 `Rabject<T>`

`Rabject<T>` 的结构很简单，如下：

```rust
pub struct Rabject<'a, T> {
    pub timeline: &'a RanimTimeline,
    pub id: usize,
    pub data: T,
}
```

当某个物件 `T` 被插入 `RanimTimeline` 中时，会被赋予一个 Id，以 `Rabject<T>` 的形式返回，同时在 `RanimTimeline` 内部会以 `T` 的值为初始状态创建一条 `RabjectTimeline`。

使用 `timeline.show(&rabject)` 和 `timeline.hide(&rabject)` 可以控制接下来 `forward` 时的表现。

当一个 `Rabject` 被 `drop` 时，它会被 `hide` 掉：

```rust
impl<T> Drop for Rabject<'_, T> {
    fn drop(&mut self) {
        self.timeline.hide(self);
    }
}
```

下面的例子使用一个 `VItem` 物件和 `timeline.insert` 在时间线中创建了一个 `Rabject<VItem>` 并展示了 `show`、`hide` 以及 `drop` 对其影响：

!example-getting_started_0

### 2. 播放动画

Ranim 中的每一个动画都会为实现了对应 Trait 的物件添加对应的创建方法。

比如对于 `FadingAnim`，凡是实现了 `Opacity + Interpolatable` Trait 的物件都会拥有 `fade_in` 和 `fade_out` 方法。

对一个 `Rabject<T>` 调用创建动画的方法会返回一个 `AnimSchedule<T>`，将它传入 `timeline.play(anim_schedule)` 即可将这段动画编码在 `RanimTimeline` 中。

```rust
let mut square = timeline.insert(square);
timeline.play(square.fade_in());
timeline.play(square.fade_out());
```

上面的动画也可以这样写：
```rust
let mut square = timeline.insert(square);
timeline.play(square.fade_in().chain(|data| data.fade_out()));
```

`AnimSchedule<T>` 的 `chain` 方法，接受一个 `impl FnOnce(T) -> Animation<T>`，会将两个动画拼接在一起。

而 `T` 与 `&'r mut Rabject<'t, T>` 相同，也有创建动画的方法，不过返回的是 `Animation<T>`。

!example-getting_started_1

### 3. 动画参数

`AnimSchedule<T>` 和 `Animation<T>` 都具有一些控制动画属性的参数，可以通过链式调用的方式来设置：
- `with_duration(duration_secs)`：设置动画持续时间
- `with_rate_func(rate_func)`：设置动画速率函数

此外在这个例子中你会发现，在播放了 `transform_to(circle)` 之后，再播放 `fade_out` 时，播放的并不是圆形的淡出，而是方形。

这并不是一个 Bug，而是一种刻意的设计，请继续向下阅读 4. 向 Rabject 应用动画变更，了解更多。

!example-getting_started_2

### 4. 向 Rabject 应用动画变更（AnimSchedule 与 apply）

使用 Rabject 创建动画时是基于 Rabject 当前的内部数据来创建的，创建与播放动画并不会修改其内部数据。
如果想要一个动画的效果实际应用到 Rabject 中，那么需要对 `AnimSchedule` 使用 `apply` 方法。

这样的好处是对于一些对数据有 **损坏性变更** 的动画（比如 unwrite 等），我们不需要提前对数据进行备份。

!example-getting_started_3

不过 `chain` 是会以第一个动画的结束状态为基础创建下一个动画的，但是要注意此时的 `AnimSchedule` 是整个被拼接后的动画，如果不调用 `apply` 是不会更新 `Rabject` 内部的数据的，而调用 `apply` 会应用整个被拼接后的动画的变更：

```rust
// <-- Rabject's data is a square
timeline.play(
    square
        .transform_to(circle)
        .chain(|data| data.unwrite())
);
// <-- Rabject's data is still a square
timeline.play(square.write()); // This plays a square's unwrite, but not circle's
```

```rust
// <-- Rabject's data is a square
timeline.play(
    square
        .transform_to(circle)
        .chain(|data| data.unwrite())
        .apply(), // <-- Rabject's data is an unwrote circle now
);
timeline.play(square.write()); // This plays nothing, because after the apply, the data is empty（unwrote circle）
```

简单来说 `AnimSchedule` 的作用就是将具有紧密关系的动画组合在一起，通过 `apply` 会应用整个动画（类似 Transaction 的感觉）。