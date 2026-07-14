// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

// keybindings and input dispatch.
// 设计原则：
// 1. 方向键在所有上下文中都能导航
// 2. Tab/Shift+Tab 切换主面板，数字键直达
// 3. 相同按键 = 相同操作（减少认知负担）
// 4. Esc 始终是"返回/关闭"，且保证焦点闭环恢复
// 5. ? 显示帮助

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, FocusedArea};
use super::widgets::{
    self, WidgetKey, popups::confirm as confirm_popup,
};
use crate::tui::error_buffer;

/// 面板循环顺序：Instances → Content → Account → Settings
const PANEL_ORDER: &[FocusedArea] = &[
    FocusedArea::Instances,
    FocusedArea::Content,
    FocusedArea::Account,
    FocusedArea::Settings,
];

fn next_panel(current: FocusedArea) -> FocusedArea {
    let idx = PANEL_ORDER.iter().position(|&p| p == current).unwrap_or(0);
    PANEL_ORDER[(idx + 1) % PANEL_ORDER.len()]
}

fn prev_panel(current: FocusedArea) -> FocusedArea {
    let idx = PANEL_ORDER.iter().position(|&p| p == current).unwrap_or(0);
    PANEL_ORDER[if idx == 0 { PANEL_ORDER.len() - 1 } else { idx - 1 }]
}

