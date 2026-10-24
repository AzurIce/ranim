use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use toml::Table;
use walkdir::WalkDir;

mod shadow;

/// File extensions synced from render outputs into the website.
const WEBSITE_OUTPUT_EXTENSIONS: &[&str] = &["png", "jpg", "mp4", "webm", /*"mov",*/ "gif"];
/// File extensions that count as preview images in `website/data/*.toml`.
const PREVIEW_EXTENSIONS: &[&str] = &["png", "jpg"];

fn copy_file(source: &Path, target_dir: &Path) -> Result<String> {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(anyhow::anyhow!("无法获取文件名"))?
        .to_string();

    let target_path = target_dir.join(&file_name);
    std::fs::copy(source, &target_path)?;

    Ok(file_name)
}

/// Root directory of generated Zola example pages, relative to the website root.
const WEBSITE_EXAMPLES_CONTENT_DIR: &str = "content/examples";
/// Top-level title used when `content/examples/_index.md` does not exist yet.
const WEBSITE_EXAMPLES_TITLE: &str = "例子";
/// Module basename shared by the individual WASM packages.
const EXAMPLES_WASM_MODULE: &str = "ranim_example";
/// Preview components and supporting modules shared by the website and rustdoc.
const PREVIEW_COMPONENTS: &[&str] = &[
    "ranim-theme.js",
    "ranim-audio.js",
    "ranim-viewport.js",
    "ranim-controls.js",
    "ranim-timeline.js",
];

/// Build each selected example's WASM cdylib and JavaScript bindings.
///
/// With no selection, builds all examples with a WASM target. Packages are
/// written to `<output_dir>/examples/<name>/pkg`, where `output_dir` defaults
/// to `website/static`. Rustdoc uses its own crate documentation directory.
/// Website builds also refresh source listings and pages, preserving published media.
pub fn build_examples_wasm(
    root_dir: impl AsRef<Path>,
    selected: &[String],
    output_dir: Option<&Path>,
) -> Result<()> {
    let root_dir = root_dir.as_ref();
    let examples = get_examples(root_dir)?;
    for name in selected {
        if !examples
            .iter()
            .any(|example| example.name == *name && example.meta.wasm)
        {
            bail!("example `{name}` has no WASM target");
        }
    }
    let output_dir = root_dir.join(output_dir.unwrap_or(Path::new("website/static")));
    let examples: Vec<_> = examples
        .iter()
        .filter(|example| {
            example.meta.wasm && (selected.is_empty() || selected.contains(&example.name))
        })
        .collect();
    // Cargo unifies features within one invocation. Keep Typst and other
    // optional dependencies out of examples that do not request them.
    let mut groups: BTreeMap<Vec<String>, Vec<&Example>> = BTreeMap::new();
    for example in &examples {
        let mut features = example.wasm_features.clone();
        features.sort();
        features.dedup();
        groups.entry(features).or_default().push(*example);
    }
    for (features, examples) in groups {
        build_wasm_group(root_dir, &features, &examples, &output_dir)?;
    }
    copy_preview_components(root_dir, &output_dir)?;
    if output_dir.canonicalize()? == root_dir.join("website/static").canonicalize()? {
        for example in examples {
            example.refresh_website_sources(root_dir)?;
        }
    }
    Ok(())
}

/// Reuse the website's components for other hosts, such as rustdoc.
fn copy_preview_components(root_dir: &Path, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    for name in PREVIEW_COMPONENTS {
        let source = root_dir
            .join("website/static")
            .join(name)
            .canonicalize()
            .with_context(|| format!("failed to find preview component {name}"))?;
        let target = output_dir.join(name);
        // Resolve aliases too: copying onto a source would truncate it.
        if target.canonicalize().ok().as_ref() != Some(&source) {
            fs::copy(&source, &target)
                .with_context(|| format!("failed to copy preview component {name}"))?;
        }
    }
    Ok(())
}

fn build_wasm_group(
    root_dir: &Path,
    features: &[String],
    examples: &[&Example],
    output_dir: &Path,
) -> Result<()> {
    let mut build = Command::new("cargo");
    build.env(
        "CARGO_BUILD_JOBS",
        std::env::var("CARGO_BUILD_JOBS").unwrap_or_else(|_| "8".to_string()),
    );
    build.current_dir(root_dir).args([
        "build",
        "-p",
        "ranim",
        "--target",
        "wasm32-unknown-unknown",
        "--release",
        "--no-default-features",
        "--message-format=json-render-diagnostics",
    ]);
    for example in examples {
        build.args(["--example", &format!("{}-wasm", example.name)]);
    }
    build.args(["--features", &features.join(",")]);
    println!(
        "[1/3] compiling {} WASM examples ({})",
        examples.len(),
        features.join(",")
    );
    let mut child = build
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run cargo build for WASM examples")?;
    let stdout = child
        .stdout
        .take()
        .context("Cargo stdout was not captured")?;
    // Use Cargo's reported paths, including custom CARGO_TARGET_DIR settings.
    let mut artifacts = HashMap::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.context("failed to read Cargo output")?;
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        if let (Some(name), Some(files)) = (
            message["target"]["name"].as_str(),
            message["filenames"].as_array(),
        ) && let Some(path) = files
            .iter()
            .filter_map(|file| file.as_str())
            .find(|file| file.ends_with(".wasm"))
        {
            artifacts.insert(name.to_string(), PathBuf::from(path));
        }
    }
    if !child.wait().context("failed to wait for Cargo")?.success() {
        bail!("failed to build WASM examples");
    }
    for example in examples {
        let target = format!("{}-wasm", example.name);
        let wasm_path = artifacts
            .get(&target)
            .with_context(|| format!("Cargo did not report a WASM artifact for {target}"))?;
        let pkg_dir = output_dir.join("examples").join(&example.name).join("pkg");
        fs::create_dir_all(&pkg_dir)?;
        println!("[2/3] running wasm-bindgen for {}", example.name);
        let status = Command::new("wasm-bindgen")
            .current_dir(root_dir)
            .args([
                "--target",
                "web",
                "--out-name",
                EXAMPLES_WASM_MODULE,
                "--out-dir",
            ])
            .arg(&pkg_dir)
            .arg(wasm_path)
            .status()
            .context("failed to run wasm-bindgen; is it installed?")?;
        if !status.success() {
            bail!("wasm-bindgen failed for {}", example.name);
        }
        println!("[3/3] optimizing {} with wasm-opt", example.name);
        optimize_wasm(&pkg_dir.join(format!("{EXAMPLES_WASM_MODULE}_bg.wasm")))?;
    }
    Ok(())
}

