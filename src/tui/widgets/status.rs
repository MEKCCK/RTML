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


// the "overview" panel showing download/install progress.
// shows a gauge when the total is known, or a spinner when it's not.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Gauge, Paragraph},
};
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::config::theme::{BORDER_STYLE, THEME};
use crate::tui::app::FocusedArea;
use crate::tui::progress::PROGRESS;

use super::styled_title;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    focused: FocusedArea,
    throbber_state: &mut ThrobberState,
) {
    let theme = THEME.as_ref();
    let border_color = if focused == FocusedArea::Overview {
        theme.accent()
    } else {
        theme.border()
    };

    let block = Block::default()
        .title(styled_title("概览", true))
        .borders(Borders::ALL)
        .border_type(BORDER_STYLE.to_border_type())
        .border_style(Style::default().fg(border_color));

    let state = match PROGRESS.lock() {
        Ok(s) => s.clone(),
        Err(_) => {
            frame.render_widget(
                Paragraph::new(Span::styled("就绪", Style::default().fg(theme.text_dim())))
                    .block(block),
                area,
            );
            return;
        }
    };

    if state.current_action.is_none() {
        frame.render_widget(
            Paragraph::new(Span::styled("就绪", Style::default().fg(theme.text_dim())))
                .block(block),
            area,
        );
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let action_text = state.current_action.as_deref().unwrap_or("");
    let sub_text = state.sub_action.as_deref().unwrap_or("");

    match state.progress {
        Some((current, total)) if total > 0 => {
            let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

            let ratio = (current as f64 / total as f64).min(1.0);
            let gauge = Gauge::default()
                .gauge_style(
                    Style::default()
                        .fg(theme.success())
                        .bg(theme.surface())
                        .add_modifier(Modifier::BOLD),
                )
                .percent((ratio * 100.0) as u16);
            frame.render_widget(gauge, chunks[0]);
            frame.render_widget(
                Paragraph::new(action_text).style(Style::default().fg(theme.text())),
                chunks[1],
            );
            if !sub_text.is_empty() {
                frame.render_widget(
                    Paragraph::new(sub_text).style(Style::default().fg(theme.text_dim())),
                    chunks[2],
                );
            }
        }
        _ => {
            let chunks =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(inner);
            let throbber = Throbber::default()
                .label(action_text)
                .style(Style::default().fg(theme.text()))
                .throbber_style(
                    Style::default()
                        .fg(theme.text_dim())
                        .add_modifier(Modifier::BOLD),
                );
            frame.render_stateful_widget(throbber, chunks[0], throbber_state);
            if !sub_text.is_empty() {
                frame.render_widget(
                    Paragraph::new(sub_text).style(Style::default().fg(theme.text_dim())),
                    chunks[1],
                );
            }
        }
    }
}
