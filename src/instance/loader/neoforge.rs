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


// neoforge installation. same java installer dance as forge (they forked from it,
// after all), just with different URLs and version naming.

use std::path::Path;

use async_trait::async_trait;

use super::{GameVersion, InstallError, ModLoaderInstaller};
use crate::instance::models::ModLoader;
use crate::net::{HttpClient, NetError, neoforge as neoforge_api};

pub struct NeoForgeInstaller;

#[async_trait]
impl ModLoaderInstaller for NeoForgeInstaller {
    fn loader_type(&self) -> ModLoader {
        ModLoader::NeoForge
    }

    async fn get_game_versions(&self, client: &HttpClient) -> Result<Vec<GameVersion>, NetError> {
        neoforge_api::fetch_neoforge_game_versions(client).await
    }

    async fn get_versions(
        &self,
        client: &HttpClient,
        game_version: &str,
    ) -> Result<Vec<String>, NetError> {
        tracing::debug!("Fetching NeoForge versions for Minecraft {}", game_version);
        let versions = neoforge_api::fetch_neoforge_versions(client, game_version).await?;
        tracing::debug!(
            "Fetched {} NeoForge version(s) for Minecraft {}",
            versions.len(),
            game_version
        );
        Ok(versions)
    }

    async fn install(
        &self,
        client: &HttpClient,
        _game_version: &str,
        loader_version: &str,
        instance_dir: &Path,
        meta_dir: &Path,
    ) -> Result<(), InstallError> {
        let installer_jar = instance_dir
            .join(".minecraft")
            .join("neoforge-installer.jar");
        tracing::info!("Installing NeoForge {}", loader_version);
        tracing::debug!("NeoForge installer path: {}", installer_jar.display());

        neoforge_api::download_neoforge_installer(client, loader_version, &installer_jar).await?;

        let java_path = crate::config::SETTINGS
            .paths
            .effective_java_path()
            .map(str::to_owned)
            .unwrap_or_else(crate::net::detect_java_path);
        tracing::debug!("Running NeoForge installer with Java {}", java_path);
        if let Err(e) =
            neoforge_api::run_neoforge_installer(&installer_jar, instance_dir, &java_path).await
        {
            let _ = tokio::fs::remove_file(&installer_jar).await;
            return Err(InstallError::Installer(e));
        }

        if let Err(e) = tokio::fs::remove_file(&installer_jar).await {
            tracing::warn!("Failed to remove NeoForge installer JAR: {}", e);
        }

        save_neoforge_profile(instance_dir, meta_dir, loader_version)
            .await
            .map_err(InstallError::Installer)?;

        tracing::debug!("Installed NeoForge {}", loader_version);
        Ok(())
    }
}

async fn save_neoforge_profile(
    instance_dir: &Path,
    meta_dir: &Path,
    loader_version: &str,
) -> Result<(), super::InstallerError> {
    let version_dir_name = format!("neoforge-{loader_version}");
    let profile_filename = format!("neoforge-{loader_version}.json");
    super::save_installer_profile(instance_dir, meta_dir, &version_dir_name, &profile_filename).await
}
