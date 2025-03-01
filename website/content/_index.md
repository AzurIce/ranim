+++
title = "介绍"
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

### 1. Timeline 和 Rabject

Ranim 使用一个 `Timeline` 结构来编码动画：
- 使用 `timeline.forward(duration_secs)` 来使时间线推进一段时间
- 使用 `timeline.insert(item)` 来将一个物件插入时间线，返回一个 `Rabject`

`Rabject` 是一个 Ranim 中受时间线管理的一个实体，所有的物件必须使用 `timeline.insert` 插入时间线中，时间线才会管理、渲染它。

下面的例子使用一个 `VItem` 物件和 `timeline.insert` 在时间线中创建了一个 `Rabject<VItem>`：

!example-getting_started_0

### 2. 播放动画

Ranim 中的每一个动画都会为实现了对应 Trait 的物件添加对应的创建方法。

比如对于 `FadingAnim`，凡是实现了 `Opacity + Interpolatable` Trait 的物件都会拥有 `fade_in` 和 `fade_out` 方法。

创建动画的方法会返回一个 `AnimSchedule`，将它传入 `timeline.play(anim_schedule)` 来播放：

!example-getting_started_1

### 3. 动画参数

`AnimSchedule` 具有一些控制动画属性的参数，可以通过链式调用的方式来设置：
- `with_duration(duration_secs)`：设置动画持续时间
- `with_rate_func(rate_func)`：设置动画速率函数

此外在这个例子中你会发现，在播放了 `transform_to(circle)` 之后，再播放 `fade_out` 时，播放的并不是圆形的淡出，而是方形。

这并不是一个 Bug，而是一种刻意的设计，请继续向下阅读 4. 向 Rabject 应用动画变更，了解更多。

!example-getting_started_2

### 4. 向 Rabject 应用动画变更

使用 Rabject 创建动画时是基于 Rabject 当前的内部数据来创建的，创建与播放动画并不会修改其内部数据。
如果想要一个动画的效果实际应用到 Rabject 中，那么需要对 `AnimSchedule` 使用 `apply` 方法。

这样的好处是对于一些对数据有 **损坏性变更** 的动画（比如 unwrite 等），我们不需要提前对数据进行备份。

!example-getting_started_3
