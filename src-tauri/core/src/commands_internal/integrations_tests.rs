//! Tests for integrations commands (HA and Paperless).
use super::*;
use tempfile::tempdir;

#[test]
fn save_paperless_settings_persists_url_and_token() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("https://documents.lacny.me".into()),
        Some("tok-1".into()),
        None,
        None, None, None,
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_url.as_deref(), Some("https://documents.lacny.me"));
    assert_eq!(loaded.paperless_api_token.as_deref(), Some("tok-1"));
}

#[test]
fn save_paperless_settings_none_args_preserves_existing() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("https://documents.lacny.me".into()),
        Some("tok-1".into()),
        None,
        None, None, None,
    ).unwrap();

    // Passing None for both args must leave the values unchanged.
    save_paperless_settings_internal(&dir.path().to_path_buf(), &app_state, None, None, None, None, None, None).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_url.as_deref(), Some("https://documents.lacny.me"));
    assert_eq!(loaded.paperless_api_token.as_deref(), Some("tok-1"));
}

#[test]
fn save_paperless_settings_rejects_invalid_url() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    let err = save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("not-a-url".into()),
        Some("tok".into()),
        None,
        None, None, None,
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
        None,
        None, None, None,
    ).unwrap_err();
    // Slovak: "len na čítanie" = "read-only"
    assert!(err.to_lowercase().contains("čítanie") || err.to_lowercase().contains("read"));
}

#[test]
fn get_paperless_settings_hides_token() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x.example".into());
    s.paperless_api_token = Some("super-secret".into());
    s.save(&dir.path().to_path_buf()).unwrap();

    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert_eq!(r.url.as_deref(), Some("https://x.example"));
    assert!(r.has_token);
}

// ============================================================================
// Environment-variable overrides (env wins; setters refuse env-pinned fields)
// ============================================================================

#[test]
fn get_paperless_settings_reflects_env_token_override() {
    let dir = tempdir().unwrap();
    crate::settings::test_env::with_env_vars(&[("PAPERLESS_API_TOKEN", "env-token")], || {
        let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
        assert!(r.has_token, "PAPERLESS_API_TOKEN env var must yield has_token=true");
    });
}

#[test]
fn get_ha_settings_flags_env_pinned_fields() {
    let dir = tempdir().unwrap();
    crate::settings::test_env::with_env_vars(
        &[("HA_URL", "http://env-ha:8123"), ("HA_API_TOKEN", "env-token")],
        || {
            let r = get_ha_settings_internal(&dir.path().to_path_buf()).unwrap();
            assert_eq!(r.url.as_deref(), Some("http://env-ha:8123"));
            assert!(r.url_from_env);
            assert!(r.token_from_env);
            assert!(r.has_token);
            // The value itself is NOT here — reveal_secret owns that (task 69).
        },
    );
}

#[test]
fn get_ha_settings_no_env_leaves_flags_false() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.ha_url = Some("http://file-ha:8123".into());
    s.ha_api_token = Some("file-token".into());
    s.save(&dir.path().to_path_buf()).unwrap();

    let r = get_ha_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert!(!r.url_from_env);
    assert!(!r.token_from_env);
    assert!(r.has_token);
}

#[test]
fn get_ha_settings_pins_url_only() {
    let dir = tempdir().unwrap();
    crate::settings::test_env::with_env_vars(&[("HA_URL", "http://env-ha:8123")], || {
        let r = get_ha_settings_internal(&dir.path().to_path_buf()).unwrap();
        assert!(r.url_from_env);
        assert!(!r.token_from_env, "token stays UI-editable when only HA_URL is pinned");
    });
}

#[test]
fn get_paperless_settings_flags_env_pinned_fields() {
    let dir = tempdir().unwrap();
    crate::settings::test_env::with_env_vars(
        &[
            ("PAPERLESS_URL", "https://env-pl"),
            ("PAPERLESS_API_TOKEN", "env-token"),
            ("PAPERLESS_ENABLED", "true"),
        ],
        || {
            let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
            assert!(r.url_from_env);
            assert!(r.token_from_env);
            assert!(r.enabled_from_env);
        },
    );
}

