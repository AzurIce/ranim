# Getting Started

> [!caution]
> 本书绝大多数内容目前均由 ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

Ranim 的场景由一个 `fn(&mut RanimScene)` 函数构造。场景函数只负责定义动画。项目是自己的二进制 crate：`main` 创建 App、注册场景，再通过 `App::run()` 提供命令行入口；Plugin 是组织场景与自定义渲染配置的可选机制，Web 预览宿主使用同一份装配。

## 准备项目

`Cargo.toml` 只需要 `ranim` 依赖：

```toml
[dependencies]
ranim = { version = "...", features = ["preview"] }
```

`preview` feature 蕴含 `render`；不需要交互预览时可以只开 `render`。
`src/main.rs` 注册场景并交给命令行入口（`AppRunExt` 已由 prelude 导出）：

```rust,ignore
use ranim::prelude::*;

fn main() {
    App::new().add_scenes([hello::scene()]).run();
}
```

`App::run()` 提供 `preview` / `output` / `render` / `inspect` 子命令，详见
[命令行入口](./cli.md)。`inspect` 是纯 CPU 查询，不执行渲染装配
（`RenderSetup`）的 GPU 回调。

动画代码通常从 prelude、item 类型和对应的动画扩展 trait 中导入 API：

```rust,ignore
use ranim::{
    anims::fading::FadingAnim,
    color::palettes::manim,
    items::vitem::geometry::Square,
    prelude::*,
};
```

### Prelude 覆盖什么

`ranim::prelude::*` 覆盖日常作者 API：`Eval` / `IntoAnimNode` / `Unplaced`
/ `PlaybackExt` 等动画协议，`AnimSequence` / `AnimStack` / `AnimLagged` 与
`seq!` / `stack!` / `lagged!`，`Pure` / `Iterative` / `Static`，`Sound` /
`AudioClip`，以及 `RanimScene` / `TimeMark` / `App` / `Plugin`。

以下类型有意**不**放进 prelude，需要时显式导入：

```rust,ignore
use ranim_core::audio::AudioError;
use ranim_core::animation::build::{At, Paramed};
use ranim_core::animation::node::{AnimNode, AnimationInfo};
use ranim_core::frame::EvaluatedFrame;
```

- `EvaluatedFrame`：读取求值结果或直接驱动渲染时使用；
- `AnimNode` / `AnimationInfo`：运行时 introspection API；
- `At` / `Paramed`：需要给返回值命名的 timing wrapper；
- `AudioError`：音频解码/文件错误处理。

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

`#[scene]` 保留场景函数，并生成同名模块中的 `hello::scene() -> Scene`。
每次调用都创建一个 owned 描述符，其中包含名称、构造函数、配置和输出列表。
把描述符注册进 App，再交给 `App::run()`：

```rust,ignore
fn main() {
    App::new().add_scenes([hello::scene()]).run();
    // 多个场景：add_scenes([first::scene(), second::scene()])
}
```

只有加入 App 的场景才会出现在命令行和 Web 入口中。
场景目录保留显式注册顺序，场景名称必须唯一。

渲染上面的场景不需要任何装配：场景输出裸 `Square` 和 `CameraFrame`，项目未注册
`RenderSetup` 时渲染与预览回退到默认配置，内置相机与矢量/网格渲染开箱即用
（见下文「自定义渲染配置」）。

### 用 Plugin 组织项目（可选）

插件持有自己的配置，在 `build(&self, app: &mut App)` 中安装项目能力。场景变多、
需要跨二进制复用，或要携带自定义渲染配置时，把它们收进插件：

```rust,ignore
pub struct DemoScenesPlugin;

impl Plugin for DemoScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_scenes([hello::scene()]);
    }
}

fn main() {
    App::new().add_plugins(DemoScenesPlugin).run();
}
```

宿主既可以直接安装插件类型，也可以安装另一个 crate 的普通 Rust 工厂函数：

```rust,ignore
let mut app = App::new();
app.add_plugins(DemoScenesPlugin);

// 在另一个应用中使用这个 crate：
let mut app = App::new();
app.add_plugins(my_project::plugin());
```

插件在添加时执行构建，也可以通过 `app.add_plugins(...)` 继续安装依赖插件。
多个插件可写成元组，例如 `app.add_plugins((GeometryPlugin, TextPlugin))`。
带配置的插件直接构造，例如 `DemoPlugin { quality: 4 }`，不要求实现 `Default`。
项目资源保存在各自的 App 中，多个项目互不共享全局注册表。

### 自定义渲染配置（可选）

渲染配置依赖 `ranim` 的 `render` feature（`preview` 会同时启用它）。项目未注册
`RenderSetup` 时，渲染与预览使用的默认配置等价于：

```rust,ignore
RenderSetup::new(|render_app| {
    render_app.add_plugins(RanimItemsPlugin);
})
```

`ranim_items::RanimItemsPlugin` 捆绑三个内置物件插件（`CameraFramePlugin`、
`VItemPlugin`、`MeshItemPlugin`），覆盖裸内置类型与 `Vec<VItem>`。它与三个单件
插件互斥：Bevy 插件在 App 内唯一，重复安装会 panic。

