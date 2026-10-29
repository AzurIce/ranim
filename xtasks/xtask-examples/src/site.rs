//! Ranim's experimental Notist SSG prepare command. Pages and template data are
//! generated from Cargo, source files, render provenance and published refs.
use crate::{
    Example, OutputFile, get_examples, render_state::RenderState, rewrite_readme_links,
    shadow::Shadow, split_output_files,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteInput {
    documents: BTreeMap<String, String>,
    data: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
struct Request {
    version: u32,
    stage: String,
    root: PathBuf,
    input: SiteInput,
}
#[derive(Serialize)]
struct Response {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<SiteInput>,
    errors: Vec<String>,
}
#[derive(Serialize)]
struct ExampleData<'a> {
    name: &'a str,
    code: &'a str,
    hash: &'a str,
    preview_imgs: Vec<String>,
    output_files: Vec<OutputFile>,
    wasm: bool,
}

/// Read one site request, return one response, and keep logs off stdout.
pub fn exchange(mut reader: impl Read, mut writer: impl Write) -> Result<()> {
    let result = (|| {
        let mut text = String::new();
        reader.read_to_string(&mut text)?;
        let request: Request = serde_json::from_str(&text)?;
        if request.version != 1 || request.stage != "prepare" {
            bail!("unsupported site extension request");
        }
        let website = request.root.canonicalize()?;
        let root = website.parent().context("website has no project root")?;
        if website.file_name().is_none_or(|name| name != "website") {
            bail!("Ranim site extension expects the website directory");
        }
        prepare(root, request.input)
    })();
    let response = match result {
        Ok(input) => Response {
            version: 1,
            input: Some(input),
            errors: vec![],
        },
        Err(error) => Response {
            version: 1,
            input: None,
            errors: vec![format!("{error:#}")],
        },
    };
    serde_json::to_writer(&mut writer, &response)?;
    writer.flush()?;
    Ok(())
}

fn index(title: &str) -> Result<String> {
    Ok(format!(
        "@!(title: {}, template: \"examples\")\n",
        serde_json::to_string(title)?
    ))
}

/// Prepare current inputs without generating per-example source or data files.
fn prepare(root: &Path, mut input: SiteInput) -> Result<SiteInput> {
    let examples = get_examples(root)?;
    let shadow = Shadow::load(root)?;
    let state = RenderState::load(root)?;
    input
        .documents
        .retain(|path, _| !path.starts_with("examples/"));
    input
        .documents
        .insert("examples/index.md".into(), index("例子")?);
    for example in &examples {
        let media =
            shadow.published_files(Path::new("website/static/examples").join(&example.name))?;
        let (previews, outputs) = split_output_files(&media);
        let base = Path::new("website/static/examples").join(&example.name);
        let preview_imgs = previews
            .iter()
            .map(|file| shadow.object_url(base.join(file)))
            .collect::<Result<_>>()?;
        let output_files = outputs
            .into_iter()
            .map(|file| {
                Ok(OutputFile {
                    url: shadow.object_url(base.join(&file.url))?,
                    kind: file.kind,
                })
            })
            .collect::<Result<_>>()?;
        let data = ExampleData {
            name: &example.name,
            code: &example.code,
            hash: state
                .hashes
                .get(&example.name)
                .map(String::as_str)
                .unwrap_or(""),
            preview_imgs,
            output_files,
            wasm: root.join(&base).join("pkg/ranim_example.js").is_file()
                && root.join(&base).join("pkg/ranim_example_bg.wasm").is_file(),
        };
        input
            .data
            .insert(example.name.clone(), serde_json::to_value(data)?);
        if example.meta.hide {
            continue;
        }
        // Only ignored README resources may need copying; page/data inputs stay
        // in memory. Identical resources are not rewritten during serve rebuilds.
        example.sync_readme_assets(root)?;
        let mut directory = PathBuf::from("examples");
        for component in example.group.components() {
            directory.push(component);
            let key = format!(
                "{}/index.md",
                directory.to_string_lossy().replace('\\', "/")
            );
            input
                .documents
                .insert(key, index(&component.as_os_str().to_string_lossy())?);
        }
        let path = directory
            .join(format!("{}.md", example.name))
            .to_string_lossy()
            .replace('\\', "/");
        input.documents.insert(path, page(example, &shadow)?);
    }
    Ok(input)
}

fn page(example: &Example, shadow: &Shadow) -> Result<String> {
    let name = serde_json::to_string(&example.name)?;
    let mut text = format!("@!(title: {name}, template: \"example\", example: {name})\n");
    let directory = example
        .path
        .parent()
        .context("example source has no parent")?;
    let readme = directory.join("README.md");
    if readme.is_file() {
        text.push('\n');
        text.push_str(&rewrite_readme_links(
            &fs::read_to_string(&readme)?,
            directory,
            &|source| {
                let file = source.file_name()?.to_str()?;
                shadow
                    .object_url(
                        Path::new("website/static/examples")
                            .join(&example.name)
                            .join(file),
                    )
                    .ok()
                    .or_else(|| {
                        source
                            .is_file()
                            .then(|| format!("/examples/{}/{file}", example.name))
                    })
            },
        ));
    }
    Ok(format!("{}\n", text.trim_end()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunStatus, clean_website_examples};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ranim-site-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let fixture = Self(root);
            fixture.write("website/content/index.md", "Home");
            fixture.write(
                "Cargo.toml",
                r#"[package]
name = "fixture"
version = "0.1.0"
[package.metadata.example.hidden]
hide = true
[[example]]
name = "foo"
path = "examples/geometry/foo/main.rs"
[[example]]
name = "hidden"
path = "examples/hidden/main.rs"
"#,
            );
            for directory in ["examples/geometry/foo", "examples/hidden"] {
                fixture.write(
                    &format!("{directory}/main.rs"),
                    "mod scenes; fn main() {}\n",
                );
                fixture.write(&format!("{directory}/scenes.rs"), "pub fn original() {}\n");
            }
            fixture.write(
                "examples/geometry/foo/README.md",
                "# Foo\n![Local](../../../assets/chart.png)\n![Published](preview.png)\n",
            );
            fixture.write("assets/chart.png", "chart bytes");
            fixture.write(
                "shadow.toml",
                "name = \"fixture\"\n[backend]\nendpoint = \"https://objects.example\"\nbucket = \"media\"\n",
            );
            for (file, character) in [("preview.png", 'a'), ("movie.mp4", 'b')] {
                fixture.write(
                    &format!(".shadow/refs/website/static/examples/foo/{file}.ref"),
                    &format!("oid = \"sha256:{}\"\n", character.to_string().repeat(64)),
                );
            }
            fixture
        }

        fn write(&self, path: &str, text: &str) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        fn request(&self) -> Value {
            serde_json::json!({
                "version": 1, "stage": "prepare", "root": self.0.join("website"),
                "config": {}, "input": {"documents": {"index.md": "Home"}, "data": {}}
            })
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn generates_grouped_pages_and_ref_media_without_persisting_sources() {
        let fixture = Fixture::new();
        let mut input = SiteInput::default();
        input.documents.insert("index.md".into(), "Home".into());
        input
            .documents
            .insert("examples/deleted.md".into(), "Old".into());
        input
            .data
            .insert("custom".into(), serde_json::json!({"value": 42}));
        let result = prepare(&fixture.0, input).unwrap();
        assert_eq!(result.documents.len(), 4);
        assert_eq!(result.documents["index.md"], "Home");
        assert!(result.documents.contains_key("examples/index.md"));
        assert!(result.documents.contains_key("examples/geometry/index.md"));
        let page = &result.documents["examples/geometry/foo.md"];
        assert!(page.contains("# Foo"));
        assert!(page.contains("/examples/foo/chart.png"));
        assert!(page.contains("https://media.objects.example/fixture/objects/sha256/aa/"));
        assert_eq!(result.data["custom"]["value"], 42);
        assert!(result.data.contains_key("hidden"));
        assert!(
            result.data["foo"]["code"]
                .as_str()
                .unwrap()
                .contains("pub fn original()")
        );
        assert_eq!(
            result.data["foo"]["preview_imgs"].as_array().unwrap().len(),
            1
        );
        assert_eq!(result.data["foo"]["output_files"][0]["kind"], "video/mp4");
        assert_eq!(result.data["foo"]["wasm"], false);
        assert!(!fixture.0.join("website/content/examples").exists());
        assert!(!fixture.0.join("website/data").exists());
        assert!(
            !fixture
                .0
                .join("website/static/examples/foo/movie.mp4")
                .exists()
        );
        let asset = fixture.0.join("website/static/examples/foo/chart.png");
        let modified = asset.metadata().unwrap().modified().unwrap();
        prepare(&fixture.0, SiteInput::default()).unwrap();
        assert_eq!(modified, asset.metadata().unwrap().modified().unwrap());
        fixture.write("assets/chart.png", "changed chart bytes");
        prepare(&fixture.0, SiteInput::default()).unwrap();
        assert_eq!(fs::read_to_string(asset).unwrap(), "changed chart bytes");
    }

    #[test]
    fn source_and_wasm_refresh_preserve_successful_render_provenance() {
        let fixture = Fixture::new();
        let examples = get_examples(&fixture.0).unwrap();
        RenderState::record(&fixture.0, &examples[0]).unwrap();
        let state = fixture.0.join("website/render-state.json");
        let before = fs::read(&state).unwrap();
        assert_eq!(
            examples[0].run(&fixture.0, true).unwrap(),
            RunStatus::UpToDate
        );
        fixture.write("examples/geometry/foo/scenes.rs", "pub fn updated() {}\n");
        fixture.write("website/static/examples/foo/pkg/ranim_example.js", "// JS");
        let input = prepare(&fixture.0, SiteInput::default()).unwrap();
        assert_eq!(input.data["foo"]["wasm"], false);
        fixture.write(
            "website/static/examples/foo/pkg/ranim_example_bg.wasm",
            "WASM",
        );
        let input = prepare(&fixture.0, SiteInput::default()).unwrap();
        assert_eq!(input.data["foo"]["hash"], examples[0].hash);
        assert!(
            input.data["foo"]["code"]
                .as_str()
                .unwrap()
                .contains("pub fn updated()")
        );
        assert_eq!(input.data["foo"]["wasm"], true);
        assert_eq!(fs::read(state).unwrap(), before);
        assert!(
            !RenderState::load(&fixture.0)
                .unwrap()
                .is_current(&get_examples(&fixture.0).unwrap()[0])
        );
        assert!(!fixture.0.join("website/data").exists());
    }

    #[test]
    fn deleted_examples_disappear_on_the_next_build_and_clean_removes_local_state() {
        let fixture = Fixture::new();
        let examples = get_examples(&fixture.0).unwrap();
        for example in &examples {
            RenderState::record(&fixture.0, example).unwrap();
            fixture.write(
                &format!("website/static/examples/{}/pkg/app.js", example.name),
                "JS",
            );
        }
        let mut input = prepare(&fixture.0, SiteInput::default()).unwrap();
        // Each SSG build starts with fresh data; sources may still include an
        // old generated document if supplied by an earlier extension.
        input.data.clear();
        fixture.write(
            "Cargo.toml",
            "[[example]]\nname = \"hidden\"\npath = \"examples/hidden/main.rs\"\n",
        );
        let input = prepare(&fixture.0, input).unwrap();
        assert!(!input.data.contains_key("foo"));
        assert!(!input.documents.contains_key("examples/geometry/foo.md"));
        let current = get_examples(&fixture.0).unwrap();
        let stats = clean_website_examples(&fixture.0, &current).unwrap();
        assert_eq!(stats.renders, 1);
        assert_eq!(stats.outputs, 1);
        assert!(!fixture.0.join("website/static/examples/foo").exists());
        assert!(
            fixture
                .0
                .join("website/static/examples/hidden/pkg/app.js")
                .exists()
        );
        assert!(
            !RenderState::load(&fixture.0)
                .unwrap()
                .hashes
                .contains_key("foo")
        );
    }

    #[test]
    fn stdio_returns_complete_input_and_reports_protocol_and_source_errors() {
        let fixture = Fixture::new();
        let request = fixture.request();
        let mut output = vec![];
        exchange(
            serde_json::to_vec(&request).unwrap().as_slice(),
            &mut output,
        )
        .unwrap();
        let response: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(response["errors"], serde_json::json!([]));
        assert_eq!(response["input"]["documents"]["index.md"], "Home");
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("stage", serde_json::json!("render")),
        ] {
            let mut invalid = request.clone();
            invalid[field] = value;
            output.clear();
            exchange(
                serde_json::to_vec(&invalid).unwrap().as_slice(),
                &mut output,
            )
            .unwrap();
            let response: Value = serde_json::from_slice(&output).unwrap();
            assert!(response.get("input").is_none());
            assert!(
                response["errors"][0]
                    .as_str()
                    .unwrap()
                    .contains("unsupported")
            );
        }
        fixture.write(
            ".shadow/refs/website/static/examples/foo/preview.png.ref",
            "oid = \"invalid\"",
        );
        output.clear();
        exchange(
            serde_json::to_vec(&request).unwrap().as_slice(),
            &mut output,
        )
        .unwrap();
        let response: Value = serde_json::from_slice(&output).unwrap();
        assert!(
            response["errors"][0]
                .as_str()
                .unwrap()
                .contains("unsupported oid")
        );
    }

    #[test]
    fn invalid_render_state_is_reported_instead_of_resetting_the_cache() {
        let fixture = Fixture::new();
        for state in ["not json", r#"{"version":2,"hashes":{}}"#] {
            fixture.write("website/render-state.json", state);
            assert!(prepare(&fixture.0, SiteInput::default()).is_err());
        }
    }
}
