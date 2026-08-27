# AGENTS

## Lints

Replicate the CI lints job (`.github/workflows/build.yml`) before every push:

```bash
./scripts/check-ci-lints.sh
```

It runs the exact CI commands (fmt / clippy `-D warnings` / doc with
`RUSTDOCFLAGS=-D warnings`), using the pinned `nightly-2026-08-01` toolchain
when rustup provides it.

Gotchas learned the hard way — partial sweeps are NOT equivalent:

- Per-package checks (`cargo clippy -p <pkg> --all-targets`) miss `benches`,
  `example-packages/*`, and the `packages/ranim-examples` sources, which only
  compile under the workspace-wide sweep.
- Feature-gated `[[example]]` targets are silently skipped by
  `cargo check/clippy -p ranim --examples` when their features are off.
- If a lint failure shows up in CI, fix it on the branch whose PR failed;
  fixing it in a stacked working tree leaves the PR red.

## Build & Test Notes

- **Limit cargo parallelism to avoid OOM.** A full `cargo test --workspace` or
  `cargo build --workspace --all-targets` compiles the entire workspace
  (wgpu, eframe/egui, 25+ cdylib examples) with one rustc job per core, which
  can exhaust memory and freeze the machine (observed with 32 cores / 29 GiB
  RAM). Prefer limiting parallelism and/or testing per package, e.g.:

  ```bash
  cargo test -p ranim-core -p ranim-items -j 8
  CARGO_BUILD_JOBS=8 cargo run -p ranim-cli -- output <scene> --example <example>
  ```

## 构建与测试注意事项

- **限制 cargo 并行度以避免内存耗尽。**全量 `cargo test --workspace` 或
  `cargo build --workspace --all-targets` 会按核心数并行编译整个 workspace
  （wgpu、eframe/egui、25+ 个 cdylib example），可能耗尽内存导致整机卡死
  （在 32 核 / 29 GiB 内存上实际发生过）。建议限制并行度或分包测试。

## PR Authoring Guidelines

This file describes how to write and update pull request descriptions in this
repository. It applies to both human contributors and AI coding agents.

### Language

- PR titles and commit messages are always written in **English**.
- The PR description is bilingual: English first, then Chinese, separated by a
  `---` divider. Both versions carry the same content, including the
  `Closes:` line and the Breaking Changes section.

### PR Description Format

When creating or editing a PR, follow this structure:

```
Closes: #<issue-number>

- **feat**: New features or capabilities
- **fix**: Bug fixes
- **refactor**: Code improvements without behavior change
- **docs**: Documentation updates
- **perf**: Performance improvements
- **test**: Test additions or changes

### Breaking Changes

- **API/Field name**: Description of the breaking change and migration path

---

## Component/Feature 1

Detailed description of the first major change.

Use code blocks, mermaid diagrams, or examples as needed.

## Component/Feature 2

...
```

#### Top section

- Concise bullet list categorized by change type (`feat` / `fix` / `refactor` /
  `docs` / `perf` / `test`).
- Include `Closes: #<issue-number>` only when the PR closes an issue.

#### Breaking changes

- Always a separate section if any exist.
- For each breaking change, name the API or field and describe the migration path.

#### Detail sections

- Group related changes logically by component or feature.
- Focus on "what changed" and "why it matters".
- Use examples instead of long prose explanations.
- Use visuals when helpful:
  - Mermaid flowcharts for workflows, pipelines, or state machines.
  - Code snippets for API changes.
  - Before/after comparisons for refactorings.
- Omit implementation noise unless it is the point.

## mdBook 数学公式

Ranim Book 通过 `mdbook-typst-math` 将公式渲染为 SVG。行内公式使用
`$...$`，独立公式使用 `$$...$$`；分隔符中的内容必须是 **Typst math
syntax**，不能直接粘贴 LaTeX。

常用写法：

- 分式：`frac(a, b)`
- 矩阵：`mat(a, b; c, d)`，分号分隔行
- 黑板粗体：`RR`、`ZZ`
- 命名算子：`op("rank")`
- 粗体符号：`bold(v)`
- 点乘符号：`a dot b`

Typst 会把相邻字母解析成一个标识符。例如，矩阵 `A` 乘向量 `p` 应写成
`A p`，而不是 `Ap`。迁移 LaTeX 公式时，应按 Typst 的语义重新书写，不要只
替换命令名称。

修改公式后，从仓库根目录构建整本书进行验证：

```bash
nix develop --command mdbook build book
```

无需单独安装预处理器：Nix 开发环境和 Book CI 都固定使用上游
`duskmoon314/mdbook-typst-math` 的提交
`e310ec82ecaec5ae8e516ac07e9cab85fb506bc3`。升级该提交时，应同时更新
`flake.nix`、`.github/workflows/build-book.yml` 和 `.github/workflows/book.yml`，
并重新执行完整的书籍构建。
