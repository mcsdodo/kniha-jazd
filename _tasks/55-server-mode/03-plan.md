# Server Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Embed an Axum HTTP server in the Tauri desktop app so any browser on the local network can access the full UI.

**Architecture:** Single process, shared state. Axum uses the same `Arc<Database>` and `Arc<AppState>` as Tauri IPC. All commands refactored to `_internal` pure functions callable from both Tauri wrappers and the RPC dispatcher. Frontend detects its runtime and calls `invoke()` or `fetch('/api/rpc')` transparently.

**Tech Stack:** Axum 0.8, tower-http 0.6 (CORS + static files), tokio (multi-threaded), local-ip-address 0.6

**Design docs:** `_tasks/55-server-mode/01-task.md`, `02-design.md`, `_design-review.md`

---

## Prerequisites

Before starting implementation:
- All 195 backend tests pass: `cd src-tauri && cargo test`
- Integration tests pass: `npm run test:integration:tier1`
- Clean git status on a feature branch

## Command Classification

**71 total commands across 8 modules.** 67 are server-safe, 4 are Tauri-only.

### Tauri-Only Commands (4)

These use desktop-only features (file dialogs, browser launch, live DB replacement):

| Command | Module | Reason |
|---------|--------|--------|
| `export_to_browser` | export_cmd.rs | Opens desktop browser via `open::that()` |
| `move_database` | settings_cmd.rs | Uses `tauri-plugin-dialog` folder picker |
| `reset_database_location` | settings_cmd.rs | Directory copy + app restart |
| `restore_backup` | backup.rs | Replaces running DB — too dangerous over HTTP |

### Server-Safe Commands (67)

All other commands are server-safe. They fall into two extraction patterns:

**Pattern A — DB + AppState only (35 commands):**
`_internal` takes `&Database` and/or `&AppState`. Simplest extraction.

- **vehicles.rs (6):** get_vehicles, get_active_vehicle, create_vehicle, update_vehicle, delete_vehicle, set_active_vehicle
- **trips.rs (10):** get_trips, get_trips_for_year, get_years_with_trips, create_trip, update_trip, delete_trip, reorder_trip, get_routes, get_purposes, get_inferred_trip_time_for_route
- **receipts_cmd.rs (10):** get_receipts, get_receipts_for_vehicle, get_unassigned_receipts, update_receipt, delete_receipt, unassign_receipt, revert_receipt_override, assign_receipt_to_trip *(has _internal)*, get_trips_for_receipt_assignment *(has _internal)*, verify_receipts *(has _internal)*
- **statistics.rs (3):** calculate_trip_stats, calculate_magic_fill_liters, preview_trip_calculation
- **settings_cmd.rs (6):** get_settings, save_settings, get_optimal_window_size, get_db_location, get_app_mode, check_target_has_db

**Pattern B — AppHandle → app_dir (32 commands):**
`_internal` takes `app_dir: &Path` instead of `app: AppHandle`. The AppHandle was only used to resolve the app data directory.

- **settings_cmd.rs (8):** get_theme_preference, set_theme_preference, get_auto_check_updates, set_auto_check_updates, get_date_prefill_mode, set_date_prefill_mode, get_hidden_columns, set_hidden_columns
- **receipts_cmd.rs (7):** get_receipt_settings, set_gemini_api_key, set_receipts_folder_path, scan_receipts, sync_receipts *(async)*, process_pending_receipts *(async)*, reprocess_receipt *(async)*
- **backup.rs (10):** create_backup, create_backup_with_type, get_cleanup_preview, cleanup_pre_update_backups *(has _internal)*, get_backup_retention, set_backup_retention, list_backups, get_backup_info, delete_backup, get_backup_path
- **export_cmd.rs (1):** export_html *(async)*
- **statistics.rs (1):** get_trip_grid_data (AppHandle used for HA push — pass `app_dir: Option<&Path>`)
- **integrations.rs (5):** get_ha_settings, get_local_settings_for_ha, save_ha_settings, test_ha_connection *(async)*, fetch_ha_odo *(async)*

**Async commands (7):** sync_receipts, process_pending_receipts, reprocess_receipt, test_ha_connection, fetch_ha_odo, export_html, export_to_browser (Tauri-only, skip). These need `async fn _internal` and are `.await`ed directly in the RPC dispatcher (not `spawn_blocking`).

---

## Task 1: Dependencies + Empty Server Module

**Goal:** Add Axum dependencies, create an empty `server/` module, verify the project compiles and all tests pass.

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/server/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod server;`)

**Steps:**

1. Add dependencies to `src-tauri/Cargo.toml`:

```toml
axum = "0.8"
tower-http = { version = "0.6", features = ["cors", "fs"] }
local-ip-address = "0.6"
tokio = { version = "1", features = ["fs", "rt-multi-thread", "macros", "net", "signal", "sync"] }
```

Note: tokio currently only has `["fs"]`. Axum 0.8 needs multi-threaded runtime + networking + signal for graceful shutdown.

2. Create `src-tauri/src/server/mod.rs`:

```rust
//! Embedded HTTP server for LAN browser access.
//!
//! When enabled, serves the same UI and RPC API that the Tauri webview uses,
//! allowing phones/tablets/other PCs to access the app over the local network.
```

3. Add `mod server;` to `src-tauri/src/lib.rs` after the existing module declarations (line 13).

**Verify:**
```bash
cd src-tauri && cargo build 2>&1 | tail -5    # compiles without errors
cd src-tauri && cargo test 2>&1 | tail -3     # all 195 tests pass
```

**Commit:** `chore: add axum dependencies and empty server module`

---

## Task 2: Axum Server Scaffold + Graceful Shutdown

**Goal:** Create an `HttpServer` that binds to 127.0.0.1, serves a `/health` endpoint, and shuts down cleanly when the Tauri app exits. Prove with a test.

**Files:**
- Modify: `src-tauri/src/server/mod.rs`
- Modify: `src-tauri/src/lib.rs` (wire shutdown channel)

**Steps:**

1. Write the failing test first. In `src-tauri/src/server/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_endpoint_responds() {
        let db = Arc::new(crate::db::Database::in_memory());
        let app_state = Arc::new(crate::app_state::AppState::new());
        let app_dir = std::env::temp_dir();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let addr = HttpServer::start(db, app_state, app_dir, 0, shutdown_rx) // port 0 = random
            .await
            .expect("server should start");

        let resp = reqwest::get(format!("http://{addr}/health"))
            .await
            .expect("request should succeed");
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.text().await.unwrap(), "ok");

        // Shutdown
        let _ = shutdown_tx.send(());
    }
}
```

2. Run: `cd src-tauri && cargo test server::tests::health_endpoint_responds` — should fail (no implementation).

3. Implement `HttpServer` in `server/mod.rs`:

```rust
use crate::app_state::AppState;
use crate::db::Database;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct ServerState {
    pub db: Arc<Database>,
    pub app_state: Arc<AppState>,
    pub app_dir: PathBuf,
}

pub struct HttpServer;

