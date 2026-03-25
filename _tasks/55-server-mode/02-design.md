**Date:** 2026-03-07
**Subject:** Server Mode - Technical Design
**Status:** Draft

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
│  │ (71 handlers)   │    │ (REST routes)        │  │
│  └───────┬────────┘    └──────────┬───────────┘  │
│          │                        │              │
│          └──────────┬─────────────┘              │
│                     ▼                            │
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

### 1. Shared Database Instance (Not a second connection)

The HTTP server uses the **same** `Arc<Database>` that Tauri commands use. This ensures:
- No SQLite locking conflicts
- Consistent data between desktop and browser views
- No connection pool needed (single Mutex serializes everything)

```rust
// In lib.rs setup:
let db = Arc::new(Database::new(path)?);

// Tauri gets a clone
app.manage(db.clone());

// Axum gets the same clone
let server = HttpServer::new(db.clone(), app_state.clone());
```

### 2. Axum Route Registration via Macro

To avoid maintaining 71 route handlers manually, use a macro that wraps existing command functions:

```rust
/// Generate an Axum handler from a Tauri command function
macro_rules! api_handler {
    // GET handler - query params
    (get $path:expr, $func:path, $params:ty) => {
        axum::routing::get(|
            State(state): State<AppState>,
            Query(params): Query<$params>,
        | async move {
            let db = state.db.clone();
            let result = tokio::task::spawn_blocking(move || {
                $func(&db, params)
            }).await.unwrap();
            match result {
                Ok(data) => Json(data).into_response(),
                Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
            }
        })
    };
}
```

### 3. Frontend API Adapter Pattern

Create a thin abstraction layer that the rest of the frontend uses:

```typescript
// src/lib/api-adapter.ts

import { invoke } from '@tauri-apps/api/core';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Base URL for HTTP mode (injected by server or detected)
function getBaseUrl(): string {
    return window.location.origin;
}

export async function apiCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    if (IS_TAURI) {
        return invoke<T>(command, args);
    }

    // Map command name to HTTP endpoint
    const endpoint = COMMAND_MAP[command];
    const response = await fetch(`${getBaseUrl()}${endpoint.path}`, {
        method: endpoint.method,
        headers: { 'Content-Type': 'application/json' },
        body: args ? JSON.stringify(args) : undefined,
    });

    if (!response.ok) {
        throw new Error(await response.text());
    }

    return response.json();
}
```

### 4. Command-to-Route Mapping

```typescript
const COMMAND_MAP: Record<string, { method: string; path: string }> = {
    // Vehicles
    'get_vehicles':        { method: 'GET',    path: '/api/vehicles' },
    'create_vehicle':      { method: 'POST',   path: '/api/vehicles' },
    'update_vehicle':      { method: 'PUT',    path: '/api/vehicles' },
    'delete_vehicle':      { method: 'DELETE', path: '/api/vehicles' },
    'get_active_vehicle':  { method: 'GET',    path: '/api/vehicles/active' },
    'set_active_vehicle':  { method: 'POST',   path: '/api/vehicles/active' },

    // Trips
    'get_trips_for_year':  { method: 'POST',   path: '/api/trips/year' },
    'create_trip':         { method: 'POST',   path: '/api/trips' },
    'update_trip':         { method: 'PUT',    path: '/api/trips' },
    'delete_trip':         { method: 'DELETE', path: '/api/trips' },
    'reorder_trip':        { method: 'POST',   path: '/api/trips/reorder' },

    // Calculations
    'get_trip_grid_data':         { method: 'POST', path: '/api/grid-data' },
    'calculate_trip_stats':       { method: 'POST', path: '/api/stats' },
    'preview_trip_calculation':   { method: 'POST', path: '/api/preview' },
    'get_compensation_suggestion':{ method: 'POST', path: '/api/compensation' },

    // ... etc for all 71 commands
};
```

**Alternative (simpler):** Use a single POST endpoint for all commands:

