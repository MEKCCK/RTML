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


// "are you sure?" popup for destructive actions. uses global state so the
// confirmation target persists across render frames.

use std::sync::LazyLock;
use std::sync::Mutex;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::config::theme::THEME;

static CONFIRM_STATE: LazyLock<Mutex<ConfirmState>> =
    LazyLock::new(|| Mutex::new(ConfirmState::default()));

#[derive(Debug, Default)]
struct ConfirmState {
    target: Option<ConfirmTarget>,
}

#[derive(Debug, Clone)]
pub enum ConfirmTarget {
    Instance {
        name: String,
    },
    Account {
        username: String,
        index: usize,
    },
    ConfigProfile {
        profile: String,
    },
    Content {
        name: String,
        path: std::path::PathBuf,
    },
}

impl ConfirmTarget {
    fn title(&self) -> String {
        format!(" 删除 '{}' ", self.name())
    }

    fn body(&self) -> &'static str {
        match self {
            ConfirmTarget::Instance { .. } => "此操作将永久删除该实例",
            ConfirmTarget::Account { .. } => "此操作将永久删除该账户",
            ConfirmTarget::ConfigProfile { .. } => {
                "此操作将永久删除该配置"
            }
            ConfirmTarget::Content { .. } => "此操作将永久删除所选内容",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ConfirmTarget::Instance { name } => name,
            ConfirmTarget::Account { username, .. } => username,
            ConfirmTarget::ConfigProfile { profile } => profile,
            ConfirmTarget::Content { name, .. } => name,
        }
    }
}

pub fn set_pending(target: ConfirmTarget) {
    match CONFIRM_STATE.lock() {
        Ok(mut s) => {
            s.target = Some(target);
        }
        Err(e) => {
            tracing::error!("Confirm popup state lock poisoned: {}", e);
        }
    }
}

pub fn set_pending_delete(name: impl Into<String>) {
    set_pending(ConfirmTarget::Instance { name: name.into() });
}

pub fn set_pending_instance_delete(name: impl Into<String>) {
    set_pending_delete(name);
}

pub fn set_pending_content_delete(name: impl Into<String>, path: impl Into<std::path::PathBuf>) {
    set_pending(ConfirmTarget::Content {
        name: name.into(),
        path: path.into(),
    });
}

pub fn pending_target() -> Option<ConfirmTarget> {
    match CONFIRM_STATE.lock() {
        Ok(s) => s.target.clone(),
        Err(_) => None,
    }
}

pub fn clear_pending() {
    match CONFIRM_STATE.lock() {
        Ok(mut s) => {
            s.target = None;
        }
        Err(e) => {
            tracing::error!("Confirm popup state lock poisoned: {}", e);
        }
    }
}

pub struct ConfirmPopup {
    title: String,
    body: &'static str,
}

impl ConfirmPopup {
    pub fn new(title: impl Into<String>, body: &'static str) -> Self {
        Self {
            title: title.into(),
            body,
        }
    }

    pub fn for_target(target: &ConfirmTarget) -> Self {
        Self::new(target.title(), target.body())
    }
}

impl Widget for ConfirmPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use super::{base::PopupFrame, keybind_line};

        let theme = THEME.as_ref();
        let title = Line::from(vec![Span::styled(
            self.title,
            Style::default()
                .fg(theme.text_dim())
                .add_modifier(Modifier::BOLD),
        )]);
        let kb = keybind_line(&[("Enter", " 确认")]);

        let border_color = theme.text_dim();
        let bg_color = theme.surface();
        let text_color = theme.text();
        let popup = PopupFrame {
            title,
            border_color,
            bg: Some(bg_color),
            keybinds: Some(kb),
            search_line: None,
            content: Box::new(move |inner, buf| {
                Paragraph::new(self.body)
                    .style(Style::default().fg(text_color))
                    .render(inner, buf);
            }),
        };

        popup.render(area, buf);
    }
}

pub fn confirm_popup_area(frame_area: Rect, target: &ConfirmTarget) -> Rect {
    use super::word_wrap_size;
    use ratatui::layout::Constraint;
    const MAX_W: usize = 48;
    let title_w = target.name().len() + 12;
    let (body_w, _) = word_wrap_size(target.body(), MAX_W);
    let inner_w = title_w.max(body_w).min(MAX_W);
    let (_, lines) = word_wrap_size(target.body(), inner_w);
    let popup_w = ((inner_w + 2) as u16).min(frame_area.width.saturating_sub(4));
    let popup_h = ((lines + 2) as u16).min(frame_area.height.saturating_sub(4));
    frame_area.centered(Constraint::Length(popup_w), Constraint::Length(popup_h))
}
