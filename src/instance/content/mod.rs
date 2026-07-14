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


// scanning and toggling instance content (mods, resource packs, shaders, worlds).
// minecraft uses a ".disabled" suffix convention for disabled content, so this leans on that heavily.

pub mod install;
pub mod mods;
pub mod resource_packs;
pub mod shaders;
pub mod worlds;

pub use mods::scan_one_mod;
pub use mods::{ContentEntry, scan_mods, toggle_entry};
pub use resource_packs::{scan_one_resource_pack, scan_resource_packs};
pub use shaders::{scan_one_shader, scan_shaders};
pub use worlds::{scan_one_world, scan_worlds};

use std::io::Read;

// figures out if a file is enabled or disabled based on the ".disabled" suffix,
// and strips the extension to get a clean stem name
pub(crate) fn parse_enabled_stem(file_name: &str, ext: &str) -> Option<(bool, String)> {
    let disabled_ext = format!("{ext}.disabled");
    if let Some(stem) = file_name.strip_suffix(&disabled_ext) {
        Some((false, stem.to_string()))
    } else {
        file_name
            .strip_suffix(ext)
            .map(|stem| (true, stem.to_string()))
    }
}

// same idea but for directories, which don't have a file extension to strip

pub(crate) fn parse_enabled_stem_dir(file_name: &str) -> (bool, String) {
    if let Some(stem) = file_name.strip_suffix(".disabled") {
        (false, stem.to_string())
    } else {
        (true, file_name.to_string())
    }
}

pub(crate) fn read_icon_from_zip(archive: &mut zip::ZipArchive<std::fs::File>) -> Option<Vec<u8>> {
    let mut entry = archive.by_name("pack.png").ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub(crate) fn open_zip(path: &std::path::Path) -> Option<zip::ZipArchive<std::fs::File>> {
    let file = std::fs::File::open(path).ok()?;
    zip::ZipArchive::new(file).ok()
}
