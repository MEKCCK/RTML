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

use std::path::{Path, PathBuf};

// width/height are read from the actual image file so the TUI can show
// dimensions. falls back to 1920x1080 if the file is corrupt or unreadable
// because honestly, what else are you gonna pick
#[derive(Debug, Clone)]
pub struct ScreenshotEntry {
    pub name: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

pub fn scan_screenshots(instances_dir: &Path, instance_name: &str) -> Vec<ScreenshotEntry> {
    let dir = instances_dir
        .join(instance_name)
        .join(".minecraft")
        .join("screenshots");

    let read_dir = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut entries: Vec<ScreenshotEntry> = read_dir
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg") {
                let (width, height) = image::image_dimensions(&path).unwrap_or((1920, 1080));
                Some(ScreenshotEntry {
                    name,
                    path,
                    width,
                    height,
                })
            } else {
                None
            }
        })
        .collect();

    // sorted newest-first since minecraft names them with timestamps

    entries.sort_by(|a, b| b.name.cmp(&a.name));
    entries
}
