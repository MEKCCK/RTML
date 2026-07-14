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


// networking layer: http client, file downloads, and shared utilities
// for fetching game assets from mod loaders and mojang.

pub mod download;
pub mod fabric;
pub mod forge;
pub mod mojang;
pub mod neoforge;
pub mod quilt;

pub mod modrinth;

pub use download::queue::{DownloadProgress, DownloadQueue, DownloadTask};
pub use download::source::SourceManager;

use reqwest::Client;
use serde::de::DeserializeOwned;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Server returned error status {status}: {url}")]
    StatusError { status: u16, url: String },
    #[error("Task failed: {0}")]
    TaskFailed(String),
    #[error("SHA1 mismatch: {0}")]
    Sha1Mismatch(String),
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    #[error("Network unreachable")]
    NetworkUnreachable,
    #[error("Invalid config: {0}")]
    InvalidConfig(String),
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}

impl From<tokio::task::JoinError> for NetError {
    fn from(e: tokio::task::JoinError) -> Self {
        NetError::TaskFailed(e.to_string())
    }
}

#[derive(Clone)]
pub struct HttpClient {
    inner: Client,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    pub fn new() -> Self {
        let user_agent = format!("RTML/{} (Minecraft Launcher)", env!("CARGO_PKG_VERSION"));
        let client = Client::builder()
            .user_agent(user_agent.clone())
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to build configured HTTP client, falling back to reqwest default: {}",
                    e
                );
                Client::new()
            });
        tracing::trace!("Created HTTP client with user-agent '{}'", user_agent);
        Self { inner: client }
    }

    pub fn inner(&self) -> &Client {
        &self.inner
    }

    pub fn from_inner(client: Client) -> Self {
        Self { inner: client }
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response, NetError> {
        tracing::trace!("HTTP GET {}", url);
        let response = self.inner.get(url).send().await?;
        if !response.status().is_success() {
            tracing::debug!(
                "HTTP GET {} returned non-success status {}",
                url,
                response.status()
            );
            return Err(NetError::StatusError {
                status: response.status().as_u16(),
                url: url.to_string(),
            });
        }
        tracing::trace!("HTTP GET {} succeeded with {}", url, response.status());
        Ok(response)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, NetError> {
        get_with_retry(self, url, |resp| async move { Ok(resp.json().await?) }).await
    }

    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, NetError> {
        get_with_retry(
            self,
            url,
            |resp| async move { Ok(resp.bytes().await?.to_vec()) },
        )
        .await
    }

    // fetch JSON and also keep the raw bytes. used by install paths that
    // want both the parsed shape (for downloading libraries from it) and
    // the original bytes (to write byte-for-byte to the loader-profiles
    // cache, so any field we don't know about survives).
    pub async fn get_json_with_raw<T: DeserializeOwned>(
        &self,
        url: &str,
        label: &str,
    ) -> Result<(T, Vec<u8>), NetError> {
        tracing::debug!("Fetching {} JSON from {}", label, url);
        let raw = self.get_bytes(url).await?;
        tracing::trace!("Fetched {} byte(s) for {}", raw.len(), label);
        let parsed: T = serde_json::from_slice(&raw)
            .map_err(|e| NetError::Parse(format!("Failed to parse {label}: {e}")))?;
        Ok((parsed, raw))
    }
}

// shared retry envelope around `client.get(url).await? -> decode`. retries
// transient failures (timeouts, connect errors, 5xx) with exponential
// backoff. used by both get_json and get_bytes.
async fn get_with_retry<T, F, Fut>(client: &HttpClient, url: &str, decode: F) -> Result<T, NetError>
where
    F: Fn(reqwest::Response) -> Fut,
    Fut: std::future::Future<Output = Result<T, NetError>>,
{
    for attempt in 0..=MAX_RETRIES {
        match client.get(url).await {
            Ok(resp) => match decode(resp).await {
                Ok(value) => return Ok(value),
                Err(e) if is_retryable(&e) => {
                    if attempt == MAX_RETRIES {
                        return Err(e);
                    }
                    sleep_before_retry("request", url, attempt, &e).await;
                }
                Err(e) => return Err(e),
            },
            Err(e) if is_retryable(&e) => {
                if attempt == MAX_RETRIES {
                    return Err(e);
                }
                sleep_before_retry("request", url, attempt, &e).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("retry loop returns on success or final error")
}

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 500;

async fn sleep_before_retry(kind: &str, url: &str, attempt: u32, err: &NetError) {
    let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
    tracing::warn!(
        "{} failed, retrying after {}ms (attempt {}/{}): {}: {}",
        kind,
        delay,
        attempt + 2,
        MAX_RETRIES + 1,
        url,
        err
    );
    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
}

// streams a file to disk in chunks, calling progress_cb(downloaded, total) along the way.
// total will be 0 if the server doesn't send content-length, so callers
// should handle that gracefully. retries transient failures with exponential backoff.
pub async fn download_file(
    client: &HttpClient,
    url: &str,
    dest: &Path,
    progress_cb: impl Fn(u64, u64),
) -> Result<(), NetError> {
    tracing::debug!("Downloading {} to {}", url, dest.display());

    for attempt in 0..=MAX_RETRIES {
        match download_file_once(client, url, dest, &progress_cb).await {
            Ok(()) => {
                tracing::debug!("Downloaded {} to {}", url, dest.display());
                return Ok(());
            }
            Err(e) if is_retryable(&e) => {
                if attempt == MAX_RETRIES {
                    return Err(e);
                }
                sleep_before_retry("download", url, attempt, &e).await;
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!("retry loop returns on success or final error")
}

// single attempt at downloading a file to disk
// 参考 PCL-N 的下载实现，支持断点续传和原子写入
async fn download_file_once(
    client: &HttpClient,
    url: &str,
    dest: &Path,
    progress_cb: &impl Fn(u64, u64),
) -> Result<(), NetError> {
    use tokio::io::AsyncWriteExt;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 原子写入：先写临时文件，完成后重命名
    let temp_path = dest.with_extension("tmp");

    // 检查是否有已下载的部分（断点续传）
    let start_offset = if temp_path.exists() {
        tokio::fs::metadata(&temp_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    let response = if start_offset > 0 {
        tracing::trace!("Resuming download from offset {} for {}", start_offset, url);
        let resp = client
            .inner()
            .get(url)
            .header("Range", format!("bytes={}-", start_offset))
            .send()
            .await
            .map_err(NetError::Http)?;

        if resp.status().as_u16() == 416 {
            // Range not satisfiable - file is complete
            tracing::trace!("File already complete: {}", dest.display());
            if let Err(e) = tokio::fs::rename(&temp_path, dest).await {
                tracing::warn!("Failed to rename temp file: {}", e);
            }
            return Ok(());
        }
        resp
    } else {
        client.get(url).await?
    };

    let total = start_offset + response.content_length().unwrap_or(0);
    tracing::trace!("Download content length for {}: {} (offset: {})", url, total, start_offset);

    let mut file = if start_offset > 0 {
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&temp_path)
            .await?
    } else {
        tokio::fs::File::create(&temp_path).await?
    };

    let mut downloaded: u64 = start_offset;
    let mut stream = response;

    while let Some(chunk) = stream.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        progress_cb(downloaded, total);
    }
    file.flush().await?;

    // 原子重命名
    if let Err(e) = tokio::fs::rename(&temp_path, dest).await {
        tracing::warn!("Failed to rename temp file to final destination: {}", e);
        // 如果重命名失败，尝试复制
        tokio::fs::copy(&temp_path, dest).await?;
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    Ok(())
}

// body decode errors and timeouts are worth retrying, but a 404 or disk
// error isn't. Parse errors stay non-retryable: by the time we hit one
// the response body has fully arrived, so the failure means the upstream
// returned malformed JSON - retrying won't fix that.
fn is_retryable(err: &NetError) -> bool {
    match err {
        NetError::Http(e) => e.is_timeout() || e.is_body() || e.is_connect(),
        NetError::StatusError { status, .. } => *status >= 500,
        _ => false,
    }
}

// tries JAVA_HOME first, then PATH, then just yolos "java" and hopes for the best
#[must_use]
pub fn detect_java_path() -> String {
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_name = if cfg!(windows) { "java.exe" } else { "java" };
        let bin = std::path::Path::new(&java_home).join("bin").join(java_name);
        if bin.exists() {
            tracing::trace!("Detected Java from JAVA_HOME: {}", bin.display());
            return bin.to_string_lossy().to_string();
        }
        tracing::warn!(
            "JAVA_HOME is set to {}, but {} does not exist",
            java_home,
            bin.display()
        );
    }
    match which::which("java") {
        Ok(path) => {
            tracing::trace!("Detected Java from PATH: {}", path.display());
            path.to_string_lossy().to_string()
        }
        Err(e) => {
            tracing::warn!(
                "Could not find java on PATH, falling back to literal 'java': {}",
                e
            );
            "java".to_string()
        }
    }
}

// converts maven coordinates like "org.example:artifact:1.0" into a
// filesystem path like "org/example/artifact/1.0/artifact-1.0.jar".
// supports optional classifier as a 4th component.

#[must_use]
pub fn maven_coord_to_path(coord: &str) -> Option<String> {
    let parts: Vec<&str> = coord.split(':').collect();
    match parts.as_slice() {
        [group, artifact, version] => {
            let group_path = group.replace('.', "/");
            Some(format!(
                "{}/{}/{}/{}-{}.jar",
                group_path, artifact, version, artifact, version
            ))
        }
        [group, artifact, version, classifier] => {
            let group_path = group.replace('.', "/");
            Some(format!(
                "{}/{}/{}/{}-{}-{}.jar",
                group_path, artifact, version, artifact, version, classifier
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_3_part_coord() {
        assert_eq!(
            maven_coord_to_path("org.example:artifact:1.0"),
            Some("org/example/artifact/1.0/artifact-1.0.jar".to_string())
        );
    }

    #[test]
    fn maven_4_part_coord_with_classifier() {
        assert_eq!(
            maven_coord_to_path("org.example:artifact:1.0:sources"),
            Some("org/example/artifact/1.0/artifact-1.0-sources.jar".to_string())
        );
    }

    #[test]
    fn maven_nested_group() {
        assert_eq!(
            maven_coord_to_path("com.google.code.gson:gson:2.10"),
            Some("com/google/code/gson/gson/2.10/gson-2.10.jar".to_string())
        );
    }

    #[test]
    fn maven_invalid_too_few_parts() {
        assert_eq!(maven_coord_to_path("org.example:artifact"), None);
    }

    #[test]
    fn maven_invalid_too_many_parts() {
        assert_eq!(maven_coord_to_path("a:b:c:d:e"), None);
    }

    #[test]
    fn maven_invalid_single_part() {
        assert_eq!(maven_coord_to_path("just-a-string"), None);
    }

    #[test]
    fn maven_empty_string() {
        assert_eq!(maven_coord_to_path(""), None);
    }
}
