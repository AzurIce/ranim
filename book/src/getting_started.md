# Getting Started

> [!caution]
> 本书绝大多数内容目前均由 ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

Ranim 的场景由一个 `fn(&mut RanimScene)` 函数构造。场景函数只负责定义动画；Plugin 注册场景与项目资源，CLI、Rust 应用和 Web 预览各自创建 App，再安装同一个插件。

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

### Prelude 覆盖什么

`ranim::prelude::*` 覆盖日常作者 API：`Eval` / `IntoAnimNode` / `Unplaced`
/ `PlaybackExt` 等动画协议，`AnimSequence` / `AnimStack` / `AnimLagged` 与
`seq!` / `stack!` / `lagged!`，`Pure` / `Iterative` / `Static`，`Sound` /
`AudioClip`，以及 `RanimScene` / `TimeMark` / `App` / `Plugin`。

以下类型有意**不**放进 prelude，需要时显式导入：

```rust,ignore
use ranim_core::Extract;
use ranim_core::audio::AudioError;
use ranim_core::animation::build::{At, Paramed};
use ranim_core::animation::node::{AnimNode, AnimationInfo};
use ranim_core::core_item::CoreItem;
```

- `Extract` / `CoreItem`：只在实现自定义 `Eval::Output` 的提取契约时需要；
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
接着在插件中显式注册，再从 crate 根导出统一入口：

```rust,ignore
pub struct DemoScenesPlugin;

impl Plugin for DemoScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_scene(hello::scene());
        // 多个场景也可以用 app.add_scenes([first::scene(), second::scene()]);
    }
}

#[ranim::export_plugin]
pub fn plugin() -> impl Plugin {
    DemoScenesPlugin
}
```

每个插件 crate 用一次 `#[ranim::export_plugin]` 标注入口工厂函数。
宏保留这个具有原返回类型的 Rust 函数，供宿主直接安装；原生动态库另导出
`ranim_plugin`，供 CLI 加载。这个统一入口将插件装箱，并捕获构造时的 panic，
返回 `Result<Box<dyn Plugin>, String>`。
App 的构造、场景选择和预览启动由宿主负责，包括 WASM 宿主。
只有加入 App 的场景才会出现在 CLI 和 Web 入口中。
场景目录保留显式注册顺序，场景名称必须唯一。

### 用 Plugin 装配项目

插件持有自己的配置，在 `build(&self, app: &mut App)` 中安装项目能力。
Rust 宿主既可以直接安装插件类型，也可以安装另一个 crate 导出的插件：

```rust,ignore
let mut app = App::new();
app.add_plugins(DemoScenesPlugin);

// 在另一个应用中使用这个 crate：
let mut app = App::new();
app.add_plugins(my_project::plugin());
```

插件在添加时执行构建，也可以通过 `app.add_plugins(...)` 继续安装依赖插件。
多个插件可写成元组，例如 `app.add_plugins((GeometryPlugin, TextPlugin))`。
工厂函数可以任意命名，返回具体插件类型或 `impl Plugin`；函数必须无参数、无泛型，
且是安全的同步函数。带配置的插件直接在函数体中构造，例如
`DemoPlugin { quality: 4 }`，不要求实现 `Default`。App 由宿主配置和持有。
项目资源保存在各自的 App 中，多个项目互不共享全局注册表；渲染、预览和导出会持有
相应 App，保持项目资源的生命周期。当前插件层负责装配场景和资源，具体 item 的渲染
注册将在后续扩展。

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

直接在 Rust 应用中运行时，创建 App 并安装插件，再选择 `SceneHandle`：

```rust,ignore
let mut app = App::new();
app.add_plugins(DemoScenesPlugin); // 也可以使用 my_project::plugin()。
let project = std::sync::Arc::new(app);
let scene = project.scene("hello").expect("hello is registered");

// 需要 render feature，渲染场景声明的所有输出；2 是 readback 缓冲数量。
ranim::cmd::render_scene(&scene, 2);
// 需要 preview feature，预览窗口持有这个场景与所属 App。
ranim::cmd::preview_scene(scene);
```

`SceneHandle` 保留 App 的所有权，预览重建渲染器和后台导出时都能继续访问同一项目。
CLI 通过动态库的 `ranim_plugin` 入口加载同一个插件，再安装到自己创建的 App 中。
Rust 原生宿主也可以启用 `dylib` feature 后自行加载：

```rust,ignore
use ranim::{App, DynamicPlugin};

let mut app = App::new();
// 调用方保证动态库使用匹配的 Ranim 版本和 Rust 工具链。
let plugin = unsafe { DynamicPlugin::load("./libmy_project.so")? };
app.try_add_plugins(plugin)?;
```

App 会持有加载的动态库，直到相关插件、资源与场景句柄全部释放。
动态加载入口使用 Rust ABI，宿主与插件必须使用匹配的 Ranim 版本和 Rust 工具链。

### WASM 预览宿主

把场景、`DemoScenesPlugin` 和带 `#[ranim::export_plugin]` 的工厂函数放在 `src/lib.rs`，再用
`src/main.rs` 编写预览宿主。普通项目可以把场景和插件定义直接写在 `lib.rs`；
是否拆成多个模块取决于代码组织需要。

WASM 宿主启用 `preview`，创建 App 后安装库导出的插件，再启动所选场景的预览。
插件库可以继续供原生 CLI 加载；Web 构建选择宿主二进制。

在 `Cargo.toml` 中为 `ranim` 开启预览功能，并给宿主二进制单独命名：

```toml
[lib]
crate-type = ["rlib", "cdylib"]

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
    app.add_plugins(my_project::plugin());
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

### CLI 渲染与预览

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
