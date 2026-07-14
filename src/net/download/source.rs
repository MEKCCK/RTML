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

use std::sync::RwLock;

pub trait DownloadSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn version_manifest_url(&self) -> &'static str;
    fn transform_url(&self, original: &str) -> String;
    fn modrinth_api_base(&self) -> &'static str {
        "https://api.modrinth.com/v2"
    }
    fn curseforge_api_base(&self) -> &'static str {
        "https://api.curseforge.com/v1"
    }
}

pub struct OfficialSource;

impl DownloadSource for OfficialSource {
    fn name(&self) -> &'static str { "official" }
    fn version_manifest_url(&self) -> &'static str {
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
    }
    fn transform_url(&self, original: &str) -> String {
        original.to_string()
    }
}

pub struct BmclapiSource;

impl DownloadSource for BmclapiSource {
    fn name(&self) -> &'static str { "bmclapi" }
    fn version_manifest_url(&self) -> &'static str {
        "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json"
    }
    fn modrinth_api_base(&self) -> &'static str {
        "https://bmclapi2.bangbang93.com/modrinth/v2"
    }
    fn curseforge_api_base(&self) -> &'static str {
        "https://bmclapi2.bangbang93.com/curseforge/v1"
    }
    fn transform_url(&self, original: &str) -> String {
        if original.starts_with("https://libraries.minecraft.net/") {
            return original.replace(
                "https://libraries.minecraft.net/",
                "https://bmclapi2.bangbang93.com/libraries/",
            );
        }
        if original.starts_with("https://resources.download.minecraft.net/") {
            return original.replace(
                "https://resources.download.minecraft.net/",
                "https://bmclapi2.bangbang93.com/assets/",
            );
        }
        if original.starts_with("https://piston-meta.mojang.com/") {
            return original.replace(
                "https://piston-meta.mojang.com/",
                "https://bmclapi2.bangbang93.com/",
            );
        }
        if original.starts_with("https://launcher.mojang.com/") {
            return original.replace(
                "https://launcher.mojang.com/",
                "https://bmclapi2.bangbang93.com/",
            );
        }
        if original.starts_with("https://launchermeta.mojang.com/") {
            return original.replace(
                "https://launchermeta.mojang.com/",
                "https://bmclapi2.bangbang93.com/",
            );
        }
        original.to_string()
    }
}

pub struct McbbsSource;

impl DownloadSource for McbbsSource {
    fn name(&self) -> &'static str { "mcbbs" }
    fn version_manifest_url(&self) -> &'static str {
        "https://download.mcbbs.net/mc/game/version_manifest_v2.json"
    }
    fn modrinth_api_base(&self) -> &'static str {
        "https://download.mcbbs.net/modrinth/v2"
    }
    fn curseforge_api_base(&self) -> &'static str {
        "https://download.mcbbs.net/curseforge/v1"
    }
    fn transform_url(&self, original: &str) -> String {
        if original.starts_with("https://libraries.minecraft.net/") {
            return original.replace(
                "https://libraries.minecraft.net/",
                "https://download.mcbbs.net/libraries/",
            );
        }
        if original.starts_with("https://resources.download.minecraft.net/") {
            return original.replace(
                "https://resources.download.minecraft.net/",
                "https://download.mcbbs.net/assets/",
            );
        }
        if original.starts_with("https://piston-meta.mojang.com/") {
            return original.replace(
                "https://piston-meta.mojang.com/",
                "https://download.mcbbs.net/",
            );
        }
        if original.starts_with("https://launcher.mojang.com/") {
            return original.replace(
                "https://launcher.mojang.com/",
                "https://download.mcbbs.net/",
            );
        }
        if original.starts_with("https://launchermeta.mojang.com/") {
            return original.replace(
                "https://launchermeta.mojang.com/",
                "https://download.mcbbs.net/",
            );
        }
        original.to_string()
    }
}

use std::sync::OnceLock;

fn source_manager() -> &'static RwLock<SourceManager> {
    static MANAGER: OnceLock<RwLock<SourceManager>> = OnceLock::new();
    MANAGER.get_or_init(|| RwLock::new(SourceManager::new()))
}

pub struct SourceManager {
    sources: Vec<Box<dyn DownloadSource>>,
    active_index: usize,
}

