# Website

The Notist SSG website owns two independent Web Components: `<ranim-viewport>` in
`static/ranim-viewport.js` and `<ranim-controls>` in `static/ranim-controls.js`.
The **App** tab composes their live WebGPU canvas and playback controls;
**Preview** shows prerendered media. Handlebars partials live
in `templates/components/`. The site is built with `notist ssg`; templates use document attributes and the directory tree.

## Build and preview

From the repository root, enter the Nix development shell. It includes the
Notist version pinned by `flake.lock`, which CI also uses:

```sh
nix develop
export CARGO_BUILD_JOBS=4
just website-preview arc
```

Set `NOTIST=/path/to/notist` to test a different local build. Update the pinned
generator with `nix flake update notist`.

Open `http://127.0.0.1:8000/examples/arc.html` and select **App**. Omit `arc` to
build every WASM example before starting the server. WebGPU requires a secure
context (HTTPS or localhost).

For a static site, stop the preview server, then run:

```sh
cargo examples build
just website
```

The output is `website/public/`. Every Notist SSG build generates example pages
and data through `cargo examples site`. `just website` also works without a WASM
build; the App tab appears only when both JS and WASM packages exist. Preview
media is resolved from tracked shadow refs without local downloads or credentials.

While Notist SSG is serving, rebuild changed Rust scenes with `cargo examples build arc`
in another development shell. Changes to the preview JavaScript files are
watched directly by Notist SSG and require no WASM rebuild or component copy. No media
rendering or shadow upload is needed for the App tab.

Notist also watches `Cargo.toml`, `examples/`, shared README assets, the extension
sources, shadow refs/config, and `render-state.json`. Source listings and README
content refresh on each site build; changed scenes still need an explicit WASM
rebuild to update the live App. Examples without media open on the App tab when
its package is available.

## Site structure

`Notist.toml` configures `notist ssg`. Documents under `content/` keep their
relative paths, with the extension changed to `.html`; `index.md` provides a
directory's landing page. Metadata uses Notist root annotations.
`templates/*.hbs` and `templates/components/*.hbs` render document attributes,
the directory tree and `site.data`; `sass/` compiles to CSS and `static/` is
copied to the output root. Deployment uses the configured
`https://azurice.github.io/ranim/` prefix; local preview uses `/`.

The Experimental `prepare` extension configured in `[[ssg.extensions]]` invokes
`cargo examples site` through JSON stdio, from the website directory. It reads
the workspace `[[example]]` targets, `scenes.rs`, `main.rs`, and README files to
create `examples/<group>/<name>.md`, group indexes, and `site.data.<name>` in
memory. `examples/` belongs to the extension; put authored documents elsewhere
under `content/`. `package.metadata.example.<name>.hide` suppresses a page while
keeping its data available to other templates. There are no generated example
source files or per-example TOML files to commit.

Published media URLs come from `.shadow/refs/` and `shadow.toml`. README resources
use published URLs when available; local resources are copied into the ignored
`static/examples/<name>/` directory only when their contents change. Preparing a
site never renders scenes or uploads media.

`cargo examples run [names...]` renders and publishes media, then records the
successful source hash in `render-state.json`. `--lazy-run` compares that record
with the current scene and entrypoint sources. `cargo examples publish` only
publishes existing media. Site/WASM builds and publish-only runs preserve render
provenance. Commit changed refs and render state after successful renders;
static media and packages stay ignored. See [the xtask guide](../xtasks/xtask-examples/README.md)
for commands and the stdio contract.

## Composing a viewport and controls

`<ranim-viewport>` owns the canvas, WASM session, and playback clock. It can run
without a control bar. `<ranim-controls>` supplies a play/pause button, seek
slider, and time display. It subscribes to its target's state and calls its
public playback API; it creates no canvas, GPU device, or animation clock.
The controls module can be imported independently of the viewport module.
It imports `ranim-timeline.js` for its expandable animation timeline.
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

## Expanding the progress bar into a timeline

The chevron at the right of the control bar opens an animation timeline below
the seek slider. It starts collapsed. To open it initially or from JavaScript:

```html
<ranim-controls for="arc-view" timeline-expanded></ranim-controls>
```

```js
controls.timelineExpanded = true;
```

The panel uses the same core animation metadata as the native egui timeline.
Ordered sequences share a track; overlapping containers disclose separate child
tracks. The tree starts with two levels of overlapping containers expanded.
Animations, held values, groups, and audio leaves have distinct clip colors.
Click a disclosure arrow to fold a group, or select a clip to inspect its name
and scene-time range. Clip names are treated as text, including Rust generics.

Click or drag the top time ruler to seek. Selecting a clip (including from
the keyboard) only updates its inspector; clips and track backgrounds do not
move the playhead. The focused ruler supports Left/Right, Home, and End. Use +/− or
Ctrl/Cmd-wheel to zoom, scroll horizontally to pan, and Fit to show the whole
scene. The playhead follows the viewport's existing clock in either view.
Placement is projected through container content durations, matching the native
timeline; easing still belongs to scene evaluation.

Each bar keeps its own expanded groups, selection, and zoom. Opening or closing
the panel retains that UI state. Changing the scene or binding another viewport
resets the tree and zoom; removing controls leaves playback running.

