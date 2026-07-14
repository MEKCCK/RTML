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


// global progress state shared between background tasks and the status bar widget.
// background tasks set the action/progress, the render loop reads it every frame.

use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub struct ProgressState {
    pub current_action: Option<String>,
    pub progress: Option<(u64, u64)>,
    pub sub_action: Option<String>,
}

pub static PROGRESS: LazyLock<Arc<Mutex<ProgressState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(ProgressState::default())));

pub fn set_action(text: impl Into<String>) {
    let text = text.into();
    match PROGRESS.lock() {
        Ok(mut state) => {
            state.current_action = Some(text.clone());
            crate::tui::request_redraw();
        }
        Err(e) => {
            tracing::error!("Progress lock poisoned: {}", e);
        }
    }
    tracing::info!("{}", text);
}

pub fn set_progress(current: u64, total: u64) {
    match PROGRESS.lock() {
        Ok(mut state) => {
            state.progress = Some((current, total));
            crate::tui::request_redraw();
        }
        Err(e) => {
            tracing::error!("Progress lock poisoned: {}", e);
        }
    }
}

pub fn set_sub_action(text: impl Into<String>) {
    let text = text.into();
    match PROGRESS.lock() {
        Ok(mut state) => {
            state.sub_action = Some(text.clone());
            crate::tui::request_redraw();
        }
        Err(e) => {
            tracing::error!("Progress lock poisoned: {}", e);
        }
    }
    tracing::debug!("  {}", text);
}

pub fn clear() {
    match PROGRESS.lock() {
        Ok(mut state) => {
            state.current_action = None;
            state.progress = None;
            state.sub_action = None;
            crate::tui::request_redraw();
        }
        Err(e) => {
            tracing::error!("Progress lock poisoned: {}", e);
        }
    }
}

pub fn is_active() -> bool {
    PROGRESS
        .lock()
        .is_ok_and(|state| state.current_action.is_some())
}
