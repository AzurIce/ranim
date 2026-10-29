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

```toml
[dependencies]
ranim = "0.2.1"
```

You can also use from git for the latest updates:

```toml
[dependencies]
ranim = { git = "https://github.com/azurice/ranim" }
```

### Scenes, project assembly, and platform entry points

v0.3(WIP, main) has a huge change on the usage paradigm of ranim.

**Scenes are still ordinary Rust functions and can live in a library.**
Rendering and preview run through a host that explicitly assembles an `App`:

- **Scene library**: defines animations and exposes scene descriptors or optional plugins.
- **Native binary**: registers scenes in an `App`, then calls `run_cli(app.build())`.
- **WASM binary**: performs the same assembly, but exports
  `app() -> ranim::wasm::WasmApp` for JavaScript instead of running a CLI.

For example, a package named `my-animation` can have both `src/lib.rs` and
`src/main.rs`. The following examples describe the current git API:

```toml
# Cargo.toml
[dependencies]
ranim = { git = "https://github.com/AzurIce/ranim" }

# Needed when exporting app() from the WASM host.
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2.129"
```

The default features include built-in items, animations, and `cmd-preview`.
For native export without the interactive UI, use `default-features = false`
and `features = ["anims", "cmd-render"]`. The `render` feature alone provides
the renderer and WASM canvas host, with no native commands or built-in items.

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

The `#[scene]` macro preserves `hello(&mut RanimScene)` and generates
`hello::scene() -> Scene`; `pub` makes both available to the binary or other
crates. It does **not** register the scene globally. Only descriptors added to
the host's `App` are available to its CLI or browser sessions. The host can
select scenes from several libraries without moving their implementations.

`ranim::App` is a re-export of `bevy_app::App`. `AppExt` supplies the scene
registration/selection methods and `RanimAppExt` supplies `build()`; both traits
are exported by the prelude.

`app.build()` is the explicit "finalize, then serve" step from a building
app to a session-ready project: it asserts the plugins are ready, runs
bevy's plugin `finish()` and `cleanup()` exactly once, and returns a
`ranim::RanimApp`. `RanimApp` is the session-time face of the project — it
dereferences to the bevy `App`, so scene selection, resource access, and
render sub-app installation keep working through it, but it is no longer a
plugin-construction target. `run_cli`, `preview_scene`, and `WasmApp::new`
take a `RanimApp`; offline render workers take a selected `Scene` and the
project's `RenderSetup`.

Plugins are optional: a library can expose a `ScenesPlugin` that registers
its scenes, and the binary installs that plugin instead of individual
descriptors:

```rust
let mut app = App::new();
app.add_plugins(my_animation::ScenesPlugin);
run_cli(app.build());
```

On native, use `cargo run -- preview hello` or `cargo run -- output hello`.
On WASM, the empty `main()` does not start an application: after wasm-bindgen
initialization, JavaScript calls the exported `app()`, then
`await project.createPreview(canvas, "hello")` and `preview.renderAt(seconds)`.
The browser owns the UI and playback clock. Compile the same binary with
`cargo build --bin my-animation --target wasm32-unknown-unknown`.
See the Book's [project entry chapter](./book/project-entry.md) for the complete
Cargo setup, native commands, WASM bindings, and the library/binary boundary.

Rendering needs no boilerplate: when the project does not register a
`ranim::render::RenderSetup`, rendering and preview fall back to a default
configuration that installs the built-in camera, vector-item, and mesh-item
plugins (`ranim::items::RanimItemsPlugin`) when `items` is enabled. Without
`items`, the default setup is empty; register a `RenderSetup` for custom types.

`Plugin` — bevy's plugin trait, re-exported as `ranim::Plugin` — is an
optional mechanism for organizing scenes and a custom render setup. A plugin
installs scenes, resources, or further plugins in
`fn build(&self, app: &mut bevy_app::App)`; because `ranim::App` is the same
type as `bevy_app::App`, writing `&mut App` there is equivalent. The `AppExt`
trait, exported by the prelude, provides the scene methods (`add_scene` /
`add_scenes` / `scene` / `first_scene`) on the app. `app.add_plugins(...)`
also accepts tuples, so a project
can compose scene collections and configuration from smaller pieces. Project
resources are plain bevy resources: store them with `app.insert_resource(...)`
(the type derives `ranim::Resource`) and read them back through `app.world()`.

#### Experimental scene hot reload

Enable the optional `subsecond` feature alongside `cmd-preview` (included in
the defaults), then run your project with Dioxus CLI 0.7.10:

```toml
[dependencies]
ranim = { git = "https://github.com/AzurIce/ranim", features = ["subsecond"] }
```

```sh
cargo install dioxus-cli --version 0.7.10 --locked
dx serve --hotpatch --desktop --args preview
```

For this repository's examples, use `dx` from `nix develop`: the flake pins
upstream Dioxus commit `7bd964ae9469daafcb76b73556162d9082be255b`, which includes
the workspace cascade fix missing from dx 0.7.10. The released CLI can fail
with `Missing rustc args for replay: 'app'` by including unrelated reverse
dependencies in its patch rebuild. The examples remain ordinary Cargo examples:

```sh
dx serve --hotpatch --desktop --example hello_ranim --features subsecond --args preview
```

The pinned main build still needs end-to-end validation with Ranim's 0.7
Subsecond runtime. Ordinary `cargo run --example hello_ranim -- preview` is
unaffected.

