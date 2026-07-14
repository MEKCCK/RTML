use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};
use tui_prompts::{FocusState, State as PromptState, TextState};

use crate::config::theme::THEME;
use crate::online::manager::{TerracottaManager, parse_invite_code};
use crate::online::profile::TerracottaProfile;
use crate::online::state::TerracottaState;
use crate::tui::widgets::popups::base::PopupFrame;
use crate::tui::widgets::styled_title;

static ONLINE_STATE: LazyLock<Arc<Mutex<OnlinePopupState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(OnlinePopupState::default())));
static ONLINE_MANAGER: LazyLock<Mutex<Option<TerracottaManager>>> =
    LazyLock::new(|| Mutex::new(None));
static POPUP_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_connected() -> bool {
    ONLINE_MANAGER.lock().ok().map_or(false, |m| {
        m.as_ref().map_or(false, |m| m.port().is_some())
    })
}

pub fn is_active() -> bool {
    POPUP_ACTIVE.load(Ordering::Acquire)
}

pub fn open() {
    POPUP_ACTIVE.store(true, Ordering::Release);
    if let Ok(mut state) = ONLINE_STATE.lock() {
        if is_connected() {
            if let Ok(manager) = ONLINE_MANAGER.lock() {
                if let Some(ref m) = *manager {
                    if let TerracottaState::HostOK { code, profiles, .. } = m.state() {
                        state.step = OnlineStep::HostReady;
                        state.invite_code = code.clone();
                        state.profiles = profiles.clone();
                        return;
                    }
                    if let TerracottaState::GuestOK { profiles, .. } = m.state() {
                        state.step = OnlineStep::GuestReady;
                        state.profiles = profiles.clone();
                        return;
                    }
                }
            }
        }
        *state = OnlinePopupState::default();
    }
}

pub fn close() {
    POPUP_ACTIVE.store(false, Ordering::Release);
    if let Ok(mut state) = ONLINE_STATE.lock() {
        state.step = OnlineStep::Menu;
        state.invite_code = String::new();
        state.error_message = String::new();
    }
}

pub fn disconnect() {
    if let Ok(mut manager) = ONLINE_MANAGER.lock() {
        if let Some(ref mut m) = *manager {
            m.kill();
        }
        *manager = None;
    }
    if is_active() {
        if let Ok(mut state) = ONLINE_STATE.lock() {
            *state = OnlinePopupState::default();
        }
    }
}

#[derive(Debug, Clone)]
pub enum OnlineStep {
    Menu,
    HostPreparing,
    HostStarting,
    HostReady,
    GuestPreparing,
    GuestInput,
    GuestConnecting,
    GuestReady,
    Error,
}

#[derive(Debug)]
pub struct OnlinePopupState {
    pub step: OnlineStep,
    pub menu_selection: usize,
    pub invite_code: String,
    pub invite_code_input: TextState<'static>,
    pub error_message: String,
    pub player_name: String,
    pub profiles: Vec<TerracottaProfile>,
}

#[derive(Debug, Clone)]
pub struct OnlinePopupSnapshot {
    pub step: OnlineStep,
    pub menu_selection: usize,
    pub invite_code: String,
    pub invite_code_value: String,
    pub error_message: String,
    pub player_name: String,
    pub profiles: Vec<TerracottaProfile>,
}

impl Default for OnlinePopupState {
    fn default() -> Self {
        let player_name = crate::auth::AccountStore::load()
            .active_account()
            .map(|a| a.username.clone())
            .unwrap_or_else(|| "Player".to_string());

        Self {
            step: OnlineStep::Menu,
            menu_selection: 0,
            invite_code: String::new(),
            invite_code_input: TextState::new().with_focus(FocusState::Focused),
            error_message: String::new(),
            player_name,
            profiles: Vec::new(),
        }
    }
}

impl OnlinePopupState {
    fn snapshot(&self) -> OnlinePopupSnapshot {
        OnlinePopupSnapshot {
            step: self.step.clone(),
            menu_selection: self.menu_selection,
            invite_code: self.invite_code.clone(),
            invite_code_value: self.invite_code_input.value().to_string(),
            error_message: self.error_message.clone(),
            player_name: self.player_name.clone(),
            profiles: self.profiles.clone(),
        }
    }
}

