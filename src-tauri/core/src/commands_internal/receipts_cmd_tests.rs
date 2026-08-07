//! Tests for receipt commands (env-var override behavior).

use super::*;
use tempfile::tempdir;

#[test]
fn get_receipt_settings_reflects_gemini_env_override() {
    let dir = tempdir().unwrap();
    crate::settings::test_env::with_env_vars(&[("GEMINI_API_KEY", "env-gemini-key")], || {
        let r = get_receipt_settings_internal(dir.path()).unwrap();
        assert_eq!(r.gemini_api_key.as_deref(), Some("env-gemini-key"));
        assert!(r.gemini_api_key_from_override);
    });
}

#[test]
fn set_gemini_api_key_rejected_when_env_pinned() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[("GEMINI_API_KEY", "env-gemini-key")], || {
        let err = set_gemini_api_key_internal(dir.path(), &app_state, "new-key".to_string())
            .unwrap_err();
        assert!(err.contains("GEMINI_API_KEY"), "error must name the env var: {}", err);
    });
    // Without the env var the setter works normally
    set_gemini_api_key_internal(dir.path(), &app_state, "new-key".to_string()).unwrap();
    let loaded = LocalSettings::load(dir.path());
    assert_eq!(loaded.gemini_api_key.as_deref(), Some("new-key"));
}
