//! Tests for integrations commands (HA and Paperless).
use super::*;
use tempfile::tempdir;

#[test]
fn save_paperless_settings_persists_url_and_token() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("https://documents.lacny.me".into()),
        Some("tok-1".into()),
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_url.as_deref(), Some("https://documents.lacny.me"));
    assert_eq!(loaded.paperless_api_token.as_deref(), Some("tok-1"));
}

#[test]
fn save_paperless_settings_rejects_invalid_url() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    let err = save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("not-a-url".into()),
        Some("tok".into()),
    ).unwrap_err();
    assert!(err.contains("URL must start with http"));
}

#[test]
fn save_paperless_settings_blocked_by_read_only() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    app_state.enable_read_only("test");
    let err = save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("https://x.example".into()), Some("t".into()),
    ).unwrap_err();
    // Slovak: "len na čítanie" = "read-only"
    assert!(err.to_lowercase().contains("čítanie") || err.to_lowercase().contains("read"));
}

#[test]
fn get_paperless_settings_hides_token() {
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x.example".into());
    s.paperless_api_token = Some("super-secret".into());
    s.save(&dir.path().to_path_buf()).unwrap();

    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert_eq!(r.url.as_deref(), Some("https://x.example"));
    assert!(r.has_token);
}
