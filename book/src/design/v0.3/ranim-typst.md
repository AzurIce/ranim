# ranim-typst 项目编译与资源系统

> 状态：构思中。本页记录 `ranim-typst` 从单源码编译器扩展为 Typst 项目编译器的目标，不代表最终 API。

## 背景

`ranim-typst` 已经建立了原生 Typst 编译到 Ranim 矢量路径的基本链路：

```text
Typst source
    │
    ▼
typst::compile<PagedDocument>
    │
    ▼
Page / Frame / Text / Shape collector
    │
    ▼
TypstPath
    │
    ▼
ranim-items::VItem / TypstText
```

当前实现适合公式、文字和基础矢量图形，但它提供的 `World` 只认识一段名为 `main.typ` 的源码。其他 source 和所有二进制文件都会返回 `NotFound`，因此相对 import、多文件项目、图片和 Typst Universe 包还不能使用。

v0.3 的目标是保留现有单字符串便捷接口，同时建立完整、可配置的项目文件与包加载边界。

## 当前能力

当前编译层支持：

- 进程内运行 Typst 0.15，不依赖外部 Typst CLI；
- 编译单字符串源码并保留 Typst 的分页结果；
- 使用 Typst 内置字体和首次调用时扫描到的系统字体；
- 提取普通文字轮廓、基础图形、纯色填充和描边；
- 应用 Typst 的平移、旋转和缩放变换；
- 将 Typst label 映射到其后代路径索引；
- 记录 glyph cluster 与路径之间的映射，供 `TypstText` morph 使用；
- 缓存最近 256 个 `(source, CompileOptions)` 编译结果。

当前转换层会对不能准确表示的内容产生 `TypstWarning`：

| Warning | 当前行为 |
| --- | --- |
| `ImageUnsupported` | 跳过 bitmap、SVG 等 image frame item |
| `ColorGlyphUnsupported` | 跳过 bitmap、SVG 或 layered color font glyph |
| `ClipPathUnsupported` | 保留内容但不执行裁剪 |
| `GradientUnsupported` | 使用不透明白色代替渐变 |
| `TilingUnsupported` | 使用不透明白色代替 tiling paint |
| `EvenOddFillUnsupported` | 输出路径，但无法保留 even-odd fill rule |

link 和 tag 当前也不会转换为 Ranim 数据。

## 当前数据结构语义

### CompileOptions

当前的 `CompileOptions` 实际只控制 Typst 布局结果如何转换为 Ranim 路径，不控制 Typst 项目环境：

| 字段 | 默认值 | 语义 |
| --- | --- | --- |
| `include_page_fill` | `false` | 将页面背景作为每页第一个路径；背景范围包含 bleed，但 `page.size` 仍是 frame 大小 |
| `center_content` | `true` | 按路径控制点包围盒，将每页内容中心移动到 Ranim 原点；不改变 `page.size` |

因此后续应将其更名或拆分为 `ConversionOptions`，避免与项目根目录、包策略、字体配置等真正的编译环境选项混在一起。

### CompileOutput

- `document`：已经转换的分页文档；
- `compiler_warnings`：Typst 编译器警告，目前只保留 message 字符串；
- `conversion_warnings`：转换阶段发生过的能力降级，在整个文档范围内去重，不带页码和位置。

当前编译错误同样只拼接 diagnostic message，会丢失文件路径、source span、hint 和 trace。项目化以后应提供结构化 diagnostic，同时可以保留面向 `Display` 的简化文本。

### TypstDocument 与 TypstPage

- `TypstDocument::pages` 按 Typst 页面顺序排列；
- `into_paths()` 和 `into_vitems()` 直接展平页面，不会自动为各页增加位移；
- `TypstPage::size` 使用 Typst pt；
- `paths` 或 `vitems` 按绘制顺序排列；
- `groups` 将 label 映射到本页路径索引，嵌套 label 可以同时包含同一路径；
- `glyphs` 保存遍历文字布局项时得到的 glyph cluster 信息。

### TypstPath 与 TypstStroke

- `points` 使用 Ranim 的二次 Bezier 路径编码；
- 坐标单位为 Typst pt；
- collector 会翻转 Y 轴，使坐标方向与 Ranim 一致；
- `fill: None` 表示无填充；
- `stroke: None` 表示无描边；
- `TypstStroke::width` 是应用变换后的 pt 宽度。

