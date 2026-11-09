//! Ranim's build-scoped Notist SSG command. Pages and template data are
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
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
struct SiteInput {
    documents: BTreeMap<String, String>,
    data: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
struct Request {
    version: u32,
    root: PathBuf,
    config: Value,
    #[serde(flatten)]
    event: Event,
}
#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    BeforeCompile {
        documents: BTreeMap<String, String>,
    },
    BeforeTemplate {
        pages: Vec<Value>,
        data: BTreeMap<String, Value>,
    },
}
#[derive(Serialize)]
struct Response {
    version: u32,
    event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    documents: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}
#[derive(Serialize)]
struct ExampleData<'a> {
    name: &'a str,
    code: &'a str,
    sources: Vec<SourceFile>,
    hash: &'a str,
    preview_imgs: Vec<String>,
    output_files: Vec<OutputFile>,
    wasm: bool,
}

#[derive(Serialize)]
struct SourceFile {
    name: &'static str,
    label: &'static str,
    code: String,
}

/// Answer each JSONL event immediately; state belongs to this build only.
pub fn exchange(mut reader: impl BufRead, mut writer: impl Write) -> Result<()> {
    let mut generated = None;
    let mut finished = false;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let event = serde_json::from_str::<Value>(&line)
            .ok()
            .and_then(|value| value["event"].as_str().map(str::to_owned))
            .unwrap_or_else(|| "before_compile".into());
        let mut response = Response {
            version: 2,
            event,
            documents: None,
            data: None,
            errors: vec![],
        };
        let result: Result<()> = (|| {
            if !line.ends_with('\n') {
                bail!("site extension request must end with a newline");
            }
            let request: Request = serde_json::from_str(&line)?;
            if request.version != 2 {
                bail!("unsupported site extension request version");
            }
            // The host owns config and final Pages; this extension changes
            // only sources and its generated template data.
            let _config = request.config;
            let website = request.root.canonicalize()?;
            let root = website.parent().context("website has no project root")?;
            if website.file_name().is_none_or(|name| name != "website") {
                bail!("Ranim site extension expects the website directory");
            }
            match request.event {
                Event::BeforeCompile { documents } => {
                    if generated.is_some() || finished {
                        bail!("before_compile must occur once before before_template");
                    }
                    let input = prepare(
                        root,
                        SiteInput {
                            documents,
                            data: BTreeMap::new(),
                        },
                    )?;
                    generated = Some((website, input.data));
                    response.documents = Some(input.documents);
                }
                Event::BeforeTemplate { pages, mut data } => {
                    let (prepared_root, generated_data) = generated
                        .take()
                        .context("before_template requires before_compile in the same process")?;
                    if prepared_root != website {
                        bail!("site root changed between events");
                    }
                    let _pages = pages;
                    data.extend(generated_data);
                    response.data = Some(data);
                    finished = true;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            response.errors.push(format!("{error:#}"));
        }
        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        if !response.errors.is_empty() {
            break;
        }
    }
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
            sources: [("scenes.rs", "共享场景与项目装配"), ("main.rs", "平台入口")]
                .into_iter()
                .map(|(name, label)| {
                    Ok(SourceFile {
                        name,
                        label,
                        code: fs::read_to_string(example.path.with_file_name(name))?,
                    })
                })
                .collect::<Result<_>>()?,
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
                "version": 2, "event": "before_compile", "root": self.0.join("website"),
                "config": {}, "documents": {"index.md": "Home"}
            })
        }

        fn template_request(&self) -> Value {
            serde_json::json!({
                "version": 2, "event": "before_template", "root": self.0.join("website"),
                "config": {}, "pages": [{"title": "Final title", "html": "<p>Final</p>"}],
                "data": {"custom": {"value": 42}, "earlier-extension": {"ready": true}}
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
        let sources = result.data["foo"]["sources"].as_array().unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0]["name"], "scenes.rs");
        assert_eq!(sources[0]["label"], "共享场景与项目装配");
        assert_eq!(sources[0]["code"], "pub fn original() {}\n");
        assert_eq!(sources[1]["name"], "main.rs");
        assert_eq!(sources[1]["code"], "mod scenes; fn main() {}\n");
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

    fn responses(requests: &[Value]) -> Vec<Value> {
        let mut input = vec![];
        for request in requests {
            serde_json::to_writer(&mut input, request).unwrap();
            input.push(b'\n');
        }
        let mut output = vec![];
        exchange(input.as_slice(), &mut output).unwrap();
        assert!(output.ends_with(b"\n"));
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn jsonl_events_return_only_stage_outputs_and_preserve_prior_template_data() {
        let fixture = Fixture::new();
        let output = responses(&[fixture.request(), fixture.template_request()]);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0]["version"], 2);
        assert_eq!(output[0]["event"], "before_compile");
        assert_eq!(output[0]["documents"]["index.md"], "Home");
        assert!(
            output[0]["documents"]
                .get("examples/geometry/foo.md")
                .is_some()
        );
        assert!(output[0].get("data").is_none());
        assert!(output[0].get("input").is_none());
        assert_eq!(output[1]["version"], 2);
        assert_eq!(output[1]["event"], "before_template");
        assert_eq!(output[1]["data"]["custom"]["value"], 42);
        assert_eq!(output[1]["data"]["earlier-extension"]["ready"], true);
        assert_eq!(output[1]["data"]["foo"]["sources"][0]["name"], "scenes.rs");
        assert!(output[1].get("documents").is_none());
        assert!(output[1].get("pages").is_none());
        assert!(
            output
                .iter()
                .all(|response| response.get("errors").is_none())
        );
    }

    #[test]
    fn jsonl_rejects_wrong_versions_unknown_events_and_cross_stage_fields() {
        let fixture = Fixture::new();
        for (field, value) in [
            ("version", serde_json::json!(1)),
            ("event", serde_json::json!("prepare")),
            ("stage", serde_json::json!("render")),
            ("data", serde_json::json!({})),
            ("pages", serde_json::json!([])),
        ] {
            let mut invalid = fixture.request();
            invalid[field] = value;
            let output = responses(&[invalid, fixture.template_request()]);
            assert_eq!(output.len(), 1, "{field}");
            assert!(output[0].get("documents").is_none(), "{field}");
            assert!(output[0].get("data").is_none(), "{field}");
            assert!(
                !output[0]["errors"][0].as_str().unwrap().is_empty(),
                "{field}"
            );
        }
        let mut invalid = fixture.template_request();
        invalid["documents"] = serde_json::json!({});
        let output = responses(&[fixture.request(), invalid]);
        assert!(
            output[1]["errors"][0]
                .as_str()
                .unwrap()
                .contains("unknown field")
        );
    }

    #[test]
    fn jsonl_rejects_invalid_event_order_and_changed_root() {
        let fixture = Fixture::new();
        for requests in [
            vec![fixture.template_request()],
            vec![fixture.request(), fixture.request()],
            vec![
                fixture.request(),
                fixture.template_request(),
                fixture.template_request(),
            ],
            vec![
                fixture.request(),
                fixture.template_request(),
                fixture.request(),
            ],
        ] {
            let output = responses(&requests);
            assert!(output.last().unwrap()["errors"][0].is_string());
        }
        let other = Fixture::new();
        let output = responses(&[fixture.request(), other.template_request()]);
        assert!(
            output[1]["errors"][0]
                .as_str()
                .unwrap()
                .contains("root changed")
        );
    }

    #[test]
    fn jsonl_reports_source_errors_and_rejects_unframed_or_malformed_requests() {
        let fixture = Fixture::new();
        fixture.write(
            ".shadow/refs/website/static/examples/foo/preview.png.ref",
            "oid = \"invalid\"",
        );
        let output = responses(&[fixture.request(), fixture.template_request()]);
        assert_eq!(output.len(), 1);
        assert!(
            output[0]["errors"][0]
                .as_str()
                .unwrap()
                .contains("unsupported oid")
        );
        for input in [
            serde_json::to_vec(&fixture.request()).unwrap(),
            b"not json\n".to_vec(),
        ] {
            let mut output = vec![];
            exchange(input.as_slice(), &mut output).unwrap();
            let response: Value = serde_json::from_slice(&output).unwrap();
            assert!(response["errors"][0].is_string());
            assert!(response.get("documents").is_none());
            assert!(response.get("data").is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn jsonl_flushes_before_eof_and_retains_one_build_snapshot() {
        use std::{io::BufReader, net::Shutdown, os::unix::net::UnixStream, time::Duration};

        let fixture = Fixture::new();
        let (mut host, worker) = UnixStream::pair().unwrap();
        host.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(host.try_clone().unwrap());
        let thread = std::thread::spawn(move || {
            exchange(BufReader::new(worker.try_clone().unwrap()), worker).unwrap();
        });
        serde_json::to_writer(&mut host, &fixture.request()).unwrap();
        host.write_all(b"\n").unwrap();
        host.flush().unwrap();
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let first: Value = serde_json::from_str(&line).unwrap();
        assert!(first.get("errors").is_none(), "{first}");
        assert!(first["documents"].get("examples/geometry/foo.md").is_some());

        // The second event uses data captured while compiling, not a fresh
        // discovery that could disagree with the compiled sources.
        fixture.write(
            "examples/geometry/foo/scenes.rs",
            "pub fn changed_between_events() {}\n",
        );
        serde_json::to_writer(&mut host, &fixture.template_request()).unwrap();
        host.write_all(b"\n").unwrap();
        host.flush().unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap();
        let second: Value = serde_json::from_str(&line).unwrap();
        assert!(second.get("errors").is_none(), "{second}");
        assert_eq!(
            second["data"]["foo"]["sources"][0]["code"],
            "pub fn original() {}\n"
        );
        host.shutdown(Shutdown::Write).unwrap();
        line.clear();
        assert_eq!(reader.read_line(&mut line).unwrap(), 0);
        thread.join().unwrap();
        let next_build = responses(&[fixture.request(), fixture.template_request()]);
        assert_eq!(
            next_build[1]["data"]["foo"]["sources"][0]["code"],
            "pub fn changed_between_events() {}\n"
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
