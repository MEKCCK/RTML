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


// quilt installation. same clean approach as fabric (profile json + library
// downloads). they're basically fabric's cooler sibling.

use std::path::Path;

use async_trait::async_trait;

use super::{GameVersion, InstallError, InstallerError, ModLoaderInstaller};
use crate::instance::models::ModLoader;
use crate::net::{HttpClient, NetError, quilt as quilt_api};

pub struct QuiltInstaller;

#[async_trait]
impl ModLoaderInstaller for QuiltInstaller {
    fn loader_type(&self) -> ModLoader {
        ModLoader::Quilt
    }

    async fn get_game_versions(&self, client: &HttpClient) -> Result<Vec<GameVersion>, NetError> {
        quilt_api::fetch_quilt_game_versions(client).await
    }

    async fn get_versions(
        &self,
        client: &HttpClient,
        game_version: &str,
    ) -> Result<Vec<String>, NetError> {
        tracing::debug!(
            "Fetching Quilt loader versions for Minecraft {}",
            game_version
        );
        let loader_versions = quilt_api::fetch_quilt_versions(client, game_version).await?;
        tracing::debug!(
            "Fetched {} Quilt loader version(s) for Minecraft {}",
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
            "Installing Quilt {} for Minecraft {}",
            loader_version,
            game_version
        );
        let (profile, raw_bytes) =
            quilt_api::fetch_quilt_profile_with_raw(client, game_version, loader_version).await?;
        tracing::debug!(
            "Fetched Quilt profile {} with {} libraries",
            profile.id,
            profile.libraries.len()
        );
        quilt_api::download_quilt_libraries(client, &profile, meta_dir).await?;
        super::save_profile_bytes(
            meta_dir,
            &format!("quilt-{game_version}-{loader_version}.json"),
            &raw_bytes,
        )
        .await
        .map_err(InstallerError::from)?;
        tracing::debug!(
            "Saved Quilt profile for Minecraft {} loader {}",
            game_version,
            loader_version
        );
        Ok(())
    }
}
