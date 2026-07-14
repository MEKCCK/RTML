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