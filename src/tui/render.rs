// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

// layout and rendering. the main frame is split into:
//   left 20%: instance sidebar
//   right 80%: title bar + content area + bottom bar (account / details / status)
// popups and error toasts render on top of everything.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};
use tachyonfx::{EffectRenderer, Interpolation, Motion, fx};

use super::app::{App, ErrorEffectState, FocusedArea};
use super::widgets::{
    self, popups::confirm as confirm_popup,
};
use crate::tui::error_buffer;
use crate::tui::widgets::popups::confirm::{ConfirmPopup, confirm_popup_area};
use crate::tui::widgets::popups::error::{ErrorPopup, popup_area};

impl App {
    pub(super) fn render_frame(&mut self, frame: &mut Frame) {
        use crate::config::theme::THEME;
        use ratatui::style::Style;
        use ratatui::widgets::Block;

        let theme = THEME.as_ref();
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.background())),
            frame.area(),
        );

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.area());

        widgets::instances::render(frame, chunks[0], self.focused, &mut self.instances_state);

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
            ])
            .split(chunks[1]);

        widgets::content::title(
            frame,
            main_chunks[0],
            self.focused,
            self.instances_state.selected_instance(),
            &mut self.throbber_state,
        );
        widgets::content::render(
            frame,
            main_chunks[1],
            self.focused,
            self.content_tab,
            self.instances_state.selected_instance(),
            &mut self.mods_state,
            &mut self.resource_packs_state,
            &mut self.shaders_state,
            &mut self.worlds_state,
            &mut self.screenshots_state,
            &mut self.logs_state,
            &self.instance_manager.instances_dir,
            &self.picker,
        );

        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(24),
                Constraint::Percentage(46),
                Constraint::Percentage(30),
            ])
            .split(main_chunks[2]);

        widgets::account::render(
            frame,
            bottom_chunks[0],
            self.focused,
            &mut self.account_state,
        );
        widgets::settings::render(
            frame,
            bottom_chunks[1],
            self.focused,
            &mut self.settings_state,
            self.instances_state.selected_instance(),
            &self.instance_manager.instances_dir,
        );
        widgets::status::render(
            frame,
            bottom_chunks[2],
            self.focused,
            &mut self.throbber_state,
        );

        if self.focused == FocusedArea::OverviewExpanded {
            self.render_log_overlay(frame);
        }

        // 帮助覆盖层
        if self.show_help {
            self.render_help(frame);
        }

        // error toasts stack from the top, each one below the previous
        self.render_error_toasts(frame);

        if self.instances_state.show_popup {
            let area = widgets::popups::new_instance::popup_rect(frame.area());
            widgets::popups::new_instance::render(frame, area, self.focused);
        }

        if self.instances_state.show_download_popup {
            let area = widgets::popups::mod_download::popup_rect(frame.area());
            let game_ver = self.instances_state.selected_instance().map(|i| i.game_version.as_str());
            widgets::popups::mod_download::render(frame, area, game_ver);
        }

        if self.instances_state.show_import_popup {
            let area = widgets::popups::import_modpack::popup_rect(frame.area());
            widgets::popups::import_modpack::render(frame, area);
        }

        if self.focused == FocusedArea::ConfirmDelete
            && let Some(target) = confirm_popup::pending_target()
        {
            let area = confirm_popup_area(frame.area(), &target);
            frame.render_widget(ConfirmPopup::for_target(&target), area);
        }

        if self.show_online_popup {
            let area = crate::tui::online::popup_rect(frame.area());
            crate::tui::online::render(frame, area);
        }
    }

    // full-screen log viewer with search highlighting and auto-scroll.
    // auto-sticks to the bottom unless the user scrolled up manually
    fn render_log_overlay(&mut self, frame: &mut Frame) {
        use crate::config::theme::{BORDER_STYLE, THEME};
        use crate::tui::logging::get_app_logs;
        use ratatui::{
            layout::{Alignment, Margin},
            style::{Modifier, Style},
            text::Line,
            widgets::{Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation},
        };

        let theme = THEME.as_ref();
        let area = frame.area();
        let overlay = area.inner(Margin::new(1, 1));

        frame.render_widget(Clear, overlay);

        let all_lines = get_app_logs();
        let filtered: Vec<&String> = all_lines
            .iter()
            .filter(|l| self.log_overlay_search.matches(l))
            .collect();

        let visible_height = overlay.height.saturating_sub(2) as usize;
        let was_at_bottom =
            self.log_overlay_scroll >= self.log_overlay_max_scroll.saturating_sub(1);
        self.log_overlay_max_scroll = filtered.len().saturating_sub(visible_height);
        if was_at_bottom || self.log_overlay_scroll > self.log_overlay_max_scroll {
            self.log_overlay_scroll = self.log_overlay_max_scroll;
        }
        self.log_overlay_scrollbar =
            ratatui::widgets::ScrollbarState::new(self.log_overlay_max_scroll)
                .position(self.log_overlay_scroll);

        let mut block = Block::bordered()
            .title_top(
                Line::from(" Logs ").style(
                    Style::default()
                        .fg(theme.text())
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .title_bottom(
                crate::tui::widgets::popups::keybind_line(&[("O", " 关闭"), ("/", " 搜索")])
                    .alignment(Alignment::Right),
            )
            .border_type(BORDER_STYLE.to_border_type())
            .border_style(Style::default().fg(theme.accent()));

        if let Some(sl) = self.log_overlay_search.title_line() {
            block = block.title_top(sl);
        }

        let inner = block.inner(overlay);
        frame.render_widget(block, overlay);

        let search = &self.log_overlay_search;
        let styled: Vec<Line> = filtered
            .iter()
            .skip(self.log_overlay_scroll)
            .take(visible_height)
            .map(|line| {
                let style = if line.contains("ERROR") || line.contains("FATAL") {
                    Style::default().fg(theme.error())
                } else if line.contains("WARN") {
                    Style::default().fg(theme.warning())
                } else if line.contains("DEBUG") || line.contains("TRACE") {
                    Style::default().fg(theme.text_dim())
                } else {
                    Style::default().fg(theme.text())
                };
                search.highlight_line(line, style)
            })
            .collect();

        frame.render_widget(Paragraph::new(styled), inner);

        let scrollbar_area = ratatui::layout::Rect {
            x: overlay.x + overlay.width.saturating_sub(1),
            y: overlay.y + 1,
            width: 1,
            height: overlay.height.saturating_sub(2),
        };
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("\u{25b2}"))
                .style(
                    Style::default()
                        .fg(theme.text_dim())
                        .add_modifier(Modifier::BOLD),
                )
                .thumb_symbol("\u{2551}")
                .track_symbol(Some(""))
                .end_symbol(Some("\u{25bc}")),
            scrollbar_area,
            &mut self.log_overlay_scrollbar,
        );
    }

    // 帮助屏幕 - 显示所有快捷键
    fn render_help(&self, frame: &mut Frame) {
        use crate::config::theme::{BORDER_STYLE, THEME};
        use ratatui::{
            layout::{Margin},
            style::{Modifier, Style},
            text::{Line, Span},
            widgets::{Block, Clear, Paragraph, Wrap},
        };

        let theme = THEME.as_ref();
        let area = frame.area();
        let overlay = area.inner(Margin::new(2, 2));

        frame.render_widget(Clear, overlay);

        let block = Block::bordered()
            .title_top(
                Line::from(" 帮助 / 快捷键 ").style(
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .title_bottom(
                Line::from(" 按 Esc 或 ? 关闭 ").style(
                    Style::default().fg(theme.text_dim()),
                ),
            )
            .border_type(BORDER_STYLE.to_border_type())
            .border_style(Style::default().fg(theme.accent()));

        let inner = block.inner(overlay);
        frame.render_widget(block, overlay);

        let help_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  全局快捷键", Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from("  ──────────────────────────────────────"),
            Line::from(vec![
                Span::styled("    Tab", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("            切换面板 (实例→内容→账户→设置)", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    Shift+Tab", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("       反向切换面板", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    1-4", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             直达面板 (实例/内容/账户/设置)", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    ?/h", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("              显示/隐藏帮助", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    q", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("                 退出程序", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    Esc", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("               返回 / 关闭弹窗", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  实例面板", Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from("  ──────────────────────────────────────"),
            Line::from(vec![
                Span::styled("    Enter/Space", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("  启动游戏", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    a", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             新建实例", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    m", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             下载 Mod", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    i", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             导入整合包", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    t", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             局域网联机", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    d", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             删除实例", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    r", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             重命名实例", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    o", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             打开实例目录", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    /", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             搜索实例", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    ↑↓/jk", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("        上下导航", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  内容面板", Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from("  ──────────────────────────────────────"),
            Line::from(vec![
                Span::styled("    ←→/hl", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("        切换标签 (模组/资源包/光影/截图/存档/日志)", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    Space", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("          切换模组启用/禁用", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    d", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             删除选中内容", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    o", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             打开目录", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    /", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             搜索", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  日志覆盖层", Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from("  ──────────────────────────────────────"),
            Line::from(vec![
                Span::styled("    O/Esc", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("        关闭日志", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    g/G", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("          跳转到顶部/底部", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(vec![
                Span::styled("    /", Style::default().fg(theme.text()).add_modifier(Modifier::BOLD)),
                Span::styled("             搜索日志", Style::default().fg(theme.text_dim())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  按 Esc 或 ? 关闭帮助", Style::default().fg(theme.text_dim())),
            ]),
        ];

        let paragraph = Paragraph::new(help_text)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.text()));
        frame.render_widget(paragraph, inner);
    }

    // keeps the effect map in sync with current errors: removes effects for
    // dismissed errors and creates slide-in effects for new ones
    fn sync_error_effects(&mut self, events: &[error_buffer::ErrorEvent]) {
        use crate::config::theme::THEME;
        let theme = THEME.as_ref();
        let bg = theme.background();
        let active_ids: std::collections::HashSet<u64> =
            events.iter().map(|event| event.id).collect();
        self.error_effects.retain(|id, _| active_ids.contains(id));

        for event in events {
            self.error_effects.entry(event.id).or_insert_with(|| {
                ErrorEffectState::SlidingIn(
                    fx::slide_in(
                        Motion::RightToLeft,
                        4,
                        0,
                        bg,
                        (250, Interpolation::CubicOut),
                    ),
                    std::time::Instant::now(),
                )
            });
        }
    }

    // drives the slide-in / idle / slide-out state machine for each error toast.
    // transitions to FadingOut once it's within fly_out_ms of auto-dismiss time
    fn render_error_effect(
        &mut self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        event: &error_buffer::ErrorEvent,
        elapsed_ms: u128,
    ) {
        use crate::config::SETTINGS;
        use crate::config::theme::THEME;
        let theme = THEME.as_ref();
        let bg = theme.background();
        let fly_out_ms = SETTINGS.ui.error_fly_out_ms as u128;
        let fly_start_ms = SETTINGS.ui.error_auto_dismiss_ms as u128
            - fly_out_ms.min(SETTINGS.ui.error_auto_dismiss_ms as u128);

        if elapsed_ms >= fly_start_ms {
            let entry = self
                .error_effects
                .entry(event.id)
                .or_insert(ErrorEffectState::Idle);
            if !matches!(entry, ErrorEffectState::FadingOut(..)) {
                *entry = ErrorEffectState::FadingOut(
                    fx::slide_out(
                        Motion::LeftToRight,
                        4,
                        0,
                        bg,
                        (fly_out_ms as u32, Interpolation::CubicIn),
                    ),
                    std::time::Instant::now(),
                );
            }
        }

        if let Some(effect_state) = self.error_effects.get_mut(&event.id) {
            match effect_state {
                ErrorEffectState::SlidingIn(effect, started) => {
                    let dt = started.elapsed().as_millis() as u32;
                    if effect.running() {
                        frame.render_effect(
                            effect,
                            area,
                            tachyonfx::Duration::from_millis(dt.min(32)),
                        );
                        *started = std::time::Instant::now();
                    } else {
                        *effect_state = ErrorEffectState::Idle;
                    }
                }
                ErrorEffectState::Idle => {}
                ErrorEffectState::FadingOut(effect, started) => {
                    let dt = started.elapsed().as_millis() as u32;
                    if effect.running() {
                        frame.render_effect(
                            effect,
                            area,
                            tachyonfx::Duration::from_millis(dt.min(32)),
                        );
                        *started = std::time::Instant::now();
                    }
                }
            }
        }
    }

    // 渲染错误提示
    fn render_error_toasts(&mut self, frame: &mut Frame) {
        let all_errors = error_buffer::peek_all_errors();
        self.sync_error_effects(&all_errors);
        let mut next_y: u16 = 1;
        for event in all_errors {
            let elapsed_ms = event.pushed_at.elapsed().as_millis();
            if let Some(area) = popup_area(frame.area(), &event.message, next_y, elapsed_ms) {
                next_y = next_y.saturating_add(area.height + 1);
                frame.render_widget(ErrorPopup::new(event.clone()), area);
                self.render_error_effect(frame, area, &event, elapsed_ms);
            }
        }
    }
}
