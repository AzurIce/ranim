# Custom render plugin example

This project demonstrates host-agnostic animation output types. `src/lib.rs`
defines `PrivateDrawing`, a complete animation output containing both vector
and mesh data. The type is private: nothing outside the library can name it,
yet the scene can still play it. It implements
`ranim::render::extract::Extract` by extracting its vector and mesh parts.

`ProjectPlugin` stores a `ranim::render::RenderSetup` resource that registers
`RanimItemsPlugin` (the built-in camera, vector, and mesh plugins) plus
`ExtractPlugin::<PrivateDrawing>`, so every renderer built from this project
knows how to draw the private output.

`src/main.rs` is the project binary: it assembles the App with
`ProjectPlugin`, finalizes plugin construction with `app.build()`
(`ranim::RanimAppExt`), and hands the resulting `ranim::RanimApp` to the
free function `run_cli` (`ranim::cmd::cli::run_cli`, exported from the
prelude), which serves the `preview`, `output`, `render`, and `inspect`
subcommands.

## Run

From the repository root:

```sh
cargo run -p ranim-render-plugin-example -- preview
cargo run -p ranim-render-plugin-example -- render render_plugin
```

The one-second `render_plugin` scene shows the private output's red vector
square and blue mesh triangle side by side. Rendering requires a native wgpu
adapter and `ffmpeg` on `PATH`.
