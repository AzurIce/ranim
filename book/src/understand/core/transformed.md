# `Transformed<T, G>`：变换表示、组合与物件语义

> [!caution]
> ai 生成，可能叙事逻辑和表述并不是很好，仅供参考。

ranim 把“物件自身的数据”和“附加在物件外的变换”分开表达：

```rust,ignore
pub struct Transformed<T, G> {
    pub inner: T,
    pub transform: G,
}
```

`T` 是物件，`G` 是 wrapper **实际存储的变换表示**。字段保持公开，可以在
自定义 evaluator 中直接读写；常规组合则推荐使用 `compose_outer` 和
`compose_inner`，使乘法顺序一目了然。

## 1. 变换类型与仿射端点

ranim 的模型变换层级如下：

```mermaid
flowchart LR
    T["Translation<br/>T(3)"] --> R["Rigid<br/>SE(3)"]
    R --> S["Similarity<br/>Sim(3)"]
    S --> A["DAffine3<br/>Aff(3)<br/>模型变换端点"]
    D["Diag<br/>轴向缩放"] --> A
    A -.-> P["Projective<br/>仅作为相机投影边界"]

    classDef endpoint fill:#dbeafe,stroke:#2563eb,stroke-width:3px,color:#172554
    classDef boundary fill:#e5e7eb,stroke:#9ca3af,stroke-width:1.5px,color:#6b7280
    class A endpoint
    class P boundary
    linkStyle 4 stroke:#9ca3af,color:#6b7280
```

- `Translation`：纯平移；
- `Rigid`：旋转和平移；
- `Similarity`：正的均匀缩放、旋转和平移；
- `Diag`：沿坐标轴缩放，它不属于 `Similarity`；
- `DAffine3`：模型变换的最一般表示，可以表达剪切和一般仿射组合；
- projective 不是模型变换的下一种存储类型，只存在于相机投影边界。

图中的实线箭头对应无损的 `From` 嵌入。`From` 在 Rust 中不传递，因此 ranim
显式提供已有层级中的全部转换：

```text
Translation -> Rigid / Similarity / DAffine3
Rigid       -> Similarity / DAffine3
Similarity  -> DAffine3
Diag        -> DAffine3
```

每个表示都实现同一个 `TransformGroup` 能力，提供单位元和同族组合。组合顺序与
仿射矩阵一致：

$$
op("compose")(g_2, g_1) = g_2 g_1
$$

也就是先作用 $g_1$，再作用 $g_2$。`Similarity` 组合时，外层的缩放和旋转也会
作用于内层平移；它不是把三个字段分别相加。

`Diag` 仍保留当前数值行为，包括零缩放。这里没有额外引入“严格可逆”的运行时
检查。

## 2. `ApplyTransform<G>`：物件能直接吸收什么

基础接口是：

```rust,ignore
pub trait ApplyTransform<G> {
    fn apply(&mut self, transform: G) -> &mut Self;
}
```

物件通过实现范围声明自己的闭包：

```rust,ignore
// 点数据可以吸收仿射变换及其子类型
impl<G: Into<DAffine3>> ApplyTransform<G> for VItem { /* ... */ }

// 圆、球、矩形等参数化形状只吸收相似变换及其子类型
impl<G: Into<Similarity>> ApplyTransform<G> for Circle { /* ... */ }
```

便利操作由这个接口派生：

| 操作 | 提交给 `ApplyTransform` 的类型 |
|---|---|
| `shift(offset)` | `Translation` |
| `rotate_on_axis(axis, angle)` | `Rigid` |
| `scale_uniform(s)` | `Similarity` |
| `scale(DVec3)` | `Diag` |

因此裸 `Rectangle` 可以平移、旋转和均匀缩放，但不能直接接受一般非均匀
`Diag`；后者通常会破坏相邻边正交这一表示不变量。

## 3. 构造 wrapper：参数就是精确的 `G`

构造器同时接收物件与变换：

```rust,ignore
let item = Transformed::new(mesh, DAffine3::IDENTITY);
```

prelude 还导出了 blanket extension trait，可以写成：

```rust,ignore
let item = mesh.transformed::<DAffine3>(DAffine3::IDENTITY);
```

`.transformed::<G>(g)` 的参数必须恰好是 `G`；这个入口不会替调用者选择更宽的
存储类型。通常可以让类型推断直接从参数得到 `G`：

```rust,ignore
let item = mesh.transformed(Translation(offset));
// 类型是 Transformed<MeshItem, Translation>
```

选择较窄的 `G` 会让可组合的操作在编译期受限；选择 `DAffine3` 则是现有
mesh/surface 场景常用的存储上界。

## 4. outer 与 inner 组合

设 wrapper 当前存储 $F$，新变换为 $H$。两种组合只有乘法方向不同：

$$
F_("outer")' = H F
$$

$$
F_("inner")' = F H
$$

对应 API：

```rust,ignore
item.compose_outer(h); // transform = h * transform
item.compose_inner(h); // transform = transform * h
```

`ApplyTransform<H> for Transformed<T, G>` 使用 **outer composition**，所以
`shift`、`rotate`、`scale_uniform` 和 `scale` 在可用时也都走左乘：

```rust,ignore
item.apply(h); // 等价于 item.compose_outer(h)
```

