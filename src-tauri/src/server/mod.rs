//! Embedded HTTP server for LAN browser access.
//!
//! When enabled, serves the same UI and RPC API that the Tauri webview uses,
//! allowing phones/tablets/other PCs to access the app over the local network.

mod dispatcher;
mod dispatcher_async;

use crate::app_state::AppState;
use crate::db::Database;
use axum::{
    extract::{Path as AxumPath, State as AxumState},
    http::{header, HeaderName, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
pub struct ServerState {
    pub db: Arc<Database>,
    pub app_state: Arc<AppState>,
    pub app_dir: PathBuf,
    pub static_dir: PathBuf,
}

// ============================================================================
// RPC Handler
// ============================================================================

#[derive(serde::Deserialize)]
struct RpcRequest {
    command: String,
    args: serde_json::Value,
}

async fn rpc_handler(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse {
    if let Some(result) =
        dispatcher_async::dispatch_async(&req.command, req.args.clone(), &state).await
    {
        return match result {
            Ok(value) => Json(value).into_response(),
            Err(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
        };
    }

    let state_clone = state.clone();
    let command = req.command.clone();
    let args = req.args;

    let result = tokio::task::spawn_blocking(move || {
        dispatcher::dispatch_sync(&command, args, &state_clone)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"));

    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(msg)) => (StatusCode::BAD_REQUEST, msg).into_response(),
        Err(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
    }
}

// ============================================================================
// Capabilities Endpoint
// ============================================================================

async fn capabilities_handler(
    AxumState(state): AxumState<ServerState>,
) -> Json<serde_json::Value> {
    let read_only = state.app_state.is_read_only();
    Json(serde_json::json!({
        "mode": "server",
        "read_only": read_only,
        "features": {
            "file_dialogs": false,
            "updater": false,
            "open_external": false,
            "restore_backup": false,
            "move_database": false,
        }
    }))
}

// ============================================================================
// Receipt Image Endpoint
// ============================================================================

async fn receipt_image_handler(
    AxumState(state): AxumState<ServerState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let db = &state.db;
    let receipt = match db.get_receipt_by_id(&id) {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Receipt not found").into_response(),
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };

    let file_path = &receipt.file_path;

    match tokio::fs::read(file_path).await {
        Ok(bytes) => {
            let content_type = if file_path.ends_with(".png") {
                "image/png"
            } else if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
                "image/jpeg"
            } else if file_path.ends_with(".webp") {
                "image/webp"
            } else {
                "application/octet-stream"
            };
            ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Image file not found on disk").into_response(),
    }
}

// ============================================================================
// CORS — LAN Origins Only
// ============================================================================

fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            let s = origin.to_str().unwrap_or("");
            is_lan_origin(s)
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("x-kj-client"),
        ])
}

fn is_lan_origin(origin: &str) -> bool {
    origin.starts_with("http://localhost")
        || origin.starts_with("http://127.")
        || origin.starts_with("http://10.")
        || origin.starts_with("http://192.168.")
        || is_rfc1918_172(origin)
}

fn is_rfc1918_172(origin: &str) -> bool {
    if let Some(rest) = origin.strip_prefix("http://172.") {
        if let Some(dot_pos) = rest.find('.') {
            if let Ok(second_octet) = rest[..dot_pos].parse::<u8>() {
                return (16..=31).contains(&second_octet);
            }
        }
    }
    false
}

// ============================================================================
// Server
// ============================================================================

pub struct HttpServer;

impl HttpServer {
    pub async fn start(
        db: Arc<Database>,
        app_state: Arc<AppState>,
        app_dir: PathBuf,
        static_dir: PathBuf,
        port: u16,
        shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<SocketAddr, String> {
        let state = ServerState {
            db,
            app_state,
            app_dir,
            static_dir,
        };

        // Build API routes
        let api_router = Router::new()
            .route("/rpc", post(rpc_handler))
            .route("/capabilities", get(capabilities_handler))
            .route("/receipts/{id}/image", get(receipt_image_handler));

        // Build full app with static fallback
        let index_html = state.static_dir.join("index.html");
        let app = if index_html.exists() {
            let static_service = ServeDir::new(&state.static_dir)
                .fallback(ServeFile::new(index_html));

            Router::new()
                .route("/health", get(|| async { "ok" }))
                .nest("/api", api_router)
                .fallback_service(static_service)
                .layer(build_cors_layer())
                .with_state(state)
        } else {
            if state.static_dir.exists() {
                log::info!(
                    "Static directory {:?} has no index.html, SPA fallback disabled",
                    state.static_dir
                );
            } else {
                log::warn!(
                    "Static frontend directory not found at {:?}",
                    state.static_dir
                );
            }
            Router::new()
                .route("/health", get(|| async { "ok" }))
                .nest("/api", api_router)
                .layer(build_cors_layer())
                .with_state(state)
        };

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Failed to bind to {addr}: {e}"))?;
        let bound_addr = listener.local_addr().map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                    log::info!("HTTP server shutting down");
                })
                .await
                .ok();
        });

        log::info!("HTTP server listening on {bound_addr}");
        Ok(bound_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn start_test_server() -> (SocketAddr, oneshot::Sender<()>) {
        let db = Arc::new(crate::db::Database::in_memory().unwrap());
        let app_state = Arc::new(crate::app_state::AppState::new());
        let app_dir = std::env::temp_dir();
        let static_dir = std::env::temp_dir(); // No index.html = no static serving in tests
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let addr = HttpServer::start(db, app_state, app_dir, static_dir, 0, shutdown_rx)
            .await
            .expect("server should start");
        (addr, shutdown_tx)
    }

    #[tokio::test]
    async fn health_endpoint_responds() {
        let (addr, shutdown_tx) = start_test_server().await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{addr}/health"))
            .send()
            .await
            .expect("request should succeed");
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.text().await.unwrap(), "ok");

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn rpc_endpoint_dispatches_command() {
        let (addr, shutdown_tx) = start_test_server().await;

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://{addr}/api/rpc"))
            .json(&serde_json::json!({
                "command": "get_vehicles",
                "args": {}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body, serde_json::json!([]));

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn capabilities_endpoint() {
        let (addr, shutdown_tx) = start_test_server().await;

        let resp = reqwest::get(format!("http://{addr}/api/capabilities"))
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["mode"], "server");
        assert_eq!(body["features"]["file_dialogs"], false);
        assert_eq!(body["features"]["updater"], false);

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn cors_allows_lan_origin() {
        let (addr, shutdown_tx) = start_test_server().await;
        let client = reqwest::Client::new();

        let resp = client
            .post(format!("http://{addr}/api/rpc"))
            .header("Origin", "http://192.168.1.50:3456")
            .header("X-KJ-Client", "1")
            .header("Content-Type", "application/json")
            .body(r#"{"command":"get_vehicles","args":{}}"#)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        assert!(resp.headers().get("access-control-allow-origin").is_some());

        let _ = shutdown_tx.send(());
    }

    #[test]
    fn lan_origin_detection() {
        assert!(is_lan_origin("http://localhost:3456"));
        assert!(is_lan_origin("http://127.0.0.1:3456"));
        assert!(is_lan_origin("http://192.168.1.50:3456"));
        assert!(is_lan_origin("http://10.0.0.1:3456"));
        assert!(is_lan_origin("http://172.16.0.1:3456"));
        assert!(is_lan_origin("http://172.31.255.255:3456"));

        assert!(!is_lan_origin("https://evil.com"));
        assert!(!is_lan_origin("http://172.15.0.1:3456"));
        assert!(!is_lan_origin("http://172.32.0.1:3456"));
        assert!(!is_lan_origin("http://example.com"));
    }

    #[tokio::test]
    async fn spa_fallback_serves_index_html() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("index.html"), "<html>app</html>").unwrap();

        let db = Arc::new(crate::db::Database::in_memory().unwrap());
        let app_state = Arc::new(crate::app_state::AppState::new());
        let app_dir = std::env::temp_dir();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let addr = HttpServer::start(
            db,
            app_state,
            app_dir,
            temp.path().to_path_buf(),
            0,
            shutdown_rx,
        )
        .await
        .unwrap();

        let resp = reqwest::get(format!("http://{addr}/vozidla/some-id"))
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        assert!(resp.text().await.unwrap().contains("<html>app</html>"));

        let _ = shutdown_tx.send(());
    }
}
