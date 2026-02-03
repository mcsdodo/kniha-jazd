---
paths:
  - "src-tauri/**/*.rs"
---

# Rust Backend Rules

## Architecture Reminder

All business logic lives in Rust backend only (ADR-008). Frontend is display-only.

- **`get_trip_grid_data`** - Returns trips + pre-calculated rates, warnings, fuel remaining
- **No calculation duplication** - Tauri IPC is local/fast, no need for client-side calculations

## Adding a New Tauri Command

1. Add function to `commands.rs` with `#[tauri::command]`
2. Register in `lib.rs` `invoke_handler` (not main.rs)
3. If write command, add `check_read_only!(app_state);` guard at start
4. Call from Svelte component via `invoke("command_name", { args })`

## Adding a New Calculation

1. Write failing test in `calculations_tests.rs` (cover all edge cases)
2. Implement in `calculations.rs` to make test pass
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

**Backend (Rust) - Authoritative source for all business logic (195 tests):**
- `commands_tests.rs` - 61 tests: receipt matching, period rates, warnings, fuel remaining, year carryover, BEV energy, receipt assignment, backup cleanup, magic fill
- `calculations_tests.rs` - 33 tests: consumption rate, spotreba, zostatok, margin, Excel verification
- `receipts_tests.rs` - 17 tests: folder detection, extraction, scanning
- `db_tests.rs` - 15 tests: CRUD lifecycle, year filtering
- `calculations_energy_tests.rs` - 15 tests: BEV battery remaining, energy consumption
- `db_location.rs` - 11 tests: custom paths, lock files, multi-PC support
- `calculations_phev_tests.rs` - 8 tests: PHEV combined fuel + energy
- `settings.rs` - 7 tests: local settings loading/saving
- `app_state.rs` - 6 tests: read-only mode, app state management
- `export.rs` - 6 tests: export totals, HTML escaping
- `gemini.rs` - 4 tests: JSON deserialization

**Remember:** Backend tests = "Is the calculation correct?"

## Key Files Reference

| File | Purpose | When to Modify |
|------|---------|----------------|
| `lib.rs` | Tauri app setup, command registration | Adding new Tauri commands |
| `calculations.rs` | All consumption/margin math | Adding/changing calculations |
| `calculations_tests.rs` | Tests for calculations | Adding calculation tests |
| `calculations_energy.rs` | BEV battery, energy calculations | Electric vehicle logic |
| `calculations_phev.rs` | PHEV combined fuel + energy | Plug-in hybrid logic |
| `suggestions.rs` | Compensation trip logic | Route matching, suggestions |
| `receipts.rs` | Receipt folder scanning | Receipt processing logic |
| `db.rs` | SQLite CRUD operations | Schema changes, queries |
| `db_location.rs` | Custom DB path, lock files | Database location features |
| `app_state.rs` | Read-only mode, app mode | App state management |
| `settings.rs` | Local settings (theme, paths) | User preferences |
| `gemini.rs` | AI receipt OCR | Receipt recognition |
| `commands.rs` | Tauri command handlers | New frontend→backend calls |
| `commands_tests.rs` | Tests for commands | Adding command tests |
| `export.rs` | HTML/PDF generation | Report format changes |
| `models.rs` | Data structures | Adding fields to Trip/Vehicle |
| `schema.rs` | Diesel ORM schema | After DB migrations |
