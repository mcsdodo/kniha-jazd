# Feature: Server Mode

> The app is an HTTP server. One Rust process serves the SvelteKit SPA and a JSON-RPC
> endpoint; every client is a browser on the local network — phone, tablet, laptop.
> There is no desktop build and no other mode.

## Deployment

| Shape | When to use | How |
|-------|-------------|-----|
| **Docker container** | The normal case: NAS, Raspberry Pi, homelab, any always-on Linux box | `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` or [`docker-compose.web.yml`](../../docker-compose.web.yml) |
| **Bare binary** | Local development, or a host where Docker is unwanted | `cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web` |

Both run the same [`kniha-jazd-web`](../../src-tauri/web/src/main.rs) binary against the
same [`kniha-jazd-core`](../../src-tauri/core/) logic — the container just supplies the
data volume, the built SPA and the env vars.

## User Flow

1. **Start** the container (or the binary) on the always-on machine
2. **Open** `http://<server-ip>:3456` on any device connected to the same LAN
3. **Use** the app normally — trips, vehicles, receipts, maps, export all work
4. **Bookmark** it; there is nothing to install on the client

**Notes:**
- The server binds `0.0.0.0`, so it is reachable from the LAN, not just localhost.
- All data changes are immediately visible to every other open browser after a refresh —
  there is one database.
- Native file dialogs do not exist. Paths (such as the receipts folder) are typed in as
  server-side paths.

## Docker Deployment

**Quick start:**

```sh
mkdir -p data
docker run -d --name kniha-jazd \
  -p 3456:3456 \
  -v "$PWD/data:/data" \
  --restart unless-stopped \
  ghcr.io/mcsdodo/kniha-jazd-web:latest
```

[`Dockerfile.web`](../../Dockerfile.web) is a three-stage build: the Rust web binary,
the SvelteKit static build, then a `debian:bookworm-slim` runtime carrying only the
binary, the SPA and `ca-certificates`. It declares a `/data` volume, exposes 3456 and
has a `HEALTHCHECK` on `/health`.

**Migrating from an old desktop install:** copy the existing database and (optionally)
the `receipts/` and `backups/` folders from the platform app-data directory into the
host's `./data/` folder. They are mounted into the container at `/data`, and migrations
run on the next start.

**Configuration (env vars):**

| Variable | Default in image | Purpose |
|----------|------------------|---------|
| `PORT` | `3456` | HTTP listen port |
| `KNIHA_JAZD_DATA_DIR` | `/data` | Where DB, receipts, backups and `local.settings.json` live (mounted as a volume) |
| `DATABASE_PATH` | `<DATA_DIR>/kniha-jazd.db` | Override the DB file path |
| `STATIC_DIR` | `/var/www/html` | Built SvelteKit assets. Leave **unset** in local dev so vite serves the UI instead |
| `GEMINI_API_KEY` | unset | Optional, enables receipt OCR (magic fill) |
| `HA_URL` | unset | Home Assistant base URL for the odometer integration |
| `HA_API_TOKEN` | unset | Home Assistant long-lived access token |
| `PAPERLESS_URL` | unset | Paperless-ngx base URL for receipt sync |
| `PAPERLESS_API_TOKEN` | unset | Paperless-ngx API token |
| `PAPERLESS_ENABLED` | unset | Enable Paperless sync — truthy values `1`/`true`/`yes` (case-insensitive); any other non-empty value means disabled |
| `KNIHA_JAZD_REVEAL_PIN` | unset | PIN required to display a secret in Settings. Unset means secrets cannot be revealed over the network at all. |

**Precedence:** The six integration/secret variables (`GEMINI_API_KEY`, `HA_URL`, `HA_API_TOKEN`, `PAPERLESS_URL`, `PAPERLESS_API_TOKEN`, `PAPERLESS_ENABLED`) override the corresponding fields in `local.settings.json` — env wins whenever the variable is set to a non-empty value (empty/whitespace-only values are treated as unset). Env values are never written to disk. When a field is pinned by an env variable, the Settings page renders it **disabled** and badges it with the variable's name; the eye icon on a pinned token reveals the live value. The setter commands still refuse such writes with an explanatory error ("… is managed by the … environment variable") — see [settings-architecture.md](./settings-architecture.md).

