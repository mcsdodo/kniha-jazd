//! Local settings override file support
//! Priority: local.settings.json > database settings

use crate::constants::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Backup retention settings for automatic pre-update backup cleanup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRetention {
    pub enabled: bool,
    pub keep_count: u32,
}

/// Date prefill mode for new trip entries
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatePrefillMode {
    #[default]
    Previous, // Prefill with last trip date + 1 day
    Today, // Prefill with today's date
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub theme: Option<String>,            // "system" | "light" | "dark"
    pub auto_check_updates: Option<bool>, // true by default if None
    pub custom_db_path: Option<String>,   // Custom database location (e.g., Google Drive, NAS)
    pub backup_retention: Option<BackupRetention>, // Backup retention settings for auto-cleanup
    pub date_prefill_mode: Option<DatePrefillMode>, // Date prefill for new trip entries
    pub infer_trip_times: Option<bool>, // None = OFF (default)
    pub hidden_columns: Option<Vec<String>>, // Hidden trip grid columns (e.g., ["time", "fuelConsumed"])
    // Home Assistant integration
    pub ha_url: Option<String>, // Home Assistant URL (e.g., "http://homeassistant.local:8123")
    pub ha_api_token: Option<String>, // Long-lived access token
    // Paperless-ngx integration
    pub paperless_url: Option<String>,
    pub paperless_api_token: Option<String>,
    pub paperless_enabled: Option<bool>,
    // Custom field name overrides — None means "use default" (see PaperlessFieldNames)
    pub paperless_field_name_datetime: Option<String>,
    pub paperless_field_name_liters: Option<String>,
    pub paperless_field_name_total: Option<String>,
    // Server mode
    pub server_enabled: Option<bool>, // Whether HTTP server was enabled (for auto-start)
    pub server_port: Option<u16>,     // Last used server port
}

impl LocalSettings {
    /// Load from local.settings.json in app data dir
    /// Returns default (empty) if file doesn't exist
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join(paths::SETTINGS_FILENAME);
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    /// Load settings with environment-variable overrides applied (env wins).
    /// Use at CONSUMPTION sites only — setters use load()/save() so env
    /// values are never persisted to disk.
    pub fn load_effective(app_data_dir: &Path) -> Self {
        let mut settings = Self::load(app_data_dir);
        settings.apply_overrides(|k| std::env::var(k).ok());
        settings
    }

    /// Pure override application — `lookup` abstracts std::env::var for testability.
    /// Empty/whitespace-only values are treated as unset (file value kept).
    /// PAPERLESS_ENABLED: "1"/"true"/"yes" (case-insensitive) = true; any other
    /// non-empty value = false.
    fn apply_overrides(&mut self, lookup: impl Fn(&str) -> Option<String>) {
        let get = |k: &str| {
            lookup(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        if let Some(v) = get("GEMINI_API_KEY") {
            self.gemini_api_key = Some(v);
        }
        if let Some(v) = get("HA_URL") {
            self.ha_url = Some(v);
        }
        if let Some(v) = get("HA_API_TOKEN") {
            self.ha_api_token = Some(v);
        }
        if let Some(v) = get("PAPERLESS_URL") {
            self.paperless_url = Some(v);
        }
        if let Some(v) = get("PAPERLESS_API_TOKEN") {
            self.paperless_api_token = Some(v);
        }
        if let Some(v) = get("PAPERLESS_ENABLED") {
            self.paperless_enabled = Some(matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"));
        }
    }

    /// True when the given env var pins a settings field (non-empty value set).
    /// Setters use this to refuse edits of env-managed fields.
    pub fn env_pinned(var: &str) -> bool {
        std::env::var(var)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    /// Save settings to local.settings.json in app data dir.
    /// Never persist a `load_effective()` result — setters must round-trip
    /// through `load()` so env-var overrides are never written to disk.
    pub fn save(&self, app_data_dir: &Path) -> std::io::Result<()> {
        use std::io::Write;
        // Ensure the directory exists before writing
        fs::create_dir_all(app_data_dir)?;
        let path = app_data_dir.join(paths::SETTINGS_FILENAME);
        let json = serde_json::to_string_pretty(self)?;
        // Use File::create + write + sync_all to ensure data is flushed to disk
        let mut file = fs::File::create(&path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()
    }
}

/// Shared helper for tests that must mutate the REAL process environment.
/// Process env is global — tests using it must serialize behind one lock.
#[cfg(test)]
pub(crate) mod test_env {
    use std::sync::{Mutex, Once};

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    static AMBIENT_SCRUB: Once = Once::new();

    /// Remove all overridable env vars once per test process — a dev machine
    /// or CI exporting e.g. GEMINI_API_KEY globally must not break the suite.
    fn scrub_ambient_env() {
        AMBIENT_SCRUB.call_once(|| {
            for var in [
                "GEMINI_API_KEY",
                "HA_URL",
                "HA_API_TOKEN",
                "PAPERLESS_URL",
                "PAPERLESS_API_TOKEN",
                "PAPERLESS_ENABLED",
            ] {
                std::env::remove_var(var);
            }
        });
    }

    /// Acquire the env lock without setting any vars — for tests whose
    /// assertions require the overridable env vars to be UNSET (they would
    /// otherwise race with tests that set them).
    pub fn lock() -> std::sync::MutexGuard<'static, ()> {
        let guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        scrub_ambient_env();
        guard
    }

    /// Run `f` with the given env vars set, serialized against all other
    /// real-env tests. Vars are always removed (even on panic) before the
    /// lock is released.
    pub fn with_env_vars<T>(vars: &[(&str, &str)], f: impl FnOnce() -> T) -> T {
        struct Cleanup<'a>(&'a [(&'a str, &'a str)]);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                for (k, _) in self.0 {
                    std::env::remove_var(k);
                }
            }
        }

        // A poisoned lock only means a previous test panicked — env cleanup
        // ran in Cleanup::drop, so it's safe to continue.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        scrub_ambient_env();
        let _cleanup = Cleanup(vars);
        for (k, v) in vars {
            std::env::set_var(k, v);
        }
        f()
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
