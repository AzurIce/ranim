# Examples

`cargo examples` 会扫描根 `Cargo.toml` 中的所有 `[[example]]`，通过
`ranim output --example <name>` 渲染它们声明的 `#[output(...)]`，并把 website
需要的产物写入：

```text
website/
├── static/examples/<name>/       # 预览图 / 视频
├── data/<name>.toml              # 模板和短代码读取的数据
└── content/examples/<group>/<name>.md
```

其中 `<group>` 从 example 在 `examples/` 下的相对目录推导：

```text
examples/arc/lib.rs                       -> content/examples/arc.md
```

`xtask` 会自动为缺失的目录创建 Zola `_index.md`，所以 Zola 侧不需要手工维护
目录结构。

## Commands

```bash
# 构建共享 wasm bundle
cargo examples build

# 渲染全部 example 并刷新 website 输出
cargo examples run

# 只渲染指定 example
cargo examples run hanoi arc

# 已有数据未变化时跳过渲染，但会补齐缺失的 content 页面
cargo examples run --lazy-run

# 仅输出错误和最终摘要
cargo examples run --quiet

# 一个失败后立即停止
cargo examples run --fail-fast

# 清理已经不存在的 example 留下的 data / static / content 产物
cargo examples run --clean
```

## Zola

Zola 页面中仍使用 `!example-<name>` 插入 example 内容。列表页和样例页左侧导航
会按 `content/examples` 的目录层级渲染；嵌套 example 的页面会带 `Examples /
<group> / <name>` 面包屑。

`just doc-examples` 会为 `ranim-examples` 生成带 `#[wasm_demo_doc]` 实时预览的
rustdoc。