非均匀缩放下，当前描边宽度使用 X/Y 缩放量的近似平均值。`center_content` 的包围盒只考虑路径控制点，不考虑描边宽度，也不是曲线极值的严格几何边界。

### GlyphInfo

| 字段 | 语义 |
| --- | --- |
| `item_index` | glyph 对应的本页路径索引；空格、无轮廓 glyph 和不支持的彩色 glyph 为 `None` |
| `text` | shaped glyph cluster 对应的文本，可能是一个字符，也可能是 `ffi` 之类的多个字符 |
| `text_range` | 该 cluster 在所属 `TextItem.text` 中的 UTF-8 字节范围，不是原始 `.typ` 文件位置或整页全局范围 |

`TypstText` 将 glyph 的 `text` 作为 morph key，通过 LCS 尽量匹配两端相同的 glyph cluster。普通 shape 使用内部占位 key，因此 `GlyphInfo` 目前不是完整的 source mapping 或语义对象模型。

## v0.3 目标

### 本地项目与相对 import

支持以明确的项目根和主文件编译 Typst 项目：

```typst
#import "theme.typ": accent
#import "components/card.typ": card
```

Typst 本身负责解析 import 并生成 `FileId`。`ranim-typst` 不应自行解析 import 语法，而应在 `World::source` 和 `World::file` 中正确实现 Typst 的虚拟文件系统协议。

项目实现应：

1. 将 main path 转换成相对于 project root 的 `VirtualPath`；
2. 使用 `VirtualRoot::Project` 创建主 `FileId`；
3. 通过 `typst_kit::files::FileStore` 缓存 source 和原始 bytes；
4. 让 `World::source` 委托给 file store；
5. 让 `World::file` 同样委托给 file store，使图片、SVG、CSV、JSON 和其他资源可以被 Typst 读取；
6. 禁止虚拟路径越过项目根，并明确处理 symlink 可能绕过词法路径限制的问题。

### 内存多文件

除文件系统项目外，还应支持不落盘的虚拟项目，便于动画代码、测试和其他工具动态生成 Typst 模块：

```rust,ignore
let output = compile_virtual(
    "main.typ",
    [
        ("main.typ", "#import \"theme.typ\": accent\n#accent"),
        ("theme.typ", "#let accent = red"),
    ],
    options,
)?;
```

虚拟项目应同时允许 source 和 raw bytes，保证 `read()`、image 和数据文件的语义与文件系统项目一致。

### Typst Universe 包

Typst Universe 支持应基于 `typst-kit` 提供的标准组件：

```text
World
  └─ FileStore<SystemFiles>
       ├─ FsRoot(project root)
       └─ SystemPackages
            ├─ system package data directory
            ├─ system package cache directory
            └─ UniversePackages + SystemDownloader
```

当源码使用：

```typst
#import "@preview/cetz:0.4.2"
```

Typst 生成的文件 ID 会带有 `VirtualRoot::Package(spec)`。`SystemFiles` 应先检查本地 package data 和 cache，在策略允许时再从 `https://packages.typst.org` 或指定镜像下载并解包到缓存。

联网和文件写入不能成为不可见的副作用。建议提供显式包策略：

```rust,ignore
pub enum PackagePolicy {
    Offline,
    DownloadMissing,
    CustomMirror(String),
}
```

需要进一步决定默认策略。库接口默认离线更可控；CLI 或编辑器可以显式启用缺失包下载。

## 拟议 API

保留当前接口作为轻量便捷入口：

```rust,ignore
pub fn compile(source: &str) -> Result<CompileOutput, TypstError>;

pub fn compile_with_options(
    source: &str,
    options: ConversionOptions,
) -> Result<CompileOutput, TypstError>;
```

增加项目级接口：