`compose_outer` 与 `compose_inner` 都要求 `G: From<H>`，先把 `H` 嵌入现有的
`G`，再在 `G` 内完成同族组合。它们不会创建新的 wrapper 类型。

### 4.1 不自动 widening，也不计算 join

下面的 wrapper 保持 `Similarity` 存储：

```rust,ignore
let mut item = sphere.transformed(Similarity::IDENTITY);
item.shift(offset);        // Translation -> Similarity
item.scale_uniform(2.0);   // 仍是 Transformed<Sphere, Similarity>
```

但一般 `Diag` 不能嵌入 `Similarity`，因此 `item.scale(non_uniform)` 不会偷偷把
类型改成 `Transformed<_, DAffine3>`，而是在编译期不可用。需要更一般的组合时，
显式 widening：

```rust,ignore
let narrow = mesh.transformed(Translation(offset));
let mut affine: Transformed<_, DAffine3> = narrow.into();
affine.scale(DVec3::new(2.0, 1.0, 1.0));
```

这种显式 `Into` 让 API 的返回类型稳定，也避免为任意两种变换表示自动推导
“最小共同上界”所带来的 coherence 问题。

### 4.2 嵌套 wrapper

嵌套依然从内向外展平：

```text
Transformed {
    transform: outer,
    inner: Transformed {
        transform: inner,
        inner: x,
    },
}

最终组合 = outer * inner
```

## 5. bake 是编译期能力

`bake` 的签名直接使用 wrapper 的 `G`：

```rust,ignore
pub fn bake(self) -> T
where
    T: ApplyTransform<G>;
```

所以它是否存在完全由类型系统决定：

```rust,ignore
let rectangle = rectangle
    .transformed(Similarity::from_scale(2.0))
    .bake(); // Rectangle: ApplyTransform<Similarity>

let affine = rectangle.transformed(DAffine3::from_scale(...));
// affine.bake(); // 编译失败：Rectangle 不吸收 DAffine3
```

wrapper 不再提供 `try_bake`。如果调用者确实需要把动态得到的 `DAffine3` 向下
检查为 `Similarity`，应先显式执行 `Similarity::try_from(affine)`，然后构造
`Transformed<T, Similarity>`；成功后 `bake` 仍然是静态能力。

这与几何闭包相符：

| 物件表示 | 可直接吸收的上界 |
|---|---|
| 点、点集、`VItem`、一般 mesh 点数据 | `DAffine3` |
| `Parallelogram` | `DAffine3` |
| `Circle` / `Sphere` | `Similarity` |
| `Rectangle` / `Square` | `Similarity` |

一般仿射变换会把圆变成椭圆、把矩形变成平行四边形，因此不能无损地 bake 回
原来的参数化类型。

## 6. extract、Aabb 与几何边界

`Transformed<T, G>` 不要求 `T: ApplyTransform<G>` 就能 extract。只要 `G` 能
转换为 `DAffine3`，wrapper 会在几何边界进行一次转换：

```text
G --Into<DAffine3>--> CoreItem / Aabb geometry
```

- `VItem`：仿射变换烘焙到点，法线使用逆转置；
- core `MeshItem`：仿射矩阵左乘已有渲染矩阵，顶点保持不变；
- `Aabb`：变换内部包围盒的八个角点，再重新取界。

这使高层物件可以保留语义表示，同时渲染结果仍能包含更一般的仿射效果。
`DAffine3` 是这里的端点；不会继续转换到 projective 模型矩阵。

## 7. 插值

只有两个 wrapper 的 `T` 与 `G` 都相同时才能直接插值：

```rust,ignore
Self {
    inner: self.inner.lerp(&target.inner, t),
    transform: self.transform.lerp(&target.transform, t),
}
```

`inner` 与 `transform` 独立插值，不会先展平为点数据。若两端 wrapper 使用不同
存储类型，应先由调用者把它们显式 widening 到同一个 `G`。

## 8. `Rectangle::scale_axes` 与 wrapper 组合

`Rectangle::scale_axes(DVec2)` 是**形状参数编辑**：它只修改 `size`，尺寸沿
矩形已经存储的两条正交 shape axes 解释。矩形即使先旋转过，调用
`scale_axes` 后仍保留这组旋转后的正交轴表示。

它不同于 wrapper 组合：

```rust,ignore
rectangle.scale_axes(dvec2(2.0, 0.5));
// 修改 Rectangle 的维度参数

wrapped.compose_inner(Diag(dvec3(2.0, 0.5, 1.0)));
// inner 不变，右乘 wrapper.transform

wrapped.compose_outer(Diag(dvec3(2.0, 0.5, 1.0)));
// inner 不变，左乘 wrapper.transform
```

不要把 `scale_axes` 理解为矩阵乘法的别名：前者维护 `Rectangle` 的正交参数化
表示，后两者维护 wrapper 的组合顺序。

## 9. 几何视角

Klein 的 Erlangen 纲领把一种几何理解为“研究某个变换群下的不变量”。在
ranim 中：

- `G` 描述允许组合的变换；
- `ApplyTransform<G>` 描述物件表示对这个群是否闭包；
- `Transformed<T, G>` 把外部组合与 `T` 的参数化语义分离；
- `bake` 在编译期重新要求闭包；
- `extract` 在仿射几何端点生成最终视觉数据。

这个分层让变换的数学顺序、Rust 类型与物件几何语义保持一致。
