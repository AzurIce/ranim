# 完整求值帧与渲染提取

动画叶子的 `Eval::Output` 原样进入 `EvaluatedFrame`，渲染侧再决定如何把它
转换为实体、组件或资源。`Vec<T>`、元组、`Node` 和 `Transformed<T, G>` 等
结构都会保留在输出值中。纯 CPU 求值不要求输出实现渲染 trait，也不依赖
renderer 或逻辑 ECS World。

## `EvaluatedFrame`：保存完整输出

作为动画叶子使用的 `Eval` 需要其 `Output: Clone + Send + Sync + 'static`。
普通采样将拥有所有权的完整输出直接写入帧；`hold` 和 lagged 静态填充则捕获
同样的原始值，播放时克隆并重放，保留原来的具体类型。

`ranim_core::frame::EvaluatedFrame` 按具体类型保存可复用批次。类型擦除发生在
整个批次上，同类型的输出连续保存。`SceneEvaluator::sample_at(sec, &mut frame)`
先清空上一帧，再写入当前结果；`clear` 保留各批次容量。

```rust,ignore
use ranim_core::frame::EvaluatedFrame;
use ranim_items::vitem::geometry::Circle;

let mut frame = EvaluatedFrame::new();
evaluator.sample_at(1.0, &mut frame);

// 若动画的 Output 是 Vec<Circle>，整个 Vec 是一条帧输入。
let groups = frame.items::<Vec<Circle>>();
// frame.items::<Circle>() 不会穿过 Vec 读取其成员。
```

每个 `FrameItem<T>` 包含：

- `id: (animation_id, part)`：根 Stack 的顶层动画序号，以及该动画本次求值的
  完整输出序号。`part` 跨活跃叶子与类型连续递增，不是容器成员或 render entity
  的序号。
- `order: usize`：所有类型共享的全局插入顺序。
- `value: T`：完整的求值结果，包括其容器和变换 wrapper。

`push(id, value)` 追加输入；`update(iterator)` 替换整帧为一个类型的批次。
`frame.len()` 只表示完整输出条目数：一条 `Vec<Circle>` 或 `SvgItem` 输入可以
产生多个渲染实体，提取也可以只更新资源。帧条目、实体、GPU instance 和 draw
call 的数量互不等同。

## `Extract`：在渲染侧解释一个值

渲染协议定义在 `ranim_render::extract`：

```rust,ignore
pub trait Extract {
    fn extract(&self, ctx: &mut ExtractContext<'_>);
}
```

实现借用原始值，通过上下文更新 render world；不返回统一的 `Output` 或图元
列表。一个物件可以委托给已有表示的 `Extract`，也可以直接写入自己的组件或
资源。例如自定义标记复用圆的提取实现：

```rust,ignore
use ranim_items::vitem::geometry::Circle;
use ranim_render::extract::{Extract, ExtractContext};

#[derive(Clone)]
struct Marker {
    radius: f64,
}

impl Extract for Marker {
    fn extract(&self, ctx: &mut ExtractContext<'_>) {
        Circle::new(self.radius).extract(ctx);
    }
}
```

`ranim-items` 的 `render` feature 提供内置物件的这类实现，顶层 `ranim` 的
`render` feature 会启用它。renderer 为 `Vec<T>`、数组、`Option<T>`、1..=15
元组及 `Transformed<T, G>` 提供递归实现，`Node` 则由 items 实现。递归发生在
render world 的提取阶段，成员通过同一个上下文共享输出顺序与身份。

上下文提供四个主要入口：

- `ctx.entity::<Bundle>()`：以输出 bundle 类型领取该输入的下一个持久实体。
  相同输入、位置与 bundle 类型复用实体；同一位置换成另一种 bundle 时重建
  实体，避免残留旧形态的组件。未再领取的输出自动删除。同一个 bundle 形态
  内自行增删的附加组件仍由实现管理。
- `ctx.world_mut()`：访问 render world，以更新资源或进行自定义操作。直接
  创建的实体不由上下文追踪，需要实现自己管理生命周期。
- `ctx.transform()`：读取当前累计的局部到世界变换。
- `ctx.with_transform(transform, |ctx| ...)`：在 `current * transform` 下提取
  子树，退出作用域后恢复父上下文。

`Transformed<T, G>` 在 `T: Extract`、`G: Clone + Into<DAffine3>` 时递归提取
`inner`。items 的提取实现把累计变换左乘到 `RenderVItem` / `RenderMeshItem` 的矩阵，
相机则变换位置与方向。局部几何和帧中的 wrapper 保持不变，重复绘制同一帧
不会累积变换。详见 [Transformed](./transformed.md)。

## `ExtractPlugin<T>`：注册精确的帧类型

实现 `Extract` 使一个值能够被解释；`ExtractPlugin<T>` 把解释入口注册到
渲染 SubApp 的提取阶段（`RenderSchedule::Extract`）。`T` 必须是帧实际保存的完整类型：

```rust,ignore
use ranim::{App, Plugin};
use ranim_items::{RanimItemsPlugin, vitem::geometry::Circle};
use ranim_render::{RenderSetup, extract::ExtractPlugin};

pub struct ProjectPlugin;

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScenesPlugin);
        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((
                RanimItemsPlugin,
                ExtractPlugin::<Vec<Circle>>::default(),
                ExtractPlugin::<Marker>::default(),
            ));
        }));
    }
}
```

