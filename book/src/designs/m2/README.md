# M2 LogicWorld

本文记录 M2 的设计：把 item 引入逻辑侧的 retained ECS World（`LogicWorld`），由
`ScenePlayer` 驱动，输出与纯求值路径（`SceneEvaluator`）逐帧一致。核心问题只有一个：
**动画表示是类型擦除的，而 ECS World 需要具体类型**——物化能力从何而来。

## 两个擦除点

动画表示里类型擦除发生在两个地方，而不是一个：

```text
E: Eval<Output = T>  ──Animation::build()──▶  Box<dyn EvalDyn>   （cell 层）
        T            ──eval_dyn──▶           DynItem             （输出层）
```

类型擦除并没有丢失类型知识，而是把它搬进了 vtable——前提是所需的操作在擦除点被
单态化为 vtable 条目。因此"擦除之后如何物化"的通用答案是：**物化必须是擦除点处
捕获的钩子**。

M2 最初（stage 1 初版）只在 cell 层补了一个 `EvalDyn::materialize_dyn`：每个节点在
`eval_dyn` 之外再实现一条平行的类型化遍历，把 `Output` upsert 进 World。这个方案被
否决了，它的失败解释了现在的设计：

- 两条遍历必须永远路由一致（sequence 的逆序查找、stack/lagged 的全遍历、
  `contains_sec` 的右端点特判各抄一遍），改一处漏一处就会让 World 与渲染静默
  脱钩，编译器无法发现；
- 输出层的擦除点补不上：`hold()` 与 lagged fill 在 build 期把 item 固化进静态
  cell，此时类型已擦除为 `DynItem`，物化只能靠宿主 downcast 猜测核心类型，
  自定义类型会被跳过并导致 part 计数错位。

## DynItem 自物化钩子

现在的设计把物化能力封进输出层的擦除点。`DynItem` 全仓只有一个构造点
（blanket `eval_dyn`），因此它自带一个在构造时单态化的物化钩子：

```rust,ignore
pub struct DynItem {
    inner: Box<dyn AnyExtractCoreItem>,
    materialize: fn(Self, &mut MaterializeCtx, u32),  // downcast 回 T 并 upsert
}
```

物化阶段不再需要自己的遍历，它就是共享的 eval 遍历加上每个 item 的自物化：

```text
materialize_at(sec) = cell.eval_at(sec) 得到 Vec<DynItem>
                    → 每个 DynItem 用自带钩子 upsert 自身（按 part 槽位）
```

- 路由只有一份（`eval_dyn`），不存在两条遍历脱钩的可能；
- 静态快照里的 `DynItem` 在捕获瞬间就带好了钩子，`hold`/lagged fill 中的自定义
  类型也能正确物化；
- `Vec<T>` 组输出作为一个 `DynItem`（经 `Batch<T>`）占一个槽位，与 extract 路径
  的 1→N 展开语义一致。

## 身份、生命周期与提取

- **身份**：`(animation_id, part)`。`animation_id` 是顶层 cell 序号，`part` 是
  cell 内输出槽位序号，与纯路径 `EvaluatedFrame` 的身份完全一致，因此渲染侧
  `RenderWorld` reconciliation 不需要任何改动。
- **生命周期 = 生产者生命周期**：`ScenePlayer.index` 持有跨帧的
  `(animation_id, part) → Entity` 索引；同 key 再次物化只替换组件（entity 稳定
  跨帧保留），本帧未出现的 key 对应 entity 被 despawn。
- **槽位类型变化必须重建 entity**：sequence 切换子段后同一槽位的输出类型可能改变，
  此时旧 entity 上的 `ItemExtractor` 与新组件类型不匹配。upsert 检测到槽位上不是
  目标类型时必须 despawn 并重新 spawn，不得只 insert 新组件（旧组件会残留并被
  旧 extractor 提取出陈旧值）。
- **提取是类型擦除的 fn 指针**：entity spawn 时由物化点写入
  `ItemExtractor(fn(Entity, &World, &mut Vec<CoreItem>))`，driver 不需要知道具体
  类型即可把组件退化为 `CoreItem`；每帧提取结果写回 entity 的
  `ExtractedItems(Vec<CoreItem>)`，buffer 跨帧复用。
- **顺序与身份是两件事**：ECS 查询顺序不构成场景顺序，collect 阶段按
  `SceneOrder` 显式排序，再把每个 entity 提取出的若干 `CoreItem` 扁平化为连续
  part 序号。

## ScenePlayer

`ScenePlayer` 是 `SceneEvaluator` 的 retained-world 对照物。由于求值是纯查询
（`eval_alpha`），它没有任何步进或 seek 簿记——方向管理内收于 `Iterative` 叶子，
session 只暴露一帧的三段式管线：

```text
frame(render_secs) = materialize_at → extract → collect
```

测试保证其与 `SceneEvaluator::sample_at` 逐帧一致，且后跳采样与前向一致。

## LogicItem 与 Batch：coherence 约束

`LogicItem`（= `Component` + 可退化为 `CoreItem`）**不得有 blanket impl，必须逐个
显式实现**。只有这样编译器才能证明 `Vec<T>` 不是 `LogicItem`，于是
`MaterializeOut` 才能同时拥有两条互不冲突的路径：单 item（`T: LogicItem`）直接
upsert；组输出（`Vec<T>`）包进 `Batch<T>` 组件作为单个 entity。给 `LogicItem`
加 blanket impl 会导致两条 `MaterializeOut` impl 重叠，无法编译。

## 边界

- 渲染侧不在本设计范围内：collect 输出格式与纯路径相同即是全部契约。
- 提取结果目前在 entity 上退化为 `CoreItem`（`ExtractedItems`）；让提取产物保持
  类型化（`Extracted<T>`，供消费者按类型查询、并承接 `TextItem` 的派生缓存）是
  本设计的既定延伸，不改变身份与生命周期模型。
- 进程内 fn 指针假设 item 类型对宿主编译期可见；让类型可以来自宿主之外
  （dylib/wasm 注册自己的物化器）需要一个同形状的全局 registry 替代 fn 指针，
  同样不改变本设计的身份模型。
