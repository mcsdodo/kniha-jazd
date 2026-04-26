**Date:** 2026-01-10 (revised 2026-04-25 after Task 55 completion)
**Subject:** Web/Headless Deployment — Run without desktop UI, deploy to Docker
**Status:** Planning (plan rewritten)

## Goal

Run Kniha Jázd without a desktop window — as a standalone server binary in Docker, and as a headless desktop app. LAN browsers connect to the embedded HTTP server. **Done = integration tests pass against a locally-running Docker container.**

## Context: Task 55 Changed Everything

[Task 55 (Server Mode)](../_done/55-server-mode/01-task.md) delivered the entire HTTP infrastructure:

| Component | Status | Where |
|-----------|--------|-------|
| Axum HTTP server | Done | [server/mod.rs](../../src-tauri/src/server/mod.rs) |
| RPC dispatcher (67 commands) | Done | [server/dispatcher.rs](../../src-tauri/src/server/dispatcher.rs) |
| `_internal` extraction (all modules) | Done | [commands/](../../src-tauri/src/commands/) |
| Frontend dual-mode API adapter | Done | [api-adapter.ts](../../src/lib/api-adapter.ts) |
| Static file serving + SPA fallback | Done | [server/mod.rs](../../src-tauri/src/server/mod.rs) |
| CORS (LAN origins) | Done | [server/mod.rs](../../src-tauri/src/server/mod.rs) |
| Receipt image endpoint | Done | `GET /api/receipts/:id/image` |
| Capabilities endpoint | Done | `GET /api/capabilities` |
| Graceful shutdown | Done | [server/mod.rs](../../src-tauri/src/server/mod.rs) |
| Settings UI (start/stop, port) | Done | Settings page |
| Auto-start from settings | Done | [lib.rs](../../src-tauri/src/lib.rs) |
| Shared `Arc<Database>` | Done | Single connection, no SQLITE_BUSY |
| Export in browser mode | Done | `export_html` + `window.open()` |
| Server-mode integration tests | Done | [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) |

**The original Task 33 plan (10 tasks, 2-3 weeks) is obsolete.** What remains is ~2-3 days of work.

## Two Deliverables

### A. Standalone Server Binary (for Docker)

A separate binary (`web`) that uses the existing [server/](../../src-tauri/src/server/) module without any Tauri dependency. Tauri requires `libwebkit2gtk` + a display server on Linux — can't run in Docker without hacks.

- Standalone binary ~100 lines: config from env vars, `Database::new()`, `HttpServer::start()`
- Multi-stage Dockerfile (Rust + Node build, `debian:bookworm-slim` runtime)
- `docker-compose.web.yml` with `/data` volume for DB + receipts + backups

### B. Headless Desktop Mode (for always-on PCs)

A `--headless` flag on the existing Tauri binary that hides the window and auto-starts the server. For users running the app on a home/office PC without the UI.

## What's No Longer Needed (vs original plan)

| Original Task 33 Plan | Why Obsolete |
|----------------------|--------------|
| Task 0: Async DB adapter | `spawn_blocking` already used in [server dispatcher](../../src-tauri/src/server/mod.rs) |
| Task 0.5: WebConfig | Simplified to env vars in standalone binary |
| Task 1: Axum module structure | [server/](../../src-tauri/src/server/) module exists |
| Task 2-5: All handlers | All 67 `_internal` fns + RPC dispatcher exist |
| Task 6: Frontend API migration | [api-adapter.ts](../../src/lib/api-adapter.ts) with dual-mode `apiCall()` exists |
| Task 7: Remove Tauri frontend code | Not needed — same frontend works in both modes |
| Task 9: Static serving + health | `ServeDir` + SPA fallback + `/health` exist |
| Task 10: Testing | 280 backend tests + server integration tests exist |

## Verification

All test suites must pass:
- Backend: `cd src-tauri && cargo test` (280 tests)
- Normal Tauri integration — all tiers: `npm run test:integration`
- Server-mode integration — all tiers (Tauri binary): `npm run test:integration:server`
- Docker integration — all tiers: `npm run test:integration:docker`

## References

- [Task 55 design](../_done/55-server-mode/02-design.md) — server architecture
- [Task 55 plan](../_done/55-server-mode/03-plan.md) — implementation details
- [Server integration test config](../../tests/integration/wdio.server.conf.ts) — existing Chrome-based test harness
- ADR-008: Backend-only calculations