pub fn handle_key(key_event: &KeyEvent) {
    if matches!(key_event.code, KeyCode::Esc) {
        close();
        return;
    }

    let mut state = match ONLINE_STATE.lock() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Online state lock poisoned: {}", e);
            POPUP_ACTIVE.store(false, Ordering::Release);
            return;
        }
    };

    if matches!(key_event.code, KeyCode::Char('d'))
        && matches!(state.step, OnlineStep::HostReady | OnlineStep::GuestReady)
    {
        tokio::spawn(async move {
            let mut manager = take_manager();
            if let Some(ref mut m) = manager {
                let _ = m.set_idle().await;
            }
            if let Some(m) = manager {
                put_manager(m);
            }
        });
        POPUP_ACTIVE.store(false, Ordering::Release);
        *state = OnlinePopupState::default();
        return;
    }

    match state.step {
        OnlineStep::Menu => handle_menu_key(&mut state, key_event),
        OnlineStep::HostPreparing | OnlineStep::HostStarting => {}
        OnlineStep::HostReady => {}
        OnlineStep::GuestPreparing | OnlineStep::GuestConnecting => {}
        OnlineStep::GuestInput => handle_guest_input_key(&mut state, key_event),
        OnlineStep::GuestReady => {}
        OnlineStep::Error => {}
    }
}

