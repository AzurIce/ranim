# Website

The Zola website owns two independent Web Components: `<ranim-viewport>` in
`static/ranim-viewport.js` and `<ranim-controls>` in `static/ranim-controls.js`.
The **App** tab composes their live WebGPU canvas and playback controls;
**Preview** shows prerendered media. Tera components live
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
in another development shell. Changes to the two component files are
watched directly by Zola and require no WASM rebuild or component copy. No media
rendering or shadow upload is needed for the App tab.

Website WASM builds also refresh the selected examples' source listings and
pages, including examples without prerendered media. Existing media URLs and
the last render's source hash are preserved, so `cargo examples run --lazy-run`
still detects scenes that need rendering. Examples without media open directly
on the App tab.

## Composing a viewport and controls

`<ranim-viewport>` owns the canvas, WASM session, and playback clock. It can run
without a control bar. `<ranim-controls>` supplies a play/pause button, seek
slider, and time display. It subscribes to its target's state and calls its
public playback API; it creates no canvas, GPU device, or animation clock.
The controls module can be imported independently of the viewport module.
Rendering uses Ranim's render sub-app directly; egui and eframe are native-only.

```html
<script type="module" src="./ranim-viewport.js"></script>
<script type="module" src="./ranim-controls.js"></script>
<main>
  <ranim-viewport id="arc-view" src="./examples/arc/pkg/ranim_example.js" scene="arc"></ranim-viewport>
</main>
<footer>
  <ranim-controls for="arc-view"></ranim-controls>
</footer>
```

The viewport's `src` points to a wasm-bindgen `--target web` module exporting `app()`. Omit
`scene` to select the first registered scene. Changing either attribute
replaces the preview. Each viewport creates its own project, evaluator, and
GPU device, including when two viewports display the same scene.

`for` resolves an ID in the controls' document or shadow root. Controls may
precede the viewport, live in another container, or share a viewport with other
controls. ID binding follows late insertion, removal, replacement, and ID
changes. An absent or unready target disables the bar.

For explicit binding, including across shadow roots:

```js
const controls = document.querySelector("ranim-controls");
controls.target = document.querySelector("ranim-viewport");
controls.target = anotherViewport; // rebind; the previous viewport keeps running
controls.target = null; // detach and disable the controls
```

Setting `target` takes precedence over `for`; changing `for` returns to ID binding.
Removing controls detaches listeners and leaves the viewport running. Reconnecting
them restores their binding. Each bar stays synchronized through viewport events.

The viewport starts paused. It exposes `play()`, `pause()`, `seek(seconds)`,
`currentTime` (read/write), `duration`, `paused`, and `ready`. Listen for `ready`
(`detail: { scene, duration }`), `timeupdate`, `play`, `pause`, `ended`, and
`error` (`detail` is the error), `loading`, and `emptied` (the session was released).
Removing the viewport stops scheduling and
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

The website uses both component sources directly. When `cargo examples build`
targets another `--out-dir`, such as the rustdoc directory, it copies both files
to that directory alongside the generated WASM packages. Website and rustdoc
markup explicitly bind sibling controls to their viewport through `target`,
so repeated previews need no generated IDs.

The controls use a dark bar, a filled seek track, and accessible icon
buttons with keyboard focus indicators. Override `--ranim-accent` or
`--ranim-controls-background` on the element to customize its colors; the
`controls` CSS part is available for host styling. The viewport exposes
`viewport` and `canvas` parts. The host chooses placement, spacing, and the
number of control bars; the components impose no shared layout.

## Browser checks

Build `arc` in the Nix development shell, then serve the repository root:

```sh
cargo examples build arc
python3 -m http.server 8000 --bind 127.0.0.1
```

Open `http://127.0.0.1:8000/website/tests/preview.test.html`. The checks use the
real WASM module and a WebGPU device to cover independent imports, ID/property
binding, multiple controls, rebinding, shadow roots, playback, independent canvases,
unmount/reconnect, input validation, rendering, resize, and disposal. Test files
live in `website/tests/`, outside the published Zola static assets.
