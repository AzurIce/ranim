# 项目入口：场景库与跨平台宿主

**场景的写法和放置位置没有变；渲染与预览的入口是显式组装的 App。**

把这两件事分开理解：

- **定义场景**：普通 Rust 函数接收 `&mut RanimScene`，描述动画。
  可以放在 lib 中，供多个程序复用，也可以写在二进制的模块中。
- **运行项目**：宿主二进制选择场景，通过 `App` 注册它们及项目配置，
  再根据目标平台提供 CLI 或 JavaScript 入口。

`#[scene]` 不会扫描或自动注册整个程序里的场景，也不生成 `main()`。
库中的场景不需要知道调用者是 native CLI、浏览器，还是另一个项目。

## 一个包同时包含场景库和入口二进制

以包名 `my-animation` 为例：

```text
my-animation/
├── Cargo.toml
└── src/
    ├── lib.rs     # 场景定义，可复用；Rust crate 名为 my_animation
    └── main.rs    # 同一个 bin target：native CLI / WASM app() 导出
```

这是 Cargo 本来就支持的 lib + bin 布局，不需要动态库或额外的场景注册工具。
也可以把 `lib.rs` 的内容放进独立的场景库 crate，再由另一个包的二进制依赖它。
这里的 WASM 产物来自 bin target，场景库保持普通 Rust 库即可，
不需要为了浏览器预览给它添加 `crate-type = ["cdylib"]`。

### Cargo 配置

下面使用当前 git API。`native` 是这个示例项目自己定义的 feature，不是
Ranim 的 feature；它把原生命令与场景共享的能力分开：

```toml
[package]
name = "my-animation"
version = "0.1.0"
edition = "2024"

[features]
default = ["native"]
native = ["ranim/cmd-preview"]

[dependencies]
ranim = { git = "https://github.com/AzurIce/ranim", default-features = false, features = ["anims", "render"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2.129"
```

共享部分开启 `anims`（同时开启内置 `items`）和 `render`。
native 默认再开启 `cmd-preview`，它会间接开启 `cmd-render`；
WASM 构建使用 `--no-default-features`，不需要原生 CLI、egui 或音频设备依赖。
若只需要 native 导出视频，可以把 `native` 改为 `["ranim/cmd-render"]`。

### lib.rs：场景仍是场景

```rust
// src/lib.rs
use ranim::{items::vitem::geometry::Square, prelude::*};

#[scene]
pub fn hello(r: &mut RanimScene) {
    r.play(CameraFrame::default().show().with_duration(2.0));
    r.play(Square::new(2.0).show().with_duration(2.0));
}
```

这里有两个不同的入口：

- `my_animation::hello(&mut scene)`：原来的场景构造函数。
- `my_animation::hello::scene()`：宏生成的描述符工厂，返回 `Scene`，其中包含
  场景名称、构造函数、清屏颜色和输出配置。创建描述符不会执行动画构造函数。

`pub` 同时让函数和宏生成的同名模块对外可见。场景在更深的模块中时，
还需要让外层模块可见，或者从库中再导出相应的 API。

**定义场景不等于注册场景。** 库不负责自动把所有场景加入调用者的项目。

### main.rs：组装 App，再选择平台入口

```rust
// src/main.rs
use ranim::prelude::*;

// 两个平台使用相同的场景选择和项目配置。
fn project() -> ranim::RanimApp {
    let mut app = App::new();
    app.add_scenes([my_animation::hello::scene()]);
    app.build()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run_cli(project());
}

// WASM 也编译同一个 binary target；main 不启动 CLI 或事件循环。
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn app() -> ranim::wasm::WasmApp {
    ranim::wasm::WasmApp::new(project())
}
```

`project()` 是示例自己提取的共享装配函数，名称没有特殊含义。
`ranim::App` 就是 `bevy_app::App`；prelude 导出的 `AppExt` 提供场景注册和选择方法，
`RanimAppExt` 提供 `build()`。

`app.build()` 定稿插件构建，执行一次插件的 `finish()` / `cleanup()`，
返回会话期项目 `RanimApp`。它不是视频编码或逐帧渲染，也不会把所有场景立即求值。
场景构造函数由后续的预览、渲染或检查会话在需要时调用。

只有注册进这个 `App` 的场景，才会暴露给它的 CLI 或浏览器预览。
不同宿主可以从同一个场景库选择不同子集；场景名称在每个 App 中必须唯一。

## Native：二进制运行 CLI

```bash
cargo run -- preview hello
cargo run -- output hello
cargo run -- inspect scenes
```

