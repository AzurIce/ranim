# examples/agents — 纯 Agent 编写的 ranim example 目录约定

本文件是 `examples/agents/` 的**目录级规则**，面向在该目录下创建、修改、补全
example 的 AI agent，也供审阅这些内容的人类维护者参考。

它不是全局 skill：不进入通用 skill 目录，只对 `examples/agents/` 路径下的内容
生效。仓库级 PR 与贡献流程遵守仓库根目录的 `AGENTS.md`。

本目录的方法参考了
`~/.dotfiles/.agents/skills/blender-modeling/SKILL.md` 的核心闭环：**程序化生成 →
渲染出图 → 视觉检查 → 修改再渲染**。区别在于这里的一切都必须围绕
**ranim 场景代码 + ranim-cli** 完成，视觉检查对象是 ranim 渲染出的 PNG/视频。

## 定位

- 本目录专门归档由 agent 以 **one-shot** 方式编写的 ranim example。
- “one-shot”指：**一个原始 prompt 对应一次 agent 任务交付**。agent 在交付前
  为满足 prompt 而进行的自查、构建、`inspect` 查询、渲染查看和修改，都属于
  同一次 one-shot 任务内部的迭代，**不算**新的任务轮次。
- 如果任务后来收到用户新的修改要求，那就不再是纯 one-shot，应先把该 example
  移出本目录，或与维护者确认后再放入。
- 与手工维护的 `examples/<name>/` 相区分：这里的每个 example 额外承担
  **可追溯性记录**职责（原始 prompt、效果图、设计思路、用 ranim-cli 迭代的
  过程、模型与 harness 环境）。
- 本目录不承担 skill 功能：不要把通用建模/动画流程写进这里；通用约定放到
  仓库级文档或 skill 文件。

## 核心工作方法：渲染 → 看图 → 迭代

```text
编写/修改 lib.rs
      │
      ▼
cargo fmt / cargo check（可选，快速失败）
      │
      ▼
ranim inspect scenes / tree / frame   ← 纯 CPU 查询，先查清结构再渲染
      │
      ▼
ranim output（最终验证）或 ranim render（快速冒烟）
      │
      ▼
用视觉能力读取 PNG/视频，逐条对照原始 prompt
      │
      ├─ 不满足 ──► 定位问题（时间轴/几何/z-order/颜色/构图），回到修改
      │
      └─ 满足 ──► 保存效果图，写 README 的迭代记录，收尾提交
```

- **不要假设效果**：每次修改后必须实际跑 CLI 并用视觉能力看图/视频；只有
  `cargo check` 或 `inspect` 通过，不代表视觉效果满足 prompt。
- **先用 `inspect` 再用 render/output**：`inspect` 不创建 GPU context，能快速
  发现场景名、输出配置、时间轴范围、enabled 状态、帧内物件种类/数量/z-order/
  几何摘要等问题；把昂贵的 GPU 渲染留给真正需要看效果的时候。
- 视觉检查使用当前环境可用的读图/读视频能力（如 `ReadMediaFile`）；需要时对
  大图裁局部看原生分辨率，参考 blender skill 的做法。
- 只记录真实执行过的命令和真实观察到的结果，禁止编造迭代过程或验证结论。

## 目录规则

```text
examples/agents/
├── AGENTS.md                 # 本文件
├── README.md                 # 本目录索引：列出所有 agent example 及状态
└── <example-name>/           # 每个 example 一个子目录，snake_case
    ├── lib.rs                # ranim 场景源码（必需，含 #[scene] 入口）
    ├── README.md             # 可追溯性记录（必需，格式见下）
    ├── preview.png           # 至少一张真实渲染的效果图（必需，见下）
    └── ...                   # 其他源码/说明文件按需存放，保持最小化
```

- 每个 example 对应且只对应一个子目录；不要在 example 下面再建多层分类目录。
- 子目录名使用仓库现有 example 风格：`snake_case`（小写 ASCII、数字、下划线），
  尽量让目录名、`lib.rs` 中主 `#[scene]` 函数名保持一致。额外的变体 scene 可用
  `<主名>_<variant>` 命名。
- 新增目录名不得与 `examples/` 下已有 example 重名，也不得与本目录已有 example
  重名。
- 每个 example 目录必须同时包含 `lib.rs`、`README.md` 和至少一张效果图。
  `lib.rs` 遵循仓库现有 example 的写法（`ranim::prelude`、`#[scene]`、
  `#[output(...)]` 等），保持代码完整、可读，不留“待实现”的占位逻辑。
