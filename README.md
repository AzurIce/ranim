# Ranim

https://github.com/user-attachments/assets/8249f9dd-bce7-439b-b2cc-31c6db2c6025

> [basic.mp4](./assets/basic.mp4)

Ranim is an animation engine crate implemented in pure rust, inspired by [Manim](https://github.com/3b1b/manim/tree/master).

> [!WARNING]
> Currently, the project is WIP. It only supports some basic *mobjects* and *animations*, the apis are unstable and may change frequently, the documentation is also not complete.

## Dependencies

Runtime dependencies:
- typst: for fonts and maths rendering
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

## Design

Once the design is stablized, I may write about it.

For now, you can check out the code and some PRs:
- [PR#5: Refactor object management and logic/rendering phases](https://github.com/AzurIce/ranim/pull/5)

## Thanks

- [3b1b/manim](https://github.com/3b1b/manim)
- [ManimCommunity/manim](https://github.com/ManimCommunity/manim/)
- [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim)