# Examples

`cargo examples` 从根 `Cargo.toml` 的 `[[example]]` 发现例子，通过
`package.metadata.example.<name>.hide` 控制页面是否展示。
设置 `package.metadata.example.<name>.standalone = true` 的原生独立示例不进入
批量构建、渲染或网站发现流程。每个 bundled 例子只有一个 bin
target；`scenes.rs` 保存场景与注册，`main.rs` 保存平台入口。

网站在每次 `notist build/serve -p ranim-website` 时通过 Experimental Site 扩展
`cargo examples site` 生成页面和模板数据。只需要修改 Cargo 配置、Rust 源码和
README，无需维护或提交 `website/content/examples/`、`website/data/*.toml`。

```text
Cargo.toml + examples/**/{scenes.rs,main.rs,README.md}
    → cargo examples site（before_compile / before_template，JSONL）
    → 内存中的 examples/<group>/<name>.md + site.data.<name>
    → Notist 解析、模板渲染 → notist-output/ranim-website/

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

# 渲染全部 example 并整理本地 website 输出，不上传
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

# 按需手动发布已有产物，不重新渲染
cargo examples publish
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

# 或生成 notist-output/ranim-website/；发布前先用 cargo examples build 构建全部例子
just website
```

修改 Rust 场景后，在另一个开发 shell 中重新运行 `cargo examples build arc`。
组件源码统一位于仓库根的 `web/`，修改后 Notist SSG 会自动刷新，
无需重新构建 WASM。Rust 场景重建后，Notist SSG 也会监听更新后的静态文件并刷新页面。
只预览实时 WASM 不需要运行 `cargo examples run`；后者只在本地渲染和整理 Preview 媒体，
不需要 shadow 或上传凭证。需要发布时再手动运行 `cargo examples publish`。

`just website` 会调用扩展并生成当前页面与数据；无需先运行 WASM 构建或媒体
渲染。只有 JS 与 WASM 文件都存在时才显示 **App** 页签；**Preview** 使用 tracked
shadow refs，不需要本地下载媒体或 shadow 凭证。README 引用的本地文件会同步到
忽略入库的静态目录；已发布的资源链接使用 shadow URL，其余使用站内 URL。

`cargo examples run` 运行每个例子的 `output` 子命令，将媒体和 README 资源整理到
`website/static/examples/<name>/`，成功后更新 `website/render-state.json`，不调用 shadow。
`--lazy-run` 只比较本地成功渲染的源码 hash，不检查上传状态。
`cargo examples publish` 显式执行一次仓库级 `shadow publish`，只发布已有产物，
不重新渲染，也不修改 render state；上传失败后可直接重试发布，无需重新渲染。
网站的媒体 URL 仍取自 `.shadow/refs/`，因此仅运行 `run` 不会更新网站引用的已发布媒体。
构建页面和 WASM 同样不会改变成功渲染的 hash。提交渲染记录；发布后提交更新的 refs，
不提交静态媒体、生成的页面或模板数据。

根 `Notist.toml` 声明 Book 与 Website 两个 workspace 成员；扩展配置位于
`website/Notist.toml` 的 `[site.extensions.examples]`。Notist 从站点目录
启动 `cargo examples site`，同次构建复用一个进程，以协议 `version: 2` 交换 JSON Lines。
`before_compile` 接收并返回 `documents`；`before_template` 接收最终 `pages` 和前序扩展
修改后的 `data`、贡献的 `files`，返回合并后的 `data` 和 `files`。`files` 是输出相对路径到
`{"text": "..."}` 或 `{"base64": "..."}` 的映射；本扩展保留前序扩展贡献的文件，
并加入 `web/` 中五个组件模块，发布到站点输出根目录，不包含 README 或测试。
每行响应携带相同的 `version` / `event`，
写入后立即 flush，不等待 stdin EOF；日志走 stderr。扩展在构建内保存生成的数据，
保留其他页面、数据和文件，重建 `examples/` 文档与各例子的数据键。`watch` 额外监听 `web/` 等项目输入，
所以修改例子、Cargo 配置或媒体 refs 会在开发服务器中重新生成页面。
协议仍为 Experimental，随 Notist 固定版本更新。

示例页通过 `@!(template: "example", example: "<name>")` 声明模板和数据键。
列表页和样例页左侧导航按生成的目录层级渲染；嵌套 example 的页面带
`Examples / <group> / <name>` 面包屑。
Tera 组件模板在 `website/templates/components/` 中，`player.tera` 输出共享的
`<ranim-viewport>` 与 `<ranim-controls>` Web Components，显式绑定后组合展示。

`just doc-examples` 会为 `ranim-examples` 生成带 `#[wasm_demo_doc]` 实时预览的
rustdoc。

`ranim-examples` 的 library 引用各 example 的 `scenes.rs`，用于生成文档。
它仅依赖共享场景和 `render`，不启用 native `cmd-preview`。
每个 bundled 例子只注册同名 bin target，`main.rs` 按平台选择入口：原生调用 CLI，
WASM 提供空 `main()`，并在同一文件直接导出 `app() -> WasmApp`，两种平台
共用 `mod scenes` 中的完整项目装配。所有未标记 `standalone` 的 example 都遵循这一可移植入口约定，
xtask 按 metadata 排除原生独立示例；`hide` 只控制网站页面展示，不影响构建。

`cargo examples build` 按 `required-features` 对所选 target 分组编译，显式选择
`--no-default-features`，选择同名 `<name>` example 与 `wasm32-unknown-unknown` 平台。只有声明需要的例子才会链接
Typst 等可选依赖。随后分别运行 `wasm-bindgen` 和 `wasm-opt`。产物位于
`website/static/examples/<name>/pkg/ranim_example.js` 和对应的 `_bg.wasm`。
`--out-dir` 改变输出根目录，仍保留 `examples/<name>/pkg/` 层级。
组件源码统一由 `web/` 管理，接口和测试见 [`web/README.md`](../../web/README.md)。
网站构建通过 `files` 发布模块，不往 `website/static/` 复制组件；指定其他
`--out-dir`（如 rustdoc 目录）时，将五个模块复制到输出根目录，保持模块间相对路径。
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
