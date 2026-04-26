//! Integration Commands
//!
//! Commands for integrating with external services like Home Assistant.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::check_read_only;
use crate::constants::mime_types;
use crate::models::SuggestedFillup;
use crate::settings::LocalSettings;

use std::path::Path;

use super::{get_app_data_dir, AppState};

// ============================================================================
// Home Assistant Settings Commands
// ============================================================================

/// Response for get_ha_settings - hides token for security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
}

/// Response for get_local_settings_for_ha - includes token for frontend API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaLocalSettingsResponse {
    pub ha_url: Option<String>,
    pub ha_api_token: Option<String>,
}

pub fn get_ha_settings_internal(app_dir: &Path) -> Result<HaSettingsResponse, String> {
    let settings = LocalSettings::load(app_dir);
    Ok(HaSettingsResponse {
        url: settings.ha_url,
        has_token: settings.ha_api_token.is_some(),
    })
}

#[tauri::command]
pub fn get_ha_settings(app_handle: tauri::AppHandle) -> Result<HaSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    get_ha_settings_internal(&app_data_dir)
}

/// Get HA settings including token for frontend to make API calls.
/// This is needed because the frontend needs the token to call HA directly.
pub fn get_local_settings_for_ha_internal(
    app_dir: &Path,
) -> Result<HaLocalSettingsResponse, String> {
    let settings = LocalSettings::load(app_dir);
    Ok(HaLocalSettingsResponse {
        ha_url: settings.ha_url,
        ha_api_token: settings.ha_api_token,
    })
}

#[tauri::command]
pub fn get_local_settings_for_ha(
    app_handle: tauri::AppHandle,
) -> Result<HaLocalSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    get_local_settings_for_ha_internal(&app_data_dir)
}

/// Test HA connection from backend (avoids CORS issues in dev mode).
/// Returns Ok(false) silently when HA isn't configured — that's a normal state,
/// not an error worth surfacing to logs or callers.
pub async fn test_ha_connection_internal(app_dir: &Path) -> Result<bool, String> {
    let settings = LocalSettings::load(app_dir);

    let (url, token) = match (settings.ha_url, settings.ha_api_token) {
        (Some(url), Some(token)) => (url, token),
        _ => return Ok(false),
    };

    let api_url = format!("{}/api/", url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&api_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", mime_types::JSON)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(response.status().is_success())
}

#[tauri::command]
pub async fn test_ha_connection(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    test_ha_connection_internal(&app_data_dir).await
}

/// Fetch ODO value from Home Assistant for a specific sensor.
/// Returns Ok(None) silently when HA isn't configured — that's a normal state.
pub async fn fetch_ha_odo_internal(
    app_dir: &Path,
    sensor_id: String,
) -> Result<Option<f64>, String> {
    let settings = LocalSettings::load(app_dir);

    let (url, token) = match (settings.ha_url, settings.ha_api_token) {
        (Some(url), Some(token)) => (url, token),
        _ => return Ok(None),
    };

    let api_url = format!("{}/api/states/{}", url.trim_end_matches('/'), sensor_id);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&api_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", mime_types::JSON)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let state = data.get("state").and_then(|s| s.as_str());
    match state {
        Some(s) if s != "unavailable" && s != "unknown" => Ok(s.parse::<f64>().ok()),
        _ => Ok(None),
    }
}

#[tauri::command]
pub async fn fetch_ha_odo(
    app_handle: tauri::AppHandle,
    sensor_id: String,
) -> Result<Option<f64>, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    fetch_ha_odo_internal(&app_data_dir, sensor_id).await
}

pub fn save_ha_settings_internal(
    app_dir: &Path,
    app_state: &AppState,
    url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);

    // Validate URL if provided
    if let Some(ref url_str) = url {
        if !url_str.is_empty() {
            // Must start with http:// or https://
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
            // Basic URL validation
            if url::Url::parse(url_str).is_err() {
                return Err("Invalid URL format".to_string());
            }
        }
    }

    let mut settings = LocalSettings::load(app_dir);

    // Update URL (allow clearing with empty string, keep existing if None)
    if let Some(u) = url {
        settings.ha_url = if u.is_empty() { None } else { Some(u) };
    }

    // Update token only if explicitly provided (None = keep existing)
    // Empty string = clear token, Some(value) = set new token
    if let Some(t) = token {
        settings.ha_api_token = if t.is_empty() { None } else { Some(t) };
    }

    settings.save(app_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_ha_settings(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    save_ha_settings_internal(&app_data_dir, &app_state, url, token)
}

// ============================================================================
// Home Assistant Sensor Push
// ============================================================================

/// Format suggested fillup for HA input_text helper.
/// Returns "20.39 L → 5.66 l/100km" or "Plná nádrž" if no suggestion needed.
pub(crate) fn format_suggested_fillup_text(suggestion: Option<&SuggestedFillup>) -> String {
    match suggestion {
        Some(s) => format!("{:.2} L → {:.2} l/100km", s.liters, s.consumption_rate),
        None => "Plná nádrž".to_string(),
    }
}

/// Push a value to a Home Assistant `input_text` helper entity.
/// Uses the `input_text/set_value` service call so the value persists across HA restarts.
/// Fire-and-forget: logs errors but never fails the caller.
pub(crate) async fn push_ha_input_text(app_data_dir: PathBuf, entity_id: String, value: String) {
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
