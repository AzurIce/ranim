# GPU 上传路径 Profiling 笔记（2026-09-09，worktree `gpu-profiling`）

目标：量化当前"每帧全量 merged-buffer 上传"的 CPU/GPU 成本，并评估替代方案。
环境：RTX 4070 Ti SUPER / Vulkan / release / 1920×1080 / 每 60 测量帧取均值。

## 工具（本 worktree 新增）

- `packages/ranim-render/src/upload_probe.rs`：按 buffer label 统计每次上传的
  字节数 / 调用数 / write 调用 CPU 时间；并实现两个**实验性上传策略**：
  - `skip-equal`：与上次上传逐字节相同 → 跳过（shadow copy memcmp）
  - `dirty-ranges`：256B 块级脏检测，只写合并后的脏区间（>16 段则整写）
  - `Merged ClipBoxes` 被豁免（GPU atomic 累积，必须每帧重初始化）
- `benches/src/bin/profile_gpu_upload.rs`：6 场景 × 3 模式；同时输出**内容冗余
  分析**（相邻帧 CoreItem 逐字段 diff → 理论最小上传字节）。
- render pass 级 GPU timer scope（`profiling` feature 补上了此前缺失的
  begin/end scope；`Renderer::take_last_gpu_scopes()` 可编程读取）。

复现：

```bash
nix develop
cargo build --release -p benches --bin profile_gpu_upload
RANIM_PROFILE_UPLOAD=count        ./target/release/profile_gpu_upload  # 基线（全量）
RANIM_PROFILE_UPLOAD=skip-equal   ./target/release/profile_gpu_upload  # 原型 A
RANIM_PROFILE_UPLOAD=dirty-ranges ./target/release/profile_gpu_upload  # 原型 B
# GPU pass 时间（注意：会强制每帧 device poll，submit 数字不可比）：
cargo build --release -p benches --bin profile_gpu_upload --features gpu-scopes
```

原始输出存于 `notes/profiling/profile_{count,skip,dirty,scopes}.txt`。

### Preview app 内置 Profiler 面板

顶栏 `📈 Profiler` 按钮打开，分四节：GPU pass 耗时表（含占比条）、
GPU 总耗时历史曲线、buffer 上传统计表、written KiB/frame 曲线。
上传策略（Off/Count/SkipEqual/DirtyRanges）可在面板里**运行时切换**，
适合对着动画实时 A/B。

```bash
# GPU timer 可用（请求 TIMESTAMP_QUERY 系特性）：
cargo run -p ranim-cli --features profiling -- preview --example <example>
# 无 profiling feature 时面板仍可开，GPU 节显示提示，上传统计照常可用：
cargo run -p ranim-cli -- preview --example <example>
```

注意：profiling feature 下每帧渲染后会 device poll 同步一次（wgpu-profiler
取 timer 所需），preview 的 render 耗时会比非 profiling 构建偏高。

## 场景

- `static(n)`：n² 个方块，单帧求值结果重复渲染（steady/idle 情形）
- `static-pan(n)`：同上 + 每帧相机平移（item 不变，只有 viewport uniform 变）
- `morph(n)`：n² 个方块 morph 成圆（最坏情形：每帧所有点都在变）

## 结果 1：上传字节与内容冗余

| 场景 | items | 全量上传/帧 | 理论最小上传/帧 | 冗余 |
|---|---|---|---|---|
| static(20) | 400 | 234.5 KiB | 7.8 KiB（仅 clip init） | **96.7%** |
| static(40) | 1600 | 937.6 KiB | 31.2 KiB | **96.7%** |
| static(60) | 3600 | 2109.5 KiB | 70.3 KiB | **96.7%** |
| static-pan(40) | 1600 | 937.6 KiB | 31.2 KiB | **96.7%** |
| morph(20) | 400 | 390.8 KiB | 126.6 KiB | **67.6%** |
| morph(40) | 1600 | 1562.6 KiB | 506.2 KiB | **67.6%** |

static(20) 的字节构成（每 item 9 点 / 5 attrs / 600B）：

| buffer | B/item | 占比 | 每帧是否真的需要上传 |
|---|---|---|---|
| points2d（全零） | 144 | 24% | **否**，compute pass 每帧全量重写 |
| points3d | 144 | 24% | 静态时否；morph 时是 |
| fill/stroke rgbas | 80+80 | 27% | 静态时否 |
| transforms | 64 | 11% | 静态时否 |
| planes / infos / clip / widths | 32/16/20/20 | 15% | clip 是 init pattern；其余静态时否 |

即：**约 27% 的每帧上传（points2d + clip_boxes）与内容变化完全无关，纯粹是
初始化数据**；静态场景另有 ~70% 可由 change tracking 消除。

## 结果 2：CPU 成本（count vs 原型）

