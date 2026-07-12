// 模组下载浏览器：搜索、浏览、下载模组
// 自动使用当前实例的版本号和加载器进行筛选

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::sync::mpsc;

use crate::config::theme::{BORDER_STYLE, THEME};
use crate::instance::models::ModLoader;
use crate::net::modrinth::{self, ModSearchResult, VersionInfo};
use crate::tui::app::FocusedArea;

use super::styled_title;

// Modrinth 常用分类
const CATEGORIES: &[(&str, &str)] = &[
    ("", "全部"),
    ("technology", "科技"),
    ("adventure", "冒险"),
    ("library", "库"),
    ("utility", "工具"),
    ("decoration", "装饰"),
    ("magic", "魔法"),
    ("storage", "存储"),
    ("transportation", "运输"),
    ("food", "食物"),
    ("combat", "战斗"),
    ("optimization", "优化"),
    ("cosmetic", "装饰"),
    ("equipment", "装备"),
    ("misc", "其他"),
];

pub struct ModDownloadState {
    pub search_query: String,
    pub search_active: bool,
    pub results: Vec<ModSearchResult>,
    pub selected: usize,
    pub versions: Vec<VersionInfo>,
    pub selected_version: usize,
    pub show_versions: bool,
    pub loading: bool,
    pub game_version: String,
    pub loader_filter: String,
    pub category_filter: String,
    pub category_index: usize,
    pub page: u32,
    pub total_hits: i64,
    pub search_rx: Option<mpsc::Receiver<Result<Vec<ModSearchResult>, String>>>,
    pub versions_rx: Option<mpsc::Receiver<Result<Vec<VersionInfo>, String>>>,
    pub download_rx: Option<mpsc::Receiver<Result<String, String>>>,
    pub downloading: bool,
    pub download_message: Option<String>,
    pub mods_dir: Option<std::path::PathBuf>,
}

impl Default for ModDownloadState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            search_active: false,
            results: Vec::new(),
            selected: 0,
            versions: Vec::new(),
            selected_version: 0,
            show_versions: false,
            loading: false,
            game_version: String::new(),
            loader_filter: String::new(),
            category_filter: String::new(),
            category_index: 0,
            page: 0,
            total_hits: 0,
            search_rx: None,
            versions_rx: None,
            download_rx: None,
            downloading: false,
            download_message: None,
            mods_dir: None,
        }
    }
}

impl ModDownloadState {
    /// 从实例配置创建，自动获取版本和加载器，并加载热门模组
    pub fn from_instance(
        game_version: &str,
        loader: ModLoader,
        mods_dir: std::path::PathBuf,
    ) -> Self {
        let loader_filter = match loader {
            ModLoader::Fabric => "fabric".to_string(),
            ModLoader::Forge => "forge".to_string(),
            ModLoader::NeoForge => "neoforge".to_string(),
            ModLoader::Quilt => "quilt".to_string(),
            ModLoader::Vanilla => String::new(),
        };

        let mut state = Self {
            game_version: game_version.to_string(),
            loader_filter,
            mods_dir: Some(mods_dir),
            search_query: String::new(), // 空搜索词 = 按下载量排序的热门模组
            ..Default::default()
        };

        // 自动加载热门模组
        state.start_search();
        state
    }

