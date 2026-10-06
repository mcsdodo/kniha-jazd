//! Fuelio files from Dropbox (Task 90).
//!
//! Fuelio backs up its drives to Dropbox (`/Apps/Fuelio/routes`). The sync
//! copies the `route-<id>.data` files of one year into `<DATA_DIR>/fuelio`,
//! the folder that [`super::scan_dir`] reads. Only files that are not there
//! yet are downloaded, so a repeat sync costs one listing (a few calls).
//!
//! The API calls sit behind [`RouteStore`] so the sync can be tested without
//! the network. [`DropboxStore`] is the real client: it trades the refresh
//! token for one short-lived access token per sync.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::parse::{drive_id, utc_ms_to_local};
use crate::constants::env_vars;

const API_URL: &str = "https://api.dropboxapi.com";
const CONTENT_URL: &str = "https://content.dropboxapi.com";
/// Where Fuelio writes its route backups in a "Full Dropbox" app.
pub const DEFAULT_FOLDER: &str = "/Apps/Fuelio/routes";
/// Dropbox caps a page at about 2000 entries.
const PAGE_LIMIT: u32 = 2000;
/// Downloads at the same time. A route file is a few KB.
const PARALLEL_DOWNLOADS: usize = 8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The Dropbox app and the user's grant. All three secrets are required.
#[derive(Debug, Clone)]
pub struct DropboxConfig {
    pub app_key: String,
    pub app_secret: String,
    pub refresh_token: String,
    /// The Dropbox folder with the route files, without a trailing slash.
    pub folder: String,
}

impl DropboxConfig {
    /// `None` when a secret is unset or blank. Pure: `lookup` stands in for
    /// `std::env::var`.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let get = |k: &str| lookup(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        Some(Self {
            app_key: get(env_vars::DROPBOX_APP_KEY)?,
            app_secret: get(env_vars::DROPBOX_APP_SECRET)?,
            refresh_token: get(env_vars::DROPBOX_REFRESH_TOKEN)?,
            folder: get(env_vars::FUELIO_DROPBOX_FOLDER)
                .map(|f| f.trim_end_matches('/').to_string())
                .unwrap_or_else(|| DEFAULT_FOLDER.to_string()),
        })
    }

    pub fn from_env() -> Option<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
}

/// The two things the sync needs from a file store.
#[async_trait::async_trait]
pub trait RouteStore: Send + Sync {
    /// The names of every file in the routes folder.
    async fn list_names(&self) -> Result<Vec<String>, String>;
    /// The content of one file of the routes folder.
    async fn download(&self, name: &str) -> Result<Vec<u8>, String>;
}

/// Dropbox over HTTP, with one access token.
pub struct DropboxStore {
    folder: String,
    api_url: String,
    content_url: String,
    token: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct ListPage {
    entries: Vec<Entry>,
    cursor: String,
    has_more: bool,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = ".tag")]
    tag: String,
    name: String,
}

async fn check(resp: reqwest::Response, what: &str) -> Result<reqwest::Response, String> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(format!("Dropbox {what}: HTTP {}: {}", status.as_u16(), body.chars().take(300).collect::<String>()))
}

impl DropboxStore {
    /// Get an access token from the refresh token.
    pub async fn connect(config: DropboxConfig) -> Result<Self, String> {
        Self::connect_at(config, API_URL, CONTENT_URL).await
    }

    /// [`Self::connect`] against other base URLs (tests).
    pub async fn connect_at(config: DropboxConfig, api_url: &str, content_url: &str) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("Could not create an HTTP client for Dropbox: {e}"))?;
        let api_url = api_url.trim_end_matches('/').to_string();
        let resp = client
            .post(format!("{api_url}/oauth2/token"))
            .basic_auth(&config.app_key, Some(&config.app_secret))
            .form(&[("grant_type", "refresh_token"), ("refresh_token", config.refresh_token.as_str())])
            .send()
            .await
            .map_err(|e| format!("Dropbox token: {e}"))?;
        let token: TokenResponse = check(resp, "token")
            .await?
            .json()
            .await
            .map_err(|e| format!("Dropbox token: {e}"))?;
        Ok(Self {
            folder: config.folder,
            api_url,
            content_url: content_url.trim_end_matches('/').to_string(),
            token: token.access_token,
            client,
        })
    }

    async fn page(&self, endpoint: &str, body: serde_json::Value) -> Result<ListPage, String> {
        let resp = self
            .client
            .post(format!("{}/2/files/{endpoint}", self.api_url))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Dropbox {endpoint}: {e}"))?;
        check(resp, endpoint)
            .await?
            .json()
            .await
            .map_err(|e| format!("Dropbox {endpoint}: {e}"))
    }
}

