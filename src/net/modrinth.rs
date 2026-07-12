// Modrinth API v2 - 模组搜索、版本获取、下载
// 公开接口免鉴权，所有请求携带 User-Agent 标识

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::instance::models::ModLoader;

// ── 常量 ──

const API_BASE: &str = "https://api.modrinth.com/v2";

// ── 数据模型 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<ProjectDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    #[serde(rename = "type")]
    pub dep_type: String, // required, optional, incompatible, embedded
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModSearchResult {
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub icon_url: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<ModSearchResult>,
    #[serde(default)]
    pub total_hits: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<VersionDependency>,
    #[serde(default)]
    pub date_published: String,
    #[serde(default)]
    pub status: String, // release, beta, alpha
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    #[serde(rename = "type")]
    pub dep_type: String, // required, optional, incompatible, embedded
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub size: u64,
    pub primary: bool,
    #[serde(default)]
    pub hashes: HashMap<String, String>, // sha1, sha512, etc.
}

// ── 搜索参数构建器 ──

pub struct SearchParams {
    pub query: Option<String>,
    pub facets: Vec<Vec<String>>,
    pub limit: u32,
    pub offset: u32,
    pub index: String, // downloads, follows, newest, updated, relevance
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: None,
            facets: vec![vec!["project_type:mod".to_string()]],
            limit: 20,
            offset: 0,
            index: "downloads".to_string(),
        }
    }
}

impl SearchParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query(mut self, q: &str) -> Self {
        if !q.is_empty() {
            self.query = Some(q.to_string());
        }
        self
    }

    pub fn game_version(mut self, version: &str) -> Self {
        if !version.is_empty() {
            self.facets.push(vec![format!("versions:{}", version)]);
        }
        self
    }

    pub fn loader(mut self, loader: &str) -> Self {
        if !loader.is_empty() {
            self.facets
                .push(vec![format!("categories:{}", loader)]);
        }
        self
    }

    pub fn category(mut self, category: &str) -> Self {
        if !category.is_empty() {
            self.facets
                .push(vec![format!("categories:{}", category)]);
        }
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit.min(100);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    pub fn index(mut self, index: &str) -> Self {
        self.index = index.to_string();
        self
    }

    /// 构建查询字符串
    pub fn to_query_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref q) = self.query {
            parts.push(format!("query={}", urlencoding::encode(q)));
        }

        let facets_json = serde_json::to_string(&self.facets).unwrap_or_default();
        parts.push(format!("facets={}", urlencoding::encode(&facets_json)));

        parts.push(format!("limit={}", self.limit));
        parts.push(format!("offset={}", self.offset));
        parts.push(format!("index={}", urlencoding::encode(&self.index)));

        parts.join("&")
    }
}

// ── 本地缓存 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedModEntry {
    pub project_id: String,
    pub title: String,
    pub icon_url: String,
    pub description: String,
    pub downloads: i64,
    pub downloaded_at: chrono::DateTime<chrono::Utc>,
    pub file_path: PathBuf,
    pub file_hash_sha512: Option<String>,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModCache {
    pub entries: HashMap<String, CachedModEntry>, // project_id -> entry
    pub version: u32,
}

impl Default for ModCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            version: 1,
        }
    }
}

impl ModCache {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_entry(&mut self, entry: CachedModEntry) {
        self.entries.insert(entry.project_id.clone(), entry);
    }

    pub fn get_entry(&self, project_id: &str) -> Option<&CachedModEntry> {
        self.entries.get(project_id)
    }

    pub fn remove_entry(&mut self, project_id: &str) {
        self.entries.remove(project_id);
    }
}

// ── API 函数 ──

/// 搜索模组（使用查询参数序列化，不手动拼 URL）
pub async fn search_mods(
    client: &crate::net::HttpClient,
    params: &SearchParams,
) -> Result<SearchResponse, crate::net::NetError> {
    let query_string = params.to_query_string();
    let url = format!("{}/search?{}", API_BASE, query_string);

    tracing::info!("Modrinth search URL: {}", url);

    let response: SearchResponse = client.get_json(&url).await?;
    tracing::info!(
        "Modrinth search returned {} results (total: {}, offset: {}, limit: {})",
        response.hits.len(),
        response.total_hits,
        response.offset,
        response.limit
    );

    Ok(response)
}

