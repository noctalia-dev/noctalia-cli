use std::{collections::HashMap, env, fs, io, path::PathBuf};

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
        // For shell component, also check if it actually exists on the filesystem
        if component == "shell" {
            let filesystem_installed = check_shell_installed();
            let config_installed = self.components.get("shell").map(|c| c.installed).unwrap_or(false);
            
            // If filesystem says installed but config says not, update the config
            if filesystem_installed && !config_installed {
                if let Ok((mut updated_cfg, path)) = CliConfig::load() {
                    updated_cfg.set_installed("shell", true);
                    let _ = updated_cfg.save(&path);
                }
            }
            
            return filesystem_installed;
        }
        
        self.components.get(component).map(|c| c.installed).unwrap_or(false)
    }

}

fn check_shell_installed() -> bool {
    // Check both possible installation locations
    let old_path = PathBuf::from("/etc/xdg/quickshell/noctalia-shell");
    let home = env::var("HOME").unwrap_or_else(|_| String::new());
    let new_path = if !home.is_empty() {
        PathBuf::from(home).join(".config/quickshell/noctalia-shell")
    } else {
        PathBuf::new()
    };
    
    // Check if either location exists
    old_path.exists() || (!new_path.as_os_str().is_empty() && new_path.exists())
}

pub fn config_path() -> PathBuf {
    let dirs = ProjectDirs::from("dev", "noctalia", "noctalia").expect("failed to resolve config dir");
    dirs.config_dir().join("cli.toml")
}

