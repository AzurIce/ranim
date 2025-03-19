# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0-alpha.7] - 2025-03-19

### 🚀 Features

- Implemented Debug for EvalResult, Animation and AnimSchedule
- Added perspective_blend, closes: #43

### 🚜 Refactor

- Seperate scale and fovy for camera_frame's ortho and persp projection
- Refactor timeline build and render func
- Rewrite all examples with new coord-system and group apis

### 📚 Documentation

- Added doc for CameraFrame

### ⚙️ Miscellaneous Tasks

- *(xtask/build-examples)* Added clean arg to clean non-exist examples

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

### WIP

- Support for RabjectGroup

### Misc

- Added logo

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
- #26
- #27
- *(website)* Fix toml output
- #29

### 🚜 Refactor

- Moved wgpu related field and functions of Mobject to ExtractedMobject, added ToMobject trait, added width for Arc and Polygon, added radius for Arc
- Make scene support different pipeline
- Adjust visibilities
- Make functions support any type implemented PipelineVertex
- Introduced Renderer trait for multi shader of object's single draw
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

### WIP

- Refactoring bezier for filling
- Fixing arc between points put start_and_end modifies the width
- Refactor to correct vectorized objects
- Finishing the refactor
- Finished refactor but with stroke and fill not finished
- Refactor project structure
- Fixing aligning for animation
- Rewrite camera
- Refactor to support hierarchy
- Refactor object management and scene render architecture, but with animation not compatible
- Introduce vello

### Website

- New codeblock style with linenos, update examples
- Examples page

<!-- generated by git-cliff -->