- **为使用 ranim-cli 迭代，example 必须可被 cargo 构建**：在根 `Cargo.toml`
  增加一条 `[[example]]`，`name` 与子目录名一致，`path` 指向
  `examples/agents/<example-name>/lib.rs`，并设置 `crate-type = ["cdylib"]`；
  如依赖 `typst` 等 feature，同时写 `required-features`。这是本目录 agent
  example 允许的、也是必需的对 `examples/agents/` 之外的修改。登记后通过
  `ranim-cli --example <example-name>` 构建与运行。
- 默认输出目录使用 `#[output(dir = "./output/agents/<example-name>")]`，与手工
  example 的输出分开。`output/` 已被仓库忽略，**不要**把生成的 mp4、PNG 序列
  等大体积渲染产物放进 example 子目录或提交到仓库；只提交挑选出的、用于 README
  的效果图。
- 每次新增、删除或明显改动 example 时，同步更新 `examples/agents/README.md`
  索引。索引至少包含：

  | Example | 一句话说明 | 模型 | 生成日期 | 状态 |
  |---|---|---|---|---|

  其中“状态”只允许写实际达到的状态，例如：`未验证`、`已构建`、
  `已渲染并视觉检查`、`待修复`。

## 每个 example 的 README 要求

README 是 example 的一部分，不是可选的说明。它必须随代码在同一次改动中完成，
并且只记录真实信息。使用标准 Markdown 语法（本仓库是普通 git 仓库，不是
Obsidian Vault；效果图用 `![...](...)` 嵌入）。

固定章节：

### 1. 效果图

- 至少嵌入一张**真实由本 example 渲染出的**效果图，放在本 example 目录中
  （推荐 `preview.png`），用相对路径引用：
  `![效果图](preview.png)`。
- 效果图来源必须是 ranim 渲染产物，推荐做法：
  1. 在 scene 中设置
     `r.insert_time_mark(<sec>, TimeMark::Capture("preview.png".to_string()));`
  2. 运行 `ranim output --example <example-name>`；
  3. 从
     `<output.dir>/<scene>_<width>x<height>_<fps>/preview.png`
     复制到 `examples/agents/<example-name>/preview.png`。
  也可以用 `save_frames = true` 的帧序列挑选、或从 mp4 抽帧，但 README 中必须
  说明这张图实际来自哪条命令、哪个产物。
- 效果图要能代表最终效果；如果一张图不足以说明，可放多张并逐一说明视角/时刻。
- 未渲染出图前不得写“效果见下图”；也不得用示意图、AI 生成图片冒充渲染结果。

### 2. 原始 Prompt

- **原样引用**用户/任务给出的 prompt，不改写、不省略、不翻译、不“润色”。
- 多轮任务按“第 1 轮 / 第 2 轮 / …”分别引用（但见“定位”中的 one-shot 限制：
  多轮用户需求原则上不应进入本目录）。
- prompt 若附带代码、图片、参考文件，列出这些附件，并说明在实现中如何使用。
- 若 prompt 含有密钥、个人信息等敏感内容，脱敏后用 `[已脱敏]` 标注，但不得
  借“脱敏”之名改变 prompt 的实际要求。

### 3. 设计与实现思路

- 这个 example 演示什么、场景/动画结构是什么。
- 关键实现路径：使用了哪些 ranim API、数据结构、相机/时序/输出设置。
- 关键取舍：为什么这样组织 scene、为什么选择这些动画或参数、与现有 example
  约定的差异。
- 输出规格：分辨率、时长、输出路径等（以代码中实际声明为准）。

### 4. 迭代过程（ranim-cli 工具使用记录）

这是可追溯性记录中最有价值的部分。逐轮记录：

````markdown
### 第 N 轮

- 命令：
  ```bash
  ranim inspect tree <scene> --example <example-name>
  ranim output <scene> --example <example-name>
  ```
- 观察：场景结构 / inspect 输出 / 渲染图 / 视频中看到什么。
- 问题：与原始 prompt 的差距或构建、渲染报错。
- 修改：改了哪些代码、参数或输出配置。
- 结论：重跑后的结果。
````

- 如果确实是 one-shot 且没有发生任何修正，明确写
  “one-shot，无迭代轮次，最终代码即第 1 版”，并说明经过哪些验证。