impl HttpServer {
    pub async fn start(
        db: Arc<Database>,
        app_state: Arc<AppState>,
        app_dir: PathBuf,
        port: u16,
        shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<SocketAddr, String> {
        let state = ServerState { db, app_state, app_dir };

        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .with_state(state);

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Failed to bind to {addr}: {e}"))?;
        let bound_addr = listener.local_addr().map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                    log::info!("HTTP server shutting down");
                })
                .await
                .ok();
        });

        log::info!("HTTP server listening on {bound_addr}");
        Ok(bound_addr)
    }
}
```

4. Wire shutdown into `lib.rs`. After `app.manage(app_state);` (line 131), add the server spawn. Replace the `.run()` event handler (line 233) to also send shutdown:

In the `.setup()` closure, add:
```rust
// Create shutdown channel for HTTP server
let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
app.manage(Arc::new(std::sync::Mutex::new(Some(shutdown_tx))));

// Start HTTP server (disabled by default — just scaffold)
// Will be activated via Settings UI in Task 13
let server_db = app.state::<Database>().inner().clone();
let server_app_state = app.state::<AppState>().inner().clone();
// TODO: Start server when enabled in settings (Task 13)
```

In the `.run()` event handler, add shutdown signal before lock release:
```rust
if let tauri::RunEvent::Exit = event {
    // Signal HTTP server to shut down
    if let Some(tx_holder) = app.try_state::<Arc<std::sync::Mutex<Option<oneshot::Sender<()>>>>>() {
        if let Some(tx) = tx_holder.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
    // ... existing lock release code
}
```

Note: Database and AppState are already `Arc`-wrapped by Tauri's `manage()` — `app.state::<T>().inner()` returns `&T` where T is behind an Arc internally. For the server, we need explicit `Arc`s. We'll resolve this in Task 13 when actually wiring the server startup. For now, just ensure the shutdown channel compiles.

5. Run: `cd src-tauri && cargo test server::tests::health_endpoint_responds` — should pass.

6. Add `reqwest` dev-dependency for async tests. In `Cargo.toml` under `[dev-dependencies]`:
```toml
reqwest = { version = "0.12", features = ["json"], default-features = false }
```

Note: `reqwest` is already in `[dependencies]` (line 37) with `blocking` + `rustls-tls` features. The dev dep uses the default async client for test convenience.

Wait — since reqwest is already a regular dependency, we can just use it in tests without a separate dev-dep. Use `reqwest::Client::new()` (async) in tests.

**Verify:**
```bash
cd src-tauri && cargo test server::tests -- --nocapture  # health test passes
cd src-tauri && cargo test 2>&1 | tail -3                # all tests still pass
```

**Commit:** `feat: add Axum server scaffold with health endpoint and graceful shutdown`

---

## Task 3: _internal Extraction — vehicles.rs + trips.rs

**Goal:** Extract 16 commands into `_internal` pure functions. Establish the pattern. Zero behavior change — existing tests are the verification.

**Files:**
- Modify: `src-tauri/src/commands/vehicles.rs`
- Modify: `src-tauri/src/commands/trips.rs`

**Pattern (vehicles.rs template):**

Before (`vehicles.rs:15-18`):
```rust
#[tauri::command]
pub fn get_vehicles(db: State<Database>) -> Result<Vec<Vehicle>, String> {
    db.get_all_vehicles().map_err(|e| e.to_string())
}
```

After:
```rust
pub fn get_vehicles_internal(db: &Database) -> Result<Vec<Vehicle>, String> {
    db.get_all_vehicles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_vehicles(db: State<Database>) -> Result<Vec<Vehicle>, String> {
    get_vehicles_internal(&db)
}
```

For write commands with `check_read_only!`:
```rust
pub fn create_vehicle_internal(
    db: &Database,
    app_state: &AppState,
    name: String,
    license_plate: String,
    // ... all other params unchanged
) -> Result<Vehicle, String> {
    check_read_only!(app_state);
    // ... entire existing body moves here
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_vehicle(
    db: State<Database>,
    app_state: State<AppState>,
    name: String,
    license_plate: String,
    // ... all other params unchanged
) -> Result<Vehicle, String> {
    create_vehicle_internal(&db, &app_state, name, license_plate, /* ... */)
}
```

**Steps:**

1. Extract all 6 vehicle commands to `_internal`. Move the FULL body into `_internal`; the Tauri wrapper becomes a one-line delegate.

   Commands: `get_vehicles`, `get_active_vehicle`, `create_vehicle`, `update_vehicle`, `delete_vehicle`, `set_active_vehicle`

2. Run: `cd src-tauri && cargo test` — all 195 tests must pass. This is the verification that the refactor is correct.

3. Extract all 10 trip commands to `_internal`:

   Commands: `get_trips`, `get_trips_for_year`, `get_years_with_trips`, `create_trip`, `update_trip`, `delete_trip`, `reorder_trip`, `get_routes`, `get_purposes`, `get_inferred_trip_time_for_route`

4. Run: `cd src-tauri && cargo test` — all tests pass.

**Verify:**
```bash
cd src-tauri && cargo test 2>&1 | tail -3  # all 195 tests pass
```

**Commit:** `refactor: extract _internal functions for vehicles and trips modules`

---

## Task 4: _internal Extraction — statistics.rs + settings_cmd.rs

**Goal:** Extract 20 commands. Introduces the **Pattern B** (AppHandle → app_dir) for settings commands.

**Files:**
- Modify: `src-tauri/src/commands/statistics.rs`
- Modify: `src-tauri/src/commands/settings_cmd.rs`

**Pattern B (AppHandle → app_dir):**

Before:
```rust
#[tauri::command]
pub fn get_theme_preference(app: AppHandle) -> Result<String, String> {
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);
    Ok(settings.theme.unwrap_or_default())
}
```

After:
```rust
pub fn get_theme_preference_internal(app_dir: &Path) -> Result<String, String> {
    let settings = LocalSettings::load(app_dir);
    Ok(settings.theme.unwrap_or_default())
}

#[tauri::command]
pub fn get_theme_preference(app: AppHandle) -> Result<String, String> {
    let app_dir = get_app_data_dir(&app)?;
    get_theme_preference_internal(&app_dir)
}
```

**Steps:**

1. **statistics.rs (4 commands):**
   - `calculate_trip_stats`, `calculate_magic_fill_liters`, `preview_trip_calculation` → Pattern A (DB only)
   - `get_trip_grid_data` → Special: takes `AppHandle` for HA push. The `_internal` takes `app_dir: Option<&Path>`. When `None`, skip the HA background push. When `Some`, load HA settings from app_dir and push.

2. Run: `cd src-tauri && cargo test` — pass.

3. **settings_cmd.rs (16 commands, skip 2 Tauri-only):**
   - Pattern A: `get_settings`, `save_settings`, `get_optimal_window_size`, `get_db_location`, `get_app_mode`, `check_target_has_db`
   - Pattern B: `get_theme_preference`, `set_theme_preference`, `get_auto_check_updates`, `set_auto_check_updates`, `get_date_prefill_mode`, `set_date_prefill_mode`, `get_hidden_columns`, `set_hidden_columns`
   - Skip: `move_database`, `reset_database_location` (Tauri-only — no _internal needed)

4. Run: `cd src-tauri && cargo test` — pass.

**Verify:**
```bash
cd src-tauri && cargo test 2>&1 | tail -3  # all tests pass
```

**Commit:** `refactor: extract _internal functions for statistics and settings modules`

---

## Task 5: _internal Extraction — receipts + backup + export + integrations

**Goal:** Extract remaining 31 commands across 4 modules. Some already have `_internal` (reuse them). Includes async commands.

**Files:**
- Modify: `src-tauri/src/commands/receipts_cmd.rs`
- Modify: `src-tauri/src/commands/backup.rs`
- Modify: `src-tauri/src/commands/export_cmd.rs`
- Modify: `src-tauri/src/commands/integrations.rs`

**Async command pattern:**

```rust
pub async fn sync_receipts_internal(
    db: &Database,
    app_state: &AppState,
    app_dir: &Path,
) -> Result<SyncResult, String> {
    // ... existing body
}

#[tauri::command]
pub async fn sync_receipts(
    app: AppHandle,
    db: State<'_, Database>,
    app_state: State<'_, AppState>,
) -> Result<SyncResult, String> {
    let app_dir = get_app_data_dir(&app)?;
    sync_receipts_internal(&db, &app_state, &app_dir).await
}
```

**Steps:**

1. **receipts_cmd.rs (17 commands, already has 3 _internal):**
   - Already done: `assign_receipt_to_trip_internal`, `get_trips_for_receipt_assignment_internal`, `verify_receipts_internal` — update wrappers to delegate
   - Pattern A: `get_receipts`, `get_receipts_for_vehicle`, `get_unassigned_receipts`, `update_receipt`, `delete_receipt`, `unassign_receipt`, `revert_receipt_override`
   - Pattern B: `get_receipt_settings`, `set_gemini_api_key`, `set_receipts_folder_path`, `scan_receipts`
   - Pattern B + async: `sync_receipts`, `process_pending_receipts`, `reprocess_receipt`

2. Run: `cd src-tauri && cargo test` — pass.

3. **backup.rs (11 commands, skip 1 Tauri-only, 1 already has _internal):**
   - Already done: `cleanup_pre_update_backups_internal` — update wrapper
   - Pattern B: `create_backup`, `create_backup_with_type`, `get_cleanup_preview`, `get_backup_retention`, `set_backup_retention`, `list_backups`, `get_backup_info`, `delete_backup`, `get_backup_path`
   - Skip: `restore_backup` (Tauri-only)

4. **export_cmd.rs (2 commands, skip 1 Tauri-only):**
   - Pattern B + async: `export_html`
   - Skip: `export_to_browser` (Tauri-only)

5. **integrations.rs (5 commands):**
   - Pattern B: `get_ha_settings`, `get_local_settings_for_ha`, `save_ha_settings`
   - Pattern B + async: `test_ha_connection`, `fetch_ha_odo`

6. Run: `cd src-tauri && cargo test` — all pass.

**Verify:**
```bash
cd src-tauri && cargo test 2>&1 | tail -3  # all tests pass
```

**Commit:** `refactor: extract _internal functions for receipts, backup, export, and integrations`

---

## Task 6: RPC Dispatcher — Sync Commands

**Goal:** Create the `POST /api/rpc` endpoint and wire all sync server-safe commands (~60). Test with dispatcher unit tests.

**Files:**
- Create: `src-tauri/src/server/dispatcher.rs`
- Modify: `src-tauri/src/server/mod.rs` (add route)

**Steps:**

1. Write failing tests first in `src-tauri/src/server/dispatcher.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_state() -> ServerState {
        ServerState {
            db: Arc::new(crate::db::Database::in_memory()),
            app_state: Arc::new(crate::app_state::AppState::new()),
            app_dir: std::env::temp_dir(),
        }
    }

    #[test]
    fn unknown_command_returns_error() {
        let state = test_state();
        let result = dispatch_sync("nonexistent", json!({}), &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown"));
    }

    #[test]
    fn get_vehicles_returns_empty_list() {
        let state = test_state();
        let result = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn create_vehicle_then_get() {
        let state = test_state();
        let args = json!({
            "name": "Test Car",
            "license_plate": "BA-123AB",
            "initial_odometer": 50000.0,
            "vehicle_type": "Ice",
            "tank_size_liters": 50.0,
            "tp_consumption": 6.5
        });
        let created = dispatch_sync("create_vehicle", args, &state).unwrap();
        assert_eq!(created["name"], "Test Car");

        let vehicles = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(vehicles.as_array().unwrap().len(), 1);
    }

    #[test]
    fn write_command_fails_in_read_only_mode() {
        let state = test_state();
        state.app_state.enable_read_only("Test read-only");

        let result = dispatch_sync("create_vehicle", json!({
            "name": "Test", "license_plate": "XX", "initial_odometer": 0.0,
            "vehicle_type": "Ice", "tank_size_liters": 50.0, "tp_consumption": 6.5
        }), &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("len na čítanie"));
    }

    #[test]
    fn bad_args_returns_error() {
        let state = test_state();
        let result = dispatch_sync("create_vehicle", json!({"wrong_field": true}), &state);
        assert!(result.is_err());
    }
}
```

2. Run: `cd src-tauri && cargo test server::dispatcher::tests` — should fail.

3. Implement `dispatch_sync` in `dispatcher.rs`. The function takes `(command, args, state)` and matches on command name:

```rust
use crate::app_state::AppState;
use crate::commands;
use crate::db::Database;
use crate::server::ServerState;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

fn parse_args<T: DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|e| format!("Invalid args: {e}"))
}

pub fn dispatch_sync(
    command: &str,
    args: Value,
    state: &ServerState,
) -> Result<Value, String> {
    let db = &state.db;
    let app_state = &state.app_state;
    let app_dir = &state.app_dir;

    match command {
        // ── vehicles ─────────────────────────────────────
        "get_vehicles" => {
            let v = commands::get_vehicles_internal(db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_active_vehicle" => {
            let v = commands::get_active_vehicle_internal(db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "create_vehicle" => {
            #[derive(serde::Deserialize)]
            struct Args {
                name: String,
                license_plate: String,
                initial_odometer: f64,
                vehicle_type: Option<String>,
                tank_size_liters: Option<f64>,
                tp_consumption: Option<f64>,
                battery_capacity_kwh: Option<f64>,
                baseline_consumption_kwh: Option<f64>,
                initial_battery_percent: Option<f64>,
                vin: Option<String>,
                driver_name: Option<String>,
            }
            let a: Args = parse_args(args)?;
            let v = commands::create_vehicle_internal(
                db, app_state, a.name, a.license_plate, a.initial_odometer,
                a.vehicle_type, a.tank_size_liters, a.tp_consumption,
                a.battery_capacity_kwh, a.baseline_consumption_kwh,
                a.initial_battery_percent, a.vin, a.driver_name,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        // ... one arm per sync server-safe command
        // Full list below — follow the same pattern for each command

        _ => Err(format!("Unknown or Tauri-only command: {command}")),
    }
}
```

4. Wire the RPC endpoint into the router in `server/mod.rs`:

```rust
mod dispatcher;

use axum::{extract::State as AxumState, http::StatusCode, response::IntoResponse, Json};

#[derive(serde::Deserialize)]
struct RpcRequest {
    command: String,
    args: serde_json::Value,
}

async fn rpc_handler(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    let command = req.command.clone();

    let result = tokio::task::spawn_blocking(move || {
        dispatcher::dispatch_sync(&command, req.args, &state_clone)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"));

    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(msg)) => (StatusCode::BAD_REQUEST, msg).into_response(),
        Err(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
    }
}
```

Add `/api/rpc` route to the router:
```rust
let app = Router::new()
    .route("/health", get(|| async { "ok" }))
    .route("/api/rpc", post(rpc_handler))
    .with_state(state);
```

5. Run dispatcher tests: `cd src-tauri && cargo test server::dispatcher::tests` — should pass.

6. Fill in remaining sync commands following the same arm-per-command pattern. Group by module for readability. The complete list of sync commands to wire:

**vehicles (6):** get_vehicles, get_active_vehicle, create_vehicle, update_vehicle, delete_vehicle, set_active_vehicle

**trips (10):** get_trips, get_trips_for_year, get_years_with_trips, create_trip, update_trip, delete_trip, reorder_trip, get_routes, get_purposes, get_inferred_trip_time_for_route

**statistics (4):** calculate_trip_stats, get_trip_grid_data, calculate_magic_fill_liters, preview_trip_calculation

**settings (14):** get_settings, save_settings, get_optimal_window_size, get_theme_preference, set_theme_preference, get_auto_check_updates, set_auto_check_updates, get_date_prefill_mode, set_date_prefill_mode, get_hidden_columns, set_hidden_columns, get_db_location, get_app_mode, check_target_has_db

**receipts (10):** get_receipts, get_receipts_for_vehicle, get_unassigned_receipts, update_receipt, delete_receipt, unassign_receipt, revert_receipt_override, assign_receipt_to_trip, get_trips_for_receipt_assignment, verify_receipts

**backup (10):** create_backup, create_backup_with_type, get_cleanup_preview, cleanup_pre_update_backups, get_backup_retention, set_backup_retention, list_backups, get_backup_info, delete_backup, get_backup_path

**integrations (3):** get_ha_settings, get_local_settings_for_ha, save_ha_settings

**export (0 sync):** export_html is async (Task 7)

**receipts scan (1):** scan_receipts (sync, needs app_dir)

7. Add an HTTP-level test to `server/mod.rs` tests:

```rust
#[tokio::test]
async fn rpc_endpoint_dispatches_command() {
    let db = Arc::new(crate::db::Database::in_memory());
    let app_state = Arc::new(crate::app_state::AppState::new());
    let app_dir = std::env::temp_dir();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let addr = HttpServer::start(db, app_state, app_dir, 0, shutdown_rx)
        .await.unwrap();

    let client = reqwest::Client::new();
    let resp = client.post(format!("http://{addr}/api/rpc"))
        .json(&serde_json::json!({
            "command": "get_vehicles",
            "args": {}
        }))
        .send().await.unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body, serde_json::json!([]));

    let _ = shutdown_tx.send(());
}
```

**Verify:**
```bash
cd src-tauri && cargo test server:: -- --nocapture  # all server tests pass
cd src-tauri && cargo test 2>&1 | tail -3           # all 195+ tests pass
```

**Commit:** `feat: add RPC dispatcher with all sync server-safe commands`

---

## Task 7: RPC Dispatcher — Async Commands

**Goal:** Add async command handling for the 6 async server-safe commands.

**Files:**
- Create: `src-tauri/src/server/dispatcher_async.rs`
- Modify: `src-tauri/src/server/mod.rs` (update rpc_handler)

**Steps:**

1. Create `dispatcher_async.rs` with `dispatch_async`:

```rust
use crate::server::ServerState;
use serde_json::Value;

pub async fn dispatch_async(
    command: &str,
    args: Value,
    state: &ServerState,
) -> Option<Result<Value, String>> {
    let db = &state.db;
    let app_state = &state.app_state;
    let app_dir = &state.app_dir;

    match command {
        "sync_receipts" => Some(
            commands::sync_receipts_internal(db, app_state, app_dir)
                .await
                .map(|v| serde_json::to_value(v).unwrap())
        ),
        "process_pending_receipts" => Some(
            commands::process_pending_receipts_internal(db, app_dir)
                .await
                .map(|v| serde_json::to_value(v).unwrap())
        ),
        "reprocess_receipt" => {
            #[derive(serde::Deserialize)]
            struct Args { id: String }
            let a: Args = match serde_json::from_value(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(format!("Invalid args: {e}"))),
            };
            Some(
                commands::reprocess_receipt_internal(db, app_state, &a.id)
                    .await
                    .map(|v| serde_json::to_value(v).unwrap())
            )
        }
        "test_ha_connection" => Some(
            commands::test_ha_connection_internal(app_dir)
                .await
                .map(|v| serde_json::to_value(v).unwrap())
        ),
        "fetch_ha_odo" => {
            #[derive(serde::Deserialize)]
            struct Args { sensor_id: String }
            let a: Args = match serde_json::from_value(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(format!("Invalid args: {e}"))),
            };
            Some(
                commands::fetch_ha_odo_internal(app_dir, &a.sensor_id)
                    .await
                    .map(|v| serde_json::to_value(v).unwrap())
            )
        }
        "export_html" => {
            // Parse args and call export_html_internal
            // ... similar pattern
            todo!("Wire export_html_internal")
        }
        _ => None, // Not an async command — fall through to sync dispatch
    }
}
```

2. Update `rpc_handler` in `server/mod.rs` to try async dispatch first:

```rust
async fn rpc_handler(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse {
    // Try async commands first
    if let Some(result) = dispatcher_async::dispatch_async(
        &req.command, req.args.clone(), &state
    ).await {
        return match result {
            Ok(value) => Json(value).into_response(),
            Err(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
        };
    }

    // Sync commands via spawn_blocking
    let state_clone = state.clone();
    let command = req.command.clone();
    let args = req.args;

    let result = tokio::task::spawn_blocking(move || {
        dispatcher::dispatch_sync(&command, args, &state_clone)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"));

    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(msg)) => (StatusCode::BAD_REQUEST, msg).into_response(),
        Err(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
    }
}
```

3. Add tests for async commands:

```rust
#[tokio::test]
async fn async_export_html_works() {
    // Create vehicle + trips, then call export_html via RPC
    // Verify response is HTML string
}
```

**Verify:**
```bash
cd src-tauri && cargo test server:: -- --nocapture  # all server tests pass
cd src-tauri && cargo test 2>&1 | tail -3           # all tests pass
```

**Commit:** `feat: add async command dispatch for receipts, HA, and export`

---

## Task 8: Capabilities + Receipt Image Endpoints

**Goal:** Add `GET /api/capabilities` and `GET /api/receipts/:id/image` endpoints.

**Files:**
- Modify: `src-tauri/src/server/mod.rs` (add routes + handlers)

**Steps:**

1. Write tests:

```rust
#[tokio::test]
async fn capabilities_endpoint() {
    let (addr, _) = start_test_server().await;

    let resp = reqwest::get(format!("http://{addr}/api/capabilities"))
        .await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["mode"], "server");
    assert_eq!(body["features"]["file_dialogs"], false);
    assert_eq!(body["features"]["updater"], false);
}
```

2. Implement capabilities handler:

```rust
async fn capabilities_handler(
    AxumState(state): AxumState<ServerState>,
) -> Json<serde_json::Value> {
    let read_only = state.app_state.is_read_only();
    Json(serde_json::json!({
        "mode": "server",
        "read_only": read_only,
        "features": {
            "file_dialogs": false,
            "updater": false,
            "open_external": false,
            "restore_backup": false,
            "move_database": false,
        }
    }))
}
```

3. Implement receipt image handler:

```rust
use axum::extract::Path as AxumPath;
use axum::http::header;

async fn receipt_image_handler(
    AxumState(state): AxumState<ServerState>,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    // Look up receipt by ID to get file path
    let db = &state.db;
    let receipt = match db.get_receipt(&id) {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Receipt not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let file_path = match receipt.file_path {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "No image file").into_response(),
    };

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => {
            let content_type = if file_path.ends_with(".png") {
                "image/png"
            } else if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
                "image/jpeg"
            } else {
                "application/octet-stream"
            };
            ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Image file not found on disk").into_response(),
    }
}
```

4. Add routes:
```rust
.route("/api/capabilities", get(capabilities_handler))
.route("/api/receipts/:id/image", get(receipt_image_handler))
```

**Verify:**
```bash
cd src-tauri && cargo test server:: -- --nocapture  # all server tests pass
cd src-tauri && cargo test 2>&1 | tail -3           # all tests pass
```

**Commit:** `feat: add capabilities and receipt image endpoints`

---

## Task 9: CORS + CSRF Security Layer

**Goal:** Add LAN-origin CORS allowlist and `X-KJ-Client` header requirement on POST.

**Files:**
- Modify: `src-tauri/src/server/mod.rs`

**Steps:**

1. Write tests:

```rust
#[tokio::test]
async fn cors_allows_lan_origin() {
    let (addr, _) = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("http://{addr}/api/rpc"))
        .header("Origin", "http://192.168.1.50:3456")
        .header("X-KJ-Client", "1")
        .header("Content-Type", "application/json")
        .body(r#"{"command":"get_vehicles","args":{}}"#)
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("access-control-allow-origin").is_some());
}

#[tokio::test]
async fn cors_rejects_public_origin() {
    let (addr, _) = start_test_server().await;
    let client = reqwest::Client::new();

    // Preflight request with public origin
    let resp = client.request(reqwest::Method::OPTIONS, format!("http://{addr}/api/rpc"))
        .header("Origin", "https://evil.com")
        .header("Access-Control-Request-Method", "POST")
        .header("Access-Control-Request-Headers", "content-type,x-kj-client")
        .send().await.unwrap();
    // Should NOT have allow-origin for evil.com
    let allow_origin = resp.headers().get("access-control-allow-origin");
    assert!(allow_origin.is_none() || allow_origin.unwrap() != "https://evil.com");
}
```

2. Implement CORS layer:

```rust
use axum::http::{header, HeaderName, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            let s = origin.to_str().unwrap_or("");
            is_lan_origin(s)
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("x-kj-client"),
        ])
}

fn is_lan_origin(origin: &str) -> bool {
    origin.starts_with("http://localhost")
        || origin.starts_with("http://127.")
        || origin.starts_with("http://10.")
        || origin.starts_with("http://192.168.")
        || is_rfc1918_172(origin)
}

fn is_rfc1918_172(origin: &str) -> bool {
    if let Some(rest) = origin.strip_prefix("http://172.") {
        if let Some(dot_pos) = rest.find('.') {
            if let Ok(second_octet) = rest[..dot_pos].parse::<u8>() {
                return (16..=31).contains(&second_octet);
            }
        }
    }
    false
}
```

3. Add CORS layer to router:
```rust
let app = Router::new()
    .route("/health", get(|| async { "ok" }))
    .route("/api/rpc", post(rpc_handler))
    .route("/api/capabilities", get(capabilities_handler))
    .route("/api/receipts/:id/image", get(receipt_image_handler))
    .layer(build_cors_layer())
    .with_state(state);
```

4. Add unit tests for `is_lan_origin`:

```rust
#[test]
fn lan_origin_detection() {
    assert!(is_lan_origin("http://localhost:3456"));
    assert!(is_lan_origin("http://127.0.0.1:3456"));
    assert!(is_lan_origin("http://192.168.1.50:3456"));
    assert!(is_lan_origin("http://10.0.0.1:3456"));
    assert!(is_lan_origin("http://172.16.0.1:3456"));
    assert!(is_lan_origin("http://172.31.255.255:3456"));

    assert!(!is_lan_origin("https://evil.com"));
    assert!(!is_lan_origin("http://172.15.0.1:3456")); // below range
    assert!(!is_lan_origin("http://172.32.0.1:3456")); // above range
    assert!(!is_lan_origin("http://example.com"));
}
```

**Verify:**
```bash
cd src-tauri && cargo test server:: -- --nocapture  # all server tests pass
cd src-tauri && cargo test 2>&1 | tail -3           # all tests pass
```

**Commit:** `feat: add CORS allowlist for LAN origins and X-KJ-Client header`

---

## Task 10: Frontend API Adapter

**Goal:** Create a dual-mode API adapter. In Tauri mode: `invoke()`. In browser mode: `fetch('/api/rpc')`. Migrate all frontend call sites. Desktop app must still work identically.

**Files:**
- Create: `src/lib/api-adapter.ts`
- Modify: `src/lib/api.ts` (swap `invoke` for `apiCall`)
- Modify: `src/lib/components/TripRow.svelte` (move direct invoke to api.ts)

**Steps:**

1. Create `src/lib/api-adapter.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function apiCall<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (IS_TAURI) {
    return invoke<T>(command, args);
  }

  const response = await fetch('/api/rpc', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-KJ-Client': '1',
    },
    body: JSON.stringify({ command, args: args ?? {} }),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text);
  }
  return response.json();
}

