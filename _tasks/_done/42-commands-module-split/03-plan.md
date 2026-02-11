# Implementation Plan: Commands Module Split

**Status:** ✅ COMPLETE
**Last Updated:** 2026-01-29

## Current State

| Module | Lines | Commands | Status |
|--------|-------|----------|--------|
| `mod.rs` | ~270 | helpers | ✅ Coordinator only |
| `vehicles.rs` | 147 | 6 | ✅ Complete |
| `backup.rs` | 581 | 11 + 8 tests | ✅ Complete |
| `trips.rs` | 255 | 9 | ✅ Complete |
| `statistics.rs` | ~1442 | stats/grid/magic-fill | ✅ Complete |
| `export_cmd.rs` | ~150 | export commands | ✅ Complete |
| `receipts_cmd.rs` | ~830 | receipt management | ✅ Complete |
| `settings_cmd.rs` | ~514 | settings/theme/etc | ✅ Complete |
| `integrations.rs` | ~200 | Home Assistant | ✅ Complete |

**Original file:** 3,285 lines → **Current mod.rs:** ~270 lines (91% reduction)

## Design Note: Module Layout Preference

The `commands/` subfolder structure (with `mod.rs` + submodules) is preferred over
flat file layout (like `calculations.rs`, `calculations_energy.rs`, etc.). This plan
uses the subfolder pattern as the reference style.

**Action:** Refactor existing `calculations*.rs` files into a `calculations/` subfolder
to match the `commands/` structure. See Phase 5 below.

## Phase 1: Setup and Low-Risk Extraction ✅ COMPLETE

### Step 1.1: Create module structure ✅
- [x] Create `src-tauri/src/commands/` directory
- [x] Create `mod.rs` with placeholder re-exports
- [x] Verify project compiles

### Step 1.2: Extract common.rs ⏭️ SKIPPED
Decided to keep helpers in mod.rs with `pub(crate)` visibility instead of separate common.rs.
- [x] Made `check_read_only!` macro available via `#[macro_export]`
- [x] Made `parse_trip_datetime()` pub(crate)
- [x] Made `get_app_data_dir()` pub(crate)
- [x] Made `get_db_paths()` pub(crate)

### Step 1.3: Extract vehicles.rs ✅
- [x] Move 6 vehicle commands
- [x] Update mod.rs exports
- [x] Run tests (229 passing)

### Step 1.4: Extract backup.rs ✅
- [x] Move 11 backup commands
- [x] Move helper functions (`parse_backup_filename`, `generate_backup_filename`, `get_cleanup_candidates`)
- [x] Add 8 unit tests
- [x] Update mod.rs exports
- [x] Run tests (229 passing)

## Phase 2: Complex Module Extraction 🟡 IN PROGRESS

### Step 2.1: Extract trips.rs ✅
- [x] Move 7 trip commands (get_trips, get_trips_for_year, get_years_with_trips, create_trip, update_trip, delete_trip, reorder_trip)
- [x] Move 2 route commands (get_routes, get_purposes)
- [x] Update mod.rs exports
- [x] Run tests (229 passing)

**Note:** Year-start helpers (`get_year_start_*`) remain in mod.rs for now - they're used by statistics and export commands.

### Step 2.2: Extract statistics.rs (largest module) ✅ COMPLETE
- [x] Move `get_trip_grid_data` and helpers
- [x] Move `calculate_trip_stats` and helpers
- [x] Move `calculate_magic_fill_liters`
- [x] Move `preview_trip_calculation`
- [x] Mark calculation helpers as `pub(crate)` for export.rs
- [x] Update imports from trips.rs (year-start helpers)
- [x] Update mod.rs exports
- [x] Run tests (235 passing)

### Step 2.3: Extract export_cmd.rs ✅ COMPLETE
- [x] Move `export_to_browser`
- [x] Move `export_html`
- [x] Update imports from statistics.rs
- [x] Update mod.rs exports
- [x] Run tests (235 passing)

## Phase 3: Integration Modules ✅ COMPLETE

