// RTML - Rust TUI Minecraft Launcher
// Copyright (C) 2026 RTML Contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This is a modified version of rmcl (https://github.com/objz/rmcl).
// Modifications made in 2026.

use chrono::Utc;
use serde::Deserialize;
use std::path::PathBuf;

use crate::instance::manager::InstanceError;
use crate::instance::models::{InstanceConfig, ModLoader};
use crate::net::download::queue::{DownloadQueue, DownloadTask};

#[derive(Debug, Clone, PartialEq)]
pub enum ModpackFormat {
    MrPack,
    CurseForge,
    Unknown,
}

#[derive(Debug, Deserialize)]
struct MrPackIndexFile {
    path: String,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default)]
    sha1: String,
    #[serde(default)]
    #[serde(rename = "fileSize")]
    file_size: u64,
    #[serde(default)]
    env: Option<MrPackEnv>,
}

#[derive(Debug, Deserialize)]
struct MrPackEnv {
    client: Option<String>,
    #[allow(dead_code)]
    server: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MrPackIndex {
    name: String,
    #[serde(rename = "versionId")]
    version_id: Option<String>,
    #[allow(dead_code)]
    summary: Option<String>,
    #[serde(default)]
    files: Vec<MrPackIndexFile>,
    #[serde(default)]
    dependencies: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct CfManifest {
    #[serde(rename = "minecraft")]
    minecraft: CfMinecraft,
    #[serde(rename = "manifestType")]
    #[allow(dead_code)]
    manifest_type: Option<String>,
    #[serde(rename = "name")]
    #[allow(dead_code)]
    name: Option<String>,
    #[serde(default)]
    files: Vec<CfManifestFile>,
}

#[derive(Debug, Deserialize)]
struct CfMinecraft {
    #[serde(rename = "version")]
    version: String,
    #[serde(rename = "modLoaders")]
    mod_loaders: Vec<CfModLoader>,
}

#[derive(Debug, Deserialize)]
struct CfModLoader {
    id: String,
    primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    required: Option<bool>,
}

fn is_client_supported(env: &Option<MrPackEnv>) -> bool {
    match env {
        Some(e) => match &e.client {
            Some(v) => v != "unsupported",
            None => true,
        },
        None => true,
    }
}

fn safe_extract_path(relative: &str) -> Option<std::path::PathBuf> {
    let safe: std::path::PathBuf = std::path::Path::new(relative)
        .components()
        .filter(|c| !matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir))
        .collect();
    if safe.as_os_str().is_empty() {
        return None;
    }
    Some(safe)
}

fn map_loader_to_modloader(loader: &str) -> ModLoader {
    match loader {
        "fabric-loader" => ModLoader::Fabric,
        "forge" => ModLoader::Forge,
        "neoforge" => ModLoader::NeoForge,
        "quilt-loader" => ModLoader::Quilt,
        _ => ModLoader::Vanilla,
    }
}

#[allow(dead_code)]
fn compute_sha1_streaming(path: &std::path::Path) -> Result<String, std::io::Error> {
    use sha1::{Digest, Sha1};
    use std::io::Read;
    let mut hasher = Sha1::new();
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn extract_zip_overrides(
    archive: &mut zip::ZipArchive<std::fs::File>,
    prefixes: &[&str],
    dest_base: &std::path::Path,
) -> Result<u32, InstanceError> {
    let mut extracted: u32 = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.ends_with('/') {
            continue;
        }
        let relative = prefixes
            .iter()
            .find_map(|p| name.strip_prefix(p))
            .unwrap_or("");
        if relative.is_empty() {
            continue;
        }
        if let Some(safe_relative) = safe_extract_path(relative) {
            let dest = dest_base.join(&safe_relative);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut entry = entry;
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
            extracted += 1;
        }
    }
    Ok(extracted)
}

pub fn detect_modpack_format(path: &str) -> Result<ModpackFormat, InstanceError> {
    let zip_path = std::path::Path::new(path);
    if !zip_path.exists() {
        return Err(InstanceError::InvalidName(format!("File not found: {}", path)));
    }

    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name == "modrinth.index.json" {
            return Ok(ModpackFormat::MrPack);
        }
        if name == "manifest.json" {
            return Ok(ModpackFormat::CurseForge);
        }
    }