Scene constructors registered with `Scene::new` (including `#[scene]` scenes)
are hot-call boundaries. After a patch, native preview rebuilds the selected
scene, its timeline, and audio, then pauses at the previous time (clamped to
the new duration). The window, GPU device, resolution, and playback settings
survive. Rebuilds are deferred until an active export finishes; a constructor
panic keeps the previous scene paused and logs an error. Constructors should
be repeatable: their external side effects are not rolled back.

This is **scene reconstruction, not state migration**. Restart preview after
changing plugins, registration, scene metadata (`#[scene]` / `#[output]`),
captured configuration, dependencies, features, or type/capture layouts.
Scene libraries remain supported; hotpatchability is a separate, experimental
tooling constraint, not a requirement to move scenes into `main.rs`.
The tested dx 0.7.10 workflow updates scenes in the binary's own modules;
patching library/workspace crates depends on dx's workspace support. The pinned
main CLI contains the cascade fix but has not yet been validated end to end
here. Helper functions can also update when reached through a hot-call boundary.
The feature does not enable Bevy system hotpatching or WASM hot reload, and
does not replace compilation with dx. Ordinary `cargo run` still works without
a devserver.

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
    fn build(&self, app: &mut bevy_app::App) {
        app.add_scenes([hello::scene()]);
        app.insert_resource(RenderSetup::new(|render_app| {
            render_app.add_plugins((RanimItemsPlugin, ExtractPlugin::<MyItem>::default()));
        }));
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(ProjectPlugin);
    run_cli(app.build());
}
```

`RenderSetup` is a bevy `Resource` (and itself a non-unique bevy `Plugin`) that
declares the project's rendering assembly and is replayed for each
GPU runtime (output rendering, preview, preview resolution changes).
`RanimItemsPlugin` bundles the three built-in item plugins
(`CameraFramePlugin`, `VItemPlugin`, `MeshItemPlugin`) — the default
configuration installs this plugin when `items` is enabled. Without `items`,
the default setup is empty; custom render plugins can be registered through
`RenderSetup` without depending on built-in items. Do not combine it with the individual
plugins: bevy plugins are unique, and installing one twice panics.

To drive the APIs directly instead of the command line, select a scene and
call the corresponding function (`cmd-render` / `cmd-preview` features enable them):

```rust
let mut app = App::new();
app.add_scenes([hello::scene()]);
let scene = app.scene("hello").expect("hello is registered");
let setup = app
    .world()
    .get_resource::<ranim::render::RenderSetup>()
    .cloned()
    .unwrap_or_else(|| {
        ranim::render::RenderSetup::new(|render_app| {
            render_app.add_plugins(ranim::items::RanimItemsPlugin);
        })
    });
ranim::cmd::render_scene(&scene, &setup, 2);
ranim::cmd::preview_scene(app.build(), scene);
```

`app.scene(name)` returns a cloned scene descriptor; the preview app takes
ownership of the finalized project (`RanimApp`) so project resources stay
available through rendering, preview resizing, and export.

Each example has one `[[example]]` binary target for native and WASM.
`main.rs` calls the CLI on native; on WASM it provides an empty `main()` and
exports `app() -> ranim::wasm::WasmApp` directly.
Both platforms share scenes and rendering setup through `scenes.rs`.
wasm-bindgen runs the empty WASM main during module initialization; application
construction and rendering remain explicitly driven by JavaScript. JavaScript
initializes the module and calls `await app().createPreview(canvas, name)`.
The returned session renders directly to the canvas with `renderAt(seconds)`;
Web UI and playback scheduling stay in JavaScript. The shared
[`<ranim-viewport>` and `<ranim-controls>` components](./website/README.md)
separate rendering from playback controls for the website and rustdoc. Controls
can bind to any viewport by ID or a JavaScript reference, and expand the progress
bar into an interactive animation timeline. Scenes with sound play through Web
Audio, with the visual playhead following the audio clock across pause and seeks.
egui/eframe are only
used by the native preview.

```bash
cargo build --example arc --target wasm32-unknown-unknown --no-default-features --features anims,render
cargo examples build arc  # wasm-bindgen + wasm-opt, ready for the website
```

Inside `nix develop`, run `CARGO_BUILD_JOBS=4 just website-preview arc` to build
the example and serve Notist SSG at `http://127.0.0.1:8000/examples/arc.html`.
Select **App** for the live canvas player. See the
[website build/preview workflow](./xtasks/xtask-examples/README.md#notist-ssg).

See [examples](./examples/README.md) for the layout and build commands.
The [example app](./example-packages/app) demonstrates a native binary host.

### Command line interface

`run_cli(app.build())` (native targets with the `cmd-render` feature) installs tracing,
parses the process arguments, and serves four subcommands — its name stays
clear of the bevy `App::run()` runner API:

- `preview [SCENE]`: Launch the interactive preview app. Requires the `cmd-preview`
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
cargo run --example <example-name> -- preview # run a preview app for the example
cargo run --example <example-name> --no-default-features --features anims,cmd-render -- output  # render every declared output of the example
```

Examples behind `required-features` (e.g. `typst`, `gltf`) also need the
corresponding feature.

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

`render` forwards `ranim-items?/render`: built-in extraction plugins are compiled
only when items are also enabled. Custom render projects need no built-in items.
`--no-default-features` leaves core scene evaluation and project assembly.
`audio-decode` adds `AudioClip::from_file` / `from_bytes` for WAV, MP3, FLAC,
AAC/M4A, and Ogg Vorbis; PCM synthesis, mixing, and playback need no decoder.
Native command dependencies are target-gated and are not compiled for WASM.
The previous `preview` feature has been removed; use `cmd-preview`. Native CLI
and export users of the former `render` bundle must select `cmd-render`.

The native preview app has a built-in profiler panel (top bar `Profiler` button):
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
