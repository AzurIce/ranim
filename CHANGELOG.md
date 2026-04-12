## [0.2.0] - 2026-04-05

### 🚀 Features

- Multi-format video output and rotating animation (#126)
- Ellipse/EllipticArc types, improved VPointVec AABB & code quality (#128)
- *(preview)* Support dynamic resolution with aspect ratio presets (#156)
- *(preview)* Improve export UX and simplify Output.dir (#159)

### 🐛 Bug Fixes

- Texture dirty mark and read back texture bytes buffer init
- Polygon inner circle
- Add auto-impl for FuncRequirement
- Make book's version selector sync with url
- Correct frame sampling interval (#137)
- Web examples build
- Fix mesh type conversion
- Make wgsl on web happy

### 💼 Other

- Refactor animation implementation (#99)
- Remove timeline state (#104)
- Experimental `VItem2d` core item with depth support (#107)
- Refactor renderer (#109)
- Experimental Order Independent Transparency (#110)
- Stabilize VItem2d and OIT (#112)
- Enhance app (#114)
- Enhancements on `BoundingBox` trait and additional `Rectangle` constructors (#116)
- New anchor system, traits and APIs refactor (#120)
- Basis2d (#123)
- Removed static limit on render and preview api (#125)
- `Parallelogram` and `TextItem` item (#129)
- Double buffer (#132)
- Move link magic and ffi things out of ranim-core, flip ranim-app dependency (#133)
- Experimental GPU-driven rendering (#138)
- Removed per-item pipeline (#142)
- Refactor module structure: Move ranim-app into ranim (#143)
- Update dependencies (#144)
- Impl Interpolatable for OpaqueColor (#149)
- New `Line` item (#150)
- Mesh Item (#146)
- Prepare v0.2.0 release (#161)

### ⚙️ Miscellaneous Tasks

- Update deps
- Udpate deps, update wasm-bindgen to 0.2.105
- Deploy book and doc to ranim-book and ranim-doc repo, support multiple tags
- Add book version select patch
- *(book)* Move main to sub dir
- *(book)* Concurrent ref build
- *(book)* Add cache for mdbook stuff
- *(book)* Fix cache key
- *(book)* Support multi refs concurrent build
- *(book)* Add run name
- *(book)* Fix version select injection for 0.4.x version of mdbook
- *(book)* Removed redirection, make website link directly to main book
## [0.1.5] - 2025-10-18

### 🚀 Features

- Added `TypstText` item with `Alignable` impl by char diff (#93)

### 🐛 Bug Fixes

- Map timeline reset next start sec to 0 when the previous inner item timeline is empty
- `save_frames = true` saves all frames to 0000.png
- Fix wasm32 build
- Fix gpu out of memory

### 💼 Other

- Render pool and pipelined rendering (#96)

### 🚜 Refactor

- Add ranim-core, ranim-anims, ranim-items to reduce user code dep (#94)

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.4] - 2025-09-16

### 🚀 Features

- Play control button for app
- Support example target for ranim-cli
- *(timeline)* Rename state -> snapshot_ref, added snapshot, with_snapshot method
- Support preview specific scene
- Added `#[scene(color = "#....")]` to support setting clear color (#90)

### 🐛 Bug Fixes

- Fix bubble_sort example's insert and show
- Wasm app freezing on init
- Fixed `Alignable` implementation for `VPointComponentVec`, `VItem` and `Group<T>` (#91)

### 🚜 Refactor

- Use inventory instead of linkme for find_scene api in wasm

### ⚙️ Miscellaneous Tasks

- *(flake)* Use rust nightly
- Update deps
- Update flake to output ranim-cli
- Release
## [0.1.3] - 2025-08-20

### 🐛 Bug Fixes

- `#[preview] not working when `#[scene]` has no args

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.2] - 2025-08-18

### 🐛 Bug Fixes

- Group's alignable impl doesn't correctly align items

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.1] - 2025-08-10

### 🐛 Bug Fixes

- Scene macro's attrs not working

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.0] - 2025-08-10

### 🚀 Features

- Ffmpeg auto download, closes: #75
- Basic live previewing based on dylib
- Implemented scene, preview, output macros, rework on render config and the main entry apis
- Implemented `ranim render`

### 🐛 Bug Fixes

- Could not find `linkme` in the list of imported crates
- Use `downcast_ref_unchecked` for CameraFrame
- Ranim preview freezes when clicked on windows close button

### 📚 Documentation

- Update website and book

### ⚙️ Miscellaneous Tasks

- Don't run on drafted PR
- Update workflow
- Use nightly
- Release
## [0.1.0-alpha.17] - 2025-07-11

### 🚀 Features

- Pre implementation of #68
- Implemented map api, closes: #68
- Added lru cache for typst_svg
- Make typst world a singleton, optimize typst_svg performance

### 🐛 Bug Fixes

- ItemTimeline eval range incorrect
- Zero-length vec normalization error, correct test usecases, closes: #70
- Fix workflow and lint

### 📚 Documentation

- Added `#![warn(missing_docs)]`, first step to enhance documents

### ⚙️ Miscellaneous Tasks

- Build wasm in action
- Update flake
- Added git-cliff to flake's shell packages
- Release
- Release
## [0.1.0-alpha.14] - 2025-07-01

### 🚀 Features

- Make preview app supports wasm web, closes: #67
- Added wasm app into website examples

### 🐛 Bug Fixes

- Fix lint
- Use chrono instead of time for typst world
- Fix lint

### 🚜 Refactor

- Added extract stage
- Traits and geometry(WIP)
- Added PinnedItem, rework on timeline and anim APIs(WIP)
- Refactor timeline with show_secs
- Migrated some items and examples to the new item and timeline system
- Refactor Anchor
- Rework timeline
- Rename RabjectTimeline to ItemTimeline
- Removed padding from AnimationSpan
- Change type_name field of AnimationSpan to a method
- Refactor book structure

### 📚 Documentation

- Update doc for Anchor::Edge

### ⚙️ Miscellaneous Tasks

- Update deps
- Release
## [0.1.0-alpha.13] - 2025-05-01

### 🚀 Features

- Implemented Renderable for tuples and arrays
- Derive macros for anim traits
- Align vitem points according to ratio, closes: #33
- An attempt to share pass between items
- Scale with stroke

### 🚜 Refactor

- Refactor Transformable Trait to Position and BoundingBox
- Rework on derive macros

### ⚙️ Miscellaneous Tasks

- Use pretty_env_logger instead of env_logger
- Added puffin_viewer to flake shell
- Release
## [0.1.0-alpha.12] - 2025-04-20

### 🚜 Refactor

- Refactor timeline and item
- Removed 't timeline reference from Rabject

### 🎨 Styling

- Lint and some docs
- Lint

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.0-alpha.11] - 2025-04-01

### 🚀 Features

- Added ease-in-out rate functions

### 🐛 Bug Fixes

- #56, fixed subtract with overflow

### 🚜 Refactor

- Added TimelineItem trait to unify Group<T> and T insertion

### 📚 Documentation

- Fix palettes blue doc

### ⚙️ Miscellaneous Tasks

- Release
- Update cargo exclude
- Release
## [0.1.0-alpha.9] - 2025-03-29

### 🚀 Features

- Added scale_to and ScaleHint
- Added profiling based on puffin and wgpu_profiler
- Basic app
- Basic progress seeking
- Viewport fit in scaling
- Added profiling for preview app
- Timeline grid painting
- Max 100 timeline * 100 anims info drawing
- Current time indicator line, window title
- Clamp the timeline zoom to 100ms~total_sec

### 🐛 Bug Fixes

- #50

### 🚜 Refactor

- Rename build_and_render_timeline to render_scene
- Moved basic traits to a separate module
- Use f64 instead of f32, closes: #38
- *(app)* Refactor TimelineState

### 📚 Documentation

- Update docs
- Update doc for group
- Update doc for Transformable

### 🎨 Styling

- Fix lint
- Fix lint
- Lint

### ⚙️ Miscellaneous Tasks

- Release
- Release
## [0.1.0-alpha.7] - 2025-03-19

### 🚀 Features

- Implemented Debug for EvalResult, Animation and AnimSchedule
- Added perspective_blend, closes: #43

### 🐛 Bug Fixes

- Fix lint
- Fix lint and docs

### 🚜 Refactor

- Seperate scale and fovy for camera_frame's ortho and persp projection
- Refactor timeline build and render func
- Rewrite all examples with new coord-system and group apis

### 📚 Documentation

- Added doc for CameraFrame

### ⚙️ Miscellaneous Tasks

- *(xtask/build-examples)* Added clean arg to clean non-exist examples
- Release
## [0.1.0-alpha.6] - 2025-03-17

### 🚀 Features

- Group animation scaling
- *(example)* Added ranim_logo example
- Implemented Transformable for slice

### 🎨 Styling

- Lint
- Fix clippy lint

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.0-alpha.5] - 2025-03-16

### 🚀 Features

- Added put_anchor_or method, renamed TransformAnchor to Anchor
- First step of supporting group rabjects
- Unified play method for AnimSchedule and Group<AnimSchedule>

### 🐛 Bug Fixes

- Use mid point when failed to get intersection in approx_cubic_with_quad

### 📚 Documentation

- Fix doc link for `Anchor`

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.0-alpha.4] - 2025-03-14

### ⚙️ Miscellaneous Tasks

- Release
- Release
## [0.1.0-alpha.3] - 2025-03-14

### 🚀 Features

- Added output_filename option into AppOptions

### 🐛 Bug Fixes

- #31

### 📚 Documentation

- Added doc for Entity

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.0-alpha.2] - 2025-03-13

### 🚜 Refactor

- Use trait and struct approach to define a scene/timeline

### ⚙️ Miscellaneous Tasks

- Update cargo-release config
- Release
## [0.1.0-alpha.1] - 2025-03-13

### ⚙️ Miscellaneous Tasks

- Update cargo-release config
- Release
## [ranim-macros-v0.0.0] - 2025-03-12

### 🚀 Features

- Compute shader based vmobject stroke
- Added Rabject VPath based on cubic bezier curve
- Basic SvgMobject
- Added center_canvas_in_frame for CameraFrame, fix: #17
- Create and uncreate animation
- Rework create and uncreate, implement correct write and unwrite
- Migrate VMobject blueprints to VItem
- Rework on WgpuBuffer util and primitive trait, implemented clip_box for VItem
- Migrate old animations to new render system
- Simple svg_item implement (not renderable yet)
- Timeline
- Partial quad bezier for vitem
- Basic SvgItem
- Creation related trait for SvgItem
- Basic website
- *(website)* Index outline
- *(website)* Basic template for index and docs
- Added timeline proc_macro_attribute and render_timeline! macro to simplify the boiler plate codes
- Set timeline args through timeline attribute macro
- Render_anim_frame macro
- Animation "stack" through "sync" concept, animation chain
- *(website)* Added preview imgs for examples

### 🐛 Bug Fixes

- Support zero length (or single point) beziers to vertex
- Fix color lerp
- #1
- Vmobject's compute uniforms are not correctly initialized
- Incorrect alignment of VMobject's points
- #2
- Auto remove updater if the target rabject is not exist
- Updater's on_create and on_destroy are not being called correctly
- #6, fix: #7
- #8, improved fading by interpolate between all 0.0 to current for fade in and current to all 0.0 for fade out
- EntityAny downcasting error
- Fix srgba color
- Polygon has no fill caused by points arranged not in clock wise
- Fixed distance_bezier and close flag
- Vitem rendering sgn calc and empty vitem
- Fix solve cubic
- Get_closedpath_flag
- Render frame to image
- AnimSchedule.apply now updates the freeze_anim of the timeline
- Fix example attachments url
- #26
- #27
- *(website)* Fix toml output
- #29

### 🚜 Refactor

- Refactor traits, added Mobject struct, impl render func on Vertex struct instead of pipeline
- Moved wgpu related field and functions of Mobject to ExtractedMobject, added ToMobject trait, added width for Arc and Polygon, added radius for Arc
- Make scene support different pipeline
- Adjust visibilities
- Make functions support any type implemented PipelineVertex
- Introduced Renderer trait for multi shader of object's single draw
- Refactoring bezier for filling
- Finished stroke based on compute shader
- Added Renderer trait, avoid depth problem for VMobject using stencil test
- Use Newell's Method to calculate VMobject's normal
- Use stencil test instead of alpha blending to calculate winding number of VMobject fill
- Moved RanimContext into Scene to simplify the API
- Animation under new structure
- Combine the rendering of vello and wgpu, refactor scene and render architecture
- Polish apis
- Added BezPath and VMobject based on vello, refactor svg based on VMobject
- Reimplemented 2drabjects based on vello, fully use vello in canvas
- Remove old 2d things
- Rework animation playing api
- Split world and renderer, refactor the app structure
- Rewrite rendering pipeline
- Fully rewrite the entity render system
- New animation system
- Redesigned traits, cleanup
- Transform3d trait
- Rewrite Timeline and Animation system
- Rework anim api and color system
- Static and dynamic anim, calc clip_info in compute shader
- Refactor timeline and eval
- Support Anim for CameraFrame
- Animation cleanup
- Timeline func, Entity trait

### 📚 Documentation

- Fix doc warnings

### 🎨 Styling

- Cargo fmt and some clippy
- Cleanup
- Cargo clippy & cargo fmt
- Fixed some clippy warnings
- Fix clippy warnings
- Lint
- Lint

### 🧪 Testing

- Tix transform test

### ⚙️ Miscellaneous Tasks

- Added flake.nix
- Added build workflow
- *(ci)* Added typst and ffmpeg dep for testing
- *(ci)* Removed macos-latest from test matrix
- *(ci)* Removed test job due to no gpu on runner
- Added github pages workflow
- Gh deploy only when push to main
