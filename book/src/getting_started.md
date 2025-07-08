# Getting Started

<div class="warning">

注意：

当前本章内容非常不完善，结构不清晰、内容不完整，目前建议结合 Example 和源码来了解。

</div>

在 Ranim 中，定义并渲染一段动画的代码基本长成下面这个样子：

```rust
use ranim::prelude::*;

{{#include ../../examples/hello_ranim/main.rs:15:19}}
        // ...
    }
}

fn main() {
    render_scene(HelloRanimScene, &AppOptions::default());
}
```

`render_scene` 函数接收一个 `impl Scene` 并对其进行构造、求值、渲染，并将渲染结果编码为视频被输出到 `<output_dir>/<scene_name>/` 目录下。

其中 `<output_dir>` 可以通过 `AppOptions` 设置，而 `<scene_name>` 则由场景的 `SceneMetaTrait` 的实现决定（见 [`Scene` Trait](#scene-trait)）。

## 1. 场景的构造

`Scene` 由两个 Trait 组合而成：

- `SceneMetaTrait`：包含场景元信息的定义。
- `SceneConstructor`：包含场景动画构造过程的定义。

```rust,ignore
{{#include ../../src/lib.rs:SceneMeta}}

{{#include ../../src/lib.rs:SceneMetaTrait}}

{{#include ../../src/lib.rs:SceneConstructor}}
```

当这两个 Trait 均被实现时，`Scene` Trait 会被自动实现。

Ranim 提供了一个 `#[scene]` 宏来便于 `SceneMetaTrait` 的实现：
- 使用 `#[scene]` 会以结构体的 snake_case 命名（去掉 `Scene` 后缀）作为 `SceneMeta` 的 `name` 字段自动实现这个 Trait
- 也可以通过 `#[scene(name = "<NAME>")]` 来手动设置场景名称。

而 `SceneConstructor` 的 `construct` 方法则是编码了整个场景动画过程的核心部分，它有两个参数：

- `&mut RanimScene`：Ranim 场景，动画编码 API 的入口。
- `ItemId<CameraFrame>`：相机物件的 Id。

`RanimScene` 和 `ItemId` 这两个类型十分关键，是整个动画编码过程的核心。

## 2. 向时间线中插入物件

`RanimScene` 中有若干个「物件时间线」，每个物件时间线中包含一个动画列表。

通过 `r.insert(state)` 可以创建一个 `Timeline`：

```rust,ignore
let square: Square = Square::new(2.0).with(|square| {
    square.set_color(manim::BLUE_C);
});
let r_square: TimelineId<Square> = r.insert(square);
```

`RanimScene` 中有关 `Timeline` 的方法如下：

|方法|描述|
|---|---|
|`r.init_timeline(state)`|创建一个 `Timeline`|
|`r.timeline(id)`|获取对应 `id` 的 `Timeline` 的不可变引用|
|`r.timeline_mut(id)`|获取对应 `id` 的 `Timeline` 的可变引用|
|`r.timelines()`|获取类型擦除后的全部时间线的不可变引用|
|`r.timelines_mut()`|获取类型擦除后的全部时间线的可变引用|

时间线是用于编码动画的结构，首先介绍几个最基本的操作：
- 使用 `timeline.forward(duration_secs)` 来使时间线推进一段时间
- 使用 `timeline.play(anim)` 来向时间线中插入一段动画
- 使用 `timeline.show()` 和 `timeline.hide()` 可以控制物体接下来 `forward` 时显示与否。

下面的例子使用一个 `Square` 初始化了一个时间线，然后编码了淡入1秒、显示0.5秒、消失0.5秒、显示0.5秒、淡出1秒的动画：

```rust,ignore
// A Square with size 2.0 and color blue
let square = Square::new(2.0).with(|square| {
    square.set_color(manim::BLUE_C);
});

let timeline = r.init_timeline(square.clone());
timeline.play(square.clone().fade_in());
timeline.forward(1.0);
timeline.hide();
timeline.forward(1.0);
timeline.show();
timeline.forward(1.0);
timeline.play(square.fade_out());
```

时间线内部维护了一个物件的状态值，在 `forward` 时会使用它来编码静态的动画，通过 `timeline.state()` 可以获取时间线内部的物件状态值。于是上面的代码也可以写作：

```rust,ignore
let timeline: &mut ItemTimeline<Square> = r.init_timeline(square);
timeline.play(timeline.state().clone().fade_in());
// ...
timeline.play(timeline.state().clone().fade_out());
```

同时为了便捷，还有一个 `timeline.play_with(builder)` 方法来编码动画：

```rust,ignore
impl<T: Clone + 'static> ItemTimeline<T> {
    // ...
    pub fn play_with(&mut self, anim_func: impl FnOnce(T) -> AnimationSpan<T>) -> T {
        self.play(anim_func(self.state.clone()))
    }
}
```

于是之前的代码也可以写作：

```rust,ignore
let timeline: &mut ItemTimeline<Square> = r.init_timeline(square);
timeline.play_with(|square| square.fade_in());
// ...
timeline.play_with(|square| square.fade_out());
```
