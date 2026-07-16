# 全局 Timeline 与 TimeCursor

> 状态：构思中。本页记录一种作者阶段的时间布局 API，不代表最终实现。

## 想法

场景只维护一个全局 `Timeline`，其中保存所有已经排布完成的动画片段。`TimeCursor` 不表示一条独立的运行时时间线，而是作者编码动画时使用的时间位置游标。

```text
TimeCursor A ─┐
TimeCursor B ─┼─ 调度动画片段 ─► Global Timeline ─► Runtime World
TimeCursor C ─┘
```

这样可以把两个目前混合在一起的概念拆开：

- `Timeline` 是场景最终的时间调度结果，可供求值器查询。
- `TimeCursor` 是构造 Timeline 时的辅助状态，用来决定下一个动画被放在哪个时间位置。

多个 cursor 可以独立前进、跳转和同步，但它们最终仍向同一个 Timeline 写入动画片段。

## 动机

当前模型倾向于让每个物件对应一条 Timeline，并通过各 Timeline 的 `cur_sec` 排布动画。这种方式适合简单的逐物件动画，但会逐渐遇到以下问题：

- “时间线”同时表示物件所有权和作者当前时间位置，职责不清晰。
- 多个物件需要同步、错开或汇合时，需要直接操作多条 Timeline。
- 一个物件难以自然参与多个独立的动画编排段落。
- Editor 中的全局时间轴与代码中的多条对象 Timeline 不完全对应。

TimeCursor 将时间布局从物件所有权中提取出来。物件由 World 持有，动画片段通过 `EntityId` 引用物件，cursor 只负责提供片段的开始时间。

## 概念 API

```rust,ignore
let main = r.create_cursor();
let annotations = r.create_cursor();

r.with_cursor(main).play(square.fade_in());
r.with_cursor(main).play(square.shift(RIGHT));

r.with_cursor(annotations)
    .seek(0.5)
    .play(label.fade_in());

r.sync((main, annotations));
```

对应的内部操作可以近似理解为：

```rust,ignore
pub struct TimeCursor {
    time: Time,
}

pub struct Timeline {
    clips: Vec<ScheduledClip>,
}

fn play(&mut self, cursor: CursorId, clip: impl IntoClip) -> Playback {
    let start = self.cursor(cursor).time;
    let playback = self.timeline.schedule(start, clip);
    self.cursor_mut(cursor).time = playback.range.end;
    playback
}
```

## Cursor 操作

TimeCursor 至少需要支持以下布局操作：

- `forward(duration)`：相对当前位置前进。
- `seek(time)`：移动到绝对时间。
- `sync(other)`：移动到另一个 cursor 的位置。
- `sync_to_end(playback)`：移动到某个已调度片段的结束位置。
- `fork()`：从当前位置创建另一个 cursor，用于并行编排。

其中 `seek` 和 `forward` 只修改作者阶段的游标，不会立即求值 World，也不表示 runtime player 的 seek。

## Cursor 的身份与借用

如果 `with_cursor` 长期持有 `&mut TimeCursor`，用户同时编排多个 cursor 时容易遇到 Rust 可变借用冲突。因此 cursor 更适合作为轻量的 `CursorId`：

```rust,ignore
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CursorId(/* generational id */);
```

cursor 的实际时间存放在 `Ranim` 或单独的 `TimelineComposer` 中。`with_cursor(cursor)` 可以返回一个短生命周期的 facade，在一次调用链结束后释放对 composer 的借用。

```rust,ignore
r.cursor(main)
    .forward(0.5)
    .play(anim);
```

## 与 Runtime 的边界

TimeCursor 只存在于 authoring/composition 阶段。场景完成构造后，求值器只需要看到已经调度好的 Timeline：

```text
Authoring:
    Cursor 操作 + play → ScheduledClip

Runtime:
    Timeline::apply_at(t, world)
```

因此 cursor 不需要参与：

- Timeline 的 `seek(t)`；
- Simulation 的固定步长推进；
- World snapshot/checkpoint；
- Renderer 的帧采样。

如果 Editor 需要显示 cursor，它显示的是编辑会话中的布局工具，而不是场景运行时状态。

## 待决问题

- 是否允许 cursor 向后移动后继续写入，从而产生重叠片段？初步倾向允许，由冲突检测负责报告同一属性的多重写入。
- `sync((a, b))` 应取最大时间、最小时间，还是要求显式指定？初步倾向默认取最大值，并提供明确的 `sync_to_min` 等操作。
- cursor 是否属于持久的场景数据？初步倾向不是；保存场景时只需要保存 scheduled clips，除非 Editor 要恢复作者的编辑位置。
- 是否保留一个隐式的默认 cursor？保留可以让简单动画继续使用 `r.play(anim)`，复杂场景再显式创建 cursor。
