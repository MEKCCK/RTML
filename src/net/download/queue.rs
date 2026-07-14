use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use reqwest::Client;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

use crate::net::download::source;
use crate::net::download::verifier;
use crate::net::NetError;

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 500;

fn compute_speed_eta(downloaded: u64, total: u64, elapsed: std::time::Duration) -> (u64, u64) {
    if downloaded == 0 || elapsed.as_millis() < 100 {
        return (0, 0);
    }
    let bytes_per_second = (downloaded as f64 / elapsed.as_secs_f64()) as u64;
    let remaining = total.saturating_sub(downloaded);
    let eta_seconds = remaining.checked_div(bytes_per_second).unwrap_or(0);
    (bytes_per_second, eta_seconds)
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub url: String,
    pub downloaded: u64,
    pub total: u64,
    pub finished: bool,
    pub error: Option<String>,
    pub bytes_per_second: u64,
    pub eta_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub target_path: PathBuf,
    pub sha1: String,
    pub size: u64,
}

impl DownloadTask {
    pub fn new(url: impl Into<String>, target_path: impl Into<PathBuf>, sha1: impl Into<String>, size: u64) -> Self {
        DownloadTask {
            url: url.into(),
            target_path: target_path.into(),
            sha1: sha1.into(),
            size,
        }
    }

    pub fn is_already_valid(&self) -> bool {
        verifier::file_exists_and_valid(&self.target_path, &self.sha1, self.size, false)
    }
}

pub struct DownloadQueue {
    client: Client,
    semaphore: Arc<Semaphore>,
    event_callback: Option<Arc<dyn Fn(DownloadProgress) + Send + Sync>>,
    paused: Arc<AtomicBool>,
    cancelled_urls: Arc<Mutex<HashSet<String>>>,
}

