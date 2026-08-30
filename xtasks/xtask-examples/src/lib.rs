use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use ranim_cli::cli::Cli;
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

/// The package and wasm names of the shared wasm scene bundle.
const EXAMPLES_BUNDLE_PACKAGE: &str = "ranim-examples";
/// Root directory of generated Zola example pages, relative to the website root.
const WEBSITE_EXAMPLES_CONTENT_DIR: &str = "content/examples";
/// Top-level title used when `content/examples/_index.md` does not exist yet.
const WEBSITE_EXAMPLES_TITLE: &str = "例子";
const EXAMPLES_BUNDLE_CRATE: &str = "ranim_examples";
const EXAMPLES_BUNDLE_WEB_DIR: &str = "ranim-examples";

fn clean_examples_wasm(root_dir: &Path) -> Result<()> {
    let output_dir = root_dir
        .join("website")
        .join("static")
        .join(EXAMPLES_BUNDLE_WEB_DIR)
        .join("pkg");
    if std::fs::exists(&output_dir)? {
        std::fs::remove_dir_all(&output_dir)
            .with_context(|| format!("failed to clean {}", output_dir.display()))?;
    }
    Ok(())
}

/// Build the shared wasm bundle containing every example scene.
///
/// The bundle is built once with the full feature set (`preview`, `anims`,
/// `items` and `typst`) and exposed to the website as
/// `website/static/ranim-examples/pkg/ranim_examples.js`.
pub fn build_examples_wasm(root_dir: impl AsRef<Path>) -> Result<()> {
    let root_dir = root_dir.as_ref();
    let website_root = root_dir.join("website");
    let output_dir = website_root.join("static").join(EXAMPLES_BUNDLE_WEB_DIR);
    std::fs::create_dir_all(&output_dir)?;

    clean_examples_wasm(root_dir)?;

    println!("[1/3] compiling {EXAMPLES_BUNDLE_PACKAGE} for wasm32-unknown-unknown");
    let status = Command::new("cargo")
        .current_dir(root_dir)
        .args([
            "build",
            "-p",
            EXAMPLES_BUNDLE_PACKAGE,
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ])
        .stdout(std::process::Stdio::null())
        .status()
        .context("failed to run cargo build for the wasm example bundle")?;
    if !status.success() {
        bail!("failed to build the wasm example bundle");
    }
    println!("[2/3] running wasm-bindgen");
    let wasm_path = root_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{EXAMPLES_BUNDLE_CRATE}.wasm"));
    let pkg_dir = output_dir.join("pkg");
    let status = Command::new("wasm-bindgen")
        .current_dir(root_dir)
        .args([
            "--out-dir",
            pkg_dir.as_os_str().to_str().unwrap(),
            "--target",
            "web",
            wasm_path.as_os_str().to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .status()
        .context("failed to run wasm-bindgen; is it installed?")?;
    if !status.success() {
        bail!("wasm-bindgen failed for the wasm example bundle");
    }

    println!("[3/3] optimizing wasm with wasm-opt");
    let bg_wasm = pkg_dir.join(format!("{EXAMPLES_BUNDLE_CRATE}_bg.wasm"));
    optimize_wasm(&bg_wasm).context("failed to optimize the wasm example bundle")?;

    clean_legacy_example_wasm_packages(root_dir)?;
    Ok(())
}

/// Remove per-example wasm packages left over from website builds that
/// predate the shared `ranim-examples` bundle.
fn clean_legacy_example_wasm_packages(root_dir: &Path) -> Result<()> {
    let examples_root = root_dir.join("website").join("static").join("examples");
    if !std::fs::exists(&examples_root)? {
        return Ok(());
    }

    for entry in std::fs::read_dir(&examples_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let pkg_dir = entry.path().join("pkg");
        if std::fs::exists(&pkg_dir)? {
            std::fs::remove_dir_all(&pkg_dir)
                .with_context(|| format!("failed to clean {}", pkg_dir.display()))?;
        }
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
        if std::fs::exists(&output_dir)? {
            std::fs::remove_dir_all(&output_dir)
                .with_context(|| format!("failed to clean {}", output_dir.display()))?;
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
        #[derive(Serialize, Deserialize)]
        struct OutputData {
            name: String,
            code: String,
            hash: String,

            preview_imgs: Vec<String>,
            output_files: Vec<String>,
            wasm: bool,
        }

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

        // `ranim output` renders every `#[output(...)]` declared by the
        // example (and processes `TimeMark::Capture`), which is exactly what
        // the website artifacts need. `ranim render` only exists for quick
        // single-scene smoke renders and requires an explicit scene name.
        let mut args = vec!["ranim", "output", "--example", &self.name];
        let features = self.required_features.join(",");
        if !features.is_empty() {
            args.extend(["--features", &features]);
        }

        // Output dirs in `#[output(...)]` are relative to the directory from
        // which `ranim output` is invoked, so run it from the workspace root.
        let previous_cwd = env::current_dir().context("failed to read current directory")?;
        if previous_cwd != root_dir {
            env::set_current_dir(root_dir).with_context(|| {
                format!("failed to set current directory to {}", root_dir.display())
            })?;
        }
        let render_result = Cli::parse_from(args)
            .run()
            .with_context(|| format!("failed to render example <{}>", self.name));
        if previous_cwd != root_dir {
            let _ = env::set_current_dir(previous_cwd);
        }
        render_result?;

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
    /// attributes, so nested examples such as `examples/agents/...` (and
    /// multi-dir examples such as `perlin_terrain`) work without duplicating
    /// their directory layout in the xtask.
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

/// Split synced output files into `preview_imgs` and `output_files` entries.
fn split_output_files(files: &[String]) -> (Vec<String>, Vec<String>) {
    let mut preview_imgs = vec![];
    let mut output_files = vec![];
    for file in files {
        let ext = Path::new(file)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        if PREVIEW_EXTENSIONS.contains(&ext) {
            preview_imgs.push(file.clone());
            output_files.push(file.clone());
        } else if WEBSITE_OUTPUT_EXTENSIONS.contains(&ext) {
            output_files.push(file.clone());
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
    #[derive(Serialize, Deserialize)]
    struct OutputData {
        name: String,
        code: String,
        hash: String,

        preview_imgs: Vec<String>,
        output_files: Vec<String>,
        wasm: bool,
    }

    files.sort();

    let (preview_imgs, output_files) = split_output_files(&files);
    let static_example_dir = root_dir
        .join("website")
        .join("static")
        .join("examples")
        .join(&example.name);
    let to_url = |file: &String| shadow.object_url(static_example_dir.join(file));
    let data = OutputData {
        name: example.name.clone(),
        code: example.code.clone(),
        hash: example.hash.clone(),
        preview_imgs: preview_imgs
            .iter()
            .map(to_url)
            .collect::<Result<Vec<_>>>()?,
        output_files: output_files
            .iter()
            .map(to_url)
            .collect::<Result<Vec<_>>>()?,
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
    #[serde(default)]
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

    let metadatas = manifest["package"]["metadata"]["example"]
        .clone()
        .try_into::<HashMap<String, ExampleMeta>>()
        .context("failed to parse [package.metadata.example]")?;

    manifest["example"]
        .as_array()
        .context("no [[example]] targets in workspace Cargo.toml")?
        .iter()
        .map(|v| v.as_table().unwrap())
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

            let path = root_dir.join(item["path"].as_str().unwrap());
            let code = fs::read_to_string(&path)
                .with_context(|| format!("failed to read example source {}", path.display()))?;
            let hash = {
                let mut hasher = Sha1::new();
                hasher.update(code.as_bytes());
                base16ct::lower::encode_string(&hasher.finalize())
            };
            let group = path
                .parent()
                .and_then(|dir| dir.strip_prefix(&examples_root).ok())
                .and_then(|relative| relative.parent())
                .map(Path::to_path_buf)
                .unwrap_or_default();
            Ok(Example {
                meta: metadatas.get(&name).cloned().unwrap_or_default(),
                name,
                group,
                path,
                code: format!("```rust\n{code}```"),
                hash,
                required_features,
            })
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_examples() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        assert_eq!(examples.len(), 36); // + iterative_spring, nbody, cloth_wrap, agents, gltf_showcase, z_fighting
        assert_eq!(
            examples
                .iter()
                .find(|example| example.name == "basic")
                .unwrap()
                .required_features,
            ["typst"]
        );
        assert_eq!(
            examples
                .iter()
                .find(|example| example.name == "arc")
                .unwrap()
                .group,
            PathBuf::new()
        );
        assert_eq!(
            examples
                .iter()
                .find(|example| example.name == "double_pendulum")
                .unwrap()
                .group,
            PathBuf::from("agents")
        );
        println!("found {} examples:", examples.len());
        examples.iter().for_each(|example| {
            println!("{}", example.name);
            println!("{:?}", example.meta)
        });
    }

    /// Renders an example end-to-end (requires a GPU); ignored by default.
    #[test]
    #[ignore]
    fn test_example_run() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        println!("{:?}", examples[0].name);
        examples[0].run(&root_dir, false).unwrap();
    }

    /// Builds the shared wasm scene bundle (requires the wasm32 target,
    /// wasm-bindgen and wasm-opt); ignored by default.
    #[test]
    #[ignore]
    fn test_example_build_wasm() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        build_examples_wasm(&root_dir).unwrap();
    }

    /// Destructive: deletes the example's website outputs; ignored by default.
    #[test]
    #[ignore]
    fn test_example_clean_output() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();
        println!("{:?}", examples[0].name);
        examples[0].clean_output(&root_dir).unwrap();
    }

    #[test]
    fn test_render_output_dirs_follow_example_layout() {
        let xtask_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_dir = xtask_root.join("../../");
        let examples = get_examples(&root_dir).unwrap();

        let double_pendulum = examples
            .iter()
            .find(|example| example.name == "double_pendulum")
            .unwrap();
        assert_eq!(
            double_pendulum.render_output_dirs(&root_dir).unwrap(),
            vec![root_dir.join("output/agents/double_pendulum")]
        );

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
    fn test_clean_website_examples() {
        let root_dir = std::env::temp_dir().join(format!(
            "ranim-xtask-clean-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root_dir);

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
        let root_dir = std::env::temp_dir().join(format!(
            "ranim-xtask-examples-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root_dir);

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
