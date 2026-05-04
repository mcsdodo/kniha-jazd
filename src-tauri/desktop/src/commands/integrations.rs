//! Home Assistant integration Tauri command wrappers.
//!
//! All `_internal` implementations live in
//! [`kniha_jazd_core::commands_internal::integrations`]. The fire-and-forget
//! HA push helpers (`format_suggested_fillup_text`, `push_ha_input_text`) stay
//! in this module because they're consumed only by other desktop wrappers
//! (e.g. `commands::statistics::get_trip_grid_data`).

pub use kniha_jazd_core::commands_internal::integrations::*;

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::integrations as inner;
use kniha_jazd_core::commands_internal::paperless_cmd as paperless_inner;
use kniha_jazd_core::constants::mime_types;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::models::PaperlessInvoiceRow;
use kniha_jazd_core::settings::LocalSettings;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use super::get_app_data_dir;

// ============================================================================
// Home Assistant Settings Commands
// ============================================================================

#[tauri::command]
pub fn get_ha_settings(app_handle: tauri::AppHandle) -> Result<HaSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::get_ha_settings_internal(&app_data_dir)
}

#[tauri::command]
pub fn get_local_settings_for_ha(
    app_handle: tauri::AppHandle,
) -> Result<HaLocalSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::get_local_settings_for_ha_internal(&app_data_dir)
}

#[tauri::command]
pub async fn test_ha_connection(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::test_ha_connection_internal(&app_data_dir).await
}

#[tauri::command]
pub async fn fetch_ha_odo(
    app_handle: tauri::AppHandle,
    sensor_id: String,
) -> Result<Option<f64>, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::fetch_ha_odo_internal(&app_data_dir, sensor_id).await
}

#[tauri::command]
pub fn save_ha_settings(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::save_ha_settings_internal(&app_data_dir, &app_state, url, token)
}

// ============================================================================
// Home Assistant Sensor Push (desktop-only — used by other desktop wrappers)
// ============================================================================

// format_suggested_fillup_text moved to core (called from commands_tests.rs).
// Available via the `pub use kniha_jazd_core::commands_internal::integrations::*` above.

/// Push a value to a Home Assistant `input_text` helper entity.
/// Uses the `input_text/set_value` service call so the value persists across HA restarts.
/// Fire-and-forget: logs errors but never fails the caller.
pub async fn push_ha_input_text(app_data_dir: PathBuf, entity_id: String, value: String) {
    let settings = LocalSettings::load(&app_data_dir);

    let url = match settings.ha_url {
        Some(u) => u,
        None => return,
    };
    let token = match settings.ha_api_token {
        Some(t) => t,
        None => return,
    };

    let api_url = format!(
        "{}/api/services/input_text/set_value",
        url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "entity_id": entity_id,
        "value": value
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("HA push: failed to build client: {}", e);
            return;
        }
    };

    if let Err(e) = client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", mime_types::JSON)
        .json(&body)
        .send()
        .await
    {
        log::warn!("HA push to {}: {}", entity_id, e);
    }
}

// ============================================================================
// Paperless-ngx Settings Commands
// ============================================================================

#[tauri::command]
pub fn get_paperless_settings(
    app_handle: tauri::AppHandle,
) -> Result<PaperlessSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::get_paperless_settings_internal(&app_data_dir)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_paperless_settings(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    url: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
    field_name_datetime: Option<String>,
    field_name_liters: Option<String>,
    field_name_total: Option<String>,
) -> Result<(), String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::save_paperless_settings_internal(
        &app_data_dir,
        &app_state,
        url,
        token,
        enabled,
        field_name_datetime,
        field_name_liters,
        field_name_total,
    )
}

#[tauri::command]
pub async fn test_paperless_connection(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::test_paperless_connection_internal(&app_data_dir).await
}

#[tauri::command]
pub fn get_invoice_source_mode(
    app_handle: tauri::AppHandle,
) -> Result<InvoiceSourceMode, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::get_invoice_source_mode_internal(&app_data_dir)
}

// ============================================================================
// Paperless-ngx Invoice / Trip Assignment Commands
// ============================================================================

#[tauri::command]
pub async fn get_paperless_invoices(
    app_handle: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<PaperlessInvoiceRow>, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    paperless_inner::get_paperless_invoices_internal(&app_data_dir, &db, &vehicle_id, year)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn assign_paperless_doc_to_trip(
    app_state: State<'_, Arc<AppState>>,
    db: State<'_, Arc<Database>>,
    doc_id: i64,
    trip_id: String,
) -> Result<(), String> {
    paperless_inner::assign_paperless_doc_to_trip_internal(&app_state, &db, doc_id, &trip_id)
}

#[tauri::command]
pub fn unassign_paperless_doc(
    app_state: State<'_, Arc<AppState>>,
    db: State<'_, Arc<Database>>,
    doc_id: i64,
) -> Result<(), String> {
    paperless_inner::unassign_paperless_doc_internal(&app_state, &db, doc_id)
}
