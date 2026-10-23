# Website

The Zola website owns the `<ranim-player>` Web Component in
`static/ranim-player.js`. The **App** tab uses this live WebGPU canvas and
playback controls; **Preview** shows prerendered media. Tera components live
in `templates/components/`. The Nix development shell and CI use Zola 0.23.6.

## Build and preview

From the repository root:

```sh
nix develop
export CARGO_BUILD_JOBS=4
just website-preview arc
```

Open `http://127.0.0.1:1111/examples/arc/` and select **App**. Omit `arc` to
build every WASM example before starting the server. WebGPU requires a secure
context (HTTPS or localhost).

For a static site, stop the preview server, then run:

```sh
cargo examples build
just website
```

The output is `website/public/`. `just website` runs only Zola, so it expects
the WASM packages to have been built already.

While Zola is serving, rebuild changed Rust scenes with `cargo examples build arc`
in another development shell. Changes to `website/static/ranim-player.js` are
watched directly by Zola and require no WASM rebuild or component copy. No media
rendering or shadow upload is needed for the App tab.

Website WASM builds also refresh the selected examples' source listings and
pages, including examples without prerendered media. Existing media URLs and
the last render's source hash are preserved, so `cargo examples run --lazy-run`
still detects scenes that need rendering. Examples without media open directly
on the App tab.

## Canvas component

`static/ranim-player.js` defines a dependency-free `<ranim-player>` Web Component:
a canvas, play/pause button, seek slider, and time display. It renders through
Ranim's render sub-app directly; egui and eframe are native-only.

```html
<script type="module" src="./ranim-player.js"></script>
<ranim-player src="./examples/arc/pkg/ranim_example.js" scene="arc"></ranim-player>
```

`src` points to a wasm-bindgen `--target web` module exporting `app()`. Omit
`scene` to select the first registered scene. Changing either attribute
replaces the preview. Each element creates its own project, evaluator, and
GPU device, including when two elements display the same scene.

The component starts paused. It exposes `play()`, `pause()`, `seek(seconds)`,
`currentTime` (read/write), `duration`, and `paused`. Listen for `ready`
(`detail: { scene, duration }`), `timeupdate`, `play`, `pause`, `ended`, and
`error` (`detail` is the error). Removing the element stops scheduling and
releases its GPU session; reconnecting creates a new one. The drawing buffer
tracks the element's width and device pixel ratio, capped at 1280 pixels wide.
This initial player has a fixed 16:9 view and no audio playback or animation-tree
timeline.

For a custom UI, use the WASM API directly:

```js
import init, { app } from "./examples/arc/pkg/ranim_example.js";
await init();
const project = app();
console.log(project.sceneNames()); // newline-separated names
const preview = await project.createPreview(canvas, "arc");
project.free(); // the session now owns the finalized project
console.log(preview.duration);
preview.resize(1280, 720); // physical pixels; render again after resizing
preview.renderAt(1.5); // false means retry on the next animation frame
preview.dispose(); // idempotent; releases the GPU device and scene
preview.free(); // release the wasm-bindgen handle, after any async work settles
```

`createPreview` consumes the project after validating the scene name, even if
GPU initialization subsequently fails. Create another project with `app()`
for another attempt. Playback scheduling belongs to the caller: sample the
elapsed wall-clock time in `requestAnimationFrame` and pass it to `renderAt`.
Times are clamped to the scene duration; non-finite values are rejected.

The website uses the component source directly. When `cargo examples build`
targets another `--out-dir`, such as the rustdoc directory, it copies this same
source to `<out-dir>/ranim-player.js` alongside the generated WASM packages.

The player uses a dark control bar, a filled seek track, and accessible icon
buttons with keyboard focus indicators. Override `--ranim-accent` or
`--ranim-controls-background` on the element to customize its colors; the
`player`, `canvas`, and `controls` CSS parts remain available for host styling.

## Browser checks

Build `arc` in the Nix development shell, then serve the repository root:

```sh
cargo examples build arc
python3 -m http.server 8000 --bind 127.0.0.1
```

Open `http://127.0.0.1:8000/website/tests/preview.test.html`. The checks use the
real WASM module and a WebGPU device to cover playback, independent canvases,
unmount/reconnect, input validation, rendering, resize, and disposal. Test files
live in `website/tests/`, outside the published Zola static assets.
