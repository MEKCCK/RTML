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

use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

use crate::net::modrinth::{ModResult, ModVersion};
use crate::tui::widgets::instances;
use crossterm::event::{KeyCode, KeyEvent};

pub(crate) static DOWNLOAD_STATE: LazyLock<Arc<Mutex<DownloadState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(DownloadState::default())));

pub(crate) static DOWNLOAD_RESULT: LazyLock<Arc<Mutex<Option<InstallParams>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

#[derive(Debug, Clone)]
pub struct InstallParams {
    pub file_url: String,
    pub filename: String,
    pub slug: String,
    pub version_id: String,
    pub sha1_hash: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum DownloadStep {
    #[default]
    Search,
    Results,
    Versions,
    Confirm,
}

#[derive(Debug, Clone)]
pub struct DownloadState {
    pub step: DownloadStep,
    pub search_query: String,
    pub search_results: Vec<ModResult>,
    pub total_results: u64,
    pub selected_result: usize,
    pub versions: Vec<ModVersion>,
    pub selected_version: usize,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            step: DownloadStep::Search,
            search_query: String::new(),
            search_results: Vec::new(),
            total_results: 0,
            selected_result: 0,
            versions: Vec::new(),
            selected_version: 0,
            loading: false,
            error: None,
        }
    }
}

impl DownloadState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn handle_key(key_event: &KeyEvent, instances_state: &mut instances::State) {
    let mut state = match DOWNLOAD_STATE.lock() {
        Ok(state) => state,
        Err(e) => {
            tracing::error!("Download state lock poisoned: {}", e);
            instances_state.show_download_popup = false;
            return;
        }
    };

    match state.step {
        DownloadStep::Search => handle_search_key(&mut state, key_event, instances_state),
        DownloadStep::Results => handle_results_key(&mut state, key_event, instances_state),
        DownloadStep::Versions => handle_versions_key(&mut state, key_event, instances_state),
        DownloadStep::Confirm => handle_confirm_key(&mut state, key_event, instances_state),
    }
}

pub fn take_result() -> Option<InstallParams> {
    match DOWNLOAD_RESULT.lock() {
        Ok(mut r) => r.take(),
        Err(_) => None,
    }
}

fn handle_search_key(
    state: &mut DownloadState,
    key_event: &KeyEvent,
    instances_state: &mut instances::State,
) {
    match key_event.code {
        KeyCode::Esc => close_popup(state, instances_state),
        KeyCode::Enter => {
            if state.search_query.trim().is_empty() {
                return;
            }
            state.loading = true;
            state.error = None;
            let query = state.search_query.trim().to_string();
            let state_arc = DOWNLOAD_STATE.clone();
            tokio::spawn(async move {
                match crate::net::modrinth::search_mods(&query, None, None, 20, 0).await {
                    Ok((results, total)) => {
                        if let Ok(mut s) = state_arc.lock() {
                            s.search_results = results;
                            s.total_results = total;
                            s.selected_result = 0;
                            s.loading = false;
                            s.step = DownloadStep::Results;
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = state_arc.lock() {
                            s.loading = false;
                            s.error = Some(e.to_string());
                        }
                    }
                }
            });
        }
        KeyCode::Backspace => {
            state.search_query.pop();
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
        }
        _ => {}
    }
}

fn handle_results_key(
    state: &mut DownloadState,
    key_event: &KeyEvent,
    instances_state: &mut instances::State,
) {
    match key_event.code {
        KeyCode::Esc => close_popup(state, instances_state),
        KeyCode::Left | KeyCode::Char('h') => {
            state.step = DownloadStep::Search;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = state.search_results.len();
            if count > 0 {
                state.selected_result = (state.selected_result + 1).min(count.saturating_sub(1));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.selected_result = state.selected_result.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            if state.search_results.is_empty() {
                return;
            }
            let slug = state.search_results[state.selected_result].slug.clone();
            state.loading = true;
            state.error = None;
            let state_arc = DOWNLOAD_STATE.clone();
            tokio::spawn(async move {
                match crate::net::modrinth::get_mod_versions(&slug, None, None).await {
                    Ok(versions) => {
                        if let Ok(mut s) = state_arc.lock() {
                            s.versions = versions;
                            s.selected_version = 0;
                            s.loading = false;
                            s.step = DownloadStep::Versions;
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = state_arc.lock() {
                            s.loading = false;
                            s.error = Some(e.to_string());
                        }
                    }
                }
            });
        }
        _ => {}
    }
}

fn handle_versions_key(
    state: &mut DownloadState,
    key_event: &KeyEvent,
    instances_state: &mut instances::State,
) {
    match key_event.code {
        KeyCode::Esc => close_popup(state, instances_state),
        KeyCode::Left | KeyCode::Char('h') => {
            state.step = DownloadStep::Results;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let count = state.versions.len();
            if count > 0 {
                state.selected_version = (state.selected_version + 1).min(count.saturating_sub(1));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.selected_version = state.selected_version.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            if state.versions.is_empty() {
                return;
            }
            state.step = DownloadStep::Confirm;
        }
        _ => {}
    }
}

fn handle_confirm_key(
    state: &mut DownloadState,
    key_event: &KeyEvent,
    instances_state: &mut instances::State,
) {
    match key_event.code {
        KeyCode::Esc => close_popup(state, instances_state),
        KeyCode::Left | KeyCode::Char('h') => {
            state.step = DownloadStep::Versions;
        }
        KeyCode::Enter => {
            if state.versions.is_empty() {
                return;
            }
            let version = &state.versions[state.selected_version];
            if let Some(file) = version.files.first() {
                let slug = state.search_results[state.selected_result].slug.clone();
                let params = InstallParams {
                    file_url: file.url.clone(),
                    filename: file.filename.clone(),
                    slug,
                    version_id: version.id.clone(),
                    sha1_hash: file.hashes.sha1.clone(),
                };
                match DOWNLOAD_RESULT.lock() {
                    Ok(mut r) => {
                        *r = Some(params);
                    }
                    Err(e) => {
                        tracing::error!("Download result lock poisoned: {}", e);
                    }
                }
                close_popup(state, instances_state);
            }
        }
        _ => {}
    }
}

fn close_popup(state: &mut DownloadState, instances_state: &mut instances::State) {
    state.reset();
    instances_state.show_download_popup = false;
}
