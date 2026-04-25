# Feature: Server Mode

> Access the app from any browser on your local network (phone, tablet, other PC) via an embedded HTTP server. Available in three deployment shapes: in-app toggle, headless desktop binary, or Docker container.

## Deployment Modes

| Mode | When to use | How |
|------|-------------|-----|
| **In-app toggle** | You normally use the desktop UI but want occasional phone/tablet access | Settings → enable Server toggle |
| **Headless desktop** | Always-on home/office PC running the server in the background | `KNIHA_JAZD_HEADLESS=1` or `--headless` flag |
| **Docker container** | Linux server, NAS, or anything without a desktop session | [`docker-compose.web.yml`](../../docker-compose.web.yml) |

All three modes share the same Axum HTTP layer and the same RPC dispatcher (see [Technical Implementation](#technical-implementation)).

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

## Headless Desktop Mode

Run the desktop binary as a background HTTP server with no window. Useful for an always-on home or office PC, behind Windows Task Scheduler or a systemd-equivalent service.

**Activation (any one of):**

- CLI flag: `kniha-jazd.exe --headless`
- Env var: `KNIHA_JAZD_HEADLESS=1`

**Behaviour:**

- The desktop window is constructed (Tauri requires it) but is hidden immediately.
- The server is force-started even if the in-app toggle is off.
- The server URL (with LAN IP) is printed to stdout for the user to share.

**Configuration:**

| Env var | Default | Purpose |
|---------|---------|---------|
| `KNIHA_JAZD_HEADLESS` | unset | Enable headless mode |
| `KNIHA_JAZD_SERVER_PORT` | `3456` | HTTP listen port |
| `KNIHA_JAZD_SERVER_AUTOSTART` | unset | Equivalent to enabling the in-app toggle |
| `KNIHA_JAZD_DATA_DIR` | platform default | Override DB / receipts / backups directory |

**Limitations:** The same browser-mode capability gating applies — file dialogs, the auto-updater, "Open external", "Restore backup", and "Move database" are all unavailable.

## Docker Deployment

A standalone [`web`](../../src-tauri/src/bin/web.rs) binary runs the same HTTP server without the Tauri shell. Packaged in a multi-stage [`Dockerfile.web`](../../Dockerfile.web) producing a slim runtime image.

**Quick start:**

```sh
mkdir -p data
docker compose -f docker-compose.web.yml up -d
# App is now at http://localhost:3456
```

**Migrating from desktop to Docker:** Copy the existing database and (optionally) the `receipts/` and `backups/` folders from the platform app data directory into the host's `./data/` folder. They'll be mounted into the container at `/data`.

**Configuration (env vars):**

| Variable | Default in image | Purpose |
|----------|------------------|---------|
| `PORT` | `3456` | HTTP listen port |
| `KNIHA_JAZD_DATA_DIR` | `/data` | Where DB, receipts, backups live (mounted as a volume) |
| `DATABASE_PATH` | `/data/kniha-jazd.db` | Override the DB file path |
| `STATIC_DIR` | `/var/www/html` | Built SvelteKit assets — generally don't change |
| `GEMINI_API_KEY` | unset | Optional, enables receipt OCR |

**Limitations:** Same as Headless Mode — no native dialogs, no auto-updater, no LAN IP display in the UI (since the container doesn't have a real LAN IP, only the Docker bridge).

**Tech debt:** The runtime image currently carries GTK/WebKit shared libraries because Tauri is a non-optional dependency of the parent crate. See [tech debt 06](../../_tasks/_TECH_DEBT/06-tauri-feature-gating.md) for the planned fix.

## Technical Implementation

### Frontend

**Runtime Detection:** [api-adapter.ts](../../src/lib/api-adapter.ts)
- Detects whether running inside Tauri webview or a regular browser
- Tauri mode: uses `invoke()` IPC as before
- Browser mode: sends `POST /api/rpc` with `{ command, args }` JSON to the server

**Capabilities Store:** [capabilities.ts](../../src/lib/stores/capabilities.ts)
- Fetches `GET /api/capabilities` on load (browser mode only)
- Returns which commands are available (68 of 72 are server-safe)
- Components check capabilities to hide unavailable features

**Settings UI:** [+page.svelte](../../src/routes/settings/+page.svelte)
- Server toggle, auto-start checkbox, URL display
- Shows local IP address for easy sharing

### Backend (Rust)

**Server Module:** [server/mod.rs](../../src-tauri/src/server/mod.rs)
- Axum HTTP server with `POST /api/rpc` endpoint
- `GET /health` for readiness checks
- Static file serving for SPA (built frontend files)
- CORS layer restricting origins to RFC 1918 private ranges + localhost

**RPC Dispatcher:** [dispatcher.rs](../../src-tauri/src/server/dispatcher.rs) + [dispatcher_async.rs](../../src-tauri/src/server/dispatcher_async.rs)
- Maps command names to `_internal` functions
- Sync commands dispatched via `spawn_blocking`
- Async commands (receipts OCR, HA integration, export) awaited directly

**Server Manager:** [manager.rs](../../src-tauri/src/server/manager.rs)
- Start/stop lifecycle management
- LAN IP detection via `local-ip-address` crate
- Port binding (default 3456)
- Graceful shutdown via tokio oneshot channel

**_internal Functions:** [commands/](../../src-tauri/src/commands/)
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
| [server/mod.rs](../../src-tauri/src/server/mod.rs) | Axum router, RPC handler, CORS, static files |
| [server/dispatcher.rs](../../src-tauri/src/server/dispatcher.rs) | Sync command dispatch (60+ commands) |
| [server/dispatcher_async.rs](../../src-tauri/src/server/dispatcher_async.rs) | Async command dispatch (7 commands) |
| [server/manager.rs](../../src-tauri/src/server/manager.rs) | Server lifecycle, LAN IP, port binding |
| [commands/](../../src-tauri/src/commands/) | `_internal` functions shared by both paths |
| [bin/web.rs](../../src-tauri/src/bin/web.rs) | Standalone server binary (Docker) |
| [lib.rs](../../src-tauri/src/lib.rs) | Tauri setup with `--headless` mode |
| [Dockerfile.web](../../Dockerfile.web) | Multi-stage Docker build |
| [docker-compose.web.yml](../../docker-compose.web.yml) | Docker Compose wiring |
| [api-adapter.ts](../../src/lib/api-adapter.ts) | Runtime detection + RPC adapter |
| [capabilities.ts](../../src/lib/stores/capabilities.ts) | Feature gating for browser mode |
| [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) | Server-mode integration test config (Tauri + Docker) |

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
- [_tasks/_done/55-server-mode/](../../_tasks/_done/55-server-mode/): Original server-mode planning and design
- [_tasks/_done/33-web-deployment/](../../_tasks/_done/33-web-deployment/): Headless and Docker deployment work
- [tech debt 06](../../_tasks/_TECH_DEBT/06-tauri-feature-gating.md): Planned image-size reduction
