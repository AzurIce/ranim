# Ranim Cli

使用 Ranim Cli 可以更方便地进行场景地预览、输出属性等的定义：

```rust,ignore
#[scene]

#[output]
pub fn scene_constructor1(r: &mut RanimScene) {
    // ...
}

#[scene(frame_height = 8.0, name = "custom")]

#[output(width = 1920, height = 1080, frame_rate = 60, save_frames = false, dir = "output")]
pub fn scene_constructor2(r: &mut RanimScene) {
    // ...
}
```

同时，不必再编写 `main.rs` 来手动调用渲染或预览 api，直接通过 cli 命令即可完成场景的预览或渲染（而且预览支持热重载）：
- `ranim preview`：调用 Cargo 构建指定的 lib，然后启动一个预览应用加载编译出的 dylib，并监听改动进行重载。
- `ranim render`：调用 Cargo 构建指定的 lib，然后加载它并渲染动画。

但是，要注意为你的 lib 添加 `crate-type = ["dylib"]` 来使得它能被编译为动态库。