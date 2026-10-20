# Examples

每个 example 是 `examples/` 下的一个独立目录（入口为 `main.rs`），需要在根
`Cargo.toml` 中显式注册 `[[example]]`（根 crate 设置了 `autoexamples = false`）。
所有 example 都声明 `required-features = ["preview"]`；需要额外 feature 的
例子在此基础上合并（如 `["preview", "typst"]`、`["preview", "gltf"]`）。

每个目录的 `scenes.rs` 保存场景实现和 `ScenesPlugin`，只通过
`app.add_scenes([...])` 注册场景。简单 example 的 `main.rs` 只有
`mod scenes;` 与 `fn main()`，直接把 `ScenesPlugin` 装进 App；项目未注册
`RenderSetup` 时，渲染与预览自动回退到内建的 `RanimItemsPlugin`
（camera frame、VItem、MeshItem）。只有需要自定义渲染的 example 才在
`main.rs` 中定义 `ProjectPlugin`，向项目 App 插入 `RenderSetup`，闭包内以
`RanimItemsPlugin` 为基础，再叠加该项目完整输出类型对应的
`ExtractPlugin<T>` 或场景私有的渲染配置（`scenes::configure_render`）。
`Vec<VItem>` 已由 `VItemPlugin`（内含于 `RanimItemsPlugin`）注册；其余
`Vec<T>`、`Transformed<T, G>` 等组合输出仍需显式注册完整类型，嵌套值由
`Extract` 实现递归处理。私有输出类型的注册保留在相应 `scenes.rs` 中。

`main.rs` 末尾的 `fn main()` 通过
`App::new().add_plugins(scenes::ScenesPlugin).run()`（有 `ProjectPlugin`
的 example 换成它）把项目交给命令行入口
（`ranim::cmd::AppRunExt`，由 `ranim::prelude` 在 native + `render` feature
下导出），提供 `preview` / `output` / `render` / `inspect` 子命令。
`#[scene]` 生成同名模块中的 owned `scene()` 描述符，注册由插件完成。

这里拆分 `scenes.rs` 与 `main.rs` 是为了复用示例源码：共享库通过 `#[path]`
只引用各例子的 `scenes.rs`（场景与 `ScenesPlugin`），而每个 example 的
`fn main`（以及可选的 `ProjectPlugin`）留在 `main.rs` 中。普通项目可以把
场景、插件定义和 `main` 放在同一个 `main.rs` 中。

共享的 `ranim-examples` 库（纯 rlib）由自己的 `ProjectPlugin` 组合各个
`ScenesPlugin`，以 `RanimItemsPlugin` 统一声明内建渲染插件并注册公共输出
类型，再叠加各模块的私有输出类型与渲染配置。
独立的 `ranim-examples-preview` 二进制创建 App 并安装组合插件：native 下走
`AppRunExt::run()` 命令行入口，wasm 下为网站与 rustdoc 导出
`preview_scene("注册名")`。Web 构建选择该二进制。网站展示场景实现与插件入口，
rustdoc 的示例来自 `scenes.rs`。

## 运行

```bash
# 交互式预览（缺省第一个场景，可指定场景名）
cargo run --example <name> --features preview -- preview [SCENE]

# 渲染该 example 声明的全部 #[output(...)]（产物写到 #[output(dir)] 声明的目录）
cargo run --example <name> --features preview -- output

# 以默认输出设置（1920x1080、60fps、mp4）渲染单个场景
cargo run --example <name> --features preview -- render <SCENE>

# 需要额外 feature 的 example 把对应 feature 一并开启
cargo run --example text_item --features preview,typst -- preview

# 渲染全部 example 并刷新 website 产物
cargo examples run
```

website 侧的产物布局与命令详见
[xtasks/xtask-examples](../xtasks/xtask-examples/README.md)。

## One-shot examples

由 agent 一次性（one-shot）编写的例子尝试已移至独立仓库 `ranim-one-shot`，
主仓库不再维护 `examples/agents/` 目录。
