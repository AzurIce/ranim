<div align="center">
<img alt="Ranim Logo" src="./assets/ranim.png" width="200" height="200" />

# Ranim
<div>
    <img alt="license" src="https://img.shields.io/github/license/AzurIce/ranim" />
    <img alt="crates.io" src="https://img.shields.io/crates/v/ranim.svg" />
    <img alt="commit" src="https://img.shields.io/github/commit-activity/m/AzurIce/ranim?color=%23ff69b4">
</div>
<div>
    <img alt="build check" src="https://github.com/AzurIce/ranim/actions/workflows/build.yml/badge.svg" />
    <img alg="website check" src="https://github.com/AzurIce/ranim/actions/workflows/website.yml/badge.svg" />
</div>
<div>
    <img alt="stars" src="https://img.shields.io/github/stars/AzurIce/ranim?style=social">
</div>
</div>

<div style="display: flex;">
    <img alt="getting_started3" src="./assets/getting_started3.gif" width="48%" />
    <img alt="ranim_logo" src="./assets/ranim_logo.gif" width="48%" />
</div>

> [examples/getting_started3](./examples/getting_started3)
> 
> [examples/ranim_logo](./examples/ranim_logo)

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
