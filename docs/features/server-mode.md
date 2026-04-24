# Feature: Server Mode

> Access the app from any browser on your local network (phone, tablet, other PC) via an embedded HTTP server.

## User Flow

1. **Navigate** to Settings
2. **Enable** the "Server" toggle
3. **See** the server URL displayed (e.g., `http://192.168.1.42:3456`)
4. **Open** the URL on any device connected to the same LAN
5. **Use** the app normally — trips, vehicles, receipts, export all work
6. **Optionally** enable "Auto-start" so the server starts automatically on next app launch

**What's different in the browser:**
- Desktop-only features are hidden (Move database, Restore backup, Export to browser, auto-updater)
- File dialogs (folder picker) are not available
- All data changes are reflected on both desktop and browser in real time (shared database)

**Disabling:** Toggle the server off in Settings. All browser sessions lose access immediately.

## Technical Implementation

### Frontend

**Runtime Detection:** `src/lib/api.ts`
- Detects whether running inside Tauri webview or a regular browser
- Tauri mode: uses `invoke()` IPC as before
- Browser mode: sends `POST /api/rpc` with `{ command, args }` JSON to the server

**Capabilities Store:** `src/lib/stores/capabilities.ts`
- Fetches `GET /api/capabilities` on load (browser mode only)
- Returns which commands are available (68 of 72 are server-safe)
- Components check capabilities to hide unavailable features

**Settings UI:** `src/routes/settings/+page.svelte`
- Server toggle, auto-start checkbox, URL display
- Shows local IP address for easy sharing

### Backend (Rust)

**Server Module:** `src-tauri/src/server/mod.rs`
- Axum HTTP server with `POST /api/rpc` endpoint
- `GET /health` for readiness checks
- Static file serving for SPA (built frontend files)
- CORS layer restricting origins to RFC 1918 private ranges + localhost

**RPC Dispatcher:** `src-tauri/src/server/dispatcher.rs` + `dispatcher_async.rs`
- Maps command names to `_internal` functions
- Sync commands dispatched via `spawn_blocking`
- Async commands (receipts OCR, HA integration, export) awaited directly

**Server Manager:** `src-tauri/src/server/manager.rs`
- Start/stop lifecycle management
- LAN IP detection via `local-ip-address` crate
- Port binding (default 3456)
- Graceful shutdown via tokio oneshot channel

**_internal Functions:** `src-tauri/src/commands/*.rs`
- Each Tauri command split into thin wrapper + pure `_internal` function
- `_internal` takes `&Database` / `&AppState` / `&Path` as plain references
- Both Tauri IPC and RPC dispatcher call the same `_internal` functions

### Data Flow

```
Browser (phone)                         Desktop (Tauri webview)
     |                                         |
     | fetch('/api/rpc')                       | invoke('command', args)
     |                                         |
     v                                         v
 Axum HTTP Server                     Tauri IPC Bridge
     |                                         |
     | dispatch(command, args)                  | extract State<Database>
     |                                         |
     +------------------+----------------------+
                        |
                        v
              _internal(db, app_state, args)
                        |
                        v
                 SQLite Database
            (separate connection per path)
```

### Capabilities Endpoint

`GET /api/capabilities` returns:

```json
{
  "mode": "server",
  "read_only": false,
  "features": {
    "file_dialogs": false,
    "updater": false,
    "open_external": false,
    "restore_backup": false,
    "move_database": false
  }
}
```

Frontend uses these feature flags to hide UI elements that aren't available in browser mode.

### CORS

The CORS layer allows origins matching RFC 1918 private IP ranges:
- `http://10.*.*.*:*`
- `http://172.16-31.*.*:*`
- `http://192.168.*.*:*`
- `http://localhost:*` / `http://127.0.0.1:*`

Requests from public IPs or other origins are blocked by the browser's preflight check.

### Receipt Image Serving

`GET /api/receipts/{id}/image` looks up the receipt by ID in the database, then serves the image file from disk. This enables browser-mode users to view scanned receipts.

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/server/mod.rs` | Axum router, RPC handler, CORS, static files |
| `src-tauri/src/server/dispatcher.rs` | Sync command dispatch (60+ commands) |
| `src-tauri/src/server/dispatcher_async.rs` | Async command dispatch (7 commands) |
| `src-tauri/src/server/manager.rs` | Server lifecycle, LAN IP, port binding |
| `src-tauri/src/commands/*.rs` | `_internal` functions shared by both paths |
| `src/lib/api.ts` | Runtime detection + RPC adapter |
| `src/lib/stores/capabilities.ts` | Feature gating for browser mode |
| `tests/integration/wdio.server.conf.ts` | Server-mode integration test config |

## Design Decisions

- **Why RPC over REST?** -- Single `POST /api/rpc` endpoint mirrors Tauri IPC model exactly. No need to design 68 separate REST routes for an internal-only API. (See ADR-015)

- **Why `_internal` extraction?** -- Tauri commands take framework-injected `State<Database>`. The RPC dispatcher has `Arc<Database>` directly. Pure `_internal` functions bridge both without abstraction overhead. (See ADR-016)

- **Why no authentication?** -- Server is LAN-only (CORS-enforced). Target environment is trusted home/office network. Authentication would add significant complexity for minimal security benefit. (See ADR-017)

- **Why single process, not a separate server?** -- The server opens a second SQLite connection to the same file. SQLite's built-in file-level locking handles concurrency. No IPC between processes, no stale caches.

- **Why 4 commands excluded?** -- `export_to_browser` opens a desktop browser, `move_database` and `reset_database_location` use native file dialogs, `restore_backup` replaces the running database. None are safe over HTTP.

## Related

- ADR-015: RPC Over REST for Server Mode API
- ADR-016: _internal Extraction Pattern
- ADR-017: LAN-Only CORS Without Authentication
- ADR-008: All business logic in Rust backend (server mode relies on this)
- `_tasks/55-server-mode/`: Original planning and design docs