/// 获取项目详情
pub async fn fetch_project(
    client: &crate::net::HttpClient,
    project_id: &str,
) -> Result<ProjectInfo, crate::net::NetError> {
    let url = format!("{}/project/{}", API_BASE, urlencoding::encode(project_id));
    tracing::debug!("Fetching Modrinth project '{}'", project_id);
    let project: ProjectInfo = client.get_json(&url).await?;
    tracing::debug!("Fetched Modrinth project '{}' ({})", project.slug, project.id);
    Ok(project)
}

/// 获取版本列表（支持游戏版本和加载器筛选）
pub async fn fetch_versions(
    client: &crate::net::HttpClient,
    project_id: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<VersionInfo>, crate::net::NetError> {
    let mut params = Vec::new();

    if let Some(gv) = game_version {
        if !gv.is_empty() {
            params.push(format!("game_versions=[\"{}\"]", gv));
        }
    }
    if let Some(ld) = loader {
        if !ld.is_empty() {
            params.push(format!("loaders=[\"{}\"]", ld));
        }
    }

    let url = if params.is_empty() {
        format!("{}/project/{}/version", API_BASE, urlencoding::encode(project_id))
    } else {
        format!(
            "{}/project/{}/version?{}",
            API_BASE,
            urlencoding::encode(project_id),
            params.join("&")
        )
    };

    tracing::debug!(
        "Fetching Modrinth versions for '{}' (version={:?} loader={:?})",
        project_id,
        game_version,
        loader
    );

    let versions: Vec<VersionInfo> = client.get_json(&url).await?;
    tracing::debug!("Fetched {} version(s) for '{}'", versions.len(), project_id);

    Ok(versions)
}

/// 获取指定版本详情
pub async fn fetch_version(
    client: &crate::net::HttpClient,
    version_id: &str,
) -> Result<VersionInfo, crate::net::NetError> {
    let url = format!("{}/version/{}", API_BASE, urlencoding::encode(version_id));
    tracing::debug!("Fetching Modrinth version '{}'", version_id);
    let version: VersionInfo = client.get_json(&url).await?;
    tracing::debug!(
        "Fetched version '{}' ({}) with {} file(s)",
        version.name,
        version.id,
        version.files.len()
    );
    Ok(version)
}

/// 获取分类列表
pub async fn fetch_categories(
    client: &crate::net::HttpClient,
) -> Result<Vec<serde_json::Value>, crate::net::NetError> {
    let url = format!("{}/tag/category", API_BASE);
    let categories: Vec<serde_json::Value> = client.get_json(&url).await?;
    Ok(categories)
}

/// 下载模组文件（带 SHA512 校验）
pub async fn download_mod_file(
    client: &crate::net::HttpClient,
    url: &str,
    dest: &Path,
    expected_hash: Option<&str>,
) -> Result<(), crate::net::NetError> {
    // 如果文件已存在且哈希匹配，跳过下载
    if dest.exists() {
        if let Some(expected) = expected_hash {
            if let Ok(local_hash) = compute_file_sha512(dest) {
                if local_hash == expected {
                    tracing::debug!("File already exists with matching hash: {}", dest.display());
                    return Ok(());
                }
            }
        }
    }

    // 下载文件
    crate::net::download_file(client, url, dest, |downloaded, total| {
        if total > 0 {
            let percent = (downloaded as f64 / total as f64 * 100.0) as u32;
            tracing::trace!("Download progress: {}%", percent);
        }
    })
    .await?;

    // 验证哈希
    if let Some(expected) = expected_hash {
        let actual_hash = compute_file_sha512(dest)
            .map_err(|e| crate::net::NetError::Parse(format!("Failed to compute hash: {}", e)))?;

        if actual_hash != expected {
            // 删除错误文件
            let _ = std::fs::remove_file(dest);
            return Err(crate::net::NetError::Parse(format!(
                "Hash mismatch: expected {}, got {}",
                expected, actual_hash
            )));
        }
        tracing::debug!("SHA512 hash verified for {}", dest.display());
    }

    Ok(())
}

/// 下载 mrpack 整合包
pub async fn download_mrpack(
    client: &crate::net::HttpClient,
    version: &VersionInfo,
    dest: &Path,
) -> Result<PathBuf, crate::net::NetError> {
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| {
            tracing::warn!(
                "Version '{}' has no primary file; using first file",
                version.id
            );
            version.files.first()
        })
        .ok_or_else(|| crate::net::NetError::Parse("No files in version".to_string()))?;

    let hash = file.hashes.get("sha512").map(|s| s.as_str());
    let mrpack_path = dest.join(&file.filename);

    tracing::info!(
        "Downloading mrpack '{}' for version '{}' to {}",
        file.filename,
        version.id,
        mrpack_path.display()
    );

    download_mod_file(client, &file.url, &mrpack_path, hash).await?;
    Ok(mrpack_path)
}

