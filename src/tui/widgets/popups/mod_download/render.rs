use super::state::{DOWNLOAD_STATE, DownloadState, DownloadStep};
use crate::config::theme::THEME;
use crate::tui::widgets::popups::base::PopupFrame;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};

pub fn render(frame: &mut Frame, area: Rect, instance_version: Option<&str>) {
    let snapshot = match DOWNLOAD_STATE.lock() {
        Ok(state) => state.clone(),
        Err(_) => DownloadState::default(),
    };

    let keybinds = step_keybinds(&snapshot);
    let theme = THEME.as_ref();

    let popup = PopupFrame {
        title: Line::from(" 下载 Mod "),
        border_color: theme.text_dim(),
        bg: Some(theme.surface()),
        keybinds: Some(keybinds),
        search_line: None,
        content: Box::new(move |popup_area, buf| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(popup_area);

            match snapshot.step {
                DownloadStep::Search => render_search_step(&snapshot, chunks[0], buf),
                DownloadStep::Results => render_results_step(&snapshot, chunks[0], buf),
                DownloadStep::Versions => render_versions_step(&snapshot, chunks[0], buf, instance_version),
                DownloadStep::Confirm => render_confirm_step(&snapshot, chunks[0], buf),
            }
        }),
    };

    frame.render_widget(popup, area);
}

pub fn popup_rect(frame_area: Rect) -> Rect {
    use ratatui::layout::Constraint;
    let w = Constraint::Percentage(55);
    let h = (frame_area.height * 2 / 3)
        .max(12)
        .min(frame_area.height.saturating_sub(4));
    frame_area.centered(w, Constraint::Length(h))
}

fn step_keybinds(state: &DownloadState) -> Line<'static> {
    use crate::tui::widgets::popups::keybind_line;
    match state.step {
        DownloadStep::Search => keybind_line(&[("Enter", " 搜索"), ("Esc", " 关闭")]),
        DownloadStep::Results => keybind_line(&[("h", " 返回"), ("Enter", " 选择"), ("Esc", " 关闭")]),
        DownloadStep::Versions => keybind_line(&[("h", " 返回"), ("Enter", " 选择"), ("Esc", " 关闭")]),
        DownloadStep::Confirm => keybind_line(&[("h", " 返回"), ("Enter", " 安装"), ("Esc", " 关闭")]),
    }
}

fn render_search_step(state: &DownloadState, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();
    let text = if state.search_query.is_empty() {
        Line::from(vec![
            Span::styled("输入搜索关键词...", Style::default().fg(theme.text_dim())),
            Span::styled("\u{2588}", Style::default().fg(theme.text_dim()).add_modifier(Modifier::SLOW_BLINK)),
        ])
    } else {
        Line::from(vec![
            Span::styled(&state.search_query, Style::default().fg(theme.text())),
            Span::styled("\u{2588}", Style::default().fg(theme.text_dim()).add_modifier(Modifier::SLOW_BLINK)),
        ])
    };
    Paragraph::new(text).render(area, buf);
}

fn render_results_step(state: &DownloadState, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();

    if state.loading {
        Paragraph::new("搜索中...")
            .style(Style::default().fg(theme.text_dim()))
            .render(area, buf);
        return;
    }

    if let Some(ref err) = state.error {
        Paragraph::new(err.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.error()))
            .render(area, buf);
        return;
    }

    if state.search_results.is_empty() {
        Paragraph::new("未找到结果")
            .style(Style::default().fg(theme.text_dim()))
            .render(area, buf);
        return;
    }

    let items: Vec<ListItem> = state
        .search_results
        .iter()
        .map(|m| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", m.title),
                    Style::default().fg(theme.text()),
                ),
                Span::styled(
                    format!("({})", m.author),
                    Style::default().fg(theme.text_dim()),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default().with_selected(Some(state.selected_result));
    StatefulWidget::render(list, area, buf, &mut list_state);
}

fn render_versions_step(
    state: &DownloadState,
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    _instance_version: Option<&str>,
) {
    let theme = THEME.as_ref();

    if state.loading {
        Paragraph::new("加载版本列表...")
            .style(Style::default().fg(theme.text_dim()))
            .render(area, buf);
        return;
    }

    if let Some(ref err) = state.error {
        Paragraph::new(err.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.error()))
            .render(area, buf);
        return;
    }

    if state.versions.is_empty() {
        Paragraph::new("该 Mod 没有可用版本")
            .style(Style::default().fg(theme.text_dim()))
            .render(area, buf);
        return;
    }

    let items: Vec<ListItem> = state
        .versions
        .iter()
        .map(|v| {
            let game_ver = v.game_versions.first().map(|s| s.as_str()).unwrap_or("");
            let loaders: String = v.loaders.iter().map(|l| format!("{l} ")).collect();
            ListItem::new(Line::from(vec![
                Span::styled(&v.name, Style::default().fg(theme.text())),
                Span::styled(format!(" [{game_ver}]"), Style::default().fg(theme.text_dim())),
                Span::styled(format!(" {loaders}"), Style::default().fg(theme.text_dim())),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default().with_selected(Some(state.selected_version));
    StatefulWidget::render(list, area, buf, &mut list_state);
}

fn render_confirm_step(state: &DownloadState, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let theme = THEME.as_ref();

    let mod_title = state
        .search_results
        .get(state.selected_result)
        .map(|m| m.title.as_str())
        .unwrap_or("<unknown>");
    let version = state.versions.get(state.selected_version);

    let label_style = Style::default().fg(theme.text_dim());

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Mod: ", label_style),
            Span::raw(mod_title),
        ]),
    ];

    if let Some(v) = version {
        lines.push(Line::from(vec![
            Span::styled("版本: ", label_style),
            Span::raw(&v.name),
        ]));
        if let Some(file) = v.files.first() {
            lines.push(Line::from(vec![
                Span::styled("文件: ", label_style),
                Span::raw(&file.filename),
            ]));
        }
    }

    Paragraph::new(lines)
        .style(Style::default().fg(theme.text()))
        .wrap(Wrap { trim: true })
        .render(area, buf);
}