#[test]
fn get_paperless_settings_no_env_hides_token_value() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://file-pl".into());
    s.paperless_api_token = Some("file-token".into());
    s.save(&dir.path().to_path_buf()).unwrap();

    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert!(!r.url_from_env);
    assert!(!r.token_from_env);
    assert!(!r.enabled_from_env);
    assert!(r.has_token);
}

#[test]
fn save_ha_settings_rejects_url_when_env_pinned() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[("HA_URL", "http://env-ha:8123")], || {
        let err = save_ha_settings_internal(
            &dir.path().to_path_buf(),
            &app_state,
            Some("http://other-ha:8123".into()),
            None,
        )
        .unwrap_err();
        assert!(err.contains("HA_URL"), "error must name HA_URL: {}", err);

        // Token alone is still editable while only HA_URL is pinned
        save_ha_settings_internal(&dir.path().to_path_buf(), &app_state, None, Some("tok".into()))
            .unwrap();
    });
}

#[test]
fn save_ha_settings_rejects_token_when_env_pinned() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[("HA_API_TOKEN", "env-token")], || {
        let err = save_ha_settings_internal(
            &dir.path().to_path_buf(),
            &app_state,
            None,
            Some("new-token".into()),
        )
        .unwrap_err();
        assert!(err.contains("HA_API_TOKEN"), "error must name HA_API_TOKEN: {}", err);
    });
}

#[test]
fn save_paperless_settings_rejects_env_pinned_fields() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();

    crate::settings::test_env::with_env_vars(&[("PAPERLESS_URL", "https://env-pl")], || {
        let err = save_paperless_settings_internal(
            &dir.path().to_path_buf(), &app_state,
            Some("https://other-pl".into()), None, None, None, None, None,
        ).unwrap_err();
        assert!(err.contains("PAPERLESS_URL"), "error must name PAPERLESS_URL: {}", err);
    });

    crate::settings::test_env::with_env_vars(&[("PAPERLESS_API_TOKEN", "env-token")], || {
        let err = save_paperless_settings_internal(
            &dir.path().to_path_buf(), &app_state,
            None, Some("new-token".into()), None, None, None, None,
        ).unwrap_err();
        assert!(err.contains("PAPERLESS_API_TOKEN"), "error must name PAPERLESS_API_TOKEN: {}", err);
    });

    crate::settings::test_env::with_env_vars(&[("PAPERLESS_ENABLED", "true")], || {
        let err = save_paperless_settings_internal(
            &dir.path().to_path_buf(), &app_state,
            None, None, Some(false), None, None, None,
        ).unwrap_err();
        assert!(err.contains("PAPERLESS_ENABLED"), "error must name PAPERLESS_ENABLED: {}", err);
    });
}

#[test]
fn save_paperless_settings_field_names_editable_while_token_env_pinned() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[("PAPERLESS_API_TOKEN", "env-token")], || {
        // Field-name-only save must succeed even though the token is env-pinned
        save_paperless_settings_internal(
            &dir.path().to_path_buf(), &app_state,
            None, None, None,
            Some("Dátum".into()), Some("Litre".into()), Some("Suma".into()),
        ).unwrap();
    });
    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_field_name_datetime.as_deref(), Some("Dátum"));
    assert_eq!(loaded.paperless_field_name_liters.as_deref(), Some("Litre"));
    assert_eq!(loaded.paperless_field_name_total.as_deref(), Some("Suma"));
}

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_paperless_connection_uses_token_auth_header_not_bearer() {
    let _env = crate::settings::test_env::lock();
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
    let _env = crate::settings::test_env::lock();
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
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let ok = test_paperless_connection_internal(&dir.path().to_path_buf()).await.unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn test_paperless_connection_401_returns_false() {
    let _env = crate::settings::test_env::lock();
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

#[test]
fn save_paperless_settings_persists_enabled_flag() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        Some("https://x.example".into()),
        Some("tok".into()),
        Some(false),
        None, None, None,
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_enabled, Some(false));
}

