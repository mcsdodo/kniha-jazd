**Date:** 2026-03-07
**Subject:** Embedded Server Mode - Access Kniha Jázd from LAN browsers
**Status:** Ready for implementation

## Goal

Add an optional HTTP server mode to the Tauri desktop app so users can access the full UI from any browser on the local network (phone, tablet, other PC). The desktop app stays running as the "host" while browsers connect to it.

## Inspiration

[claude-devtools](https://github.com/matt1398/claude-devtools) implements a similar feature:
- Electron app embeds a Fastify HTTP server
- Toggle in settings to start/stop
- Same UI served to browser via static files
- Frontend detects Electron vs browser mode, uses IPC or HTTP accordingly
- SSE for real-time events

## Key Difference from Task 33 (Web Deployment)

| Aspect | Task 33 (Web Deployment) | Task 55 (Server Mode) |
|--------|-------------------------|----------------------|
| Architecture | Separate binary + Docker | Embedded in existing Tauri app |
| Data source | Docker volume | Desktop app's database |
| When to use | Always-on server | On-demand (toggle in Settings) |
| Build complexity | New binary, Dockerfile | Feature flag in existing app |
| Frontend | Separate build (no Tauri) | Dual-mode API adapter |

**Server Mode is simpler and more useful for the typical user** — they just toggle a switch and open their phone's browser.

## Requirements

### Functional
- Toggle server on/off in Settings (Nastavenia)
- Show server URL (e.g., `http://192.168.1.5:3456`) when active
- Browser clients see the same UI as the desktop app
- All CRUD operations work (vehicles, trips, receipts, routes)
- All calculations work identically (backend-only per ADR-008)
- Read-only mode awareness (if desktop is read-only, browser is too)

### Non-Functional
- **LAN only by default** - bind to `0.0.0.0` (local network)
- **No authentication** - trusted home/office network
- **Single database** - desktop app's existing SQLite database
- **No data sync conflicts** - single Mutex<SqliteConnection> serializes all access
- **Minimal latency** - Tauri IPC is already local, HTTP on LAN adds ~1ms

## Technical Approach

### Backend: Embed Axum HTTP server in Tauri app

The Tauri app already runs a tokio runtime. We can spawn an Axum server on it.

```
┌──────────────────────────────────────────────────┐
│                 Tauri Desktop App                 │
│                                                  │
│  ┌────────────────┐   ┌───────────────────────┐  │
│  │  Tauri Webview  │   │  Axum HTTP Server     │  │
│  │  (localhost)    │   │  (0.0.0.0:3456)       │  │
│  │                 │   │                       │  │
│  │  invoke() ──────┼───┤  GET/POST /api/* ─────┤  │
│  │                 │   │                       │  │
│  └────────┬───────┘   └──────────┬────────────┘  │
│           │                      │               │
│           ▼                      ▼               │
│  ┌────────────────────────────────────────────┐  │
│  │     Shared State: Database + AppState       │  │
│  │     (Arc<Database>, Arc<AppState>)          │  │
│  └────────────────────────────────────────────┘  │
│           │                                      │
│           ▼                                      │
│  ┌────────────────────────────────────────────┐  │
│  │            SQLite (Mutex<Connection>)       │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

### Frontend: Dual-mode API adapter

Similar to claude-devtools' approach:

```typescript
// src/lib/api.ts - detect runtime environment
function getApi(): ApiBackend {
  if (window.__TAURI_INTERNALS__) {
    return tauriBackend;   // invoke() - desktop
  }
  return httpBackend;      // fetch() - browser
}
```

### Why this works well for our app

1. **Database is already thread-safe** - `Mutex<SqliteConnection>` serializes access
2. **Tokio runtime exists** - Tauri uses tokio, Axum runs on tokio
3. **All business logic is in Rust** - HTTP handlers call the same functions as Tauri commands
4. **Serde serialization exists** - All types already implement `Serialize`/`Deserialize`
5. **SvelteKit builds to static files** - Can be served by Axum's `ServeDir`

## What claude-devtools does differently (and what we can skip)

| claude-devtools feature | Our approach |
|------------------------|-------------|
| Fastify (Node.js HTTP server) | Axum (Rust HTTP server) - natural for our stack |
| SSE for real-time events | Skip for MVP - not needed (no file watching) |
| SSH remote connections | Skip - not relevant |
| Standalone Docker mode | Already covered by Task 33 |
| Auto-reconnect EventSource | Skip for MVP |
| Multiple render path resolution | Simple: serve from Tauri's `frontendDist` path |

## MVP Scope

### Include
- [ ] Axum HTTP server embedded in Tauri app
- [ ] Single `/api/rpc` endpoint dispatching to all server-safe Tauri commands (not 71 individual REST routes — see `02-design.md` §2)
- [ ] `_internal` extraction pass across command modules so Tauri wrappers and RPC dispatcher share the same pure fn
- [ ] `/api/capabilities` endpoint exposing which commands are Tauri-only (updater, file dialogs, window mgmt)
- [ ] Graceful shutdown wired to Tauri's `RunEvent::ExitRequested`
- [ ] Settings UI toggle (start/stop server) + port input with conflict error
- [ ] Display server URL with local IP (via `local-ip-address` crate)
- [ ] Frontend API adapter (detect Tauri vs browser) + read-only store reading `get_app_mode` over RPC
- [ ] Static file serving with SPA fallback to `index.html` (`ServeDir::not_found_service`)
- [ ] CORS: allow-list of LAN origins (see Security below), not `*`
- [ ] Receipt image serving via `GET /api/receipts/:id/image` + shared `receiptImageUrl(id)` helper

### Exclude (future)
- Authentication
- HTTPS/TLS
- SSE for real-time updates
- Mobile-responsive layout
- QR code for easy phone connection
- Auto-discovery (mDNS/Bonjour)

## Security Considerations

- **No auth in MVP** — acceptable for trusted home/office LAN; PIN/password protection deferred to a follow-up task
- **Bind to 0.0.0.0** only when user explicitly enables server mode; default bind is 127.0.0.1 at scaffold time
- **Read-only mode** — if desktop has lock conflict, browser also gets read-only (enforced by existing `check_read_only!` macro; browser UI reads `get_app_mode` over RPC)
- **CORS:** allow-list private-range origins (`http://10.*`, `http://172.16-31.*`, `http://192.168.*`, and `http://localhost`) rather than `*`. A public-domain page must not be able to `fetch()` into the LAN service from a user's browser.
- **CSRF mitigation:** require a custom header `X-KJ-Client: 1` on all POST requests. Custom headers trigger a CORS preflight, which our Origin allow-list will reject from non-LAN pages. Not cryptographic protection — it raises the bar above passive browser-based attacks.
- **Attack surface acknowledged:** with `bind 0.0.0.0` + no auth, any device already on the LAN can read and mutate data. The Origin allow-list + `X-KJ-Client` header stops **cross-site** attacks from malicious pages visited on LAN devices, not **on-network** attackers. Users who can't trust their LAN should leave server mode off.
- **Future:** optional PIN, HTTPS via self-signed cert or mDNS + Let's Encrypt DNS-01, per-device tokens.

## Dependencies to Add

```toml
# src-tauri/Cargo.toml
axum = "0.8"
tower-http = { version = "0.6", features = ["cors", "fs"] }

# tokio is already in the manifest but only has ["fs"]. Axum 0.8 needs a multi-threaded
# runtime, macros, networking, and (for graceful shutdown) signal handling:
tokio = { version = "1", features = ["fs", "rt-multi-thread", "macros", "net", "signal", "sync"] }

# LAN IP detection for the Settings URL display. `hostname` alone doesn't give us the
# outbound interface IP; this crate picks the default-route interface.
local-ip-address = "0.6"
```

Expected compile-time impact: +8–15s cold builds due to Axum pulling in hyper, h2, and tower. Binary size grows ~1.5 MB. Acceptable for desktop.

## References

- claude-devtools server mode: `_other_app/claude-devtools/src/main/services/infrastructure/HttpServer.ts`
- claude-devtools API adapter: `_other_app/claude-devtools/src/renderer/api/index.ts`
- Existing web deployment plan: `_tasks/33-web-deployment/`
- ADR-008: Backend-only calculations
