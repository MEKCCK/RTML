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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};

use super::metadata::TerracottaMetadata;
use super::state::TerracottaState;

static ONLINE_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_active() -> bool {
    ONLINE_ACTIVE.load(Ordering::Acquire)
}

#[derive(Debug)]
pub struct TerracottaManager {
    binary_path: PathBuf,
    process: Option<Child>,
    port: Option<u16>,
    state: TerracottaState,
}

impl TerracottaManager {
    pub async fn new() -> Result<Self, String> {
        if !TerracottaMetadata::is_supported() {
            return Err("terracotta is not supported on this platform".to_string());
        }

        let binary_path = TerracottaMetadata::ensure_binary_downloaded().await?;

        Ok(Self {
            binary_path,
            process: None,
            port: None,
            state: TerracottaState::Bootstrap,
        })
    }

    pub fn state(&self) -> &TerracottaState {
        &self.state
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Wait for terracotta to start and report its HTTP port.
    /// Monitors child process liveness and captures stderr on failure.
    async fn wait_for_port(
        child: &mut Child,
        port_file: &Path,
        timeout: Duration,
        binary_path: &Path,
    ) -> Result<u16, String> {
        let start = std::time::Instant::now();
        let mut stderr_buf = Vec::new();
        let mut stdout_buf = Vec::new();

        loop {
            if start.elapsed() > timeout {
                // Try to read any output the process produced
                if let Some(ref mut stderr) = child.stderr {
                    use tokio::io::AsyncReadExt;
                    let _ = stderr.read_to_end(&mut stderr_buf).await;
                }
                if let Some(ref mut stdout) = child.stdout {
                    use tokio::io::AsyncReadExt;
                    let _ = stdout.read_to_end(&mut stdout_buf).await;
                }
                let err_text = String::from_utf8_lossy(&stderr_buf);
                let out_text = String::from_utf8_lossy(&stdout_buf);
                return Err(format!(
                    "启动 Terracotta 超时 (30s)\n路径: {}\nstderr: {}\nstdout: {}",
                    binary_path.display(),
                    if err_text.is_empty() { "(无)" } else { &err_text },
                    if out_text.is_empty() { "(无)" } else { &out_text },
                ));
            }

            // Check if process is still alive
            match child.try_wait() {
                Ok(Some(status)) => {
                    if let Some(ref mut stderr) = child.stderr {
                        use tokio::io::AsyncReadExt;
                        let _ = stderr.read_to_end(&mut stderr_buf).await;
                    }
                    if let Some(ref mut stdout) = child.stdout {
                        use tokio::io::AsyncReadExt;
                        let _ = stdout.read_to_end(&mut stdout_buf).await;
                    }
                    let err_text = String::from_utf8_lossy(&stderr_buf);
                    let out_text = String::from_utf8_lossy(&stdout_buf);
                    let msg = format!(
                        "Terracotta 进程退出 (code: {status:?})\n路径: {}\nstderr: {}\nstdout: {}",
                        binary_path.display(),
                        if err_text.is_empty() { "(无)" } else { &err_text },
                        if out_text.is_empty() { "(无)" } else { &out_text },
                    );
                    return Err(msg);
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(format!("监测进程状态失败: {e}"));
                }
            }

            if port_file.exists() {
                let content = tokio::fs::read_to_string(port_file)
                    .await
                    .map_err(|e| format!("read port file: {e}"))?;

                let json: serde_json::Value =
                    serde_json::from_str(&content).map_err(|e| format!("parse port JSON: {e}"))?;

                if let Some(port) = json.get("port").and_then(|v| v.as_u64()) {
                    return Ok(port as u16);
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Launch the terracotta subprocess and wait for it to be ready.
    pub async fn start(&mut self) -> Result<(), String> {
        if self.process.is_some() {
            return Ok(());
        }

        if !self.binary_path.exists() {
            return Err(format!(
                "Terracotta 二进制文件不存在: {}",
                self.binary_path.display()
            ));
        }

        tracing::info!("Starting terracotta from: {:?}", self.binary_path);

        let tmp_dir = std::env::temp_dir().join(format!(
            "rtml-terracotta-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("create temp dir: {e}"))?;

        let port_file = tmp_dir.join("http");

        let mut child = Command::new(&self.binary_path)
            .arg("--hmcl")
            .arg(&port_file)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn terracotta: {e}"))?;

        let port =
            Self::wait_for_port(&mut child, &port_file, Duration::from_secs(30), &self.binary_path)
                .await?;

        self.process = Some(child);
        self.port = Some(port);
        self.state = TerracottaState::Unknown { port };
        ONLINE_ACTIVE.store(true, Ordering::Release);

        tracing::info!("Terracotta started on port {port}");
        Ok(())
    }

    /// Poll the terracotta HTTP API for current state.
    /// Matches HMCL's `GET /state` endpoint.
    pub async fn poll_state(&mut self) -> Result<TerracottaState, String> {
        let port = self.port.ok_or("not started")?;
        let url = format!("http://127.0.0.1:{port}/state");

        let client = reqwest::Client::new();
        match client.get(&url).timeout(Duration::from_secs(5)).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return Err(format!("HTTP {}", resp.status()));
                }
                let body = resp
                    .text()
                    .await
                    .map_err(|e| format!("read response: {e}"))?;

                tracing::debug!("Terracotta state response: {body}");
                match TerracottaState::from_raw_state(&body) {
                    Ok(new_state) => {
                        self.state = new_state.clone();
                        Ok(new_state)
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse terracotta state: {e}");
                        Err(e)
                    }
                }
            }
            Err(e) => Err(format!("poll state failed: {e}")),
        }
    }

    /// Start scanning as host.
    /// HMCL sends: GET /state/scanning?player=XXX&public_nodes=YYY
    /// The terracotta daemon handles LAN port auto-detection internally.
    pub async fn start_host(&mut self, player_name: &str) -> Result<(), String> {
        let port = self.port.ok_or("not started")?;

        let nodes = fetch_public_nodes().await;
        let mut query = vec![("player", player_name.to_string())];
        for node in &nodes {
            query.push(("public_nodes", node.to_string()));
        }

        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{port}/state/scanning");
        let resp = client
            .get(&url)
            .query(&query)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("start host request: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("start host HTTP {}", resp.status()));
        }

        self.state = TerracottaState::HostScanning;
        Ok(())
    }

    /// Join as guest with an invite code.
    /// HMCL sends: GET /state/guesting?room=CODE&player=XXX&public_nodes=YYY
    pub async fn start_guest(
        &mut self,
        invite_code: &str,
        player_name: &str,
    ) -> Result<(), String> {
        let port = self.port.ok_or("not started")?;

        let nodes = fetch_public_nodes().await;
        let mut query = vec![
            ("room", invite_code.to_string()),
            ("player", player_name.to_string()),
        ];
        for node in &nodes {
            query.push(("public_nodes", node.to_string()));
        }

        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{port}/state/guesting");
        let resp = client
            .get(&url)
            .query(&query)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("start guest request: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("start guest HTTP {}", resp.status()));
        }

        self.state = TerracottaState::GuestConnecting;
        Ok(())
    }

    /// Set terracotta to waiting/idle state.
    /// HMCL sends: GET /state/ide
    pub async fn set_idle(&mut self) -> Result<(), String> {
        let port = self.port.ok_or("not started")?;
        let url = format!("http://127.0.0.1:{port}/state/ide");

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("set idle request: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("set idle HTTP {}", resp.status()));
        }

        self.state = TerracottaState::Waiting;
        Ok(())
    }

    /// Kill the terracotta subprocess and clean up.
    pub fn kill(&mut self) {
        ONLINE_ACTIVE.store(false, Ordering::Release);

        if let Some(mut child) = self.process.take() {
            let _pid = child.id().map(|id| id.to_string());

            // start_kill sends the signal; kill_on_drop(true) on the command
            // ensures the process is reaped when Child is dropped
            let _ = child.start_kill();

            // On Windows, also clean up virtual NIC and process remnants
            #[cfg(windows)]
            {
                if let Some(ref pid_str) = _pid {
                    kill_windows_process_tree(pid_str);
                }
            }

            // Don't block_on here — this may be called from async contexts.
            // kill_on_drop(true) on Command handles cleanup when Child drops.
            drop(child);

            tracing::info!("Terracotta process terminated");
        }

        self.port = None;
        self.state = TerracottaState::Bootstrap;
    }

    /// Scan Minecraft game process for listening ports.
    /// Reads /proc/{pid}/net/tcp (Linux) or uses netstat (Windows).
    pub async fn scan_game_ports(pid: u32) -> Result<Vec<u16>, String> {
        #[cfg(target_os = "linux")]
        {
            scan_proc_net_tcp(pid).await
        }
        #[cfg(target_os = "windows")]
        {
            scan_windows_netstat(pid).await
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = pid;
            Err("unsupported platform".to_string())
        }
    }
}

#[cfg(target_os = "linux")]
async fn scan_proc_net_tcp(pid: u32) -> Result<Vec<u16>, String> {
    use std::io::BufRead;

    let tcp_path = format!("/proc/{pid}/net/tcp");
    let tcp6_path = format!("/proc/{pid}/net/tcp6");

    let mut ports = Vec::new();

    for path in [&tcp_path, &tcp6_path] {
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = std::io::BufReader::new(file);

        for line in reader.lines().flatten().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }
            // st column (index 3) = 0A means TCP_LISTEN
            let st = parts[3];
            if st != "0A" {
                continue;
            }
            // local_address format: HEX_IP:HEX_PORT
            let local_addr = parts[1];
            if let Some(hex_port) = local_addr.split(':').nth(1) {
                if let Ok(port) = u16::from_str_radix(hex_port, 16) {
                    ports.push(port);
                }
            }
        }
    }

    ports.sort();
    ports.dedup();
    Ok(ports)
}

#[cfg(target_os = "windows")]
async fn scan_windows_netstat(pid: u32) -> Result<Vec<u16>, String> {
    use tokio::process::Command;

    let output = Command::new("netstat")
        .args(["-ano"])
        .output()
        .await
        .map_err(|e| format!("netstat failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pid_str = pid.to_string();
    let mut ports = Vec::new();

    for line in stdout.lines() {
        if !line.contains(&pid_str) {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        // TCP line:  TCP    0.0.0.0:25565   0.0.0.0:0   LISTENING      1234
        if parts.len() < 5 {
            continue;
        }
        if parts[0] != "TCP" && parts[0] != "TCP6" {
            continue;
        }
        if parts[3] != "LISTENING" {
            continue;
        }
        let local = parts[1];
        if let Some(port_str) = local.rsplit(':').next() {
            if let Ok(port) = port_str.parse::<u16>() {
                ports.push(port);
            }
        }
    }

    ports.sort();
    ports.dedup();
    Ok(ports)
}

impl Drop for TerracottaManager {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Fetch public relay nodes from HMCL's node list.
/// HMCL fetches from: https://terracotta.glavo.site/nodes
async fn fetch_public_nodes() -> Vec<String> {
    let urls = [
        "https://terracotta.glavo.site/nodes",
        "https://download.mc9y.com/terracotta/nodes.json",
    ];

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok();

    for url in &urls {
        let Some(ref client) = client else {
            continue;
        };

        match client.get(*url).send().await {
            Ok(resp) if resp.status().is_success() => {
                // HMCL returns a list of objects with "url" and "region" fields
                match resp.json::<Vec<serde_json::Value>>().await {
                    Ok(nodes) => {
                        let uris: Vec<String> = nodes
                            .iter()
                            .filter_map(|n| n.get("url")?.as_str().map(String::from))
                            .collect();
                        if !uris.is_empty() {
                            tracing::info!("Fetched {} public nodes", uris.len());
                            return uris;
                        }
                    }
                    Err(e) => {
                        tracing::debug!("parse nodes from {url}: {e}");
                    }
                }
            }
            _ => continue,
        }
    }

    tracing::warn!("Failed to fetch public nodes, using default STUN server");
    vec!["stun:stun.l.google.com:19302".to_string()]
}

/// On Windows, recursively kill a process tree.
/// Uses taskkill /F /T to force-kill all child processes.
#[cfg(windows)]
fn kill_windows_process_tree(pid: &str) {
    use std::process::Command;
    let _ = Command::new("taskkill")
        .args(&["/F", "/T", "/PID", pid])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Parse an HMCL/PCL2 CE invite code into the raw form to send to the
/// terracotta daemon. HMCL passes the code as-is to the daemon via the
/// `room` query parameter. The daemon handles all validation internally.

pub fn parse_invite_code(code: &str) -> Option<String> {
    let code = code.trim().to_uppercase();
    if code.is_empty() {
        return None;
    }
    Some(code)
}