fn handle_menu_key(state: &mut OnlinePopupState, key_event: &KeyEvent) {
    match key_event.code {
        KeyCode::Char('j') | KeyCode::Down => {
            state.menu_selection = (state.menu_selection + 1).min(2);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.menu_selection = state.menu_selection.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            let player_name = state.player_name.clone();
            match state.menu_selection {
                0 => {
                    state.step = OnlineStep::HostPreparing;
                    spawn_host(player_name);
                }
                1 => {
                    state.step = OnlineStep::GuestPreparing;
                    spawn_guest_download();
                }
                2 => {
                    POPUP_ACTIVE.store(false, Ordering::Release);
                    state.step = OnlineStep::Menu;
                    state.invite_code = String::new();
                    state.error_message = String::new();
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_guest_input_key(state: &mut OnlinePopupState, key_event: &KeyEvent) {
    match key_event.code {
        KeyCode::Enter => {
            let code = state.invite_code_input.value().trim().to_string();
            if code.is_empty() {
                return;
            }
            if let Some(parsed) = parse_invite_code(&code) {
                state.invite_code = parsed.clone();
                state.step = OnlineStep::GuestConnecting;
                spawn_guest_connect(parsed, state.player_name.clone());
            }
        }
        _ => {
            state.invite_code_input.handle_key_event(*key_event);
        }
    }
}

fn take_manager() -> Option<TerracottaManager> {
    ONLINE_MANAGER.lock().ok()?.take()
}

fn put_manager(manager: TerracottaManager) {
    if let Ok(mut m) = ONLINE_MANAGER.lock() {
        *m = Some(manager);
    }
}

fn spawn_host(player_name: String) {
    let state_arc = ONLINE_STATE.clone();

    tokio::spawn(async move {
        let pid = crate::instance::running::running_instance_pid();
        if pid.is_none() {
            if let Ok(mut state) = state_arc.lock() {
                state.error_message =
                    "没有正在运行的游戏实例，请先启动游戏并在游戏中开放局域网".to_string();
                state.step = OnlineStep::Error;
            }
            return;
        }

        let mut manager = match TerracottaManager::new().await {
            Ok(m) => m,
            Err(e) => {
                if let Ok(mut state) = state_arc.lock() {
                    state.error_message = format!("准备失败: {e}");
                    state.step = OnlineStep::Error;
                }
                return;
            }
        };

        put_manager(manager);
        if let Ok(mut state) = state_arc.lock() {
            state.step = OnlineStep::HostStarting;
        }

        manager = match take_manager() {
            Some(m) => m,
            None => return,
        };

        if let Err(e) = manager.start().await {
            put_manager(manager);
            if let Ok(mut state) = state_arc.lock() {
                state.error_message = format!("启动失败: {e}");
                state.step = OnlineStep::Error;
            }
            return;
        }

        let _ = manager.set_idle().await;

        if let Err(e) = manager.start_host(&player_name).await {
            put_manager(manager);
            if let Ok(mut state) = state_arc.lock() {
                state.error_message = format!("创建房间失败: {e}");
                state.step = OnlineStep::Error;
            }
            return;
        }

        let poll_start = std::time::Instant::now();
        loop {
            if poll_start.elapsed().as_secs() > 60 {
                put_manager(manager);
                if let Ok(mut state) = state_arc.lock() {
                    state.error_message = "超时".to_string();
                    state.step = OnlineStep::Error;
                }
                return;
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            match manager.poll_state().await {
                Ok(TerracottaState::HostOK { code, profiles, .. }) => {
                    put_manager(manager);
                    if let Ok(mut state) = state_arc.lock() {
                        state.invite_code = code;
                        state.profiles = profiles;
                        state.step = OnlineStep::HostReady;
                    }
                    spawn_profile_poller();
                    return;
                }
                Ok(TerracottaState::Exception(ex)) => {
                    put_manager(manager);
                    if let Ok(mut state) = state_arc.lock() {
                        state.error_message = format!("异常: {ex:?}");
                        state.step = OnlineStep::Error;
                    }
                    return;
                }
                Err(e) => {
                    put_manager(manager);
                    if let Ok(mut state) = state_arc.lock() {
                        state.error_message = format!("状态获取失败: {e}");
                        state.step = OnlineStep::Error;
                    }
                    return;
                }
                _ => {
                    put_manager(manager);
                    manager = match take_manager() {
                        Some(m) => m,
                        None => return,
                    };
                }
            }
        }
    });
}

fn spawn_guest_download() {
    let state_arc = ONLINE_STATE.clone();
    tokio::spawn(async move {
        match TerracottaManager::new().await {
            Ok(manager) => {
                put_manager(manager);
                if let Ok(mut state) = state_arc.lock() {
                    state.step = OnlineStep::GuestInput;
                }
            }
            Err(e) => {
                if let Ok(mut state) = state_arc.lock() {
                    state.error_message = format!("下载失败: {e}");
                    state.step = OnlineStep::Error;
                }
            }
        }
    });
}

fn spawn_guest_connect(invite_code: String, player_name: String) {
    let state_arc = ONLINE_STATE.clone();
    tokio::spawn(async move {
        let mut manager = match take_manager() {
            Some(m) => m,
            None => {
                if let Ok(mut state) = state_arc.lock() {
                    state.error_message = "内部错误: 管理器未初始化".to_string();
                    state.step = OnlineStep::Error;
                }
                return;
            }
        };

        if let Err(e) = manager.start().await {
            put_manager(manager);
            if let Ok(mut state) = state_arc.lock() {
                state.error_message = format!("启动失败: {e}");
                state.step = OnlineStep::Error;
            }
            return;
        }

        if let Err(e) = manager.start_guest(&invite_code, &player_name).await {
            put_manager(manager);
            if let Ok(mut state) = state_arc.lock() {
                state.error_message = format!("连接失败: {e}");
                state.step = OnlineStep::Error;
            }
            return;
        }

        let poll_start = std::time::Instant::now();
        loop {
            if poll_start.elapsed().as_secs() > 60 {
                put_manager(manager);
                if let Ok(mut state) = state_arc.lock() {
                    state.error_message = "连接超时".to_string();
                    state.step = OnlineStep::Error;
                }
                return;
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            match manager.poll_state().await {
                Ok(TerracottaState::GuestOK { profiles, .. }) => {
                    put_manager(manager);
                    if let Ok(mut state) = state_arc.lock() {
                        state.profiles = profiles;
                        state.step = OnlineStep::GuestReady;
                    }
                    spawn_profile_poller();
                    return;
                }
                Ok(TerracottaState::Exception(ex)) => {
                    put_manager(manager);
                    if let Ok(mut state) = state_arc.lock() {
                        state.error_message = format!("异常: {ex:?}");
                        state.step = OnlineStep::Error;
                    }
                    return;
                }
                Err(e) => {
                    put_manager(manager);
                    if let Ok(mut state) = state_arc.lock() {
                        state.error_message = format!("状态获取失败: {e}");
                        state.step = OnlineStep::Error;
                    }
                    return;
                }
                _ => {
                    put_manager(manager);
                    manager = match take_manager() {
                        Some(m) => m,
                        None => return,
                    };
                }
            }
        }
    });
}

fn spawn_profile_poller() {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            if !is_connected() {
                return;
            }

            let mut manager = match take_manager() {
                Some(m) => m,
                None => return,
            };

            match manager.poll_state().await {
                Ok(TerracottaState::HostOK { profiles, .. }) => {
                    if let Ok(mut state) = ONLINE_STATE.lock() {
                        state.profiles = profiles;
                    }
                    put_manager(manager);
                }
                Ok(TerracottaState::GuestOK { profiles, .. }) => {
                    if let Ok(mut state) = ONLINE_STATE.lock() {
                        state.profiles = profiles;
                    }
                    put_manager(manager);
                }
                Ok(TerracottaState::Exception(ex)) => {
                    let _ = manager.set_idle().await;
                    put_manager(manager);
                    if let Ok(mut state) = ONLINE_STATE.lock() {
                        state.error_message = format!("房间已关闭: {ex:?}");
                        state.step = OnlineStep::Error;
                    }
                    return;
                }
                Err(_) => {
                    put_manager(manager);
                    if let Ok(mut state) = ONLINE_STATE.lock() {
                        state.error_message = "与守护进程的连接已断开".to_string();
                        state.step = OnlineStep::Error;
                    }
                    return;
                }
                _ => {
                    put_manager(manager);
                }
            }
        }
    });
}

pub fn render(frame: &mut Frame, area: Rect) {
    let snapshot = match ONLINE_STATE.lock() {
        Ok(s) => s.snapshot(),
        Err(e) => {
            tracing::error!("Online state lock poisoned: {}", e);
            return;
        }
    };

    let keybinds = step_keybinds(&snapshot);
    let theme = THEME.as_ref();

    let popup = PopupFrame {
        title: styled_title("联机", false),
        border_color: theme.text_dim(),
        bg: Some(theme.surface()),
        keybinds: Some(keybinds),
        search_line: None,
        content: Box::new(move |popup_area, buf| {
            match snapshot.step {
                OnlineStep::Menu => render_menu(&snapshot, popup_area, buf),
                OnlineStep::HostPreparing => {
                    Paragraph::new("正在准备 Terracotta...")
                        .style(Style::default().fg(theme.text_dim()))
                        .render(popup_area, buf);
                }
                OnlineStep::HostStarting => {
                    Paragraph::new("正在寻找游戏端口...")
                        .style(Style::default().fg(theme.text_dim()))
                        .render(popup_area, buf);
                }
                OnlineStep::HostReady => render_host_ready(&snapshot, popup_area, buf),
                OnlineStep::GuestPreparing => {
                    Paragraph::new("正在准备 Terracotta...")
                        .style(Style::default().fg(theme.text_dim()))
                        .render(popup_area, buf);
                }
                OnlineStep::GuestInput => render_guest_input(&snapshot, popup_area, buf),
                OnlineStep::GuestConnecting => {
                    Paragraph::new("正在连接房间...")
                        .style(Style::default().fg(theme.text_dim()))
                        .render(popup_area, buf);
                }
                OnlineStep::GuestReady => render_guest_ready(&snapshot, popup_area, buf),
                OnlineStep::Error => render_error(&snapshot, popup_area, buf),
            }
        }),
    };

    frame.render_widget(popup, area);
}

pub fn popup_rect(frame_area: Rect) -> Rect {
    use ratatui::layout::Constraint;
    let w = Constraint::Percentage(45);
    let h = Constraint::Length(20);
    frame_area.centered(w, h)
}

fn step_keybinds(state: &OnlinePopupSnapshot) -> Line<'static> {
    use crate::tui::widgets::popups::keybind_line;
    match state.step {
        OnlineStep::Menu => keybind_line(&[("j/k", " 导航"), ("Enter", " 选择"), ("Esc", " 关闭")]),
        OnlineStep::HostReady | OnlineStep::GuestReady => keybind_line(&[("d", " 退出房间"), ("Esc", " 隐藏")]),
        OnlineStep::GuestInput => keybind_line(&[("Enter", " 连接"), ("Esc", " 返回")]),
        OnlineStep::Error => keybind_line(&[("Esc", " 关闭")]),
        _ => keybind_line(&[]),
    }
}

fn render_menu(state: &OnlinePopupSnapshot, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();
    let items = [
        ("创建房间 (主机)", "作为房主，生成邀请码"),
        ("加入房间 (访客)", "输入邀请码加入"),
        ("取消", "返回"),
    ];

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (title, desc))| {
            let is_selected = i == state.menu_selection;
            let marker = if is_selected { "▶ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme.accent())),
                Span::styled(*title, Style::default().fg(theme.text())),
                Span::raw(" "),
                Span::styled(*desc, Style::default().fg(theme.text_dim())),
            ]))
        })
        .collect();

    let list = List::new(list_items)
        .highlight_style(
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");
    let mut list_state = ListState::default().with_selected(Some(state.menu_selection));
    StatefulWidget::render(list, area, buf, &mut list_state);
}