#[async_trait::async_trait]
impl RouteStore for DropboxStore {
    async fn list_names(&self) -> Result<Vec<String>, String> {
        let mut page = self
            .page("list_folder", serde_json::json!({ "path": self.folder, "limit": PAGE_LIMIT }))
            .await?;
        let mut names = Vec::new();
        loop {
            names.extend(page.entries.into_iter().filter(|e| e.tag == "file").map(|e| e.name));
            if !page.has_more {
                return Ok(names);
            }
            page = self
                .page("list_folder/continue", serde_json::json!({ "cursor": page.cursor }))
                .await?;
        }
    }

    async fn download(&self, name: &str) -> Result<Vec<u8>, String> {
        let arg = serde_json::json!({ "path": format!("{}/{name}", self.folder) }).to_string();
        let resp = self
            .client
            .post(format!("{}/2/files/download", self.content_url))
            .bearer_auth(&self.token)
            .header("Dropbox-API-Arg", arg)
            .send()
            .await
            .map_err(|e| format!("Dropbox download {name}: {e}"))?;
        let bytes = check(resp, "download")
            .await?
            .bytes()
            .await
            .map_err(|e| format!("Dropbox download {name}: {e}"))?;
        Ok(bytes.to_vec())
    }
}

/// What one sync did.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub year: i32,
    /// Drives of the year in Dropbox.
    pub in_dropbox: usize,
    pub downloaded: usize,
    pub already_local: usize,
    /// File names whose download failed.
    pub failed: Vec<String>,
}

/// Download the `route-<id>.data` files of `year` that `dir` does not have.
/// The year is the local start year of the drive, as on the page. A file is
/// written to a temporary name first, so a failed download leaves nothing.
pub async fn sync_year(store: Arc<dyn RouteStore>, dir: &Path, year: i32) -> Result<SyncReport, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let wanted: Vec<String> = store
        .list_names()
        .await?
        .into_iter()
        .filter(|name| {
            drive_id(name)
                .filter(|id| name == &format!("route-{id}.data"))
                .and_then(|id| id.parse::<i64>().ok())
                .is_some_and(|ms| utc_ms_to_local(ms).year() == year)
        })
        .collect();
    let (have, missing): (Vec<_>, Vec<_>) = wanted.iter().cloned().partition(|n| dir.join(n).exists());

    let mut downloaded = 0;
    let mut failed = Vec::new();
    for batch in missing.chunks(PARALLEL_DOWNLOADS) {
        let mut tasks = tokio::task::JoinSet::new();
        for name in batch {
            let (store, name) = (store.clone(), name.clone());
            tasks.spawn(async move {
                let result = store.download(&name).await;
                (name, result)
            });
        }
        while let Some(joined) = tasks.join_next().await {
            let (name, result) = joined.map_err(|e| format!("Download task failed: {e}"))?;
            let written = result.and_then(|bytes| {
                let tmp = dir.join(format!(".{name}.part"));
                std::fs::write(&tmp, bytes)
                    .and_then(|_| std::fs::rename(&tmp, dir.join(&name)))
                    .map_err(|e| {
                        let _ = std::fs::remove_file(&tmp);
                        e.to_string()
                    })
            });
            match written {
                Ok(()) => downloaded += 1,
                Err(e) => {
                    log::warn!("fuelio dropbox: {name}: {e}");
                    failed.push(name);
                }
            }
        }
    }
    failed.sort();
    Ok(SyncReport {
        year,
        in_dropbox: wanted.len(),
        downloaded,
        already_local: have.len(),
        failed,
    })
}

#[cfg(test)]
#[path = "dropbox_tests.rs"]
mod tests;
