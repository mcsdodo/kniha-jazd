**Date:** 2026-01-10 (revised 2026-04-25 after Task 55 completion)
**Subject:** Web/Headless Deployment — Run without desktop UI
**Status:** Planning (plan needs rewrite)

## Goal

Run Kniha Jázd without a desktop window — either as a headless process on a desktop/server PC, or as a standalone binary in Docker. LAN browsers connect to the embedded HTTP server.

## Context: Task 55 Changed Everything

[Task 55 (Server Mode)](_done/55-server-mode/01-task.md) delivered the entire HTTP infrastructure:

| Component | Status | Where |
|-----------|--------|-------|
| Axum HTTP server | ✅ Done | `server/mod.rs` |
| RPC dispatcher (67 commands) | ✅ Done | `server/dispatcher.rs`, `dispatcher_async.rs` |
| `_internal` extraction (all modules) | ✅ Done | `commands/*.rs` |
| Frontend dual-mode API adapter | ✅ Done | `src/lib/api-adapter.ts` |
| Static file serving + SPA fallback | ✅ Done | `server/mod.rs` |
| CORS (LAN origins) | ✅ Done | `server/mod.rs` |
| Receipt image endpoint | ✅ Done | `GET /api/receipts/:id/image` |
| Capabilities endpoint | ✅ Done | `GET /api/capabilities` |
| Graceful shutdown | ✅ Done | `server/mod.rs` |
| Settings UI (start/stop, port) | ✅ Done | Settings page |
| Auto-start from settings | ✅ Done | `lib.rs` |
| Shared `Arc<Database>` | ✅ Done | Single connection, no SQLITE_BUSY |
| Export in browser mode | ✅ Done | `export_html` + `window.open()` |

**The original Task 33 plan (10 tasks, 2-3 weeks) is obsolete.** What remains is ~1-2 days of work.

## Two Deployment Targets

### Target A: Headless Desktop (hidden window)

Run the existing Tauri app without showing a window. For always-on home PCs or office servers.

- **Pros:** Zero new code beyond a CLI flag, reuses the exact same build artifact
- **Cons:** Requires a machine that can run the desktop app (display system on Linux)
- **Binary size:** 17MB release — negligible

### Target B: Docker (standalone binary)

A separate binary that uses the `server/` module without Tauri. For Docker/NAS deployment.

- **Pros:** No webview dependency, ~5MB smaller, runs anywhere Linux runs
- **Cons:** Needs a small new binary + Dockerfile + env-based config
- **Why Tauri can't run in Docker:** Tauri v2 on Linux requires `libwebkit2gtk` and a display server (X11/Wayland). Even with a hidden window, GTK runtime initialization fails without a display. Xvfb is a hack, not a solution.

## Decision: Hidden Window (Target A) First

Ship Target A first — it's essentially free. Target B (Docker) is a follow-up that reuses the same `server/` module, just wrapped in a standalone main().

## What Remains

### For Target A (hidden window)
1. CLI arg or env var (`--headless` / `KNIHA_JAZD_HEADLESS=1`) to skip window creation
2. Auto-start server in headless mode (already works via `KNIHA_JAZD_SERVER_AUTOSTART`)
3. Console output with server URL
4. Documentation

### For Target B (Docker) — follow-up
1. `src-tauri/src/bin/web.rs` — standalone main() (~100 lines)
2. `WebConfig` struct reading env vars (database path, port, Gemini key)
3. Receipt path normalization (desktop absolute paths → Docker volume relative paths)
4. `Dockerfile.web` + `docker-compose.web.yml`
5. Migration guide (copy DB + receipts to Docker volume)

## What's No Longer Needed (vs original plan)

| Original Task 33 Plan | Why Obsolete |
|----------------------|--------------|
| Task 0: Async DB adapter | `spawn_blocking` already used in server dispatcher |
| Task 0.5: WebConfig | Only needed for Docker (Target B) |
| Task 1: Axum module structure | `server/` module exists |
| Task 2-5: All handlers | All 67 `_internal` fns + RPC dispatcher exist |
| Task 6: Frontend API migration | `api-adapter.ts` with dual-mode `apiCall()` exists |
| Task 7: Remove Tauri frontend code | Not needed — same frontend works in both modes |
| Task 9: Static serving + health | `ServeDir` + SPA fallback + `/health` exist |
| Task 10: Testing | 280 backend tests + 5 server tests exist |

## References

- [Task 55 design](../_done/55-server-mode/02-design.md) — server architecture
- [Task 55 plan](../_done/55-server-mode/03-plan.md) — implementation details
- ADR-008: Backend-only calculations
