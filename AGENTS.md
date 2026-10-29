# AGENTS

## Lints

**Run all Rust tooling inside the Nix development shell.** The flake pins its
Rust to the exact nightly CI uses (`nightly-2026-10-01`, see
`.github/workflows/build.yml`); an arbitrary host cargo has a different lint
set and will disagree with CI.

```bash
nix develop
just lint   # workspace clippy/doc + isolated minimal and WASM feature checks
```

Gotchas learned the hard way — partial sweeps are NOT equivalent:

- Per-package checks (`cargo clippy -p <pkg> --all-targets`) miss `benches`,
  `example-packages/*`, and the `packages/ranim-examples` sources, which only
  compile under the workspace-wide sweep.
- Feature-gated `[[example]]` targets are silently skipped by
  `cargo check/clippy -p ranim --examples` when their features are off.
- If a lint failure shows up in CI, fix it on the branch whose PR failed;
  fixing it in a stacked working tree leaves the PR red.
- When changing the pinned toolchain, update it in both places at once:
  the CI workflows and `flake.nix`.

## Build & Test Notes

- **Limit cargo parallelism to avoid OOM.** A full `cargo test --workspace` or
  `cargo build --workspace --all-targets` compiles the entire workspace
  (wgpu, eframe/egui, 25+ examples) with one rustc job per core, which
  can exhaust memory and freeze the machine (observed with 32 cores / 29 GiB
  RAM). Prefer limiting parallelism and/or testing per package, e.g.:

  ```bash
  cargo test -p ranim-core -p ranim-items -j 8
  CARGO_BUILD_JOBS=8 cargo run --example <example> --no-default-features --features anims,cmd-render -- output
  ```

## 构建与测试注意事项

- **限制 cargo 并行度以避免内存耗尽。**全量 `cargo test --workspace` 或
  `cargo build --workspace --all-targets` 会按核心数并行编译整个 workspace
  （wgpu、eframe/egui、25+ 个 example），可能耗尽内存导致整机卡死
  （在 32 核 / 29 GiB 内存上实际发生过）。建议限制并行度或分包测试。

## Website example media（shadow 管理）

`website/static/examples/` 下的渲染产物（mp4/png）**不入库**，由
[shadow](https://github.com/AzurIce/shadow) 管理：xtask 在渲染后运行
`shadow publish` 上传到 TOS。SSG 构建时通过 Experimental prepare 扩展
`cargo examples site` 从 Cargo targets、源码、README 和 `.shadow/refs/` 生成
内存中的页面与模板数据，媒体 URL 由 refs 中的 oid 解析。
入库的是 ref、`shadow.toml` 和 `website/render-state.json`（上次成功渲染的 hash）；
不再维护 `website/data/*.toml` 或 `website/content/examples/`。

- 构建网站：`just website`，扩展自动生成 example 页面与数据，无需渲染、下载媒体或凭证。
- WASM：`cargo examples build [examples...]`；实际存在的 JS/WASM 决定是否显示 App 页签。
- 渲染 + 发布：`cargo examples run [examples...]`（需要 `shadow` 在 PATH
  上，`cargo install --git https://github.com/AzurIce/shadow`）；成功后更新 render state。
- 不渲染、只发布已有产物：`cargo examples publish`，保持上次成功渲染的 hash。
- `--lazy-run` 仅比较 render state；网站与 WASM 构建不得覆盖该 hash。
- 凭证在仓库根 `.env`（`TOS_ACCESS_KEY`/`TOS_SECRET_KEY`，已 gitignore，
  绝不提交）
- CI 在每次网站部署成功后运行 `shadow free --keep-at origin/main --delete`
  释放被替代的旧对象（30 天宽限期保护回滚与在途上传）

## PR Authoring Guidelines

This file describes how to write and update pull request descriptions in this
repository. It applies to both human contributors and AI coding agents.

### Language

- PR titles and commit messages are always written in **English**.
- The PR description is fully bilingual: write the **entire** description in
  English first, then repeat the **entire** description in Chinese, separated
  by a single `---` divider. "Entire" means every section — the summary
  bullets, the Breaking Changes section, and all detail sections alike.
  Translating only the top summary is the most common mistake and does not
  count as bilingual.

### PR Description Format

When creating or editing a PR, follow this structure. It shows **one
language half**; the finished description is this structure twice — English
first, then Chinese after the single `---` divider (see **The bilingual
split** below):

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

## Component/Feature 1

Detailed description of the first major change.

Use code blocks, mermaid diagrams, or examples as needed.

## Component/Feature 2

...
```

#### The bilingual split

- The `---` divider appears exactly **once**, between the English half and
  the Chinese half — never between the summary and the detail sections.
- Below the divider, repeat the document above **in full and in the same
  order**: summary bullets → Breaking Changes → every detail section,
  heading for heading. Only the language changes; the content stays
  identical.

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

## Book 数学公式与 issue 引用

Book 使用 notist-book 构建，Typst 和 Pele 组件位于 `book/_components/`。
行内公式使用 `$...$`（Typst math syntax）；独立公式使用
`#typst.math<BlockContent>(...)` 并导入 typst 组件。图表使用
`#pele.diagram(...)`。issue/PR 引用使用 `#issue("123")`，并导入
相对路径的 `_components/issue.notc`；链接指向 GitHub `/pull/N`。

修改 Book 后运行 `nix develop --command just book` 验证。