/// Optimize a wasm-bindgen output module for website delivery.
///
/// `wasm-bindgen` may rewrite the module it receives, so this must run after
/// bindgen has produced the final `*_bg.wasm` file in the package directory.
fn optimize_wasm(wasm_path: &Path) -> Result<()> {
    let optimized_path = wasm_path.with_extension("wasm-opt.wasm");
    if optimized_path.exists() {
        std::fs::remove_file(&optimized_path)
            .with_context(|| format!("failed to remove {}", optimized_path.display()))?;
    }

    let status = Command::new("wasm-opt")
        .args(["-Oz", "--output"])
        .arg(&optimized_path)
        .arg(wasm_path)
        .stdout(std::process::Stdio::null())
        .status()
        .context(
            "failed to run wasm-opt; install Binaryen and ensure `wasm-opt` is available on PATH",
        )?;
    if !status.success() {
        bail!("wasm-opt failed for {}", wasm_path.display());
    }

    std::fs::copy(&optimized_path, wasm_path).with_context(|| {
        format!(
            "failed to replace {} with wasm-opt output",
            wasm_path.display()
        )
    })?;
    std::fs::remove_file(&optimized_path)
        .with_context(|| format!("failed to remove {}", optimized_path.display()))?;
    Ok(())
}

/// What happened when [`Example::run`] was invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// The example was rendered and its website outputs were refreshed.
    Rendered,
    /// `--lazy-run` was enabled and the existing outputs were already current.
    UpToDate,
}

/// Removed stale website artifacts, split by artifact kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CleanStats {
    /// Removed generated Zola example pages.
    pub pages: usize,
    /// Removed generated `website/data/*.toml` files.
    pub data: usize,
    /// Removed generated `website/static/examples/*` directories.
    pub outputs: usize,
}