export { IS_TAURI };
```

2. In `src/lib/api.ts`, replace `import { invoke }` with `import { apiCall }` and swap all `invoke('command', args)` calls to `apiCall('command', args)`. This is a mechanical find-replace across the file (~56 calls).

Example change:
```typescript
// Before
import { invoke } from '@tauri-apps/api/core';
export async function getVehicles(): Promise<Vehicle[]> {
    return await invoke('get_vehicles');
}

// After
import { apiCall } from './api-adapter';
export async function getVehicles(): Promise<Vehicle[]> {
    return await apiCall('get_vehicles');
}
```

Keep the `revealItemInDir` import from `@tauri-apps/plugin-opener` — that's a Tauri-only function used in `revealBackup()`. Gate it with `IS_TAURI`:

```typescript
import { IS_TAURI } from './api-adapter';

export async function revealBackup(filename: string): Promise<void> {
    const path = await apiCall<string>('get_backup_path', { filename });
    if (IS_TAURI) {
        const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
        await revealItemInDir(path);
    }
}
```

3. Move TripRow.svelte's direct `invoke()` call (line 173) to api.ts as a wrapper function:

In `api.ts`, add:
```typescript
export async function getInferredTripTimeForRoute(
    vehicleId: string, origin: string, destination: string, rowDate: string
): Promise<InferredTripTime | null> {
    return await apiCall('get_inferred_trip_time_for_route', {
        vehicleId, origin, destination, rowDate,
    });
}
```

In `TripRow.svelte`, import from api.ts instead of direct invoke.

4. Verify the desktop app still works:

```bash
npm run build                  # build frontend
npm run tauri dev              # start app — click around, verify all pages work
```

5. Run integration tests to confirm no regressions:

```bash
npm run test:integration:tier1  # existing Tauri tests must pass
```

**Verify:**
- Desktop app works (manual smoke test: vehicles, trips, receipts, settings)
- `npm run test:integration:tier1` passes

**Commit:** `feat: add dual-mode API adapter, migrate all invoke() calls`

---

## Task 11: Capabilities Store + Feature Gating

**Goal:** Frontend reads `/api/capabilities` and hides Tauri-only UI elements in browser mode.

**Files:**
- Create: `src/lib/stores/capabilities.ts`
- Modify: `src/routes/settings/+page.svelte` (hide move-database, restore-backup)
- Modify: `src/routes/doklady/+page.svelte` (gate openPath)
- Modify: `src/routes/+layout.svelte` (gate window resize)
- Modify: `src/lib/stores/update.ts` (gate updater)

**Steps:**

1. Create capabilities store:

```typescript
// src/lib/stores/capabilities.ts
import { writable } from 'svelte/store';
import { IS_TAURI } from '$lib/api-adapter';

