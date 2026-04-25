//! Standalone web server binary.
//!
//! Runs the Kniha Jázd HTTP server without the Tauri UI shell.
//! Used for Docker deployment and headless server scenarios.
//!
//! Configuration via environment variables:
//! - `PORT` (default: 3456) — HTTP listen port
//! - `KNIHA_JAZD_DATA_DIR` (default: `/data`) — directory for DB, receipts, backups
//! - `DATABASE_PATH` (default: `<DATA_DIR>/kniha-jazd.db`) — SQLite DB file
//! - `STATIC_DIR` (default: `/var/www/html`) — built SvelteKit assets

use app_lib::app_state::AppState;
use app_lib::db::Database;
use app_lib::server::HttpServer;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3456);

    let data_dir = std::env::var("KNIHA_JAZD_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/data"));

    let db_path = std::env::var("DATABASE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_dir.join("kniha-jazd.db"));

    let static_dir = std::env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/www/html"));

    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    let db = Arc::new(Database::new(db_path).expect("Failed to open database"));
    let app_state = Arc::new(AppState::new());

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let addr = HttpServer::start(
            db,
            app_state,
            data_dir,
            static_dir,
            port,
            true,
            shutdown_rx,
        )
        .await
        .expect("Failed to start server");

        println!("Kniha Jázd server running at http://{addr}");
        println!("Press Ctrl+C to stop.");

        wait_for_shutdown_signal().await;

        println!("Shutting down...");
        let _ = shutdown_tx.send(());
    });
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM (Docker stop, systemd) to trigger graceful shutdown.
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => {},
            _ = sigint.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
    }
}
