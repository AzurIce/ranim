#!/usr/bin/env bash
# Replicates the CI "lints" job (.github/workflows/build.yml) locally.
#
# Run this before every push. Do NOT replace it with per-package sweeps:
# benches, example-packages/*, and feature-gated example sources are only
# compiled by the workspace-wide --all-targets run, which is how partial
# checks keep missing real failures.
set -euo pipefail
cd "$(dirname "$0")/.."

# CI pins nightly-2026-08-01; use it when rustup provides it locally,
# otherwise fall back to the default toolchain (lint sets may differ).
toolchain_flag=""
if command -v rustup >/dev/null 2>&1 &&
  rustup toolchain list 2>/dev/null | grep -q "nightly-2026-08-01"; then
  toolchain_flag="+nightly-2026-08-01"
else
  echo "note: pinned CI toolchain nightly-2026-08-01 not found;" \
    "checking with $(cargo -V | cut -d' ' -f1-2) instead." >&2
fi

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-8}"

echo "== cargo fmt --all --check =="
cargo $toolchain_flag fmt --all --check

echo "== clippy --workspace --all-targets -- -D warnings =="
cargo $toolchain_flag clippy --workspace --all-targets -- -D warnings

echo "== cargo doc (-D warnings) =="
RUSTDOCFLAGS="-D warnings" cargo $toolchain_flag doc --no-deps --workspace --document-private-items

echo "All CI lints passed."
