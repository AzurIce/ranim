# Examples

每个 example 是 `examples/` 下的一个独立目录（入口为 `lib.rs`），需要在根
`Cargo.toml` 中显式注册 `[[example]]`（根 crate 设置了 `autoexamples = false`）。
部分例子声明了 `required-features`（如 `typst`、`gltf`），运行时需要开启对应
feature。

每个目录的 `scenes.rs` 保存场景实现和 `ScenesPlugin`，在插件的 `build` 中通过
`app.add_scenes([...])` 显式注册该例子的场景。`lib.rs` 再导出 `ScenesPlugin`，并
定义带 `#[ranim::export_plugin]` 的 `pub fn plugin() -> impl Plugin` 工厂，
返回 `ScenesPlugin`，并导出唯一的原生动态库入口 `ranim_plugin`。
宿主创建 App 后安装插件；Rust 应用可直接调用
`app.add_plugins(example::ScenesPlugin)` 或 `app.add_plugins(example::plugin())`。
`#[scene]` 生成同名模块中的 owned `scene()` 描述符，注册由插件完成。

这里拆分 `scenes.rs` 与 `lib.rs` 是为了复用示例源码：共享库通过 `#[path]`
引用各例子的场景与插件定义。如果直接引用带有 `#[ranim::export_plugin]` 的 `lib.rs`，
同一个原生产物就会包含多个同名 `ranim_plugin` 导出。普通项目可以把场景、插件定义
和导出宏放在同一个 `lib.rs` 中。

共享的 `ranim-examples` 库直接引用各例子的 `scenes.rs`，通过 `ExamplesPlugin`
组合各个 `ScenesPlugin`，然后导出这个组合插件。独立的
`ranim-examples-preview` 二进制创建 App、安装组合插件，并为网站与 rustdoc
导出 `preview_scene("注册名")`。Web 构建选择该二进制；插件库不创建 App 或
预览入口。网站展示场景实现与插件入口，rustdoc 的示例来自 `scenes.rs`。

## 运行

```bash
# 渲染单个 example（产物写到 #[output(dir)] 声明的目录）
cargo run -p ranim-cli -- output --example <name>

# 渲染全部 example 并刷新 website 产物
cargo examples run
```

website 侧的产物布局与命令详见
[xtasks/xtask-examples](../xtasks/xtask-examples/README.md)。

## One-shot examples

由 agent 一次性（one-shot）编写的例子尝试已移至独立仓库 `ranim-one-shot`，
主仓库不再维护 `examples/agents/` 目录。
