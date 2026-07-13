// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

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