impl CleanStats {
    pub fn is_empty(&self) -> bool {
        self.pages == 0 && self.data == 0 && self.outputs == 0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Example {
    pub path: PathBuf,
    /// The name of the example, corresponding to the string passed to `--example`
    pub name: String,
    /// Path of the Zola section that owns this example, relative to
    /// `website/content/examples/`. Empty means the examples root section.
    pub group: PathBuf,
    pub code: String,
    pub hash: String,
    pub required_features: Vec<String>,
    /// Features required by the paired WASM cdylib, independent of native UI.
    pub wasm_features: Vec<String>,

    pub meta: ExampleMeta,
}

impl Example {
    pub fn clean_output(&self, root_dir: impl AsRef<Path>) -> Result<()> {
        let root_dir = root_dir.as_ref();

        let output_dir = root_dir
            .join("website")
            .join("static")
            .join("examples")
            .join(&self.name);
        if output_dir.exists() {
            for entry in fs::read_dir(&output_dir)? {
                let entry = entry?;
                // Rendering refreshes media while the independently built
                // WASM package remains available to the website.
                if entry.file_name() == "pkg" && entry.file_type()?.is_dir() {
                    continue;
                }
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }
        std::fs::create_dir_all(&output_dir)?;
        Ok(())
    }

    /// Render the scene and refresh `website/static/examples/<example-name>/`,
    /// `website/data/<example-name>.toml`, and the generated Zola page under
    /// `website/content/examples/<group>/<example-name>.md`.
    ///
    /// The render runs **before** any existing website output is cleaned, so a
    /// failed render leaves the previous outputs untouched.
    pub fn run(&self, root_dir: impl AsRef<Path>, lazy: bool) -> Result<RunStatus> {
        let root_dir = root_dir.as_ref();
        let website_root = root_dir.join("website");
        let output_dir = website_root
            .join("static")
            .join("examples")
            .join(&self.name);
        std::fs::create_dir_all(&output_dir)?;
        let data_dir = website_root.join("data");

        let data_path = data_dir.join(format!("{}.toml", self.name));
        // Missing or outdated data is not an error — it just means "regenerate".
        let up_to_date = std::fs::read_to_string(&data_path)
            .ok()
            .and_then(|data| toml::from_str::<OutputData>(&data).ok())
            .is_some_and(|data| data.hash == self.hash);
        if lazy && up_to_date {
            // The data is current, but the generated page may still be missing
            // after a hierarchy change or an interrupted previous run.
            if !self.meta.hide {
                self.create_example_page(root_dir)?;
            }
            return Ok(RunStatus::UpToDate);
        }

        // `<example> output` renders every `#[output(...)]` declared by the
        // example (and processes `TimeMark::Capture`), which is exactly what
        // the website artifacts need. `render <scene>` only exists for quick
        // single-scene smoke renders and requires an explicit scene name.
        let mut command = Command::new("cargo");
        command.args([
            "run",
            "--release",
            "--no-default-features",
            "--example",
            &self.name,
        ]);
        let features = self.required_features.join(",");
        if !features.is_empty() {
            command.args(["--features", &features]);
        }
        command.arg("--").arg("output");

        // Output dirs in `#[output(...)]` are relative to the working
        // directory of the example process, so run it from the workspace
        // root. stdio is inherited so the render progress bars stay live.
        let status = command
            .current_dir(root_dir)
            .status()
            .with_context(|| format!("failed to render example <{}>", self.name))?;
        if !status.success() {
            bail!("failed to render example <{}> ({status})", self.name);
        }

        self.clean_output(root_dir)?;

        let mut files = vec![];
        for render_dir in self.render_output_dirs(root_dir)? {
            if !fs::exists(&render_dir)? {
                continue;
            }
            for entry in WalkDir::new(&render_dir) {
                let entry = entry.with_context(|| {
                    format!("failed to walk render output {}", render_dir.display())
                })?;
                let path = entry.into_path();
                if !path.is_file() {
                    continue;
                }
                let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                if WEBSITE_OUTPUT_EXTENSIONS.contains(&ext) {
                    files.push(copy_file(&path, &output_dir)?);
                }
            }
        }

        // README-referenced images join the published set ahead of `shadow
        // publish` so their links resolve to shadow URLs like the renders.
        self.sync_readme_images(root_dir)?;
        let shadow = shadow::Shadow::load(root_dir)?;
        shadow.publish()?;

        write_example_data(root_dir, self, files, &shadow)?;
        if !self.meta.hide {
            self.create_example_page(root_dir)?;
        }
        Ok(RunStatus::Rendered)
    }

    /// Refresh source listings and WASM availability without rendering or publishing.
    fn refresh_website_sources(&self, root_dir: &Path) -> Result<()> {
        let data_dir = root_dir.join("website/data");
        fs::create_dir_all(&data_dir)?;
        let data_path = data_dir.join(format!("{}.toml", self.name));
        let mut data = if data_path.try_exists()? {
            toml::from_str::<OutputData>(&fs::read_to_string(&data_path)?)
                .with_context(|| format!("failed to parse {}", data_path.display()))?
        } else {
            OutputData::default()
        };
        data.name.clone_from(&self.name);
        data.code.clone_from(&self.code);
        data.wasm = self.meta.wasm;
        // `hash` describes the last media render. Updating it here would make
        // `run --lazy-run` incorrectly skip media for changed scenes.
        fs::write(&data_path, toml::to_string(&data)?)?;
        if !self.meta.hide {
            self.create_example_page(root_dir)?;
        }
        Ok(())
    }

    /// Generate (or refresh) the Zola page for this example.
    ///
    /// The page is written below `website/content/examples/<group>/` so the
    /// generated content tree mirrors the layout of the `examples/` directory.
    pub fn create_example_page(&self, root_dir: impl AsRef<Path>) -> Result<PathBuf> {
        let root_dir = root_dir.as_ref();
        let website_root = root_dir.join("website");
        let content_examples = website_root.join(WEBSITE_EXAMPLES_CONTENT_DIR);

        self.ensure_section_indexes(&content_examples)?;

        let page_dir = if self.group.as_os_str().is_empty() {
            content_examples.clone()
        } else {
            content_examples.join(&self.group)
        };
        fs::create_dir_all(&page_dir)
            .with_context(|| format!("failed to create {}", page_dir.display()))?;

        let output_page_path = page_dir.join(format!("{}.md", self.name));

        // If the example moved between groups, remove the stale page at its
        // previous location instead of leaving two pages with the same name.
        remove_stale_pages_for_example(&content_examples, &self.name, &output_page_path)?;

        let example_dir = self.path.parent().with_context(|| {
            format!(
                "example path {} has no parent directory",
                self.path.display()
            )
        })?;
        let readme_path = example_dir.join("README.md");

        // 创建markdown内容
        let mut md_content = format!(
            r#"+++
title = "{}"
template = "examples-page.html"
+++

"#,
            self.name
        );

        // 如果README.md存在，则复制其内容
        if readme_path.exists() {
            let readme_content = fs::read_to_string(readme_path)?;
            let static_example_dir = website_root
                .join("static")
                .join("examples")
                .join(&self.name);
            let readme_content = rewrite_readme_images(
                &readme_content,
                example_dir,
                &static_example_dir,
                &|file: &str| {
                    shadow::Shadow::try_object_url(root_dir, static_example_dir.join(file))
                        .unwrap_or_else(|| format!("/examples/{}/{}", self.name, file))
                },
            )?;
            md_content.push_str(&readme_content);

            // 确保README内容后有换行
            if !readme_content.ends_with('\n') {
                md_content.push('\n');
            }

            // 如果最后一行不是空行，添加一个空行
            if !md_content.ends_with("\n\n") {
                md_content.push('\n');
            }
        }

        // 添加示例标记
        md_content.push_str(&format!("!example-{}\n", self.name));

        // 写入markdown文件
        fs::write(&output_page_path, &md_content)
            .with_context(|| format!("failed to write {}", output_page_path.display()))?;
        Ok(output_page_path)
    }

    /// Copy README-referenced local images into the static example directory
    /// ahead of `shadow publish`, so their links resolve to shadow URLs just
    /// like the rendered outputs.
    fn sync_readme_images(&self, root_dir: &Path) -> Result<()> {
        let example_dir = self.path.parent().with_context(|| {
            format!(
                "example path {} has no parent directory",
                self.path.display()
            )
        })?;
        let readme_path = example_dir.join("README.md");
        if !readme_path.exists() {
            return Ok(());
        }
        let static_example_dir = root_dir
            .join("website")
            .join("static")
            .join("examples")
            .join(&self.name);
        let readme = fs::read_to_string(&readme_path)?;
        copy_readme_images(&readme, example_dir, &static_example_dir)
    }

    /// Directories that `ranim output` writes for this example.
    ///
    /// The directories are parsed from the example's `#[output(dir = "...")]`
    /// attributes, so multi-dir examples such as `perlin_terrain` work without
    /// duplicating their directory layout in the xtask.
    fn render_output_dirs(&self, root_dir: &Path) -> Result<Vec<PathBuf>> {
        let source = fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read example source {}", self.path.display()))?;
        let mut dirs = parse_output_dirs(&source)
            .into_iter()
            .map(|dir| {
                let dir = dir.strip_prefix("./").unwrap_or(&dir);
                root_dir.join(dir)
            })
            .collect::<Vec<_>>();

        if dirs.is_empty() {
            dirs.push(root_dir.join("output").join(&self.name));
        }

        let mut seen = HashSet::new();
        dirs.retain(|dir| seen.insert(dir.clone()));
        Ok(dirs)
    }

    fn ensure_section_indexes(&self, content_examples: &Path) -> Result<()> {
        // Keep the root examples section available even on fresh checkouts.
        ensure_section_index(content_examples, WEBSITE_EXAMPLES_TITLE)?;

        let mut current = content_examples.to_path_buf();
        for component in self.group.components() {
            current.push(component);
            let title = component.as_os_str().to_string_lossy();
            ensure_section_index(&current, &title)?;
        }
        Ok(())
    }
}

fn parse_output_dirs(source: &str) -> Vec<String> {
    let mut dirs = vec![];
    let mut rest = source;
    while let Some(attr_start) = rest.find("#[output(") {
        let attr_start = attr_start + "#[output(".len();
        let Some(attr_end) = rest[attr_start..].find(")]") else {
            break;
        };
        let attr_end = attr_start + attr_end;
        let attr = &rest[attr_start..attr_end];

        let mut cursor = 0;
        while let Some(dir_start) = attr[cursor..].find("dir") {
            let dir_start = cursor + dir_start;
            let after_dir = &attr[dir_start + "dir".len()..];
            let Some(eq) = after_dir.find('=') else {
                break;
            };
            let value = after_dir[eq + 1..].trim_start();
            let Some(value) = value.strip_prefix('"') else {
                cursor = dir_start + "dir".len();
                continue;
            };
            let Some(end) = value.find('"') else {
                break;
            };
            dirs.push(value[..end].to_string());
            cursor = dir_start + "dir".len() + eq + 1 + end + 1;
        }

        rest = &rest[attr_end + 2..];
    }
    dirs
}

/// Rewrite local markdown image references from an example README so Zola
/// serves them from `website/static/examples/<name>/`.
///
/// Source READMEs use `![...](preview.png)`, which Zola would resolve relative
/// to the generated content page (`/examples/agents/<name>/preview.png`) and
/// break. This rewrites those links through `resolve_url` — normally the
/// shadow content-addressed URL, falling back to
/// `/examples/<name>/<file>` — and copies missing source images into the
/// static example directory as a fallback.
fn rewrite_readme_images(
    readme: &str,
    example_dir: &Path,
    static_example_dir: &Path,
    resolve_url: &dyn Fn(&str) -> String,
) -> Result<String> {
    let mut result = String::with_capacity(readme.len());
    let mut cursor = 0usize;

    while let Some((path_start, path_end)) = next_markdown_image(readme, cursor) {
        let path = readme[path_start..path_end].trim();
        result.push_str(&readme[cursor..path_start]);

        let local_path = path.strip_prefix("./").unwrap_or(path);
        let source_path = example_dir.join(local_path);
        let rewritten = if local_path.contains('/') || !source_path.is_file() {
            path.to_string()
        } else if let Some(file_name) = source_path.file_name().and_then(|name| name.to_str()) {
            if !static_example_dir.join(file_name).exists() {
                fs::create_dir_all(static_example_dir).with_context(|| {
                    format!("failed to create {}", static_example_dir.display())
                })?;
                fs::copy(&source_path, static_example_dir.join(file_name)).with_context(|| {
                    format!(
                        "failed to copy {} to {}",
                        source_path.display(),
                        static_example_dir.display()
                    )
                })?;
            }
            resolve_url(file_name)
        } else {
            path.to_string()
        };

        result.push_str(&rewritten);
        cursor = path_end;
    }

    result.push_str(&readme[cursor..]);
    Ok(result)
}

fn next_markdown_image(source: &str, start: usize) -> Option<(usize, usize)> {
    let link_start = source[start..].find("](")? + start;
    let path_start = link_start + 2;
    let path_end = source[path_start..].find(')')? + path_start;
    Some((path_start, path_end))
}

/// Copy README-referenced local images into the static example directory so
/// they get published with the rest of the example outputs.
fn copy_readme_images(readme: &str, example_dir: &Path, static_example_dir: &Path) -> Result<()> {
    let mut cursor = 0usize;
    while let Some((path_start, path_end)) = next_markdown_image(readme, cursor) {
        let path = readme[path_start..path_end].trim();
        cursor = path_end;

        let local_path = path.strip_prefix("./").unwrap_or(path);
        if local_path.contains('/') {
            continue;
        }
        let source_path = example_dir.join(local_path);
        if !source_path.is_file() {
            continue;
        }
        let Some(file_name) = source_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !static_example_dir.join(file_name).exists() {
            fs::create_dir_all(static_example_dir)
                .with_context(|| format!("failed to create {}", static_example_dir.display()))?;
            fs::copy(&source_path, static_example_dir.join(file_name)).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    static_example_dir.display()
                )
            })?;
        }
    }
    Ok(())
}

