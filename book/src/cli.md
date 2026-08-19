# Ranim CLI

`ranim-cli` 是 Ranim 的命令行工具，二进制名为 `ranim`。它负责把场景代码构建成
dylib、加载其中通过 `#[scene]` 注册的场景，并围绕场景提供四个子命令：

```text
ranim <command>
├── preview   启动预览 app，watch 场景代码并在变更时自动重建 dylib
├── output    渲染场景声明的所有 #[output(...)]（成片输出）
├── render    用默认输出设置快速渲染一个场景一次（冒烟）
└── inspect   不渲染，纯 CPU 检查场景 / 动画树 / 单帧物件
```

在仓库内可以直接用 cargo 运行；也可以安装到 PATH：

```bash
cargo run -p ranim-cli -- <command> ...
cargo install --path packages/ranim-cli   # 之后可直接使用 ranim <command> ...
```

## 工作方式

每次调用都会先 `cargo build` 目标（lib 或 example，需为 `cdylib`），再加载 dylib
中的 scene inventory。因此命令报错时应先看 cargo 的编译输出——大多数失败是场景
代码本身的编译错误。

## 通用 target 参数

以下参数对所有子命令可用：

```text
-p, --package <PACKAGE>   指定 workspace 中的 package（优先于当前目录推断）
    --lib                 使用 package 的 lib target（与 --example 互斥）
    --example <EXAMPLE>   构建并加载指定的 example target，并自动解析到声明它的 package
    --features <FEATURES> 透传给 cargo build
-- <cargo args>...        其余 cargo 构建参数，例如 `-- --release`
```

- 不显式指定时，CLI 根据当前目录推断 package 并使用其 lib target。
- `-- --release` 只影响场景 dylib 的 profile，CLI 本体的 profile 由外层 cargo 决定。
- 调试迭代一般不需要 release：仓库为 dev profile 开了 `opt-level = 1`、依赖
  `opt-level = 3`，`inspect` 是纯 CPU 查询，渲染也足够快。

## `ranim preview [SCENE]`

启动预览 app，并 watch 场景代码，文件变更时自动重建 dylib 刷新画面。适合编写场景
时的实时调试。

## `ranim output [SCENES...]`

渲染每个选中场景声明的**所有** `#[output(...)]`；不指定场景时渲染全部场景。这是
交付前的最终验证命令。

`#[output(...)]` 可用的属性（默认值：`1920x1080`、60 fps、mp4、
`dir = "./output"`、`save_frames = false`）：

```rust,ignore
#[output(
    width = 1920,            // 像素宽
    height = 1080,           // 像素高
    fps = 60,                // 帧率
    format = "mp4",          // mp4 / webm / mov / gif
    dir = "./output",        // 输出目录（相对路径基于当前工作目录）
    name = "my_video",       // 可选，覆盖 {name}（默认用场景名）
    name_template = "{name}_{width}x{height}_{fps}", // 输出文件主名模板
    save_frames = false,     // 同时保存 PNG 帧序列
)]
```

一个场景可以声明多个 `#[output(...)]`（例如同时输出 mp4 和 gif）。

产物位置（以 `dir = "./output"`、场景名 `hello` 为例）：

```text
output/hello_1920x1080_60.mp4            视频：<dir>/<模板展开的主名>.<ext>
output/hello_1920x1080_60-frames/NNNN.png 帧序列（save_frames = true 时）
output/hello_1920x1080_60/<filename>     TimeMark::Capture 截图
```

场景中通过 `r.insert_time_mark(sec, TimeMark::Capture("x.png".to_string()))`
声明的截图，在主视频渲染完成后统一处理。

`--buffer-count <N>`（默认 2）控制 GPU readback 缓冲数量：越大并行度越高，但占用
更多显存。

需要 GPU 与 ffmpeg；PATH 中找不到 ffmpeg 时 CLI 会尝试在当前目录查找或下载。

## `ranim render <SCENE>`

用固定默认设置（`1920x1080`、60 fps、mp4）把单个场景快速渲染一次，输出到
`./output/<scene>_1920x1080_60.mp4`。

它**不读取**任何 `#[output(...)]` 声明，也**不处理** `TimeMark::Capture`。适合
迭代中只想快速看整体效果的情况；正式验收仍应使用 `ranim output`。

## `ranim inspect`

纯 CPU 检查，不创建 GPU context，可以在无 GPU 的环境运行。所有子命令支持
`--format text|json`（默认 `text`；JSON 输出顶层含 `schema_version`，适合脚本化）。

### `ranim inspect scenes`

```bash
ranim inspect scenes --example hello_ranim
```

不调用场景构造函数，只列出 dylib 中注册的场景及其 `#[output(...)]` 摘要（尺寸、
fps、格式、输出目录、`name_template`、`save_frames`）。适合开工第一步：确认场景
名拼写、场景是否注册成功、输出配置是否符合预期。

### `ranim inspect tree [SCENE]`

```bash
ranim inspect tree hello_ranim --example hello_ranim
```

构建场景并输出层级动画树。每个节点包含：DFS `path`、`kind`
（eval/sequence/stack/lagged/static）、`anim_name`、父局部坐标下的 `range`、
`content_duration_secs`、`rate_func`、`enabled` 与 `children`；iterative 节点
额外包含 `sim_step`（`with_steps(N)` 声明的进度步长 `1/N`，未声明时为默认值）。
当库里只有一个场景时 `[SCENE]` 可省略。

注意 `range` 是**父局部坐标**，不要直接当成全局时间。

### `ranim inspect frame <SCENE> --at <sec>`

```bash
ranim inspect frame hello_ranim --at 1.5 --example hello_ranim --verbose
```

以 120 Hz 逻辑时钟在 `<sec>` 采样一帧，输出该帧的物件列表。每个物件包含
`z_order`（帧内渲染/遮挡顺序）、`id` / `animation_id` / `part`、`kind`
（camera/vitem/mesh）、来源根动画 `source` 和 `data` 摘要（VItem 的点数/颜色/
AABB，Mesh 的点数/三角形数/transform/AABB，Camera 的 pos/facing/up/投影参数）。
`--verbose` 追加完整几何数据（VItem points、Mesh 顶点/索引/颜色/法线）。

用于渲染前定位「某时刻物件不对 / 位置不对 / z-order 不对 / 颜色不对」等问题，
避免直接上 GPU 盲调。已知局限（如实输出，不要误读）：

- `source` 只能回溯到根动画的 `animation_id`，不能定位树内叶子节点；
- `SvgItem` / `TypstText` 等用户层 item 会 extract 成多个 CoreItem（1→N），此时
  `part` 是 extract 后的序号，不是用户层 item 的序号。

## 推荐工作流

```text
inspect scenes   确认场景与输出配置
      │
inspect tree     确认动画组织、时间范围、rate_func / enabled
      │
inspect frame    在关键时刻确认物件、几何、z-order 与颜色（不上 GPU）
      │
render           快速冒烟，看整体效果
      │
output           最终验证：成片、帧序列与 Capture 截图
```

原则：能用便宜的 `inspect` 查清的问题，不要留到昂贵的 GPU 渲染之后才发现。
