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
ranim = { git = "https://github.com/azurice/ranim", features = ["render"] }
```

### Project entry and plugins

A Ranim project is its own binary crate. `main` assembles an `App` with the
project's scenes, then hands over to the command line interface through
`App::run()`:

```toml
# Cargo.toml
[dependencies]
ranim = { version = "0.3", features = ["preview"] }
```

The `preview` feature implies `render`; depend on `features = ["render"]` alone
when the interactive preview is not needed.

```rust
// src/main.rs
use ranim::{items::vitem::geometry::Square, prelude::*};

#[scene]
fn hello(scene: &mut RanimScene) {
    scene.play(CameraFrame::default().show().with_duration(2.0));
    scene.play(Square::new(2.0).show().with_duration(2.0));
}

fn main() {
    App::new().add_scenes([hello::scene()]).run();
}
```

`#[scene]` creates an owned descriptor through `hello::scene()`; add it with
`add_scene` or `add_scenes` to make it available. Only registered scenes appear
in the command line interface.

Rendering needs no boilerplate: when the project does not register a
`ranim::render::RenderSetup`, rendering and preview fall back to a default
configuration that installs the built-in camera, vector-item, and mesh-item
plugins (`ranim::items::RanimItemsPlugin`).

`Plugin` is an optional mechanism for organizing scenes and a custom render
setup. A plugin installs scenes, resources, or further plugins in
`build(&self, app)`; `app.add_plugins(...)` also accepts tuples, so a project
can compose scene collections and configuration from smaller pieces.

#### Custom render setup

Register a `RenderSetup` when the project renders more than the types covered
by the built-in plugins — composite outputs such as `Vec<Circle>` or
`Transformed<T, G>`, or custom types, need an `ExtractPlugin<T>` for the exact
frame type:

```rust
use ranim::{
    items::RanimItemsPlugin,
    prelude::*,
    render::{RenderSetup, extract::ExtractPlugin},
};

struct ProjectPlugin;

impl Plugin for ProjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_scenes([hello::scene()]);
        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((RanimItemsPlugin, ExtractPlugin::<MyItem>::default()));
        }));
    }
}

fn main() {
    App::new().add_plugins(ProjectPlugin).run();
}
```

`RenderSetup` declares the project's rendering assembly and is replayed for each
GPU runtime (output rendering, preview, preview resolution changes).
`RanimItemsPlugin` bundles the three built-in item plugins
(`CameraFramePlugin`, `VItemPlugin`, `MeshItemPlugin`) — the default
configuration is exactly this plugin. Do not combine it with the individual
plugins: bevy plugins are unique, and installing one twice panics.

To drive the APIs directly instead of the command line, select a scene handle
and call the corresponding function (`render` / `preview` features enable them):

```rust,ignore
let mut app = App::new();
app.add_scenes([hello::scene()]);
let project = std::sync::Arc::new(app);
let scene = project.scene("hello").expect("hello is registered");
ranim::cmd::render_scene(&scene, 2);
ranim::cmd::preview_scene(scene);
```

A `SceneHandle` keeps its App and project resources alive through rendering,
preview resizing, and export.

For a WebAssembly preview, the same binary constructs the App and starts the
preview directly: `AppRunExt::run` is native-only, so select the scene and call
`ranim::cmd::preview_scene` from `main`. Build the binary for
`wasm32-unknown-unknown` and process its `.wasm` with `wasm-bindgen`. See the
[example app](./example-packages/app) for both Trunk and manual builds.

### Command line interface

`App::run()` (the `ranim::cmd::AppRunExt` trait, re-exported by the prelude)
parses the process arguments and serves four subcommands:

- `preview [SCENE]`: Launch the interactive preview app. Requires the `preview`
  feature; defaults to the first scene.
- `output [SCENES...] [--buffer-count N]`: Render every `#[output(...)]`
  declared by the selected scenes, including capture marks. When no scene name
  is specified, render all scenes.
- `render <SCENE> [--buffer-count N]`: Render one scene once with default output
  settings (`1920x1080`, 60 fps, mp4). This is the low-level/ad-hoc render
  command: it ignores `#[output(...)]` and does not process capture marks.
- `inspect scenes [--format text|json]`: List available scenes and their output
  summaries.
- `inspect tree [SCENE] [--format text|json]`: Build a scene and print its
  animation tree. The scene name can be omitted when there is only one scene.
- `inspect frame <SCENE> --at <SEC> [--format text|json] [--verbose]`: Sample a
  scene at a time and print the evaluated frame items.

```bash
cargo run -- preview
cargo run -- output hello
cargo run -- render hello
cargo run -- inspect frame hello --at 1.5
```

For the usage, check out the [examples](./examples) folder. You can run the examples with:

```bash
cargo run --example <example-name> --features preview -- preview # run a preview app for the example
cargo run --example <example-name> --features preview -- output  # render every declared output of the example
```

Examples behind `required-features` (e.g. `typst`, `gltf`) also need the
corresponding feature.

## Feature Flags

- Default features

  - `anims`: re-export `ranim-anims`, default on.

  - `items`: re-export `ranim-items`, default on.

- `render`: enable the render api and the command line interface in the cmd module
  
  use `render_scene` or `render_scene_output` to render scene to output, or call
  `App::run()` from the project binary

- `preview`: enable preview api in cmd module (implies `render`)

  use `preview_scene` api to launch an preview app on a scene
  https://github.com/user-attachments/assets/5bf287e2-b06f-42f8-83b6-76f3775e298e
- `audio-decode`: enable audio file decoding (`AudioClip::from_file` / `from_bytes`)

  pure-Rust decoding via symphonia (WAV, MP3, FLAC, AAC/M4A, Ogg Vorbis) with rubato
  resampling to the 48 kHz master rate — no ffmpeg binary required
- `profiling`: enable GPU timer scopes (wgpu-profiler) for the preview profiler panel

The preview app has a built-in profiler panel (top bar `Profiler` button):
a progress-axis chart of GPU pass times / CPU spans / buffer-upload stats
with click-to-seek. GPU timer scopes (wgpu-profiler) are runtime-toggled
in the panel or via `RANIM_PROFILE_GPU=1`; upload tracking via
`RANIM_PROFILE_UPLOAD=1`. While GPU timers are enabled, each frame pays a
device poll.

![profiler panel](https://github.com/user-attachments/assets/746d044c-8fe8-44d8-a0dc-ee00eceb9284)

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
