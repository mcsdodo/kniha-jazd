**Date:** 2026-01-10
**Subject:** Web App Deployment - Convert Tauri desktop app to Docker-hosted web app
**Status:** Planning

## Goal

Deploy Kniha Jázd as a web application in Docker, enabling remote access without desktop installation while preserving all existing business logic and the SQLite database.

## Requirements

### Functional
- Full CRUD operations for vehicles, trips, receipts, routes
- All calculations (consumption, margins, compensation suggestions) work identically
- Receipt viewing and management
- HTML export functionality
- Backup/restore capability

### Non-Functional
- **No authentication** - Single user, trusted network deployment
- **Keep SQLite** - No database migration to PostgreSQL
- **No mobile layouts** - Desktop-focused web UI (existing layout)
- **Docker deployment** - Single container with volume for data persistence

## Technical Constraints

### What MUST Stay Unchanged
- `calculations.rs` - All consumption/margin logic (108 tests validate this)
- `db.rs` - SQLite + Diesel ORM (with async wrapper)
- `suggestions.rs` - Compensation trip logic
- `models.rs` - All data structures
- `export.rs` - HTML generation
- Database schema - No changes
- Frontend components - Only API layer changes

### What Changes
| Component | Current | Web Version |
|-----------|---------|-------------|
| Backend framework | Tauri | Axum (thin HTTP wrapper) |
| Frontend API | `invoke()` | `fetch()` |
| Window management | Tauri APIs | Remove (not needed) |
| File access | Desktop paths | Docker volume `/data` |
| Receipt viewing | `openPath()` | Serve via API or static files |
| Settings/paths | `LocalSettings` + `app_data_dir` | `WebConfig` from environment |
| Gemini API key | `local.settings.json` | `GEMINI_API_KEY` env var |
| Receipt paths | Absolute Windows paths | Normalized relative paths |

## Technical Notes (from Plan Review)

### Critical Issues Addressed in Plan

1. **Async + Mutex Deadlock** - Current `Database` uses `std::sync::Mutex<SqliteConnection>`. Holding across async `.await` causes deadlock. Solution: Use `spawn_blocking` wrapper for all DB operations.

2. **Path Abstraction** - Desktop uses `get_app_data_dir(&app)` and `LocalSettings::load()`. Web uses `WebConfig` struct reading from environment variables.

3. **Receipt Image Paths** - Desktop stores absolute paths (`C:\Users\...\receipts\file.jpg`). Web normalizes to relative paths and serves from `/data/receipts/`.

4. **Gemini API Key** - Desktop reads from `local.settings.json`. Web reads from `GEMINI_API_KEY` environment variable.

### MVP Simplifications

- **No real-time progress** - Receipt sync shows loading state, refreshes on completion (no SSE/WebSocket)
- **No mobile layouts** - Desktop-focused UI only
- **No authentication** - Single user, trusted network deployment

## Architecture

```
┌─────────────────────────────────────────────────┐
│           SvelteKit Frontend (Static)           │
│         fetch('/api/...') instead of invoke()   │
├─────────────────────────────────────────────────┤
│              Axum HTTP Server                   │
│         (Thin wrapper - ~200 lines new code)    │
├─────────────────────────────────────────────────┤
│         UNCHANGED: calculations.rs              │
│         UNCHANGED: db.rs (SQLite + Diesel)      │
│         UNCHANGED: suggestions.rs               │
├─────────────────────────────────────────────────┤
│              Docker Volume: /data               │
│         kniha-jazd.db + receipts/               │
└─────────────────────────────────────────────────┘
```

## Security Considerations

Since there's no authentication:
- Deploy only on trusted/private networks
- Use VPN for remote access, OR
- Add nginx reverse proxy with basic auth if needed later

## Success Criteria

1. All 108 Rust backend tests pass
2. All existing integration tests pass (adapted for web)
3. Can create/edit/delete vehicles and trips via browser
4. Can view and manage receipts
5. Export functionality works
6. Data persists across container restarts
7. Existing desktop database can be migrated by copying file

## References

- Detailed analysis: `C:\Users\Dodo\.claude\plans\curious-bouncing-wigderson.md`
- ADR-008: Backend-only calculations (must be preserved)
