**Date:** 2026-04-22
**Subject:** Design review for Task 55 — Server Mode
**Status:** Review (pre-implementation-plan)
**Reviewer:** Claude (Opus 4.7)

## Recommendation

**Needs revisions before writing the implementation plan.** The direction is sound and fits ADR-008 well, but several load-bearing claims don't match the codebase as-is, and a few architectural decisions (RPC vs REST, how to reuse existing command bodies, graceful shutdown, CSRF/CORS posture) are left unresolved. Resolve the Critical items and the plan can proceed.

Verified against codebase state at commit `02f429a`.

---

## Critical — must resolve before writing the plan

### C1. RPC dispatcher can't call existing Tauri command functions directly

The design example at `02-design.md:213-225` suggests:

```rust
"get_vehicles" => {
    let vehicles = db.get_vehicles()?;
    ...
}
"create_trip" => {
    check_read_only_web(app_state)?;
    let trip = db.create_trip(/* ... */)?;
}
```

But every existing Tauri command takes `tauri::State<Database>` / `tauri::State<AppState>` wrappers, not `&Database` / `&AppState`:

```
src-tauri/src/commands/vehicles.rs:16
  pub fn get_vehicles(db: State<Database>) -> Result<Vec<Vehicle>, String>
src-tauri/src/commands/vehicles.rs:25-29
  pub fn create_vehicle(db: State<Database>, app_state: State<AppState>, ...)
```

So the dispatcher can't "just call" the command; it has to call the **lower layer** (`db.get_all_vehicles()`, `db.create_vehicle(...)`) and re-implement the argument unpacking and read-only guard itself. With 71 commands, the "one big match" and "71 individual routes" save you roughly the same amount of code — the savings from an RPC approach are largely illusory at this layer.

**Options to decide between:**

a) **Extract `_internal` fns for every command** (the pattern already exists — see `cleanup_pre_update_backups_internal` invoked from `lib.rs:143`). Each Tauri command becomes a two-line wrapper around a pure fn. The RPC dispatcher calls the same pure fn. DRY, testable, but a ~71-function refactor.

b) **Accept duplication**: RPC dispatcher calls `db.*` / `app_state.*` directly, re-implementing arg deserialization. Faster to land; drift risk forever.

c) **Hybrid**: extract `_internal` only for commands that do non-trivial work (calculations, grid data, compensation); trivial CRUD stays duplicated.

Pick one and document in the plan. I'd lean (a), spread across several commits grouped by command module.

### C2. Axum needs tokio features not currently enabled

Current dependency in `src-tauri/Cargo.toml:39`:

```toml
tokio = { version = "1", features = ["fs"] }
```

Axum 0.8 requires `rt-multi-thread`, `macros`, `net`, and (for graceful shutdown) `signal`. The design says "tokio already present" (`01-task.md:144`) — technically true, but misleading; the plan must explicitly enable:

```toml
tokio = { version = "1", features = ["fs", "rt-multi-thread", "macros", "net", "signal"] }
```

Worth noting because (i) it grows the binary, (ii) it affects Tauri's existing single-threaded assumptions if any, (iii) build-time jumps measurably.

### C3. MVP scope contradicts the design's own recommendation

`01-task.md:118` ("REST API routes mirroring all Tauri commands") is incompatible with `02-design.md:178` ("RPC-Style Backend Handler — Recommended"). Both surfaces in the same planning folder say different things. Pick one — my read is the RPC single endpoint is the right call for v1 (minimal frontend churn), but then `01-task.md` should be updated to say so rather than promising REST routes that won't exist.

---

## Important — decide these before coding starts

### I1. Tauri-only commands need an explicit list, not "hide in browser"

`02-design.md:274-279` handles this with a bulleted list at the category level ("window management → Hide in browser"). In practice, commands touching `tauri::AppHandle`, `tauri::Window`, `tauri-plugin-dialog`, `tauri-plugin-updater`, or the filesystem-via-dialog pattern **cannot** be trivially wrapped. Grep for `AppHandle` / `Window` parameters in `commands/*.rs` and enumerate:

- Backup restore (file dialog)
- Database location change (folder picker)
- Updater
- "Open folder" / `open` crate calls
- Any `get_window(...)` / `.show()` / `.hide()` calls

The dispatcher should either return a structured `Error::NotAvailableInServerMode` or the command should be absent from the `COMMAND_MAP`. Frontend then feature-gates UI on a single `/api/capabilities` endpoint rather than scattering `IS_TAURI` checks everywhere.

### I2. Graceful shutdown is missing

When the Tauri window closes, Axum must stop listening and drain in-flight requests. The standard pattern is:

```rust
let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
axum::serve(listener, app)
    .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
    .await?;
```

…plumbed to Tauri's `RunEvent::ExitRequested` / window-close. Without this, the port can stay bound or in-flight requests can leave the DB mutex poisoned. Not in the design at all.

### I3. Port conflict handling

Port 3456 will be in use some of the time. The open question in `02-design.md:265-266` needs a concrete answer in the plan:

- Default 3456, configurable in settings (design's recommendation)
- On bind failure: fail with a Slovak error surfaced in the Settings UI, don't silently try another port (avoid confusing URL changes)

### I4. Read-only state must be discoverable over HTTP

`check_read_only!` guards writes on the server side (good — `commands/mod.rs:63`). But the **browser UI** also needs to hide/disable write controls, exactly like the desktop does today via the read-only mode store. The plan must:

1. Include `get_app_mode` (or similar) in the RPC command whitelist.
2. Confirm the frontend mode store reads the same value via the adapter.

Otherwise you'll get "Save" buttons in the browser that surface a Slovak error after every click.

### I5. SPA fallback for ServeDir

`adapter-static` produces a `build/` tree with `index.html` as the SPA shell; Svelte routes like `/doklady` or `/vozidla/abc` don't have matching files on disk. `tower-http`'s `ServeDir` returns 404 for those by default. Design mentions "Handle SPA routing (fallback to index.html)" as a Phase-4 bullet but no implementation sketch. Concretely:

```rust
let static_svc = ServeDir::new("dist")
    .not_found_service(ServeFile::new("dist/index.html"));
```

Call this out in the plan so nobody spends a morning chasing 404s on refresh.

### I6. CORS + bind-0.0.0.0 + no-auth is a real attack surface

`02-design.md:136` is honest — "No auth - acceptable for home/office LAN" — but with `Allow-Origin: *` a malicious website visited on any LAN device can POST to `http://<host>:3456/api/rpc` from the victim's browser. That is not theoretical; it's how DNS-rebinding attacks work against private-network services.

Minimum mitigations to pick from (the plan should commit to at least one, even if it's "accept the risk"):

- **Origin/Referer allowlist**: only accept requests whose `Origin` header matches a LAN IP range.
- **Require a custom header** (e.g. `X-KJ-Client: 1`) — trivially bypassed by a determined attacker but defeats passive CSRF from browser-based XHR because custom headers trigger preflight.
- **Optional PIN**: listed as "future" in `01-task.md:137` — consider promoting to MVP if the app touches financial data (which it does, via fuel costs and tax compliance).

Also: CORS in `tower-http` needs explicit `allow_methods`, `allow_headers` — not just `allow_origin(Any)`.

### I7. Receipt image URL scheme divergence

Tauri resolves receipt images via `convertFileSrc()` on absolute FS paths. Browser can't do that. The plan needs a clear split:

- Backend: `GET /api/receipts/:id/image` (serves the file with correct `Content-Type`)
- Frontend: a single helper `receiptImageUrl(id)` that returns `convertFileSrc(path)` in Tauri mode and `/api/receipts/:id/image` in HTTP mode
- Backend responses should always include the **ID** so the frontend can build the right URL; don't return absolute FS paths to a browser client

Design flags this at a high level (`02-design.md:272`) but doesn't specify the URL shape or the frontend helper.

---

## Minor

- **M1. Lock file is fine as-is** — the heartbeat thread runs in the desktop process (`lib.rs:109-119`) regardless of whether the user is driving via Tauri or via browser. Worth one sentence in the design confirming this so a future reader doesn't worry about lock expiry while the app is "browser-only in use."
- **M2. Local IP detection not specified.** The `hostname` crate (already in `Cargo.toml:43`) doesn't give you the LAN IP. Consider the `local_ip_address` crate, or enumerate interfaces and skip loopback/docker. Multiple interfaces (Wi-Fi + Ethernet + VPN) are common — decide whether to show one (default-route) or a list.
- **M3. Persistence location.** Server-enabled flag + port belong in `local.settings.json` (already per-machine), not the SQLite DB — the DB follows the data around when users relocate it, but server prefs are host-local.
- **M4. Naming drift.** Design uses `SharedState`, `AppState`, `State<AppState>` interchangeably in the Rust snippets. Unify before the plan to avoid confusion during implementation.
- **M5. Invented helper `check_read_only_web`** (`02-design.md:218`) — reuse the existing `check_read_only!` macro, don't fork a web variant.
- **M6. Test strategy absent.** TDD is mandatory (CLAUDE.md). Plan should commit to: (i) unit tests for the RPC dispatcher (arg deserialization, error propagation, unknown command, read-only guard), (ii) a handful of HTTP-level integration tests (one GET, one POST-RPC, one image serve, one read-only write rejection), (iii) an explicit decision on whether WebdriverIO runs in browser mode too or stays Tauri-only. My lean: keep WebdriverIO Tauri-only; add a thin Playwright suite against the running server.
- **M7. Port choice.** 3456 is arbitrary. No conflict with common dev tooling (Vite 5173, pgAdmin 5050, Grafana 3000), so fine — just note the rationale in the plan so it isn't re-litigated.

---

## What's good

- **Correct shared-state primitive.** `Arc<Database>` + `Mutex<SqliteConnection>` is the natural choice and matches the current implementation exactly (`db.rs:48-49`).
- **`spawn_blocking` for Diesel** is the right call — Diesel is sync, and Axum handlers are async; off-loading keeps the runtime unblocked.
- **ADR-008 alignment.** Business logic stays in Rust; HTTP is a second input channel, not a second source of truth.
- **Clear MVP/exclude split** (`01-task.md:113-130`). Prevents scope creep from auth/HTTPS/mDNS being snuck into v1.
- **Good comparison to Task 33** and honest note that 33's binary can reuse this module — worth carrying forward.
- **`adapter-static` is already configured** — no build pipeline changes needed to serve the frontend from Axum.

---

## Suggested plan ordering (when you get to writing it)

1. Dependencies + empty Axum scaffold behind a feature flag, binding to 127.0.0.1 at first.
2. RPC endpoint with 2-3 read commands (`get_vehicles`, `get_trips_for_year`, `get_app_mode`) to prove the dispatcher shape.
3. Extract-`_internal` refactor for the first command module (vehicles) — establish the pattern.
4. Frontend `api-adapter.ts` + `IS_TAURI` detection, used by exactly one route to prove the dual-mode works.
5. Static file serving + SPA fallback.
6. Fill in the rest of the commands, module by module.
7. Settings UI (toggle, URL display, port).
8. Graceful shutdown + port-conflict handling.
9. Decide + implement CORS/CSRF posture.
10. Tests: dispatcher units, HTTP integration, capabilities endpoint.
11. Switch bind from 127.0.0.1 to 0.0.0.0 behind the "Enabled" toggle.

Step 11 last deliberately — so the server is never exposed to the LAN while half-built.