### Step 3.1: Extract receipts_cmd.rs ✅ COMPLETE
- [x] Move 8 receipt commands
- [x] Move helper functions (SyncResult, ScanResult, ReceiptSettings, TripForAssignment)
- [x] Update mod.rs exports
- [x] Run tests (235 passing)

### Step 3.2: Extract settings_cmd.rs ✅ COMPLETE
- [x] Move theme commands (2)
- [x] Move auto-update commands (2)
- [x] Move date prefill commands (2)
- [x] Move hidden columns commands (2)
- [x] Move DB location commands (4)
- [x] Move settings CRUD (2)
- [x] Move window size command
- [x] Update mod.rs exports
- [x] Run tests (235 passing)

### Step 3.3: Extract integrations.rs ✅ COMPLETE
- [x] Move Home Assistant commands (5)
- [x] Move helper structs (HaSettingsResponse, HaLocalSettingsResponse)
- [x] Update mod.rs exports
- [x] Run tests (235 passing)

## Phase 4: Cleanup and Verification ✅ COMPLETE

### Step 4.1: Final cleanup ✅
- [x] Verify all commands moved
- [x] Remove any dead code from mod.rs
- [x] Clean up unused imports
- [x] Run full test suite: 235 tests passing

### Step 4.2: Integration test verification ⏳ PENDING (separate task)
- [ ] Run `npm run test:integration:tier1`
- [ ] Fix any IPC issues

### Step 4.3: Documentation update ⏳ PENDING
- [ ] Update CLAUDE.md Key Files section
- [ ] Add note about new module structure

## Testing Strategy

After each extraction step:
```bash
cd src-tauri && cargo test --lib
```

After Phase 4:
```bash
npm run test:all
```

## Commits Made

1. `refactor(commands): convert to module directory structure (ADR-011)` - vehicles.rs extracted
2. `refactor(commands): extract backup.rs and trips.rs modules (ADR-011)` - backup + trips extracted
3. `refactor(commands): extract statistics.rs module (ADR-011)` - statistics + export_cmd + receipts_cmd extracted
4. `refactor(commands): extract settings_cmd.rs and integrations.rs (ADR-011)` - final modules extracted

## Final Summary

The commands module refactoring is **complete**. The original 3,285-line `mod.rs` has been split into 8 feature-based submodules:

| Module | Purpose | Lines |
|--------|---------|-------|
| `backup.rs` | Backup/restore, CSV export | ~581 |
| `vehicles.rs` | Vehicle CRUD | ~147 |
| `trips.rs` | Trip CRUD, routes, purposes | ~255 |
| `statistics.rs` | Grid data, stats, magic fill | ~1442 |
| `export_cmd.rs` | HTML export, print | ~150 |
| `receipts_cmd.rs` | Receipt management, OCR | ~830 |
| `settings_cmd.rs` | Settings, theme, columns | ~514 |
| `integrations.rs` | Home Assistant integration | ~200 |
| `mod.rs` | Helpers, re-exports | ~270 |

All 235 unit tests pass with no warnings.

## Phase 5: Consolidate Calculations Module ✅ COMPLETE

Refactor flat `calculations*.rs` files into a `calculations/` subfolder to match `commands/` style.

### Step 5.1: Create calculations directory
- [x] Create `src-tauri/src/calculations/` directory
- [x] Create `mod.rs` with re-exports

### Step 5.2: Move existing files
- [x] Move `calculations.rs` → `calculations/mod.rs` (core logic)
- [x] Move `calculations_tests.rs` → `calculations/tests.rs`
- [x] Move `calculations_energy.rs` → `calculations/energy.rs`
- [x] Move `calculations_energy_tests.rs` → `calculations/energy_tests.rs`
- [x] Move `calculations_phev.rs` → `calculations/phev.rs`
- [x] Move `calculations_phev_tests.rs` → `calculations/phev_tests.rs`

### Step 5.3: Update imports
- [x] Update `lib.rs` to use `mod calculations;`
- [x] Update `commands/mod.rs` imports
- [x] Update internal imports in moved files
- [x] Run tests: 232 passing
