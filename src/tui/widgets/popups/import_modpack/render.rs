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

use super::state::{IMPORT_STATE, ImportState, ImportStep};
use crate::config::theme::THEME;
use crate::instance::import::ModpackFormat;
use crate::tui::widgets::popups::base::PopupFrame;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

pub fn render(frame: &mut Frame, area: Rect) {
    let snapshot = match IMPORT_STATE.lock() {
        Ok(state) => state.clone(),
        Err(_) => ImportState::default(),
    };

    let theme = THEME.as_ref();

    let keybinds = match snapshot.step {
        ImportStep::Path => {
            crate::tui::widgets::popups::keybind_line(&[("Enter", " 检测"), ("Esc", " 关闭")])
        }
        ImportStep::Detecting | ImportStep::Importing => {
            crate::tui::widgets::popups::keybind_line(&[])
        }
        ImportStep::Confirm => {
            crate::tui::widgets::popups::keybind_line(&[("h", " 返回"), ("Enter", " 导入"), ("Esc", " 关闭")])
        }
        ImportStep::Done(_) | ImportStep::Error(_) => {
            crate::tui::widgets::popups::keybind_line(&[("Esc", " 关闭")])
        }
    };

    let popup = PopupFrame {
        title: Line::from(" 导入整合包 "),
        border_color: theme.text_dim(),
        bg: Some(theme.surface()),
        keybinds: Some(keybinds),
        search_line: None,
        content: Box::new(move |popup_area, buf| {
            match snapshot.step {
                ImportStep::Path => render_path_step(&snapshot, popup_area, buf),
                ImportStep::Detecting => render_loading_step("正在检测文件格式...", popup_area, buf),
                ImportStep::Confirm => render_confirm_step(&snapshot, popup_area, buf),
                ImportStep::Importing => render_loading_step("正在导入整合包...", popup_area, buf),
                ImportStep::Done(ref msg) => render_message_step(msg, false, popup_area, buf),
                ImportStep::Error(ref msg) => render_message_step(msg, true, popup_area, buf),
            }
        }),
    };

    frame.render_widget(popup, area);
}

pub fn popup_rect(frame_area: Rect) -> Rect {
    let w = Constraint::Percentage(50);
    let h = (frame_area.height * 2 / 3).max(10).min(frame_area.height.saturating_sub(4));
    frame_area.centered(w, Constraint::Length(h))
}

fn render_path_step(state: &ImportState, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();
    let text = if state.path.is_empty() {
        Line::from(vec![
            Span::styled("输入整合包文件路径...", Style::default().fg(theme.text_dim())),
            Span::styled("\u{2588}", Style::default().fg(theme.text_dim()).add_modifier(Modifier::SLOW_BLINK)),
        ])
    } else {
        Line::from(vec![
            Span::styled(&state.path, Style::default().fg(theme.text())),
            Span::styled("\u{2588}", Style::default().fg(theme.text_dim()).add_modifier(Modifier::SLOW_BLINK)),
        ])
    };
    Paragraph::new(text).render(area, buf);
}

fn render_loading_step(msg: &str, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();
    Paragraph::new(msg)
        .style(Style::default().fg(theme.text_dim()))
        .render(area, buf);
}

fn render_confirm_step(state: &ImportState, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();
    let label_style = Style::default().fg(theme.text_dim());
    let fmt = match state.detected_format {
        Some(ModpackFormat::MrPack) => "Modrinth (.mrpack)",
        Some(ModpackFormat::CurseForge) => "CurseForge (.zip)",
        Some(ModpackFormat::Unknown) | None => "未知格式",
    };

    let lines = vec![
        Line::from(vec![Span::styled("路径: ", label_style), Span::raw(&state.path)]),
        Line::from(vec![Span::styled("格式: ", label_style), Span::raw(fmt)]),
        Line::from(Span::styled("按 Enter 导入，h 返回修改路径", Style::default().fg(theme.text_dim()))),
    ];

    Paragraph::new(lines)
        .style(Style::default().fg(theme.text()))
        .wrap(Wrap { trim: true })
        .render(area, buf);
}

fn render_message_step(msg: &str, is_error: bool, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();
    let color = if is_error { theme.error() } else { theme.success() };
    Paragraph::new(msg)
        .style(Style::default().fg(color))
        .wrap(Wrap { trim: true })
        .render(area, buf);
}
