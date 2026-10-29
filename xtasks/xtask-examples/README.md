# Examples

`cargo examples` 从根 `Cargo.toml` 的 `[[example]]` 发现例子，通过
`package.metadata.example.<name>.hide` 控制页面是否展示。每个例子只有一个 bin
target；`scenes.rs` 保存场景与注册，`main.rs` 保存平台入口。

网站在每次 `notist build/serve -p ranim-website` 时通过 Experimental Site 扩展
`cargo examples site` 生成页面和模板数据。只需要修改 Cargo 配置、Rust 源码和
README，无需维护或提交 `website/content/examples/`、`website/data/*.toml`。

```text
Cargo.toml + examples/**/{scenes.rs,main.rs,README.md}
    → cargo examples site（prepare 阶段，JSON stdio）
    → 内存中的 examples/<group>/<name>.md + site.data.<name>
    → Notist 解析、模板渲染 → dist/ranim-website/website/

.shadow/refs/ + shadow.toml → 已发布媒体 URL
website/render-state.json → 上次成功渲染的源码 hash
website/static/examples/<name>/pkg/ → 实际存在的 WASM 模块
```

`<group>` 从例子所在目录推导，例如
`examples/geometry/arc/main.rs` → `examples/geometry/arc.md`。扩展同时生成
各级目录的 `index.md`，并以例子的 README 作为正文。`examples/` 是扩展管理的
文档命名空间，手写文章放在 `content/` 的其他目录。隐藏例子不生成页面，仍提供数据，
供首页等模板引用。源码展示每次构建直接读取 `scenes.rs` 与 `main.rs`。

## Commands

```bash
# 构建各 example 的 WASM bin、绑定和优化后的模块
cargo examples build

# 仅构建指定 example
cargo examples build arc hanoi

# 输出到 rustdoc 目录
cargo examples build --out-dir target/doc/ranim_examples

# 渲染全部 example 并刷新 website 输出
cargo examples run

# 只渲染指定 example
cargo examples run hanoi arc

# 源码 hash 与上次成功渲染一致时跳过渲染
cargo examples run --lazy-run

# 仅输出错误和最终摘要
cargo examples run --quiet

# 一个失败后立即停止
cargo examples run --fail-fast

# 清理已经不存在的 example 留下的渲染记录和本地静态产物
cargo examples --clean run
```

## Notist SSG

从仓库根目录进入开发环境，其中包含固定的 Rust 工具链与 flake.lock 锁定的 Notist；CI 使用相同的 Notist 版本：

```bash
nix develop
export CARGO_BUILD_JOBS=4

# 构建 arc 的 WASM，然后启动 Notist SSG 开发服务器
just website-preview arc
```

打开 `http://127.0.0.1:8000/examples/arc.html`，切换到 **App** 页签使用实时
canvas 播放器；**Preview** 页签显示预渲染的视频或图片。
`just website-preview` 不带参数会先构建所有支持 WASM 的例子。

分步执行或生成完整静态站点：

```bash
# 第一步：构建选定例子的 WASM；省略名字则构建全部
cargo examples build arc gltf_showcase

# 第二步：开发预览（也监听 Cargo 配置、例子源码、README 和 shadow refs）
notist serve -p ranim-website

# 或生成 dist/ranim-website/website/；发布前先用 cargo examples build 构建全部例子
just website
```

修改 Rust 场景后，在另一个开发 shell 中重新运行 `cargo examples build arc`。
组件源码直接位于 `website/static/ranim-viewport.js` 与 `ranim-controls.js`，修改后 Notist SSG 会自动刷新，
无需重新构建 WASM。Rust 场景重建后，Notist SSG 也会监听更新后的静态文件并刷新页面。
只预览实时 WASM 不需要运行 `cargo examples run`，也不需要 shadow 凭证；
后者用于渲染和发布 Preview 页签的媒体。

`just website` 会调用扩展并生成当前页面与数据；无需先运行 WASM 构建或媒体
渲染。只有 JS 与 WASM 文件都存在时才显示 **App** 页签；**Preview** 使用 tracked
shadow refs，不需要本地下载媒体或 shadow 凭证。README 引用的本地文件会同步到
忽略入库的静态目录；已发布的资源链接使用 shadow URL，其余使用站内 URL。