- 只记录真实发生过的迭代；不得为“看起来完整”而编造试错过程。
- 迭代记录应体现对 ranim-cli 各工具的自主使用：`inspect scenes` 确认场景与
  输出、`inspect tree` 检查动画树与时间轴、`inspect frame` 检查具体时刻的物件
  与几何、`render`/`output` 做视觉验证。

### 5. 验证情况（没有就写没有）

- 若实际执行过构建、渲染或视觉检查，记录：执行命令、输出产物位置、检查结果、
  发现的问题及是否已修复。
- 若未执行，明确写“未构建 / 未渲染 / 未视觉验证”，并把索引中的状态标为
  `未验证`。**不得**写“效果正常”“编译通过”等未经执行的结论。

### 6. 模型与 Harness 环境

至少记录以下字段，未知的字段写 `未记录` 或 `未知`，**禁止猜测**：

| 项 | 值 |
|---|---|
| 生成日期 | `yyyy-mm-dd` |
| 生成方式 | `one-shot（无迭代）` / `one-shot（内部 N 轮视觉迭代）` |
| 模型 | 模型名与可确认的版本；无法确认时写 `未记录` |
| Harness / Agent 环境 | agent 框架、CLI 或运行环境及版本；无法确认时写 `未记录` |
| 关键参数 | 如 temperature 等；未提供时写 `未记录` |
| 仓库版本 | 当前 commit 或版本；未记录时写 `未记录` |

若模型名只能知道厂商而不知道具体版本，按实际可确认的信息填写，并标注“具体版本
未确认”，不要虚构精确版本号。

## ranim-cli 工具指南

agent 在本目录工作时，统一通过 ranim-cli 完成“查询 → 渲染 → 看图 → 修改”的
闭环。命令示例中的 `<example-name>` 与 `<scene>` 是 example 目录名与
`#[scene]` 场景名。

### 通用 target 参数

以下参数对所有子命令可用：

```bash
ranim <command> [-p <package>] [--lib | --example <example-name>] [--features <features>] [-- <cargo args>...]
```

- `--example <example-name>`：构建并加载名为 `<example-name>` 的 example
  target，并自动解析到声明该 example 的 package。agent example 登记进根
  `Cargo.toml` 后，用它迭代：
  `ranim inspect scenes --example <example-name>`。
- `-p/--package <package>`：指定 workspace 中的 package，优先于当前目录推断。
- `--lib`：使用当前 package 的 lib target；与 `--example` 互斥。
- `--features`：透传给 `cargo build`。
- `--` 之后是额外的 cargo 构建参数，例如 `ranim output <scene> --example <name> -- --release`。

ranim-cli 每次都会先 `cargo build` 目标 dylib，再加载其中的 `#[scene]`
inventory；因此命令报错时先看 cargo 编译输出。

### `ranim inspect scenes`

```bash
ranim inspect scenes [--format text|json] [--example <example-name>]
```

- 不调用 scene constructor，只列出 dylib 中注册的 scene 及其 `#[output(...)]`
  摘要（尺寸、fps、格式、输出目录、`name_template`、`save_frames`）。
- 用于开工第一步：确认场景名拼写、scene 是否注册成功、输出配置是否符合预期。
- JSON 输出的顶层字段为 `schema_version` 与 `scenes`，适合脚本化检查。

### `ranim inspect tree [<scene>]`

```bash
ranim inspect tree [<scene>] [--format text|json] [--example <example-name>]
```

- 构建 scene 并输出层级动画树；不创建 GPU context，可在无 GPU 环境运行。
- 每个节点包含：DFS `path`、`kind`（eval/sequence/stack/lagged/static）、
  `anim_name`、父局部坐标下的 `range`、`content_duration_secs`、`rate_func`、
  `enabled` 和 `children`。
- 用于检查动画组织是否符合设计：顺序/叠加关系、每段起止时间、是否误用了默认
  时长、某段是否被 `with_enabled(false)` 关闭。注意 `range` 是父局部坐标，
  不要直接当成全局时间。
- `<scene>` 省略时，只有一个 scene 会自动选择；多个 scene 会报错并列出可用
  名称。

### `ranim inspect frame <scene> --at <sec>`

```bash
ranim inspect frame <scene> --at <sec> [--format text|json] [--verbose] [--example <example-name>]
```

