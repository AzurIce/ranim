# Examples

每个 example 在根 `Cargo.toml` 中显式注册（`autoexamples = false`）。
支持浏览器预览的 example 使用三个文件：

```text
examples/arc/
├── main.rs    # [[example]] bin：mod scenes; run_cli(app.build())
├── wasm.rs    # [[example]] cdylib：mod scenes; 导出 app() -> WasmApp
└── scenes.rs  # 两个 target 共享的场景、插件和渲染装配
```

`ScenesPlugin` 注册场景，并在需要时插入项目自己的 `RenderSetup`；
自定义输出的 `ExtractPlugin<T>` 也在这里装配。两个入口创建 App、安装同一个
`scenes::ScenesPlugin`，再调用 `app.build()` 定稿。原生入口交给 `run_cli`，
WASM 入口通过 `#[wasm_bindgen] pub fn app()` 返回 `ranim::wasm::WasmApp`。
JavaScript 调用 `await app().createPreview(canvas, name)` 创建预览会话，
再用 `renderAt(seconds)` 驱动画面；`sceneNames()` 返回以换行分隔的注册场景名。
每个项目 handle 创建一次预览，需要另一个预览时重新调用 `app()`。
网站和 rustdoc 共用 [`<ranim-viewport>` 与 `<ranim-controls>`](../website/README.md)：
视口负责 canvas 和播放会话，独立控制条通过 `for` 或 `target` 绑定视口，
提供播放、暂停和拖动进度；WASM 直接呈现到 canvas，不依赖 egui。
控制条右侧的展开按钮可打开动画树与片段轨道，点击片段只查看详情，
点击或拖动上方时间刻度可定位，还支持缩放和折叠节点；
也可通过 `timeline-expanded` 属性或 `controls.timelineExpanded = true` 默认展开。
有声音的场景（如 `midi_visualizer`）在点击播放后通过 Web Audio 发声，
画面跟随音频时钟；播放、暂停和定位保持同步，暂停中定位不发声。
扬声器按钮可调节音量或静音，同一视口的控制条共享音量设置。

原生 target 名为 `<name>`，需要 `anims,cmd-render`；WASM target 名为 `<name>-wasm`，
设置 `crate-type = ["cdylib"]`，需要 `anims,render`（其中 `anims` 启用内置物件）。需要 `typst` 等额外
feature 的例子在两个 target 中一致声明。
`main.rs` 使用以下 crate 级属性限定原生代码：

```rust
#![cfg_attr(target_arch = "wasm32", no_main)]
#![cfg(not(target_arch = "wasm32"))]
```

WASM 批量检查时整个原生入口（含 `mod scenes` 与 CLI）被排除；`no_main` 让这个
空的 bin target 无需提供 `main`。`wasm.rs` 则通过
`#![cfg(target_arch = "wasm32")]` 限定 WASM 代码，原生批量检查时该 cdylib 为空。
编译时用 `--example <name>` / `--example <name>-wasm` 显式选择入口，
用 `--target wasm32-unknown-unknown` 指定 WASM 平台；Cargo 不会按平台自动切换
example 名称。批量构建 WASM 包使用 `cargo examples build`。
`gltf_showcase` 通过 `include_bytes!` 嵌入 GLB 模型，原生与 WASM 使用同一套
无文件系统 I/O 的模型加载与项目装配。

未提供 `RenderSetup` 时，渲染与预览使用默认的 `RanimItemsPlugin`。
它包含 camera frame、VItem、MeshItem 及 `Vec<VItem>` 的渲染支持；其他
`Vec<T>`、`Transformed<T, G>` 等输出需要显式注册完整类型。
`#[scene]` 生成同名模块中的 `scene()` 描述符，插件通过 `app.add_scenes([...])`
注册它们。

`ranim-examples` 仅作为 rustdoc 库，库入口保留在
`examples/ranim_examples_bundle.rs`，通过 `#[path]` 引用各例子的 `scenes.rs`，
该布局用于规避 rust-analyzer 的问题。
聚合库只启用渲染装配，不依赖 native `cmd-preview`。
网站和 rustdoc 都加载各 example 自己的 WASM 模块，使用与原生入口相同的项目装配。

## 运行

```bash
# 交互式预览（缺省第一个场景，可指定场景名）
cargo run --example <name> -- preview [SCENE]

# 渲染该 example 声明的全部 #[output(...)]（产物写到 #[output(dir)] 声明的目录）
cargo run --example <name> --no-default-features --features anims,cmd-render -- output

# 以默认输出设置（1920x1080、60fps、mp4）渲染单个场景
cargo run --example <name> --no-default-features --features anims,cmd-render -- render <SCENE>

# 需要额外 feature 的 example 把对应 feature 一并开启
cargo run --example text_item --features typst -- preview

# 只编译 arc 的 WASM cdylib
cargo build --example arc-wasm --target wasm32-unknown-unknown --no-default-features --features anims,render

# 批量检查全部 example target，原生入口在 WASM 下为空
cargo check -p ranim --examples --target wasm32-unknown-unknown --no-default-features --features anims,cmd-preview,typst,gltf

# 构建网站需要的各例子 WASM 包（也可只构建 arc）
cargo examples build
cargo examples build arc

# 渲染全部 example 并刷新 website 产物
cargo examples run
```

website 侧的产物布局与命令详见
[xtasks/xtask-examples](../xtasks/xtask-examples/README.md)。

## One-shot examples

由 agent 一次性（one-shot）编写的例子尝试已移至独立仓库 `ranim-one-shot`，
主仓库不再维护 `examples/agents/` 目录。
