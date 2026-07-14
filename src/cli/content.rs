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


// unified content management for mods, resource packs, and shaders.
// all three content types share the same list/enable/disable workflow,
// just with different scanner functions plugged in.

use std::io;
use std::path::Path;

use clap::ArgMatches;

use super::utils::{require_instance, required_arg};
use crate::cli::output::print_table;
use crate::instance::ContentEntry;

type CliResult = Result<(), Box<dyn std::error::Error>>;
type Scanner = fn(&Path, &str) -> Vec<ContentEntry>;

pub fn handle_mod(matches: &ArgMatches) -> CliResult {
    handle_content(matches, "mod", crate::instance::content::mods::scan_mods)
}

pub fn handle_pack(matches: &ArgMatches) -> CliResult {
    handle_content(
        matches,
        "pack",
        crate::instance::content::resource_packs::scan_resource_packs,
    )
}

pub fn handle_shader(matches: &ArgMatches) -> CliResult {
    handle_content(
        matches,
        "shader",
        crate::instance::content::shaders::scan_shaders,
    )
}

fn handle_content(matches: &ArgMatches, kind: &str, scan: Scanner) -> CliResult {
    match matches.subcommand() {
        Some(("list", sub_matches)) => list_entries(required_arg(sub_matches, "instance")?, scan),
        Some(("enable", sub_matches)) => toggle_entry(
            required_arg(sub_matches, "instance")?,
            required_arg(sub_matches, kind)?,
            true,
            kind,
            scan,
        ),
        Some(("disable", sub_matches)) => toggle_entry(
            required_arg(sub_matches, "instance")?,
            required_arg(sub_matches, kind)?,
            false,
            kind,
            scan,
        ),
        _ => Ok(()),
    }
}

// match by filename stem (no extension) so users don't have to type ".jar"

pub(crate) fn find_entry_by_stem<'a>(
    entries: &'a [ContentEntry],
    target: &str,
) -> Option<&'a ContentEntry> {
    entries
        .iter()
        .find(|entry| entry.file_stem.eq_ignore_ascii_case(target))
}

fn list_entries(instance: &str, scan: Scanner) -> CliResult {
    let instances_dir = crate::config::SETTINGS.paths.resolve_instances_dir();
    require_instance(&instances_dir, instance)?;
    let rows = scan(&instances_dir, instance)
        .into_iter()
        .map(|entry| {
            vec![
                entry.name,
                if entry.enabled {
                    "enabled".to_string()
                } else {
                    "disabled".to_string()
                },
            ]
        })
        .collect::<Vec<_>>();

    print_table(&["Name", "State"], &rows);
    Ok(())
}

fn toggle_entry(
    instance: &str,
    target: &str,
    should_enable: bool,
    kind: &str,
    scan: Scanner,
) -> CliResult {
    let instances_dir = crate::config::SETTINGS.paths.resolve_instances_dir();
    require_instance(&instances_dir, instance)?;
    let entries = scan(&instances_dir, instance);
    let entry = find_entry_by_stem(&entries, target)
        .ok_or_else(|| io::Error::other(format!("{} '{}' not found", kind, target)))?;

    if entry.enabled == should_enable {
        println!(
            "Already {}d.",
            if should_enable { "enable" } else { "disable" }
        );
        return Ok(());
    }

    crate::instance::content::mods::toggle_entry(entry)?;
    println!(
        "{}d '{}'.",
        if should_enable { "Enable" } else { "Disable" },
        entry.name
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::find_entry_by_stem;
    use crate::instance::ContentEntry;
    use std::path::PathBuf;

    fn entry(file_stem: &str) -> ContentEntry {
        ContentEntry {
            file_stem: file_stem.to_string(),
            name: file_stem.to_string(),
            description: String::new(),
            enabled: true,
            icon_bytes: None,
            path: PathBuf::from(file_stem),
            icon_lines: None,
        }
    }

    #[test]
    fn matches_by_stem_case_insensitively() {
        let entries = vec![entry("Sodium"), entry("Lithium")];
        let found = find_entry_by_stem(&entries, "sOdIuM").expect("entry should match");
        assert_eq!(found.file_stem, "Sodium");
    }

    #[test]
    fn returns_none_for_missing_stem() {
        let entries = vec![entry("Sodium")];
        assert!(find_entry_by_stem(&entries, "iris").is_none());
    }
}
