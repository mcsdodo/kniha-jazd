**Date:** 2026-08-10
**Subject:** Require a PIN to reveal secrets, and stop serving them to the network
**Status:** Complete

# PIN-Gated Secret Reveal Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** No command returns a secret except one dedicated reveal command, which requires a throttled PIN when the request arrives over the network.

**Architecture:** Locality is decided by code path, not by a claim in the request — the Tauri wrapper passes `RevealAuth::LocalTrusted`, the RPC dispatcher always passes `RevealAuth::Pin`. A constant-time compare against `KNIHA_JAZD_REVEAL_PIN` plus an escalating lockout on `AppState` makes a 4-character PIN viable. Three existing commands stop carrying secrets.

**Tech Stack:** Rust (core + Tauri desktop crates), Diesel/SQLite, SvelteKit + TypeScript, WebdriverIO.

Task: [01-task.md](./01-task.md) · Design: [02-design.md](./02-design.md)

---

### Task 1: Reveal throttle on AppState

**Files:**
- Modify: [app_state.rs](../../src-tauri/core/src/app_state.rs)
- Test: [app_state_tests.rs](../../src-tauri/core/src/app_state_tests.rs) (or the inline `#[cfg(test)]` module already there)

**Step 1: Write the failing tests**

```rust
#[test]
fn reveal_throttle_allows_until_threshold() {
    let s = AppState::new();
    for _ in 0..4 { assert!(s.reveal_check().is_ok()); s.reveal_record_failure(); }
    // 5th failure trips the lockout
    assert!(s.reveal_check().is_ok());
    s.reveal_record_failure();
    assert!(s.reveal_check().is_err());
}

#[test]
fn reveal_throttle_success_resets() {
    let s = AppState::new();
    for _ in 0..5 { s.reveal_record_failure(); }
    assert!(s.reveal_check().is_err());
    s.reveal_record_success();
    assert!(s.reveal_check().is_ok());
}

#[test]
fn reveal_throttle_error_names_remaining_seconds() {
    let s = AppState::new();
    for _ in 0..5 { s.reveal_record_failure(); }
    let e = s.reveal_check().unwrap_err();
    assert!(e.contains("60") || e.to_lowercase().contains("sekúnd") || e.to_lowercase().contains("second"), "got: {e}");
}
```

