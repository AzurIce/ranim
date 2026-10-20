# App example

`src/lib.rs` defines `hello_ranim` and its scene-only `ScenesPlugin`.
This scene emits bare `CameraFrame`, `Square`, and `VItem` values, which the
built-in `RanimItemsPlugin` (camera frame, vector, and mesh plugins) already
covers, so the project stores no custom `ranim::render::RenderSetup`: projects
without one fall back to those built-in plugins for each new GPU renderer,
including resolution changes.

`src/main.rs` is the project binary: it assembles the App with
`app::ScenesPlugin` and hands over to `ranim::cmd::AppRunExt::run`, which
serves the `preview`, `output`, `render`, and `inspect` subcommands.

## Run

From the repository root:

```sh
cargo run -p app -- preview            # interactive preview (defaults to the first scene)
cargo run -p app -- preview hello_ranim
cargo run -p app -- render hello_ranim # render once at 1920x1080 60fps mp4
cargo run -p app -- output             # render every #[output(...)] declaration
cargo run -p app -- inspect scenes     # list scenes without rendering
```
