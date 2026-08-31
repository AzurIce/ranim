# Examples

每个 example 是 `examples/` 下的一个独立目录（入口为 `lib.rs`），需要在根
`Cargo.toml` 中显式注册 `[[example]]`（根 crate 设置了 `autoexamples = false`）。
部分例子声明了 `required-features`（如 `typst`、`gltf`），运行时需要开启对应
feature。

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