**Step 2: Run to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core reveal_throttle`
Expected: FAIL — no method `reveal_check`.

**Step 3: Implement**

Add to `AppState`: `reveal_throttle: Mutex<RevealThrottle>` with
`RevealThrottle { consecutive_failures: u32, locked_until: Option<Instant> }`.
Lockout ladder after every 5 consecutive failures: 60s → 300s → 900s → 3600s (cap).

**Step 4: Verify pass**

Run: same command. Expected: 3 passed.

**Step 5: Commit** — `git add src-tauri/core/src/app_state*.rs && git commit -m "feat(reveal): add PIN failure throttle to AppState"`

---

### Task 2: `reveal_secret_internal`

**Files:**
- Create: `src-tauri/core/src/commands_internal/reveal.rs`
- Create: `src-tauri/core/src/commands_internal/reveal_tests.rs`
- Modify: [commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) — register module
- Modify: [settings.rs](../../src-tauri/core/src/settings.rs) — add `env_vars::REVEAL_PIN`

**Step 1: Write the failing tests**

Cover, one test each:
- `local_trusted_reveals_without_pin` — `RevealAuth::LocalTrusted` returns the value even with no PIN configured.
- `correct_pin_reveals_value`
- `wrong_pin_is_rejected` — error says "PIN", not the value.
- `unset_pin_disables_reveal` — error mentions `KNIHA_JAZD_REVEAL_PIN`, and is **different** from the wrong-PIN error.
- `env_value_wins_over_file` — file has `file-token`, `HA_API_TOKEN=env-token`, reveal returns `env-token`.
- `each_field_maps_to_its_setting` — `GeminiApiKey`/`HaApiToken`/`PaperlessApiToken` return their own values.
- `missing_secret_errors` — field not configured at all → `Err`, not `Ok("")`.
- `repeated_wrong_pins_lock_out` — 5 wrong, 6th returns the lockout error even if the PIN is now correct.
- `success_resets_throttle` — 4 wrong then correct, then wrong again still allowed.

Use `crate::settings::test_env::with_env_vars` for `KNIHA_JAZD_REVEAL_PIN`.

**Step 2: Verify failure** — `cargo test … -p kniha-jazd-core reveal::` → FAIL (module missing).

**Step 3: Implement** per [02-design.md](./02-design.md): throttle check → PIN configured? → constant-time compare → resolve field from `LocalSettings::load_effective`.

Constant-time compare (no early return, no `==`):

```rust
fn pin_matches(supplied: &str, expected: &str) -> bool {
    let (a, b) = (supplied.as_bytes(), expected.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        diff |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    diff == 0
}
```

**Step 4: Verify pass** — all reveal tests green.

**Step 5: Commit** — `git commit -m "feat(reveal): add PIN-gated reveal_secret_internal"`

---

### Task 3: Wire both frontends

**Files:**
- Modify: [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) — add `reveal_secret` arm (sync; no `.await` needed)
- Modify: `src-tauri/desktop/src/commands/settings_cmd.rs` (or `integrations.rs`) — `#[tauri::command] reveal_secret`
- Modify: [lib.rs](../../src-tauri/desktop/src/lib.rs) — register the command
- Test: dispatcher `#[cfg(test)]`

**Step 1: Write the failing tests**

```rust
#[test]
fn reveal_secret_over_rpc_requires_pin() {
    // No KNIHA_JAZD_REVEAL_PIN set
    let err = dispatch_sync("reveal_secret", json!({"field":"haApiToken","pin":"1234"}), &state).unwrap_err();
    assert!(err.contains("KNIHA_JAZD_REVEAL_PIN"));
}

#[test]
fn reveal_secret_over_rpc_accepts_correct_pin() { /* with_env_vars, assert value */ }

#[test]
fn reveal_secret_over_rpc_rejects_missing_pin_argument() {
    // args without "pin" must not fall through to an unauthenticated reveal
}
```

The dispatcher must construct `RevealAuth::Pin` **unconditionally** — a missing `pin`
argument becomes `Pin("")`, never `LocalTrusted`.

**Step 2–4:** verify fail → implement → verify pass.

**Step 5: Commit** — `git commit -m "feat(reveal): expose reveal_secret on both frontends"`

---

### Task 4: Stop returning secrets (the regression guards)

**Files:**
- Modify: [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) — drop `token_env_value` ×2; delete `get_local_settings_for_ha_internal` + `HaLocalSettingsResponse`
- Modify: [receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) — `gemini_api_key` → `has_gemini_api_key: bool`
- Modify: [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs), [desktop integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs), [lib.rs](../../src-tauri/desktop/src/lib.rs) — remove the deleted command
- Modify: [integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs) — task 68's `tokenEnvValue` assertions move to reveal tests

**Step 1: Write the failing guards** — serialize each response and assert the secret
string appears **nowhere** in the JSON:

```rust
#[test]
fn settings_responses_never_carry_secrets() {
    // file + env secrets set to distinctive values
    let json = serde_json::to_string(&get_ha_settings_internal(dir)?).unwrap();
    assert!(!json.contains("super-secret"), "HA settings leaked a token: {json}");
    // same for get_paperless_settings_internal and get_receipt_settings_internal
}
```

Asserting on the serialized JSON rather than named fields is deliberate: it fails for
a *newly added* leaky field too, which is how `tokenEnvValue` slipped in.

**Step 2–4:** verify fail → implement → verify pass → `cargo test --workspace`.

**Step 5: Commit** — `git commit -m "fix(settings): stop returning secrets from settings reads"`

---

### Task 5: Frontend — types, PIN modal, write-only Gemini key

**Files:**
- Modify: [types.ts](../../src/lib/types.ts), [api.ts](../../src/lib/api.ts) — drop `tokenEnvValue`/`geminiApiKey`/`getLocalSettingsForHa`, add `revealSecret(field, pin?)`
- Modify: [settings/+page.svelte](../../src/routes/settings/+page.svelte) — eye buttons call reveal; PIN modal in server mode; Gemini field write-only
- Modify: [sk/index.ts](../../src/lib/i18n/sk/index.ts), [en/index.ts](../../src/lib/i18n/en/index.ts) — modal strings; regenerate types with `npx typesafe-i18n --no-watch`

Behaviors: re-mask/navigate-away clears the revealed value; a failed reveal keeps the
modal open with the backend's message; `saveReceiptSettingsNow` sends the Gemini key
**only when non-empty**, so a blank untouched field can't wipe a configured key.

**Verify:** `npm run check` → 0 errors.

**Commit** — `git commit -m "feat(settings): PIN prompt for revealing secrets"`

---

### Task 6: Integration coverage

**Files:**
- Modify: [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) — add `KNIHA_JAZD_REVEAL_PIN` to `ENV_PINNED_FIXTURE`, and to `SCRUBBED_ENV` for normal runs
- Modify: [env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts) — the task-68 "eye reveals the live token" case now expects a PIN prompt

Cases: eye opens the modal; wrong PIN shows an error and reveals nothing; correct PIN
reveals; re-masking and re-clicking prompts again.

**Verify:** `npm run test:integration:server:env`, then `npm run test:integration:server` (must stay green — the Gemini-key change touches [receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts)).

**Commit** — `git commit -m "test: cover PIN-gated reveal in server mode"`

---

### Task 7: Documentation

- [CHANGELOG.md](../../CHANGELOG.md) — user-visible, under `[Unreleased]`.
- [DECISIONS.md](../../DECISIONS.md) — **ADR-027**, explicitly revising the trade-off
  paragraph of ADR-025 ("readable by anyone who can reach the app") and recording why
  CORS is not an access control.
- [settings-architecture.md](../../docs/features/settings-architecture.md) and
  [server-mode.md](../../docs/features/server-mode.md) — document `KNIHA_JAZD_REVEAL_PIN`.
- [index.md](../index.md) — task 69 row.

**Commit** — `git commit -m "docs: record PIN-gated reveal (ADR-027)"`

---

## Risks

- **`get_local_settings_for_ha` may have an unknown consumer.** Verified dead in
  [src/](../../src/) — only [api.ts](../../src/lib/api.ts) mentions it. Re-grep before deleting.
- **Gemini field regression.** Making it write-only can wipe a stored key if the save
  path sends an empty string; Task 5 addresses it and
  [receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts) must pass.
- **Lockout during integration runs.** Specs that submit a wrong PIN then a correct one
  must stay under 5 failures per server instance.
