# CoreItem 与 Extract

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

场景中所有可见内容最终都归结为少数几种 **core item**。动画系统求值得到的是
用户层 item（`ranim-items` 中的 `VItem`、`Surface` 等高层类型），而渲染器只
认识 core item；连接二者的是 `Extract` trait。

## `CoreItem`

`CoreItem` 定义在 `ranim-core` 的 `core_item` 模块，是渲染管线的输入枚举：

```rust,ignore
pub enum CoreItem {
    CameraFrame(CameraFrame),
    VItem(VItem),
    MeshItem(MeshItem),
}
```

三种 core item 的共同特征：数据为 f32（`Vec3` / `Vec4` / `Mat4`）、位于世界
空间、不再携带任何动画辅助结构，每种直接对应渲染管线的一条路径。字段级的
说明见 [Core Items](./core_items/README.md)。

## `Extract`

```rust,ignore
pub trait Extract {
    type Target: Clone;

    /// 把提取结果追加到 buf。
    fn extract_into(&self, buf: &mut Vec<Self::Target>);

    /// 提取为新分配的 Vec。
    fn extract(&self) -> Vec<Self::Target>;
}
```

要点：

- **提取可以是 1→N**：一个用户层 item 可以 extract 成任意个 core item。
  高层 `VItem` 恰好产生 1 个 core `VItem`；`Surface` 经高层 `MeshItem` 产生
  1 个 core `MeshItem`；而 `SvgItem` / `TypstText` 这类复合 item 会产生**多个**
  core `VItem`。
- **可组合**：`Vec<E>`、数组、元组等都有 `Extract` 实现，逐个把成员的提取
  结果追加进同一个 buffer，因此一帧的输出可以任意拼接。

## 从动画求值到 extract

动画叶子的产出先被类型擦除为 `DynItem`：

```rust,ignore
pub trait AnyExtractCoreItem: Any + Extract<Target = CoreItem> + DynClone {}
pub struct DynItem(pub Box<dyn AnyExtractCoreItem>);
```

`SceneEvaluator::sample_at(render_secs, out)` 对根 Stack 的每个顶层 cell 求值，
再把每个产出 item 逐个 `extract()` 展开，得到一帧的 core item 列表：

```text
AnimationCell::eval_at(sec)  ->  Vec<DynItem>
  每个 DynItem.extract()     ->  Vec<CoreItem>      （这里发生 1→N）
  汇总                       ->  EvaluatedFrame
                                 = Vec<((animation_id, part), CoreItem)>
```

`EvaluatedFrame` 中每个 core item 附带的身份是 `(animation_id, part)`：

- `animation_id`：该 item 来自的根 Stack 顶层动画序号；
- `part`：extract 展开后的序号。由于存在 1→N 映射，`part` 是 core item 序号，
  **不**等于用户层 item 的序号——这也是 `ranim inspect frame` 输出中 `part`
  字段的含义（见 [Ranim CLI](../../cli.md)）。

## 谁来消费

渲染器（`ranim-render`）按 `CoreItem` 变体分发到对应的渲染路径：core
`VItem` 走矢量渲染（平面投影 + 三角化），core `MeshItem` 走 3D 网格渲染，
`CameraFrame` 提供每帧的视图/投影矩阵。preview 与离线渲染共用同一条
`sample_at` → `EvaluatedFrame` 路径。
