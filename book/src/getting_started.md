# Getting Started

> [!caution]
> 本书绝大多数内容目前均由 ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

Ranim 的场景由一个 `fn(&mut RanimScene)` 函数构造。场景函数只负责定义动画。项目是自己的二进制 crate：`ranim::App` 就是 `bevy_app::App` 的直接再导出，场景注册与选择（`add_scene` / `add_scenes` / `scenes` / `scene(name)` / `first_scene()`）由 `ranim::AppExt` trait（prelude 已导出）在 App 上提供；`main` 创建 App、注册场景，再用 `app.build()`（`ranim::RanimAppExt`，prelude 已导出）把构建中的 App 定稿为 `ranim::RanimApp`，交给自由函数 `run_cli`（prelude 已导出）解析进程参数、分发命令行子命令；`ranim::Plugin` 是 bevy `Plugin` 的再导出，是组织场景与自定义渲染配置的可选机制，Web 预览宿主使用同一份装配。

## 准备项目

`Cargo.toml` 只需要 `ranim` 依赖：

```toml
[dependencies]
ranim = { version = "...", features = ["preview"] }
```

`preview` feature 蕴含 `render`；不需要交互预览时可以只开 `render`。
`src/main.rs` 注册场景，`build()` 定稿后交给 `run_cli`：

```rust,ignore
use ranim::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_scenes([hello::scene()]);
    run_cli(app.build());
}
```

`app.build()`（`RanimAppExt`，prelude 已导出）是「构建中的 App → 可开
会话的项目」的显式转换：断言插件全部就绪（Ready），执行一次 bevy 的
`finish()` / `cleanup()`，返回 `ranim::RanimApp`。`RanimApp` 是项目的
会话期形态——`Deref` / `DerefMut` 到 bevy `App`，update、渲染 SubApp
安装、资源访问与场景选择照常工作，但不再是插件构建目标；`run_cli`、
`preview_scene` 等会话入口都以它为输入。

`run_cli` 安装 tracing、解析进程参数，随即提供
`preview` / `output` / `render` / `inspect` 子命令，详见
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
把描述符注册进 App（`add_scene` / `add_scenes` 由 `AppExt` 提供），`build()` 定稿后交给 `run_cli`：

```rust,ignore
fn main() {
    let mut app = App::new();
    app.add_scenes([hello::scene()]);
    run_cli(app.build());
    // 多个场景：app.add_scenes([first::scene(), second::scene()])
}
```

只有加入 App 的场景才会出现在命令行和 Web 入口中。
场景目录保留显式注册顺序，场景名称必须唯一。

渲染上面的场景不需要任何装配：场景输出裸 `Square` 和 `CameraFrame`，项目未注册
`RenderSetup` 时渲染与预览回退到默认配置，内置相机与矢量/网格渲染开箱即用
（见下文「自定义渲染配置」）。

### 用 Plugin 组织项目（可选）

`ranim::Plugin` 就是 bevy 的 `Plugin`。插件持有自己的配置，在 `build` 中安装项目能力——`build` 的签名是 `fn build(&self, app: &mut bevy_app::App)`；`ranim::App` 与 `bevy_app::App` 是同一类型，写成 `&mut App` 完全等价。场景注册等方法由 `AppExt` trait（prelude 已导出）在 App 上提供。场景变多、
需要跨二进制复用，或要携带自定义渲染配置时，把它们收进插件：