`ExtractPlugin<Vec<Circle>>` 读取 `frame.items::<Vec<Circle>>()`，随后调用容器和
成员的值级 `Extract`。只安装 `ExtractPlugin<Circle>` 不会读取 `Vec<Circle>`；
`Transformed<Circle, Translation>`、元组和树等完整输出类型也要分别显式注册。
递归委托不要求为每个内部成员另外安装插件。

三个物件插件来自 `ranim-items`：`CameraFramePlugin` 注册裸相机输入及 viewport
准备，`VItemPlugin` 注册裸 `VItem`、`Vec<VItem>`、几何、SVG 与启用 `typst` 后的
文字输入；`MeshItemPlugin` 注册裸 `MeshItem`、`Surface` 与 `Sphere`。它们同时安装
对应 GPU 缓冲、管线和渲染系统，并由 `RanimItemsPlugin` 捆绑为一——项目未注册
`RenderSetup` 时的默认配置就是它；`RanimItemsPlugin` 与三个单件插件互斥
（Bevy 插件唯一，重复安装会 panic）。`RenderVItem` / `RenderMeshItem` 也归各自
items 的 `render` 模块，renderer 本体不依赖这些具体组件。它们是 render world
的数据组件，不实现动画或值级提取协议，普通动画输出使用逻辑物件。

family plugin 不枚举容器或 wrapper 组合。项目插件可保存一份
`RenderSetup`，统一声明整个项目需要的渲染插件；场景插件只注册场景，避免
组合多个场景时重复安装同一 Bevy plugin。`RenderSetup::new` 保存可重复执行
的装配回调，构造 setup 不创建 GPU 对象。

output 与 preview 从项目 App 读取这份 resource（缺省时回退到默认配置），克隆后
交给渲染 SubApp 的安装函数。低层宿主也可自行装配：

```rust,ignore
use ranim_render::{RenderSetup, install_render_app};

let setup = app.world().resource::<RenderSetup>().clone();
install_render_app(&mut app, &ctx, width, height, oit_layers, setup);
```

`RenderSetup` 是 bevy `Resource`，也是 `is_unique = false` 的普通 bevy plugin；
每次安装渲染 SubApp（输出、预览、分辨率重建）时重放回调，安装新的 systems、
组件和 GPU 资源。`install_render_app` 把构建好的 SubApp 以 `RanimRenderApp`
标签挂进项目 App，并挂 extract 钩子：每次 `App::update()` 主 schedule 结束后，
主世界的 `EvaluatedFrame` 被交换进渲染世界。只需要裸渲染图时也可以直接用
`build_render_app(ctx, width, height, oit_layers)` 创建独立的 `SubApp`。

纯 CPU 求值与 `inspect` 不执行这份渲染装配；未知检查类型仍保留输入元数据。

## 公共 schedule 与 phase

渲染 SubApp 的公共阶段是 `ranim_render::schedule` 中 pub 的枚举：
`RenderSchedule::{Extract, Prepare, View}` 是 schedule label，
`ViewPhase::{Clear, Compute, Depth, Color, Resolve}` 是 `View` schedule 内
按序排列的 system set。手写插件直接引用这些值注册系统：

```rust,ignore
use bevy_ecs::schedule::IntoScheduleConfigs;
use ranim_render::schedule::{RenderSchedule, ViewPhase};

render_app.add_systems(RenderSchedule::Extract, my_extract_system);
render_app.add_systems(RenderSchedule::Prepare, my_prepare_system);
render_app.add_systems(
    RenderSchedule::View,
    my_color_pass.in_set(ViewPhase::Color),
);
```

`ExtractPlugin<T>` 同样注册到 `RenderSchedule::Extract`。枚举值即阶段身份：
所有插件的系统进入同一阶段并遵循同一顺序（`View` 内五个 phase 已 `chain`），
不需要任何句柄 resource。只在插件内部使用的 schedule label 或 system set 仍可
照常定义，通过公共阶段建立所需依赖。

独立 CPU 测试不经过 `install_render_app` 时，由测试宿主自行
`init_schedule(RenderSchedule::Extract)` 等初始化所需阶段；读取和注册都不创建
GPU。

## 持久输出、变更检测与顺序

泛型提取插件按输入类型分别维护身份映射，以输入 ID 和输出子序号定位实体。
同类型、同 ID 的输入复用相应输出；输入消失或展开结果变少时，未领取的实体
被删除。内置 primitive 的值相同则跳过组件写入，保留 `Changed<T>` 的增量
语义；自定义实现直接写组件时需要自己判断是否有变化。

每次 `ctx.entity::<Bundle>()` 写入
`ranim_render::world::SourceOrder { order, part }`：`order` 来自完整输入，`part`
是其递归提取的输出序号。renderer 在提取完成后统一排序为连续的 `SceneOrder`，
再准备 GPU 数据。这样文字、SVG 和容器内部的顺序都排在相邻输入之间，深度
偏置范围由实际渲染顺序决定。

`RenderAppExt::render_frame`（`SubApp` 的扩展 trait）接收 `&mut EvaluatedFrame`，
临时将帧作为 Resource 交换进渲染 SubApp，完成一次 `update()` 后归还输入与批次
容量。`Extract` 借用输入，不会把渲染后的 primitive 写回原帧。

## 类型身份

typed batch 按实际 `TypeId` 匹配：提取系统只读取类型身份匹配的批次，不会根据
字符串类型名强制转换。宿主与渲染插件在同一个编译单元内构建，类型身份自然一致。
渲染插件可以为任何类型注册提取，包括下游 crate 的私有类型——注册发生在运行时，
不要求宿主在源码层面命名该类型。