```typescript
// Simple RPC-style: POST /api/rpc { command: "get_vehicles", args: {} }
export async function apiCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    if (IS_TAURI) {
        return invoke<T>(command, args);
    }

    const response = await fetch(`${getBaseUrl()}/api/rpc`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ command, args: args || {} }),
    });

    if (!response.ok) throw new Error(await response.text());
    return response.json();
}
```

This RPC approach is **much simpler** — one Axum handler routes to all existing functions.

### 5. RPC-Style Backend Handler (Recommended)

Instead of 71 individual routes, use a single RPC dispatcher:

```rust
#[derive(Deserialize)]
struct RpcRequest {
    command: String,
    args: serde_json::Value,
}

async fn rpc_handler(
    State(state): State<SharedState>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse {
    let db = state.db.clone();
    let app_state = state.app_state.clone();

    let result = tokio::task::spawn_blocking(move || {
        dispatch_command(&req.command, req.args, &db, &app_state)
    }).await.unwrap();

    match result {
        Ok(value) => Json(value).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

fn dispatch_command(
    command: &str,
    args: serde_json::Value,
    db: &Database,
    app_state: &AppState,
) -> Result<serde_json::Value, String> {
    match command {
        "get_vehicles" => {
            let vehicles = db.get_vehicles()?;
            Ok(serde_json::to_value(vehicles).unwrap())
        }
        "create_trip" => {
            check_read_only_web(app_state)?;
            let params: CreateTripParams = serde_json::from_value(args)
                .map_err(|e| e.to_string())?;
            let trip = db.create_trip(/* ... */)?;
            Ok(serde_json::to_value(trip).unwrap())
        }
        // ... all 71 commands
        _ => Err(format!("Unknown command: {}", command)),
    }
}
```

**Advantages of RPC approach:**
- Minimal frontend changes (just swap `invoke()` for `fetch /api/rpc`)
- One route to maintain
- Same argument structure as Tauri IPC (JSON object with named params)
- Easy to add new commands without route changes

## Implementation Plan (High-Level)

### Phase 1: Backend HTTP Server (Rust)
1. Add axum + tower-http dependencies
2. Create `src-tauri/src/server/` module
3. Implement RPC dispatcher with all commands
4. Add start/stop server Tauri commands
5. Wire server to shared Database + AppState

### Phase 2: Frontend API Adapter
1. Create `api-adapter.ts` with Tauri/HTTP detection
2. Refactor `api.ts` to use adapter
3. Handle receipt image URLs (Tauri path vs HTTP URL)
4. Handle Tauri-specific APIs (window management, file dialogs)

### Phase 3: Settings UI
1. Add "Server mode" section to Settings page
2. Toggle button with status indicator
3. Display URL with local network IP
4. Port configuration (optional)
5. Persist server enabled state in settings

### Phase 4: Static File Serving
1. Serve SvelteKit build output from Axum
2. Serve receipt images via HTTP
3. Handle SPA routing (fallback to index.html)

## Open Questions

1. **Port selection:** Fixed (3456) or configurable?
   - Recommendation: Default 3456, configurable in settings

2. **Auto-start:** Should server start automatically on app launch if previously enabled?
   - Recommendation: Yes, persist preference

3. **Receipts in browser:** Serve images via HTTP endpoint or embed base64?
   - Recommendation: HTTP endpoint (`/api/receipts/images/:id`) - simpler, cacheable

4. **Tauri-only features in browser:**
   - Window management → Hide in browser
   - File dialogs (backup restore) → Use `<input type="file">` or hide
   - Auto-updater → Hide in browser
   - Database location → Hide in browser (uses host's path)
   - Recommendation: Conditionally render based on `IS_TAURI`

5. **Relationship with Task 33:**
   - Task 33's Axum web binary can **reuse** the `server/` module from this task
   - The RPC dispatcher, route handlers, and static serving code are identical
   - Task 33 just wraps it in a standalone binary instead of embedding in Tauri
