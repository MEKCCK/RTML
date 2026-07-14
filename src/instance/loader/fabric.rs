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


// fabric installation. the nice one: just fetch a profile json and download
// the libraries. no java installer nonsense required.

use std::path::Path;

use async_trait::async_trait;

use super::{GameVersion, InstallError, InstallerError, ModLoaderInstaller};
use crate::instance::models::ModLoader;
use crate::net::{HttpClient, NetError, fabric as fabric_api};

pub struct FabricInstaller;

#[async_trait]
impl ModLoaderInstaller for FabricInstaller {
    fn loader_type(&self) -> ModLoader {
        ModLoader::Fabric
    }

    async fn get_game_versions(&self, client: &HttpClient) -> Result<Vec<GameVersion>, NetError> {
        fabric_api::fetch_fabric_game_versions(client).await
    }

    async fn get_versions(
        &self,
        client: &HttpClient,
        game_version: &str,
    ) -> Result<Vec<String>, NetError> {
        tracing::debug!(
            "Fetching Fabric loader versions for Minecraft {}",
            game_version
        );
        let loader_versions = fabric_api::fetch_fabric_versions(client, game_version).await?;
        tracing::debug!(
            "Fetched {} Fabric loader version(s) for Minecraft {}",
            loader_versions.len(),
            game_version
        );
        Ok(loader_versions
            .into_iter()
            .map(|lv| lv.loader.version)
            .collect())
    }

    async fn install(
        &self,
        client: &HttpClient,
        game_version: &str,
        loader_version: &str,
        _instance_dir: &Path,
        meta_dir: &Path,
    ) -> Result<(), InstallError> {
        tracing::info!(
            "Installing Fabric {} for Minecraft {}",
            loader_version,
            game_version
        );
        let (profile, raw_bytes) =
            fabric_api::fetch_fabric_profile_with_raw(client, game_version, loader_version).await?;
        tracing::debug!(
            "Fetched Fabric profile {} with {} libraries",
            profile.id,
            profile.libraries.len()
        );
        fabric_api::download_fabric_libraries(client, &profile, meta_dir).await?;
        super::save_profile_bytes(
            meta_dir,
            &format!("fabric-{game_version}-{loader_version}.json"),
            &raw_bytes,
        )
        .await
        .map_err(InstallerError::from)?;
        tracing::debug!(
            "Saved Fabric profile for Minecraft {} loader {}",
            game_version,
            loader_version
        );
        Ok(())
    }
}
