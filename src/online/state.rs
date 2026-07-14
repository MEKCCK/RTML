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

use serde::Deserialize;

use super::profile::TerracottaProfile;

#[derive(Debug, Clone)]
pub enum TerracottaState {
    Bootstrap,
    Uninitialized { has_legacy: bool },
    Preparing,
    Launching,
    Unknown { port: u16 },
    Waiting,
    HostScanning,
    HostStarting,
    HostOK {
        port: u16,
        code: String,
        profiles: Vec<TerracottaProfile>,
    },
    GuestConnecting,
    GuestStarting,
    GuestOK {
        port: u16,
        profiles: Vec<TerracottaProfile>,
    },
    Exception(StateException),
    Fatal(FatalType),
}

#[derive(Debug, Clone)]
pub enum StateException {
    PingHostFail,
    PingHostRst,
    GuestEtCrash,
    HostEtCrash,
    PingServerRst,
    ScaffoldingInvalidResponse,
}

#[derive(Debug, Clone)]
pub enum FatalType {
    Os,
    Network,
    Install,
    Unknown,
}

impl TerracottaState {
    pub fn from_raw_state(state_str: &str) -> Result<Self, String> {
        let raw: RawReadyState =
            serde_json::from_str(state_str).map_err(|e| format!("JSON 解析失败: {e}\n原始响应: {state_str}"))?;

        match raw.state.as_str() {
            "waiting" => Ok(TerracottaState::Waiting),
            "host-scanning" => Ok(TerracottaState::HostScanning),
            "host-starting" => Ok(TerracottaState::HostStarting),
            "host-ok" => Ok(TerracottaState::HostOK {
                port: 0,
                code: raw.room.unwrap_or_default(),
                profiles: raw.profiles.unwrap_or_default(),
            }),
            "guest-connecting" => Ok(TerracottaState::GuestConnecting),
            "guest-starting" => Ok(TerracottaState::GuestStarting),
            "guest-ok" => Ok(TerracottaState::GuestOK {
                port: 0,
                profiles: raw.profiles.unwrap_or_default(),
            }),
            "exception" => {
                let ex = match raw.error_type {
                    Some(0) => StateException::PingHostFail,
                    Some(1) => StateException::PingHostRst,
                    Some(2) => StateException::GuestEtCrash,
                    Some(3) => StateException::HostEtCrash,
                    Some(4) => StateException::PingServerRst,
                    Some(5) => StateException::ScaffoldingInvalidResponse,
                    _ => return Err(format!("unknown exception type: {:?}", raw.error_type)),
                };
                Ok(TerracottaState::Exception(ex))
            }
            other => Err(format!("unknown state: {other}")),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TerracottaState::HostOK { .. }
                | TerracottaState::GuestOK { .. }
                | TerracottaState::Fatal(_)
                | TerracottaState::Exception(_)
        )
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            TerracottaState::HostScanning
                | TerracottaState::HostStarting
                | TerracottaState::GuestConnecting
                | TerracottaState::GuestStarting
                | TerracottaState::Waiting
        )
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawReadyState {
    #[serde(rename = "state")]
    state: String,
    #[serde(default)]
    index: Option<i32>,
    #[serde(default)]
    room: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default, rename = "type")]
    error_type: Option<i32>,
    #[serde(default)]
    profiles: Option<Vec<TerracottaProfile>>,
    #[serde(default)]
    difficulty: Option<String>,
    #[serde(default, rename = "profile_index")]
    profile_index: Option<i32>,
}
