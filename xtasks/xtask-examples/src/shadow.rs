//! Integration with the [`shadow`](https://github.com/AzurIce/shadow)
//! content-addressed storage tool.
//!
//! Rendered example outputs are uploaded to object storage with
//! `shadow publish` and referenced from the website by content-addressed
//! URLs, so rendered media never has to live in the Git repository.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
struct ShadowToml {
    name: String,
    backend: BackendToml,
}

#[derive(Deserialize)]
struct BackendToml {
    endpoint: String,
    bucket: String,
    #[serde(default)]
    prefix: String,
}

/// Loaded `shadow.toml` plus the derived public object URL base.
///
/// The public base follows the backend's virtual-hosted form
/// (`https://<bucket>.<endpoint-host>/[<prefix>/]`). When a CDN or custom
/// domain is added later, only this derivation changes.
pub struct Shadow {
    root: PathBuf,
    name: String,
    objects_base: String,
}

impl Shadow {
    pub fn load(root_dir: impl AsRef<Path>) -> Result<Self> {
        let root_dir = root_dir.as_ref();
        let path = root_dir.join("shadow.toml");
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: ShadowToml =
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;

        let endpoint = config
            .backend
            .endpoint
            .strip_prefix("https://")
            .or_else(|| config.backend.endpoint.strip_prefix("http://"))
            .with_context(|| format!("unsupported endpoint {}", config.backend.endpoint))?
            .trim_end_matches('/');
        let mut objects_base = format!("https://{}.{}/", config.backend.bucket, endpoint);
        let prefix = config.backend.prefix.trim_matches('/');
        if !prefix.is_empty() {
            objects_base.push_str(prefix);
            objects_base.push('/');
        }

        Ok(Self {
            root: root_dir.to_path_buf(),
            name: config.name,
            objects_base: objects_base.to_string(),
        })
    }

    /// Upload every managed worktree file and write its ref
    /// (`shadow publish`). Publishing is idempotent: unchanged content is
    /// skipped by a remote `stat`, so this is cheap to call after every run.
    pub fn publish(&self) -> Result<()> {
        let status = Command::new("shadow")
            .arg("publish")
            .current_dir(&self.root)
            .status()
            .context(
                "failed to run `shadow publish`; install it with `cargo install --git https://github.com/AzurIce/shadow`",
            )?;
        if !status.success() {
            bail!("`shadow publish` failed");
        }
        Ok(())
    }

    /// Content-addressed URL of the object behind a managed worktree path.
    ///
    /// The path may be relative to the repository root or absolute; it must
    /// have been published before, i.e. a ref must exist under
    /// `.shadow/refs/`.
    pub fn object_url(&self, worktree_path: impl AsRef<Path>) -> Result<String> {
        let worktree_path = worktree_path.as_ref();
        // `Path::join` replaces the whole base for absolute paths, so reduce
        // to the repo-relative form before appending the refs prefix.
        let worktree_path = worktree_path
            .strip_prefix(&self.root)
            .unwrap_or(worktree_path);
        let mut ref_file = worktree_path.as_os_str().to_os_string();
        ref_file.push(".ref");
        let ref_path = self.root.join(".shadow").join("refs").join(&ref_file);
        let text = fs::read_to_string(&ref_path)
            .with_context(|| format!("failed to read {}", ref_path.display()))?;

        #[derive(Deserialize)]
        struct RefDocument {
            oid: String,
        }
        let reference: RefDocument = toml::from_str(&text)
            .with_context(|| format!("failed to parse {}", ref_path.display()))?;
        let hex = reference.oid.strip_prefix("sha256:").with_context(|| {
            format!(
                "unsupported oid {} in {}",
                reference.oid,
                ref_path.display()
            )
        })?;
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("invalid oid {} in {}", reference.oid, ref_path.display());
        }
        Ok(self.url_for_oid_hex(hex))
    }

    /// Like [`Self::object_url`], but returns `None` instead of failing when
    /// shadow is not configured for `root_dir` or the path is unpublished.
    pub fn try_object_url(
        root_dir: impl AsRef<Path>,
        worktree_path: impl AsRef<Path>,
    ) -> Option<String> {
        Self::load(root_dir)
            .and_then(|shadow| shadow.object_url(worktree_path))
            .ok()
    }

    fn url_for_oid_hex(&self, hex: &str) -> String {
        format!(
            "{objects_base}{name}/objects/sha256/{head}/{tail}",
            objects_base = self.objects_base,
            name = self.name,
            head = &hex[..2],
            tail = &hex[2..],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ranim-xtask-shadow-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_config(root: &Path, prefix: &str) {
        fs::write(
            root.join("shadow.toml"),
            format!(
                "version = 1\nname = \"ranim\"\n\n[backend]\ntype = \"volcengine_tos\"\nendpoint = \"https://tos-cn-beijing.volces.com\"\nregion = \"cn-beijing\"\nbucket = \"azurice-shadow\"\nprefix = \"{prefix}\"\n"
            ),
        )
        .unwrap();
    }

    fn write_ref(root: &Path, worktree_path: &str, hex: &str) {
        let ref_path = root
            .join(".shadow/refs")
            .join(format!("{worktree_path}.ref"));
        fs::create_dir_all(ref_path.parent().unwrap()).unwrap();
        fs::write(
            ref_path,
            format!("version = 1\noid = \"sha256:{hex}\"\nsize = 1\n"),
        )
        .unwrap();
    }

    const OID: &str = "dc024c2008d5ce22ae8c79fb31b6cb6c453b1cef8b750a3e92ae95199714ff66";

    #[test]
    fn derives_virtual_hosted_object_urls() {
        for (prefix, expected_base) in [
            (
                "",
                "https://azurice-shadow.tos-cn-beijing.volces.com/".to_string(),
            ),
            (
                "shadow",
                "https://azurice-shadow.tos-cn-beijing.volces.com/shadow/".to_string(),
            ),
            (
                "shadow/",
                "https://azurice-shadow.tos-cn-beijing.volces.com/shadow/".to_string(),
            ),
        ] {
            let root = temp_root("url");
            write_config(&root, prefix);
            write_ref(&root, "website/static/examples/foo/a.mp4", OID);

            let shadow = Shadow::load(&root).unwrap();
            let url = shadow
                .object_url("website/static/examples/foo/a.mp4")
                .unwrap();

            assert_eq!(
                url,
                format!("{expected_base}ranim/objects/sha256/dc/{}", &OID[2..])
            );
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn try_object_url_falls_back_to_none_without_config() {
        let root = temp_root("missing");
        assert!(Shadow::try_object_url(&root, "any/path.png").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn try_object_url_is_none_for_unpublished_paths() {
        let root = temp_root("unpublished");
        write_config(&root, "");
        assert!(Shadow::try_object_url(&root, "any/path.png").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    /// Regression: `Path::join` replaces the whole base for absolute paths,
    /// so an absolute worktree path used to lose the `.shadow/refs` prefix.
    #[test]
    fn resolves_absolute_worktree_paths() {
        let root = temp_root("absolute");
        write_config(&root, "");
        write_ref(&root, "website/static/examples/foo/a.png", OID);

        let shadow = Shadow::load(&root).unwrap();
        let worktree = root.join("website/static/examples/foo/a.png");
        assert_eq!(
            shadow.object_url(&worktree).unwrap(),
            shadow
                .object_url("website/static/examples/foo/a.png")
                .unwrap()
        );
        let _ = fs::remove_dir_all(&root);
    }
}
