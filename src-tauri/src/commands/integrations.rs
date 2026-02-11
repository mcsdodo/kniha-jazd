//! Integration Commands
//!
//! Commands for integrating with external services like Home Assistant.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

use crate::check_read_only;
use crate::constants::mime_types;
use crate::models::SuggestedFillup;
use crate::settings::LocalSettings;

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

#[tauri::command]
pub fn get_ha_settings(app_handle: tauri::AppHandle) -> Result<HaSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(HaSettingsResponse {
        url: settings.ha_url,
        has_token: settings.ha_api_token.is_some(),
    })
}

/// Get HA settings including token for frontend to make API calls.
/// This is needed because the frontend needs the token to call HA directly.
#[tauri::command]
pub fn get_local_settings_for_ha(
    app_handle: tauri::AppHandle,
) -> Result<HaLocalSettingsResponse, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let settings = LocalSettings::load(&app_data_dir);
    Ok(HaLocalSettingsResponse {
        ha_url: settings.ha_url,
        ha_api_token: settings.ha_api_token,
    })
}

/// Test HA connection from backend (avoids CORS issues in dev mode)
#[tauri::command]
pub async fn test_ha_connection(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    println!("[HA test] Loading settings from: {:?}", app_data_dir);
    let settings = LocalSettings::load(&app_data_dir);
    println!(
        "[HA test] ha_url: {:?}, has_token: {}",
        settings.ha_url,
        settings.ha_api_token.is_some()
    );

    let url = settings.ha_url.ok_or("HA URL not configured")?;
    let token = settings.ha_api_token.ok_or("HA token not configured")?;

    let api_url = format!("{}/api/", url.trim_end_matches('/'));
    println!("[HA test] Testing: {}", api_url);

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
        .map_err(|e| {
            println!("[HA test] Error: {}", e);
            e.to_string()
        })?;

    let is_ok = response.status().is_success();
    println!(
        "[HA test] Response: {} ({})",
        response.status(),
        if is_ok { "OK" } else { "FAILED" }
    );
    Ok(is_ok)
}

/// Fetch ODO value from Home Assistant for a specific sensor
#[tauri::command]
pub async fn fetch_ha_odo(
    app_handle: tauri::AppHandle,
    sensor_id: String,
) -> Result<Option<f64>, String> {
    println!("[HA ODO] Fetching sensor: {}", sensor_id);
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let settings = LocalSettings::load(&app_data_dir);

    let url = match settings.ha_url {
        Some(u) => u,
        None => {
            println!("[HA ODO] No URL configured");
            return Ok(None);
        }
    };
    let token = match settings.ha_api_token {
        Some(t) => t,
        None => {
            println!("[HA ODO] No token configured");
            return Ok(None);
        }
    };

    let api_url = format!("{}/api/states/{}", url.trim_end_matches('/'), sensor_id);
    println!("[HA ODO] Calling: {}", api_url);

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
        .map_err(|e| {
            println!("[HA ODO] Request error: {}", e);
            e.to_string()
        })?;

    println!("[HA ODO] Response status: {}", response.status());
    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    // HA returns { state: "12345.6", ... }
    let state = data.get("state").and_then(|s| s.as_str());
    println!("[HA ODO] State value: {:?}", state);

    match state {
        Some(s) if s != "unavailable" && s != "unknown" => {
            let value = s.parse::<f64>().ok();
            println!("[HA ODO] Parsed value: {:?}", value);
            Ok(value)
        }
        _ => Ok(None),
    }
}

#[tauri::command]
pub fn save_ha_settings(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
    url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_data_dir = get_app_data_dir(&app_handle)?;

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

    let mut settings = LocalSettings::load(&app_data_dir);

    // Update URL (allow clearing with empty string, keep existing if None)
    if let Some(u) = url {
        settings.ha_url = if u.is_empty() { None } else { Some(u) };
    }

    // Update token only if explicitly provided (None = keep existing)
    // Empty string = clear token, Some(value) = set new token
    if let Some(t) = token {
        settings.ha_api_token = if t.is_empty() { None } else { Some(t) };
    }

    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

// ============================================================================
// Home Assistant Sensor Push
// ============================================================================

/// Format suggested fillup for HA sensor state.
/// Returns "20.39 L → 5.66 l/100km" or "" if no suggestion.
pub(crate) fn format_suggested_fillup_text(suggestion: Option<&SuggestedFillup>) -> String {
    match suggestion {
        Some(s) => format!("{:.2} L → {:.2} l/100km", s.liters, s.consumption_rate),
        None => String::new(),
    }
}

/// Push a state value to a Home Assistant sensor entity.
/// Fire-and-forget: logs errors but never fails the caller.
pub(crate) async fn push_ha_sensor_state(app_data_dir: PathBuf, sensor_id: String, state: String) {
    let settings = LocalSettings::load(&app_data_dir);

    let url = match settings.ha_url {
        Some(u) => u,
        None => return,
    };
    let token = match settings.ha_api_token {
        Some(t) => t,
        None => return,
    };

    let api_url = format!("{}/api/states/{}", url.trim_end_matches('/'), sensor_id);

    let body = serde_json::json!({
        "state": state,
        "attributes": {
            "friendly_name": "Kniha jázd - Návrh tankovania",
            "icon": "mdi:gas-station"
        }
    });

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("[HA push] Failed to build client: {}", e);
            return;
        }
    };

    match client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", mime_types::JSON)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            println!(
                "[HA push] {} → {} ({})",
                sensor_id,
                if state.is_empty() { "\"\"" } else { &state },
                resp.status()
            );
        }
        Err(e) => {
            println!("[HA push] Error pushing to {}: {}", sensor_id, e);
        }
    }
}