若完整输出是默认配置未覆盖的类型——如 `Vec<Circle>`、`Transformed<T, G>` 或
自定义类型——在插件中注册自己的 `RenderSetup`，为实际完整类型加入
`ExtractPlugin<T>`（只注册 `T` 不会读取这些组合）：

```rust,ignore
use ranim::{
    items::RanimItemsPlugin,
    render::{RenderSetup, extract::ExtractPlugin},
};

pub struct ProjectPlugin;

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_scenes([hello::scene()]);
        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((
                RanimItemsPlugin,
                ExtractPlugin::<MyOutput>::default(),
            ));
        }));
    }
}
```

`RenderSetup` 保存在项目 App 中，preview / output 从所属 App 取得并在每次创建
renderer 时重放，分辨率变化和后台导出也使用同一项目的配置。保存 setup 不会
初始化 GPU，纯 CPU 求值与 `inspect` 也不执行该回调。详见
[完整求值帧与渲染提取](./understand/core/extract.md)。

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
{{#rustdoc_include ../../examples/getting_started0/scenes.rs:construct}}
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
{{#rustdoc_include ../../examples/getting_started1/scenes.rs:construct}}
```

多个独立 Sequence 的组合示例：

```rust,ignore
{{#rustdoc_include ../../examples/getting_started2/scenes.rs}}
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

`App::run()` 已把常用流程做成子命令；需要在 Rust 代码里直接驱动时，创建 App、注册场景，再选择 `SceneHandle`：

```rust,ignore
let mut app = App::new();
app.add_scenes([hello::scene()]); // 或安装项目插件：app.add_plugins(my_project::plugin())
let project = std::sync::Arc::new(app);
let scene = project.scene("hello").expect("hello is registered");

// 需要 render feature，渲染场景声明的所有输出；2 是 readback 缓冲数量。
ranim::cmd::render_scene(&scene, 2);
// 需要 preview feature，预览窗口持有这个场景与所属 App。
ranim::cmd::preview_scene(scene);
```

`SceneHandle` 保留 App 的所有权，预览重建渲染器和后台导出时都能继续访问同一项目。

### WASM 预览宿主

`App::run()` 是原生入口；WASM 预览由自己的宿主二进制启动。把场景（以及
可选的项目插件）放在 `src/lib.rs`（小项目也可以全部写进 `main.rs`），
`src/main.rs` 创建 App、注册场景并启动所选场景的预览。

在 `Cargo.toml` 中为 `ranim` 开启预览功能，并给宿主二进制单独命名：

```toml
[[bin]]
name = "my-project-preview"
path = "src/main.rs"

[dependencies]
ranim = { git = "https://github.com/AzurIce/ranim", features = ["preview"] }
```

库和二进制使用不同名称，避免 WASM 构建时生成同名产物。

假设 package 名为 `my_project`，`src/main.rs` 可以同时用于原生和 WASM：

```rust,ignore
use std::sync::Arc;

fn main() {
    let mut app = ranim::App::new();
    app.add_scenes([my_project::hello::scene()]);
    let app = Arc::new(app);
    let scene = app.scene("hello").expect("hello is registered");
    ranim::cmd::preview_scene(scene);
}
```

构建二进制后，用 `wasm-bindgen` 生成 JavaScript 模块：

```bash
cargo build --bin my-project-preview --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg --out-name my_project target/wasm32-unknown-unknown/debug/my-project-preview.wasm
```

在页面中提供场景名称对应的画布，并初始化模块：

```html
<canvas id="ranim-app-hello" width="1280" height="720"></canvas>
<script type="module">
  import init from "./pkg/my_project.js";
  await init();
</script>
```

初始化模块会执行二进制的 `main`，由宿主启动预览，无需额外的
`#[wasm_bindgen(start)]` 或 JavaScript `preview_scene` 函数。
如果页面需要按名称启动多个场景，可以像 `ranim-examples-preview` 一样，
由宿主自行导出 `preview_scene(name)`；这时宿主需要直接依赖 `wasm-bindgen`。

### 命令行：预览与渲染

项目二进制的 `App::run()` 提供全部命令行入口：

```bash
cargo run -- preview
cargo run -- output
cargo run -- output hello
cargo run -- render hello
```

不渲染、只查询场景信息时使用 `inspect`：

```bash
cargo run -- inspect scenes
cargo run -- inspect tree
cargo run -- inspect frame hello --at 1.0
```

`tree` 的 Scene 名称在只有一个 Scene 时可以省略；`frame` 必须指定 Scene 名称，`--at` 为采样时间（秒）。需要完整几何数据时给 `frame` 加 `--verbose`，机器可读输出加 `--format json`。

`preview` 可以接收一个可选 Scene 名称；`output` 可以接收零个或多个 Scene 名称，并渲染它们声明的所有 `#[output(...)]`；`render` 接收恰好一个 Scene 名称，使用默认输出设置（`1920x1080`、60 fps、mp4）做一次临时渲染，不读取 `#[output(...)]`。完整说明见[命令行入口](./cli.md)。

在本仓库中运行示例：

```bash
cargo run --example getting_started0 --features preview -- preview
cargo run --example getting_started0 --features preview -- output
```

带 `required-features`（如 `typst`、`gltf`）的示例需要同时开启对应 feature。