    Ok(ModpackFormat::Unknown)
}

pub async fn import_modpack(
    path: &str,
    instances_dir: &PathBuf,
    _meta_dir: &PathBuf,
    download_callback: Option<Box<dyn Fn(String, u32, u32) + Send + Sync>>,
) -> Result<InstanceConfig, InstanceError> {
    let zip_path = std::path::Path::new(path);
    if !zip_path.exists() {
        return Err(InstanceError::NotFound(format!("File not found: {}", path)));
    }

    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut index_json = String::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.name() == "modrinth.index.json" {
            std::io::Read::read_to_string(&mut entry, &mut index_json)?;
            break;
        }
    }

    if index_json.is_empty() {
        return Err(InstanceError::InvalidName(
            "modrinth.index.json not found in .mrpack".into(),
        ));
    }

    let index: MrPackIndex = serde_json::from_str(&index_json)?;

    let version_id = index
        .dependencies
        .get("minecraft")
        .cloned()
        .or(index.version_id)
        .unwrap_or_else(|| "1.21".to_string());

    let loader_type = index
        .dependencies
        .keys()
        .find(|k| matches!(k.as_str(), "fabric-loader" | "forge" | "neoforge" | "quilt-loader"))
        .cloned();

    let loader_version = loader_type
        .as_ref()
        .and_then(|lt| index.dependencies.get(lt))
        .cloned();

    let instance_name = format!(
        "{}-{}",
        index.name.replace(' ', "-").to_lowercase(),
        version_id
    );

    let instance = InstanceConfig {
        name: instance_name.clone(),
        game_version: version_id.clone(),
        loader: loader_type
            .as_deref()
            .map(map_loader_to_modloader)
            .unwrap_or(ModLoader::Vanilla),
        loader_version,
        created: Utc::now(),
        last_played: None,
        java_path: None,
        memory_max: Some("4G".to_string()),
        memory_min: Some("512M".to_string()),
        jvm_args: vec![],
        resolution: None,
        config_sync_profile: None,
    };

    let instance_dir = instances_dir.join(&instance_name);
    if instance_dir.join("instance.json").exists() {
        return Err(InstanceError::AlreadyExists(instance_name));
    }

    let minecraft_dir = instance_dir.join(".minecraft");
    std::fs::create_dir_all(minecraft_dir.join("mods"))?;
    std::fs::create_dir_all(minecraft_dir.join("config"))?;
    std::fs::create_dir_all(minecraft_dir.join("resourcepacks"))?;
    std::fs::create_dir_all(minecraft_dir.join("shaderpacks"))?;
    std::fs::create_dir_all(minecraft_dir.join("saves"))?;

    let instance_json = instance_dir.join("instance.json");
    let data = serde_json::to_string_pretty(&instance)?;
    std::fs::write(&instance_json, data)?;

    let mods_dir = minecraft_dir.join("mods");
    let mut download_tasks: Vec<DownloadTask> = Vec::new();
    for f in &index.files {
        if !is_client_supported(&f.env) {
            tracing::debug!("Skipping server-only file: {}", f.path);
            continue;
        }
        if let Some(url) = f.downloads.first() {
            let file_name = std::path::Path::new(&f.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| f.path.clone());
            let dest = mods_dir.join(&file_name);
            download_tasks.push(DownloadTask::new(url.clone(), dest, f.sha1.clone(), f.file_size));
        }
    }

    if !download_tasks.is_empty() {
        tracing::info!(
            "Downloading {} mod files for modpack '{}'...",
            download_tasks.len(),
            index.name
        );
        let total = download_tasks.len() as u32;

        let queue = if let Some(cb) = download_callback {
            let completed = std::sync::atomic::AtomicU32::new(0);
            DownloadQueue::new().with_callback(move |progress| {
                if progress.finished {
                    let c = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    cb(progress.url.clone(), total, c);
                }
            })
        } else {
            DownloadQueue::new()
        };

        let results = queue.download_all(download_tasks).await?;
        let succeeded = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.len() - succeeded;
        if failed > 0 {
            tracing::warn!("{}/{} modpack mod downloads failed", failed, results.len());
        }
    }

    let client_count =
        extract_zip_overrides(&mut archive, &["client-overrides/", "overrides/"], &minecraft_dir)?;
    tracing::info!(
        "Extracted {} override files for modpack '{}'",
        client_count,
        index.name
    );

    tracing::info!("Modpack '{}' imported as instance '{}'", index.name, instance_name);
    Ok(instance)
}

