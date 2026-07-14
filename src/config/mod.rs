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


// config loading: reads config.toml from the platform config dir, creates defaults if missing.
// everything lands in the SETTINGS static so the rest of the app can just grab it.

use config::{Config as ConfigLoader, ConfigError, File};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

pub mod settings;
pub mod theme;
pub(crate) mod migrate;

pub use settings::Config;

#[must_use]
pub fn get_config_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RTML")
}

// seeds the config file from the bundled default on first run

fn ensure_config_exists() -> PathBuf {
    let config_path = get_config_path().join("config.toml");
    if !config_path.exists() {
        if let Some(parent) = config_path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            tracing::warn!(
                "Failed to create config directory {}: {}",
                parent.display(),
                e
            );
        }
        match fs::write(&config_path, include_str!("../../assets/config.toml")) {
            Ok(()) => tracing::debug!("Wrote default config to {}", config_path.display()),
            Err(e) => tracing::warn!(
                "Failed to write default config to {}: {}",
                config_path.display(),
                e
            ),
        }
    } else {
        tracing::trace!("Using existing config at {}", config_path.display());
    }
    config_path
}

pub fn load_config(config_path: &std::path::Path) -> Result<Config, ConfigError> {
    tracing::debug!("Loading config from {}", config_path.display());
    ConfigLoader::builder()
        .add_source(File::from(config_path).required(false))
        .build()?
        .try_deserialize()
}

pub static SETTINGS: LazyLock<Config> = LazyLock::new(|| {
    let path = ensure_config_exists();
    load_config(&path).unwrap_or_else(|e| {
        tracing::error!("Config load failed, using defaults: {}", e);
        Config {
            general: settings::General::default(),
            paths: settings::Paths::default(),
            defaults: settings::Defaults::default(),
            ui: settings::Ui::default(),
        }
    })
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config_from_valid_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
            [defaults]
            memory_max = "4G"
            "#,
        )
        .unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.defaults.memory_max, "4G");
        assert_eq!(config.defaults.memory_min, "512M");
    }

    #[test]
    fn load_config_from_empty_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "").unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.defaults.memory_max, "2G");
    }

    #[test]
    fn load_config_missing_file_uses_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.toml");
        load_config(&path).unwrap();
    }

    #[test]
    fn load_config_partial_sections() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
            [paths]
            instances_dir = "/custom/path"
            "#,
        )
        .unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.paths.instances_dir, "/custom/path");
        assert!(config.paths.java_path.is_none());
    }
}