/// 解析 mrpack 整合包
pub fn parse_mrpack(path: &Path) -> Result<MrpackIndex, String> {
    tracing::debug!("Parsing .mrpack from {}", path.display());
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open .mrpack: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP: {e}"))?;
    let entry = archive
        .by_name("modrinth.index.json")
        .map_err(|_| "Missing modrinth.index.json in .mrpack".to_string())?;
    let index: MrpackIndex =
        serde_json::from_reader(entry).map_err(|e| format!("Invalid manifest JSON: {e}"))?;
    tracing::debug!(
        "Parsed .mrpack '{}' version_id={} files={} deps={}",
        index.name,
        index.version_id,
        index.files.len(),
        index.dependencies.len()
    );
    Ok(index)
}

// ── mrpack 模型 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackIndex {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    pub game: String,
    #[serde(rename = "versionId")]
    pub version_id: String,
    pub name: String,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub files: Vec<MrpackFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackFile {
    pub path: String,
    pub downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
}

// ── 依赖解析 ──

/// 解析依赖并返回需要下载的项目 ID 列表（递归 required 依赖）
pub fn resolve_dependencies(
    dependencies: &[VersionDependency],
    downloaded: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut result = Vec::new();

    for dep in dependencies {
        if dep.dep_type == "required" {
            if let Some(ref project_id) = dep.project_id {
                if !downloaded.contains(project_id) {
                    result.push(project_id.clone());
                }
            }
        }
    }

    result
}

/// 从 mrpack 依赖中解析加载器
pub fn loader_from_dependencies(
    deps: &HashMap<String, String>,
) -> (Option<ModLoader>, Option<String>) {
    let loaders = [
        ("fabric-loader", ModLoader::Fabric),
        ("forge", ModLoader::Forge),
        ("neoforge", ModLoader::NeoForge),
        ("quilt-loader", ModLoader::Quilt),
    ];
    for (key, loader) in &loaders {
        if let Some(version) = deps.get(*key) {
            tracing::trace!(
                "Resolved loader dependency {}={} as {}",
                key,
                version,
                loader
            );
            return (Some(*loader), Some(version.clone()));
        }
    }
    tracing::trace!("No loader dependency found; treating as vanilla");
    (None, None)
}

/// 从 mrpack 依赖中获取游戏版本
pub fn game_version_from_dependencies(deps: &HashMap<String, String>) -> Option<String> {
    deps.get("minecraft").cloned()
}

// ── 工具函数 ──

/// 计算文件 SHA512 哈希
pub fn compute_file_sha512(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha512};

    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha512::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// 验证文件哈希