impl App {
    pub(super) fn handle_key_event(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        // ── 帮助覆盖层 ──
        if self.show_help {
            match key_event.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                    self.show_help = false;
                }
                _ => {}
            }
            return Ok(());
        }

        // ── 日志覆盖层 ──
        if self.focused == FocusedArea::OverviewExpanded {
            if self.log_overlay_search.active {
                match key_event.code {
                    KeyCode::Enter => self.log_overlay_search.confirm(),
                    KeyCode::Esc => self.log_overlay_search.deactivate(),
                    KeyCode::Backspace => self.log_overlay_search.pop(),
                    KeyCode::Char(c) => self.log_overlay_search.push(c),
                    _ => {}
                }
                return Ok(());
            }
            match key_event.code {
                KeyCode::Char('O') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.focused = self.pre_overlay_focused;
                    self.log_overlay_search.deactivate();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.log_overlay_scroll < self.log_overlay_max_scroll {
                        self.log_overlay_scroll += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.log_overlay_scroll = self.log_overlay_scroll.saturating_sub(1);
                }
                KeyCode::Char('G') | KeyCode::End => {
                    self.log_overlay_scroll = self.log_overlay_max_scroll;
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    self.log_overlay_scroll = 0;
                }
                KeyCode::Char('/') => self.log_overlay_search.activate(),
                _ => {}
            }
            return Ok(());
        }

        // ── 确认删除弹窗 ──
        if self.focused == FocusedArea::ConfirmDelete {
            match key_event.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let focus_after = match confirm_popup::pending_target() {
                        Some(confirm_popup::ConfirmTarget::Instance { name }) => {
                            if let Err(e) = self.instance_manager.delete(&name) {
                                tracing::error!("Failed to delete instance '{}': {}", name, e);
                            } else {
                                self.instances_state.remove_instance(&name);
                            }
                            FocusedArea::Instances
                        }
                        Some(confirm_popup::ConfirmTarget::Account { index, .. }) => {
                            let count = self.account_state.store.accounts.len();
                            self.account_state.store.remove(index);
                            if count > 1 {
                                self.account_state.list_state.selected = Some(index.min(
                                    self.account_state.store.accounts.len().saturating_sub(1),
                                ));
                            } else {
                                self.account_state.list_state.selected = None;
                            }
                            FocusedArea::Account
                        }
                        Some(confirm_popup::ConfirmTarget::ConfigProfile { profile }) => {
                            if let Err(e) = self.delete_config_profile(&profile) {
                                error_buffer::push_error(error_buffer::ErrorEvent {
                                    id: 0,
                                    level: tracing::Level::ERROR,
                                    message: e.to_string(),
                                    pushed_at: std::time::Instant::now(),
                                });
                            }
                            FocusedArea::Settings
                        }
                        Some(confirm_popup::ConfirmTarget::Content { name, path }) => {
                            match delete_content_path(&path) {
                                Ok(()) => self.remove_content_path_from_states(&path),
                                Err(e) => {
                                    tracing::error!("Failed to delete content '{}': {}", name, e);
                                }
                            }
                            FocusedArea::Content
                        }
                        None => FocusedArea::Instances,
                    };
                    confirm_popup::clear_pending();
                    self.focused = focus_after;
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    let focus_after = match confirm_popup::pending_target() {
                        Some(confirm_popup::ConfirmTarget::Content { .. }) => FocusedArea::Content,
                        Some(confirm_popup::ConfirmTarget::Account { .. }) => FocusedArea::Account,
                        Some(confirm_popup::ConfirmTarget::ConfigProfile { .. }) => {
                            FocusedArea::Settings
                        }
                        _ => FocusedArea::Instances,
                    };
                    confirm_popup::clear_pending();
                    self.focused = focus_after;
                }
                _ => {}
            }
            return Ok(());
        }

        // ── 新建实例向导 ──
        if self.focused == FocusedArea::Popup {
            if self.instances_state.show_popup {
                widgets::popups::new_instance::handle_key(&key_event, &mut self.instances_state);
            } else if self.instances_state.show_download_popup {
                widgets::popups::mod_download::handle_key(&key_event, &mut self.instances_state);
            } else if self.instances_state.show_import_popup {
                widgets::popups::import_modpack::handle_key(&key_event, &mut self.instances_state);
            }
            if !self.instances_state.wants_popup() {
                self.focused = self.pre_popup_focused;
            }
            return Ok(());
        }

        // ── 全局快捷键 ──
        match key_event.code {
            // Tab/Shift+Tab 切换主面板
            KeyCode::Tab => {
                self.focused = if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                    prev_panel(self.focused)
                } else {
                    next_panel(self.focused)
                };
                return Ok(());
            }
            // 数字键直达面板
            KeyCode::Char('1') => {
                self.focused = FocusedArea::Instances;
                return Ok(());
            }
            KeyCode::Char('2') => {
                self.focused = FocusedArea::Content;
                return Ok(());
            }
            KeyCode::Char('3') => {
                self.focused = FocusedArea::Account;
                return Ok(());
            }
            KeyCode::Char('4') => {
                self.focused = FocusedArea::Settings;
                return Ok(());
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.show_help = true;
                return Ok(());
            }
            KeyCode::Char('q') => {
                self.exit = true;
                return Ok(());
            }
            _ => {}
        }

        // ── 内容面板：←/→/h/l 切换子标签 ──
        if self.focused == FocusedArea::Content {
            match key_event.code {
                KeyCode::Right | KeyCode::Char('l') => {
                    self.content_tab = self.content_tab.next();
                    return Ok(());
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.content_tab = self.content_tab.previous();
                    return Ok(());
                }
                _ => {}
            }
        }

        // ── 内容面板操作 ──
        if self.focused == FocusedArea::Content {
            if self.content_tab == widgets::content::ContentTab::Logs {
                if key_event.code == KeyCode::Char('d')
                    && !self.logs_state.search.active
                    && !self.logs_state.viewer_search.active
                {
                    if let Some(pending) = self.logs_state.pending_delete() {
                        confirm_popup::set_pending_content_delete(pending.name, pending.path);
                        self.focused = FocusedArea::ConfirmDelete;
                    }
                    return Ok(());
                }
                if widgets::logs_viewer::handle_key(&key_event, &mut self.logs_state) {
                    return Ok(());
                }
            } else if self.content_tab == widgets::content::ContentTab::Screenshots {
                if key_event.code == KeyCode::Char('d') && !self.screenshots_state.search.active {
                    if let Some(pending) = self.screenshots_state.pending_delete() {
                        confirm_popup::set_pending_content_delete(pending.name, pending.path);
                        self.focused = FocusedArea::ConfirmDelete;
                    }
                    return Ok(());
                }
                if widgets::screenshots_grid::handle_key(&key_event, &mut self.screenshots_state) {
                    return Ok(());
                }
            } else if self.content_tab == widgets::content::ContentTab::Worlds {
                if key_event.code == KeyCode::Char('d') && !self.worlds_state.search.active {
                    if let Some(pending) = self.worlds_state.pending_delete() {
                        confirm_popup::set_pending_content_delete(pending.name, pending.path);
                        self.focused = FocusedArea::ConfirmDelete;
                    }
                    return Ok(());
                }
                if widgets::content::list::handle_key_no_toggle(&key_event, &mut self.worlds_state)
                {
                    return Ok(());
                }
            } else {
                let state = match self.content_tab {
                    widgets::content::ContentTab::Mods => Some(&mut self.mods_state),
                    widgets::content::ContentTab::ResourcePacks => {
                        Some(&mut self.resource_packs_state)
                    }
                    widgets::content::ContentTab::Shaders => Some(&mut self.shaders_state),
                    _ => None,
                };
                if let Some(state) = state {
                    if key_event.code == KeyCode::Char('d') && !state.search.active {
                        if let Some(pending) = state.pending_delete() {
                            confirm_popup::set_pending_content_delete(pending.name, pending.path);
                            self.focused = FocusedArea::ConfirmDelete;
                        }
                        return Ok(());
                    }
                    if widgets::content::list::handle_key(&key_event, state) {
                        return Ok(());
                    }
                }
            }
        }

        // ── 账户面板 ──
        if self.focused == FocusedArea::Account {
            if key_event.code == KeyCode::Char('d')
                && let Some(index) = self.account_state.list_state.selected
                && let Some(account) = self.account_state.store.accounts.get(index)
            {
                confirm_popup::set_pending(confirm_popup::ConfirmTarget::Account {
                    username: account.username.clone(),
                    index,
                });
                self.focused = FocusedArea::ConfirmDelete;
                return Ok(());
            }
            if widgets::account::handle_key(&key_event, &mut self.account_state) {
                return Ok(());
            }
        }

        // ── 设置面板 ──
        if self.focused == FocusedArea::Settings {
            match widgets::settings::handle_key(
                &key_event,
                &mut self.settings_state,
                self.instances_state.selected_instance(),
                &self.instance_manager.instances_dir,
            ) {
                widgets::settings::SettingsAction::EditInstance(path)
                | widgets::settings::SettingsAction::EditGlobal(path) => {
                    self.pending_editor = Some(path);
                }
                widgets::settings::SettingsAction::ToggleDesktop => {
                    if let Some(inst) = self.instances_state.selected_instance() {
                        let name = inst.name.clone();
                        match crate::instance::desktop::toggle(inst) {
                            Ok(true) => {
                                error_buffer::push_error(error_buffer::ErrorEvent {
                                    id: 0,
                                    level: tracing::Level::INFO,
                                    message: format!("桌面快捷方式已创建: '{}'", name),
                                    pushed_at: std::time::Instant::now(),
                                });
                            }
                            Ok(false) => {
                                error_buffer::push_error(error_buffer::ErrorEvent {
                                    id: 0,
                                    level: tracing::Level::INFO,
                                    message: format!("桌面快捷方式已移除: '{}'", name),
                                    pushed_at: std::time::Instant::now(),
                                });
                            }
                            Err(e) => {
                                tracing::error!("Failed to toggle desktop shortcut: {}", e);
                            }
                        }
                    }
                }
                widgets::settings::SettingsAction::SelectProfile(profile) => {
                    if let Some(inst) = self.instances_state.selected_instance().cloned() {
                        let instance_dir = self.instance_manager.instances_dir.join(&inst.name);
                        match crate::instance::config_sync::switch_profile(
                            &inst.name,
                            inst.config_sync_profile.as_deref(),
                            profile.as_deref(),
                            &self.instance_manager.meta_dir,
                            &instance_dir,
                        ) {
                            Ok(selected) => {
                                let mut updated = inst.clone();
                                updated.config_sync_profile = selected;
                                if let Err(e) = self.instance_manager.save(&updated) {
                                    tracing::error!("Failed to save config profile: {}", e);
                                } else {
                                    self.instances_state.replace_instance(&inst.name, updated);
                                }
                            }
                            Err(e) => {
                                error_buffer::push_error(error_buffer::ErrorEvent {
                                    id: 0,
                                    level: tracing::Level::ERROR,
                                    message: e.to_string(),
                                    pushed_at: std::time::Instant::now(),
                                });
                            }
                        }
                    }
                }
                widgets::settings::SettingsAction::ConfirmDeleteProfile(profile) => {
                    confirm_popup::set_pending(confirm_popup::ConfirmTarget::ConfigProfile {
                        profile,
                    });
                    self.focused = FocusedArea::ConfirmDelete;
                }
                _ => {}
            }
            return Ok(());
        }

        // ── 实例面板 ──
        if self.focused == FocusedArea::Instances {
            // 重命名模式
            if self.instances_state.renaming.is_some() {
                match key_event.code {
                    KeyCode::Enter => {
                        if let Some(ref name) = self.instances_state.renaming {
                            let name = name.trim().to_string();
                            if !name.is_empty() {
                                if let Some(inst) = self.instances_state.selected_instance() {
                                    let old_name = inst.name.clone();
                                    if old_name != name {
                                        let old_dir = self
                                            .instance_manager
                                            .instances_dir
                                            .join(&old_name);
                                        let new_dir =
                                            self.instance_manager.instances_dir.join(&name);
                                        if !new_dir.exists() {
                                            if let Err(e) = std::fs::rename(&old_dir, &new_dir) {
                                                tracing::error!("Failed to rename directory: {}", e);
                                            } else {
                                                let mut updated = inst.clone();
                                                updated.name = name;
                                                let _ = self.instance_manager.save(&updated);
                                                self.instances_state.replace_instance(
                                                    &old_name,
                                                    updated,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            self.instances_state.renaming = None;
                        }
                        return Ok(());
                    }
                    KeyCode::Esc => {
                        self.instances_state.renaming = None;
                        return Ok(());
                    }
                    KeyCode::Backspace => {
                        if let Some(ref mut name) = self.instances_state.renaming {
                            name.pop();
                        }
                        return Ok(());
                    }
                    KeyCode::Char(c) => {
                        if let Some(ref mut name) = self.instances_state.renaming {
                            name.push(c);
                        }
                        return Ok(());
                    }
                    _ => {}
                }
            }

            // 搜索模式
            if self.instances_state.search.active {
                self.instances_state.handle_key(&key_event);
                return Ok(());
            }

            // 普通模式 - 实例面板快捷键
            match key_event.code {
                // 启动游戏
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(instance) = self.instances_state.selected_instance().cloned() {
                        let can_launch = matches!(
                            crate::instance::running::get(&instance.name),
                            None | Some(crate::instance::running::RunState::Crashed(_))
                        );
                        if can_launch {
                            crate::instance::running::remove(&instance.name);
                            crate::instance::logs::clear(&instance.name);
                            self.spawn_launch(instance);
                        }
                    }
                }
                // 新建实例
                KeyCode::Char('a') => {
                    self.pre_popup_focused = self.focused;
                    self.instances_state.show_popup = true;
                    self.instances_state.update_scrollbar();
                }
                // 删除实例
                KeyCode::Char('d') => {
                    if let Some(instance) = self.instances_state.selected_instance() {
                        let name = instance.name.clone();
                        confirm_popup::set_pending_instance_delete(&name);
                        self.focused = FocusedArea::ConfirmDelete;
                    }
                }
                // 重命名实例
                KeyCode::Char('r') => {
                    if let Some(inst) = self.instances_state.selected_instance() {
                        self.instances_state.renaming = Some(inst.name.clone());
                    }
                }
                // 打开实例目录
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    if let Some(instance) = self.instances_state.selected_instance() {
                        let dir = self
                            .instance_manager
                            .instances_dir
                            .join(&instance.name)
                            .join(".minecraft");
                        if let Err(e) = open::that_detached(&dir) {
                            tracing::error!("Failed to open instance directory: {}", e);
                        }
                    }
                }
                // 搜索
                KeyCode::Char('/') => {
                    self.instances_state.search.activate();
                    self.instances_state.list_state.selected = Some(0);
                    self.instances_state.update_scrollbar();
                }
                // 下载 Mod
                KeyCode::Char('m') => {
                    self.pre_popup_focused = self.focused;
                    self.instances_state.show_download_popup = true;
                }
                // 导入整合包
                KeyCode::Char('i') => {
                    self.pre_popup_focused = self.focused;
                    self.instances_state.show_import_popup = true;
                }
                // 终止运行中的实例
                KeyCode::Esc => {
                    if let Some(instance) = self.instances_state.selected_instance() {
                        crate::instance::running::send_kill(&instance.name);
                    }
                }
                // 导航
                KeyCode::Down | KeyCode::Char('j') => self.instances_state.handle_key(&key_event),
                KeyCode::Up | KeyCode::Char('k') => self.instances_state.handle_key(&key_event),
                _ => {}
            }
        }

        // ── 检查弹窗状态（闭环焦点恢复） ──
        if self.instances_state.wants_popup() {
            self.focused = FocusedArea::Popup;
        } else if self.focused == FocusedArea::Popup {
            self.focused = self.pre_popup_focused;
        }

        Ok(())
    }

    fn delete_config_profile(&mut self, profile: &str) -> color_eyre::Result<()> {
        let instances = self.instance_manager.load_all();
        for instance in instances
            .into_iter()
            .filter(|instance| instance.config_sync_profile.as_deref() == Some(profile))
        {
            let instance_dir = self.instance_manager.instances_dir.join(&instance.name);
            let mut updated = instance.clone();
            updated.config_sync_profile = crate::instance::config_sync::switch_profile(
                &instance.name,
                instance.config_sync_profile.as_deref(),
                None,
                &self.instance_manager.meta_dir,
                &instance_dir,
            )?;
            self.instance_manager.save(&updated)?;
            self.instances_state
                .replace_instance(&instance.name, updated);
        }

        crate::instance::config_sync::delete_profile(&self.instance_manager.meta_dir, profile)?;
        self.settings_state
            .profiles
            .retain(|candidate| candidate != profile);
        Ok(())
    }

    fn remove_content_path_from_states(&mut self, path: &std::path::Path) {
        self.mods_state.remove_path(path);
        self.resource_packs_state.remove_path(path);
        self.shaders_state.remove_path(path);
        self.worlds_state.remove_path(path);
        self.screenshots_state.remove_path(path);
        self.logs_state.remove_path(path);
    }
}

fn delete_content_path(path: &std::path::Path) -> std::io::Result<()> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