| 场景 | submit（全量） | upload cpu | submit（skip-equal） | submit（dirty-ranges） | written% |
|---|---|---|---|---|---|
| static(20) | 0.09 ms | 0.02 ms | 0.08 ms | 0.09 ms | 3.4% |
| static(40) | 0.25 ms | 0.06 ms | 0.21 ms | 0.20 ms | 3.3% |
| static(60) | 0.47 ms | 0.11 ms | 0.40 ms | 0.42 ms | 3.3% |
| static-pan(40) | 0.24 ms | 0.06 ms | 0.24 ms | 0.21 ms | 3.3% |
| morph(20) | 0.14 ms | 0.03 ms | 0.13 ms | 0.12 ms | 32.4% |
| morph(40) | 0.39 ms | 0.09 ms | 0.36 ms | 0.34 ms | 32.4% |

- 上传 CPU 成本 ≈ **33 ns/item**（3600 items → 0.11 ms/帧），占 submit 的
  ~25%，线性增长；60fps 预算内占比很小（0.7%）。
- skip-equal 把 upload cpu 降到 ~0.01 ms，submit 降 10–20%；**written% 恰好
  达到理论冗余上限**（static 3.3%、morph 32.4%）。
- dirty-ranges 与 skip-equal 收益相同：静态场景二者等价；morph 场景脏数据
  集中在大块连续区间，块级切分无从细化。**按字段/按 item 拆分才是结构性手段**。

## 结果 3：GPU pass 时间（static(60)，3600 items）

| pass | GPU 时间 |
|---|---|
| clear | ~0 |
| vitem::compute（点投影 + clip 累积） | 10.3 μs |
| vitem::depth | 69.6 μs |
| vitem::color | 79.9 μs |
| **oit::resolve** | **293.7 μs** |

- GPU pass 总计 ~454 μs，与无 profiler 的全帧时间（~0.61 ms）同量级；
  GPU 时间几乎全部在 pass 执行，**上传本身在 GPU 侧不是大头**。
- **OIT resolve 占 GPU pass 时间的 ~65%** 且随 item 数/重叠度增长——如果要降
  GPU 消耗，这里比上传路径的杠杆大一个数量级。
- 全帧时间从 400→3600 items 几乎不变（0.51→0.61 ms），瓶颈在固定分辨率的
  resolve/填充率，不在几何量。

## 建议路线（按性价比排序）

1. **删除 points2d 的每帧零上传**（省 24% 上传字节，零风险）：compute pass
   每帧无条件重写该 buffer；初始化可交给 `clear_buffer`（GPU 侧零填充）或
   干脆依赖 compute 覆写语义。
2. **clip_boxes 初始化移到 GPU 侧**（再省 3.3%，并解锁 static≈0 上传）：
   当前每帧从 CPU 上传 `[MAX,MIN,MAX,MIN,0]` pattern；可改为一个
   1 workgroup/item 的 init compute，或改用可 `clear_buffer` 的 sentinel
   语义（如存 0xFFFF.. 表示空）。
3. **把 ECS change detection 接到上传路径**（结构性方案）：`reconcile` 已经
   只在 `PartialEq` 判变化时才 `replace_component`，但 `prepare_vitems/
   prepare_mesh_items` 目前对全量 query 收集并上传。让 prepare 感知
   `Changed<VItem>`，配合**稳定的 item→slot 映射**（拓扑不变时槽位保持），
   就能做 per-item 的 `write_buffer(offset..)` 部分上传——这是 morph 类场景
   （上限 67.6% 冗余）能吃到的大部分收益。
4. **preview idle 路径加 buffer 级 skip-equal**（本 worktree 原型已验证）：
   preview 已有"时间未变则不重渲染"的短路，但一旦触发重渲染就是全量重传；
   字节级 skip 只需一次 memcmp（<10 μs @ 2 MiB）+ 一份 shadow。
   dirty-ranges 在实测场景中无额外收益，不建议投入。
5. 上传侧不是当前 GPU 瓶颈；若目标是降 GPU 消耗/帧耗时，优先看
   **OIT resolve**（每像素节点数、layer 数、resolve 的 fragment 开销）。

## Side findings

- `SealedRanimScene::eval_at_alpha` **是幂等的**（已验证：重复同 alpha、
  前进后重复同一时刻、来回 seek 共 7 种模式，static/morph 两类场景，401 个
  item 逐位一致）。ranim-core 源码注释所称的 "stateful segments reset/replay
  or integrate internally" 是 segment 内部为回答任意时刻查询所做的状态管理，
  对外是纯查询。早期一轮 profiling 曾把它误判为"不幂等"，实际是 bench
  代码自身的 bug，见下一条。
- 写逐 id diff 的 bench 时注意：`for ((id, _), item)` 解构拿到的是
  **anim_id**（`usize`），不是完整 `(anim_id, part)`；`find` 若按它匹配，
  同一动画的所有 part 会全部命中第一项（400 个方块共享 anim_id=0，
  全被误判为每帧 changed，且症状酷似"求值状态漂移"）。必须比较完整元组。
