//! Tests for PIN-gated secret reveal.
//!
//! The PIN lives in a real process env var, so every test that depends on it
//! must serialize behind `test_env` (see settings.rs).

use super::*;
use tempfile::tempdir;

const PIN_VAR: &str = "KNIHA_JAZD_REVEAL_PIN";

fn dir_with_secrets() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.gemini_api_key = Some("file-gemini".into());
    s.ha_api_token = Some("file-ha".into());
    s.paperless_api_token = Some("file-paperless".into());
    s.save(dir.path()).unwrap();
    dir
}

#[test]
fn local_trusted_reveals_without_any_pin_configured() {
    let _env = crate::settings::test_env::lock();
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();

    let value = reveal_secret_internal(
        dir.path(),
        &app_state,
        SecretField::HaApiToken,
        RevealAuth::LocalTrusted,
    )
    .unwrap();
    assert_eq!(value, "file-ha");
}

#[test]
fn correct_pin_reveals_value() {
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[(PIN_VAR, "4269")], || {
        let value = reveal_secret_internal(
            dir.path(),
            &app_state,
            SecretField::HaApiToken,
            RevealAuth::Pin("4269".into()),
        )
        .unwrap();
        assert_eq!(value, "file-ha");
    });
}

#[test]
fn wrong_pin_is_rejected_without_leaking_the_value() {
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[(PIN_VAR, "4269")], || {
        let err = reveal_secret_internal(
            dir.path(),
            &app_state,
            SecretField::HaApiToken,
            RevealAuth::Pin("0000".into()),
        )
        .unwrap_err();
        assert!(err.to_lowercase().contains("pin"), "got: {err}");
        assert!(!err.contains("file-ha"), "error must not leak the secret: {err}");
    });
}

#[test]
fn unset_pin_disables_reveal_with_a_distinct_error() {
    let _env = crate::settings::test_env::lock();
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();

    let err = reveal_secret_internal(
        dir.path(),
        &app_state,
        SecretField::HaApiToken,
        RevealAuth::Pin("anything".into()),
    )
    .unwrap_err();
    // Misconfiguration and rejection are different problems for the operator
    assert!(err.contains(PIN_VAR), "error must name the variable to set: {err}");
}

#[test]
fn env_value_wins_over_file() {
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(
        &[(PIN_VAR, "4269"), ("HA_API_TOKEN", "env-ha")],
        || {
            let value = reveal_secret_internal(
                dir.path(),
                &app_state,
                SecretField::HaApiToken,
                RevealAuth::Pin("4269".into()),
            )
            .unwrap();
            // Revealing exists to show what is actually live
            assert_eq!(value, "env-ha");
        },
    );
}

#[test]
fn each_field_maps_to_its_own_setting() {
    let _env = crate::settings::test_env::lock();
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();

    let get = |f| {
        reveal_secret_internal(dir.path(), &app_state, f, RevealAuth::LocalTrusted).unwrap()
    };
    assert_eq!(get(SecretField::GeminiApiKey), "file-gemini");
    assert_eq!(get(SecretField::HaApiToken), "file-ha");
    assert_eq!(get(SecretField::PaperlessApiToken), "file-paperless");
}

#[test]
fn unconfigured_secret_errors_rather_than_returning_empty() {
    let _env = crate::settings::test_env::lock();
    let dir = tempdir().unwrap(); // nothing configured
    let app_state = crate::app_state::AppState::new();

    let err = reveal_secret_internal(
        dir.path(),
        &app_state,
        SecretField::GeminiApiKey,
        RevealAuth::LocalTrusted,
    )
    .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn repeated_wrong_pins_lock_out_even_a_correct_one() {
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[(PIN_VAR, "4269")], || {
        for _ in 0..5 {
            let _ = reveal_secret_internal(
                dir.path(),
                &app_state,
                SecretField::HaApiToken,
                RevealAuth::Pin("0000".into()),
            );
        }
        // The right PIN is now refused too — that's the point of a lockout
        let err = reveal_secret_internal(
            dir.path(),
            &app_state,
            SecretField::HaApiToken,
            RevealAuth::Pin("4269".into()),
        )
        .unwrap_err();
        assert!(err.contains("Too many"), "got: {err}");
    });
}

#[test]
fn success_resets_the_failure_counter() {
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[(PIN_VAR, "4269")], || {
        for _ in 0..4 {
            let _ = reveal_secret_internal(
                dir.path(),
                &app_state,
                SecretField::HaApiToken,
                RevealAuth::Pin("0000".into()),
            );
        }
        // A correct PIN clears the count...
        reveal_secret_internal(
            dir.path(),
            &app_state,
            SecretField::HaApiToken,
            RevealAuth::Pin("4269".into()),
        )
        .unwrap();
        // ...so four more wrong ones still don't lock us out
        for _ in 0..4 {
            let err = reveal_secret_internal(
                dir.path(),
                &app_state,
                SecretField::HaApiToken,
                RevealAuth::Pin("0000".into()),
            )
            .unwrap_err();
            assert!(!err.contains("Too many"), "locked out too early: {err}");
        }
    });
}

#[test]
fn local_trusted_is_never_throttled() {
    let dir = dir_with_secrets();
    let app_state = crate::app_state::AppState::new();
    crate::settings::test_env::with_env_vars(&[(PIN_VAR, "4269")], || {
        for _ in 0..6 {
            let _ = reveal_secret_internal(
                dir.path(),
                &app_state,
                SecretField::HaApiToken,
                RevealAuth::Pin("0000".into()),
            );
        }
        // The desktop path isn't network-reachable, so a network attacker
        // must not be able to lock the local user out of their own app
        let value = reveal_secret_internal(
            dir.path(),
            &app_state,
            SecretField::HaApiToken,
            RevealAuth::LocalTrusted,
        )
        .unwrap();
        assert_eq!(value, "file-ha");
    });
}

#[test]
fn pin_comparison_is_length_safe() {
    // Guards the constant-time helper against index panics on unequal lengths
    assert!(pin_matches("4269", "4269"));
    assert!(!pin_matches("4269", "426"));
    assert!(!pin_matches("426", "4269"));
    assert!(!pin_matches("", "4269"));
    assert!(!pin_matches("4269", ""));
}