```rust,ignore
pub struct DemoScenesPlugin;

impl Plugin for DemoScenesPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_scenes([hello::scene()]);
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DemoScenesPlugin);
    run_cli(app.build());
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
项目资源就是 bevy world 资源：`app.insert_resource(...)` 存入（类型需
`#[derive(Resource)]`，`ranim::Resource` 已再导出），经 `app.world()` 读取；
多个项目互不共享全局注册表。

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
    fn build(&self, app: &mut bevy_app::App) {
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

`RenderSetup` 是 bevy `Resource`，保存在项目 App 中，preview / output 从所属
App 取得并在每次安装渲染 SubApp 时重放，分辨率变化和后台导出也使用同一项目的
配置。保存 setup 不会初始化 GPU，纯 CPU 求值与 `inspect` 也不执行该回调。详见
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

`run_cli` 已把常用流程做成子命令；需要在 Rust 代码里直接驱动时，创建 App、
注册场景，再选择场景（`app.scene(name)` 返回克隆的描述符）：

```rust,ignore
let mut app = App::new();
app.add_scenes([hello::scene()]); // 或安装项目插件：app.add_plugins(my_project::plugin())
let scene = app.scene("hello").expect("hello is registered");

// 项目的渲染装配：读取 RenderSetup 资源，未注册时回退到默认配置
let setup = app
    .world()
    .get_resource::<ranim::render::RenderSetup>()
    .cloned()
    .unwrap_or_else(|| {
        ranim::render::RenderSetup::new(|render_app| {
            render_app.add_plugins(ranim::items::RanimItemsPlugin);
        })
    });

// 需要 render feature，渲染场景声明的所有输出；2 是 readback 缓冲数量。
ranim::cmd::render_scene(&scene, &setup, 2);
// 需要 preview feature，预览 app 接管定稿后的项目（RanimApp），项目资源全程可用。
ranim::cmd::preview_scene(app.build(), scene);
```

### WASM 预览宿主

命令行入口 `run_cli` 只面向原生目标（需要 `render` feature）。仓库的 example
将编译入口拆成三个文件：

```text
examples/arc/
├── main.rs    # 原生 bin
├── wasm.rs    # WASM cdylib
└── scenes.rs  # 共享场景、ScenesPlugin 与 RenderSetup
```

`main.rs` 创建 App、安装 `scenes::ScenesPlugin` 后调用 `run_cli(app.build())`。
`wasm.rs` 使用同一套装配，但把定稿后的项目交给 JavaScript：

```rust,ignore
#![cfg(target_arch = "wasm32")]

mod scenes;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn app() -> ranim::wasm::WasmApp {
    use ranim::prelude::*;

    let mut app = App::new();
    app.add_plugins(scenes::ScenesPlugin);
    ranim::wasm::WasmApp::new(app.build())
}
```

在根 `Cargo.toml` 中分别声明两个 target：

```toml
[[example]]
name = "arc"
path = "examples/arc/main.rs"
required-features = ["preview"]

[[example]]
name = "arc-wasm"
path = "examples/arc/wasm.rs"
crate-type = ["cdylib"]
required-features = ["render"]
```

native 的 egui 预览需要 `preview`，WASM 直接向 canvas 呈现，只需要 `render`；
场景需要的 `typst`、`gltf` 等额外 feature 在两个 target 中一致声明。
仓库的 `main.rs` 使用
`#![cfg(not(target_arch = "wasm32"))]` 排除 WASM 下的原生代码，并用
`#![cfg_attr(target_arch = "wasm32", no_main)]` 让空 bin 无需 `main`；`wasm.rs`
通过 `#![cfg(target_arch = "wasm32")]` 限定 WASM 代码。两种平台的批量检查均会
排除另一种入口的实现。
`--example` 选择入口，`--target` 指定平台，Cargo 不会按平台自动切换 example 名称。
独立项目也可以使用 `[lib] crate-type = ["cdylib"]`，开启 `ranim/render` 并
直接依赖 `wasm-bindgen`，由自己的库导出上述 `app()`。

```bash
cargo build --example arc-wasm --target wasm32-unknown-unknown --features render
wasm-bindgen --target web --out-dir pkg --out-name arc target/wasm32-unknown-unknown/debug/examples/arc_wasm.wasm
```

页面初始化模块后，显式选择场景并启动预览：

```html
<canvas id="preview" width="1280" height="720"></canvas>
<script type="module">
  import init, { app } from "./pkg/arc.js";
  await init();
  const project = app();
  console.log(project.sceneNames());
  const canvas = document.getElementById("preview");
  const preview = await project.createPreview(canvas, "arc");
  project.free();
  preview.renderAt(1.5);
  // 页面卸载预览时：preview.dispose(); preview.free();
</script>
```

`sceneNames()` 返回以换行分隔的注册场景名。`createPreview` 的 Promise 等待
GPU 与场景初始化；返回的会话提供 `duration`、`renderAt(seconds)`、
`resize(width, height)`（物理像素）与 `dispose()`。播放时钟由 Web 端管理，
WASM 不自行启动事件循环。每个 `WasmApp` 创建一次预览；需要新的预览时重新
调用 `app()`。

仓库的 `cargo examples build [EXAMPLES...]` 会完成编译、`wasm-bindgen`
和 `wasm-opt`。播放器源码位于 `website/static/ranim-player.js`，由 Zola 直接
提供；为 rustdoc 等其他输出目录构建时，会复制同一份组件到输出根目录。
网站和 rustdoc 使用同一个 `<ranim-player>`，提供播放、暂停、进度拖动与
时间显示，无需 egui：

```html
<script type="module" src="./ranim-player.js"></script>
<ranim-player src="./examples/arc/pkg/ranim_example.js" scene="arc"></ranim-player>
```

### 命令行：预览与渲染

项目二进制的 `run_cli` 提供全部命令行入口：

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
