**Date:** 2026-08-07
**Subject:** Non-blocking review follow-ups
**Status:** Complete

# Non-Blocking Review Follow-ups

Deferred items from the code review of the env-var configuration work (commits `071baa9`, `feee140`). Neither blocks the feature; both are candidates for a future cleanup pass.

## 1. English error strings in backend guards/validation

The env-pinned guard errors ("… is managed by the … environment variable") are English while the UI is Slovak. Precedent already exists — e.g. the "URL must start with http://..." validation message. Consider i18n-izing backend validation errors together as one batch rather than piecemeal.

## 2. Secrets returned in full to the frontend

- `get_receipt_settings` returns the full Gemini API key to the frontend (pre-existing behaviour; now also exposes an env-provided key).
- `get_local_settings_for_ha` returns the HA token.

Consider masking these like the Paperless `has_token` pattern (boolean flag instead of the secret). Requires reworking the Settings UI prefill (fields can no longer be pre-populated with the real value). The LAN-trust model (ADR-017) makes this low urgency.
