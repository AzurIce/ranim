clean:
    -rm *.log

fmt:
    cargo fmt --all

lint: lint-no-features
    just lint-features app
    just lint-features serde
    just lint-features profiling

lint-no-features: fmt
    cargo clippy --workspace --all-targets -- -D warnings
    cargo doc --no-deps --workspace --document-private-items

lint-features *FEATURES: fmt
    cargo clippy --workspace --all-targets --features {{FEATURES}} -- -D warnings
    cargo doc --no-deps --workspace --document-private-items --features {{FEATURES}}

changelog:
    git cliff -o CHANGELOG.md

website:
    just doc-nightly
    just book
    zola --root website build

doc-nightly:
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps -p ranim --document-private-items --all-features
    -rm -r website/static/doc/
    cp -r target/doc/ website/static/doc/

doc-examples:
    RUSTDOCFLAGS="--cfg docsrs --cfg docrs_dep --html-in-header ./packages/ranim-examples/docs-rs/header.html" \
    RUSTFLAGS="--cfg docsrs_dep" \
        cargo doc --no-deps -p ranim-examples --document-private-items --all-features \
        --target-dir ./packages/ranim-examples/target/
    
    cargo build -p ranim-examples --release --target wasm32-unknown-unknown
    wasm-bindgen --target web ./target/wasm32-unknown-unknown/release/ranim_examples.wasm \
        --out-dir ./packages/ranim-examples/target/doc/ranim_examples/pkg

doc:
    cargo doc --no-deps -p ranim --document-private-items --all-features
    -rm -r website/static/doc/
    cp -r target/doc/ website/static/doc/

book:
    mdbook build book