native 的 `main()` 把组装后的项目交给 `run_cli`，由它解析进程参数、
选择场景并启动会话。`preview` 是原生 egui/eframe 窗口；`output` 输出视频；
`inspect` 查询场景、动画树或帧数据。参数与 feature 要求见[命令行入口](./cli.md)。

## WASM：二进制导出 app()，由 JavaScript 驱动

同一个 bin target 可以编译成 WASM 模块：

```bash
cargo build --bin my-animation --target wasm32-unknown-unknown --no-default-features
wasm-bindgen --target web --out-dir pkg --out-name my_animation target/wasm32-unknown-unknown/debug/my-animation.wasm
```

需要安装 Rust 的 `wasm32-unknown-unknown` target；`wasm-bindgen` CLI 的版本
必须与 Cargo 实际解析到的 `wasm-bindgen` crate 一致。本仓库的 Nix shell 已提供
对应工具，仓库 example 也可用 `cargo examples build <example>` 自动完成绑定与优化。

`pub fn app()` 不是 native linker 查找的 `main` 符号，而是通过
`#[wasm_bindgen]` 暴露给 JavaScript 的模块导出。加载模块时空的 `main()`
立即返回，不会自动注册项目、申请 GPU 或开始播放。

页面显式调用导出的 `app()`：

```html
<canvas id="preview" width="1280" height="720"></canvas>
<script type="module">
  import init, { app } from "./pkg/my_animation.js";

  await init();
  const project = app();
  console.log(project.sceneNames());
  const canvas = document.getElementById("preview");
  const preview = await project.createPreview(canvas, "hello");
  project.free();
  preview.renderAt(1.0);
  // 会话结束时：preview.dispose(); preview.free();
</script>
```

`app()` 每次组装并返回一个新项目；`createPreview` 选择其中一个已注册的场景，
初始化 GPU 和场景，并把项目移交给预览会话。一个 `WasmApp` 只能创建一次预览，
需要另一个会话时再次调用 `app()`。

`renderAt(seconds)` 只渲染指定时刻，播放时钟、按钮和浏览器事件循环由
JavaScript 管理。WASM 并不是把 native CLI 或 egui 窗口搬到网页里。
可直接使用仓库的 `ranim-viewport` / `ranim-controls` 组件，见
[Getting Started 的 WASM 预览宿主](./getting_started.md#wasm-预览宿主)。

## Plugin 是可选的复用方式

如果不想让宿主逐个注册场景，场景库可以同时提供一个 Plugin：

```rust
// 追加到 src/lib.rs
pub struct ScenesPlugin;

impl Plugin for ScenesPlugin {
    fn build(&self, app: &mut App) {
        app.add_scenes([hello::scene()]);
    }
}
```

宿主的 `project()` 改为：

```rust
fn project() -> ranim::RanimApp {
    let mut app = App::new();
    app.add_plugins(my_animation::ScenesPlugin);
    app.build()
}
```

Plugin 封装的是注册和项目配置，不是新的场景编写格式。它也可以携带配置，
或安装自定义 `RenderSetup`。启用内置 `items` 时，普通内置场景使用默认渲染配置即可；
复合输出或自定义类型的装配见[完整求值帧与渲染提取](./understand/core/extract.md)。

## 仓库的 example 仍然是 example

仓库的 `[[example]]` target 把共享代码放在 `scenes.rs` 模块，而不是 lib：

```text
examples/hello_ranim/
├── main.rs     # native main() / WASM 空 main() 和 app() 导出
└── scenes.rs   # 场景与可选 ScenesPlugin
```

这是示例的组织选择，不是框架要求。普通项目把同样的场景和 Plugin 放进 lib 后，
入口只需把 `scenes::ScenesPlugin` 换成库导出的插件，例如 `my_animation::ScenesPlugin`。
无需复制场景，也无需给 example 添加独立 Cargo 项目。

## 热重载不会改变场景库与宿主的边界

可选的 `subsecond` feature 给场景 constructor 增加热调用边界。native 预览收到
补丁后重新构建当前场景，在原播放位置暂停；它不会重跑整个项目的 Plugin 注册。

**场景能放在 lib 中，与特定版本 dx 能否热补丁这个 lib，是两个问题。**
普通编译、渲染与预览支持上述 lib + bin 布局。修改库中场景能否无需重启生效，
取决于 dx 的 workspace 补丁支持；不能把工具限制写成场景必须在 `main.rs` 中的 API 约束。
项目注册、配置、依赖和类型布局改变后仍应重新构建并重启。

本仓库 flake 固定的 dx main 包含 0.7.10 缺少的 workspace 级联筛选修复；
该 main CLI 与当前 0.7 Subsecond runtime 的端到端兼容性尚待验证。
