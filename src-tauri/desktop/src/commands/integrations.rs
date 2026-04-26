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
use kniha_jazd_core::constants::mime_types;
use kniha_jazd_core::models::SuggestedFillup;
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

/// Format suggested fillup for HA input_text helper.
/// Returns "20.39 L → 5.66 l/100km" or "Plná nádrž" if no suggestion needed.
pub fn format_suggested_fillup_text(suggestion: Option<&SuggestedFillup>) -> String {
    match suggestion {
        Some(s) => format!("{:.2} L → {:.2} l/100km", s.liters, s.consumption_rate),
        None => "Plná nádrž".to_string(),
    }
}

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
