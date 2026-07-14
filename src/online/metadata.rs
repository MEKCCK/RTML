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

use std::io::Read;
use std::path::PathBuf;

use serde::Deserialize;
use sha2::{Digest, Sha512};

use crate::config::SETTINGS;

#[derive(Debug, Clone, Deserialize)]
struct PackageEntry {
    hash: String,
    files: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TerracottaConfig {
    #[serde(rename = "version_latest")]
    version_latest: String,
    packages: std::collections::HashMap<String, PackageEntry>,
    downloads: Vec<String>,
    #[serde(rename = "downloads_CN")]
    downloads_cn: Vec<String>,
}

pub struct TerracottaMetadata;

impl TerracottaMetadata {
    fn config() -> TerracottaConfig {
        serde_json::from_str(include_str!("../../assets/terracotta.json"))
            .expect("embedded terracotta.json is valid")
    }

    pub fn platform_classifier() -> Option<&'static str> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        match (os, arch) {
            ("windows", "x86_64") => Some("windows-x86_64"),
            ("linux", "x86_64") => Some("linux-x86_64"),
            ("linux", "aarch64") => Some("linux-arm64"),
            ("macos", "x86_64") => Some("macos-x86_64"),
            ("macos", "aarch64") => Some("macos-arm64"),
            _ => None,
        }
    }

    pub fn binary_name() -> Option<String> {
        let classifier = Self::platform_classifier()?;
        let cfg = Self::config();
        let suffix = if classifier.starts_with("windows") {
            ".exe"
        } else {
            ""
        };
        Some(format!(
            "terracotta-{}-{}{}",
            cfg.version_latest, classifier, suffix
        ))
    }

    pub fn is_supported() -> bool {
        Self::platform_classifier().is_some()
    }

    pub fn latest_version() -> String {
        Self::config().version_latest
    }

    fn replace_tokens(s: &str, version: &str, classifier: &str) -> String {
        s.replace("${version}", version)
            .replace("${classifier}", classifier)
    }

    pub fn bundle_info(
    ) -> Option<(
        Vec<String>,
        String,
        std::collections::HashMap<String, String>,
    )> {
        let cfg = Self::config();
        let classifier = Self::platform_classifier()?;

        let pkg = cfg.packages.get(classifier)?;

        let urls: Vec<String> = cfg
            .downloads_cn
            .iter()
            .chain(cfg.downloads.iter())
            .map(|u| Self::replace_tokens(u, &cfg.version_latest, classifier))
            .collect();

        let files: std::collections::HashMap<String, String> = pkg
            .files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Some((urls, pkg.hash.clone(), files))
    }

    pub fn cache_dir() -> PathBuf {
        SETTINGS
            .paths
            .resolve_meta_dir()
            .join("terracotta")
            .join(Self::latest_version())
    }

    pub fn binary_path() -> Option<PathBuf> {
        let name = Self::binary_name()?;
        Some(Self::cache_dir().join(name))
    }

    pub fn is_binary_valid() -> bool {
        let binary_path = match Self::binary_path() {
            Some(p) => p,
            None => return false,
        };

        if !binary_path.exists() {
            return false;
        }

        let (_, _, files) = match Self::bundle_info() {
            Some(info) => info,
            None => return false,
        };

        let binary_name = match Self::binary_name() {
            Some(n) => n,
            None => return false,
        };

        match files.get(&binary_name) {
            Some(expected_hash) => match sha512_file(&binary_path) {
                Some(actual_hash) => {
                    if actual_hash.to_lowercase() != expected_hash.to_lowercase() {
                        tracing::warn!(
                            "Terracotta binary hash mismatch: expected={}, actual={}",
                            expected_hash,
                            actual_hash
                        );
                        return false;
                    }
                    true
                }
                None => false,
            },
            None => false,
        }
    }

    pub async fn ensure_binary_downloaded() -> Result<PathBuf, String> {
        if Self::is_binary_valid() {
            return Self::binary_path().ok_or("binary path resolved previously".to_string());
        }

        let (urls, bundle_hash, files) = Self::bundle_info().ok_or("unsupported platform")?;
        let cache_dir = Self::cache_dir();
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("failed to create cache dir: {e}"))?;

        let binary_name = Self::binary_name().ok_or("unsupported platform")?;
        let binary_path = cache_dir.join(&binary_name);

        if binary_path.exists() {
            return Ok(binary_path);
        }

        // Download the .tar.gz bundle, verify SHA-512, extract binary
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| format!("reqwest client: {e}"))?;

        let mut last_err = String::new();
        let mut bundle_bytes = Vec::new();

        for url in &urls {
            tracing::info!("Downloading terracotta bundle from {url}");
            match client.get(url).send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        last_err = format!("HTTP {}", resp.status());
                        continue;
                    }
                    match resp.bytes().await {
                        Ok(bytes) => {
                            bundle_bytes = bytes.to_vec();
                            // Verify bundle SHA-512
                            let actual_hash = hex::encode(Sha512::digest(&bundle_bytes));
                            if actual_hash.to_lowercase() != bundle_hash.to_lowercase() {
                                last_err = format!(
                                    "bundle SHA-512 mismatch: expected={bundle_hash}, got={actual_hash}"
                                );
                                continue;
                            }
                            last_err.clear();
                            break;
                        }
                        Err(e) => {
                            last_err = format!("read body: {e}");
                            continue;
                        }
                    }
                }
                Err(e) => {
                    last_err = format!("request failed: {e}");
                }
            }
        }

        if !last_err.is_empty() {
            return Err(format!(
                "failed to download terracotta: {last_err}"
            ));
        }

        // Extract binary from tar.gz
        let decoder = flate2::read::GzDecoder::new(&bundle_bytes[..]);
        let mut archive = tar::Archive::new(decoder);

        let mut extracted = false;
        for entry in archive.entries().map_err(|e| format!("tar read: {e}"))? {
            let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
            let path = entry.path().map_err(|e| format!("entry path: {e}"))?;

            if path.ends_with(&binary_name) {
                // Verify extracted binary hash

                let mut binary_data = Vec::new();
                entry
                    .read_to_end(&mut binary_data)
                    .map_err(|e| format!("read binary from tar: {e}"))?;

                let expected_hash = files.get(&binary_name).ok_or("no hash for binary")?;
                let actual_hash = hex::encode(Sha512::digest(&binary_data));
                if actual_hash.to_lowercase() != expected_hash.to_lowercase() {
                    return Err(format!(
                        "binary SHA-512 mismatch: expected={expected_hash}, got={actual_hash}"
                    ));
                }

                std::fs::write(&binary_path, &binary_data)
                    .map_err(|e| format!("write binary: {e}"))?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))
                        .map_err(|e| format!("chmod: {e}"))?;
                }

                extracted = true;
                break;
            }
        }

        if !extracted {
            return Err(format!("binary {binary_name} not found in bundle"));
        }

        tracing::info!("Terracotta binary cached at {:?}", binary_path);
        Ok(binary_path)
    }
}

fn sha512_file(path: &std::path::Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha512::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hex::encode(hasher.finalize()))
}