pub async fn import_curseforge_modpack(
    path: &str,
    instances_dir: &PathBuf,
    _meta_dir: &PathBuf,
    download_callback: Option<Box<dyn Fn(String, u32, u32) + Send + Sync>>,
) -> Result<InstanceConfig, InstanceError> {
    let zip_path = std::path::Path::new(path);
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut manifest_json = String::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.name() == "manifest.json" {
            std::io::Read::read_to_string(&mut entry, &mut manifest_json)?;
            break;
        }
    }

    if manifest_json.is_empty() {
        return Err(InstanceError::InvalidName(
            "manifest.json not found in CurseForge modpack".into(),
        ));
    }

    let manifest: CfManifest = serde_json::from_str(&manifest_json)?;

    let version_id = manifest.minecraft.version.clone();

    let (loader_type, loader_version) = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|ml| ml.primary.unwrap_or(true))
        .or_else(|| manifest.minecraft.mod_loaders.first())
        .map(|ml| {
            let id = &ml.id;
            if let Some(dash_pos) = id.find('-') {
                let lt = id[..dash_pos].to_string();
                let lv = id[dash_pos + 1..].to_string();
                (Some(lt), Some(lv))
            } else {
                (Some(id.clone()), None)
            }
        })
        .unwrap_or((None, None));

    let instance_name = format!("cf-{}-{}", version_id, chrono::Utc::now().timestamp_millis());

    let instance = InstanceConfig {
        name: instance_name.clone(),
        game_version: version_id.clone(),
        loader: loader_type
            .as_deref()
            .map(|l| match l {
                "fabric" => ModLoader::Fabric,
                "forge" => ModLoader::Forge,
                "neoforge" => ModLoader::NeoForge,
                "quilt" => ModLoader::Quilt,
                _ => ModLoader::Vanilla,
            })
            .unwrap_or(ModLoader::Vanilla),
        loader_version,
        created: Utc::now(),
        last_played: None,
        java_path: None,
        memory_max: Some("4G".to_string()),
        memory_min: Some("512M".to_string()),
        jvm_args: vec![],
        resolution: None,
        config_sync_profile: None,
    };

    let instance_dir = instances_dir.join(&instance_name);
    let minecraft_dir = instance_dir.join(".minecraft");
    std::fs::create_dir_all(minecraft_dir.join("mods"))?;
    std::fs::create_dir_all(minecraft_dir.join("config"))?;
    std::fs::create_dir_all(minecraft_dir.join("resourcepacks"))?;
    std::fs::create_dir_all(minecraft_dir.join("shaderpacks"))?;
    std::fs::create_dir_all(minecraft_dir.join("saves"))?;

    let instance_json = instance_dir.join("instance.json");
    let data = serde_json::to_string_pretty(&instance)?;
    std::fs::write(&instance_json, data)?;

    let mods_dir = minecraft_dir.join("mods");

    let curseforge_api_key = crate::config::SETTINGS
        .general
        .curseforge_api_key
        .clone()
        .or_else(|| std::env::var("CURSEFORGE_API_KEY").ok());
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(ref key) = curseforge_api_key {
        if let Ok(hv) = reqwest::header::HeaderValue::from_str(key) {
            headers.insert("x-api-key", hv);
        }
    }
    let client = reqwest::Client::builder()
        .user_agent(format!("RTML/{} (Minecraft Launcher)", env!("CARGO_PKG_VERSION")))
        .default_headers(headers)
        .build()
        .unwrap_or_default();

    let required_files: Vec<_> = manifest
        .files
        .iter()
        .filter(|f| f.required.unwrap_or(true))
        .collect();
    let total_cf = required_files.len() as u32;

    let mut api_handles = Vec::with_capacity(required_files.len());
    for cf_file in &required_files {
        let client = client.clone();
        let file_url = format!(
            "https://api.curseforge.com/v1/mods/{}/files/{}",
            cf_file.project_id, cf_file.file_id
        );
        api_handles.push(tokio::spawn(async move {
            let resp = client.get(&file_url).send().await.ok()?;
            let json: serde_json::Value = resp.json().await.ok()?;
            let data = json.get("data")?;
            let download_url = data.get("downloadUrl")?.as_str()?.to_string();
            let filename = data
                .get("fileName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown.jar")
                .to_string();
            let sha1_hash = data
                .get("hashes")
                .and_then(|h| h.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|h| h.get("algo").and_then(|a| a.as_i64()) == Some(1))
                })
                .and_then(|h| h.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let file_size = data.get("fileLength").and_then(|v| v.as_u64()).unwrap_or(0);
            Some((download_url, filename, sha1_hash, file_size))
        }));
    }

    let mut download_tasks: Vec<DownloadTask> = Vec::new();
    let mut failed_count: u32 = 0;
    for handle in api_handles {
        match handle.await.unwrap_or_default() {
            Some((url, name, sha1, size)) => {
                if url.is_empty() {
                    tracing::warn!("CF file has no download URL, skipping");
                    failed_count += 1;
                } else {
                    download_tasks.push(DownloadTask::new(url, mods_dir.join(&name), sha1, size));
                }
            }
            None => {
                failed_count += 1;
            }
        }
    }

    let mut downloaded_count: u32 = 0;
    if !download_tasks.is_empty() {
        let queue = match download_callback {
            Some(cb) => {
                let completed = std::sync::atomic::AtomicU32::new(0);
                DownloadQueue::new().with_callback(move |progress| {
                    if progress.finished {
                        let c = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        cb(progress.url.clone(), total_cf, c);
                    }
                })
            }
            None => DownloadQueue::new(),
        };
        let results = queue.download_all(download_tasks).await.unwrap_or_default();
        for r in &results {
            if r.is_ok() {
                downloaded_count += 1;
            } else {
                failed_count += 1;
            }
        }
    }

    tracing::info!(
        "CF mod download: {} succeeded, {} failed",
        downloaded_count,
        failed_count
    );

    let override_count =
        extract_zip_overrides(&mut archive, &["overrides/"], &minecraft_dir)?;
    tracing::info!("Extracted {} override files for CF modpack", override_count);

    tracing::info!("CurseForge modpack imported as instance '{}'", instance_name);
    Ok(instance)
}

pub async fn import_modpack_auto(
    path: &str,
    instances_dir: &PathBuf,
    meta_dir: &PathBuf,
    download_callback: Option<Box<dyn Fn(String, u32, u32) + Send + Sync>>,
) -> Result<InstanceConfig, InstanceError> {
    let format = detect_modpack_format(path)?;
    match format {
        ModpackFormat::MrPack => {
            import_modpack(path, instances_dir, meta_dir, download_callback).await
        }
        ModpackFormat::CurseForge => {
            import_curseforge_modpack(path, instances_dir, meta_dir, download_callback).await
        }
        ModpackFormat::Unknown => Err(InstanceError::InvalidName(
            "Unknown modpack format. Supported: .mrpack (Modrinth), CurseForge ZIP".into(),
        )),
    }
}
