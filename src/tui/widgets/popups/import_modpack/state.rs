// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

use crate::instance::import::ModpackFormat;
use crate::tui::widgets::instances;
use crossterm::event::{KeyCode, KeyEvent};

pub(crate) static IMPORT_STATE: LazyLock<Arc<Mutex<ImportState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(ImportState::default())));

pub(crate) static IMPORT_RESULT: LazyLock<Arc<Mutex<Option<ImportParams>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

#[derive(Debug, Clone)]
pub struct ImportParams {
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub enum ImportStep {
    #[default]
    Path,
    Detecting,
    Confirm,
    Importing,
    Done(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ImportState {
    pub step: ImportStep,
    pub path: String,
    pub detected_format: Option<ModpackFormat>,
}

impl Default for ImportState {
    fn default() -> Self {
        Self {
            step: ImportStep::Path,
            path: String::new(),
            detected_format: None,
        }
    }
}

impl ImportState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn handle_key(key_event: &KeyEvent, instances_state: &mut instances::State) {
    let mut state = match IMPORT_STATE.lock() {
        Ok(state) => state,
        Err(e) => {
            tracing::error!("Import state lock poisoned: {}", e);
            instances_state.show_import_popup = false;
            return;
        }
    };

    match state.step {
        ImportStep::Path => handle_path_key(&mut state, key_event, instances_state),
        ImportStep::Detecting => {}
        ImportStep::Confirm => handle_confirm_key(&mut state, key_event, instances_state),
        ImportStep::Importing => {}
        ImportStep::Done(_) | ImportStep::Error(_) => handle_done_key(&mut state, key_event, instances_state),
    }
}

pub fn take_result() -> Option<ImportParams> {
    match IMPORT_RESULT.lock() {
        Ok(mut r) => r.take(),
        Err(_) => None,
    }
}

fn handle_path_key(
    state: &mut ImportState,
    key_event: &KeyEvent,
    instances_state: &mut instances::State,
) {
    match key_event.code {
        KeyCode::Esc => close_popup(state, instances_state),
        KeyCode::Enter => {
            if state.path.trim().is_empty() {
                return;
            }
            let path = state.path.trim().to_string();
            state.detected_format = Some(
                std::path::Path::new(&path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| match e.to_lowercase().as_str() {
                        "mrpack" => ModpackFormat::MrPack,
                        "zip" => match crate::instance::import::detect_modpack_format(&path) {
                            Ok(f) => f,
                            Err(_) => ModpackFormat::Unknown,
                        },
                        _ => ModpackFormat::Unknown,
                    })
                    .unwrap_or(ModpackFormat::Unknown),
            );

            state.step = ImportStep::Confirm;
        }
        KeyCode::Backspace => {
            state.path.pop();
        }
        KeyCode::Char(c) => {
            state.path.push(c);
        }
        _ => {}
    }
}

fn handle_confirm_key(
    state: &mut ImportState,
    key_event: &KeyEvent,
    instances_state: &mut instances::State,
) {
    match key_event.code {
        KeyCode::Esc => close_popup(state, instances_state),
        KeyCode::Left | KeyCode::Char('h') => {
            state.step = ImportStep::Path;
        }
        KeyCode::Enter => {
            let path = state.path.trim().to_string();
            state.step = ImportStep::Importing;

            let state_arc = IMPORT_STATE.clone();
            let result_arc = IMPORT_RESULT.clone();

            tokio::spawn(async move {
                let instances_dir = crate::config::SETTINGS.paths.resolve_instances_dir();
                let meta_dir = crate::config::SETTINGS.paths.resolve_meta_dir();
                let result = crate::instance::import::import_modpack_auto(
                    &path,
                    &instances_dir,
                    &meta_dir,
                    None,
                )
                .await;

                match result {
                    Ok(instance) => {
                        if let Ok(mut r) = result_arc.lock() {
                            *r = Some(ImportParams { path });
                        }
                        if let Ok(mut s) = state_arc.lock() {
                            s.step = ImportStep::Done(format!("实例 '{}' 导入成功", instance.name));
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = state_arc.lock() {
                            s.step = ImportStep::Error(format!("导入失败: {e}"));
                        }
                    }
                }
            });
        }
        _ => {}
    }
}

fn handle_done_key(
    state: &mut ImportState,
    _key_event: &KeyEvent,
    instances_state: &mut instances::State,
) {
    close_popup(state, instances_state);
}

fn close_popup(state: &mut ImportState, instances_state: &mut instances::State) {
    state.reset();
    instances_state.show_import_popup = false;
}
