example target:
    # -rm -rf examples/{{target}}/output/*
    # -rm -rf output/{{target}}/*
    # -mkdir examples/{{target}}/output

    cargo run --example {{target}} --release

    # cp output/{{target}}/* examples/{{target}}/output/
    -cp output/{{target}}/output.mp4 assets/{{target}}.mp4
    -cp output/{{target}}/output.png assets/{{target}}.png

examples:
    just example arc
    just example arc_between_points
    just example basic
    just example palettes

clean:
    -rm *.log

fmt:
    cargo fmt --all

lint: fmt
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
    cargo doc --no-deps --workspace --document-private-items

changelog:
    git cliff -o CHANGELOG.md