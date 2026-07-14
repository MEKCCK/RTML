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


// neoforge: the forge fork that broke away after 1.20.1.
// uses a different versioning scheme where neoforge versions map to
// minecraft versions by dropping the "1." prefix (e.g. MC 1.21 = NF 21.x).
// like forge, installation requires running their installer jar.

use std::path::Path;

use serde::Deserialize;

use crate::instance::loader::GameVersion;
use crate::instance::loader::InstallerError;
use crate::net::{HttpClient, NetError, download_file};
use crate::tui::progress::set_action;

const NEOFORGE_MAVEN_BASE: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const NEOFORGE_API_BASE: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[derive(Debug, Deserialize)]
struct NeoForgeMavenVersions {
    versions: Vec<String>,
}

// neoforge historically dropped the "1." from minecraft versions in
// their scheme, so "1.20.4" becomes prefix "20.4." and "1.21" becomes
// "21.0.". newer minecraft versions such as "26.1.2" keep their full
// version at the start of the neoforge coordinate: "26.1.2.76".
fn game_version_to_neoforge_prefix(game_version: &str) -> Option<String> {
    let parts: Vec<&str> = game_version.split('.').collect();
    match parts.as_slice() {
        // "1.21" → prefix "21.0."
        ["1", minor] => Some(format!("{}.0.", minor)),
        // "1.20.4" → prefix "20.4."
        ["1", minor, patch] => Some(format!("{}.{}.", minor, patch)),
        // "26.1.2" → prefix "26.1.2."
        [major, minor, patch] if leading_u32(major).is_some_and(|major| major >= 26) => {
            Some(format!("{major}.{minor}.{patch}."))
        }
        // "26.1" → prefix "26.1."
        [major, minor] if leading_u32(major).is_some_and(|major| major >= 26) => {
            Some(format!("{major}.{minor}."))
        }
        _ => None,
    }
}

