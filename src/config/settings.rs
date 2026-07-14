// ========================================================================
//                     项目许可说明 / License Notice
// ========================================================================
//
// 本项目 RustedTuiMcLauncher (RTML) 基于 rmcl 项目开发。
// Original code derived from rmcl (https://github.com/objz/rmcl).
//
// This is a modified version of rmcl. Modifications made in 2026 by RTML Contributors.
//
// Copyright (C) 2024-2026 objz (rmcl original author)
// Copyright (C) 2026 RTML Contributors
//
// 本项目包含 rmcl 的原始代码以及 RTML 的新增功能。
// This project contains original code from rmcl and additional features by RTML.
//
// 所有代码均采用 GPL-3.0 许可证授权。
// All code is licensed under the GNU General Public License v3.0.
//
// 部分代码还参考/移植自 BonNext (https://github.com/anomalyco/BonNextMinecraftLauncher-Rust)。
// Additional code referenced/ported from BonNext (https://github.com/anomalyco/BonNextMinecraftLauncher-Rust).
//
// Copyright (C) 2024-2026 anomalyco (BonNext author)
//
// The Terracotta online multiplayer (陶瓦联机) feature is modeled after
// HMCL (Hello Minecraft! Launcher, https://github.com/HMCL-dev/HMCL),
// Copyright (C) 2025 huangyuhui and contributors.
//
// ========================================================================


// all the config structs that map to sections in config.toml.
// everything has sane defaults so a blank file (or no file) still works.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageProtocol {
    Halfblocks,
    Quadrants,
    #[default]
    Kitty,
    Iterm2,
}

#[derive(Debug, Deserialize)]
pub struct General {
    #[serde(default)]
    pub download_source: DownloadSourceConfig,
    #[serde(default)]
    pub curseforge_api_key: Option<String>,
}

impl Default for General {
    fn default() -> Self {
        Self {
            download_source: DownloadSourceConfig::default(),
            curseforge_api_key: None,
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadSourceConfig {
    #[default]
    Official,
    Mirror,
}

#[derive(Debug, Deserialize)]
pub struct Paths {
    #[serde(default = "default_instances_dir")]
    pub instances_dir: String,
    #[serde(default = "default_meta_dir")]
    pub meta_dir: String,
    #[serde(default)]
    pub java_path: Option<String>,
}

fn default_instances_dir() -> String {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RTML")
        .join("instances")
        .to_string_lossy()
        .into_owned()
}

fn default_meta_dir() -> String {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RTML")
        .join("meta")
        .to_string_lossy()
        .into_owned()
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            instances_dir: default_instances_dir(),
            meta_dir: default_meta_dir(),
            java_path: None,
        }
    }
}

// expand ~ in paths since toml doesn't do that for us
pub fn resolve_path(raw: &str) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = dirs_next::home_dir() {
            return home.join(stripped);
        }
    } else if let Some(stripped) = raw.strip_prefix("~\\") {
        if let Some(home) = dirs_next::home_dir() {
            return home.join(stripped);
        }
    } else if raw == "~"
        && let Some(home) = dirs_next::home_dir()
    {
        return home;
    }
    PathBuf::from(raw)
}

impl Paths {
    pub fn effective_java_path(&self) -> Option<&str> {
        self.java_path.as_deref().filter(|s| !s.is_empty())
    }

    pub fn resolve_instances_dir(&self) -> PathBuf {
        if self.instances_dir.is_empty() {
            PathBuf::from(default_instances_dir())
        } else {
            resolve_path(&self.instances_dir)
        }
    }

    pub fn resolve_meta_dir(&self) -> PathBuf {
        if self.meta_dir.is_empty() {
            PathBuf::from(default_meta_dir())
        } else {
            resolve_path(&self.meta_dir)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_memory_min")]
    pub memory_min: String,
    #[serde(default = "default_memory_max")]
    pub memory_max: String,
}

fn default_memory_min() -> String {
    "512M".to_owned()
}
fn default_memory_max() -> String {
    "2G".to_owned()
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            memory_min: default_memory_min(),
            memory_max: default_memory_max(),
        }
    }
}

#[derive(Debug, Deserialize)]
// timing knobs for the error toast animation: show for 5s, start sliding at 3.5s,
// fly off screen over 300ms. tweak these if the toasts feel too fast or slow.

pub struct Ui {
    #[serde(default)]
    pub image_protocol: ImageProtocol,
    #[serde(default = "default_error_auto_dismiss_ms")]
    pub error_auto_dismiss_ms: u64,
    #[serde(default = "default_error_slide_start_ms")]
    pub error_slide_start_ms: u64,
    #[serde(default = "default_error_fly_out_ms")]
    pub error_fly_out_ms: u64,
    #[serde(default = "default_max_error_events")]
    pub max_error_events: usize,
}

fn default_error_auto_dismiss_ms() -> u64 {
    5000
}
fn default_error_slide_start_ms() -> u64 {
    3500
}
fn default_error_fly_out_ms() -> u64 {
    300
}
fn default_max_error_events() -> usize {
    50
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            image_protocol: ImageProtocol::default(),
            error_auto_dismiss_ms: default_error_auto_dismiss_ms(),
            error_slide_start_ms: default_error_slide_start_ms(),
            error_fly_out_ms: default_error_fly_out_ms(),
            max_error_events: default_max_error_events(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub ui: Ui,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_java_path_none_when_absent() {
        let paths = Paths {
            java_path: None,
            ..Paths::default()
        };
        assert!(paths.effective_java_path().is_none());
    }

    #[test]
    fn effective_java_path_none_when_empty() {
        let paths = Paths {
            java_path: Some(String::new()),
            ..Paths::default()
        };
        assert!(paths.effective_java_path().is_none());
    }

    #[test]
    fn effective_java_path_some_when_set() {
        let paths = Paths {
            java_path: Some("/usr/bin/java".to_owned()),
            ..Paths::default()
        };
        assert_eq!(paths.effective_java_path(), Some("/usr/bin/java"));
    }

    #[test]
    fn resolve_path_absolute() {
        assert_eq!(resolve_path("/opt/RTML"), PathBuf::from("/opt/RTML"));
    }

    #[test]
    fn resolve_path_tilde_prefix() {
        let resolved = resolve_path("~/games/RTML");
        assert!(!resolved.to_string_lossy().starts_with('~'));
        assert!(resolved.ends_with("games/RTML"));
    }

    #[test]
    fn resolve_path_bare_tilde() {
        let resolved = resolve_path("~");
        assert!(!resolved.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn config_deserializes_from_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.defaults.memory_max, "2G");
    }

    #[test]
    fn config_deserializes_partial_toml() {
        let toml_str = r#"
[general]
debug = true

[defaults]
memory_max = "8G"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.defaults.memory_max, "8G");
        assert_eq!(config.defaults.memory_min, "512M");
    }
}
