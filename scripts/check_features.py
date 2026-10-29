#!/usr/bin/env python3
"""Check facade feature boundaries without workspace-wide feature unification.

Run inside the Nix development shell. This inspects resolved normal dependencies
and lints/documents each supported minimal configuration, then checks WASM hosts.
"""

import os
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parent.parent
ENV = {**os.environ, "CARGO_BUILD_JOBS": os.environ.get("CARGO_BUILD_JOBS", "4")}


def run(args, *, capture=False, env=ENV):
    return subprocess.run(
        args, cwd=ROOT, env=env, check=True, text=True, capture_output=capture
    )


def check(features, target=None, defaults=False):
    label = features or ("default" if defaults else "minimal")
    print(f"Checking {target or 'native'}: {label}", flush=True)
    flags = [] if defaults else ["--no-default-features"]
    if features:
        flags += ["--features", features]
    if target:
        flags += ["--target", target]

    tree = run(
        ["cargo", "tree", "-p", "ranim", "-e", "normal", "--prefix", "none", *flags],
        capture=True,
    ).stdout
    packages = {line.split()[0] for line in tree.splitlines() if line}
    enabled = set(features.split(",")) if features else set()
    if defaults:
        enabled |= {"anims", "items", "cmd-preview"}
    if "cmd-preview" in enabled:
        enabled |= {"cmd-render"}
    if "cmd-render" in enabled:
        enabled |= {"render"}
    if "anims" in enabled:
        enabled |= {"items"}
    for feature, package in [
        ("anims", "ranim-anims"), ("items", "ranim-items"), ("render", "ranim-render")
    ]:
        assert (package in packages) == (feature in enabled), (label, package)
    for package in ["clap", "reqwest"]:
        assert (package in packages) == (not target and "cmd-render" in enabled), (
            label, package
        )
    for package in ["eframe", "egui", "rodio", "cpal"]:
        assert (package in packages) == (not target and "cmd-preview" in enabled), (
            label, package
        )
    assert "symphonia" not in packages, "PCM playback must not enable file decoding"
    assert ("dioxus-devtools" in packages) == (not target and "subsecond" in enabled), (
        label, "dioxus-devtools"
    )

    run(["cargo", "clippy", "-p", "ranim", "--lib", "--tests", *flags, "--", "-D", "warnings"])
    run(
        ["cargo", "doc", "-p", "ranim", "--no-deps", "--document-private-items", *flags],
        env={**ENV, "RUSTDOCFLAGS": "-D warnings"},
    )


for feature_set in ["", "items", "anims", "render", "render,items", "cmd-render", "cmd-preview", "subsecond"]:
    check(feature_set)
for feature_set, defaults in [("render", False), ("anims,render", False), ("render,subsecond", False), ("", True)]:
    check(feature_set, "wasm32-unknown-unknown", defaults)
run([
    "cargo", "clippy", "-p", "ranim", "--examples", "--no-default-features",
    "--features", "anims,cmd-render,typst,gltf", "--", "-D", "warnings",
])
for features in ["anims,render,typst,gltf", "anims,cmd-preview,typst,gltf"]:
    run([
        "cargo", "clippy", "-p", "ranim", "--examples", "--no-default-features",
        "--target", "wasm32-unknown-unknown", "--features", features,
        "--", "-D", "warnings",
    ])
