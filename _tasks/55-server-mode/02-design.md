**Date:** 2026-03-07 (revised 2026-04-22 after design review)
**Subject:** Server Mode - Technical Design
**Status:** Implementation plan written (03-plan.md)

## Architecture Overview

Embed an Axum HTTP server inside the running Tauri app. Both Tauri IPC and HTTP share the same `Database` and `AppState` instances (via `Arc`).

### Data Flow

```
Desktop User                          LAN Browser User
     │                                       │
     ▼                                       ▼
┌─────────────┐                    ┌──────────────────┐
│ Tauri Webview│                    │ Browser (phone)  │
│ invoke()     │                    │ fetch('/api/..') │
└──────┬──────┘                    └────────┬─────────┘
       │                                    │
       │ IPC                          HTTP (port 3456)
       │                                    │
       ▼                                    ▼
┌──────────────────────────────────────────────────┐
│              Tauri App Process                   │
│                                                  │
│  ┌────────────────┐    ┌──────────────────────┐  │
│  │ Tauri Commands  │    │ Axum HTTP Server     │  │
│  │ (thin wrappers) │    │ (RPC dispatcher)     │  │
│  └───────┬────────┘    └──────────┬───────────┘  │
│          │                        │              │
│          └──────────┬─────────────┘              │
│                     ▼                            │
│        ┌──────────────────────┐                  │
│        │ *_internal pure fns   │                  │
│        │ (shared logic)        │                  │
│        └──────────┬───────────┘                  │
│                   ▼                              │
│        ┌──────────────────────┐                  │
│        │ Shared Business Logic │                  │
│        │ (db.rs, calculations) │                  │
│        └──────────┬───────────┘                  │
│                   ▼                              │
│        ┌──────────────────────┐                  │
│        │ SQLite (Mutex<Conn>) │                  │
│        └──────────────────────┘                  │
└──────────────────────────────────────────────────┘
```

## Key Design Decisions

### 1. Shared Database Instance (not a second connection)

The HTTP server uses the **same** `Arc<Database>` that Tauri commands use. Verified shape in `db.rs:48-49`:

```rust
pub struct Database {
    conn: Mutex<SqliteConnection>,
}
```

The single mutex serializes everything — no locking conflicts, no connection pool needed. Setup stays in `lib.rs:130-131`:

```rust
let db = Arc::new(Database::new(path)?);
let app_state = Arc::new(AppState::new());

app.manage(db.clone());           // Tauri IPC side
app.manage(app_state.clone());

// New: HTTP server gets the same Arcs
let server_handle = HttpServer::spawn(db.clone(), app_state.clone(), shutdown_rx);
```

### 2. Two-layer command architecture (`_internal` extraction)

**Resolves C1 from design review.** Existing Tauri commands take `tauri::State<Database>` wrappers, so the RPC dispatcher cannot call them directly. We refactor each command into two pieces — a pure `_internal` function that takes `&Database` / `&AppState`, and a thin Tauri wrapper:

```rust
// src-tauri/src/commands/vehicles.rs

// Pure function: callable from Tauri wrapper AND RPC dispatcher
pub fn create_vehicle_internal(
    db: &Database,
    app_state: &AppState,
    name: String,
    license_plate: String,
    initial_odometer: f64,
    vehicle_type: Option<String>,
    // ... other args
) -> Result<Vehicle, String> {
    check_read_only!(app_state);
    let vehicle = db.create_vehicle(/* ... */).map_err(|e| e.to_string())?;
    Ok(vehicle)
}

// Thin Tauri wrapper: unwraps State<T>, delegates to _internal
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_vehicle(
    db: State<Database>,
    app_state: State<AppState>,
    name: String,
    license_plate: String,
    initial_odometer: f64,
    vehicle_type: Option<String>,
    // ...
) -> Result<Vehicle, String> {
    create_vehicle_internal(&db, &app_state, name, license_plate, initial_odometer, vehicle_type /* ... */)
}
```

Existing precedent: `cleanup_pre_update_backups_internal` (called from `lib.rs:143`) and `assign_receipt_to_trip_internal` in `receipts_cmd.rs:392`. This just extends the pattern to everything.

**Scope:** ~71 commands across 8 modules (`vehicles.rs`, `trips.rs`, `receipts_cmd.rs`, `statistics.rs`, `export_cmd.rs`, `integrations.rs`, `settings_cmd.rs`, `backup.rs`). Plan should tackle this module-by-module in separate commits so review stays tractable.

