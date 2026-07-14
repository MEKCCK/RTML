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


// base frame that all popups render inside. handles the border, title bar,
// keybind footer, and optional search indicator. content is injected via closure
// so each popup type only worries about its inner area.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Widget},
};

use crate::config::theme::BORDER_STYLE;

type ContentFn<'a> = Box<dyn Fn(Rect, &mut Buffer) + 'a>;

pub struct PopupFrame<'a> {
    pub title: Line<'a>,
    pub border_color: Color,
    pub bg: Option<Color>,
    pub keybinds: Option<Line<'a>>,
    pub search_line: Option<Line<'a>>,
    pub content: ContentFn<'a>,
}

impl<'a> Widget for PopupFrame<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // clear first so the popup doesn't layer on top of whatever was underneath

        Clear.render(area, buf);

        if let Some(bg) = self.bg {
            buf.set_style(area, Style::default().bg(bg));
        }

        let mut block = Block::bordered()
            .title_top(self.title)
            .border_type(BORDER_STYLE.to_border_type())
            .border_style(Style::default().fg(self.border_color));

        if let Some(sl) = self.search_line {
            block = block.title_top(sl.alignment(Alignment::Right));
        }

        if let Some(kb) = self.keybinds {
            block = block.title_bottom(kb.alignment(Alignment::Right));
        }

        let inner = block.inner(area);
        block.render(area, buf);
        (self.content)(inner, buf);
    }
}
