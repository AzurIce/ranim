# App example

`src/lib.rs` defines `hello_ranim`, registers it in `ScenesPlugin`, and exports
`pub fn plugin() -> impl Plugin` using `#[ranim::export_plugin]`. Rust hosts
install `app::ScenesPlugin` or `app::plugin()` directly; the CLI loads the same
plugin through the dynamic library's `ranim_plugin` entry.
Each host owns its App. `src/main.rs` creates an App, installs `app::plugin()`,
wraps the App in an `Arc`, selects its `SceneHandle`, and calls `cmd::preview_scene`.
This host binary is named `app-preview` so its WASM artifact has a different name
from the plugin library.

To load the project through the CLI (with matching Ranim and Rust versions), run
`ranim preview -p app` from the repository root.

## Run Native

```
cargo run
```

## Run on web

### Using trunk

See [index.html](./index.html)

```
trunk serve
```

### Manually

See [manual.html](./manual.html)

```
cargo build --bin app-preview --target wasm32-unknown-unknown
wasm-bindgen --out-dir ./out --out-name app --target web ../../target/wasm32-unknown-unknown/debug/app-preview.wasm
```

The binary's `main` creates the host App, installs the library's plugin, and
starts the preview. `manual.html` initializes the module, which runs `main` and
uses the `ranim-app-hello_ranim` canvas. The plugin library has no Web preview
entry. Then use a file server to host the page:

```
miniserve .
```
