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

### Published release: v0.2.1

```toml
[dependencies]
ranim = "0.2.1"
```

Documentation:
[v0.2.1 guide](https://github.com/AzurIce/ranim/blob/v0.2.1/README.md) and
[API reference](https://docs.rs/ranim/0.2.1/ranim/).

### Development: v0.3 (unreleased)

```toml
[dependencies]
ranim = { git = "https://github.com/AzurIce/ranim" }
```

The following examples target the v0.3 App/Plugin API.

### Project entry

Define scenes as Rust functions in a library or module, then register their
descriptors in an `App`. Native hosts run the CLI; WASM hosts export the project
to JavaScript. For a package named `my-animation`:

```toml
# Cargo.toml
[dependencies]
ranim = { git = "https://github.com/AzurIce/ranim" }

# Needed when exporting app() from the WASM host.
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2.129"
```

```rust
// src/lib.rs — reusable scene definition; no CLI, GPU, or browser startup.
use ranim::{items::vitem::geometry::Square, prelude::*};

#[scene]
pub fn hello(scene: &mut RanimScene) {
    scene.play(CameraFrame::default().show().with_duration(2.0));
    scene.play(Square::new(2.0).show().with_duration(2.0));
}
```

```rust
// src/main.rs — the same binary target serves native and WASM.
use ranim::prelude::*;

fn project() -> ranim::RanimApp {
    let mut app = App::new();
    app.add_scenes([my_animation::hello::scene()]);
    app.build()
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run_cli(project());
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn app() -> ranim::wasm::WasmApp {
    ranim::wasm::WasmApp::new(project())
}
```

`#[scene]` generates `hello::scene()` for explicit registration.
`App` and `Plugin` are Bevy types. Plugins optionally group scenes, resources,
and render configuration; `app.build()` finalizes the project for either host.
See [project entry](./book/project-entry.md) for setup and platform bindings.

#### Experimental scene hot reload

Enable `subsecond` with `cmd-preview` and use Dioxus CLI:

```sh
dx serve --hotpatch --desktop --example hello_ranim --features subsecond --args preview
```

Patches rebuild the scene and pause at the previous time, preserving the
window/GPU. This is **scene reconstruction, not state migration**; structural
changes require a restart. Native only. The flake pins a dx workspace fix;
that CLI/runtime combination awaits end-to-end validation. See
[limitations](./book/project-entry.md#热重载不会改变场景库与宿主的边界).

#### Custom render setup

Built-in camera, vector, and mesh rendering needs no setup when `items` is
enabled. For custom types or composite outputs, register a `RenderSetup` with
an `ExtractPlugin<T>` matching the complete frame type. See
[render extraction](./book/understand/core/extract.md).

### Web examples

Examples share native/WASM entries in one binary target. Browser UI and playback
use independent [viewport and controls components](./website/README.md), with
an expandable timeline and synchronized Web Audio. Native preview uses egui.

```bash
cargo examples build arc  # WASM + bindings + optimization
CARGO_BUILD_JOBS=4 just website-preview arc  # build and serve locally
```

Run inside `nix develop`, then open `http://127.0.0.1:8000/examples/arc.html`
and select **App**. See [examples](./examples/README.md) for layout and commands.

### Command line interface

| Command | Purpose |
| --- | --- |
| `preview [SCENE]` | Interactive native preview; defaults to the first scene |
| `output [SCENES...]` | Render declared outputs and capture marks; defaults to all scenes |
| `render <SCENE>` | One default-settings export, ignoring declared outputs and captures |
| `inspect scenes\|tree\|frame` | Inspect registration, animation trees, or evaluated items |

```bash
cargo run -- preview hello
cargo run -- output hello
cargo run -- inspect frame hello --at 1.5
cargo run --example hello_ranim -- preview  # repository example
```

See [CLI reference](./book/cli.md) for options. Some examples require `typst` or
`gltf`; native export without the UI uses `--no-default-features --features anims,cmd-render`.

## Feature Flags

Defaults are `anims`, `items`, and `cmd-preview`.

| Feature | Capability |
| --- | --- |
| `items` | Built-in item types and their re-export; optional dependency |
| `anims` | Built-in animations and their re-export; enables `items` |
| `render` | GPU rendering API and WASM canvas host; independent of built-in items and native commands |
| `cmd-render` | Native CLI (`output`, `render`, `inspect`) and offline export; enables `render` |
| `cmd-preview` | Native egui preview and PCM playback, including the export panel; enables `cmd-render` |
| `typst` | Typst text support; enables `items` |
| `gltf` | glTF import; enables `items` |
| `audio-decode` | Audio file decoding with symphonia and resampling with rubato |
| `subsecond` | Experimental native scene hot reload with Dioxus CLI |

`--no-default-features` keeps core evaluation and project assembly. Native command
dependencies are excluded from WASM.

The native **Profiler** panel shows GPU passes, CPU spans, and upload stats.
Enable GPU timestamps with `RANIM_PROFILE_GPU=1` (adds a per-frame device poll),
or upload tracking with `RANIM_PROFILE_UPLOAD=1`.

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
