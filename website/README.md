# Website

The Notist SSG website consumes the shared [Ranim Web Components](../web/README.md).
The **App** tab composes their live WebGPU canvas and playback controls;
**Preview** shows prerendered media. Site-specific Tera components live in
`templates/components/`, and `static/site.js` binds controls to their viewport.

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

The output is `notist-output/ranim-website/`. Every Notist Site build generates
example pages and data through `cargo examples site`. `just website` also works
without a WASM build; the App tab appears only when both JS and WASM packages
exist. Preview media uses tracked shadow refs without downloads or credentials.

Rebuild changed Rust scenes with `cargo examples build arc` in another shell.
Notist watches `../web/` and republishes the component modules on each build;
editing them needs no WASM rebuild or manual copy. No media rendering or shadow
upload is needed for the App tab.

Notist also watches `Cargo.toml`, `examples/`, shared README assets, the extension
sources, shadow refs/config, and `render-state.json`. Source listings and README
content refresh on each site build; changed scenes still need an explicit WASM
rebuild. Examples without media open on the App tab when its package is available.

## Site structure

The repository-root `Notist.toml` declares the `book/` and `website/` workspace
members. `website/Notist.toml` declares the `ranim-website` package and its `[site]`.
From the repository root, use `notist build -p ranim-website` or
`notist serve -p ranim-website`; inside `website/`, these commands select this
project automatically. `just publications` builds both projects, with the Book
output in `notist-output/ranim-book/`. Their deployment URLs remain independent.

Documents under `content/` keep their relative paths with an `.html` extension;
`index.md` provides a directory's landing page. Metadata uses Notist root
annotations. `templates/*.tera` and `templates/components/*.tera` render document
attributes, the directory tree and `site.data`; `sass/` compiles to CSS and
`static/` is copied to the output root. Deployment uses the configured
`https://azurice.github.io/ranim/` prefix; local preview uses `/`.

The extension configured in `[site.extensions.examples]` runs
`cargo examples site` through build-scoped JSON Lines (protocol version 2), from
the website directory. One process handles `before_compile` (source documents)
and `before_template` (template data and contributed files), flushing each
response before waiting for the next event. It reads bundled Cargo targets,
`scenes.rs`, `main.rs`, and README files to create `examples/<group>/<name>.md`,
group indexes, and `site.data.<name>` in memory. Standalone examples are excluded.

`examples/` belongs to the extension; put authored documents elsewhere under
`content/`. `package.metadata.example.<name>.hide` suppresses a page while keeping
its data available. There are no generated example source or TOML files to commit.

The five modules from `../web/` are contributed through `files` at the output
root. Their public URLs remain `ranim-viewport.js`, `ranim-controls.js`, etc., so
`static/site.js` can import them without knowing the source directory. Tests and
the Web Components README are not published. Existing files from earlier
extensions are retained when contributing these modules.

Published media URLs come from `.shadow/refs/` and `shadow.toml`. Files are
published only when they match the media rules below `# shadow` in the root
`.gitignore`; WASM `pkg/` directories are excluded and built by CI instead.
Removed refs stop retaining their objects after deployment; remote cleanup uses
the existing Shadow GC policy and grace period.
README resources
use published URLs when available; local resources are copied into the ignored
`static/examples/<name>/` directory only when their contents change. Preparing a
site never renders scenes or uploads media.

`cargo examples run [names...]` renders and stages local media, then records the
successful source hash in `render-state.json`. It does not invoke shadow or upload
files. `--lazy-run` compares that record with the current scene and entrypoint
sources, independently of publication. Run `cargo examples publish` explicitly
to publish existing media; it does not render or change render state. Until then,
the site keeps using the media referenced by the existing shadow refs.
Site/WASM builds also preserve render provenance. Commit render state after
successful renders and changed refs after publication;
static media and packages stay ignored. See [the xtask guide](../xtasks/xtask-examples/README.md)
for commands and the stdio contract.

## Web Components

Embedding, JavaScript APIs, audio, timeline, theming and browser test instructions
are documented in [`web/README.md`](../web/README.md).