impl SourceManager {
    fn new() -> Self {
        let sources: Vec<Box<dyn DownloadSource>> = vec![
            Box::new(OfficialSource),
            Box::new(BmclapiSource),
            Box::new(McbbsSource),
        ];
        let active_index = Self::resolve_active_index(&sources);
        SourceManager { sources, active_index }
    }

    fn resolve_active_index(sources: &[Box<dyn DownloadSource>]) -> usize {
        use crate::config::settings::DownloadSourceConfig;
        match crate::config::SETTINGS.general.download_source {
            DownloadSourceConfig::Mirror => {
                sources.iter().position(|s| s.name() == "bmclapi").unwrap_or(0)
            }
            DownloadSourceConfig::Official => 0,
        }
    }

    pub fn active_source_name() -> String {
        let mgr = source_manager().read().unwrap();
        mgr.sources[mgr.active_index].name().to_string()
    }

    pub fn set_active(name: &str) {
        let mut mgr = source_manager().write().unwrap();
        if let Some(idx) = mgr.sources.iter().position(|s| s.name() == name) {
            mgr.active_index = idx;
        }
    }

    pub fn version_manifest_url() -> String {
        let mgr = source_manager().read().unwrap();
        mgr.sources[mgr.active_index].version_manifest_url().to_string()
    }

    pub fn transform_url(original: &str) -> String {
        let mgr = source_manager().read().unwrap();
        mgr.sources[mgr.active_index].transform_url(original)
    }

    pub fn modrinth_api_base() -> &'static str {
        let mgr = source_manager().read().unwrap();
        mgr.sources[mgr.active_index].modrinth_api_base()
    }

    pub fn curseforge_api_base() -> &'static str {
        let mgr = source_manager().read().unwrap();
        mgr.sources[mgr.active_index].curseforge_api_base()
    }

    pub fn transform_with_fallback(original: &str) -> Vec<(String, String)> {
        let mgr = source_manager().read().unwrap();
        let mut results = Vec::new();
        results.push((
            mgr.sources[mgr.active_index].name().to_string(),
            mgr.sources[mgr.active_index].transform_url(original),
        ));
        for (i, source) in mgr.sources.iter().enumerate() {
            if i != mgr.active_index {
                results.push((
                    source.name().to_string(),
                    source.transform_url(original),
                ));
            }
        }
        results
    }
}

pub fn transform_url(original: &str) -> String {
    SourceManager::transform_url(original)
}

pub fn modrinth_api_base() -> &'static str {
    SourceManager::modrinth_api_base()
}

pub fn curseforge_api_base() -> &'static str {
    SourceManager::curseforge_api_base()
}

pub fn set_active(name: &str) {
    SourceManager::set_active(name)
}

pub fn active_source_name() -> String {
    SourceManager::active_source_name()
}

pub fn version_manifest_url() -> String {
    SourceManager::version_manifest_url()
}

pub fn all_modrinth_bases() -> Vec<&'static str> {
    let mgr = source_manager().read().unwrap();
    let active = mgr.sources[mgr.active_index].modrinth_api_base();
    let mut bases = vec![active];
    for (i, source) in mgr.sources.iter().enumerate() {
        if i != mgr.active_index {
            let base = source.modrinth_api_base();
            if !bases.contains(&base) {
                bases.push(base);
            }
        }
    }
    bases
}

pub fn all_curseforge_bases() -> Vec<&'static str> {
    let mgr = source_manager().read().unwrap();
    let active = mgr.sources[mgr.active_index].curseforge_api_base();
    let mut bases = vec![active];
    for (i, source) in mgr.sources.iter().enumerate() {
        if i != mgr.active_index {
            let base = source.curseforge_api_base();
            if !bases.contains(&base) {
                bases.push(base);
            }
        }
    }
    bases
}

pub fn asset_download_url(hash: &str) -> String {
    let prefix = if hash.len() >= 2 { &hash[..2] } else { "00" };
    let original = format!(
        "https://resources.download.minecraft.net/{}/{}",
        prefix, hash
    );
    transform_url(&original)
}

pub fn asset_local_path(assets_dir: &std::path::Path, hash: &str) -> std::path::PathBuf {
    let prefix = if hash.len() >= 2 { &hash[..2] } else { "00" };
    assets_dir.join("objects").join(prefix).join(hash)
}
