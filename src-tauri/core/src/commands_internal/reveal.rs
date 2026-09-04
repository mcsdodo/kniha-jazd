//! PIN-gated reveal of configured secrets.
//!
//! Ordinary settings reads never return a secret (see task 69). This is the only
//! way a credential leaves the backend, and the gate is unconditional: the
//! JSON-RPC dispatcher is the process's single entry point and it is reachable
//! from the LAN, so there is no caller that could be trusted to skip the PIN.
//! Repeated wrong PINs lock the reveal out via [`AppState::reveal_check`].

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::app_state::AppState;
use crate::settings::LocalSettings;

/// Which secret to reveal.
///
/// An enum, not a free-form field name, so the command cannot be aimed at
/// arbitrary settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretField {
    GeminiApiKey,
    HaApiToken,
    PaperlessApiToken,
}

impl SecretField {
    fn label(self) -> &'static str {
        match self {
            SecretField::GeminiApiKey => "Gemini API key",
            SecretField::HaApiToken => "Home Assistant token",
            SecretField::PaperlessApiToken => "Paperless token",
        }
    }

    fn value_from(self, settings: LocalSettings) -> Option<String> {
        match self {
            SecretField::GeminiApiKey => settings.gemini_api_key,
            SecretField::HaApiToken => settings.ha_api_token,
            SecretField::PaperlessApiToken => settings.paperless_api_token,
        }
    }
}

/// Constant-time PIN comparison.
///
/// Accumulates differences over the longer of the two inputs instead of
/// returning early, so neither the PIN's length nor its matching prefix is
/// observable through timing.
fn pin_matches(supplied: &str, expected: &str) -> bool {
    let (a, b) = (supplied.as_bytes(), expected.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        diff |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    diff == 0
}

/// Reveal one configured secret, enforcing the PIN.
///
/// A missing `pin` argument arrives as `""` and is rejected like any other wrong
/// PIN — there is no unauthenticated path.
///
/// Returns the **effective** value (env override beats the settings file) —
/// revealing exists to show what is actually live.
pub fn reveal_secret_internal(
    app_dir: &Path,
    app_state: &AppState,
    field: SecretField,
    pin: &str,
) -> Result<String, String> {
    // Checked before the comparison so a lockout can't be probed by timing it.
    app_state.reveal_check()?;

    let expected = std::env::var(crate::settings::env_vars::REVEAL_PIN)
        .ok()
        .filter(|v| !v.trim().is_empty());

    let expected = match expected {
        Some(p) => p,
        // Deliberately distinct from "incorrect PIN": misconfiguration and
        // rejection are different problems, and telling them apart gives an
        // attacker nothing they could not already infer.
        None => {
            return Err(format!(
                "Revealing secrets is disabled: {} is not set on the server.",
                crate::settings::env_vars::REVEAL_PIN
            ))
        }
    };

    if !pin_matches(pin, &expected) {
        app_state.reveal_record_failure();
        return Err("Incorrect PIN.".to_string());
    }
    app_state.reveal_record_success();

    field
        .value_from(LocalSettings::load_effective(app_dir))
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("{} is not configured.", field.label()))
}

#[cfg(test)]
#[path = "reveal_tests.rs"]
mod tests;
