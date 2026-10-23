# Examples

`cargo examples` 会扫描根 `Cargo.toml` 中的原生 `[[example]]`（跳过 `-wasm` target），通过运行各 example
二进制的 `output` 子命令（`cargo run --release --example <name> --features ... -- output`）
渲染它们声明的 `#[output(...)]`，并把 website 需要的产物写入：

```text
website/
├── static/examples/<name>/       # 预览图 / 视频
├── data/<name>.toml              # 模板和短代码读取的数据
└── content/examples/<group>/<name>.md
```

其中 `<group>` 从 example 在 `examples/` 下的相对目录推导：

```text
examples/arc/scenes.rs                    -> content/examples/arc.md
```

`xtask` 会自动为缺失的目录创建 Zola `_index.md`，所以 Zola 侧不需要手工维护
目录结构。`data/<name>.toml` 的 `code` 同时展示 `scenes.rs`（场景与注册）
、`main.rs`（原生入口）和 `wasm.rs`（WASM 入口），各自附有文件名。

## Commands

```bash
# 构建各 example 的 wasm cdylib、绑定和优化后的模块
cargo examples build

# 仅构建指定 example
cargo examples build arc hanoi

# 输出到 rustdoc 目录
cargo examples build --out-dir target/doc/ranim_examples

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

从仓库根目录进入开发环境（Zola 0.23.6，与 CI 一致）：

```bash
nix develop
export CARGO_BUILD_JOBS=4

# 构建 arc 的 WASM，然后启动 Zola 开发服务器
just website-preview arc
```

打开 `http://127.0.0.1:1111/examples/arc/`，切换到 **App** 页签使用实时
canvas 播放器；**Preview** 页签显示预渲染的视频或图片。
`just website-preview` 不带参数会先构建所有支持 WASM 的例子。

分步执行或生成完整静态站点：

```bash
# 第一步：构建选定例子的 WASM；省略名字则构建全部
cargo examples build arc gltf_showcase

# 第二步：开发预览（监听 website 下的文件并自动刷新）
zola --root website serve --base-url http://127.0.0.1

# 或生成 website/public/；发布前先用 cargo examples build 构建全部例子
just website
```

修改 Rust 场景后，在另一个开发 shell 中重新运行 `cargo examples build arc`。
播放器源码直接位于 `website/static/ranim-player.js`，修改后 Zola 会自动刷新，
无需重新构建 WASM。Rust 场景重建后，Zola 也会监听更新后的静态文件并刷新页面。
只预览实时 WASM 不需要运行 `cargo examples run`，也不需要 shadow 凭证；
后者用于渲染和发布 Preview 页签的媒体。

构建到网站目录时也会同步所选例子的源码展示和页面；没有预渲染媒体的例子
直接显示 App 页签。已有媒体 URL 和上次渲染的 hash 保持不变，后续
`cargo examples run --lazy-run` 仍能识别需要重新渲染的场景。

Zola 页面中仍使用 `!example-<name>` 插入 example 内容。列表页和样例页左侧导航
会按 `content/examples` 的目录层级渲染；嵌套 example 的页面会带 `Examples /
<group> / <name>` 面包屑。
Tera 组件在 `website/templates/components/` 中，`examples.player` 输出共享的
`<ranim-player>` Web Component。

`just doc-examples` 会为 `ranim-examples` 生成带 `#[wasm_demo_doc]` 实时预览的
rustdoc。

`ranim-examples` 的 library 引用各 example 的 `scenes.rs`，用于生成文档。
它仅依赖共享场景和 `render`，不启用 native `preview`。
每个例子的 `<name>-wasm` target 独立导出 `app() -> WasmApp`，共享该例子原生
入口使用的完整项目装配。是否支持 WASM 由配对的 cdylib target 推导，
`package.metadata.example` 只保存 `hide` 等展示设置。

`cargo examples build` 按 `required-features` 对所选 target 分组编译，显式选择
`<name>-wasm` 入口与 `wasm32-unknown-unknown` 平台。只有声明需要的例子才会链接
Typst 等可选依赖。随后分别运行 `wasm-bindgen` 和 `wasm-opt`。产物位于
`website/static/examples/<name>/pkg/ranim_example.js` 和对应的 `_bg.wasm`。
`--out-dir` 改变输出根目录，仍保留 `examples/<name>/pkg/` 层级。
播放器源码由 Zola 的 `website/static/ranim-player.js` 管理；网站直接使用，
指定其他 `--out-dir`（如 rustdoc 目录）时才复制到输出根目录。网站和 rustdoc 共用
`<ranim-player>` 组件：组件调用 `await app().createPreview(canvas, name)`，
用 Web 端播放时钟驱动 `renderAt(seconds)`。WASM 只需要 `render`，
egui/eframe 属于 native 的 `preview`。
