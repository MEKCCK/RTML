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

#![allow(dead_code)]

use crate::net::download::source;
use crate::net::{HttpClient, NetError};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

fn api_base() -> &'static str {
    source::modrinth_api_base()
}

fn all_api_bases() -> Vec<&'static str> {
    source::all_modrinth_bases()
}

fn deserialize_null_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModResult {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub follows: u64,
    pub icon_url: String,
    pub client_side: String,
    pub server_side: String,
    pub project_type: String,
    pub latest_version: Option<String>,
    pub date_created: String,
    pub date_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProjectFull {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub author: String,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub follows: u64,
    pub icon_url: String,
    pub client_side: String,
    pub server_side: String,
    pub project_type: String,
    pub gallery: Vec<ModGalleryImage>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub license: Option<ModLicense>,
    pub date_created: String,
    pub date_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModGalleryImage {
    pub url: String,
    pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModLicense {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModVersion {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<ModFile>,
    #[serde(default)]
    pub dependencies: Vec<ModDependency>,
    #[serde(default)]
    pub date_published: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub size: u64,
    pub hashes: ModHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModHashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDependency {
    pub project_id: Option<String>,
    pub dependency_type: String,
    pub version_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthSearchHit>,
    total_hits: u64,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchHit {
    slug: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    categories: Vec<String>,
    downloads: u64,
    follows: u64,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    icon_url: String,
    #[serde(default)]
    client_side: String,
    #[serde(default)]
    server_side: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    project_type: String,
    #[serde(default)]
    date_created: String,
    #[serde(default)]
    date_modified: String,
    latest_version: Option<String>,
}

impl From<ModrinthSearchHit> for ModResult {
    fn from(h: ModrinthSearchHit) -> Self {
        ModResult {
            slug: h.slug,
            title: h.title,
            description: h.description,
            author: h.author,
            categories: h.categories,
            downloads: h.downloads,
            follows: h.follows,
            icon_url: h.icon_url,
            client_side: h.client_side,
            server_side: h.server_side,
            project_type: h.project_type,
            latest_version: h.latest_version,
            date_created: h.date_created,
            date_modified: h.date_modified,
        }
    }
}

impl From<ModrinthProject> for ModResult {
    fn from(p: ModrinthProject) -> Self {
        ModResult {
            slug: p.slug,
            title: p.title,
            description: p.description,
            author: p.author,
            categories: p.categories,
            downloads: p.downloads,
            follows: p.follows,
            icon_url: p.icon_url,
            client_side: p.client_side,
            server_side: p.server_side,
            project_type: p.project_type,
            latest_version: None,
            date_created: p.date_created,
            date_modified: p.date_modified,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ModrinthProject {
    slug: String,
    title: String,
    description: String,
    author: String,
    categories: Vec<String>,
    downloads: u64,
    follows: u64,
    icon_url: String,
    client_side: String,
    server_side: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    project_type: String,
    date_created: String,
    date_modified: String,
}

#[derive(Debug, Deserialize)]
struct ModrinthProjectFull {
    #[serde(default, deserialize_with = "deserialize_null_string")]
    slug: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    title: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    description: String,
    #[serde(default)]
    body: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    author: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    team: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    organization: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    follows: u64,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    icon_url: String,
    #[serde(default)]
    client_side: String,
    #[serde(default)]
    server_side: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    project_type: String,
    #[serde(default)]
    gallery: Vec<ModrinthGalleryImage>,
    #[serde(default)]
    issues_url: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    wiki_url: Option<String>,
    #[serde(default)]
    discord_url: Option<String>,
    #[serde(default)]
    license: Option<ModrinthLicense>,
    #[serde(default)]
    date_created: String,
    #[serde(default)]
    date_modified: String,
}

#[derive(Debug, Deserialize)]
struct ModrinthGalleryImage {
    #[serde(default, deserialize_with = "deserialize_null_string", alias = "url")]
    url: String,
    #[serde(default)]
    featured: bool,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    created: String,
}

#[derive(Debug, Deserialize)]
struct ModrinthLicense {
    #[serde(default, deserialize_with = "deserialize_null_string")]
    id: String,
    #[serde(default, deserialize_with = "deserialize_null_string")]
    name: String,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthVersion {
    id: String,
    name: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    files: Vec<ModrinthFile>,
    #[serde(default)]
    dependencies: Vec<ModrinthDependency>,
    #[serde(default)]
    date_published: String,
}

#[derive(Debug, Deserialize)]
struct ModrinthFile {
    url: String,
    filename: String,
    #[serde(default)]
    size: u64,
    hashes: ModrinthHashes,
}

#[derive(Debug, Deserialize)]
struct ModrinthHashes {
    sha1: Option<String>,
    sha512: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModrinthDependency {
    project_id: Option<String>,
    dependency_type: String,
    version_id: Option<String>,
}

fn build_api_client() -> &'static HttpClient {
    static CLIENT: std::sync::LazyLock<HttpClient> = std::sync::LazyLock::new(|| {
        let client = reqwest::Client::builder()
            .user_agent(format!("RTML/{} (Minecraft Launcher)", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        HttpClient::from_inner(client)
    });
    &CLIENT
}

async fn retry_api_get_json_with_fallback<T: DeserializeOwned>(
    url_template: &str,
    api_bases: &[&str],
    max_retries: u32,
) -> Result<T, NetError> {
    let client = build_api_client();
    let mut last_err = None;

    for (base_idx, base) in api_bases.iter().enumerate() {
        let url = url_template.replace("{API_BASE}", base);
        if base_idx > 0 {
            tracing::info!("Modrinth API JSON fallback: trying base {}", base);
        }

        match retry_get_with_headers(client, &url, max_retries, None).await {
            Ok(resp) => {
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_lowercase();
                if !content_type.contains("json") && !content_type.is_empty() {
                    tracing::warn!(
                        "Non-JSON response (Content-Type: {}) from {}, trying next source",
                        content_type, base
                    );
                    last_err = Some(NetError::StatusError {
                        status: 200,
                        url: url.clone(),
                    });
                    continue;
                }
                match resp.json::<T>().await {
                    Ok(v) => return Ok(v),
                    Err(e) => {
                        tracing::warn!(
                            "JSON decode failed on base {} for {}: {}, trying next source ({}/{})",
                            base, url, e, base_idx + 1, api_bases.len()
                        );
                        last_err = Some(NetError::Http(e));
                        continue;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Modrinth API request with base {} failed: {}, trying next source ({}/{})",
                    base, e, base_idx + 1, api_bases.len()
                );
                last_err = Some(e);
                continue;
            }
        }
    }

    Err(last_err.unwrap_or(NetError::NetworkUnreachable))
}

async fn retry_get_with_fallback(
    url_template: &str,
    api_bases: &[&str],
    max_retries: u32,
) -> Result<reqwest::Response, NetError> {
    let client = build_api_client();
    let mut last_err = None;

    for (base_idx, base) in api_bases.iter().enumerate() {
        let url = url_template.replace("{API_BASE}", base);
        if base_idx > 0 {
            tracing::info!("Modrinth API fallback: trying base {}", base);
        }
        match retry_get_with_headers(client, &url, max_retries, None).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                tracing::warn!(
                    "Modrinth API request with base {} failed: {}, trying next ({}/{})",
                    base, e, base_idx + 1, api_bases.len()
                );
                last_err = Some(e);
                continue;
            }
        }
    }

    Err(last_err.unwrap_or(NetError::NetworkUnreachable))
}

async fn retry_get_with_headers(
    client: &HttpClient,
    url: &str,
    max_retries: u32,
    _headers: Option<reqwest::header::HeaderMap>,
) -> Result<reqwest::Response, NetError> {
    let mut last_err = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            let delay = Duration::from_millis(500 * 2u64.pow(attempt - 1));
            tokio::time::sleep(delay).await;
        }
        match client.get(url).await {
            Ok(resp) => {
                if resp.status().is_success() {
                    return Ok(resp);
                }
                let status = resp.status();
                if status.is_server_error() && attempt < max_retries {
                    tracing::warn!("API request to {} returned {}, retrying ({}/{})", url, status, attempt + 1, max_retries);
                    last_err = Some(NetError::StatusError {
                        status: status.as_u16(),
                        url: url.to_string(),
                    });
                    continue;
                }
                match resp.error_for_status() {
                    Ok(r) => return Ok(r),
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) => {
                if attempt < max_retries {
                    tracing::warn!("API request to {} failed: {}, retrying ({}/{})", url, e, attempt + 1, max_retries);
                    last_err = Some(e.into());
                    continue;
                }
                return Err(e.into());
            }
        }
    }
    Err(last_err.unwrap_or(NetError::NetworkUnreachable))
}

pub async fn search_mods(
    query: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<(Vec<ModResult>, u64), NetError> {
    let mut facets = vec![r#"["project_type:mod"]"#.to_string()];

    if let Some(ver) = game_version {
        facets.push(format!(r#"["versions:{}"]"#, ver));
    }
    if let Some(ldr) = loader {
        facets.push(format!(r#"["categories:{}"]"#, ldr));
    }

    let facets_param = format!("[{}]", facets.join(","));
    let url_template = format!(
        "{{API_BASE}}/search?query={}&facets={}&limit={}&offset={}",
        urlencoding::encode(query), facets_param, limit.min(50), offset
    );

    tracing::debug!("Modrinth search: {}", url_template);

    let bases = all_api_bases();
    let resp: ModrinthSearchResponse = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;

    let results: Vec<ModResult> = resp.hits.into_iter().map(Into::into).collect();
    Ok((results, resp.total_hits))
}

pub async fn get_mod(slug: &str) -> Result<ModResult, NetError> {
    let url_template = format!("{{API_BASE}}/project/{}", slug);
    let bases = all_api_bases();

    let resp: ModrinthProject = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;
    Ok(resp.into())
}

pub async fn get_mod_versions(
    slug: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<ModVersion>, NetError> {
    let mut url_template = format!("{{API_BASE}}/project/{}/version", slug);

    let mut params = Vec::new();
    if let Some(ver) = game_version {
        params.push(format!("game_versions=[\"{}\"]", ver));
    }
    if let Some(ldr) = loader {
        params.push(format!("loaders=[\"{}\"]", ldr));
    }
    if !params.is_empty() {
        url_template.push('?');
        url_template.push_str(&params.join("&"));
    }

    let bases = all_api_bases();
    let resp: Vec<ModrinthVersion> = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;

    Ok(resp
        .into_iter()
        .map(|v| ModVersion {
            id: v.id,
            name: v.name,
            version_number: v.version_number,
            game_versions: v.game_versions,
            loaders: v.loaders,
            files: v.files.into_iter().map(|f| ModFile {
                url: f.url,
                filename: f.filename,
                size: f.size,
                hashes: ModHashes { sha1: f.hashes.sha1, sha512: f.hashes.sha512 },
            }).collect(),
            dependencies: v.dependencies.into_iter().map(|d| ModDependency {
                project_id: d.project_id,
                dependency_type: d.dependency_type,
                version_id: d.version_id,
            }).collect(),
            date_published: v.date_published,
        })
        .collect())
}

pub async fn get_popular_mods(
    game_version: Option<&str>,
    limit: u64,
) -> Result<Vec<ModResult>, NetError> {
    let mut facets = vec![r#"["project_type:mod"]"#.to_string()];

    if let Some(ver) = game_version {
        facets.push(format!(r#"["versions:{}"]"#, ver));
    }

    let facets_param = format!("[{}]", facets.join(","));
    let url_template = format!(
        "{{API_BASE}}/search?facets={}&limit={}&index=downloads",
        facets_param, limit.min(50)
    );

    let bases = all_api_bases();
    let resp: ModrinthSearchResponse = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;
    Ok(resp.hits.into_iter().map(Into::into).collect())
}

pub async fn search_with_facets(
    query: &str,
    project_type: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
    sort: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<(Vec<ModResult>, u64), NetError> {
    let mut facets = vec![format!(r#"["project_type:{}"]"#, project_type)];

    if let Some(ver) = game_version {
        if !ver.is_empty() {
            facets.push(format!(r#"["versions:{}"]"#, ver));
        }
    }
    if let Some(ldr) = loader {
        if !ldr.is_empty() {
            facets.push(format!(r#"["categories:{}"]"#, ldr));
        }
    }

    let facets_param = format!("[{}]", facets.join(","));
    let sort_order = match sort.unwrap_or("relevance") {
        "downloads" => "downloads",
        "newest" => "newest",
        "updated" => "updated",
        _ => "relevance",
    };

    let url_template = format!(
        "{{API_BASE}}/search?query={}&facets={}&limit={}&offset={}&index={}",
        urlencoding::encode(query),
        facets_param,
        limit.min(50),
        offset,
        sort_order,
    );

    tracing::debug!("Modrinth search (facets): {}", url_template);

    let bases = all_api_bases();
    let resp: ModrinthSearchResponse = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;
    let results: Vec<ModResult> = resp.hits.into_iter().map(Into::into).collect();
    Ok((results, resp.total_hits))
}

pub async fn get_project_full(slug: &str) -> Result<ModProjectFull, NetError> {
    let url_template = format!("{{API_BASE}}/project/{}", slug);
    let bases = all_api_bases();

    let resp: ModrinthProjectFull = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;

    Ok(ModProjectFull {
        slug: resp.slug,
        title: resp.title,
        description: resp.description,
        body: resp.body,
        author: if resp.author.is_empty() {
            if !resp.team.is_empty() { resp.team }
            else if !resp.organization.is_empty() { resp.organization }
            else { String::new() }
        } else { resp.author },
        categories: resp.categories,
        downloads: resp.downloads,
        follows: resp.follows,
        icon_url: resp.icon_url,
        client_side: resp.client_side,
        server_side: resp.server_side,
        project_type: resp.project_type,
        gallery: resp.gallery.into_iter().map(|g| ModGalleryImage {
            url: g.url,
            featured: g.featured,
            title: g.title,
            description: g.description,
            created: g.created,
        }).collect(),
        issues_url: resp.issues_url,
        source_url: resp.source_url,
        wiki_url: resp.wiki_url,
        discord_url: resp.discord_url,
        license: resp.license.map(|l| ModLicense {
            id: l.id,
            name: l.name,
            url: l.url,
        }),
        date_created: resp.date_created,
        date_modified: resp.date_modified,
    })
}

pub async fn get_version_by_id(version_id: &str) -> Result<ModVersion, NetError> {
    let url_template = format!("{{API_BASE}}/version/{}", version_id);
    let bases = all_api_bases();

    let v: ModrinthVersion = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;

    Ok(ModVersion {
        id: v.id,
        name: v.name,
        version_number: v.version_number,
        game_versions: v.game_versions,
        loaders: v.loaders,
        files: v.files.into_iter().map(|f| ModFile {
            url: f.url,
            filename: f.filename,
            size: f.size,
            hashes: ModHashes { sha1: f.hashes.sha1, sha512: f.hashes.sha512 },
        }).collect(),
        dependencies: v.dependencies.into_iter().map(|d| ModDependency {
            project_id: d.project_id,
            dependency_type: d.dependency_type,
            version_id: d.version_id,
        }).collect(),
        date_published: v.date_published,
    })
}

pub async fn get_popular_by_type(
    project_type: &str,
    game_version: Option<&str>,
    limit: u64,
) -> Result<Vec<ModResult>, NetError> {
    let mut facets = vec![format!(r#"["project_type:{}"]"#, project_type)];
    if let Some(ver) = game_version {
        if !ver.is_empty() {
            facets.push(format!(r#"["versions:{}"]"#, ver));
        }
    }

    let facets_param = format!("[{}]", facets.join(","));
    let url_template = format!(
        "{{API_BASE}}/search?facets={}&limit={}&index=downloads",
        facets_param, limit.min(50)
    );

    tracing::debug!("Modrinth popular by type: {}", url_template);

    let bases = all_api_bases();
    let resp: ModrinthSearchResponse = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;
    Ok(resp.hits.into_iter().map(Into::into).collect())
}

pub async fn get_recently_updated(
    project_type: Option<&str>,
    limit: u64,
) -> Result<Vec<ModResult>, NetError> {
    let mut facets = Vec::new();
    if let Some(pt) = project_type {
        if !pt.is_empty() {
            facets.push(format!(r#"["project_type:{}"]"#, pt));
        }
    }

    let url_template = if facets.is_empty() {
        format!("{{API_BASE}}/search?limit={}&index=updated", limit.min(50))
    } else {
        let facets_param = format!("[{}]", facets.join(","));
        format!("{{API_BASE}}/search?facets={}&limit={}&index=updated", facets_param, limit.min(50))
    };

    tracing::debug!("Modrinth recently updated: {}", url_template);

    let bases = all_api_bases();
    let resp: ModrinthSearchResponse = retry_api_get_json_with_fallback(&url_template, &bases, 2).await?;
    Ok(resp.hits.into_iter().map(Into::into).collect())
}

pub async fn download_file_to_instance(
    file_url: &str,
    filename: &str,
    instance_mc_dir: &std::path::Path,
    content_type: &str,
    sha1_hash: Option<&str>,
    on_progress: Option<&(dyn Fn(u64, u64) + Sync)>,
) -> Result<String, NetError> {
    let target_dir = match content_type {
        "resourcepack" => instance_mc_dir.join("resourcepacks"),
        "shader" => instance_mc_dir.join("shaderpacks"),
        _ => instance_mc_dir.join("mods"),
    };
    tokio::fs::create_dir_all(&target_dir).await?;
    let target_path = target_dir.join(filename);

    let client = reqwest::Client::builder()
        .user_agent(format!("RTML/{} (Minecraft Launcher)", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let mut response = client.get(file_url).send().await?.error_for_status()?;

    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = tokio::io::BufWriter::new(tokio::fs::File::create(&target_path).await?);

    let mut hasher = sha1_hash.map(|_| <sha1::Sha1 as Default>::default());

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_progress = std::time::Instant::now();

    while let Some(chunk) = response.chunk().await? {
        if let Some(ref mut h) = hasher {
            use sha1::Digest;
            h.update(&chunk);
        }
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let now = std::time::Instant::now();
            if now.duration_since(last_progress).as_millis() >= 200 {
                last_progress = now;
                if let Some(cb) = on_progress {
                    cb(downloaded, total_size);
                }
            }
        }
    }

    file.flush().await?;

    if let (Some(expected_sha1), Some(h)) = (sha1_hash, hasher) {
        use sha1::Digest;
        let actual = hex::encode(h.finalize());
        if !actual.eq_ignore_ascii_case(expected_sha1) {
            let _ = tokio::fs::remove_file(&target_path).await;
            return Err(NetError::Sha1Mismatch(format!(
                "File {} expected SHA1 {} but got {}",
                filename, expected_sha1, actual
            )));
        }
    }

    tracing::info!("Content downloaded: {} -> {}", filename, target_path.display());
    Ok(target_path.to_string_lossy().to_string())
}

pub async fn download_mod_file(
    file_url: &str,
    filename: &str,
    instance_mc_dir: &std::path::Path,
    sha1_hash: Option<&str>,
    on_progress: Option<&(dyn Fn(u64, u64) + Sync)>,
) -> Result<String, NetError> {
    download_file_to_instance(file_url, filename, instance_mc_dir, "mod", sha1_hash, on_progress).await
}