fn leading_u32(raw: &str) -> Option<u32> {
    let digits: String = raw.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn neoforge_version_to_game_version(version: &str) -> Option<String> {
    let parts: Vec<&str> = version.split('.').collect();
    let major = leading_u32(parts.first()?)?;

    if major >= 26 {
        let minor = leading_u32(parts.get(1)?)?;
        let patch = leading_u32(parts.get(2)?)?;
        return Some(format!("{major}.{minor}.{patch}"));
    }

    let minor = leading_u32(parts.get(1)?)?;
    Some(if minor == 0 {
        format!("1.{major}")
    } else {
        format!("1.{major}.{minor}")
    })
}

pub async fn fetch_neoforge_versions(
    client: &HttpClient,
    game_version: &str,
) -> Result<Vec<String>, NetError> {
    fetch_neoforge_versions_from(client, NEOFORGE_API_BASE, game_version).await
}

// same as fetch_neoforge_versions but lets tests point at a wiremock server.
pub async fn fetch_neoforge_versions_from(
    client: &HttpClient,
    api_url: &str,
    game_version: &str,
) -> Result<Vec<String>, NetError> {
    let prefix = match game_version_to_neoforge_prefix(game_version) {
        Some(p) => p,
        None => {
            return Err(NetError::Parse(format!(
                "Invalid game version for NeoForge: {}",
                game_version
            )));
        }
    };

    let maven_versions: NeoForgeMavenVersions = client.get_json(api_url).await?;

    let versions: Vec<String> = maven_versions
        .versions
        .into_iter()
        .filter(|v| v.starts_with(&prefix) && !v.contains("-beta") && !v.contains("-alpha"))
        .collect();

    tracing::debug!(
        "Resolved {} NeoForge version(s) for Minecraft {} with prefix {}",
        versions.len(),
        game_version,
        prefix
    );
    Ok(versions)
}

// reverse-engineers minecraft versions from neoforge version numbers.
// e.g. neoforge "21.0.x" means MC 1.21, "20.4.x" means MC 1.20.4

pub async fn fetch_neoforge_game_versions(
    client: &HttpClient,
) -> Result<Vec<GameVersion>, NetError> {
    fetch_neoforge_game_versions_from(client, NEOFORGE_API_BASE).await
}

pub async fn fetch_neoforge_game_versions_from(
    client: &HttpClient,
    api_url: &str,
) -> Result<Vec<GameVersion>, NetError> {
    let maven: NeoForgeMavenVersions = client.get_json(api_url).await?;

    let mut game_versions: Vec<String> = Vec::new();
    for version in &maven.versions {
        if let Some(mc_version) = neoforge_version_to_game_version(version)
            && !game_versions.contains(&mc_version)
        {
            game_versions.push(mc_version);
        }
    }
    game_versions.reverse();
    tracing::debug!("Resolved {} NeoForge game version(s)", game_versions.len());

    Ok(game_versions
        .into_iter()
        .map(|version| GameVersion {
            id: version,
            stable: true,
        })
        .collect())
}

pub async fn download_neoforge_installer(
    client: &HttpClient,
    neoforge_version: &str,
    dest: &Path,
) -> Result<(), NetError> {
    let url = format!(
        "{}/{}/neoforge-{}-installer.jar",
        NEOFORGE_MAVEN_BASE, neoforge_version, neoforge_version
    );

    set_action(format!("Downloading NeoForge {}...", neoforge_version));
    tracing::info!(
        "Downloading NeoForge installer {} to {}",
        neoforge_version,
        dest.display()
    );

    download_file(client, &url, dest, |downloaded, total| {
        crate::tui::progress::set_progress(downloaded, total);
    })
    .await
}

pub async fn run_neoforge_installer(
    installer_path: &Path,
    instance_dir: &Path,
    java_path: &str,
) -> Result<(), InstallerError> {
    use tokio::process::Command;

    set_action("Running NeoForge installer...");

    let output = match Command::new(java_path)
        .arg(format!("-Duser.home={}", instance_dir.display()))
        .arg("-jar")
        .arg(installer_path)
        .arg("--installClient")
        .current_dir(instance_dir.join(".minecraft"))
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            tracing::debug!(
                "Failed to spawn NeoForge installer {} with Java {}: {}",
                installer_path.display(),
                java_path,
                e
            );
            return Err(InstallerError::Io(e));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.lines().last().unwrap_or("").trim();
        tracing::debug!(
            "NeoForge installer {} failed with status {:?}: {}",
            installer_path.display(),
            output.status.code(),
            detail
        );
        return Err(InstallerError::ProcessFailed(format!(
            "NeoForge installer exited with {:?}",
            output.status.code()
        )));
    }

    tracing::debug!("NeoForge installer completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::HttpClient;

    #[tokio::test]
    #[ignore = "hits live NeoForge API"]
    async fn test_fetch_versions() {
        let client = HttpClient::new();
        match fetch_neoforge_versions(&client, "1.21").await {
            Ok(versions) => {
                assert!(
                    !versions.is_empty(),
                    "Should have NeoForge versions for 1.21"
                );
            }
            Err(e) => panic!("fetch_neoforge_versions failed: {}", e),
        }
    }

    #[tokio::test]
    #[ignore = "hits live NeoForge API"]
    async fn test_fetch_game_versions() {
        let client = HttpClient::new();
        match fetch_neoforge_game_versions(&client).await {
            Ok(versions) => {
                assert!(!versions.is_empty(), "Should have NeoForge game versions");
                assert!(versions.iter().any(|version| version.id == "1.21"));
            }
            Err(e) => panic!("fetch_neoforge_game_versions failed: {}", e),
        }
    }

    #[test]
    fn test_game_version_to_neoforge_prefix() {
        assert_eq!(
            game_version_to_neoforge_prefix("1.21"),
            Some("21.0.".to_string())
        );
        assert_eq!(
            game_version_to_neoforge_prefix("1.20.4"),
            Some("20.4.".to_string())
        );
        assert_eq!(
            game_version_to_neoforge_prefix("1.21.1"),
            Some("21.1.".to_string())
        );
        assert_eq!(
            game_version_to_neoforge_prefix("26.1.2"),
            Some("26.1.2.".to_string())
        );
        assert_eq!(game_version_to_neoforge_prefix("invalid"), None);
    }
}
