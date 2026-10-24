set windows-shell := ["powershell.exe", "-Command"]

# Mirror the CI lints job (.github/workflows/build.yml): fmt + workspace-wide
# clippy/doc with `-D warnings`. Run inside the flake dev shell so the Rust
# toolchain matches CI's pinned nightly-2026-10-01.
cargo_jobs := env_var_or_default("CARGO_BUILD_JOBS", "8")

clean:
    -rm *.log

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint: lint-default
    just lint-feature-boundaries
    just lint-features audio-decode
    just lint-features cmd-preview,typst,gltf

lint-default: fmt-check
    CARGO_BUILD_JOBS={{ cargo_jobs }} cargo clippy --workspace --all-targets -- -D warnings
    RUSTDOCFLAGS="-D warnings" CARGO_BUILD_JOBS={{ cargo_jobs }} cargo doc --no-deps --workspace --document-private-items

lint-features *FEATURES: fmt-check
    CARGO_BUILD_JOBS={{ cargo_jobs }} cargo clippy --workspace --all-targets --features {{ FEATURES }} -- -D warnings
    RUSTDOCFLAGS="-D warnings" CARGO_BUILD_JOBS={{ cargo_jobs }} cargo doc --no-deps --workspace --document-private-items --features {{ FEATURES }}

# Run separately from workspace sweeps so other packages cannot enable features.
lint-feature-boundaries:
    CARGO_BUILD_JOBS={{ cargo_jobs }} python3 scripts/check_features.py

changelog:
    git cliff -o CHANGELOG.md

website:
    zola --root website build

# Build the requested WASM examples (all by default), then serve Zola locally.
website-preview *EXAMPLES:
    CARGO_BUILD_JOBS={{ cargo_jobs }} cargo examples build {{ EXAMPLES }}
    zola --root website serve --base-url http://127.0.0.1

doc:
    RUSTDOCFLAGS="--cfg docsrs --html-in-header packages/ranim-examples/docs-rs/header.html" \
        cargo doc --workspace --no-deps --document-private-items --all-features \
        --exclude app --exclude xtask-examples --exclude benches
    just _build-example-doc-assets

_build-example-doc-assets:
    CARGO_BUILD_JOBS={{ cargo_jobs }} cargo examples build --out-dir target/doc/ranim_examples

doc-examples:
    RUSTDOCFLAGS="--cfg docsrs --html-in-header packages/ranim-examples/docs-rs/header.html" \
        cargo doc --no-deps -p ranim-examples --document-private-items --all-features \
        --target-dir packages/ranim-examples/target
    CARGO_BUILD_JOBS={{ cargo_jobs }} cargo examples build --out-dir packages/ranim-examples/target/doc/ranim_examples

book:
    mdbook build book

preview EXAMPLE:
    cargo run --example {{ EXAMPLE }} -- preview