**Read-only guard:** `_internal` fns call the existing `check_read_only!(app_state)` macro. No new `_web` variant — one enforcement point covers both entry channels.

### 3. RPC dispatcher

Single endpoint `POST /api/rpc` with body `{ command: string, args: object }`. Dispatcher matches on `command`, deserializes `args` into each command's parameter struct, calls the `_internal` fn, serializes the result:

```rust
#[derive(Deserialize)]
struct RpcRequest {
    command: String,
    args: serde_json::Value,
}

async fn rpc_handler(
    State(state): State<ServerState>,
    Json(req): Json<RpcRequest>,
) -> Response {
    let db = state.db.clone();
    let app_state = state.app_state.clone();

    let result = tokio::task::spawn_blocking(move || {
        dispatch_command(&req.command, req.args, &db, &app_state)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"));

    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(msg)) => (StatusCode::BAD_REQUEST, msg).into_response(),
        Err(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
    }
}

fn dispatch_command(
    command: &str,
    args: serde_json::Value,
    db: &Database,
    app_state: &AppState,
) -> Result<serde_json::Value, String> {
    // Helper: deserialize args into a typed params struct
    fn parse_args<T: DeserializeOwned>(args: serde_json::Value) -> Result<T, String> {
        serde_json::from_value(args).map_err(|e| format!("Invalid args: {e}"))
    }

    match command {
        "get_vehicles" => {
            let vehicles = get_vehicles_internal(db)?;
            Ok(serde_json::to_value(vehicles).unwrap())
        }
        "create_vehicle" => {
            #[derive(Deserialize)]
            struct Args {
                name: String,
                license_plate: String,
                initial_odometer: f64,
                vehicle_type: Option<String>,
                // ...
            }
            let a: Args = parse_args(args)?;
            let v = create_vehicle_internal(
                db, app_state,
                a.name, a.license_plate, a.initial_odometer, a.vehicle_type, /* ... */
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        // ... one arm per server-safe command
        _ => Err(format!("Unknown or Tauri-only command: {command}")),
    }
}
```

Why RPC, not REST: frontend invocations keep the same `(command, args)` shape that `invoke()` already uses, so the adapter is a 10-line swap. Verb-noun REST would force us to invent URL shapes for 71 operations — churn with no payoff for a single-client API.

### 4. Frontend API adapter

```typescript
// src/lib/api-adapter.ts

import { invoke } from '@tauri-apps/api/core';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function apiCall<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (IS_TAURI) {
    return invoke<T>(command, args);
  }

  const response = await fetch(`${window.location.origin}/api/rpc`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-KJ-Client': '1',   // CSRF mitigation — see §9
    },
    body: JSON.stringify({ command, args: args ?? {} }),
  });

  if (!response.ok) throw new Error(await response.text());
  return response.json();
}
```

All existing call sites migrate from `invoke(...)` to `apiCall(...)` with no signature change.

### 5. Tauri-only commands and capabilities endpoint

**Resolves I1.** Not every command survives the Tauri → HTTP boundary. Classification criteria:

| Category | Server-safe? | Reason |
|----------|-------------|--------|
| CRUD on DB (`get_vehicles`, `create_trip`, …) | ✅ Yes | DB + AppState only |
| Calculations (`get_trip_grid_data`, `calculate_trip_stats`) | ✅ Yes | Pure computation |
| App mode / read-only (`get_app_mode`, `get_db_location`) | ✅ Yes | Needed so browser UI can hide write controls |
| Theme preference (uses `AppHandle` only for data-dir) | ✅ Yes | Data-dir is resolvable from shared state |
| Receipt image (raw bytes) | ✅ Yes | Served at `/api/receipts/:id/image`, not RPC |
| File-picker dialogs (backup restore, move-database) | ❌ No | `tauri-plugin-dialog` has no browser equivalent |
| Updater commands | ❌ No | `tauri-plugin-updater` desktop-only by nature |
| Window management, `open::that(...)` | ❌ No | Meaningless over HTTP |

The plan's step 1 is to apply this classification to all 71 commands and mark each as `server-safe`, `tauri-only`, or `server-safe-with-caveat` (e.g. backup creation works but restore doesn't).

A dedicated endpoint advertises capabilities to the browser UI:

