example target:
    rm -rf examples/{{target}}/output*
    cargo run --example {{target}} --release
    mv output* examples/{{target}}/
