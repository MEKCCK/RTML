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
use std::path::Path;

use sha1::{Digest, Sha1};
use sha2::Sha256;

use crate::net::NetError;

pub fn verify_file_sha1(path: &Path, expected_sha1: &str) -> Result<(), NetError> {
    let file = std::fs::File::open(path).map_err(|e| {
        NetError::Sha1Mismatch(format!(
            "cannot read file {}: {}",
            path.display(),
            e
        ))
    })?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).map_err(|e| {
            NetError::Sha1Mismatch(format!("cannot read file {}: {}", path.display(), e))
        })?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let actual_sha1 = hex::encode(hasher.finalize());

    if actual_sha1.eq_ignore_ascii_case(expected_sha1) {
        Ok(())
    } else {
        Err(NetError::Sha1Mismatch(format!(
            "file {} expected {} but got {}",
            path.display(),
            expected_sha1,
            actual_sha1
        )))
    }
}

pub fn compute_sha1(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn file_exists_and_valid(path: &Path, expected_sha1: &str, expected_size: u64, strict: bool) -> bool {
    if !path.exists() {
        return false;
    }

    if expected_size > 0 {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() != expected_size {
                return false;
            }
        } else {
            return false;
        }
    }

    if expected_sha1.is_empty() {
        if strict {
            return false;
        }
        return true;
    }

    verify_file_sha1(path, expected_sha1).is_ok()
}

pub async fn verify_file_sha1_async(path: impl AsRef<std::path::Path> + Send + 'static, expected_sha1: String) -> Result<(), NetError> {
    tokio::task::spawn_blocking(move || verify_file_sha1(path.as_ref(), &expected_sha1)).await?
}

pub fn verify_file_sha256(path: &Path, expected_sha256: &str) -> Result<(), NetError> {
    let file = std::fs::File::open(path).map_err(|e| {
        NetError::Sha1Mismatch(format!(
            "cannot read file {}: {}",
            path.display(),
            e
        ))
    })?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).map_err(|e| {
            NetError::Sha1Mismatch(format!("cannot read file {}: {}", path.display(), e))
        })?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let actual_sha256 = hex::encode(hasher.finalize());

    if actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        Ok(())
    } else {
        Err(NetError::Sha1Mismatch(format!(
            "file {} SHA256 expected {} but got {}",
            path.display(),
            expected_sha256,
            actual_sha256
        )))
    }
}

pub fn compute_sha256(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub async fn verify_file_sha256_async(path: impl AsRef<std::path::Path> + Send + 'static, expected_sha256: String) -> Result<(), NetError> {
    tokio::task::spawn_blocking(move || verify_file_sha256(path.as_ref(), &expected_sha256)).await?
}

pub fn file_exists_and_valid_sha256(path: &Path, expected_sha256: &str, expected_size: u64, strict: bool) -> bool {
    if !path.exists() {
        return false;
    }

    if expected_size > 0 {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() != expected_size {
                return false;
            }
        } else {
            return false;
        }
    }

    if expected_sha256.is_empty() {
        if strict {
            return false;
        }
        return true;
    }

    verify_file_sha256(path, expected_sha256).is_ok()
}
