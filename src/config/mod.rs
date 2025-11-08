use std::{collections::HashMap, fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Release,
    Git,
}

impl Default for SourceKind {
    fn default() -> Self { SourceKind::Release }
}

impl std::fmt::Display for SourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceKind::Release => write!(f, "release"),
            SourceKind::Git => write!(f, "git"),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ComponentConfig {
    pub source: SourceKind,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CliConfig {
    pub components: HashMap<String, ComponentConfig>,
}

impl CliConfig {
    pub fn load() -> io::Result<(Self, PathBuf)> {
        let path = config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let cfg: CliConfig = toml::from_str(&content).unwrap_or_default();
            Ok((cfg, path))
        } else {
            Ok((CliConfig::default(), path))
        }
    }

    pub fn save(&self, to: &PathBuf) -> io::Result<()> {
        if let Some(parent) = to.parent() { fs::create_dir_all(parent)?; }
        let serialized = toml::to_string_pretty(self).unwrap_or_default();
        fs::write(to, serialized)
    }

    pub fn get_component_source(&self, component: &str) -> Option<SourceKind> {
        self.components.get(component).map(|c| c.source)
    }

    pub fn set_component_source(&mut self, component: &str, source: SourceKind) {
        let entry = self.components.entry(component.to_string()).or_default();
        entry.source = source;
    }

    pub fn set_installed(&mut self, component: &str, installed: bool) {
        let entry = self.components.entry(component.to_string()).or_default();
        entry.installed = installed;
    }

    pub fn get_component_version(&self, component: &str) -> Option<String> {
        self.components.get(component).and_then(|c| c.version.clone())
    }

    pub fn set_component_version(&mut self, component: &str, version: String) {
        let entry = self.components.entry(component.to_string()).or_default();
        entry.version = Some(version);
    }

    pub fn is_component_installed(&self, component: &str) -> bool {
        self.components.get(component).map(|c| c.installed).unwrap_or(false)
    }

}

pub fn config_path() -> PathBuf {
    let dirs = ProjectDirs::from("dev", "noctalia", "noctalia").expect("failed to resolve config dir");
    dirs.config_dir().join("cli.toml")
}