/// One entry of `output_files` in `website/data/*.toml`.
///
/// Content-addressed URLs carry no file extension, so the MIME type derived
/// from the rendered file's extension is stored alongside and the website
/// branches on it instead of the URL shape.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OutputFile {
    pub url: String,
    pub kind: String,
}

/// Shared website data for media publication and source-only refreshes.
#[derive(Serialize, Deserialize, Default)]
struct OutputData {
    name: String,
    code: String,
    hash: String,
    preview_imgs: Vec<String>,
    output_files: Vec<OutputFile>,
    wasm: bool,
}

fn mime_for_extension(ext: &str) -> &'static str {
    match ext {
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "gif" => "image/gif",
        "jpg" => "image/jpeg",
        _ => "image/png",
    }
}

/// Split synced output files into `preview_imgs` and `output_files` entries.
fn split_output_files(files: &[String]) -> (Vec<String>, Vec<OutputFile>) {
    let mut preview_imgs = vec![];
    let mut output_files = vec![];
    for file in files {
        let ext = Path::new(file)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        if PREVIEW_EXTENSIONS.contains(&ext) {
            preview_imgs.push(file.clone());
        }
        if WEBSITE_OUTPUT_EXTENSIONS.contains(&ext) {
            output_files.push(OutputFile {
                url: file.clone(),
                kind: mime_for_extension(ext).to_string(),
            });
        }
    }
    (preview_imgs, output_files)
}

