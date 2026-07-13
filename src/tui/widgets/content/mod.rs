// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

// the tabbed content area: mods, resource packs, shaders, screenshots, worlds, logs

pub mod list;
pub mod tabs;

pub use list::{ContentListState, handle_key, handle_key_no_toggle};
pub use tabs::{ContentTab, render, title};
