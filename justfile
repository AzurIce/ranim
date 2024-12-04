example target:
    rm -rf examples/{{target}}/output*
    cargo run --example {{target}} --release
    mv output* examples/{{target}}/
    -cp examples/{{target}}/output.mp4 assets/{{target}}.mp4
    -cp examples/{{target}}/output.png assets/{{target}}.png

examples:
    just example arc
    just example arc_between_points
    just example basic
    just example palettes