impl DownloadQueue {
    pub fn new() -> Self {
        let max_concurrent: usize = 6;
        DownloadQueue {
            client: Client::builder()
                .user_agent(format!("RTML/{} (Minecraft Launcher)", env!("CARGO_PKG_VERSION")))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            event_callback: None,
            paused: Arc::new(AtomicBool::new(false)),
            cancelled_urls: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        self.event_callback = Some(Arc::new(callback));
        self
    }

    fn emit_progress(&self, progress: DownloadProgress) {
        if let Some(ref cb) = self.event_callback {
            cb(progress);
        }
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn cancel(&self, url: &str) {
        self.cancelled_urls.lock().unwrap().insert(url.to_string());
    }

    pub fn is_cancelled(&self, url: &str) -> bool {
        self.cancelled_urls.lock().unwrap().contains(url)
    }

    pub fn clear_cancelled(&self, url: &str) {
        self.cancelled_urls.lock().unwrap().remove(url);
    }

    async fn wait_while_paused(&self) {
        while self.paused.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    pub async fn download_single(&self, task: &DownloadTask) -> Result<(), NetError> {
        if self.is_cancelled(&task.url) {
            return Err(NetError::DownloadFailed("cancelled".to_string()));
        }

        if task.is_already_valid() {
            self.emit_progress(DownloadProgress {
                url: task.url.clone(),
                downloaded: task.size,
                total: task.size,
                finished: true,
                error: None,
                bytes_per_second: 0,
                eta_seconds: 0,
            });
            return Ok(());
        }

        let fallback_urls = source::SourceManager::transform_with_fallback(&task.url);
        let mut last_error: Option<String> = None;

        for (source_name, transformed_url) in &fallback_urls {
            for attempt in 0..=MAX_RETRIES {
                self.wait_while_paused().await;

                if self.is_cancelled(&task.url) {
                    let _ = tokio::fs::remove_file(&task.target_path).await;
                    return Err(NetError::DownloadFailed("cancelled".to_string()));
                }

                if attempt > 0 {
                    let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt - 1);
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    tracing::warn!(
                        "Retrying download (attempt {}/{}): {}",
                        attempt,
                        MAX_RETRIES,
                        transformed_url
                    );
                }

                match self.do_download(transformed_url, &task.url, &task.target_path, task.size).await {
                    Ok(downloaded) => {
                        if !task.sha1.is_empty() {
                            if let Err(e) = verifier::verify_file_sha1(&task.target_path, &task.sha1) {
                                tracing::error!("SHA1 verification failed: {}", e);
                                let _ = tokio::fs::remove_file(&task.target_path).await;
                                last_error = Some(e.to_string());
                                continue;
                            }
                        }

                        self.emit_progress(DownloadProgress {
                            url: task.url.clone(),
                            downloaded,
                            total: task.size,
                            finished: true,
                            error: None,
                            bytes_per_second: 0,
                            eta_seconds: 0,
                        });
                        return Ok(());
                    }
                    Err(e) => {
                        last_error = Some(e.to_string());
                        tracing::error!(
                            "Download failed via {} (attempt {}/{}): {} - {}",
                            source_name,
                            attempt + 1,
                            MAX_RETRIES + 1,
                            transformed_url,
                            e
                        );
                    }
                }
            }

            tracing::warn!("Source {} exhausted, trying next source...", source_name);
        }

        let _ = tokio::fs::remove_file(&task.target_path).await;

        Err(NetError::DownloadFailed(
            last_error.unwrap_or_else(|| "all sources failed".to_string()),
        ))
    }

    async fn do_download(
        &self,
        url: &str,
        original_url: &str,
        target_path: &Path,
        expected_size: u64,
    ) -> Result<u64, NetError> {
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut response = self.client.get(url).send().await?;
        response = response.error_for_status()?;

        let total_size = response.content_length().unwrap_or(expected_size);

        let mut file = tokio::io::BufWriter::new(tokio::fs::File::create(target_path).await?);

        let mut downloaded: u64 = 0;
        let mut last_emit = std::time::Instant::now();
        let start_time = std::time::Instant::now();

        while let Some(chunk) = response.chunk().await? {
            self.wait_while_paused().await;

            if self.is_cancelled(url) || self.is_cancelled(original_url) {
                drop(file);
                let _ = tokio::fs::remove_file(target_path).await;
                return Err(NetError::DownloadFailed("cancelled".to_string()));
            }

            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            let now = std::time::Instant::now();
            if now.duration_since(last_emit).as_millis() >= 200 {
                last_emit = now;
                let elapsed = start_time.elapsed();
                let (bytes_per_second, eta_seconds) = compute_speed_eta(downloaded, total_size, elapsed);
                self.emit_progress(DownloadProgress {
                    url: url.to_string(),
                    downloaded,
                    total: total_size,
                    finished: false,
                    error: None,
                    bytes_per_second,
                    eta_seconds,
                });
            }
        }

        file.flush().await?;

        Ok(downloaded)
    }

    pub async fn download_all(
        &self,
        tasks: Vec<DownloadTask>,
    ) -> Result<Vec<Result<(), NetError>>, NetError> {
        let total = tasks.len();
        let mut results = Vec::with_capacity(total);
        let mut handles = Vec::with_capacity(total);

        for (index, task) in tasks.into_iter().enumerate() {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let client = self.client.clone();
            let callback = self.event_callback.clone();
            let semaphore = self.semaphore.clone();
            let paused = self.paused.clone();
            let cancelled_urls = self.cancelled_urls.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                let queue = DownloadQueue {
                    client,
                    semaphore,
                    event_callback: callback,
                    paused,
                    cancelled_urls,
                };
                let result = queue.download_single(&task).await;
                if let Some(ref cb) = queue.event_callback {
                    cb(DownloadProgress {
                        url: task.url.clone(),
                        downloaded: if result.is_ok() { task.size } else { 0 },
                        total: task.size,
                        finished: true,
                        error: result.as_ref().err().map(|e| e.to_string()),
                        bytes_per_second: 0,
                        eta_seconds: 0,
                    });
                }
                (index, result)
            });

            handles.push(handle);
        }

        for handle in handles {
            let (index, result) = handle.await?;
            while results.len() <= index {
                results.push(Err(NetError::TaskFailed("missing result".to_string())));
            }
            results[index] = result;
        }

        Ok(results)
    }
}