fn render_host_ready(state: &OnlinePopupSnapshot, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("房间已创建!", Style::default().fg(theme.success())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("邀请码: ", Style::default().fg(theme.text_dim())),
            Span::styled(
                &state.invite_code,
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    if !state.profiles.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("成员列表", Style::default().fg(theme.text_dim())),
        ]));
        lines.push(Line::from(""));
        for profile in &state.profiles {
            let kind_label = match profile.kind {
                crate::online::profile::ProfileKind::HOST => "房主",
                crate::online::profile::ProfileKind::LOCAL => "自己",
                crate::online::profile::ProfileKind::GUEST => "访客",
            };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&profile.name, Style::default().fg(theme.text())),
                Span::raw(" "),
                Span::styled(kind_label, Style::default().fg(theme.text_dim())),
            ]));
        }
    }

    Paragraph::new(lines)
        .style(Style::default().fg(theme.text()))
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn render_guest_input(state: &OnlinePopupSnapshot, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();
    let value = &state.invite_code_value;

    let lines = vec![
        Line::from(vec![
            Span::styled("请输入邀请码", Style::default().fg(theme.text_dim())),
        ]),
        Line::from(""),
        Line::from(if value.is_empty() {
            vec![
                Span::styled("例如: U/GH71-SWWU-0Z9P-N4VQ", Style::default().fg(theme.text_dim())),
                Span::styled(
                    "\u{2588}",
                    Style::default()
                        .fg(theme.text_dim())
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]
        } else {
            vec![
                Span::styled(value, Style::default().fg(theme.text())),
                Span::styled(
                    "\u{2588}",
                    Style::default()
                        .fg(theme.text_dim())
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]
        }),
    ];

    Paragraph::new(lines)
        .style(Style::default().fg(theme.text()))
        .render(area, buf);
}

fn render_guest_ready(state: &OnlinePopupSnapshot, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("已加入房间!", Style::default().fg(theme.success())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("P2P 通道已建立", Style::default().fg(theme.text())),
        ]),
        Line::from(vec![
            Span::styled("可以启动游戏了", Style::default().fg(theme.text_dim())),
        ]),
        Line::from(""),
    ];

    if !state.profiles.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("成员列表", Style::default().fg(theme.text_dim())),
        ]));
        lines.push(Line::from(""));
        for profile in &state.profiles {
            let kind_label = match profile.kind {
                crate::online::profile::ProfileKind::HOST => "房主",
                crate::online::profile::ProfileKind::LOCAL => "自己",
                crate::online::profile::ProfileKind::GUEST => "访客",
            };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&profile.name, Style::default().fg(theme.text())),
                Span::raw(" "),
                Span::styled(kind_label, Style::default().fg(theme.text_dim())),
            ]));
        }
    }

    Paragraph::new(lines)
        .style(Style::default().fg(theme.text()))
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn render_error(state: &OnlinePopupSnapshot, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();

    Paragraph::new(vec![
        Line::from(vec![
            Span::styled("错误", Style::default().fg(theme.error()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(&state.error_message, Style::default().fg(theme.error())),
        ]),
    ])
    .style(Style::default().fg(theme.text()))
    .wrap(Wrap { trim: false })
    .render(area, buf);
}