pub fn verify_file_hash(path: &Path, expected_hash: &str, algorithm: &str) -> Result<bool, String> {
    match algorithm {
        "sha512" => {
            let actual = compute_file_sha512(path)?;
            Ok(actual == expected_hash)
        }
        "sha1" => {
            use sha1::{Digest, Sha1};
            let data = std::fs::read(path).map_err(|e| e.to_string())?;
            let mut hasher = Sha1::new();
            hasher.update(&data);
            let result = hasher.finalize();
            let actual = hex::encode(result);
            Ok(actual == expected_hash)
        }
        _ => Err(format!("Unsupported hash algorithm: {}", algorithm)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_params_to_query_string() {
        let params = SearchParams::new()
            .query("fabric api")
            .game_version("1.21.1")
            .loader("fabric")
            .limit(20)
            .offset(0);

        let qs = params.to_query_string();
        assert!(qs.contains("query=fabric%20api"));
        assert!(qs.contains("limit=20"));
        assert!(qs.contains("offset=0"));
        assert!(qs.contains("facets="));
    }

    #[test]
    fn search_params_default_facets() {
        let params = SearchParams::new();
        assert_eq!(params.facets, vec![vec!["project_type:mod".to_string()]]);
    }

    #[test]
    fn search_params_add_loader() {
        let params = SearchParams::new().loader("neoforge");
        assert!(params.facets.contains(&vec!["categories:neoforge".to_string()]));
    }

    #[test]
    fn loader_from_fabric_deps() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.21.4".to_string());
        deps.insert("fabric-loader".to_string(), "0.16.10".to_string());
        let (loader, version) = loader_from_dependencies(&deps);
        assert_eq!(loader, Some(ModLoader::Fabric));
        assert_eq!(version, Some("0.16.10".to_string()));
    }

    #[test]
    fn loader_from_forge_deps() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.1".to_string());
        deps.insert("forge".to_string(), "47.2.0".to_string());
        let (loader, version) = loader_from_dependencies(&deps);
        assert_eq!(loader, Some(ModLoader::Forge));
        assert_eq!(version, Some("47.2.0".to_string()));
    }

    #[test]
    fn game_version_from_deps() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.21.4".to_string());
        assert_eq!(
            game_version_from_dependencies(&deps),
            Some("1.21.4".to_string())
        );
    }

    #[test]
    fn mod_cache_save_load() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_path = tmp.path().join("cache.json");

        let mut cache = ModCache::default();
        cache.add_entry(CachedModEntry {
            project_id: "test-mod".to_string(),
            title: "Test Mod".to_string(),
            icon_url: String::new(),
            description: "A test mod".to_string(),
            downloads: 100,
            downloaded_at: chrono::Utc::now(),
            file_path: PathBuf::from("mods/test-mod.jar"),
            file_hash_sha512: None,
            file_size: 1024,
        });

        cache.save(&cache_path).unwrap();

        let loaded = ModCache::load(&cache_path);
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.get_entry("test-mod").is_some());
    }

    #[test]
    fn resolve_required_dependencies() {
        let deps = vec![
            VersionDependency {
                project_id: Some("dep-1".to_string()),
                version_id: None,
                dep_type: "required".to_string(),
            },
            VersionDependency {
                project_id: Some("dep-2".to_string()),
                version_id: None,
                dep_type: "optional".to_string(),
            },
            VersionDependency {
                project_id: Some("dep-3".to_string()),
                version_id: None,
                dep_type: "required".to_string(),
            },
        ];

        let downloaded = std::collections::HashSet::from(["dep-1".to_string()]);
        let needed = resolve_dependencies(&deps, &downloaded);

        assert_eq!(needed, vec!["dep-3".to_string()]);
    }
}

#[cfg(test)]
mod url_tests {
    use super::*;

    #[test]
    fn search_params_generates_correct_url() {
        let params = SearchParams::new()
            .query("sodium")
            .game_version("1.21.1")
            .loader("neoforge")
            .limit(20)
            .offset(0);
        let url = format!("{}/search?{}", API_BASE, params.to_query_string());
        eprintln!("Generated URL: {}", url);
        // Should contain all facets
        assert!(url.contains("project_type%3Amod"));
        assert!(url.contains("versions%3A1.21.1"));
        assert!(url.contains("categories%3Aneoforge"));
        assert!(url.contains("query=sodium"));
    }

    #[test]
    fn search_params_wildcard_url() {
        let params = SearchParams::new()
            .limit(20)
            .offset(0);
        let url = format!("{}/search?{}", API_BASE, params.to_query_string());
        eprintln!("Wildcard URL: {}", url);
        // Should not contain query param
        assert!(!url.contains("query="));
        assert!(url.contains("project_type%3Amod"));
    }
}
