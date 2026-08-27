set windows-shell := ["powershell.exe", "-Command"]

# Mirror the CI lints job (.github/workflows/build.yml): fmt + workspace-wide
# clippy/doc with `-D warnings`. Run inside the flake dev shell so the Rust
# toolchain matches CI's pinned nightly-2026-08-01.
cargo_jobs := env_var_or_default("CARGO_BUILD_JOBS", "8")

clean:
    -rm *.log

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint: lint-no-features
    just lint-features render
    just lint-features profiling

lint-no-features: fmt-check
    CARGO_BUILD_JOBS={{ cargo_jobs }} cargo clippy --workspace --all-targets -- -D warnings
    RUSTDOCFLAGS="-D warnings" CARGO_BUILD_JOBS={{ cargo_jobs }} cargo doc --no-deps --workspace --document-private-items

lint-features *FEATURES: fmt-check
    CARGO_BUILD_JOBS={{ cargo_jobs }} cargo clippy --workspace --all-targets --features {{ FEATURES }} -- -D warnings
    RUSTDOCFLAGS="-D warnings" CARGO_BUILD_JOBS={{ cargo_jobs }} cargo doc --no-deps --workspace --document-private-items --features {{ FEATURES }}

changelog:
    git cliff -o CHANGELOG.md

website:
    zola --root website build

build-engine-demo:
    cargo run -p xtask-examples -- build-engine-demo

doc-nightly:
    RUSTDOCFLAGS="--cfg docsrs --html-in-header packages/ranim-examples/docs-rs/header.html" \
        cargo +nightly doc --workspace --no-deps --document-private-items --all-features \
        --exclude app --exclude xtask-examples --exclude benches
    just _build-example-doc-assets

doc:
    RUSTDOCFLAGS="--cfg docsrs --html-in-header packages/ranim-examples/docs-rs/header.html" \
        cargo doc --workspace --no-deps --document-private-items --all-features \
        --exclude app --exclude xtask-examples --exclude benches
    just _build-example-doc-assets

_build-example-doc-assets:
    cargo build -p ranim-examples --release --target wasm32-unknown-unknown
    wasm-bindgen --target web target/wasm32-unknown-unknown/release/ranim_examples.wasm \
        --out-dir target/doc/ranim_examples/pkg

doc-examples:
    RUSTDOCFLAGS="--cfg docsrs --html-in-header packages/ranim-examples/docs-rs/header.html" \
        cargo doc --no-deps -p ranim-examples --document-private-items --all-features \
        --target-dir packages/ranim-examples/target
    cargo build -p ranim-examples --release --target wasm32-unknown-unknown
    wasm-bindgen --target web target/wasm32-unknown-unknown/release/ranim_examples.wasm \
        --out-dir packages/ranim-examples/target/doc/ranim_examples/pkg

book:
    mdbook build book

preview EXAMPLE:
    cargo run -p ranim-cli -- preview --example {{ EXAMPLE }}
