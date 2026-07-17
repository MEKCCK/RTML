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


// crate root. main.rs is a thin wrapper that imports the two entry points
// re-exported below; everything else stays crate-private. integration tests
// in tests/ that need to reach in deeper can use `RTML::auth`, `RTML::net`,
// etc. directly; cli + migrate stay private because they have nothing
// general to expose.

pub mod auth;
mod cli;
pub mod config;
pub mod instance;
pub mod launch_profile;
pub mod net;
pub mod online;
pub mod tui;

pub use cli::init as cli_init;
pub use config::migrate::run_legacy_rename as migrate_legacy_rename;

use std::sync::atomic::{AtomicBool, Ordering};

static HM_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_hm_mode(enabled: bool) {
    HM_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_hm_mode() -> bool {
    HM_MODE.load(Ordering::Relaxed)
}

/// 许可证全文，编译时嵌入。

pub const LICENSE_TEXT: &str = include_str!("../LICENSE");
