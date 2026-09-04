---
paths:
  - "src-tauri/**/*.rs"
---

# Rust Backend Rules

## Architecture Reminder

All business logic lives in Rust backend only (ADR-008). Frontend is display-only.

- **`get_trip_grid_data`** - Returns trips + pre-calculated rates, warnings, fuel remaining
- **No calculation duplication** - the RPC round-trip is same-host and cheap, no need for client-side calculations

The workspace has two members: `kniha-jazd-core` (everything - logic, DB, HTTP
server, tests) and `kniha-jazd-web` (a thin `main.rs` that reads env vars and starts
the server). New backend code almost always belongs in `core`.

## Adding a New Command

1. Add a `*_internal` function to the right module under
   `src-tauri/core/src/commands_internal/`
2. Register the command name in `src-tauri/core/src/server/dispatcher.rs` (sync) or
   `dispatcher_async.rs` (async)
3. If it writes, add `check_read_only!(app_state);` guard at start
4. Call it from the frontend via `apiCall("command_name", { args })` in `src/lib/api.ts`

## Adding a New Calculation

1. Write failing test in `calculations/tests.rs` (cover all edge cases)
2. Implement in `calculations/mod.rs` to make test pass
3. Expose via `get_trip_grid_data` or new command
4. Frontend receives pre-calculated value (no client-side calculation)
5. If new UI element, add integration test for display verification (see `.claude/rules/integration-tests.md`)

## Test Organization

Tests are split into separate `*_tests.rs` files using the `#[path]` attribute pattern:

```rust
// In calculations.rs
#[cfg(test)]
#[path = "calculations_tests.rs"]
mod tests;
```

This keeps source files clean while maintaining private access (tests are still submodules).

**When adding tests:** Write tests in `*_tests.rs` companion file, not in the source file.

## Backend Test Coverage

**Backend (Rust) - Authoritative source for all business logic.** All of it lives in
`kniha-jazd-core`; `kniha-jazd-web` has no logic to test. Companion `*_tests.rs` files
cover, among others:
- `commands_internal/commands_tests.rs` - receipt matching, period rates, warnings, fuel remaining, year carryover, BEV energy, receipt assignment, backup cleanup, magic fill
- `calculations/tests.rs` - consumption rate, spotreba, zostatok, margin, Excel verification
- `calculations/energy_tests.rs` / `calculations/phev_tests.rs` - BEV battery and PHEV split
- `receipts_tests.rs` - folder detection, extraction, scanning
- `db_tests.rs` - CRUD lifecycle, year filtering
- `migration_tests.rs` - migration data integrity
- `settings_tests.rs` - local settings loading/saving, env overrides
- `export_tests.rs` - export totals, HTML escaping
- `gemini_tests.rs` - JSON deserialization

Run them with `npm run test:backend`
(`cargo test --manifest-path src-tauri/Cargo.toml --workspace`).

**Remember:** Backend tests = "Is the calculation correct?"

## Key Files Reference

Paths are relative to `src-tauri/core/src/` unless noted.

| File | Purpose | When to Modify |
|------|---------|----------------|
| `server/mod.rs` | Axum router, `/api/rpc`, `/health`, CORS, static files | HTTP surface changes |
| `server/dispatcher.rs` | Sync command dispatch (68 commands) | Registering a new command |
| `server/dispatcher_async.rs` | Async command dispatch (12 commands) | Registering a new async command |
| `commands_internal/` | The `*_internal` functions the dispatcher calls | New frontend→backend calls |
| `commands_internal/commands_tests.rs` | Tests for commands | Adding command tests |
| `calculations/mod.rs` | All consumption/margin math | Adding/changing calculations |
| `calculations/tests.rs` | Tests for calculations | Adding calculation tests |
| `calculations/energy.rs` | BEV battery, energy calculations | Electric vehicle logic |
| `calculations/phev.rs` | PHEV combined fuel + energy | Plug-in hybrid logic |
| `suggestions.rs` | Compensation trip logic | Route matching, suggestions |
| `receipts.rs` | Receipt folder scanning | Receipt processing logic |
| `db.rs` | SQLite CRUD operations | Schema changes, queries |
| `app_state.rs` | Read-only mode, app mode | App state management |
| `settings.rs` | Local settings + env overrides | User preferences, new env vars |
| `gemini.rs` | AI receipt OCR | Receipt recognition |
| `paperless.rs` | Paperless-ngx client | Invoice source integration |
| `export.rs` | HTML/PDF generation | Report format changes |
| `models.rs` | Data structures | Adding fields to Trip/Vehicle |
| `schema.rs` | Diesel ORM schema | After DB migrations |
| `../../web/src/main.rs` | Env var wiring + server start | New process-level config |
