//! Tests for receipt commands (env-var override behavior).

use super::*;
use tempfile::tempdir;

#[test]
fn get_receipt_settings_reflects_gemini_env_override() {
    let dir = tempdir().unwrap();
    crate::settings::test_env::with_env_vars(&[("GEMINI_API_KEY", "env-gemini-key")], || {
        let r = get_receipt_settings_internal(dir.path()).unwrap();
        // The key itself is never returned — only that one is configured (task 69)
        assert!(r.has_gemini_api_key);
        assert!(r.gemini_api_key_from_env);
    });
}

#[test]
fn get_receipt_settings_file_key_not_flagged_as_env() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let mut s = LocalSettings::default();
    s.gemini_api_key = Some("file-key".into());
    s.receipts_folder_path = Some("C:/receipts".into());
    s.save(dir.path()).unwrap();

    let r = get_receipt_settings_internal(dir.path()).unwrap();
    assert!(r.has_gemini_api_key);
    // A configured key is not the same as an env-pinned one
    assert!(!r.gemini_api_key_from_env);
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
}

#[test]
fn set_gemini_api_key_works_when_env_not_pinned() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    set_gemini_api_key_internal(dir.path(), &app_state, "new-key".to_string()).unwrap();
    let loaded = LocalSettings::load(dir.path());
    assert_eq!(loaded.gemini_api_key.as_deref(), Some("new-key"));
}
