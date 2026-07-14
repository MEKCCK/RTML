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


// in-memory ring buffer for stdout/stderr from running mc instances.
// capped at 2000 lines per instance so it doesn't eat all the RAM
// if someone leaves a server running for a week. you're welcome.

use std::collections::{HashMap, VecDeque};
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

use crate::instance::launch::parser::{LogLevel, LogStream, ParsedLogEvent};

const MAX_LINES: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveLogLine {
    pub level: LogLevel,
    pub stream: LogStream,
    pub text: String,
}

type LogsMap = Arc<Mutex<HashMap<String, VecDeque<LiveLogLine>>>>;
pub static LOGS: LazyLock<LogsMap> = LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

pub fn push(name: &str, line: impl Into<String>) {
    push_line(name, LogLevel::Info, LogStream::Stdout, line);
}

pub fn push_event(name: &str, event: ParsedLogEvent) {
    for line in event.lines {
        push_line(name, event.level, event.primary_stream, line);
    }
}

pub fn push_line(name: &str, level: LogLevel, stream: LogStream, line: impl Into<String>) {
    if let Ok(mut logs) = LOGS.lock() {
        let buf = logs.entry(name.to_string()).or_insert_with(VecDeque::new);
        buf.push_back(LiveLogLine {
            level,
            stream,
            text: line.into(),
        });
        while buf.len() > MAX_LINES {
            buf.pop_front();
        }
    }
}

pub fn get_entries(name: &str) -> Vec<LiveLogLine> {
    LOGS.lock()
        .ok()
        .and_then(|logs| logs.get(name).map(|buf| buf.iter().cloned().collect()))
        .unwrap_or_default()
}

pub fn get_all(name: &str) -> Vec<String> {
    LOGS.lock()
        .ok()
        .and_then(|logs| {
            logs.get(name)
                .map(|buf| buf.iter().map(|line| line.text.clone()).collect())
        })
        .unwrap_or_default()
}

pub fn clear(name: &str) {
    if let Ok(mut logs) = LOGS.lock() {
        logs.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_get_all() {
        let name = "test_push_get";
        push(name, "line1");
        push(name, "line2");
        let lines = get_all(name);
        assert!(lines.contains(&"line1".to_string()));
        assert!(lines.contains(&"line2".to_string()));
    }

    #[test]
    fn get_all_missing_instance_returns_empty() {
        let lines = get_all("nonexistent_instance_xyz");
        assert!(lines.is_empty());
    }

    #[test]
    fn clear_removes_instance() {
        let name = "test_clear";
        push(name, "data");
        assert!(!get_all(name).is_empty());
        clear(name);
        assert!(get_all(name).is_empty());
    }

    #[test]
    fn clear_nonexistent_is_noop() {
        clear("never_existed_xyz");
    }

    #[test]
    fn buffer_respects_max_lines() {
        let name = "test_max_lines";
        for i in 0..(MAX_LINES + 100) {
            push(name, format!("line-{i}"));
        }
        let lines = get_all(name);
        assert_eq!(lines.len(), MAX_LINES);
        assert!(
            lines
                .last()
                .unwrap()
                .contains(&format!("{}", MAX_LINES + 99))
        );
    }
}
