//! Home Assistant integration Tauri command wrappers.
//!
//! All `_internal` implementations live in
//! [`kniha_jazd_core::commands_internal::integrations`], including the
//! fire-and-forget HA push (`ha_fillup_push_payload`, `push_ha_input_text`) —
//! the server's async dispatcher has to perform the identical push, so it
//! cannot live here. Re-exported below via the glob `pub use`.

pub use kniha_jazd_core::commands_internal::integrations::*;

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::integrations as inner;
use kniha_jazd_core::commands_internal::paperless_cmd as paperless_inner;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::models::PaperlessInvoiceRow;
use kniha_jazd_core::paperless::CustomFieldInfo;
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

// Home Assistant sensor push (format_suggested_fillup_text, ha_fillup_push_payload,
// push_ha_input_text) moved to core so the server shares it — available via the
// `pub use kniha_jazd_core::commands_internal::integrations::*` above.

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
pub async fn list_paperless_custom_fields(
    app_handle: tauri::AppHandle,
) -> Result<Vec<CustomFieldInfo>, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    paperless_inner::list_paperless_custom_fields_internal(&app_data_dir)
        .await
        .map_err(|e| e.to_string())
}

