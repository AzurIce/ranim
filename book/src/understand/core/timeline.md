# 动画序列与并行组合

v0.3 不再使用 `TimelineId` 管理 Scene 内的可变时间线。动画先在用户代码中组合为完整的 `Animation`，再通过 `RanimScene::play` 加入场景。

当前有两个动态组合容器：

- `AnimSequence`：子动画按 cursor 顺序排列，适合描述一个完整状态序列。
- `AnimStack`：子动画共享同一个局部原点，适合叠加互不干扰的动画层。

`RanimScene` 自带一个根 `AnimStack`：

```rust,ignore
pub fn play<A: Animation + 'static>(&mut self, animation: A) -> &mut Self {
    self.root.push(animation);
    self
}
```

因此，多次根级 `play` 默认都从 0 秒开始。它们是并行动画，不存在后一次调用覆盖前一次调用的隐含对象语义。

## `AnimSequence`

`AnimSequence::push` 先将动画 build 为局部 `AnimationCell`，再把它移动到当前 cursor，并按 cell duration 推进 cursor：

```rust,ignore
let mut intro = AnimSequence::new();
intro
    .push(square.clone().fade_in())
    .hold(1.0)
    .push(square.fade_out());

r.play(intro);
```

Sequence 是动态类型擦除边界，但不会展开传入动画的组合树。每次 `push` 只将直接子动画转换为一个 `AnimationCell`；如果子动画是 Stack 或 Sequence，其内部层级会继续保留。

Sequence 自己通过 cursor 决定子动画的位置，因此 `push` 只接受尚未显式放置的 `Placeable`。`At<A>` 已经固定父时间坐标，不能进入 Sequence。

Sequence 本身仍实现 `Animation`，所以可以先独立构造，再整体使用 `at` 放置或加入另一个组合：

```rust,ignore
r.play(intro.at(2.0));
```

## `forward` 与 `hold`

两者都会推进 Sequence cursor，但输出语义不同：

- `forward(secs)` 只推进 cursor，产生的空白区间没有输出。
- `hold(secs)` 取得 cursor 处的 Sequence 状态，将它保存为持续 `secs` 的静态运行时节点。
- `forward_to(target)` 和 `hold_to(target)` 是对应的绝对 cursor 版本。

`hold` 没有额外的状态协议，它直接采用 Sequence 在 cursor 处的正常求值结果。Sequence 在同一时刻只求值最后一个适用的直接子动画；如果这个子动画是 Stack，则由 Stack 求值其中所有仍然适用的子动画。已经提前结束的 Stack 子动画不会被自动延长。

```text
child A: [0, 1)
child B: [0, 2)
cursor:        2

hold at 2 -> 只保持 B 的左侧终态
```

连续 `hold` 会分别保存每次调用时的求值结果，形成相邻的静态区间。

## `show`、`hide` 与最终求值

`show()` 和 `hide()` 都是普通的零时长动画：

- `show()` 是 enabled 的静态动画，求值时输出对应物件；
- `hide()` 是 disabled 的静态动画，求值时不输出内容。

它们不需要 `hold` 特判。因为 Sequence 在边界上选择最后一个适用的直接子动画，末尾的 `show()` 会成为最终求值结果，末尾的 `hide()` 则自然得到空结果；`hold` 只负责把这个结果保存为静态动画。

```rust,ignore
let mut content = AnimSequence::new();
content
    .push(square.show())
    .hold(1.0)
    .push(square.hide())
    .hold(1.0);
```

这里 `hide` 只改变 `content` 这条 Sequence 的状态。它不会查找或影响根 Stack 中另一个独立动画。

如果两个物件需要独立生命周期，应分别使用两个 Sequence：

```rust,ignore
r.play(square_sequence);
r.play(circle_sequence);
```

如果两个物件需要在同一时刻一起求值，应直接 push 一个 `stack![...]` 组合。

## `AnimStack` 与根场景

`AnimStack::push` 不推进其他子动画；Stack duration 是所有子动画 duration 的最大值：

```rust,ignore
let animation = stack![
    background.show().with_duration(5.0),
    content.at(1.0),
    camera.show().with_duration(5.0),
];

r.play(animation);
```

Stack 接受普通 `Placeable` 动画和已经放置的 `At<A>`。普通动画从 Stack 局部 0 开始，`At<A>` 使用自己的显式 offset。参数必须在调用 `at` 之前设置。

运行时数量不固定时可以直接构造 `AnimStack`：

```rust,ignore
let mut layers = AnimStack::new();
for animation in animations {
    layers.push(animation);
}
r.play(layers);
```

## `AnimLagged`

`AnimLagged` 把一组**未放置**（`Placeable`）的子动画按 stagger 规则相继排布：第 `i` 个子动画的起点是 `start_{i-1} + lag_ratio · d_{i-1}`。`lag_ratio` 插值在两种容器语义之间：

- `0.0` —— 所有子动画同时开始（类似 `AnimStack`）；
- `1.0` —— 首尾相接（类似 `AnimSequence`）；
- 中间值 —— 重叠相继。

```rust,ignore
let animation = lagged![0.2;
    square.fade_in(),
    circle.fade_in(),
    text.write(),
];
r.play(animation);
```

子动画窗口之外的时间默认由**真实的静态动画**填充：每个元素在 build 时被物化为一条 `[前填充][动画][后填充]` 的 per-item `AnimSequence` 轨道（前=初态，后=末态，采样自窗口边缘，空的填充会被跳过），因此 preview 时间线看到的就是实际渲染的内容，没有隐藏的求值规则。每端的行为可以用 `with_leading`/`with_trailing` 配置（`LaggedFill::{Hold, Empty}`，默认都是 `Hold`）；若希望元素在窗口结束后消失，让它的动画以 `hide` 结尾即可（如 `seq![item.fade_in(), item.hide()]`）。

填充在 build 时采样，因此子动画应当是纯（闭式）动画——迭代式子动画的末态填充会得到其初态。

对一组元素施加同一个动画时，用迭代器收集（core 的 `AnimIterExt`）：

```rust,ignore
let animation = group
    .iter_mut()
    .map(|item| item.fade_in().with_rate_func(smooth))
    .into_lagged(0.2);
```

迭代器还可以收集为另外两个容器：`into_stack()`/`into_seq()`，或直接 `collect::<AnimStack>()`/`collect::<AnimSequence>()`。

## 场景时长与显式生命周期

Scene 总时长是根 Stack 中最长子动画的 duration。新模型不会像旧 Timeline 那样在 seal 时自动把静态物件和相机延长到 Scene 结束。

需要全程存在的内容应显式指定生命周期：

```rust,ignore
let total_secs = content.cursor_sec();

let mut camera = AnimSequence::new();
camera
    .push(CameraFrame::default().show())
    .hold_to(total_secs);

r.play(camera);
r.play(content);
```

这种写法使空白和保持区间成为动画定义的一部分。后续可以增加默认相机或 `through_scene_end` 等辅助 API，但它们不改变 Sequence/Stack 的组合语义。

## `seq!`、`stack!` 与 `lagged!`

固定写法可以使用宏简化：

```rust,ignore
let intro = seq![
    square.clone().fade_in(),
    square.fade_out(),
];

let scene = stack![intro, camera];
r.play(scene);
```

`seq!` 返回 `AnimSequence`，`stack!` 返回 `AnimStack`。二者都只是构造辅助，最终 build 为保留子节点层级的运行时动画树。`lagged![0.2; a, b, c]` 以 0.2 的 stagger ratio 返回 `AnimLagged`（见上文）。