#[test]
fn get_paperless_settings_returns_enabled_field() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x.example".into());
    s.paperless_api_token = Some("tok".into());
    s.paperless_enabled = Some(false);
    s.save(&dir.path().to_path_buf()).unwrap();

    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert!(!r.enabled);
}

#[test]
fn save_paperless_settings_persists_custom_field_names() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();

    save_paperless_settings_internal(
        &dir.path().to_path_buf(),
        &app_state,
        Some("https://paperless.example.com".to_string()),
        Some("token123".to_string()),
        Some(true),
        Some("Dátum dokladu".to_string()),
        Some("Litre".to_string()),
        Some("Suma".to_string()),
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_field_name_datetime.as_deref(), Some("Dátum dokladu"));
    assert_eq!(loaded.paperless_field_name_liters.as_deref(), Some("Litre"));
    assert_eq!(loaded.paperless_field_name_total.as_deref(), Some("Suma"));
}

#[test]
fn save_paperless_settings_empty_field_name_clears_to_use_default() {
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();

    // First save custom values.
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, None,
        Some("custom_dt".to_string()),
        Some("custom_lt".to_string()),
        Some("custom_tt".to_string()),
    ).unwrap();

    // Then clear with empty strings.
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, None,
        Some("".to_string()),
        Some("".to_string()),
        Some("".to_string()),
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_field_name_datetime, None);
    assert_eq!(loaded.paperless_field_name_liters, None);
    assert_eq!(loaded.paperless_field_name_total, None);
}

#[test]
fn save_paperless_settings_none_field_name_keeps_existing() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();

    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, None,
        Some("existing_dt".to_string()),
        Some("existing_lt".to_string()),
        Some("existing_tt".to_string()),
    ).unwrap();

    // Update only enabled, leave field names as None.
    save_paperless_settings_internal(
        &dir.path().to_path_buf(), &app_state,
        None, None, Some(false),
        None, None, None,
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(&dir.path().to_path_buf());
    assert_eq!(loaded.paperless_field_name_datetime.as_deref(), Some("existing_dt"));
    assert_eq!(loaded.paperless_field_name_liters.as_deref(), Some("existing_lt"));
    assert_eq!(loaded.paperless_field_name_total.as_deref(), Some("existing_tt"));
    assert_eq!(loaded.paperless_enabled, Some(false));
}

#[test]
fn get_paperless_settings_returns_default_field_names_when_unset() {
    let dir = tempdir().unwrap();
    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert_eq!(r.field_name_datetime, "receipt_datetime");
    assert_eq!(r.field_name_liters, "liters");
    assert_eq!(r.field_name_total, "total_price_eur");
}

#[test]
fn get_paperless_settings_returns_custom_field_names_when_set() {
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_field_name_datetime = Some("Dátum dokladu".to_string());
    s.paperless_field_name_liters = Some("Litre".to_string());
    s.paperless_field_name_total = Some("Suma".to_string());
    s.save(&dir.path().to_path_buf()).unwrap();

    let r = get_paperless_settings_internal(&dir.path().to_path_buf()).unwrap();
    assert_eq!(r.field_name_datetime, "Dátum dokladu");
    assert_eq!(r.field_name_liters, "Litre");
    assert_eq!(r.field_name_total, "Suma");
}

// ============================================================================
// Suggested-fillup push to Home Assistant
// ============================================================================

fn vehicle_with_sensor(sensor: Option<&str>) -> crate::models::Vehicle {
    let mut v = crate::models::Vehicle::new_ice("Test".into(), "BA-123AB".into(), 50.0, 6.5, 0.0);
    v.ha_fillup_sensor = sensor.map(|s| s.to_string());
    v
}

