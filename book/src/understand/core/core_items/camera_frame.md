# Core `CameraFrame`

相机数据，定义于 `ranim_core::core_item::camera_frame`。它同时携带正交与透视
两套投影参数，由 `perspective_blend` 在二者之间混合。

```rust,ignore
pub struct CameraFrame {
    pub pos: DVec3,              // 位置
    pub up: DVec3,               // 上方向单位向量
    pub facing: DVec3,           // 朝向单位向量
    pub near: f64,               // 近平面（far > near）
    pub far: f64,                // 远平面
    pub perspective_blend: f64,  // 正交(0.0) ↔ 透视(1.0) 混合
    pub frame_height: f64,       // 正交：视野高度
    pub scale: f64,              // 正交：缩放系数
    pub fovy: f64,               // 透视：纵向视场角（弧度）
}
```

默认值（`CameraFrame::default()`）：位于原点、朝 `-Z`、`+Y` 为上；
`perspective_blend = 0.0`（纯 2D 正交）；`frame_height = 8.0`；
`near = -1000`、`far = 1000`；`fovy = π/2`。2D 场景用默认值即可。

## 投影

```rust,ignore
let view = cam.view_matrix();                       // look_to(pos, facing, up)
let proj = cam.projection_matrix(aspect_ratio);
//   = orthographic_mat(aspect).lerp(perspective_mat(aspect), perspective_blend)
```

正交矩阵由 `frame_height * scale` 与宽高比推出；透视矩阵使用 `fovy`，且
`near` 会被钳到至少 `0.1`。`perspective_blend` 取中间值时两矩阵逐元素插值，
可用于「2D 场景平滑进入 3D 透视」的运镜（见
`examples/perspective_blend`）。

## 3D 定位

```rust,ignore
// 球坐标定位（Z-up），看向原点；perspective_blend 自动设为 1.0
let cam = CameraFrame::from_spherical(phi, theta, distance);
// phi：与 +Z 的极角（0 = 正上方，π/2 = XY 平面）
// theta：方位角（0 = +X，π/2 = +Y）

// 或围绕任意目标点：
cam.set_spherical(phi, theta, distance, target);
cam.look_at(target); // 只改朝向
```

注意 `from_spherical` / `set_spherical` 把 `up` 固定为 `+Z`。

## 其他

- `set_view_matrix` / `with_view_matrix`：从视图矩阵反解 `pos` / `up` /
  `facing`。
- `center_canvas_in_frame(center, width, height, up, normal, aspect_ratio)`：
  透视模式下调整相机位置，使给定矩形画布恰好充满画面。
- `CameraFrame` 实现了 `Interpolatable`，可以直接用 `morph` 做运镜动画。
