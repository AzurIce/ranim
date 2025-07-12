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

## 2. 时间线

每一个 `ItemId` 都唯一对应一条时间线，时间线是一种用于编码物件动画的结构，
它的内部有一个存储了动画以及展示时间的列表，以及用于编码静态动画的物件状态。

编码动画的过程本质上是在向时间线中插入动态或静态的动画：

![Timeline](timeline.png)

### 2.1 创建时间线

通过 `r.insert(state)` 可以创建一条时间线：

```rust,ignore
let square = Square::new(2.0).with(|x| {
    x.set_color(manim::BLUE_C);
});
let circle = Circle::new(1.0).with(|x| {
    x.set_color(manim::RED_C);
});

let r_square1 = r.insert(square.clone()); // 类型为 `ItemId<Square>`
let r_square2 = r.insert(square); // 类型为 `ItemId<Square>`
let r_circle = r.insert(circle); // 类型为 `ItemId<Circle>`
```

### 2.1 访问时间线

时间线在被创建之后，需要通过 `r.timeline(&index)` 或 `r.timeline_mut(&index)` 来访问：

```rust,ignore
{
    // 类型为 `&ItemTimeline<Square>`
    let square_timeline_ref = r.timeline(&r_square1);
}
{
    // 类型为 `&ItemTimeline<Circle>`
    let circle_timeline_ref = r.timeline(&r_circle);
}
```

除了通过单一的 `&ItemId` 来访问单一的时间线，也可以通过 `&[&ItemId<T>; N]` 来访问多条时间线：

```rust,ignore
// 类型为 `[&mut ItemTimeline<Square>]`
let [sq1_timeline_ref, sq2_timeline_ref] = r.timeline_ref(&[&r_square1, &r_square2]);
```

同时也可以访问全部时间线的切片的不可变/可变引用，不过元素是类型擦除后的 `ItemDynTimelines`：

```rust,ignore
// 类型为 &[ItemDynTimelines]
let timelines = r.timelines();
```

### 2.2 操作时间线

`ItemTimeline<T>` 和 `ItemDynTimelines` 都具有一些用于编码动画的操作方法：

|方法|`ItemTimeline<T>`|`ItemDynTimelines`|描述|
|---|---|---|---|
|`show` / `hide`|✅|✅|显示/隐藏时间线中的物体|
|`forward` / `forward_to`|✅|✅|推进时间线|
|`play` / `play_with`|✅|❌|向时间线中插入动画|
|`update` / `update_with`|✅|❌|更新时间线中物体状态|
|`state`|✅|❌|获取时间线中物体状态|

有关方法的具体详细定义可以参考 API 文档。

下面的例子使用一个 `Square` 物件创建了一个时间线，然后编码了淡入1秒、显示0.5秒、消失0.5秒、显示0.5秒、淡出1秒的动画：

```rust,ignore
{{#rustdoc_include ../../examples/getting_started0/main.rs:construct}}
```

### 2.3 转换时间线类型

在对一个物件进行动画编码的过程中有时会涉及物件类型的转换，比如一个 `Square` 物件需要被转换为更低级的 `VItem` 才能够被应用 Write 和 UnWrite 动画，
此时就需要对时间线类型进行转换：

```rust,ignore
{{#rustdoc_include ../../examples/getting_started1/main.rs:construct}}
```