/// A real (empty) grid from the shared builder — TripGridData has no Default and
/// hand-rolling its ~15 fields would rot on every schema change.
fn grid_with_suggestion(suggestion: Option<SuggestedFillup>) -> TripGridData {
    let db = crate::db::Database::in_memory().unwrap();
    let v = vehicle_with_sensor(None);
    db.create_vehicle(&v).unwrap();
    let mut g =
        crate::commands_internal::build_trip_grid_data(&db, &v.id.to_string(), 2026).unwrap();
    g.legend_suggested_fillup = suggestion;
    g
}

#[test]
fn ha_fillup_payload_none_when_sensor_unset() {
    let grid = grid_with_suggestion(None);
    assert!(ha_fillup_push_payload(&vehicle_with_sensor(None), &grid).is_none());
    // A blank entity id is "not configured", not an entity named ""
    assert!(ha_fillup_push_payload(&vehicle_with_sensor(Some("   ")), &grid).is_none());
}

#[test]
fn ha_fillup_payload_formats_suggestion() {
    let grid = grid_with_suggestion(Some(SuggestedFillup {
        liters: 20.394,
        consumption_rate: 5.664,
    }));
    let (entity, value) =
        ha_fillup_push_payload(&vehicle_with_sensor(Some("input_text.fillup")), &grid).unwrap();
    assert_eq!(entity, "input_text.fillup");
    assert_eq!(value, "20.39 L → 5.66 l/100km");
}

#[test]
fn ha_fillup_payload_reports_full_tank_when_no_suggestion() {
    let grid = grid_with_suggestion(None);
    let (_, value) =
        ha_fillup_push_payload(&vehicle_with_sensor(Some("input_text.fillup")), &grid).unwrap();
    assert_eq!(value, "Plná nádrž");
}

#[tokio::test]
async fn push_ha_input_text_calls_set_value_service() {
    let _env = crate::settings::test_env::lock();
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/services/input_text/set_value"))
        .and(header("authorization", "Bearer ha-token"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "entity_id": "input_text.fillup",
            "value": "20.39 L → 5.66 l/100km",
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.ha_url = Some(mock.uri());
    s.ha_api_token = Some("ha-token".into());
    s.save(dir.path()).unwrap();

    push_ha_input_text(
        dir.path().to_path_buf(),
        "input_text.fillup".into(),
        "20.39 L → 5.66 l/100km".into(),
    )
    .await;
    // `expect(1)` is verified on drop
}

#[tokio::test]
async fn push_ha_input_text_noop_when_ha_unconfigured() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap();
    // No ha_url / ha_api_token — must return without panicking or hanging
    push_ha_input_text(dir.path().to_path_buf(), "input_text.fillup".into(), "x".into()).await;
}

// ============================================================================
// Leak guards — no settings read may carry a secret (task 69)
// ============================================================================

/// Asserts on the SERIALIZED response rather than named fields, so a newly added
/// leaky field fails too. That is exactly how `tokenEnvValue` slipped in.
#[test]
fn settings_responses_never_carry_secrets() {
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.ha_url = Some("https://ha.example".into());
    s.ha_api_token = Some("SECRET-ha-token".into());
    s.paperless_url = Some("https://pl.example".into());
    s.paperless_api_token = Some("SECRET-paperless-token".into());
    s.gemini_api_key = Some("SECRET-gemini-key".into());
    s.save(dir.path()).unwrap();

    crate::settings::test_env::with_env_vars(
        &[
            ("HA_API_TOKEN", "SECRET-env-ha"),
            ("PAPERLESS_API_TOKEN", "SECRET-env-paperless"),
            ("GEMINI_API_KEY", "SECRET-env-gemini"),
        ],
        || {
            let responses = [
                serde_json::to_string(&get_ha_settings_internal(dir.path()).unwrap()).unwrap(),
                serde_json::to_string(&get_paperless_settings_internal(dir.path()).unwrap())
                    .unwrap(),
                serde_json::to_string(
                    &crate::commands_internal::receipts_cmd::get_receipt_settings_internal(
                        dir.path(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            ];
            for json in responses {
                assert!(
                    !json.contains("SECRET-"),
                    "a settings read leaked a secret: {json}"
                );
            }
        },
    );
}
