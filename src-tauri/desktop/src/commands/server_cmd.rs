use crate::app_state::AppState;
use crate::commands::get_app_data_dir;
use crate::db::Database;
use crate::server::manager::{ServerManager, ServerStatus};
use crate::server::HttpServer;
use crate::settings::LocalSettings;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_server_status(manager: State<Arc<ServerManager>>) -> Result<ServerStatus, String> {
    Ok(manager.status())
}

#[tauri::command]
pub async fn start_server(
    app: tauri::AppHandle,
    manager: State<'_, Arc<ServerManager>>,
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
    port: u16,
) -> Result<ServerStatus, String> {
    let app_dir = get_app_data_dir(&app)?;
    let static_dir = crate::server::resolve_static_dir_from_handle(&app);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let _addr = HttpServer::start(
        db.inner().clone(),
        app_state.inner().clone(),
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
