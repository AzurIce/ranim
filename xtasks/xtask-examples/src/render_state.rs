//! Successful render provenance, independent of generated site template data.
use crate::Example;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::ErrorKind, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenderState {
    version: u32,
    pub hashes: BTreeMap<String, String>,
}
impl Default for RenderState {
    fn default() -> Self {
        Self {
            version: 1,
            hashes: BTreeMap::new(),
        }
    }
}
impl RenderState {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("website/render-state.json");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()));
            }
        };
        let state: Self = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if state.version != 1 {
            bail!("unsupported render state version {}", state.version);
        }
        Ok(state)
    }
    pub fn is_current(&self, example: &Example) -> bool {
        self.hashes.get(&example.name) == Some(&example.hash)
    }
    pub fn record(root: &Path, example: &Example) -> Result<()> {
        let mut state = Self::load(root)?;
        state
            .hashes
            .insert(example.name.clone(), example.hash.clone());
        state.save(root)
    }
    pub fn save(&self, root: &Path) -> Result<()> {
        let path = root.join("website/render-state.json");
        fs::create_dir_all(path.parent().unwrap())?;
        let temporary = path.with_extension("json.tmp");
        fs::write(
            &temporary,
            format!("{}\n", serde_json::to_string_pretty(self)?),
        )?;
        fs::rename(&temporary, &path).with_context(|| format!("failed to write {}", path.display()))
    }
}
