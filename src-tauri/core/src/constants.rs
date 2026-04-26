//! Application-wide constants
//!
//! This module contains constant values used across the Rust backend.
//! Use these constants instead of magic strings for consistency and maintainability.

/// File paths and names used by the application
pub mod paths {
    /// Main database filename
    pub const DB_FILENAME: &str = "kniha-jazd.db";

    /// Lock file for concurrent access prevention
    pub const LOCK_FILENAME: &str = "kniha-jazd.lock";

    /// Backups directory name
    pub const BACKUPS_DIR: &str = "backups";

    /// Local settings file
    pub const SETTINGS_FILENAME: &str = "local.settings.json";

    /// Backup file prefix
    pub const BACKUP_PREFIX: &str = "kniha-jazd-backup-";

    /// Backup file extension
    pub const BACKUP_EXTENSION: &str = ".db";

    /// Pre-update backup version marker
    pub const PRE_UPDATE_MARKER: &str = "-pre-v";
}

/// Date format strings for parsing and formatting
pub mod date_formats {
    /// ISO date format (YYYY-MM-DD) - used for database storage
    pub const ISO_DATE: &str = "%Y-%m-%d";

    /// Backup timestamp format (YYYY-MM-DD-HHMMSS)
    pub const BACKUP_TIMESTAMP: &str = "%Y-%m-%d-%H%M%S";

    /// European display format (DD.MM.YYYY) - available for gradual adoption
    #[allow(dead_code)]
    pub const DISPLAY_DATE: &str = "%d.%m.%Y";

    /// Display format with time (DD.MM. HH:MM)
    pub const DISPLAY_DATE_TIME: &str = "%d.%m. %H:%M";

    /// Year extraction format - available for gradual adoption
    #[allow(dead_code)]
    pub const YEAR_ONLY: &str = "%Y";

    /// Time only format
    pub const TIME_ONLY: &str = "%H:%M";
}

/// MIME types for file handling
pub mod mime_types {
    pub const JPEG: &str = "image/jpeg";
    pub const PNG: &str = "image/png";
    pub const WEBP: &str = "image/webp";
    pub const PDF: &str = "application/pdf";
    pub const JSON: &str = "application/json";
}

/// Environment variable names
pub mod env_vars {
    /// Override data directory location
    pub const DATA_DIR: &str = "KNIHA_JAZD_DATA_DIR";

    /// Enable mock Gemini responses for testing
    pub const MOCK_GEMINI_DIR: &str = "KNIHA_JAZD_MOCK_GEMINI_DIR";
}

/// Default values
pub mod defaults {
    /// Default tank size for legacy/unknown vehicles
    pub const TANK_SIZE_LITERS: f64 = 50.0;

    /// Default battery capacity for EVs - available for gradual adoption
    #[allow(dead_code)]
    pub const BATTERY_CAPACITY_KWH: f64 = 50.0;

    /// Lock file expiry in seconds (stale if older) - available for gradual adoption
    #[allow(dead_code)]
    pub const LOCK_EXPIRY_SECS: u64 = 300;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_constants() {
        assert!(paths::DB_FILENAME.ends_with(".db"));
        assert!(paths::LOCK_FILENAME.ends_with(".lock"));
        assert!(paths::BACKUP_PREFIX.starts_with("kniha-jazd"));
    }

    #[test]
    fn test_date_formats_constants() {
        // Verify formats are valid strftime patterns
        assert!(date_formats::ISO_DATE.contains("%Y"));
        assert!(date_formats::BACKUP_TIMESTAMP.contains("%H%M%S"));
        assert!(date_formats::DISPLAY_DATE.contains("%d"));
    }

    #[test]
    fn test_mime_types_constants() {
        assert!(mime_types::JPEG.starts_with("image/"));
        assert!(mime_types::PDF.starts_with("application/"));
    }
}
