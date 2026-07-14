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


// shared widget utilities and the trait all key-handling widgets implement

use crate::config::theme::THEME;
use crossterm::event::KeyEvent;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub mod account;
pub mod content;
pub mod instances;
pub mod logs_viewer;
pub mod popups;
pub mod screenshots_grid;
pub mod search;
pub mod settings;
pub mod status;

// highlight the first character of a title with the accent color,
// gives the UI that "keyboard shortcut hint" look

pub fn styled_title(title: &str, highlight: bool) -> Line<'_> {
    let theme = THEME.as_ref();
    if !highlight || title.is_empty() {
        Line::from(Span::raw(title))
    } else {
        let mut chars = title.chars();
        let first = chars.next().unwrap_or_default().to_string();
        let rest: String = chars.collect();
        Line::from(vec![
            Span::styled(first, Style::default().fg(theme.accent())),
            Span::styled(rest, Style::default().fg(theme.text())),
        ])
    }
}

pub trait WidgetKey {
    fn handle_key(&mut self, key_event: &KeyEvent);
}
