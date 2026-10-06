use super::*;
use std::sync::Arc;
use wiremock::matchers::{body_string_contains, header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
}

#[test]
fn config_needs_key_secret_and_refresh_token() {
    let full = [("DROPBOX_APP_KEY", "k"), ("DROPBOX_APP_SECRET", "s"), ("DROPBOX_REFRESH_TOKEN", "r")];
    let c = DropboxConfig::from_lookup(env(&full)).unwrap();
    assert_eq!((c.app_key.as_str(), c.folder.as_str()), ("k", DEFAULT_FOLDER));
    assert!(DropboxConfig::from_lookup(env(&full[..2])).is_none());
    let blank = [("DROPBOX_APP_KEY", "k"), ("DROPBOX_APP_SECRET", " "), ("DROPBOX_REFRESH_TOKEN", "r")];
    assert!(DropboxConfig::from_lookup(env(&blank)).is_none());
}

#[test]
fn config_takes_a_folder_override() {
    let pairs = [
        ("DROPBOX_APP_KEY", "k"),
        ("DROPBOX_APP_SECRET", "s"),
        ("DROPBOX_REFRESH_TOKEN", "r"),
        ("FUELIO_DROPBOX_FOLDER", "/Fuelio/routes/"),
    ];
    assert_eq!(DropboxConfig::from_lookup(env(&pairs)).unwrap().folder, "/Fuelio/routes");
}

fn config() -> DropboxConfig {
    DropboxConfig {
        app_key: "key".into(),
        app_secret: "secret".into(),
        refresh_token: "refresh".into(),
        folder: "/Apps/Fuelio/routes".into(),
    }
}

async fn token_mock(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .and(header_exists("authorization"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "AT", "token_type": "bearer", "expires_in": 14400
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn list_follows_the_cursor_over_all_pages() {
    let server = MockServer::start().await;
    token_mock(&server).await;
    Mock::given(method("POST"))
        .and(path("/2/files/list_folder"))
        .and(header("authorization", "Bearer AT"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entries": [{".tag": "file", "name": "route-1.data"}, {".tag": "folder", "name": "x"}],
            "cursor": "c1", "has_more": true
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/2/files/list_folder/continue"))
        .and(body_string_contains("c1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entries": [{".tag": "file", "name": "route-2.data"}],
            "cursor": "c2", "has_more": false
        })))
        .mount(&server)
        .await;

    let store = DropboxStore::connect_at(config(), &server.uri(), &server.uri()).await.unwrap();
    assert_eq!(store.list_names().await.unwrap(), vec!["route-1.data", "route-2.data"]);
}

#[tokio::test]
async fn download_asks_for_the_file_in_the_folder() {
    let server = MockServer::start().await;
    token_mock(&server).await;
    Mock::given(method("POST"))
        .and(path("/2/files/download"))
        .and(header("authorization", "Bearer AT"))
        .and(header("dropbox-api-arg", r#"{"path":"/Apps/Fuelio/routes/route-1.data"}"#))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PK-zip".to_vec()))
        .mount(&server)
        .await;
    let store = DropboxStore::connect_at(config(), &server.uri(), &server.uri()).await.unwrap();
    assert_eq!(store.download("route-1.data").await.unwrap(), b"PK-zip");
}

#[tokio::test]
async fn a_refused_refresh_token_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#))
        .mount(&server)
        .await;
    let err = DropboxStore::connect_at(config(), &server.uri(), &server.uri())
        .await
        .err()
        .unwrap();
    assert!(err.contains("400"), "{err}");
}

/// A store in memory: names and contents, and names whose download fails.
struct FakeStore {
    files: Vec<(String, Vec<u8>)>,
    broken: Vec<String>,
    downloads: std::sync::Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl RouteStore for FakeStore {
    async fn list_names(&self) -> Result<Vec<String>, String> {
        Ok(self.files.iter().map(|(n, _)| n.clone()).chain(self.broken.clone()).collect())
    }
    async fn download(&self, name: &str) -> Result<Vec<u8>, String> {
        self.downloads.lock().unwrap().push(name.to_string());
        self.files
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.clone())
            .ok_or_else(|| format!("no {name}"))
    }
}

// 2026-09-28 02:30 UTC and 2025-06-01: one drive per year.
const D2026: &str = "route-1790562640107.data";
const D2025: &str = "route-1748736000000.data";

#[tokio::test]
async fn sync_downloads_only_the_missing_drives_of_the_year() {
    let dir = tempfile::tempdir().unwrap();
    let have = "route-1790566554295.data"; // 2026, already local
    std::fs::write(dir.path().join(have), b"old").unwrap();
    let store = Arc::new(FakeStore {
        files: vec![
            (D2026.into(), b"new".to_vec()),
            (have.into(), b"remote".to_vec()),
            (D2025.into(), b"other year".to_vec()),
            ("route-1790562640107.route".into(), b"polyline".to_vec()),
        ],
        broken: vec![],
        downloads: Default::default(),
    });

    let r = sync_year(store.clone(), dir.path(), 2026).await.unwrap();

    assert_eq!((r.in_dropbox, r.downloaded, r.already_local), (2, 1, 1));
    assert!(r.failed.is_empty());
    assert_eq!(*store.downloads.lock().unwrap(), vec![D2026.to_string()]);
    assert_eq!(std::fs::read(dir.path().join(D2026)).unwrap(), b"new");
    assert_eq!(std::fs::read(dir.path().join(have)).unwrap(), b"old", "a local file is kept");
    assert!(!dir.path().join(D2025).exists());
}

#[tokio::test]
async fn sync_reports_a_failed_download_and_keeps_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(FakeStore {
        files: vec![(D2026.into(), b"ok".to_vec())],
        broken: vec!["route-1790566554295.data".into()],
        downloads: Default::default(),
    });
    let r = sync_year(store, dir.path(), 2026).await.unwrap();
    assert_eq!(r.downloaded, 1);
    assert_eq!(r.failed, vec!["route-1790566554295.data".to_string()]);
    assert!(!dir.path().join("route-1790566554295.data").exists());
    // No temporary file stays behind.
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec![D2026.to_string()]);
}

#[tokio::test]
async fn sync_creates_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("fuelio");
    let store = Arc::new(FakeStore { files: vec![(D2026.into(), b"x".to_vec())], broken: vec![], downloads: Default::default() });
    sync_year(store, &target, 2026).await.unwrap();
    assert!(target.join(D2026).exists());
}
