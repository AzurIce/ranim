set windows-shell := ["powershell.exe", "-Command"]

clean:
    -rm *.log

fmt:
    cargo fmt --all

lint: lint-no-features
    just lint-features render
    just lint-features profiling

lint-no-features: fmt
    cargo clippy --workspace --all-targets -- -D warnings
    cargo doc --no-deps --workspace --document-private-items

lint-features *FEATURES: fmt
    cargo clippy --workspace --all-targets --features {{ FEATURES }} -- -D warnings
    cargo doc --no-deps --workspace --document-private-items --features {{ FEATURES }}

changelog:
    git cliff -o CHANGELOG.md

website:
    zola --root website build

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
