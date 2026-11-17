# Examples

均为 `bin` target：
- native：`ranim::cmd::run_cli(app.build()).unwrap()` 运行注册了对应 Scene 的 App。
- wasm：空 main，导出 `app() -> WasmApp` 符号。

`ranim_examples_bundle` 用于 `ranim-examples` 收集 examples，
主要两个用途：
- 集中的 cargo doc 文档
- 集中的 `run_app` 应用
