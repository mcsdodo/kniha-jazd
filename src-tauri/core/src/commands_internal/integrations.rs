//! Home Assistant integration command implementations (framework-free).
//!
//! Pure logic for the HA integration commands, including the suggested-fillup
//! push. The push lives here rather than in a caller so the server's async RPC
//! dispatcher performs it on every `get_trip_grid_data` (see ADR-024 — the
//! server is the canonical deployment).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::app_state::AppState;
use crate::check_read_only;
use crate::constants::mime_types;
use crate::models::{SuggestedFillup, TripGridData, Vehicle};
use crate::settings::{env_vars, LocalSettings};

/// Format suggested fillup for HA input_text helper.
/// Returns "20.39 L → 5.66 l/100km" or "Plná nádrž" if no suggestion needed.
pub fn format_suggested_fillup_text(suggestion: Option<&SuggestedFillup>) -> String {
    match suggestion {
        Some(s) => format!("{:.2} L → {:.2} l/100km", s.liters, s.consumption_rate),
        None => "Plná nádrž".to_string(),
    }
}

/// Decide whether a trip-grid refresh should push to HA, and what to send.
///
/// Returns `(entity_id, value)` when the vehicle has an `input_text` helper
/// configured, `None` otherwise. Split out from the push itself so both call
/// sites share one rule and it can be tested without a network.
pub fn ha_fillup_push_payload(
    vehicle: &Vehicle,
    grid_data: &TripGridData,
) -> Option<(String, String)> {
    let entity_id = vehicle.ha_fillup_sensor.clone()?;
    if entity_id.trim().is_empty() {
        return None;
    }
    Some((
        entity_id,
        format_suggested_fillup_text(grid_data.legend_suggested_fillup.as_ref()),
    ))
}

/// Push a value to a Home Assistant `input_text` helper entity.
/// Uses the `input_text/set_value` service call so the value persists across HA restarts.
/// Fire-and-forget: logs errors but never fails the caller.
pub async fn push_ha_input_text(app_data_dir: PathBuf, entity_id: String, value: String) {
    let settings = LocalSettings::load_effective(&app_data_dir);

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
// Home Assistant Settings
// ============================================================================

/// Response for get_ha_settings.
///
/// Carries NO secret: the token is reported only as `has_token`. Revealing the
/// value goes through `reveal_secret`, which demands a PIN over the network
/// (task 69 / ADR-027).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
    pub url_from_env: bool,
    pub token_from_env: bool,
}

// HaLocalSettingsResponse / get_local_settings_for_ha deleted in task 69: it
// handed the full HA token to any RPC caller so the frontend could call HA
// directly, but the frontend goes through test_ha_connection / fetch_ha_odo
// instead and nothing referenced it.

pub fn get_ha_settings_internal(app_dir: &Path) -> Result<HaSettingsResponse, String> {
    let settings = LocalSettings::load_effective(app_dir);
    Ok(HaSettingsResponse {
        has_token: settings.ha_api_token.is_some(),
        url_from_env: LocalSettings::env_pinned(env_vars::HA_URL),
        token_from_env: LocalSettings::env_pinned(env_vars::HA_API_TOKEN),
        url: settings.ha_url,
    })
}

/// Test HA connection from backend (avoids CORS issues in dev mode).
/// Returns Ok(false) silently when HA isn't configured — that's a normal state,
/// not an error worth surfacing to logs or callers.
pub async fn test_ha_connection_internal(app_dir: &Path) -> Result<bool, String> {
    let settings = LocalSettings::load_effective(app_dir);

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
    let settings = LocalSettings::load_effective(app_dir);

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

    // Env-pinned fields are managed outside the app — refuse only when the
    // call actually attempts to change them.
    if url.is_some() && LocalSettings::env_pinned(env_vars::HA_URL) {
        return Err(
            "Home Assistant URL is managed by the HA_URL environment variable".to_string(),
        );
    }
    if token.is_some() && LocalSettings::env_pinned(env_vars::HA_API_TOKEN) {
        return Err(
            "Home Assistant token is managed by the HA_API_TOKEN environment variable".to_string(),
        );
    }

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
    pub enabled: bool,
    // Resolved custom field names (defaults applied when settings are None/empty)
    pub field_name_datetime: String,
    pub field_name_liters: String,
    pub field_name_total: String,
    // Env-var pinning — see HaSettingsResponse. Carries no secret either.
    pub url_from_env: bool,
    pub token_from_env: bool,
    pub enabled_from_env: bool,
}

pub fn get_paperless_settings_internal(app_dir: &Path) -> Result<PaperlessSettingsResponse, String> {
    use crate::paperless::PaperlessFieldNames;

    let settings = LocalSettings::load_effective(app_dir);
    let names = PaperlessFieldNames::from_settings(&settings);
    let enabled = settings.paperless_enabled.unwrap_or(true);
    Ok(PaperlessSettingsResponse {
        url: settings.paperless_url,
        has_token: settings
            .paperless_api_token
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty()),
        enabled,
        field_name_datetime: names.datetime,
        field_name_liters: names.liters,
        field_name_total: names.total,
        url_from_env: LocalSettings::env_pinned(env_vars::PAPERLESS_URL),
        token_from_env: LocalSettings::env_pinned(env_vars::PAPERLESS_API_TOKEN),
        enabled_from_env: LocalSettings::env_pinned(env_vars::PAPERLESS_ENABLED),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn save_paperless_settings_internal(
    app_dir: &Path,
    app_state: &AppState,
    url: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
    field_name_datetime: Option<String>,
    field_name_liters: Option<String>,
    field_name_total: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);

    // Env-pinned fields are managed outside the app — refuse only when the
    // call actually attempts to change them. Field-name overrides stay editable.
    if url.is_some() && LocalSettings::env_pinned(env_vars::PAPERLESS_URL) {
        return Err(
            "Paperless URL is managed by the PAPERLESS_URL environment variable".to_string(),
        );
    }
    if token.is_some() && LocalSettings::env_pinned(env_vars::PAPERLESS_API_TOKEN) {
        return Err(
            "Paperless token is managed by the PAPERLESS_API_TOKEN environment variable"
                .to_string(),
        );
    }
    if enabled.is_some() && LocalSettings::env_pinned(env_vars::PAPERLESS_ENABLED) {
        return Err(
            "Paperless enabled flag is managed by the PAPERLESS_ENABLED environment variable"
                .to_string(),
        );
    }

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
    if let Some(e) = enabled {
        settings.paperless_enabled = Some(e);
    }
    // Field-name overrides — empty string clears (= use default), None keeps existing
    if let Some(v) = field_name_datetime {
        let v = v.trim().to_string();
        settings.paperless_field_name_datetime = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = field_name_liters {
        let v = v.trim().to_string();
        settings.paperless_field_name_liters = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = field_name_total {
        let v = v.trim().to_string();
        settings.paperless_field_name_total = if v.is_empty() { None } else { Some(v) };
    }
    settings.save(app_dir).map_err(|e| e.to_string())
}

/// Test Paperless-ngx connection. Auth header is `Token <PAT>` (DRF), NOT Bearer.
pub async fn test_paperless_connection_internal(app_dir: &Path) -> Result<bool, String> {
    let settings = LocalSettings::load_effective(app_dir);
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
    Ok(get_invoice_source_mode_from_settings(&LocalSettings::load_effective(app_dir)))
}

#[cfg(test)]
#[path = "integrations_tests.rs"]
mod tests;