```rust,ignore
pub struct TypstProject {
    pub root: PathBuf,
    pub main: PathBuf,
}

pub struct ProjectCompileOptions {
    pub conversion: ConversionOptions,
    pub package_policy: PackagePolicy,
    pub package_data_dir: Option<PathBuf>,
    pub package_cache_dir: Option<PathBuf>,
    pub universe_url: Option<String>,
}

pub fn compile_file(
    main: impl AsRef<Path>,
    options: ProjectCompileOptions,
) -> Result<CompileOutput, TypstError>;

pub fn compile_project(
    project: &TypstProject,
    options: ProjectCompileOptions,
) -> Result<CompileOutput, TypstError>;
```

具体 API 还需决定 `TypstProject` 是一次性描述，还是持有可复用 `FileStore`、字体和增量编译状态的长生命周期 session。编辑器和反复渲染更适合后者。

## 缓存与可复现性

当前 LRU key 只有 source 字符串和转换选项。加入项目文件后，如果继续使用这一 key，依赖文件变化将错误地命中旧结果。

文件项目至少需要选择一种策略：

- 初期关闭外层结果 LRU，以正确性优先；
- 将所有实际访问的依赖内容或稳定 fingerprint 纳入 key；
- 使用可复用 `FileStore` 和 Typst 增量能力，在每轮编译前 reset 并重新检查依赖。

包版本、包源、字体集合、编译输入和日期也会影响结果。可复现构建应尽量要求显式 package version，并记录实际加载的依赖。`today()` 与当前结果缓存之间也需要重新定义：相同源码不应无限期返回旧日期结果。

系统字体使用全局 `OnceLock`，因此进程运行期间新增字体不会被发现。后续需要决定字体集合是进程级固定环境，还是允许项目/session 提供额外字体目录并参与缓存 fingerprint。

## 实施阶段

### 阶段一：本地文件项目

- 引入基于 project root 的 `FileId`；
- 使用 `FileStore` 加载 source 和 raw bytes；
- 支持相对 import、include、read 和本地图片；
- 增加越界路径、安全性和依赖变更测试；
- 文件项目暂不使用当前 source-only LRU。

### 阶段二：虚拟项目与诊断

- 增加内存多文件和 raw asset API；
- 保留 diagnostic 的文件、span、hint 和 trace；
- 统一文件系统项目与虚拟项目的 World 实现边界；
- 让错误信息可以映射回具体项目文件。

### 阶段三：本地包与 Universe

- 启用 `typst-kit` 的 `system-files` 能力；
- 支持标准 package data/cache 目录；
- 将 downloader 作为可选 feature 或策略注入；
- 支持官方 Universe 和自定义镜像；
- 测试离线缓存、首次下载、404、损坏 archive 和并发下载。

### 阶段四：转换完整性

- 评估 image/SVG 到 Ranim item 的表示方式；
- 实现或明确 clip path、gradient、tiling 和 even-odd fill 的降级边界；
- 为 warning 添加页码、对象位置或其他上下文；
- 评估更完整的 source-to-path 和 glyph mapping。

## 验收标准

- `#import "relative.typ"` 能从主文件所在项目中加载；
- 嵌套相对 import 按各自文件路径正确解析；
- `read()`、JSON/CSV 和本地图片能够通过 `World::file` 加载；
- 离线模式不会发起网络请求或写入下载缓存；
- 允许下载时，`@preview` 包可以从 Universe 获取并在后续离线编译中复用；
- 修改任一已访问依赖后不会返回旧的缓存结果；
- diagnostic 能指出出错的文件和位置；
- 当前单字符串 `compile`、`compile_vitems` 和 `TypstText` 用法保持兼容；
- 所有不完整转换都有明确 warning，不静默生成明显错误的视觉结果。

## 待决问题

- `CompileOptions` 是直接更名为 `ConversionOptions`，还是保留别名完成渐进迁移？
- `compile_file` 是否自动以 main 文件父目录作为 root，还是必须显式传 root？
- 网络下载是否作为默认关闭的 Cargo feature，以避免 TLS 和下载依赖进入基础构建？
- package cache 和项目依赖是否需要 lockfile 或 manifest，以加强可复现性？
- session 是否暴露依赖列表、实际包版本和字体信息给编辑器？
- package 或项目携带的字体如何加入 `FontBook`，以及它们是否允许影响其他项目？
- 图片应转换成新的 Ranim image item，还是在 `ranim-typst` 中栅格化/矢量化？
- 多页文档展平时是否应提供标准页面排列策略，而不是继续直接重叠？