```typescript
GET /api/capabilities
→ {
    mode: "server",              // "server" | "desktop"  — matches IS_TAURI semantics
    read_only: false,
    features: {
      file_dialogs: false,
      updater: false,
      open_external: false,
    }
  }
```

Svelte feature-gates write-UI, updater banner, backup-restore button, etc. on `capabilities.features.*` and the existing `read_only` flag.

### 6. Graceful shutdown

**Resolves I2.** Shutdown is driven by Tauri's runtime event:

```rust
// In lib.rs
let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

app.on_run_event({
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));
    move |_app, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            if let Some(tx) = shutdown_tx.lock().unwrap().take() {
                let _ = tx.send(());
            }
        }
    }
});

// In the Axum spawn:
axum::serve(listener, app)
    .with_graceful_shutdown(async move {
        let _ = shutdown_rx.await;
        log::info!("Axum server shutting down");
    })
    .await?;
```

Benefits: in-flight requests drain cleanly, the port is released before process exit, the DB mutex isn't left poisoned.

### 7. Port selection and conflict handling

**Resolves I3.**

- **Default:** `3456`. Not in common use by dev tooling.
- **Configurable:** user can set any port 1024–65535 in Settings.
- **On bind failure:** surface a Slovak error in the Settings UI (`Port 3456 je obsadený. Zvoľte iný port.`) and leave the server **off**. Do **not** silently try `+1` — that produces confusing URL drift.
- **Persistence:** `local.settings.json` fields `server_enabled: bool` and `server_port: u16`. Stays host-local; survives DB relocation.
- **Auto-start:** if `server_enabled == true` at app launch, attempt to bind; on failure, log and display the error in Settings without blocking the desktop UI.

### 8. Static files + SPA fallback

**Resolves I5.** `svelte.config.js` already uses `adapter-static` with `fallback: 'index.html'`, producing a `build/` directory. Serve it via `tower-http::services::{ServeDir, ServeFile}`:

```rust
use tower_http::services::{ServeDir, ServeFile};

let static_dir = app_dir.join("dist");  // resolved at runtime
let static_service = ServeDir::new(&static_dir)
    .not_found_service(ServeFile::new(static_dir.join("index.html")));

let api_router = Router::new()
    .route("/rpc", post(rpc_handler))
    .route("/capabilities", get(capabilities_handler))
    .route("/receipts/:id/image", get(receipt_image_handler));

let app = Router::new()
    .nest("/api", api_router)
    .fallback_service(static_service);   // SPA fallback for deep links like /vozidla/abc
```

The `not_found_service` + `fallback_service` combo is what makes `/doklady` and `/vozidla/<uuid>` work on refresh.

### 9. Receipt image URL scheme

**Resolves I7.** Tauri resolves receipts via `convertFileSrc()` on absolute filesystem paths; browsers can't. Split into a shared helper:

```typescript
// src/lib/receipt-url.ts
import { convertFileSrc } from '@tauri-apps/api/core';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export function receiptImageUrl(receiptId: string, absolutePath?: string): string {
  if (IS_TAURI && absolutePath) {
    return convertFileSrc(absolutePath);
  }
  return `/api/receipts/${encodeURIComponent(receiptId)}/image`;
}
```

Backend responses always include the **receipt ID**. The absolute path is included only as an optimization for the Tauri path — browser clients ignore it.

### 10. CORS and CSRF posture

**Resolves I6.** `bind 0.0.0.0 + no auth + Allow-Origin: *` would let any public website visited on a LAN device POST to the RPC endpoint (DNS-rebinding / cross-site attack on private networks). Mitigations committed for MVP:

- **Origin allow-list** via `tower-http::cors::CorsLayer`:

  ```rust
  CorsLayer::new()
      .allow_origin(AllowOrigin::predicate(|origin, _| {
          let s = origin.to_str().unwrap_or("");
          s.starts_with("http://localhost")
              || s.starts_with("http://127.")
              || s.starts_with("http://10.")
              || s.starts_with("http://192.168.")
              // RFC1918 172.16.0.0/12 — first octet + 172.16-172.31
              || (s.starts_with("http://172.") && /* parse second octet 16..=31 */ true)
      }))
      .allow_methods([Method::GET, Method::POST])
      .allow_headers([header::CONTENT_TYPE, HeaderName::from_static("x-kj-client")])
  ```

