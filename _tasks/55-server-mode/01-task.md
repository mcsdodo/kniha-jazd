**Date:** 2026-03-07
**Subject:** Embedded Server Mode - Access Kniha Jázd from LAN browsers
**Status:** Planning

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
- [ ] REST API routes mirroring all Tauri commands
- [ ] Settings UI toggle (start/stop server)
- [ ] Display server URL with local IP
- [ ] Frontend API adapter (detect Tauri vs browser)
- [ ] Static file serving (SvelteKit build output)
- [ ] CORS configuration for LAN access
- [ ] Receipt image serving via HTTP

### Exclude (future)
- Authentication
- HTTPS/TLS
- SSE for real-time updates
- Mobile-responsive layout
- QR code for easy phone connection
- Auto-discovery (mDNS/Bonjour)

## Security Considerations

- **No auth** - acceptable for home/office LAN
- **Bind to 0.0.0.0** - accessible to all devices on network
- **Read-only mode** - if desktop has lock conflict, browser also gets read-only
- **CORS** - allow all origins (LAN devices have various IPs)
- **Future**: add optional PIN/password protection

## Dependencies to Add

```toml
# src-tauri/Cargo.toml
axum = "0.8"
tower-http = { version = "0.6", features = ["cors", "fs"] }
# tokio already present
```

## References

- claude-devtools server mode: `_other_app/claude-devtools/src/main/services/infrastructure/HttpServer.ts`
- claude-devtools API adapter: `_other_app/claude-devtools/src/renderer/api/index.ts`
- Existing web deployment plan: `_tasks/33-web-deployment/`
- ADR-008: Backend-only calculations
