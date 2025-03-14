<div align="center">
    <img alt="Ranim Logo" src="./assets/ranim.png" width="256" height="256" />
</div>

# Ranim

![Licence](https://img.shields.io/github/license/AzurIce/ranim)
![crates.io](https://img.shields.io/crates/v/ranim.svg)
![Check](https://github.com/AzurIce/ranim/actions/workflows/build.yml/badge.svg)
![Website](https://github.com/AzurIce/ranim/actions/workflows/website.yml/badge.svg)

https://github.com/user-attachments/assets/2176093e-758b-429b-89e0-2e3dd39b8a17

> [hello_ranim.mp4](./assets/hello_ranim.mp4)

Ranim is an animation engine crate implemented in pure rust, inspired heavily by [Manim](https://github.com/3b1b/manim/tree/master) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).

> [!WARNING]
> Ranim is now WIP. It only supports some basic *items* and *animations*, the apis are unstable and may change frequently, the documentation is also not complete.

## Dependencies

Runtime dependencies:
- typst: fonts and maths rendering
- ffmpeg: encode videos

## Installation

Currently, it is experimental on crates.io:

```toml
[dependencies]
ranim = "0.1.0-alpha.1"
```

You can also use from git for the latest updates:

```toml
[dependencies]
ranim = { git = "https://github.com/azurice/ranim" }
```

For the usage, check out the [examples](./examples) folder. You can run the examples with:

```bash
cargo run --example <example-name>
```

and you can use `--release` flag for faster rendering.

## Design

Once the design is stablized, I may write about it.

For now, you can check out the code.

## Aknowledgements

- [3b1b/manim](https://github.com/3b1b/manim)
- [ManimCommunity/manim](https://github.com/ManimCommunity/manim/)
- [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim)