- **Custom header `X-KJ-Client: 1`** required on all POSTs. Custom non-CORS-safelisted headers force a preflight, which the Origin allow-list rejects for non-LAN pages. Not cryptographic — it raises the bar above passive drive-by attacks.

- **Documented limits:** this protects against cross-site attacks originating from browsers visiting public sites. It does **not** protect against an already-compromised LAN device. That's called out in `01-task.md` Security Considerations.

## Implementation Plan (High-Level)

Ordering deliberately puts LAN exposure last — the server is never externally reachable until everything else is proven.

### Phase 1: Backend scaffold (bind to 127.0.0.1 only)
1. Add `axum`, `tower-http`, `local-ip-address`; expand `tokio` feature set.
2. Create `src-tauri/src/server/` module with `mod.rs`, `dispatcher.rs`, `capabilities.rs`, `receipts.rs`.
3. Implement graceful shutdown plumbing wired to `RunEvent::ExitRequested`.
4. RPC endpoint with `get_vehicles`, `get_app_mode`, `get_trips_for_year` to prove the dispatcher shape.

### Phase 2: `_internal` extraction (one module per commit)
5. Extract `*_internal` fns for `vehicles.rs`. Tauri command becomes a 2-line wrapper. Verify existing tests pass.
6. Repeat for `trips.rs`, `receipts_cmd.rs`, `statistics.rs`, `export_cmd.rs`, `integrations.rs`, `settings_cmd.rs`, `backup.rs`.
7. Fill the RPC dispatcher match with every server-safe command.

### Phase 3: Frontend adapter + capabilities
8. `src/lib/api-adapter.ts` with `IS_TAURI` detection. Migrate one page (e.g. `/vozidla`) as proof.
9. `/api/capabilities` endpoint; Svelte store reads it on load.
10. Migrate remaining pages to `apiCall`. Feature-gate Tauri-only UI (updater banner, backup restore, move-database) behind capabilities.

### Phase 4: Static files + receipts
11. `ServeDir` + SPA fallback. `app_dir.join("dist")` resolves at runtime from Tauri's `frontendDist` path.
12. `GET /api/receipts/:id/image` handler. Shared `receiptImageUrl()` helper. Migrate all `<img>` sites.

### Phase 5: Settings UI
13. Server Mode card on Settings page: toggle, port input, status pill, URL display using `local_ip_address::local_ip()`.
14. Port-conflict error surfacing. Persist to `local.settings.json`.
15. Auto-start on launch if `server_enabled`.

### Phase 6: Security + ship
16. CORS allow-list for LAN Origins. `X-KJ-Client` header requirement.
17. **Switch bind from 127.0.0.1 to 0.0.0.0 behind the toggle** — last step.
18. README/docs update: "Server Mode" page in `docs/features/`.

### Testing (interleaved, not a separate phase)
- **Dispatcher unit tests** in `server/dispatcher_tests.rs`: unknown command → error, malformed args → 400, read-only + write command → Slovak error, every server-safe command has at least one round-trip test.
- **HTTP integration tests** (new harness in `tests/server/`): spawn the server, hit `/api/rpc` + `/api/capabilities` + `/api/receipts/:id/image` + SPA fallback. Run in CI on all platforms.
- **WebdriverIO stays Tauri-only** — browser-mode E2E would duplicate what dispatcher + integration tests already prove.

## Resolved Decisions

| Question | Decision |
|----------|----------|
| **Port** | Default 3456, user-configurable, fail loudly on conflict (§7). |
| **Auto-start** | Yes if `server_enabled == true`; silent failure logged + surfaced in Settings. |
| **Receipt images** | HTTP endpoint with receipt ID; shared URL helper (§9). |
| **Tauri-only features in browser** | Capability gating via `/api/capabilities` (§5). |
| **RPC vs REST** | RPC (§3) — minimizes frontend churn and route-maintenance cost. |
| **Command reuse** | `_internal` extraction (§2) — single source of truth, shared with Tauri wrappers. |
| **Shutdown** | `tokio::sync::oneshot` driven by Tauri `RunEvent::ExitRequested` (§6). |
| **CORS/CSRF** | LAN-origin allow-list + `X-KJ-Client` header (§10). |
| **Auth** | None in MVP; deferred to a follow-up task. |
| **Relationship with Task 33** | Task 33's binary can reuse this `server/` module verbatim; it just wraps it in a standalone binary instead of embedding in Tauri. |
