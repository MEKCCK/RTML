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


// instance management: creation, launching, and all the
// bookkeeping that comes with pretending to be a real launcher

pub mod config_sync;
pub mod content;
pub mod desktop;
pub mod import;
pub mod launch;
pub mod loader;
pub mod log_files;
pub mod logs;
pub mod manager;
pub mod models;
pub mod running;
pub mod screenshots;

pub use content::{
    ContentEntry, scan_mods, scan_one_mod, scan_one_resource_pack, scan_one_shader, scan_one_world,
    scan_resource_packs, scan_shaders, scan_worlds, toggle_entry,
};
pub use launch::LaunchError;
pub use loader::{GameVersion, ModLoaderInstaller, VanillaInstaller, get_installer};
pub use manager::{InstanceError, InstanceManager};
pub use models::{InstanceConfig, ModLoader, normalize_memory_value};
