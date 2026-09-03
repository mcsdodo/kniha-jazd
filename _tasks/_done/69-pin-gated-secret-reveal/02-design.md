**Date:** 2026-08-10
**Subject:** Require a PIN to reveal secrets, and stop serving them to the network
**Status:** Complete

# Design — PIN-gated secret reveal

Background and goals: [01-task.md](./01-task.md).

## The core idea: the entry point is the trust boundary

There is no flag or header a caller can set to claim "I am local". Locality is
decided by *which code path the request arrived on*, and the two paths are already
separate:

```
Tauri window ──► desktop/src/commands/*  ──► RevealAuth::LocalTrusted ──┐
                                                                        ├──► reveal_secret_internal
Browser / curl ─► POST /api/rpc ─► dispatcher ─► RevealAuth::Pin(pin) ──┘
```

The dispatcher — the only network-reachable path — has no way to construct
`LocalTrusted`, because it never has a reason to. This is why the desktop exemption
is safe: it isn't a permission check that could be bypassed, it's a code path that
the network cannot reach.

## Backend

### `reveal.rs` (new, in core)

```
pub enum SecretField { GeminiApiKey, HaApiToken, PaperlessApiToken }   // serde camelCase

pub enum RevealAuth { LocalTrusted, Pin(String) }

pub fn reveal_secret_internal(
    app_dir: &Path,
    app_state: &AppState,
    field: SecretField,
    auth: RevealAuth,
) -> Result<String, String>
```

An **enum**, not a string field name, so the command cannot be aimed at arbitrary
settings — `reveal_secret("custom_db_path")` is not expressible.

Resolution order for `Pin`:

1. **Throttle check.** Locked out → `Err` naming the remaining seconds. Checked
   *before* the comparison so a lockout can't be probed for timing.
2. **PIN configured?** `KNIHA_JAZD_REVEAL_PIN` unset/blank → `Err("reveal is
   disabled: KNIHA_JAZD_REVEAL_PIN is not set")`. Distinct from "wrong PIN":
   misconfiguration and rejection are different problems for the operator, and the
   distinction leaks nothing an attacker can act on.
3. **Constant-time compare.** Byte-wise accumulate-difference over equal-length
   padding, never `==` on the strings.
4. On mismatch record the failure and `Err("incorrect PIN")`; on match reset the
   counter and return the value.

`LocalTrusted` skips 1–4 entirely.

The returned value is the **effective** one — `LocalSettings::load_effective`, so an
env override wins. The point of revealing is to see what is actually live.

### Throttle

```
pub struct RevealThrottle { consecutive_failures: u32, locked_until: Option<Instant> }
```

Held as a `Mutex` on [`AppState`](../../src-tauri/core/src/app_state.rs), which both
frontends already share, so tests get a fresh instance instead of fighting over a
global.

Policy — **5 failures, then escalating lockout**: 60s, 5min, 15min, 60min (capped).
Any success resets both fields. The counter is **global, not per-IP**: a per-IP
counter is defeated by rotating source addresses, which is trivial on a LAN. The
cost is that an attacker can lock the operator out; on a closed network that trade
is correct, and it is recorded in [01-task.md](./01-task.md).

With a 4-character PIN, 5 attempts per minute means the 10,000-value space takes
years rather than seconds. Throttling is what makes a short PIN viable at all.

### Removing the ambient exposure

| Change | File |
|--------|------|
| Drop `token_env_value` from `HaSettingsResponse` and `PaperlessSettingsResponse` | [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) |
| `ReceiptSettings.gemini_api_key` → `has_gemini_api_key: bool` | [receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) |
| Delete `get_local_settings_for_ha` + `HaLocalSettingsResponse` | [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs), [desktop wrapper](../../src-tauri/desktop/src/commands/integrations.rs), [lib.rs](../../src-tauri/desktop/src/lib.rs), [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs), [api.ts](../../src/lib/api.ts) |

The `*FromEnv` booleans from task 68 **stay** — they say nothing secret and drive the
disabled state and badges.

## Frontend

### Reveal flow

The eye button stops being a pure CSS toggle and becomes an action:

- **Desktop** (`$capabilities.mode === 'desktop'`) — click calls `reveal_secret`
  and swaps the input to `type="text"` with the returned value.
- **Server** — click opens a small PIN modal. On submit, call with the PIN; on
  success reveal, on failure show the backend's message inline and keep the modal
  open.

Re-masking (second click), navigating away, or a failed reveal all clear the value
from component state. Nothing is cached, so the next reveal prompts again — goal 1.

### Gemini key becomes write-only

The receipts section currently binds `geminiApiKey` to an editable input seeded with
the stored key. With the key no longer returned, it behaves like the HA and Paperless
token fields: `********` placeholder when `hasGeminiApiKey`, and
`saveReceiptSettingsNow` sends the key **only when the user typed one**, so an
untouched blank field can no longer wipe a configured key.

### A side effect worth stating

Today the eye is a no-op for file-stored tokens, because those are never sent. Routed
through `reveal_secret` it starts working for them — the icon becomes consistently
meaningful, and on the network it is PIN-gated like everything else.

## Test ownership

| Use-case | Backend unit | Integration |
|----------|--------------|-------------|
| `LocalTrusted` bypasses the PIN | ✅ | ❌ |
| Correct PIN returns the effective value (env beats file) | ✅ | ❌ |
| Wrong PIN / unset PIN produce distinct errors | ✅ | ❌ |
| Each `SecretField` maps to the right setting | ✅ | ❌ |
| Lockout after 5 failures; success resets it | ✅ | ❌ |
| **Settings responses carry no secret** | ✅ (regression guard) | ❌ |
| `reveal_secret` over RPC needs a PIN | ✅ (dispatcher) | ❌ |
| PIN modal appears and reveals in the browser | ❌ | ✅ |

The "responses carry no secret" tests are the ones that matter long-term — they are
what stops a future field from quietly reopening the hole, exactly as `tokenEnvValue`
did.

Integration coverage extends the existing env-pinned suite
([env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts)),
which already boots a server with fixture env vars; it gains
`KNIHA_JAZD_REVEAL_PIN`.

## Rejected alternatives

- **Per-IP throttling.** Defeated by rotating source addresses on a LAN.
- **A session token after one PIN entry.** Directly contradicts "entered on every
  reveal", and a stolen session token is a bearer credential with none of the PIN's
  throttling.
- **Refusing to start without a PIN.** Turns a forgotten variable into homelab
  downtime; a disabled eye icon is the proportionate failure.
- **Reveal the variable *name* instead of the value.** Already rejected in
  [ADR-025](../../DECISIONS.md) for good reasons; the PIN addresses the actual
  objection (network exposure) without giving up the useful behavior.
