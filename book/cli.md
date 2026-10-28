# 命令行入口

每个 Ranim 项目是自己的二进制 crate：一个 bevy App（`ranim::App` 就是
`bevy_app::App` 的再导出）。`main` 装配 `App` 后用 `app.build()`
（`RanimAppExt`，prelude 导出）定稿插件构建——断言插件就绪、执行一次
bevy 的 `finish()` / `cleanup()`——得到 `ranim::RanimApp`，把它交给自由
函数 `run_cli`（prelude 导出，native + `cmd-render` feature 门控；命名刻意避开
bevy App 自带的 `run()` runner 入口）：它安装 tracing、解析进程参数，
随即分发为四个子命令：

```rust,ignore
use ranim::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_scenes([hello::scene()]);
    run_cli(app.build());
}
```

```text
<project> <command>
├── preview   启动交互预览 app（需要 cmd-preview feature）
├── output    渲染场景声明的所有 #[output(...)]（成片输出）
├── render    用默认输出设置快速渲染一个场景一次（冒烟）
└── inspect   不渲染，纯 CPU 检查场景 / 动画树 / 单帧物件
```

在自己的项目中以 `cargo run -- <command>` 调用；在本仓库中运行示例：

```bash
cargo run --example <example> -- <command> ...
```

默认包含 `cmd-preview`，开启全部四类子命令；关闭 default 并只开启 `cmd-render`
时，CLI 只提供 `output`、`render` 和 `inspect`，帮助中不显示 preview。

带 `required-features`（如 `typst`、`gltf`）的示例需同时开启对应 feature。
仓库为 dev profile 开了 `opt-level = 1`、依赖 `opt-level = 3`，`inspect` 是纯
CPU 查询，渲染也足够快，调试迭代一般不需要 release。

## 项目装配

子命令只能看到 App 中显式注册的场景；在 `main` 或插件的 `build` 中通过
`add_scene` / `add_scenes` 注册（场景方法由 `AppExt` 提供；插件 `build` 的
`app: &mut bevy_app::App` 与 `&mut App` 是同一类型；完整示例见
[Getting Started](./getting_started.md)）：

```rust,ignore
fn main() {
    let mut app = App::new();
    app.add_scenes([hello::scene()]);
    run_cli(app.build());
}
```

渲染装配是可选的：启用 `items` 的项目不注册 `RenderSetup` 时，渲染与预览使用默认配置，
即注册内置相机与矢量/网格物件渲染（`ranim_items::RanimItemsPlugin`）。关闭 `items`
时默认配置为空，自定义渲染项目应注册自己的 `RenderSetup`。需要
渲染默认配置未覆盖的完整输出（容器、变换 wrapper、自定义类型）时，才在插件
中提供自己的 `RenderSetup`：

```rust,ignore
pub struct ProjectPlugin;

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(ScenesPlugin);
        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((
                RanimItemsPlugin,
                ExtractPlugin::<MyOutput>::default(), // 实际完整输出类型
            ));
        }));
    }
}
```

项目的 `RenderSetup` 在每次安装渲染 SubApp 时重放，安装提取与渲染插件。
`inspect` 只查询 CPU 数据，不执行 setup 中的 GPU 装配。

命令报错时先看编译输出；运行成功但场景未出现时，检查 `main` 或插件的
`build` 是否显式调用了 `add_scene` / `add_scenes`。

## `preview [SCENE]`

启动交互预览 app。需要 `cmd-preview` feature（它启用 `cmd-render`，后者启用 `render`）；不传场景名时
预览第一个场景。适合编写场景时的实时调试。

## `output [SCENES...]`

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

需要 GPU 与 ffmpeg；PATH 中找不到 ffmpeg 时会尝试在当前目录查找或下载。

## `render <SCENE>`

用固定默认设置（`1920x1080`、60 fps、mp4）把单个场景快速渲染一次，输出到
`./output/<scene>_1920x1080_60.mp4`。

它**不读取**任何 `#[output(...)]` 声明，也**不处理** `TimeMark::Capture`。适合
迭代中只想快速看整体效果的情况；正式验收仍应使用 `output`。

## `inspect`

纯 CPU 检查，不创建 GPU context，可以在无 GPU 的环境运行。所有子命令支持
`--format text|json`（默认 `text`；JSON 输出顶层含 `schema_version`，适合脚本化）。

### `inspect scenes`

```bash
cargo run --example hello_ranim --no-default-features --features anims,cmd-render -- inspect scenes
```

列出 App 中注册的场景及其 `#[output(...)]` 摘要（尺寸、fps、格式、输出目录、
`name_template`、`save_frames`），不调用场景构造函数。
适合开工第一步：确认场景名拼写、场景是否注册成功、输出配置是否符合预期。

### `inspect tree [SCENE]`

```bash
cargo run --example hello_ranim --no-default-features --features anims,cmd-render -- inspect tree hello_ranim
```

构建场景并输出层级动画树。每个节点包含：DFS `path`、`kind`
（anim/children/audio）、`anim_name`、父局部坐标下的 `range`、
`content_duration_secs`、`rate_func`、`enabled` 与 `children`；iterative 节点
额外包含 `sim_step`（`with_steps(N)` 声明的进度步长 `1/N`，未声明时为默认值）。
当项目里只有一个场景时 `[SCENE]` 可省略。

注意 `range` 是**父局部坐标**，不要直接当成全局时间。

### `inspect frame <SCENE> --at <sec>`

```bash
cargo run --example hello_ranim --no-default-features --features anims,cmd-render -- inspect frame hello_ranim --at 1.5 --verbose
```

在 `<sec>` 直接采样类型化帧；迭代动画使用各自声明的内容步长。JSON frame
输出的 `schema_version` 为 `2`。每个输入包含 `order`（完整输出的全局插入
顺序）、`id` / `animation_id` / `part`、`input_type` 与来源根动画 `source`。
已支持的裸内置类型还提供 CPU 投影数据：矢量、网格、相机，或 SVG / 文字的
图元分组；`--verbose` 追加完整几何数据。未提供检查适配的复合或自定义类型
保留为 opaque 输入元数据，不需要 GPU 或该类型的渲染插件。

用于渲染前定位「某时刻物件不对 / 位置不对 / z-order 不对 / 颜色不对」等问题，
避免直接上 GPU 盲调。已知局限（如实输出，不要误读）：

- `source` 只能回溯到根动画的 `animation_id`，不能定位树内叶子节点；
- `part` 是根动画中完整求值输出的序号，跨活跃叶子与类型连续递增；容器成员
  不单独占用输入序号。一个 `SvgItem` 或 `TypstText` 是一条输入，其 CPU 投影
  数据可以包含多个图元；
- `order` 是输入顺序。渲染器在提取后还会按 `SourceOrder { order, part }` 排序
  生成最终 `SceneOrder`，不能把输入序号直接当作 render entity 数量或最终 rank。

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
