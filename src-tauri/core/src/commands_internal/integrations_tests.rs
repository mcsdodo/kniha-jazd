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
fn save_paperless_settings_none_args_preserves_existing() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("https://documents.lacny.me".into()),
        Some("tok-1".into()),
    ).unwrap();

    // Passing None for both args must leave the values unchanged.
    save_paperless_settings_internal(&dir.path().to_path_buf(), &app_state, None, None).unwrap();

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

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_paperless_connection_uses_token_auth_header_not_bearer() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/ui_settings/"))
        .and(header("authorization", "Token my-pat-123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&mock).await;

    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(mock.uri());
    s.paperless_api_token = Some("my-pat-123".into());
    s.save(&dir.path().to_path_buf()).unwrap();

    let ok = test_paperless_connection_internal(&dir.path().to_path_buf()).await.unwrap();
    assert!(ok);
}

#[tokio::test]
async fn test_paperless_connection_rejects_bearer_header() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/ui_settings/"))
        .and(header("authorization", "Bearer my-pat-123"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock).await;

    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(mock.uri());
    s.paperless_api_token = Some("my-pat-123".into());
    s.save(&dir.path().to_path_buf()).unwrap();

    let ok = test_paperless_connection_internal(&dir.path().to_path_buf()).await.unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn test_paperless_connection_unconfigured_returns_false_silently() {
    let dir = tempdir().unwrap();
    let ok = test_paperless_connection_internal(&dir.path().to_path_buf()).await.unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn test_paperless_connection_401_returns_false() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/ui_settings/"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock).await;

    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(mock.uri());
    s.paperless_api_token = Some("bad".into());
    s.save(&dir.path().to_path_buf()).unwrap();

    assert!(!test_paperless_connection_internal(&dir.path().to_path_buf()).await.unwrap());
}

#[test]
fn invoice_source_mode_is_paperless_when_both_fields_populated() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Paperless);
}

#[test]
fn invoice_source_mode_is_local_when_url_missing() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_api_token = Some("t".into());
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Local);
}

#[test]
fn invoice_source_mode_is_local_when_token_missing() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Local);
}

#[test]
fn invoice_source_mode_is_local_when_url_is_empty_string() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(String::new());
    s.paperless_api_token = Some("t".into());
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Local);
}

#[test]
fn invoice_source_mode_is_local_when_disabled_even_with_credentials() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    s.paperless_enabled = Some(false);
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Local);
}

#[test]
fn invoice_source_mode_is_paperless_when_enabled_true_with_credentials() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    s.paperless_enabled = Some(true);
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Paperless);
}

#[test]
fn invoice_source_mode_is_paperless_when_enabled_none_with_credentials_backward_compat() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    // None means "not explicitly set" — treat as enabled for backward compat
    s.paperless_enabled = None;
    assert_eq!(get_invoice_source_mode_from_settings(&s), InvoiceSourceMode::Paperless);
}