`cargo examples run` 运行每个例子的 `output` 子命令、同步媒体并执行
`shadow publish`，成功后才更新 `website/render-state.json`。`cargo examples publish`
只发布已有静态产物。构建页面、WASM 或仅发布媒体均不会改变上次成功渲染的 hash，
因此 `--lazy-run` 仍能发现需要重渲染的场景。提交媒体变更时提交 `.shadow/refs/`
和渲染记录，不提交静态媒体、生成的页面或模板数据。

根 `Notist.toml` 声明 Book 与 Website 两个 workspace 成员；扩展配置位于
`website/Notist.toml` 的 `[[site.extensions]]`。Notist 仍从站点目录
启动 `cargo examples site`，通过 stdin 传入 `version: 1`、`stage: "prepare"`、
`root`、`config` 和 `input: {documents, data}`；命令 stdout 返回
`{version: 1, input: {documents, data}, errors: []}`，日志走 stderr。扩展保留其他
页面和数据，重建 `examples/` 文档与各例子的数据键。`watch` 额外监听项目输入，
所以修改例子、Cargo 配置或媒体 refs 会在开发服务器中重新生成页面。
协议仍为 Experimental，随 Notist 固定版本更新。

示例页通过 `@!(template: "example", example: "<name>")` 声明模板和数据键。
列表页和样例页左侧导航按生成的目录层级渲染；嵌套 example 的页面带
`Examples / <group> / <name>` 面包屑。
Handlebars 局部模板在 `website/templates/components/` 中，`player.hbs` 输出共享的
`<ranim-viewport>` 与 `<ranim-controls>` Web Components，显式绑定后组合展示。

`just doc-examples` 会为 `ranim-examples` 生成带 `#[wasm_demo_doc]` 实时预览的
rustdoc。

`ranim-examples` 的 library 引用各 example 的 `scenes.rs`，用于生成文档。
它仅依赖共享场景和 `render`，不启用 native `cmd-preview`。
每个例子只注册同名 bin target，`main.rs` 按平台选择入口：原生调用 CLI，
WASM 提供空 `main()`，并在同一文件直接导出 `app() -> WasmApp`，两种平台
共用 `mod scenes` 中的完整项目装配。所有注册的 example 都遵循这一可移植入口约定，
xtask 不再依赖额外文件识别 WASM 支持；`package.metadata.example` 只保存 `hide` 等展示设置。

`cargo examples build` 按 `required-features` 对所选 target 分组编译，显式选择
`--no-default-features`，选择同名 `<name>` example 与 `wasm32-unknown-unknown` 平台。只有声明需要的例子才会链接
Typst 等可选依赖。随后分别运行 `wasm-bindgen` 和 `wasm-opt`。产物位于
`website/static/examples/<name>/pkg/ranim_example.js` 和对应的 `_bg.wasm`。
`--out-dir` 改变输出根目录，仍保留 `examples/<name>/pkg/` 层级。
组件源码由 Notist SSG 的 `website/static/ranim-viewport.js` 和 `ranim-controls.js`
管理；网站直接使用，指定其他 `--out-dir`（如 rustdoc 目录）时将两个文件
复制到输出根目录，同时复制视口使用的 `ranim-audio.js` 和控制条使用的
`ranim-timeline.js`，以及共用主题模块 `ranim-theme.js`。
组件继承宿主字体和配色；网站把画布、控制条与 timeline 放进同一个细边框容器。
视口调用 `await app().createPreview(canvas, name)`，
用 Web 端播放时钟驱动 `renderAt(seconds)`；独立控制条通过 `for="视口ID"`
或 `controls.target = viewport` 连接，布局和数量由宿主决定。WASM 宿主只需要 `render`，内置示例另声明 `anims`；
egui/eframe 属于 native 的 `cmd-preview`。原生导出同样关闭 default，
在共享 `required-features` 上追加 `cmd-render` 构建，不编译预览窗口。

有声音的场景通过 `WasmPreview.audio()` 读取一次预混音 PCM，由 Web Audio 播放；
画面跟随音频输出时钟，播放、暂停和定位同时控制声音。无声音场景使用墙钟。
切换场景或卸载视口会停止音源并关闭音频设备，移动或增减控制条不影响音频会话。

控制条可通过右侧展开按钮、`timeline-expanded` 或 `timelineExpanded` 属性打开
动画 timeline。视口就绪时缓存一次 `WasmPreview.timeline()` 的元数据，控制条
据此显示动画树和片段；播放帧只同步指针，不重复读取动画树。