    /// 检查异步搜索结果
    pub fn check_search_results(&mut self) {
        if let Some(rx) = &self.search_rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.search_rx = None;
                    self.loading = false;
                    match result {
                        Ok(results) => {
                            tracing::info!("Modrinth search returned {} results", results.len());
                            self.results = results;
                            self.selected = 0;
                        }
                        Err(e) => {
                            tracing::error!("Modrinth search failed: {}", e);
                            self.download_message = Some(format!("搜索失败: {}", e));
                        }
                    }
                    crate::tui::request_redraw();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // 还没收到结果，继续等待
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // 发送端已关闭，可能是线程 panic
                    tracing::error!("Search channel disconnected");
                    self.search_rx = None;
                    self.loading = false;
                    self.download_message = Some("搜索通道断开".to_string());
                    crate::tui::request_redraw();
                }
            }
        }
        self.check_download_results();
    }

    /// 检查异步版本获取结果
    pub fn check_version_results(&mut self) {
        if let Some(rx) = &self.versions_rx {
            if let Ok(result) = rx.try_recv() {
                self.versions_rx = None;
                match result {
                    Ok(versions) => {
                        self.versions = versions;
                        self.selected_version = 0;
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch mod versions: {}", e);
                    }
                }
            }
        }
    }

    /// 启动异步搜索（自动使用当前实例的版本和加载器）
    pub fn start_search(&mut self) {
        self.loading = true;
        let query = self.search_query.clone();
        let game_version = self.game_version.clone();
        let loader = self.loader_filter.clone();
        let category = self.category_filter.clone();
        let page = self.page;

        let (tx, rx) = mpsc::channel();
        self.search_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = crate::net::HttpClient::new();

                let mut params = modrinth::SearchParams::new()
                    .limit(20)
                    .offset(page * 20);

                // 只有非空搜索词才传递 query
                if !query.is_empty() && query != "*" {
                    params = params.query(&query);
                }

                // 添加游戏版本筛选
                if !game_version.is_empty() {
                    params = params.game_version(&game_version);
                }

                // 添加加载器筛选
                if !loader.is_empty() {
                    params = params.loader(&loader);
                }

                // 添加分类筛选
                if !category.is_empty() {
                    params = params.category(&category);
                }

                tracing::info!("=== SEARCH START ===");
                tracing::info!("Query: '{}', Version: '{}', Loader: '{}', Category: '{}'",
                    params.query.as_deref().unwrap_or("(empty)"),
                    game_version, loader, category);
                tracing::info!("Facets: {:?}", params.facets);

                match modrinth::search_mods(&client, &params).await {
                    Ok(response) => {
                        tracing::info!("=== SEARCH OK: {} hits ===", response.hits.len());
                        for (i, hit) in response.hits.iter().take(3).enumerate() {
                            tracing::info!("  Hit {}: {} ({})", i + 1, hit.title, hit.project_id);
                        }
                        let _ = tx.send(Ok(response.hits));
                    }
                    Err(e) => {
                        tracing::error!("=== SEARCH FAILED: {} ===", e);
                        let _ = tx.send(Err(e.to_string()));
                    }
                }
            });
        });
    }

    /// 切换分类
    pub fn next_category(&mut self) {
        self.category_index = (self.category_index + 1) % CATEGORIES.len();
        self.category_filter = CATEGORIES[self.category_index].0.to_string();
        self.page = 0;
        self.start_search();
    }

    pub fn prev_category(&mut self) {
        self.category_index = if self.category_index == 0 {
            CATEGORIES.len() - 1
        } else {
            self.category_index - 1
        };
        self.category_filter = CATEGORIES[self.category_index].0.to_string();
        self.page = 0;
        self.start_search();
    }

    /// 启动异步获取版本
    pub fn start_fetch_versions(&mut self, project_id: &str) {
        self.loading = true;
        let project_id = project_id.to_string();
        let game_version = self.game_version.clone();
        let loader = self.loader_filter.clone();

        let (tx, rx) = mpsc::channel();
        self.versions_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = crate::net::HttpClient::new();
                let result = modrinth::fetch_versions(
                    &client,
                    &project_id,
                    Some(&game_version),
                    Some(&loader),
                )
                .await
                .map_err(|e| e.to_string());
                let _ = tx.send(result);
            });
        });
    }

    /// 启动异步下载模组
    pub fn start_download(&mut self, file_url: &str, filename: &str, mods_dir: &std::path::Path) {
        if self.downloading {
            return;
        }

        self.downloading = true;
        self.download_message = Some(format!("正在下载 {}...", filename));

        let file_url = file_url.to_string();
        let filename = filename.to_string();
        let dest = mods_dir.join(&filename);

        let (tx, rx) = mpsc::channel();
        self.download_rx = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = crate::net::HttpClient::new();
                let result = modrinth::download_mod_file(&client, &file_url, &dest, None)
                    .await
                    .map(|_| filename)
                    .map_err(|e| e.to_string());
                let _ = tx.send(result);
            });
        });
    }

    /// 检查异步下载结果
    pub fn check_download_results(&mut self) {
        if let Some(rx) = &self.download_rx {
            if let Ok(result) = rx.try_recv() {
                self.download_rx = None;
                self.downloading = false;
                match result {
                    Ok(filename) => {
                        self.download_message = Some(format!("已下载: {}", filename));
                        tracing::info!("模组下载完成: {}", filename);
                    }
                    Err(e) => {
                        self.download_message = Some(format!("下载失败: {}", e));
                        tracing::error!("模组下载失败: {}", e);
                    }
                }
            }
        }
    }

    /// 获取当前分类名称
    pub fn current_category_name(&self) -> &str {
        CATEGORIES[self.category_index].1
    }
}

