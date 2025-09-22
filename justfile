clean:
    -rm *.log

fmt:
    cargo fmt --all

lint: lint-no-features
    just lint-features preview
    just lint-features render
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

doc:
    RUSTDOCFLAGS="--cfg docsrs" cargo doc --no-deps -p ranim --document-private-items --all-features
    -rm -r website/static/doc/
    cp -r target/doc/ website/static/doc/

book:
    mdbook build book