/// Write `website/data/<name>.toml`, referencing outputs by their
/// content-addressed shadow URLs.
///
/// Every referenced file must have been published already — [`Example::run`]
/// and [`publish_website_outputs`] both run `shadow publish` beforehand.
fn write_example_data(
    root_dir: &Path,
    example: &Example,
    mut files: Vec<String>,
    shadow: &shadow::Shadow,
) -> Result<()> {
    files.sort();

    let (preview_imgs, mut output_files) = split_output_files(&files);
    let static_example_dir = root_dir
        .join("website")
        .join("static")
        .join("examples")
        .join(&example.name);
    let to_url = |file: &String| shadow.object_url(static_example_dir.join(file));
    let preview_imgs = preview_imgs
        .iter()
        .map(to_url)
        .collect::<Result<Vec<_>>>()?;
    for entry in &mut output_files {
        entry.url = to_url(&entry.url)?;
    }
    let data = OutputData {
        name: example.name.clone(),
        code: example.code.clone(),
        hash: example.hash.clone(),
        preview_imgs,
        output_files,
        wasm: example.meta.wasm,
    };

    let data_dir = root_dir.join("website").join("data");
    fs::create_dir_all(&data_dir)?;
    let data_path = data_dir.join(format!("{}.toml", example.name));
    fs::write(&data_path, toml::to_string(&data)?)
        .with_context(|| format!("failed to write {}", data_path.display()))?;
    Ok(())
}

/// Refresh website data and generated pages from the **existing**
/// `website/static/examples/<name>/` outputs, linking them through shadow.
///
/// Unlike [`Example::run`] this never renders; it publishes whatever is on
/// disk (usually the output of a previous render) and rewrites
/// `website/data/*.toml` plus the generated pages to reference
/// content-addressed object URLs.
pub fn publish_website_outputs(root_dir: impl AsRef<Path>, examples: &[Example]) -> Result<()> {
    let root_dir = root_dir.as_ref();
    let shadow = shadow::Shadow::load(root_dir)?;
    shadow.publish()?;

    let static_examples = root_dir.join("website").join("static").join("examples");
    for example in examples {
        let output_dir = static_examples.join(&example.name);
        if !fs::exists(&output_dir)? {
            continue;
        }
        let mut files = vec![];
        for entry in WalkDir::new(&output_dir) {
            let path = entry
                .with_context(|| format!("failed to walk {}", output_dir.display()))?
                .into_path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            if WEBSITE_OUTPUT_EXTENSIONS.contains(&ext) {
                files.push(path.file_name().unwrap().to_string_lossy().into_owned());
            }
        }
        if files.is_empty() {
            continue;
        }

        write_example_data(root_dir, example, files, &shadow)?;
        if !example.meta.hide {
            example.create_example_page(root_dir)?;
        }
        println!("[published] {}", example.name);
    }
    Ok(())
}

fn ensure_section_index(dir: &Path, title: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let index_path = dir.join("_index.md");
    if index_path.exists() {
        return Ok(());
    }

    let title = title.replace('\"', "\\\"");
    let frontmatter = format!(
        "+++\ntitle = \"{title}\"\ntemplate = \"examples.html\"\nsort_by = \"slug\"\ninsert_anchor_links = \"right\"\n+++\n"
    );
    fs::write(&index_path, frontmatter)
        .with_context(|| format!("failed to write {}", index_path.display()))?;
    Ok(())
}

