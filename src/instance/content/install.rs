// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::instance::manager::InstanceError;

type MetadataMap = HashMap<String, InstallRecord>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRecord {
    pub slug: String,
    pub version_id: Option<String>,
    pub content_type: String,
    pub installed_at: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub pinned: bool,
}

fn default_source() -> String {
    "modrinth".to_string()
}

fn get_metadata_path(instances_dir: &PathBuf, instance_name: &str) -> PathBuf {
    instances_dir
        .join(instance_name)
        .join(".minecraft")
        .join("installed_content.json")
}

pub fn load_metadata(instances_dir: &PathBuf, instance_name: &str) -> Result<MetadataMap, InstanceError> {
    let path = get_metadata_path(instances_dir, instance_name);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = std::fs::read_to_string(&path)?;
    let map: MetadataMap = serde_json::from_str(&data).unwrap_or_default();
    Ok(map)
}

fn save_metadata(instances_dir: &PathBuf, instance_name: &str, map: &MetadataMap) -> Result<(), InstanceError> {
    let path = get_metadata_path(instances_dir, instance_name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(map)?;
    std::fs::write(&path, data)?;
    Ok(())
}

pub fn record_install(
    instances_dir: &PathBuf,
    instance_name: &str,
    filename: &str,
    slug: &str,
    version_id: Option<&str>,
    content_type: &str,
    source: &str,
) -> Result<(), InstanceError> {
    let mut map = load_metadata(instances_dir, instance_name)?;
    map.insert(
        filename.to_string(),
        InstallRecord {
            slug: slug.to_string(),
            version_id: version_id.map(|s| s.to_string()),
            content_type: content_type.to_string(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            source: source.to_string(),
            pinned: false,
        },
    );
    save_metadata(instances_dir, instance_name, &map)
}

pub fn remove_record(instances_dir: &PathBuf, instance_name: &str, filename: &str) -> Result<(), InstanceError> {
    let mut map = load_metadata(instances_dir, instance_name)?;
    map.remove(filename);
    save_metadata(instances_dir, instance_name, &map)
}

pub fn pin_mod(instances_dir: &PathBuf, instance_name: &str, slug: &str) -> Result<bool, InstanceError> {
    let mut map = load_metadata(instances_dir, instance_name)?;
    let mut found = false;
    for record in map.values_mut() {
        if record.slug == slug {
            record.pinned = true;
            found = true;
        }
    }
    if found {
        save_metadata(instances_dir, instance_name, &map)?;
    }
    Ok(found)
}

pub fn unpin_mod(instances_dir: &PathBuf, instance_name: &str, slug: &str) -> Result<bool, InstanceError> {
    let mut map = load_metadata(instances_dir, instance_name)?;
    let mut found = false;
    for record in map.values_mut() {
        if record.slug == slug {
            record.pinned = false;
            found = true;
        }
    }
    if found {
        save_metadata(instances_dir, instance_name, &map)?;
    }
    Ok(found)
}

pub fn is_pinned(instances_dir: &PathBuf, instance_name: &str, slug: &str) -> Result<bool, InstanceError> {
    let map = load_metadata(instances_dir, instance_name)?;
    Ok(map.values().any(|r| r.slug == slug && r.pinned))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub filename: String,
    pub slug: String,
    pub installed_version: Option<String>,
    pub latest_version: String,
    pub content_type: String,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSessionState {
    pub session_id: String,
    pub instance_name: String,
    pub phase: String,
    pub total_files: u32,
    pub completed_files: u32,
    pub failed: bool,
    pub error_message: Option<String>,
}

pub struct AtomicInstaller {
    instance_name: String,
    session_id: String,
    instances_dir: PathBuf,
    temp_dir: PathBuf,
    backup_dir: PathBuf,
    backups: Vec<(PathBuf, PathBuf)>,
}

impl AtomicInstaller {
    pub fn new(instances_dir: PathBuf, instance_name: &str, session_id: &str) -> Self {
        let mc_dir = instances_dir.join(instance_name).join(".minecraft");
        AtomicInstaller {
            instances_dir,
            instance_name: instance_name.to_string(),
            session_id: session_id.to_string(),
            temp_dir: mc_dir.join(".install_tmp").join(session_id),
            backup_dir: mc_dir.join(".install_tmp").join(session_id).join("_backups"),
            backups: Vec::new(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }

    pub fn prepare(&self) -> Result<(), InstanceError> {
        std::fs::create_dir_all(&self.temp_dir)?;
        std::fs::create_dir_all(&self.backup_dir)?;
        Ok(())
    }

    pub fn backup_existing(&mut self, target_path: &std::path::Path) -> Result<(), InstanceError> {
        if target_path.exists() {
            let backup_name = format!(
                "{}_{}",
                target_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                chrono::Utc::now().timestamp_millis()
            );
            let backup_path = self.backup_dir.join(&backup_name);
            std::fs::copy(target_path, &backup_path)?;
            self.backups.push((target_path.to_path_buf(), backup_path));
        }
        Ok(())
    }

    pub fn temp_path_for(&self, filename: &str) -> PathBuf {
        self.temp_dir.join(filename)
    }

    pub fn commit(self) -> Result<(), InstanceError> {
        for (target, _) in &self.backups {
            let _ = std::fs::remove_file(target);
        }

        let entries: Vec<_> = std::fs::read_dir(&self.temp_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path() != self.backup_dir)
            .collect();

        for entry in &entries {
            let src = entry.path();
            if src.is_file() && !src.starts_with(&self.backup_dir) {
                if let Some(filename) = src.file_name() {
                    let target_dir = self.determine_target_dir();
                    let dest = target_dir.join(filename);
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::rename(&src, &dest).or_else(|_| {
                        std::fs::copy(&src, &dest)?;
                        std::fs::remove_file(&src)
                    })?;
                }
            }
        }

        let _ = std::fs::remove_dir_all(&self.temp_dir);
        Ok(())
    }

    pub fn rollback(self) -> Result<(), InstanceError> {
        tracing::warn!("Rolling back install session {}", self.session_id);

        for (target, backup) in &self.backups {
            if backup.exists() {
                if let Err(e) = std::fs::copy(backup, target) {
                    tracing::error!("Failed to restore backup {} -> {}: {}", backup.display(), target.display(), e);
                }
            }
        }

        let temp_entries: Vec<_> = std::fs::read_dir(&self.temp_dir)
            .ok()
            .map(|d| d.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        for entry in temp_entries {
            if entry.path().is_file() && !entry.path().starts_with(&self.backup_dir) {
                let _ = std::fs::remove_file(entry.path());
            }
        }

        let _ = std::fs::remove_dir_all(&self.temp_dir);
        Ok(())
    }

    fn determine_target_dir(&self) -> PathBuf {
        self.instances_dir
            .join(&self.instance_name)
            .join(".minecraft")
            .join("mods")
    }

    pub fn get_state(&self, phase: &str, total_files: u32, completed_files: u32) -> InstallSessionState {
        InstallSessionState {
            session_id: self.session_id.clone(),
            instance_name: self.instance_name.clone(),
            phase: phase.to_string(),
            total_files,
            completed_files,
            failed: false,
            error_message: None,
        }
    }
}