interface Capabilities {
    mode: 'desktop' | 'server';
    readOnly: boolean;
    features: {
        fileDialogs: boolean;
        updater: boolean;
        openExternal: boolean;
        restoreBackup: boolean;
        moveDatabase: boolean;
    };
}

const defaultDesktop: Capabilities = {
    mode: 'desktop',
    readOnly: false,
    features: {
        fileDialogs: true,
        updater: true,
        openExternal: true,
        restoreBackup: true,
        moveDatabase: true,
    },
};

export const capabilities = writable<Capabilities>(defaultDesktop);

export async function loadCapabilities(): Promise<void> {
    if (IS_TAURI) {
        // Desktop mode — all features available, read-only from app_state
        const { apiCall } = await import('$lib/api-adapter');
        const mode = await apiCall<{ mode: string }>('get_app_mode');
        capabilities.set({
            ...defaultDesktop,
            readOnly: mode.mode === 'ReadOnly',
        });
        return;
    }

    // Browser mode — fetch capabilities from server
    try {
        const resp = await fetch('/api/capabilities');
        const data = await resp.json();
        capabilities.set({
            mode: 'server',
            readOnly: data.read_only,
            features: {
                fileDialogs: data.features.file_dialogs,
                updater: data.features.updater,
                openExternal: data.features.open_external,
                restoreBackup: data.features.restore_backup,
                moveDatabase: data.features.move_database,
            },
        });
    } catch {
        // Fallback: assume desktop
        capabilities.set(defaultDesktop);
    }
}
```

2. Call `loadCapabilities()` in `+layout.svelte` `onMount`.

3. Gate Tauri-only UI elements:
   - Settings page: wrap "Move database" / "Restore backup" buttons in `{#if $capabilities.features.moveDatabase}`
   - Doklady page: wrap `openPath()` calls in `IS_TAURI` check, show download link in browser mode
   - Layout: wrap window resize logic in `IS_TAURI` check
   - Update store: skip update check when `!$capabilities.features.updater`

4. Add i18n strings for browser-mode alternatives (e.g., "Stiahni zálohu" instead of "Otvoriť priečinok").

**Verify:**
```bash
npm run tauri dev  # desktop still works, all features visible
# Open http://localhost:3456 in Chrome — Tauri-only buttons should be hidden
```

**Commit:** `feat: add capabilities store and feature-gate Tauri-only UI`

---

## Task 12: Static File Serving + SPA Fallback

**Goal:** Axum serves the built frontend from `build/` directory with SPA fallback so deep links like `/vozidla/abc` work on refresh.

**Files:**
- Modify: `src-tauri/src/server/mod.rs`

**Steps:**

1. Write test:

```rust
#[tokio::test]
async fn spa_fallback_serves_index_html() {
    // Create a temp dir with a fake index.html
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("index.html"), "<html>app</html>").unwrap();

    let (addr, _) = start_test_server_with_static_dir(temp.path()).await;

    // Deep link should return index.html (SPA fallback)
    let resp = reqwest::get(format!("http://{addr}/vozidla/some-id"))
        .await.unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.text().await.unwrap().contains("<html>app</html>"));
}
```

2. Add static file serving with SPA fallback:

```rust
use tower_http::services::{ServeDir, ServeFile};

// In HttpServer::start, resolve static_dir:
let static_dir = if cfg!(debug_assertions) {
    // Dev: project root's build/ directory
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build")
} else {
    // Production: Tauri resource directory
    // Passed in via ServerState
    state.static_dir.clone()
};

// Build router with API routes + static fallback
let api_router = Router::new()
    .route("/rpc", post(rpc_handler))
    .route("/capabilities", get(capabilities_handler))
    .route("/receipts/:id/image", get(receipt_image_handler));

let app = if static_dir.exists() {
    let static_service = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(static_dir.join("index.html")));

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .nest("/api", api_router)
        .fallback_service(static_service)
        .layer(build_cors_layer())
        .with_state(state)
} else {
    log::warn!("Static frontend directory not found at {static_dir:?}. Run 'npm run build' first.");
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .nest("/api", api_router)
        .layer(build_cors_layer())
        .with_state(state)
};
```

3. Add `static_dir: PathBuf` to `ServerState`. In production, pass `app.path().resource_dir()`. In dev, pass `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build")`.

**Verify:**
```bash
npm run build                            # build frontend
cd src-tauri && cargo test server::      # static serving test passes
npm run tauri dev                        # start app
# Open http://localhost:3456 in Chrome — should show the full app
# Navigate to /vozidla, refresh — SPA fallback works
```

**Commit:** `feat: add static file serving with SPA fallback`

---

## Task 13: Settings UI — Server Mode Controls

**Goal:** Add a "Server Mode" card to the Settings page with toggle, port input, status indicator, and URL display.

**Files:**
- Modify: `src/routes/settings/+page.svelte`
- Modify: `src/lib/i18n/sk/index.ts` (Slovak strings)
- Modify: `src/lib/i18n/en/index.ts` (English strings)
- Modify: `src-tauri/src/settings.rs` (add server fields to LocalSettings)
- Create: `src-tauri/src/commands/server_cmd.rs` (server control commands)
- Modify: `src-tauri/src/commands/mod.rs` (add module)
- Modify: `src-tauri/src/lib.rs` (register commands)

**Steps:**

1. Add server fields to `LocalSettings` in `settings.rs`:

```rust
pub struct LocalSettings {
    // ... existing fields
    pub server_enabled: Option<bool>,
    pub server_port: Option<u16>,
}
```

2. Add Tauri commands for server control:

```rust
// commands/server_cmd.rs
#[tauri::command]
pub fn get_server_status() -> Result<ServerStatus, String> { ... }

#[tauri::command]
pub fn start_server(app: AppHandle, port: u16) -> Result<ServerStatus, String> { ... }

#[tauri::command]
pub fn stop_server() -> Result<(), String> { ... }

#[derive(Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub url: Option<String>,  // e.g., "http://192.168.1.5:3456"
}
```

Use `local_ip_address::local_ip()` to get the LAN IP for the URL display.

3. Add i18n strings:

```typescript
// Slovak
serverMode: 'Režim servera',
serverModeDescription: 'Sprístupnite aplikáciu z telefónu alebo tabletu cez prehliadač.',
serverEnabled: 'Server aktívny',
serverPort: 'Port',
serverUrl: 'Adresa pre prehliadač',
serverPortConflict: 'Port {port} je obsadený. Zvoľte iný port.',
serverStopped: 'Server je vypnutý',

// English
serverMode: 'Server Mode',
serverModeDescription: 'Access the app from your phone or tablet via browser.',
serverEnabled: 'Server active',
serverPort: 'Port',
serverUrl: 'Browser address',
serverPortConflict: 'Port {port} is in use. Choose a different port.',
serverStopped: 'Server is off',
```

4. Add Settings card UI (in settings page, after existing cards):

```svelte
<div class="settings-card">
    <h3>{LL.serverMode()}</h3>
    <p class="description">{LL.serverModeDescription()}</p>

    <div class="setting-row">
        <label>{LL.serverPort()}</label>
        <input type="number" bind:value={serverPort} min="1024" max="65535"
               disabled={serverRunning} />
    </div>

    <div class="setting-row">
        <label>{LL.serverEnabled()}</label>
        <button on:click={toggleServer}>
            {serverRunning ? LL.stop() : LL.start()}
        </button>
    </div>

    {#if serverRunning && serverUrl}
        <div class="server-url">
            <label>{LL.serverUrl()}</label>
            <code>{serverUrl}</code>
        </div>
    {/if}

    {#if serverError}
        <div class="error">{serverError}</div>
    {/if}
</div>
```

5. Wire toggle to start/stop commands. Persist port + enabled state to `local.settings.json`.

**Verify:**
```bash
npm run tauri dev
# Open Settings → Server Mode card visible
# Set port 3456, click Start → URL shown
# Open URL in Chrome → app loads
# Click Stop → server stops
# Change port, restart → works on new port
npm run test:integration:tier1  # existing tests still pass
```

**Commit:** `feat: add server mode settings UI with start/stop controls`

---

## Task 14: LAN Binding + Auto-Start

**Goal:** Switch from 127.0.0.1 to 0.0.0.0 when server is enabled (so LAN devices can connect). Auto-start on app launch if `server_enabled` was true.

**Files:**
- Modify: `src-tauri/src/server/mod.rs` (parameterize bind address)
- Modify: `src-tauri/src/lib.rs` (auto-start on launch)
- Modify: `src-tauri/src/commands/server_cmd.rs` (bind to 0.0.0.0)

**Steps:**

1. Change `HttpServer::start` to accept a bind address parameter:

```rust
pub async fn start(
    db: Arc<Database>,
    app_state: Arc<AppState>,
    app_dir: PathBuf,
    static_dir: PathBuf,
    port: u16,
    bind_all: bool,  // true = 0.0.0.0, false = 127.0.0.1
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<SocketAddr, String> {
    let addr = if bind_all {
        SocketAddr::from(([0, 0, 0, 0], port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], port))
    };
    // ...
}
```

2. Add auto-start logic in `lib.rs` setup:

```rust
// After managing db and app_state, check for server auto-start
let auto_start = std::env::var("KNIHA_JAZD_SERVER_AUTOSTART").is_ok();
let settings = LocalSettings::load(&app_dir);
if auto_start || settings.server_enabled.unwrap_or(false) {
    let port = settings.server_port.unwrap_or(3456);
    // Start server in background...
}
```

3. Port conflict handling: if bind fails, log the error and surface it in the Settings UI on next load. Don't block app startup.

**Verify:**
```bash
npm run tauri dev
# Enable server in Settings
# On phone/tablet on same Wi-Fi, open the URL → app loads
# Close and reopen app → server auto-starts
```

**Commit:** `feat: LAN binding and server auto-start on launch`

---

## Task 15: Dual-Mode Integration Tests

**Goal:** Run the SAME integration test suite against a Chrome browser connected to the HTTP server. Both Tauri and Chrome suites pass.

**Files:**
- Create: `tests/integration/wdio.server.conf.ts`
- Modify: `tests/integration/utils/db.ts` (dual-mode `invokeBackend`)
- Modify: `tests/integration/utils/app.ts` (dual-mode `waitForAppReady`)
- Modify: `package.json` (add npm scripts)

**Steps:**

1. Create dual-mode invoke helper in `utils/db.ts`:

```typescript
/**
 * Detect if running in Tauri or Server (Chrome) mode.
 * Set by wdio config via capabilities or env var.
 */
const IS_SERVER_MODE = process.env.WDIO_SERVER_MODE === '1';

/**
 * Execute a backend command — Tauri IPC in webview mode, HTTP RPC in server mode.
 */
async function invokeBackend<T>(
    cmd: string,
    args: Record<string, unknown> = {}
): Promise<T> {
    if (IS_SERVER_MODE) {
        // HTTP RPC
        const baseUrl = process.env.WDIO_SERVER_URL || 'http://localhost:3456';
        const resp = await fetch(`${baseUrl}/api/rpc`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-KJ-Client': '1',
            },
            body: JSON.stringify({ command: cmd, args }),
        });
        if (!resp.ok) {
            const text = await resp.text();
            throw new Error(`RPC '${cmd}' failed: ${text}`);
        }
        return await resp.json() as T;
    }

    // Existing Tauri IPC path
    return invokeTauri<T>(cmd, args);
}
```

Replace all `invokeTauri()` calls with `invokeBackend()` throughout `db.ts`. The existing Tauri path is unchanged.

2. Update `utils/app.ts` `waitForAppReady()`:

```typescript
export async function waitForAppReady(): Promise<void> {
    const isServerMode = process.env.WDIO_SERVER_MODE === '1';

    // Wait for DOM
    const header = await $('h1');
    await header.waitForDisplayed({ timeout: 15000 });

    if (!isServerMode) {
        // Tauri mode: wait for IPC bridge
        await browser.waitUntil(async () => {
            return browser.execute(() =>
                typeof window.__TAURI__ !== 'undefined' &&
                typeof window.__TAURI__.core?.invoke === 'function'
            );
        }, { timeout: 10000 });
    }
    // Server mode: DOM ready is sufficient (API calls go over HTTP, not IPC)
}
```

3. Create `tests/integration/wdio.server.conf.ts`:

```typescript
import { config as baseConfig } from './wdio.conf';
import { spawn, ChildProcess } from 'child_process';
import { join } from 'path';

let tauriProcess: ChildProcess;
const SERVER_PORT = 3456;
const SERVER_URL = `http://localhost:${SERVER_PORT}`;

export const config: WebdriverIO.Config = {
    ...baseConfig,

    // Override: use Chrome instead of tauri-driver
    hostname: 'localhost',
    port: 9515,  // chromedriver default
    path: '/',

    capabilities: [{
        browserName: 'chrome',
        'goog:chromeOptions': {
            args: ['--no-sandbox', '--disable-gpu'],
        },
    }],

    baseUrl: SERVER_URL,

    // Override onPrepare: start Tauri binary with server mode, not tauri-driver
    onPrepare: async function () {
        // Create temp data dir (same as base config)
        const testDataDir = join(os.tmpdir(), `kniha-jazd-server-test-${Date.now()}`);
        fs.mkdirSync(testDataDir, { recursive: true });
        process.env.KNIHA_JAZD_DATA_DIR = testDataDir;
        process.env.WDIO_SERVER_MODE = '1';
        process.env.WDIO_SERVER_URL = SERVER_URL;

        // Start Tauri binary with server auto-start
        const binaryPath = getBinaryPath();
        tauriProcess = spawn(binaryPath, [], {
            env: {
                ...process.env,
                KNIHA_JAZD_DATA_DIR: testDataDir,
                KNIHA_JAZD_SERVER_AUTOSTART: '1',
            },
        });

        // Wait for HTTP server to be ready
        await waitForUrl(`${SERVER_URL}/health`, 30000);
    },

    // Override before: navigate to server URL
    before: async function () {
        await browser.url(SERVER_URL);
        await waitForAppReady();
    },

    // Override beforeTest: clean DB via RPC, not file deletion
    beforeTest: async function () {
        // Reset database by deleting the file and refreshing
        // (same as base config, but refresh loads from server URL)
        // ... delete DB file from testDataDir
        await browser.url(SERVER_URL);
        await waitForAppReady();
    },

    // Override onComplete: kill Tauri process
    onComplete: async function () {
        if (tauriProcess) tauriProcess.kill();
        // Cleanup temp dir
    },
};
```

4. Handle Tauri-only test skipping. Tests that use `export_to_browser`, `move_database`, `restore_backup`, or `reset_database_location` should be skipped in server mode. Add a helper:

```typescript
// utils/skip.ts
export function skipInServerMode(testName: string): void {
    if (process.env.WDIO_SERVER_MODE === '1') {
        pending(`Skipped in server mode: ${testName}`);
    }
}
```

Use in tests:
```typescript
it('should restore backup', async () => {
    skipInServerMode('restore backup uses file dialog');
    // ... existing test
});
```

5. Add npm scripts to `package.json`:

```json
"test:integration:server": "set WDIO_SERVER_MODE=1 && wdio run tests/integration/wdio.server.conf.ts",
"test:integration:server:tier1": "set TIER=1 && npm run test:integration:server",
"test:all": "npm run test:backend && npm run test:integration && npm run test:integration:server"
```

6. Install chromedriver for WebdriverIO:
```bash
npm install --save-dev @wdio/chromedriver-service chromedriver
```

**Verify:**
```bash
npm run build                              # build frontend for static serving
npm run test:integration:tier1             # Tauri tests still pass
npm run test:integration:server:tier1      # Chrome/server tests pass
```

**Commit:** `feat: add dual-mode integration test infrastructure (Tauri + Chrome)`

---

## Task 16: CI Pipeline + Documentation

**Goal:** Both test suites run in CI. Feature documented.

**Files:**
- Modify: `.github/workflows/test.yml`
- Create: `docs/features/server-mode.md`
- Modify: `CHANGELOG.md`
- Modify: `DECISIONS.md`

**Steps:**

1. Add server test job to `.github/workflows/test.yml`:

```yaml
integration-test-server:
    name: Integration Tests (Server/Chrome) - Tier 1
    needs: build
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: '20' }
      # ... setup Chrome, chromedriver
      - name: Download debug build
        uses: actions/download-artifact@v4
        with: { name: tauri-debug-build }
      - name: Build frontend
        run: npm run build
      - name: Run server integration tests
        env:
          TIER: '1'
          WDIO_SERVER_MODE: '1'
          KNIHA_JAZD_SERVER_AUTOSTART: '1'
        run: npm run test:integration:server
