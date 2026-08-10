**Date:** 2026-08-10
**Subject:** Require a PIN to reveal secrets, and stop serving them to the network
**Status:** Planning

## Background

[Task 68](../68-env-managed-settings-ui/01-task.md) made env-managed settings visible
in the UI and, per [ADR-025](../../DECISIONS.md), let the eye icon reveal the live
value of an env-pinned token. The reasoning was that the operator already controls
those values through the deployment, and that this matched the LAN/tailnet trust
model of [ADR-017](../../DECISIONS.md).

That reasoning is too generous. The trust model it leans on is enforced by a **CORS
allowlist**, and CORS is a browser control: it constrains what a *web page* from
another origin may do. It does nothing about a direct HTTP client. Verified against a
running instance — `curl` with no `Origin` header reaches every RPC command:

```
POST /api/rpc  {"command":"get_receipt_settings","args":{}}
→ {"geminiApiKey":"AIza…", …}
```

So on the always-on homelab deployment ([ADR-024](../../DECISIONS.md)), anyone who
can reach the app — LAN or tailnet, any device, no browser required — can read the
secrets. Three commands hand them out with no challenge at all:

| Command | Leaks |
|---------|-------|
| `get_receipt_settings` | `geminiApiKey`, in full, always (file or env) |
| `get_local_settings_for_ha` | `haApiToken`, in full, always |
| `get_ha_settings` / `get_paperless_settings` | `tokenEnvValue` when env-pinned (added by task 68) |

`get_local_settings_for_ha` is the worst of the three and the easiest to fix: it is
**dead code**. It exists to let the frontend call Home Assistant directly, but the
frontend routes through the backend instead ([`test_ha_connection`](../../src-tauri/core/src/server/dispatcher_async.rs),
`fetch_ha_odo`), and nothing outside [api.ts](../../src/lib/api.ts) references it.

## User-visible goals

1. **Revealing a secret requires a PIN when the request comes over the network.**
   The PIN is operator-supplied via an environment variable and must be entered on
   **every** reveal — no session, no caching, no "remember for 5 minutes".

2. **The local desktop app reveals without a PIN.** A user sitting at the machine
   running the Tauri app is already past every boundary a PIN would defend. The
   distinction is structural, not a flag: the Tauri command path is not reachable
   from the network.

3. **Reveal is unavailable on the server until a PIN is configured.** No variable
   set means no reveal, rather than falling back to the current open behavior. The
   server still starts normally — refusing to boot would take the homelab down on
   upgrade over a setting that only gates an eye icon.

4. **Ordinary settings reads stop carrying secrets entirely.** After this task, no
   command returns a secret except the one dedicated reveal command. The Gemini key
   becomes write-only in the UI, matching how the HA and Paperless tokens already
   behave (`hasToken` + `********` placeholder).

5. **Wrong PINs are throttled.** The PIN is short (4 characters — 10,000
   combinations), so an unthrottled endpoint on the tailnet is exhausted in seconds
   and the gate is decoration. Repeated failures lock reveal out for escalating
   periods.

## Non-goals

- **General authentication for the app.** [ADR-017](../../DECISIONS.md)'s no-login
  model stands; trip data, vehicles, and invoices remain readable by anyone on the
  trusted network. This task is only about credentials, which are qualitatively
  different: they grant access to *other* systems (Google, Home Assistant,
  Paperless) far beyond this app's own data.
- **Encrypting secrets at rest.** They stay in `local.settings.json` / env vars.
- **Per-user PINs, roles, or an audit trail.** Single operator, single PIN.

## Accepted consequences

- Losing the PIN means losing in-app reveal. The values remain readable where they
  were set — the compose file, the env, or `local.settings.json` — so nothing is
  permanently inaccessible.
- A determined attacker on the tailnet can lock the operator out of reveal by
  burning attempts. On a closed, owner-controlled network that is the right failure
  direction: denial beats disclosure.

## Related

- [ADR-025](../../DECISIONS.md) — the decision this revises.
- [ADR-017](../../DECISIONS.md) — LAN-only CORS without authentication.
- [ADR-024](../../DECISIONS.md) — homelab server as canonical deployment.
- [docs/features/settings-architecture.md](../../docs/features/settings-architecture.md)
