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


// vanilla "installer". doesn't actually install anything since the launch
// process already handles downloading vanilla assets/libraries. this just
// exists so vanilla fits the same ModLoaderInstaller trait as everyone else.

use std::path::Path;

use async_trait::async_trait;

use super::{GameVersion, InstallError, ModLoaderInstaller};
use crate::instance::models::ModLoader;
use crate::net::{HttpClient, NetError, mojang};

pub struct VanillaInstaller;

#[async_trait]
impl ModLoaderInstaller for VanillaInstaller {
    fn loader_type(&self) -> ModLoader {
        ModLoader::Vanilla
    }

    async fn get_game_versions(&self, client: &HttpClient) -> Result<Vec<GameVersion>, NetError> {
        let manifest = mojang::fetch_version_manifest(client).await?;
        Ok(manifest
            .versions
            .into_iter()
            .map(|v| GameVersion {
                id: v.id,
                stable: v.version_type == "release",
            })
            .collect())
    }

    async fn get_versions(
        &self,
        _client: &HttpClient,
        _game_version: &str,
    ) -> Result<Vec<String>, NetError> {
        Ok(vec!["vanilla".to_owned()])
    }

    async fn install(
        &self,
        _client: &HttpClient,
        _game_version: &str,
        _loader_version: &str,
        _instance_dir: &Path,
        _meta_dir: &Path,
    ) -> Result<(), InstallError> {
        Ok(())
    }
}
