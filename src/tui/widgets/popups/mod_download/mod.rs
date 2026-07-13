// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

mod render;
mod state;

pub use render::{popup_rect, render};
pub use state::{DownloadStep, DownloadState, InstallParams, handle_key, take_result};
