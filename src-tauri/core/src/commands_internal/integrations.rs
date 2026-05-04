//! Home Assistant integration command implementations (framework-free).
//!
//! Pure logic for the HA integration commands. Helpers consumed only by
//! Tauri-flavored callers (e.g. `format_suggested_fillup_text`,
//! `push_ha_input_text`) intentionally remain in the desktop crate's
//! `commands::integrations` module.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::constants::mime_types;
use crate::models::SuggestedFillup;
use crate::settings::LocalSettings;

/// Format suggested fillup for HA input_text helper.
/// Returns "20.39 L → 5.66 l/100km" or "Plná nádrž" if no suggestion needed.
pub fn format_suggested_fillup_text(suggestion: Option<&SuggestedFillup>) -> String {
    match suggestion {
        Some(s) => format!("{:.2} L → {:.2} l/100km", s.liters, s.consumption_rate),
        None => "Plná nádrž".to_string(),
    }
}

// ============================================================================
// Home Assistant Settings
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

// ============================================================================
// Paperless-ngx Settings
// ============================================================================

// Paperless settings response - hides token (mirrors HaSettingsResponse pattern)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperlessSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
}

pub fn get_paperless_settings_internal(app_dir: &Path) -> Result<PaperlessSettingsResponse, String> {
    let settings = LocalSettings::load(app_dir);
    Ok(PaperlessSettingsResponse {
        url: settings.paperless_url,
        has_token: settings
            .paperless_api_token
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty()),
    })
}

pub fn save_paperless_settings_internal(
    app_dir: &Path,
    app_state: &AppState,
    url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);
    if let Some(ref url_str) = url {
        if !url_str.is_empty() {
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
            if url::Url::parse(url_str).is_err() {
                return Err("Invalid URL format".to_string());
            }
        }
    }
    let mut settings = LocalSettings::load(app_dir);
    if let Some(u) = url {
        let u = u.trim().to_string();
        settings.paperless_url = if u.is_empty() { None } else { Some(u) };
    }
    if let Some(t) = token {
        let t = t.trim().to_string();
        settings.paperless_api_token = if t.is_empty() { None } else { Some(t) };
    }
    settings.save(app_dir).map_err(|e| e.to_string())
}

/// Test Paperless-ngx connection. Auth header is `Token <PAT>` (DRF), NOT Bearer.
pub async fn test_paperless_connection_internal(app_dir: &Path) -> Result<bool, String> {
    let settings = LocalSettings::load(app_dir);
    let (url, token) = match (settings.paperless_url, settings.paperless_api_token) {
        (Some(u), Some(t)) if !u.is_empty() && !t.is_empty() => (u, t),
        _ => return Ok(false),
    };
    let api_url = format!("{}/api/ui_settings/", url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build().map_err(|e| e.to_string())?;

    let response = client.get(&api_url)
        .header("Authorization", format!("Token {}", token))
        .header("Accept", "application/json")
        .send().await
        .map_err(|e| e.to_string())?;

    Ok(response.status().is_success())
}

/// Single source of truth for "are we in Paperless mode?" — frontend never inspects raw settings (ADR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InvoiceSourceMode {
    Local,
    Paperless,
}

pub fn get_invoice_source_mode_from_settings(s: &LocalSettings) -> InvoiceSourceMode {
    let enabled = s.paperless_enabled.unwrap_or(true);
    match (&s.paperless_url, &s.paperless_api_token) {
        (Some(u), Some(t)) if enabled && !u.trim().is_empty() && !t.trim().is_empty() => {
            InvoiceSourceMode::Paperless
        }
        _ => InvoiceSourceMode::Local,
    }
}

pub fn get_invoice_source_mode_internal(app_dir: &Path) -> Result<InvoiceSourceMode, String> {
    Ok(get_invoice_source_mode_from_settings(&LocalSettings::load(app_dir)))
}

#[cfg(test)]
#[path = "integrations_tests.rs"]
mod tests;