**Secrets are never served to the network.** Settings reads report only whether a credential is configured; displaying one requires `KNIHA_JAZD_REVEAL_PIN` and is throttled after repeated failures. This does not extend the CORS allowlist into an access control — it is a separate gate on credentials specifically, because CORS only constrains browsers. See [ADR-027](../../DECISIONS.md). Preferences such as theme, hidden columns, and Paperless custom field names remain file/UI-managed. See [settings.rs](../../src-tauri/core/src/settings.rs).

**Limitations:** no native dialogs and no LAN IP display in the UI (the container has
only the Docker bridge address, so the operator supplies the reachable URL).

### Homelab Deployment

The release workflow publishes the official image on every `v*` tag:
`ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` (plus `latest`), built by the `docker-image` job
in [release.yml](../../.github/workflows/release.yml). That image **is** the release —
no GitHub Release, no installers, no updater. Updating means pulling a newer tag and
restarting the container; the `/data` volume carries the database across.

## Local Development

Two processes:

```sh
# 1) backend on 3456 — STATIC_DIR unset, so it serves the API only
cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web

# 2) frontend on 5173 — vite serves the SPA and proxies /api to 3456
npm run dev
```

The proxy lives in [vite.config.ts](../../vite.config.ts). Set `KNIHA_JAZD_DATA_DIR` to
a scratch folder first, otherwise the binary falls back to `/data`.

## Technical Implementation

### Frontend

**RPC adapter:** [api-adapter.ts](../../src/lib/api-adapter.ts)
- A single `apiCall(command, args)` that POSTs `{ command, args }` to `/api/rpc` with an
  `X-KJ-Client: 1` header, and throws the response body on a non-2xx
- [api.ts](../../src/lib/api.ts) wraps each backend command in a typed function; components
  never call `fetch` themselves

**Read-only state:** [appMode.ts](../../src/lib/stores/appMode.ts)
- Calls `get_app_mode` and exposes `isReadOnly` + `readOnlyReason` to the UI
- Falls back to a permissive default if the call fails

### Backend (Rust)

**Binary:** [web/src/main.rs](../../src-tauri/web/src/main.rs)
- Reads `PORT`, `KNIHA_JAZD_DATA_DIR`, `DATABASE_PATH`, `STATIC_DIR`
- Creates the data directory, opens the database, then starts the server on a tokio runtime

**Server Module:** [server/mod.rs](../../src-tauri/core/src/server/mod.rs)
- Axum router: `POST /api/rpc`, `GET /api/capabilities`, `GET /api/receipts/{id}/image`,
  `GET /health`
- Static file serving for the SPA, with `index.html` as the SPA fallback. If `STATIC_DIR`
  has no `index.html` the fallback is skipped and only the API is served — which is
  exactly what local dev wants
- CORS layer restricting origins to RFC 1918 private ranges + localhost
- Binds `0.0.0.0` when `bind_all` is set (the binary sets it), `127.0.0.1` otherwise

**RPC Dispatcher:** [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) + [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)
- Maps command names to `_internal` functions — one `match` arm per command, currently
  64 sync and 12 async (count the arms in `dispatch_sync` / `dispatch_async` rather than
  trusting this number; it moves whenever a command is added or removed)
- Sync commands dispatched via `spawn_blocking`
- Async commands (receipts OCR, HA integration, export, `get_trip_grid_data`) awaited directly
- Backup filenames arriving over RPC pass through `validate_backup_filename` ([commands_internal/backup.rs](../../src-tauri/core/src/commands_internal/backup.rs)) — empty names, path separators, `..`, and drive/ADS colons are rejected before any file access (defense-in-depth for the network-reachable restore/delete endpoints)

**_internal Functions:** [commands_internal/](../../src-tauri/core/src/commands_internal/)
- Plain functions taking `&Database` / `&AppState` / `&Path` as references
- All orchestration lives here; the dispatcher only parses args and forwards

### Data Flow