The viewport starts paused. It exposes `play()`, `pause()`, `seek(seconds)`,
`currentTime` (read/write), `duration`, `paused`, `ready`, `timeline`, `hasAudio`,
`volume` (0–1, read/write), and `muted` (read/write). Listen for `ready`
(`detail: { scene, duration }`), `timeupdate`, `play`, `pause`, `ended`, and
`error` (`detail` is the error), `loading`, `volumechange`, and `emptied` (the session was released).
Removing the viewport stops scheduling and
releases its GPU session; reconnecting creates a new one. The drawing buffer
tracks the element's width and device pixel ratio, capped at 1280 pixels wide.
The viewport has a fixed 16:9 view. Scenes with sound, including `audio_demo`
and `midi_visualizer`, play their baked mix through Web Audio. Click Play to
activate browser audio. Loading a scene or seeking while paused stays silent.
Playback, pause, slider seeking, and timeline seeking share one audio clock;
seeking while playing replaces the source at the new absolute scene position.
The visual clock follows audio output timestamps when available, accounting
for device latency ([Web Audio specification](https://www.w3.org/TR/webaudio/#dom-audiocontext-getoutputtimestamp)).
Silent scenes use the wall clock and open no audio device. Each viewport owns
its audio context, which is closed on scene replacement, failure, or removal;
extra control bars do not create extra voices. Audio-start failures pause
playback and report `error`, while keeping the scene available for another try.

The speaker button opens a volume slider and mute button; Escape or clicking
outside closes the panel. Volume changes apply through a smoothed gain without
restarting the source or moving its clock. All bars bound to one viewport share
the volume and mute state. Muting retains the chosen volume, and each viewport
keeps its settings across scene changes and reconnection. Silent scenes disable
the speaker button. Custom hosts can set `viewport.volume = 0.5` and
`viewport.muted = true` and observe `volumechange`.

`WasmPreview.audio()` returns `null` for silent scenes, otherwise
`{ sampleRate, channels }`: the scene's baked stereo mix at 48 kHz as an array
of JavaScript-owned `Float32Array` channels. The viewport reads it once at
initialization and creates a reusable `AudioBuffer`; seeking never remixes the
scene or copies PCM across WASM. Custom hosts can use the same snapshot with
their own audio player. The snapshot stays valid after session disposal.

`viewport.timeline` is a snapshot fetched once when the session becomes ready,
and is `null` while unloaded. All controls bound to that viewport share this
metadata without fetching it on each playback frame. Custom WASM hosts can call
`preview.timeline()` to obtain the same ordinary JavaScript object:

```js
const { duration, roots } = preview.timeline();
// Each node: { name, kind, range: [start, end], contentDuration,
//              enabled, simStep, children }
// range is in parent-local seconds; kind is "anim", "children", or "audio".
```

Snapshots remain usable after the renderer is disposed and need no `free()`.

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
audio playback time (or wall time for silent scenes) in `requestAnimationFrame`
and pass it to `renderAt`. The built-in viewport imports `ranim-audio.js` to
manage audio playback and synchronization.
Times are clamped to the scene duration; non-finite values are rejected.

The website uses the component sources directly. When `cargo examples build`
targets another `--out-dir`, such as the rustdoc directory, it copies
`ranim-viewport.js`, `ranim-audio.js`, `ranim-controls.js`, `ranim-timeline.js`,
and their shared `ranim-theme.js` tokens to that directory
alongside the generated WASM packages. Website and rustdoc
markup explicitly bind sibling controls to their viewport through `target`,
so repeated previews need no generated IDs.

The components inherit the host's fonts and color scheme. Their shared theme
uses the website's Markdown/Shoelace colors or rustdoc's theme variables, with
neutral surfaces, thin borders, and a filled seek track. The website composes
canvas, controls, and timeline inside one bordered frame; standalone components
remain independently placeable. Override `--ranim-accent`,
`--ranim-controls-background`, `--ranim-text`, `--ranim-muted`, `--ranim-border`,
or `--ranim-radius` on the element or a shared ancestor to customize the theme. The
`controls` and `timeline` CSS parts are available for host styling. The viewport exposes
`viewport` and `canvas` parts. The host chooses placement, spacing, and the
number of control bars; the components impose no shared layout.

## Browser checks

Build `arc` in the Nix development shell, then serve the repository root:

```sh
cargo examples build arc
python3 -m http.server 8000 --bind 127.0.0.1
```

The timeline model and audio playback state have tests that need no browser or GPU:

```sh
node --test website/tests/*.test.mjs
```

Open `http://127.0.0.1:8000/website/tests/preview.test.html`. The checks use the
real WASM module and a WebGPU device to cover independent imports, ID/property
binding, multiple controls, timeline metadata/expansion/zoom/selection, rebinding,
shadow roots, playback, independent canvases,
unmount/reconnect, input validation, rendering, resize, and disposal. Test files
live in `website/tests/`, outside the published Notist SSG static assets.

For audio integration checks, build `cargo examples build arc audio_demo midi_visualizer`,
open `http://127.0.0.1:8000/website/tests/audio.test.html`, and click **Run with audio**.
These checks inspect real MIDI PCM and an analyser attached to the Web Audio
graph, suspend the audio context to verify that visual time also freezes, and
cover pause, both seek controls, replay, multiple controls, scene changes, and
cleanup. The button supplies the user activation required for browser audio.
