# 动画序列与并行组合

v0.3 不再使用 `TimelineId` 管理 Scene 内的可变时间线。动画先在用户代码中组合为完整的 `Animation`，再通过 `RanimScene::play` 加入场景。

当前有两个动态组合容器：

- `AnimSequence`：子动画按 cursor 顺序排列，适合描述一个完整状态序列。
- `AnimStack`：子动画共享同一个局部原点，适合叠加互不干扰的动画层。

`RanimScene` 自带一个根 `AnimStack`：

```rust,ignore
pub fn play<A: Animation>(&mut self, animation: A) -> &mut Self {
    self.root.push(animation);
    self
}
```

因此，多次根级 `play` 默认都从 0 秒开始。它们是并行动画，不存在后一次调用覆盖前一次调用的隐含对象语义。

## `AnimSequence`

`AnimSequence::push` 在当前 cursor 处 build 动画，再按动画的局部 duration 推进 cursor：

```rust,ignore
let mut intro = AnimSequence::new();
intro
    .push(square.clone().fade_in())
    .hold(1.0)
    .push(square.fade_out());

r.play(intro);
```

Sequence 是动态类型擦除边界。传入的静态 Animation 组合树会被展开为扁平的 `Vec<BuiltAnimation>`，但时间范围、rate function、enabled 和 evaluator 名称仍保存在 Box 外。

Sequence 本身仍实现 `Animation`，所以可以先独立构造，再整体使用 `at` 放置或加入另一个组合：

```rust,ignore
r.play(intro.at(2.0));
```

## `forward` 与 `hold`

两者都会推进 Sequence cursor，但输出语义不同：

- `forward(secs)` 只推进 cursor，产生的空白区间没有输出。
- `hold(secs)` 取得 cursor 处的 Sequence 状态，将它保存为持续 `secs` 的静态 BuiltAnimation。
- `forward_to(target)` 和 `hold_to(target)` 是对应的绝对 cursor 版本。

没有零时长 state event 时，`hold` 采样 cursor 左侧仍然活动的动画。已经提前结束的 Stack 子动画不会被自动延长。

```text
child A: [0, 1)
child B: [0, 2)
cursor:        2

hold at 2 -> 只保持 B 的左侧终态
```

连续 hold 会直接延长相邻的静态区间，不会形成递归的 `DynItem<Vec<DynItem<...>>>` 包装。

## `show`、`hide` 与状态快照

`show()` 和 `hide()` 都产生零时长 state event：

- `show()` 是 enabled 的静态状态；
- `hide()` 是 disabled 的空状态标记。

当 cursor 上存在 state event 时，后续 `hold` 不再继承左侧状态，而是将该时刻全部 enabled events 的输出作为 Sequence 的新完整快照。只有 `hide` 时快照为空，因此后续区间不输出内容。

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

如果两个物件属于同一个状态快照，则在同一 cursor 提交两个 enabled events，或直接 push 一个 `stack![...]` 组合。

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

运行时数量不固定时可以直接构造 `AnimStack`：

```rust,ignore
let mut layers = AnimStack::new();
for animation in animations {
    layers.push(animation);
}
r.play(layers);
```

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

## `seq!` 与 `stack!`

固定写法可以使用宏简化：

```rust,ignore
let intro = seq![
    square.clone().fade_in(),
    square.fade_out(),
];

let scene = stack![intro, camera];
r.play(scene);
```

`seq!` 返回 `AnimSequence`，`stack!` 返回 `AnimStack`。二者都只是构造辅助，最终遵循同一套 `Animation::build` 规则。
