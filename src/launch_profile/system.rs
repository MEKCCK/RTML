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


// system-detection helpers shared by launch and install paths. mojang
// names some things differently from rust's std::env::consts (e.g. macOS
// is "osx" in mojang profile rules), so this module is the single source
// of truth for translating.

pub fn mojang_os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        other => other,
    }
}

pub fn mojang_arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    }
}

// the host OS version string. mojang rules occasionally constrain natives
// selection on os.version with a regex (e.g. macOS 10.x-only natives).
// rust's stdlib doesn't expose this, so we read it where it's cheap and
// reliable: linux via /proc/sys/kernel/osrelease, other platforms return
// empty. when the host string is empty, version-gated rules don't match
// (conservative default in the rule evaluator) - which is fine because
// real-world profiles using os.version are vanishingly rare.

pub fn mojang_os_version() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
            return s.trim().to_string();
        }
    }
    String::new()
}
