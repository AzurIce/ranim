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
    <img alt="hello_ranim" src="./assets/hello_ranim.gif" width="48%" />
    <img alt="ranim_logo" src="./assets/ranim_logo.gif" width="48%" />
</div>

> [examples/hello_ranim](./examples/hello_ranim)
> 
> [examples/ranim_logo](./examples/ranim_logo)

Ranim is an animation engine crate implemented in pure rust, inspired heavily by [Manim](https://github.com/3b1b/manim/tree/master) and [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim).

> [!WARNING]
> Ranim is now WIP. It only supports some basic *items* and *animations*, the apis are unstable and may change frequently, the documentation is also not complete.

Related:
- [AzurIce/ranim-doc](https://github.com/AzurIce/ranim-doc): rustdoc for main
- [AzurIce/ranim-book](https://github.com/AzurIce/ranim-book): ranim book
- [AzurIce/ranim-one-shot](https://github.com/AzurIce/ranim-one-shot): one-shot attempts with ranim
- [AzurIce/ranim-bench](https://github.com/AzurIce/ranim-bench): benchmarks for ranim

## Dependencies

Runtime dependencies:
- ffmpeg: encode videos
  Ranim will try to use ffmpeg from system's path environment variable first, if no ffmpeg is found, ranim will automatic download ffmpeg to the current working dir.

## Installation

Currently, it is experimental on crates.io:

```toml
[dependencies]
ranim = "0.2.1"
```

You can also use from git for the latest updates:

```toml
[dependencies]
ranim = { git = "https://github.com/azurice/ranim" }
```

For the usage, check out the [examples](./examples) folder. You can run the examples with:

```bash
cargo run -p ranim-cli --release -- preview --example <example-name> # run a preview app for the example
cargo run -p ranim-cli --release -- output --example <example-name> # render every declared output of the example
```

See Ranim Cli for more.

### Ranim Cli

Ranim cli is a command line tool to help you build the animation. It enables animation previewing with hot reload through dylib.

Please notice that you can use ranim without ranim-cli, the previewing and rendering process can be achieved by simply invoking apis provided by *ranim*, but it may enhance your experience.

You can install it with:

```bash
cargo install ranim-cli
```

Or install from git:

```bash
cargo install --git https://github.com/azurice/ranim ranim-cli --locked
```

Or you can create a bin, and run cli from `ranim_cli` directly (this can make sure the version of `ranim` matches `ranim_cli`):

```rust
fn main() {
    ranim_cli::main();
}
```

Basic Usage:
- `ranim preview[ <scene_name>]`: Launch a preview app and invoke cargo to build your library automatically when the source code is changed, then reload it through *libloading* and show it in the preview app.
- `ranim output[ <scene-name1> <scene_name2> ...]`: Render every `#[output(...)]` declared by the selected scenes, including capture marks. When no scene name is specified, render all scenes.
- `ranim render <scene_name>`: Render one scene once with default output settings (`1920x1080`, 60 fps, mp4). This is the low-level/ad-hoc render command, ignores `#[output(...)]`, and does not process capture marks.
- `ranim inspect scenes [--format text|json]`: List available scenes and their output summaries.
- `ranim inspect tree [<scene_name>] [--format text|json]`: Build a scene and print its animation tree. The scene name can be omitted when there is only one scene.
- `ranim inspect frame <scene_name> --at <sec> [--format text|json] [--verbose]`: Sample a scene at a time and print the evaluated frame items.

You can specify the package with `--package` and `--example` (just like cargo, note that your anim target should have crate-type of `dylib` or `cdylib`), and other aditional arguments you want to pass to `cargo build` can be passed after `--`.

For example:

```bash
ranim output -p render scene_a scene_b -- --release
ranim render -p render scene_a -- --release
```

## Feature Flags

- Default features

  - `anims`: re-export `ranim-anims`, default on.

  - `items`: re-export `ranim-items`, default on.

- `render`: enable render api in cmd module
  
  use `render_scene` or `render_scene_output` to render scene to output

- `preview`: enbale preview api in cmd module

  use `preview_scene` api to launch an preview app on a scene
  https://github.com/user-attachments/assets/5bf287e2-b06f-42f8-83b6-76f3775e298e
- `profiling`: enable profiling with https://github.com/EmbarkStudios/puffin

  CPU uses `127.0.0.1:8585` and GPU uses `127.0.0.1:8586`
  
  ![image](https://github.com/user-attachments/assets/36bf841c-e30f-45cc-adbc-bd4bfff9bc4c)
   

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## Design

Once the design is stablized, I may write about it.

For now, you can check out the code.

## Aknowledgements

- [3b1b/manim](https://github.com/3b1b/manim)
- [ManimCommunity/manim](https://github.com/ManimCommunity/manim/)
- [jkjkil4/JAnim](https://github.com/jkjkil4/JAnim)

## Star History

[![Star History Chart](https://star-history.dera.page/svg?repos=AzurIce/ranim&type=Date)](https://star-history.dera.page/#AzurIce/ranim&Date)
