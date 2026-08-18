# Getting Started

Ranim 的场景由一个 `fn(&mut RanimScene)` 函数构造。场景函数只负责定义动画；预览、渲染和输出配置由 `#[scene]`、`#[output]` 与 ranim CLI 处理。

## 准备项目

使用 CLI 热加载 lib target 时，crate 需要生成动态库：

```toml
[lib]
crate-type = ["rlib", "cdylib"]
```

动画代码通常从 prelude、item 类型和对应的动画扩展 trait 中导入 API：

```rust,ignore
use ranim::{
    anims::fading::FadingAnim,
    color::palettes::manim,
    items::vitem::geometry::Square,
    prelude::*,
};
```

## 第一个场景

下面的场景让一个蓝色正方形淡入、保持一秒，再淡出。相机作为独立 Sequence 与内容并行播放：

```rust,ignore
use ranim::{
    anims::fading::FadingAnim,
    color::palettes::manim,
    items::vitem::geometry::Square,
    prelude::*,
};

#[scene(clear_color = "#000000")]
#[output(width = 1280, height = 720, fps = 30, format = "mp4")]
fn hello(r: &mut RanimScene) {
    let square = Square::new(2.0).with(|square| {
        square.set_color(manim::BLUE_C);
    });

    let mut content = AnimSequence::new();
    content
        .push(square.clone().fade_in())
        .hold(1.0)
        .push(square.fade_out());

    let mut camera = AnimSequence::new();
    camera
        .push(CameraFrame::default().show())
        .hold_to(content.cursor_sec());

    r.play(camera);
    r.play(content);
}
```

`#[scene]` 会保留场景函数，并生成、注册对应的静态 Scene 描述。通常不需要手工创建 `Scene` 或编写 `main`。

## Scene 的根是并行 Stack

`RanimScene::play` 等价于向根 `AnimStack` 执行 `push`：

```rust,ignore
r.play(camera);
r.play(content);
```

这两个动画共享局部 0 秒并行播放，互不覆盖。`play` 不维护全局 cursor，也不会根据 item 值查找并修改之前加入的动画。

需要顺序播放时，先使用 `AnimSequence` 组织一条完整状态序列，再将 Sequence 加入 Scene。

## 使用 `AnimSequence`

`AnimSequence` 维护自己的 cursor：

| 方法 | 行为 |
| --- | --- |
| `push(animation)` | 在当前 cursor 加入动画，并按其 duration 推进 cursor |
| `forward(secs)` | 只推进 cursor，空白区间没有输出 |
| `forward_to(sec)` | 将 cursor 推进到指定绝对时间 |
| `hold(secs)` | 保持 cursor 处的 Sequence 状态并推进 cursor |
| `hold_to(sec)` | 将当前状态保持到指定绝对时间 |
| `cursor_sec()` | 返回当前 Sequence 时长/cursor |

完整的 show/hide 示例可以直接查看：

```rust,ignore
{{#rustdoc_include ../../examples/getting_started0/lib.rs:construct}}
```

### `hold` 与 `forward`

`forward` 表示明确的空白时间；`hold` 表示把当前状态延长一段时间。新模型不会隐式认为一个动画结束后物件仍然存在。

```rust,ignore
sequence
    .push(square.fade_in())
    .hold(1.0)    // 保持淡入后的状态
    .forward(0.5) // 接下来 0.5 秒没有该 Sequence 的输出
    .push(circle.fade_in());
```

### `show` 与 `hide`

`show()` 和 `hide()` 是用于 Sequence 状态切换的零时长事件。cursor 上出现状态事件时，后续 `hold` 使用这些事件组成新的完整状态快照，不再继承左侧状态。

```rust,ignore
sequence
    .push(square.show())
    .hold(1.0)
    .push(square.hide())
    .hold(1.0);
```

`hide` 不会跨 Sequence 查找同一个 item。需要独立显示/隐藏的内容应放在独立 Sequence 中，再通过根 Stack 并行组合。

## 并行组合

固定数量的并行动画可以使用 `stack!`：

```rust,ignore
let scene = stack![
    background.show().with_duration(total_secs),
    content,
    camera.show().with_duration(total_secs),
];
r.play(scene);
```

运行时动态生成的动画使用 `AnimStack`：

```rust,ignore
let mut layers = AnimStack::new();
for animation in animations {
    layers.push(animation);
}
r.play(layers);
```

`AnimStack` 的 duration 是最长子动画的 duration。较短子动画结束后不会自动保持到 Stack 结束。

## 类型转换与动画扩展 trait

动画方法由 requirement/extension trait 提供，使用前需要导入对应 trait。例如 `fade_in` 来自 `FadingAnim`，`morph_to` 来自 `MorphAnim`，`write`/`unwrite` 来自 `WritingAnim`。

有些动画只对更底层的 `VItem` 实现。几何 item 可以通过 `VItem::from` 或 `.into()` 转换：

```rust,ignore
{{#rustdoc_include ../../examples/getting_started1/lib.rs:construct}}
```

多个独立 Sequence 的组合示例：

```rust,ignore
{{#rustdoc_include ../../examples/getting_started2/lib.rs}}
```

## Scene 与 Output 属性

`#[scene]` 支持：

- `name = "..."`：设置注册的场景名称，默认使用函数名。
- `clear_color = "..."`：设置 CSS 格式的清屏颜色，默认 `#333333ff`。

每个 `#[output]` 定义一个输出；一个 Scene 可以声明多个 output：

- `width`、`height`：输出像素尺寸，默认 1920x1080。
- `fps`：帧率，默认 60。
- `format`：`mp4`、`webm`、`mov` 或 `gif`。
- `dir`：输出目录，默认 `./output`。
- `name`：输出文件名前缀；未设置时使用 Scene 名称。
- `save_frames`：是否保存逐帧图片，默认 `false`。

没有写 `#[output]` 时会使用默认输出配置。

## 预览与渲染

安装 CLI：

```bash
cargo install ranim-cli
```

预览或渲染当前 package 的 lib target：

```bash
ranim preview
ranim output
ranim output hello
ranim render hello
```

指定 workspace package 或 example target：

```bash
ranim preview -p package_name --example example_name
ranim output -p package_name --example example_name
ranim render -p package_name --example example_name hello
```

不渲染、只查询场景信息时使用 `inspect`：

```bash
ranim inspect scenes --example example_name
ranim inspect tree --example example_name
ranim inspect frame <scene_name> --at 1.0 --example example_name
```

`tree` 的 Scene 名称在只有一个 Scene 时可以省略；`frame` 必须指定 Scene 名称，`--at` 为采样时间（秒）。需要完整几何数据时给 `frame` 加 `--verbose`，机器可读输出加 `--format json`。

`preview` 可以接收一个可选 Scene 名称；`output` 可以接收零个或多个 Scene 名称，并渲染它们声明的所有 `#[output(...)]`；`render` 接收恰好一个 Scene 名称，使用默认输出设置（`1920x1080`、60 fps、mp4）做一次临时渲染，不读取 `#[output(...)]`。额外的 Cargo 构建参数放在 `--` 后，例如：

```bash
ranim output hello -- --release
ranim render hello -- --release
```

在本仓库中可以直接运行 CLI package：

```bash
cargo run -p ranim-cli --release -- preview --example getting_started0
cargo run -p ranim-cli --release -- output --example getting_started0
```
