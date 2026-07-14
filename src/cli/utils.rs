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


// shared CLI helpers: confirmation prompts, arg extraction, instance validation

use std::io::{self, Write};

use clap::ArgMatches;

pub fn confirm(message: &str) -> Result<bool, io::Error> {
    print!("{}? [y/N] ", message);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

pub fn required_arg<'a>(matches: &'a ArgMatches, name: &str) -> Result<&'a str, io::Error> {
    matches
        .get_one::<String>(name)
        .map(String::as_str)
        .ok_or_else(|| io::Error::other(format!("missing required argument '{name}'")))
}

// checks for instance.json rather than just the directory, since a folder
// without config is just a sad empty directory pretending to be an instance

pub fn require_instance(instances_dir: &std::path::Path, name: &str) -> Result<(), io::Error> {
    if !instances_dir.join(name).join("instance.json").exists() {
        return Err(io::Error::other(format!("Instance '{name}' not found")));
    }
    Ok(())
}
