# Ranim

https://github.com/user-attachments/assets/5b68c84d-df33-4bb4-bd98-f128c0f013fe

> [basic.mp4](./assets/basic.mp4)

Ranim is an animation engine crate implemented in pure rust, inspired by [Manim](https://github.com/3b1b/manim/tree/master).

It is now just a pure rust crate, but in the future, there are many possibilities:
- It may support interactive scene, like manim's `interact` mode.
- It may support web through wasm, and make scenes renderable and interactable in browser.
- It may support python through pyo3, and offers the same api as manim to render manim's scenes directly.
- ......

> [!WARNING]
> Currently, the project is WIP. It only supports some basic *mobjects* and *animations*, the apis are unstable and may change frequently, the documentation is also not complete.

## Dependencies

runtime dependencies:
- ffmpeg: ranim spawns a ffmpeg process to encode videos

## Installation

Currently, it is not published to crates.io, but you can add it to your `Cargo.toml`'s `[dependencies]` section with git url:

```toml
ranim = { git = "https://github.com/azurice/ranim" }
```

For the usage, check out the [examples](./examples) folder. You can run the examples with:

```bash
cargo run --example <example-name>
```

and you can use `--release` flag for faster rendering.