```
Browser (phone, tablet, laptop)
     |
     | fetch('/api/rpc', { command, args })
     v
 Axum HTTP Server  (kniha-jazd-web)
     |
     | dispatch(command, args)   ->  spawn_blocking / await
     v
 _internal(db, app_state, args)  (kniha-jazd-core)
     |
     v
 SQLite Database  (/data/kniha-jazd.db)
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
    "restore_backup": true,
    "move_database": false,
    "route_maps": true
  }
}
```

`read_only` reflects `AppState::is_read_only()`. Note that **nothing currently sets it** —
the migration-compatibility check that used to arm it has no caller left, so this field is
always `false` in practice. See [read-only-mode.md](./read-only-mode.md). The UI reads the
same state through the `get_app_mode` command rather than this endpoint.

The `features` block records what this deployment can and cannot do. The flags that are
permanently `false` are the native affordances only a desktop shell could provide.
`restore_backup` is served: it performs an `fs::copy`, and the browser reloads the page
afterwards so every view re-reads the restored database.

### CORS

The CORS layer allows origins matching RFC 1918 private IP ranges:
- `http://10.*.*.*:*`
- `http://172.16-31.*.*:*`
- `http://192.168.*.*:*`
- `http://localhost:*` / `http://127.0.0.1:*`

Requests from public IPs or other origins are blocked by the browser's preflight check.
This is not authentication — see ADR-017 and the tailnet-trust model in ADR-024.

### Receipt Image Serving

`GET /api/receipts/{id}/image` looks up the receipt by ID in the database, then serves the image file from disk, so the browser can display scanned receipts.

## Key Files

| File | Purpose |
|------|---------|
| [web/src/main.rs](../../src-tauri/web/src/main.rs) | Binary entrypoint: env vars, DB open, server start |
| [server/mod.rs](../../src-tauri/core/src/server/mod.rs) | Axum router, RPC handler, capabilities, CORS, static files |
| [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | Sync command dispatch (`dispatch_sync`, 64 arms) |
| [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | Async command dispatch (`dispatch_async`, 12 arms) |
| [commands_internal/](../../src-tauri/core/src/commands_internal/) | The `_internal` functions the dispatcher calls |
| [Dockerfile.web](../../Dockerfile.web) | Multi-stage Docker build |
| [docker-compose.web.yml](../../docker-compose.web.yml) | Local build + run wiring |
| [api-adapter.ts](../../src/lib/api-adapter.ts) | `apiCall()` — the single RPC entry point |
| [api.ts](../../src/lib/api.ts) | Typed wrapper per backend command |
| [appMode.ts](../../src/lib/stores/appMode.ts) | Read-only gating for the UI |
| [vite.config.ts](../../vite.config.ts) | Dev proxy: `/api` → `localhost:3456` |
| [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) | Integration test config (spawned binary or container) |

## Design Decisions

- **Why RPC over REST?** -- A single `POST /api/rpc` endpoint keeps one command name per backend function. No need to design 80 separate REST routes for an internal-only API. (See ADR-015)

- **Why `_internal` extraction?** -- The pattern originally bridged two callers (desktop IPC and the HTTP dispatcher). Only the dispatcher remains, but plain functions over `&Database` / `&AppState` are still the right shape: they are directly unit-testable without a server. (See ADR-016)

- **Why no authentication?** -- Server is LAN-only (CORS-enforced). Target environment is a trusted home network or tailnet. Authentication would add significant complexity for minimal security benefit. (See ADR-017, ADR-024)

- **Why one process?** -- One process owns one SQLite file. No IPC, no stale caches, and no second writer to coordinate with.

- **Why are some capability flags permanently false?** -- `file_dialogs`, `updater`, `open_external` and `move_database` describe native affordances that only a desktop shell could provide. They are reported as `false` so the UI never offers them.

## Related

- ADR-015: RPC Over REST for Server Mode API
- ADR-016: _internal Extraction Pattern
- ADR-017: LAN-Only CORS Without Authentication
- ADR-024: Homelab server is the canonical deployment
- ADR-008: All business logic in Rust backend (server mode relies on this)
- [_tasks/_done/55-server-mode/](../../_tasks/_done/55-server-mode/): Original server-mode planning and design
- [_tasks/_done/33-web-deployment/](../../_tasks/_done/33-web-deployment/): Headless and Docker deployment work