pub fn handle_key(key_event: &KeyEvent, state: &mut ModDownloadState) -> bool {
    // 先检查异步结果
    state.check_search_results();
    state.check_version_results();

    if state.show_versions {
        // 版本选择模式
        match key_event.code {
            KeyCode::Esc => {
                state.show_versions = false;
                state.versions.clear();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_version > 0 {
                    state.selected_version -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_version + 1 < state.versions.len() {
                    state.selected_version += 1;
                }
                true
            }
            KeyCode::Enter => {
                // 下载选中的版本
                let download_info = state.versions.get(state.selected_version)
                    .and_then(|v| v.files.iter().find(|f| f.primary).or_else(|| v.files.first()))
                    .map(|f| (f.url.clone(), f.filename.clone()));

                if let Some((url, filename)) = download_info {
                    let mods_dir = state.mods_dir.clone()
                        .unwrap_or_else(|| {
                            std::env::current_dir()
                                .unwrap_or_default()
                                .join("mods")
                        });
                    let _ = std::fs::create_dir_all(&mods_dir);
                    state.start_download(&url, &filename, &mods_dir);
                }
                true
            }
            _ => true,
        }
    } else if state.search_active {
        // 搜索输入模式
        match key_event.code {
            KeyCode::Enter => {
                state.search_active = false;
                state.page = 0;
                state.start_search();
                true
            }
            KeyCode::Esc => {
                state.search_active = false;
                true
            }
            KeyCode::Backspace => {
                state.search_query.pop();
                true
            }
            KeyCode::Char(c) => {
                state.search_query.push(c);
                true
            }
            _ => true,
        }
    } else {
        // 结果列表模式
        match key_event.code {
            KeyCode::Char('/') => {
                state.search_active = true;
                true
            }
            // 分类切换
            KeyCode::Left | KeyCode::Char('h') => {
                state.prev_category();
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                state.next_category();
                true
            }
            // 翻页
            KeyCode::PageUp => {
                if state.page > 0 {
                    state.page -= 1;
                    state.start_search();
                }
                true
            }
            KeyCode::PageDown => {
                let max_page = (state.total_hits as u32 / 20).saturating_sub(1);
                if state.page < max_page {
                    state.page += 1;
                    state.start_search();
                }
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected > 0 {
                    state.selected -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected + 1 < state.results.len() {
                    state.selected += 1;
                }
                true
            }
            KeyCode::Enter => {
                // 打开版本选择
                let project_id = state.results.get(state.selected)
                    .map(|m| m.project_id.clone());
                if let Some(pid) = project_id {
                    state.start_fetch_versions(&pid);
                    state.show_versions = true;
                }
                true
            }
            KeyCode::Esc => false, // 退出模组下载模式
            _ => false,
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, focused: FocusedArea, state: &mut ModDownloadState) {
    let theme = THEME.as_ref();
    let color = if focused == FocusedArea::ModDownload {
        theme.accent()
    } else {
        theme.border()
    };

    // 检查下载状态
    state.check_download_results();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 搜索栏
            Constraint::Length(3), // 筛选栏（版本 + 加载器 + 分类）
            Constraint::Min(5),   // 结果列表
            Constraint::Length(if state.downloading || state.download_message.is_some() { 2 } else { 0 }),
        ])
        .split(area);

    // 搜索栏
    let search_text = if state.search_active {
        format!("搜索: {}█", state.search_query)
    } else if state.search_query.is_empty() {
        "按 / 搜索模组".to_string()
    } else {
        format!("搜索: {}", state.search_query)
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BORDER_STYLE.to_border_type())
        .border_style(Style::default().fg(color))
        .title(styled_title("模组搜索", false));

    let search_paragraph = Paragraph::new(Line::from(Span::styled(
        &search_text,
        Style::default().fg(theme.text()),
    )))
    .block(search_block);
    frame.render_widget(search_paragraph, chunks[0]);

    // 筛选栏：显示当前实例版本、加载器、分类
    let filter_text = format!(
        "版本: {} | 加载器: {} | 分类: {} (←→ 切换)",
        if state.game_version.is_empty() { "未选择" } else { &state.game_version },
        if state.loader_filter.is_empty() { "未选择" } else { &state.loader_filter },
        state.current_category_name()
    );

    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BORDER_STYLE.to_border_type())
        .border_style(Style::default().fg(theme.border()))
        .title(styled_title("筛选条件", false));

    let filter_paragraph = Paragraph::new(Line::from(Span::styled(
        &filter_text,
        Style::default().fg(theme.text_dim()),
    )))
    .block(filter_block);
    frame.render_widget(filter_paragraph, chunks[1]);

    // 结果列表
    let result_title = format!(
        "搜索结果 ({} 个模组, 第 {}/{} 页)",
        state.total_hits,
        state.page + 1,
        (state.total_hits as u32 / 20).max(1)
    );
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BORDER_STYLE.to_border_type())
        .border_style(Style::default().fg(color))
        .title(styled_title(&result_title, false));

    if state.loading {
        let loading_text = if state.show_versions {
            "加载版本列表..."
        } else {
            "搜索中..."
        };
        let loading_paragraph = Paragraph::new(Line::from(Span::styled(
            loading_text,
            Style::default().fg(theme.text_dim()),
        )))
        .block(list_block);
        frame.render_widget(loading_paragraph, chunks[2]);
    } else if state.show_versions {
        let versions = state.versions.clone();
        let selected_version = state.selected_version;
        render_versions(frame, chunks[2], &versions, selected_version, list_block);
    } else if state.results.is_empty() {
        let empty_text = if state.search_query.is_empty() {
            "输入关键词搜索 Modrinth 模组"
        } else {
            "没有找到结果"
        };
        let empty_paragraph = Paragraph::new(Line::from(Span::styled(
            empty_text,
            Style::default().fg(theme.text_dim()),
        )))
        .block(list_block);
        frame.render_widget(empty_paragraph, chunks[2]);
    } else {
        let results = state.results.clone();
        let selected = state.selected;

        let items: Vec<ListItem> = results
            .iter()
            .enumerate()
            .map(|(i, mod_info)| {
                let style = if i == selected {
                    Style::default()
                        .fg(theme.accent())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text())
                };

                let desc = if mod_info.description.len() > 50 {
                    format!("{}...", &mod_info.description[..50])
                } else {
                    mod_info.description.clone()
                };

                ListItem::new(vec![
                    Line::from(Span::styled(&mod_info.title, style)),
                    Line::from(Span::styled(
                        format!("  {} | {} 下载", mod_info.author, mod_info.downloads),
                        Style::default().fg(theme.text_dim()),
                    )),
                    Line::from(Span::styled(
                        format!("  {}", desc),
                        Style::default().fg(theme.text_dim()),
                    )),
                ])
            })
            .collect();

        let list = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            );

        let mut list_state = ListState::default();
        list_state.select(Some(state.selected));
        frame.render_stateful_widget(list, chunks[2], &mut list_state);
    }

    // 下载状态栏
    if chunks.len() > 3 && chunks[3].height > 0 {
        if state.downloading {
            let loading_text = state.download_message.as_deref().unwrap_or("下载中...");
            let loading_paragraph = Paragraph::new(Line::from(Span::styled(
                loading_text,
                Style::default().fg(theme.accent()),
            )));
            frame.render_widget(loading_paragraph, chunks[3]);
        } else if let Some(msg) = &state.download_message {
            let msg_style = if msg.starts_with("下载失败") {
                Style::default().fg(theme.error())
            } else {
                Style::default().fg(theme.success())
            };
            let msg_paragraph = Paragraph::new(Line::from(Span::styled(
                msg.as_str(),
                msg_style,
            )));
            frame.render_widget(msg_paragraph, chunks[3]);
        }
    }
}

fn render_versions(
    frame: &mut Frame,
    area: Rect,
    versions: &[VersionInfo],
    selected_version: usize,
    list_block: Block<'_>,
) {
    let theme = THEME.as_ref();

    if versions.is_empty() {
        let empty_paragraph = Paragraph::new(Line::from(Span::styled(
            "没有可用版本",
            Style::default().fg(theme.text_dim()),
        )))
        .block(list_block);
        frame.render_widget(empty_paragraph, area);
        return;
    }

    let items: Vec<ListItem> = versions
        .iter()
        .enumerate()
        .map(|(i, version)| {
            let style = if i == selected_version {
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text())
            };

            let type_label = match version.status.as_str() {
                "release" => "正式版",
                "beta" => "测试版",
                "alpha" => "开发版",
                _ => "未知",
            };

            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("{} ({})", version.name, type_label),
                    style,
                )),
                Line::from(Span::styled(
                    format!("  {} | {} 文件", version.game_versions.join(", "), version.files.len()),
                    Style::default().fg(theme.text_dim()),
                )),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .fg(theme.accent())
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(selected_version));
    frame.render_stateful_widget(list, area, &mut list_state);
}