/// Remove generated pages for `name` anywhere below `content/examples`, except
/// the path that is about to be written.
fn remove_stale_pages_for_example(content_examples: &Path, name: &str, keep: &Path) -> Result<()> {
    let page_file_name = format!("{name}.md");
    for entry in WalkDir::new(content_examples).min_depth(1) {
        let entry =
            entry.with_context(|| format!("failed to walk {}", content_examples.display()))?;
        let path = entry.into_path();
        if !path.is_file() {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some(page_file_name.as_str())
            && path != keep
        {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }
    Ok(())
}

/// Remove website artifacts that do not correspond to any current example.
///
/// Generated example pages are only removed when they look like pages produced
/// by this xtask, so hand-written pages below `content/examples` are preserved.
pub fn clean_website_examples(
    root_dir: impl AsRef<Path>,
    examples: &[Example],
) -> Result<CleanStats> {
    let root_dir = root_dir.as_ref();
    let website_root = root_dir.join("website");
    let current: HashSet<&str> = examples
        .iter()
        .map(|example| example.name.as_str())
        .collect();
    let mut stats = CleanStats::default();

    let content_examples = website_root.join(WEBSITE_EXAMPLES_CONTENT_DIR);
    if fs::exists(&content_examples)? {
        for entry in WalkDir::new(&content_examples).min_depth(1) {
            let entry =
                entry.with_context(|| format!("failed to walk {}", content_examples.display()))?;
            let path = entry.into_path();
            if !path.is_file() {
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) == Some("_index.md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            if current.contains(stem) || !is_generated_example_page(&path)? {
                continue;
            }
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            stats.pages += 1;
        }
    }

    let data_dir = website_root.join("data");
    if fs::exists(&data_dir)? {
        for entry in fs::read_dir(&data_dir)
            .with_context(|| format!("failed to read {}", data_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            if current.contains(stem) {
                continue;
            }
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            stats.data += 1;
        }
    }

    let static_examples = website_root.join("static").join("examples");
    if fs::exists(&static_examples)? {
        for entry in fs::read_dir(&static_examples)
            .with_context(|| format!("failed to read {}", static_examples.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().into_string().ok() else {
                continue;
            };
            if current.contains(name.as_str()) {
                continue;
            }
            fs::remove_dir_all(entry.path())
                .with_context(|| format!("failed to remove {}", entry.path().display()))?;
            stats.outputs += 1;
        }
    }

    Ok(stats)
}

fn is_generated_example_page(path: &Path) -> Result<bool> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(content.contains("template = \"examples-page.html\""))
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExampleMeta {
    #[serde(skip)]
    wasm: bool,
    #[serde(default)]
    hide: bool,
}

pub fn get_examples(root_dir: impl AsRef<Path>) -> Result<Vec<Example>> {
    let root_dir = root_dir.as_ref();
    let examples_root = root_dir.join("examples");

    let manifest_path = root_dir.join("Cargo.toml");
    let manifest_file = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;

    let manifest = manifest_file
        .parse::<Table>()
        .context("failed to parse workspace Cargo.toml")?;

    let metadatas = manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("example"))
        .cloned()
        .unwrap_or_else(|| toml::Value::Table(Table::new()))
        .try_into::<HashMap<String, ExampleMeta>>()
        .context("failed to parse [package.metadata.example]")?;

    let targets = manifest["example"]
        .as_array()
        .context("no [[example]] targets in workspace Cargo.toml")?;

    targets
        .iter()
        .map(|v| v.as_table().unwrap())
        // The `<name>-wasm` cdylib entries are the WASM hosts of the examples,
        // not renderable examples themselves.
        .filter(|item| !item["name"].as_str().unwrap().ends_with("-wasm"))
        .map(|item| {
            let name = item["name"].as_str().unwrap().to_string();
            let required_features = item
                .get("required-features")
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect();

            let entry_path = root_dir.join(item["path"].as_str().unwrap());
            let path = entry_path.with_file_name("scenes.rs");
            let wasm_target = targets
                .iter()
                .find(|target| target["name"].as_str() == Some(&format!("{name}-wasm")));
            let wasm_features = wasm_target
                .and_then(|target| target.get("required-features"))
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect();
            let mut meta = metadatas.get(&name).cloned().unwrap_or_default();
            meta.wasm = wasm_target.is_some();
            if let Some(target) = wasm_target {
                let expected_path = entry_path.with_file_name("wasm.rs");
                let target_path =
                    root_dir.join(target["path"].as_str().context("WASM target has no path")?);
                if target_path != expected_path
                    || target["crate-type"]
                        .as_array()
                        .is_none_or(|types| types.len() != 1 || types[0].as_str() != Some("cdylib"))
                {
                    bail!(
                        "{name}-wasm must be a cdylib at {}",
                        expected_path.display()
                    );
                }
            }
            let mut hasher = Sha1::new();
            let mut code = String::new();
            for (file, label) in [
                ("scenes.rs", "共享场景与项目装配"),
                ("main.rs", "原生入口"),
                ("wasm.rs", "WASM 入口"),
            ] {
                if file == "wasm.rs" && !meta.wasm {
                    continue;
                }
                let source_path = entry_path.with_file_name(file);
                let source = fs::read_to_string(&source_path)
                    .with_context(|| format!("failed to read {}", source_path.display()))?;
                hasher.update(source.as_bytes());
                if !code.is_empty() {
                    code.push_str("\n\n");
                }
                code.push_str(&format!("`{file}`（{label}）\n\n```rust\n{source}```"));
            }
            let hash = base16ct::lower::encode_string(&hasher.finalize());
            let group = path
                .parent()
                .and_then(|dir| dir.strip_prefix(&examples_root).ok())
                .and_then(|relative| relative.parent())
                .map(Path::to_path_buf)
                .unwrap_or_default();
            Ok(Example {
                meta,
                name,
                group,
                path,
                code,
                hash,
                required_features,
                wasm_features,
            })
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ranim-xtask-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_preview_components_copy_without_overwriting_their_sources() {
        let root = temp_root("player-component");
        let website = root.join("website/static");
        fs::create_dir_all(&website).unwrap();
        for name in PREVIEW_COMPONENTS {
            fs::write(website.join(name), format!("// {name}\n")).unwrap();
        }

        copy_preview_components(&root, &website).unwrap();
        copy_preview_components(&root, &website.join("../static")).unwrap();

        let docs = root.join("docs/ranim_examples");
        copy_preview_components(&root, &docs).unwrap();
        for name in PREVIEW_COMPONENTS {
            let contents = format!("// {name}\n");
            assert_eq!(fs::read_to_string(website.join(name)).unwrap(), contents);
            assert_eq!(fs::read_to_string(docs.join(name)).unwrap(), contents);
            fs::write(website.join(name), format!("// updated {name}\n")).unwrap();
        }
        copy_preview_components(&root, &docs).unwrap();
        for name in PREVIEW_COMPONENTS {
            let updated = format!("// updated {name}\n");
            assert_eq!(fs::read_to_string(website.join(name)).unwrap(), updated);
            assert_eq!(fs::read_to_string(docs.join(name)).unwrap(), updated);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_get_examples() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        for entry in fs::read_dir(root_dir.join("examples")).unwrap() {
            let path = entry.unwrap().path();
            if path.join("main.rs").exists() {
                assert!(
                    examples
                        .iter()
                        .any(|example| example.path == path.join("scenes.rs")),
                    "unregistered example entry: {}",
                    path.display()
                );
            }
        }
        assert_eq!(
            examples
                .iter()
                .find(|example| example.name == "basic")
                .unwrap()
                .required_features,
            ["anims", "cmd-render", "typst"]
        );
        let arc = examples
            .iter()
            .find(|example| example.name == "arc")
            .unwrap();
        assert_eq!(arc.group, PathBuf::new());
        assert_eq!(arc.path.file_name().unwrap(), "scenes.rs");
        assert!(arc.code.contains("fn arc(r: &mut RanimScene)"));
        assert!(arc.code.starts_with("`scenes.rs`"));
        assert!(arc.code.contains("`main.rs`（原生入口）"));
        assert!(arc.code.contains("`wasm.rs`（WASM 入口）"));
        assert!(arc.meta.wasm);
        assert_eq!(arc.wasm_features, ["anims", "render"]);
        assert!(
            examples
                .iter()
                .all(|example| !example.name.ends_with("-wasm"))
        );
        assert!(examples.iter().all(|example| example.meta.wasm));
        assert_eq!(
            examples
                .iter()
                .find(|example| example.name == "gltf_showcase")
                .unwrap()
                .required_features,
            ["anims", "cmd-render", "gltf"]
        );
        assert_eq!(
            examples
                .iter()
                .find(|example| example.name == "gltf_showcase")
                .unwrap()
                .wasm_features,
            ["anims", "render", "gltf"]
        );
        assert!(
            arc.code
                .contains("app.add_plugins(scenes::ScenesPlugin);\n    run_cli(app.build());")
        );
    }

    #[test]
    fn test_website_source_refresh_preserves_rendered_media_and_hash() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut example = get_examples(workspace)
            .unwrap()
            .into_iter()
            .find(|example| example.name == "arc")
            .unwrap();
        let root = temp_root("source-refresh");
        example.path = root.join("examples/arc/scenes.rs");
        fs::create_dir_all(example.path.parent().unwrap()).unwrap();

        example.refresh_website_sources(&root).unwrap();
        let data_path = root.join("website/data/arc.toml");
        let mut data: OutputData =
            toml::from_str(&fs::read_to_string(&data_path).unwrap()).unwrap();
        assert_eq!(data.code, example.code);
        assert!(data.wasm);
        assert!(data.hash.is_empty());
        assert!(data.output_files.is_empty());
        assert!(root.join("website/content/examples/arc.md").exists());

        data.hash = "previous-media-source-hash".into();
        data.preview_imgs = vec!["https://media.example/preview-oid".into()];
        data.output_files = vec![OutputFile {
            url: "https://media.example/video-oid".into(),
            kind: "video/mp4".into(),
        }];
        fs::write(&data_path, toml::to_string(&data).unwrap()).unwrap();
        example.code = "updated shared scenes and entry points".into();
        example.refresh_website_sources(&root).unwrap();
        let refreshed: OutputData =
            toml::from_str(&fs::read_to_string(&data_path).unwrap()).unwrap();
        assert_eq!(refreshed.code, example.code);
        assert_eq!(refreshed.hash, data.hash);
        assert_eq!(refreshed.preview_imgs, data.preview_imgs);
        assert_eq!(refreshed.output_files, data.output_files);

        example.name = "hidden".into();
        example.meta.hide = true;
        example.refresh_website_sources(&root).unwrap();
        assert!(root.join("website/data/hidden.toml").exists());
        assert!(!root.join("website/content/examples/hidden.md").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_wasm_targets_require_render_and_shared_scene_features() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest = fs::read_to_string(root.join("Cargo.toml"))
            .unwrap()
            .parse::<Table>()
            .unwrap();
        let targets = manifest["example"].as_array().unwrap();
        for target in targets
            .iter()
            .filter(|target| target["name"].as_str().unwrap().ends_with("-wasm"))
        {
            let name = target["name"]
                .as_str()
                .unwrap()
                .strip_suffix("-wasm")
                .unwrap();
            let native = targets
                .iter()
                .find(|target| target["name"].as_str() == Some(name))
                .unwrap();
            let features: Vec<_> = native["required-features"]
                .as_array()
                .unwrap()
                .iter()
                .map(|feature| {
                    if feature.as_str() == Some("cmd-render") {
                        toml::Value::String("render".into())
                    } else {
                        feature.clone()
                    }
                })
                .collect();
            assert_eq!(
                target["required-features"].as_array().unwrap(),
                &features,
                "{name}"
            );
            assert_eq!(
                target["crate-type"].as_array().unwrap(),
                &[toml::Value::String("cdylib".into())]
            );
        }
    }

    #[test]
    fn test_wasm_entry_changes_invalidate_source_hash() {
        let root = temp_root("wasm-hash");
        let source_dir = root.join("examples/foo");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"
[package]
name = "ranim"
[[example]]
name = "foo"
path = "examples/foo/main.rs"
[[example]]
name = "foo-wasm"
path = "examples/foo/wasm.rs"
crate-type = ["cdylib"]
"#,
        )
        .unwrap();
        fs::write(source_dir.join("scenes.rs"), "// shared assembly\n").unwrap();
        fs::write(source_dir.join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(source_dir.join("wasm.rs"), "// first entry\n").unwrap();
        let before = get_examples(&root).unwrap();
        fs::write(source_dir.join("wasm.rs"), "// changed entry\n").unwrap();
        let after = get_examples(&root).unwrap();
        assert_eq!(before.len(), 1);
        assert!(after[0].meta.wasm);
        assert_ne!(before[0].hash, after[0].hash);
        assert!(after[0].code.contains("// changed entry"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_render_output_dirs_follow_example_layout() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();

        let perlin_terrain = examples
            .iter()
            .find(|example| example.name == "perlin_terrain")
            .unwrap();
        let dirs = perlin_terrain.render_output_dirs(&root_dir).unwrap();
        assert!(dirs.contains(&root_dir.join("output/perlin_terrain/erosion")));
        assert!(dirs.contains(&root_dir.join("output/perlin_terrain/fractal")));
        assert!(dirs.contains(&root_dir.join("output/perlin_terrain/perlin")));
    }

    #[test]
    fn test_split_output_files_assigns_kinds() {
        let files = ["preview.png", "scene.mp4", "clip.webm", "anim.gif"].map(str::to_owned);
        let (previews, outputs) = split_output_files(&files);
        assert_eq!(previews, ["preview.png"]);
        let kinds: Vec<_> = outputs
            .iter()
            .map(|file| (file.url.as_str(), file.kind.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                ("preview.png", "image/png"),
                ("scene.mp4", "video/mp4"),
                ("clip.webm", "video/webm"),
                ("anim.gif", "image/gif")
            ]
        );
    }

    #[test]
    fn test_render_cleanup_preserves_wasm_package() {
        let root = temp_root("render-clean");
        let output = root.join("website/static/examples/foo");
        fs::create_dir_all(output.join("pkg")).unwrap();
        fs::create_dir_all(output.join("stale")).unwrap();
        fs::write(output.join("preview.png"), "old render").unwrap();
        fs::write(output.join("pkg/ranim_example.js"), "WASM bindings").unwrap();
        let example = Example {
            name: "foo".into(),
            path: root.join("examples/foo/scenes.rs"),
            group: PathBuf::new(),
            code: String::new(),
            hash: String::new(),
            required_features: vec![],
            wasm_features: vec![],
            meta: ExampleMeta::default(),
        };
        example.clean_output(&root).unwrap();
        assert!(output.join("pkg/ranim_example.js").is_file());
        assert!(!output.join("preview.png").exists());
        assert!(!output.join("stale").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_clean_website_examples() {
        let root_dir = temp_root("clean");

        let content = root_dir.join("website").join(WEBSITE_EXAMPLES_CONTENT_DIR);
        let data = root_dir.join("website").join("data");
        let outputs = root_dir.join("website").join("static").join("examples");
        fs::create_dir_all(&content).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&outputs).unwrap();

        let generated_page = |name: &str| {
            format!("+++\ntitle = \"{name}\"\ntemplate = \"examples-page.html\"\n+++\n")
        };
        fs::write(content.join("stale.md"), generated_page("stale")).unwrap();
        fs::write(content.join("foo.md"), generated_page("foo")).unwrap();
        fs::write(content.join("manual.md"), "+++\ntitle = \"manual\"\n+++\n").unwrap();
        fs::write(data.join("stale.toml"), "name = \"stale\"\n").unwrap();
        fs::write(data.join("foo.toml"), "name = \"foo\"\n").unwrap();
        fs::create_dir_all(outputs.join("stale")).unwrap();
        fs::create_dir_all(outputs.join("foo")).unwrap();

        let examples = vec![Example {
            path: root_dir.join("examples/foo/lib.rs"),
            name: "foo".to_string(),
            group: PathBuf::new(),
            code: "```rust\nfn main() {}\n```".to_string(),
            hash: "deadbeef".to_string(),
            required_features: vec![],
            wasm_features: vec![],
            meta: ExampleMeta::default(),
        }];

        let stats = clean_website_examples(&root_dir, &examples).unwrap();
        assert_eq!(stats.pages, 1);
        assert_eq!(stats.data, 1);
        assert_eq!(stats.outputs, 1);

        assert!(!content.join("stale.md").exists());
        assert!(content.join("foo.md").exists());
        assert!(content.join("manual.md").exists());
        assert!(!data.join("stale.toml").exists());
        assert!(data.join("foo.toml").exists());
        assert!(!outputs.join("stale").exists());
        assert!(outputs.join("foo").exists());

        let _ = fs::remove_dir_all(&root_dir);
    }

    #[test]
    fn test_create_nested_example_page() {
        let root_dir = temp_root("page");

        let example_dir = root_dir.join("examples").join("agents").join("foo");
        fs::create_dir_all(&example_dir).unwrap();
        fs::write(example_dir.join("lib.rs"), "fn main() {}\n").unwrap();
        fs::write(
            example_dir.join("README.md"),
            "# Foo Example\n\n![preview](preview.png)\n",
        )
        .unwrap();
        fs::write(example_dir.join("preview.png"), "not-a-real-png").unwrap();

        let example = Example {
            path: example_dir.join("lib.rs"),
            name: "foo".to_string(),
            group: PathBuf::from("agents"),
            code: "```rust\nfn main() {}\n```".to_string(),
            hash: "deadbeef".to_string(),
            required_features: vec![],
            wasm_features: vec![],
            meta: ExampleMeta::default(),
        };

        let page = example.create_example_page(&root_dir).unwrap();
        assert_eq!(
            page,
            root_dir
                .join("website")
                .join(WEBSITE_EXAMPLES_CONTENT_DIR)
                .join("agents")
                .join("foo.md")
        );
        let page_content = fs::read_to_string(&page).unwrap();
        assert!(page_content.contains("template = \"examples-page.html\""));
        assert!(page_content.contains("!example-foo"));
        assert!(page_content.contains("# Foo Example"));
        assert!(page_content.contains("](/examples/foo/preview.png)"));
        assert!(page_content.ends_with("!example-foo\n"));
        assert!(
            root_dir
                .join("website/static/examples/foo/preview.png")
                .exists()
        );
        assert!(
            root_dir
                .join("website")
                .join(WEBSITE_EXAMPLES_CONTENT_DIR)
                .join("_index.md")
                .exists()
        );
        assert!(
            root_dir
                .join("website")
                .join(WEBSITE_EXAMPLES_CONTENT_DIR)
                .join("agents")
                .join("_index.md")
                .exists()
        );

        let _ = fs::remove_dir_all(&root_dir);
    }
}
