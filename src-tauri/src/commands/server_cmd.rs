use crate::app_state::AppState;
use crate::commands::get_app_data_dir;
use crate::server::manager::{ServerManager, ServerStatus};
use crate::server::HttpServer;
use crate::settings::LocalSettings;
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
pub fn get_server_status(manager: State<Arc<ServerManager>>) -> Result<ServerStatus, String> {
    Ok(manager.status())
}

#[tauri::command]
pub async fn start_server(
    app: tauri::AppHandle,
    manager: State<'_, Arc<ServerManager>>,
    app_state: State<'_, AppState>,
    port: u16,
) -> Result<ServerStatus, String> {
    let app_dir = get_app_data_dir(&app)?;

    // Resolve static dir for serving frontend
    let static_dir = if cfg!(debug_assertions) {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build")
    } else {
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|e: tauri::Error| e.to_string())?;
        resource_dir.join("_up_")
    };

    // Create a new Database connection to the same file for the server
    let db_path = app_state
        .get_db_path()
        .ok_or("No database path configured")?;
    let db_arc = Arc::new(
        crate::db::Database::new(db_path).map_err(|e| format!("Failed to open database: {e}"))?,
    );

    // Create a new AppState for the server, copying read-only state
    let server_app_state = Arc::new(AppState::new());
    if app_state.is_read_only() {
        server_app_state.enable_read_only(
            &app_state
                .get_read_only_reason()
                .unwrap_or_else(|| "Unknown reason".to_string()),
        );
    }

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let _addr = HttpServer::start(
        db_arc,
        server_app_state,
        app_dir.clone(),
        static_dir,
        port,
        true,
        shutdown_rx,
    )
    .await?;

    let local_ip = local_ip_address::local_ip()
        .map(|ip| format!("http://{}:{}", ip, port))
        .unwrap_or_else(|_| format!("http://localhost:{}", port));

    manager.set_running(port, local_ip.clone(), shutdown_tx);

    // Save enabled state
    let mut settings = LocalSettings::load(&app_dir);
    settings.server_enabled = Some(true);
    settings.server_port = Some(port);
    settings.save(&app_dir).map_err(|e| e.to_string())?;

    Ok(ServerStatus {
        running: true,
        port: Some(port),
        url: Some(local_ip),
    })
}

#[tauri::command]
pub fn stop_server(
    app: tauri::AppHandle,
    manager: State<Arc<ServerManager>>,
) -> Result<(), String> {
    manager.stop()?;

    // Save disabled state
    let app_dir = get_app_data_dir(&app)?;
    let mut settings = LocalSettings::load(&app_dir);
    settings.server_enabled = Some(false);
    settings.save(&app_dir).map_err(|e| e.to_string())?;

    Ok(())
}
