//! Embedded HTTP server for LAN browser access.
//!
//! When enabled, serves the same UI and RPC API that the Tauri webview uses,
//! allowing phones/tablets/other PCs to access the app over the local network.

mod dispatcher;
mod dispatcher_async;

use crate::app_state::AppState;
use crate::db::Database;
use axum::{
    extract::State as AxumState,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct ServerState {
    pub db: Arc<Database>,
    pub app_state: Arc<AppState>,
    pub app_dir: PathBuf,
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
    // Try async commands first
    if let Some(result) =
        dispatcher_async::dispatch_async(&req.command, req.args.clone(), &state).await
    {
        return match result {
            Ok(value) => Json(value).into_response(),
            Err(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
        };
    }

    // Sync commands via spawn_blocking
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
// Server
// ============================================================================

pub struct HttpServer;

impl HttpServer {
    pub async fn start(
        db: Arc<Database>,
        app_state: Arc<AppState>,
        app_dir: PathBuf,
        port: u16,
        shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<SocketAddr, String> {
        let state = ServerState {
            db,
            app_state,
            app_dir,
        };

        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route("/api/rpc", post(rpc_handler))
            .with_state(state);

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

    #[tokio::test]
    async fn health_endpoint_responds() {
        let db = Arc::new(crate::db::Database::in_memory().unwrap());
        let app_state = Arc::new(crate::app_state::AppState::new());
        let app_dir = std::env::temp_dir();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let addr = HttpServer::start(db, app_state, app_dir, 0, shutdown_rx)
            .await
            .expect("server should start");

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
        let db = Arc::new(crate::db::Database::in_memory().unwrap());
        let app_state = Arc::new(crate::app_state::AppState::new());
        let app_dir = std::env::temp_dir();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let addr = HttpServer::start(db, app_state, app_dir, 0, shutdown_rx)
            .await
            .unwrap();

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
}