- 用 120 Hz 逻辑时钟在 `<sec>` 采样一帧，输出 `EvaluatedFrame` 中的物件列表。
- 每个物件包含：`z_order`（帧内渲染/遮挡顺序）、`id`/`animation_id`/`part`、
  `kind`（camera/vitem/mesh）、来源根动画 `source` 和 `data`。
- `data` 摘要：VItem 的点数/子路径/闭合/颜色/线宽/法线/AABB，Mesh 的点数/
  三角形数/transform/AABB，Camera 的 pos/facing/up/投影参数。
- `--verbose` 追加完整几何数据（如 VItem points、Mesh 顶点/索引/颜色/法线）。
- 用于渲染前定位“某时刻物件不对/位置不对/z-order 不对/颜色不对”等问题，避免
  直接上 GPU 盲调。
- 已知局限（v1 如实输出，不要误读）：
  - `source` 只能回溯到根动画的 `animation_id`，不能定位树内叶子节点；
  - `SvgItem`/`TypstText` 等用户层 item 会 extract 成多个 CoreItem，存在 1→N
    映射；此时 `part` 是 extract 后的序号，不是用户层 item 的序号。

### `ranim output [<scenes>...]`

```bash
ranim output [<scene1> <scene2> ...] [--example <example-name>]
```

- 渲染每个选中 scene 声明的**所有** `#[output(...)]`；不指定 scene 时渲染
  全部 scene。
- 按声明输出视频；`save_frames = true` 时还会保存 PNG 帧序列；渲染完主视频后
  会处理 `TimeMark::Capture`，输出截图。
- 产物位置（以 `#[output(dir = "./output/agents/<name>")]` 为例）：
  - 视频：`<dir>/<basename>.<ext>`，其中 `<basename>` 由 `name_template`
    展开，默认模板为 `{name}_{width}x{height}_{fps}`；
  - 帧序列：`<dir>/<scene>_<width>x<height>_<fps>-frames/NNNN.png`；
  - capture：`<dir>/<scene>_<width>x<height>_<fps>/<filename>`。
- 这是 one-shot 交付前的最终验证命令：用它确认最终输出配置、成片与效果图。
- 需要 GPU 与 ffmpeg；缺 ffmpeg 时 CLI 会尝试在当前目录查找或下载。

### `ranim render <scene>`

```bash
ranim render <scene> [--example <example-name>]
```

- 快速渲染一个 scene **一次**，使用默认输出 `1920x1080`、60 fps、mp4，
  输出到 `./output/<scene>_1920x1080_60.mp4`。
- **不读取**任何 `#[output(...)]` 声明，也**不处理** `TimeMark::Capture`。
- 用于迭代中只想快速看整体效果、不想等待/写盘多输出配置的情况；正式验收仍应
  使用 `ranim output`。

### 推荐的验证顺序

1. `cargo fmt -p ranim`，并检查 `git diff` 只包含预期改动。
2. `cargo check -p ranim --example <example-name>`（若 example 声明了
   `required-features`，同时加上 `--features <features>`；也可直接交给下一步的
   ranim-cli 构建）排查编译错误。
3. `ranim inspect scenes --example <example-name>`：确认 scene 与 output。
4. `ranim inspect tree --example <example-name>` 或指定 scene：确认动画树、
   时间范围与 rate_func/enabled。
5. 对关键时间点（起幅、中段、收幅、capture 点）跑
   `ranim inspect frame <scene> --at <sec> [--verbose]`：确认物件、几何、
   z-order 与颜色。
6. `ranim render <scene>` 快速冒烟；发现问题回到第 1 步。
7. `ranim output --example <example-name>` 做最终验证，用视觉能力读取成片与
   capture 图，对照 prompt。
8. 保存效果图、写 README（含真实迭代记录）、更新
   `examples/agents/README.md` 索引。

## 写入纪律

- README 中的 prompt、效果图来源、模型、harness、参数、commit、运行结果都必须
  来自真实输入或真实执行；没有的信息写“未知/未记录”。
- 不要为了显得完整而补写不存在的设计动机、迭代轮次或验证结果。
- 代码中的输出路径、场景时长、渲染设置等必须与 README 描述一致。
- 只提交挑选出的效果图；不提交 `output/` 下的大体积渲染产物。
- 未在本文件中出现的 CLI 能力（如尚未实现的 agent 子命令、自动登记流程）不得
  被假设，也不得写进代码或 README。