```

2. Create `docs/features/server-mode.md` with user flow + technical implementation + design rationale (following `docs/CLAUDE.md` template).

3. Update `CHANGELOG.md` under `[Unreleased]`:
```markdown
### Added
- Server Mode: access the app from any browser on your local network (phone, tablet, other PC)
- Settings toggle to enable/disable the embedded HTTP server
- Automatic CORS protection for LAN-only access
```

4. Add decisions to `DECISIONS.md`:
- ADR: RPC over REST for server mode API
- ADR: `_internal` extraction pattern for command reuse
- ADR: LAN-only CORS allowlist without authentication

**Verify:**
```bash
npm run test:backend                        # all Rust tests pass
npm run test:integration:tier1              # Tauri tests pass
npm run test:integration:server:tier1       # Chrome tests pass
git status                                  # review all changes
```

**Commit:** `docs: add server mode feature doc, CI pipeline, changelog`

---

## Post-Implementation Verification

After all 16 tasks are complete, run the full verification:

```bash
# 1. All backend tests
cd src-tauri && cargo test

# 2. All integration tests (Tauri)
npm run test:integration

# 3. All integration tests (Chrome/Server)
npm run test:integration:server

# 4. Manual smoke test
npm run tauri dev
# → Enable server in Settings
# → Open phone browser to the URL
# → Create a vehicle, add trips, view receipts
# → Verify desktop and phone show same data

# 5. Verify Tauri-only features hidden in browser
# → No updater banner
# → No "Move database" button
# → No "Restore backup" button
```

---

## Execution Notes

- **Tasks 3-5** (_internal extraction) are the most tedious — mechanically repetitive but each must be verified with `cargo test`. Consider parallelizing across modules using subagents.
- **Task 10** (frontend adapter) is the largest single frontend change — 56 call sites in api.ts. It's mechanical but must be verified carefully.
- **Task 15** (dual-mode tests) depends on Tasks 1-14 being complete. It's the final proof that everything works end-to-end.
- **LAN exposure (Task 14) is deliberately last** — the server is never reachable from the network until all security and functionality is proven.
