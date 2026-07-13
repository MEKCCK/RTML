// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

// primitives for parsing, merging, and rendering mojang-format launch
// profiles. consumed by the vanilla launcher and the loader install paths
// (forge/neoforge/fabric/quilt) - anything that reads a mojang-style
// version JSON.

pub mod model;
pub mod render;
pub mod resolve;
pub mod rules;
pub mod system;
pub mod templates;
