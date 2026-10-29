# Engine benchmarks

Benchmarks are registered in `Cargo.toml` and use Criterion. Run Rust tooling
inside the repository's pinned Nix development shell, with limited parallelism:

```sh
nix develop --command cargo bench -p benches --bench gpu_render -j 4
```

## Frame rendering (`gpu_render` target)

The target name is retained for command compatibility. Its metrics are **not
pure GPU timings**:

- `frame_latency/{static,morph}/{count}` measures evaluation, extraction, buffer
  preparation, submission, and waiting for GPU completion.
- `frame_cpu/{static,morph}/{count}` measures evaluation through submission. Each
  iteration also waits for GPU completion, but excludes that wait from the
  measured interval, preventing an accumulating queue from distorting the result.

Both metrics evaluate a fresh frame inside every measured iteration. A static
scene means its contents do not animate, not that its evaluated input is reused.
This matches the evaluation/render boundary of playback and export, including
conversions that may move between evaluation and extraction during refactoring.

Scenes use 25, 100, 400, 1600, and 3600 vector items, plus a camera. Rendering is
1920×1080 with 8 OIT layers. Scene construction, GPU initialization, 20 initial
warmup frames, and pixel readback are outside the measured intervals. Upload
counting and GPU timestamps are disabled. Use profiler timestamp scopes to
inspect GPU pass time separately.

The old `gpu_render`, `cpu_submit`, `merged_gpu_render`, and `merged_cpu_submit`
groups evaluated once outside the loop; their results are **not comparable** to
these new groups. The two merged groups also duplicated the same renderer path.
Do not interpret old-to-new Criterion numbers as a regression or speedup.

`eval` and `frame` remain stage-specific diagnostics, not end-to-end metrics.
The `render` target measures actual output/export through the engine command API.
Diagnostic executables live in `src/bin/`; shared scenes are in `src/lib.rs`.
