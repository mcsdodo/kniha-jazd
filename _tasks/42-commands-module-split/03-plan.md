# Implementation Plan: Commands Module Split

**Status:** 🟡 In Progress (Phase 1 complete, Phase 2 started)
**Last Updated:** 2026-01-29

## Current State

| Module | Lines | Commands | Status |
|--------|-------|----------|--------|
| `mod.rs` | 3105 | remaining | 🟡 In progress |
| `vehicles.rs` | 147 | 6 | ✅ Complete |
| `backup.rs` | 581 | 11 + 8 tests | ✅ Complete |
| `trips.rs` | 255 | 9 | ✅ Complete |

**Original file:** 3,908 lines → **Current mod.rs:** 3,105 lines

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

### Step 2.2: Extract statistics.rs (largest module) ⏳ PENDING
- [ ] Move `get_trip_grid_data` and helpers
- [ ] Move `calculate_trip_stats` and helpers
- [ ] Move `calculate_magic_fill_liters`
- [ ] Move `preview_trip_calculation`
- [ ] Mark calculation helpers as `pub(crate)` for export.rs
- [ ] Update imports from trips.rs (year-start helpers)
- [ ] Update mod.rs exports
- [ ] Run tests

### Step 2.3: Extract export.rs ⏳ PENDING
- [ ] Move `export_to_browser`
- [ ] Move `export_html`
- [ ] Update imports from statistics.rs
- [ ] Update mod.rs exports
- [ ] Run tests

## Phase 3: Integration Modules ⏳ PENDING

### Step 3.1: Extract receipts.rs
- [ ] Move 8 receipt commands
- [ ] Move helper functions
- [ ] Update mod.rs exports
- [ ] Run tests

### Step 3.2: Extract settings.rs
- [ ] Move theme commands (2)
- [ ] Move auto-update commands (2)
- [ ] Move date prefill commands (2)
- [ ] Move hidden columns commands (2)
- [ ] Move DB location commands (4)
- [ ] Move settings CRUD (2)
- [ ] Move window size command
- [ ] Update mod.rs exports
- [ ] Run tests

### Step 3.3: Extract integrations.rs
- [ ] Move Gemini API key command
- [ ] Move receipts folder command
- [ ] Move Home Assistant commands (6)
- [ ] Update mod.rs exports
- [ ] Run tests

## Phase 4: Cleanup and Verification ⏳ PENDING

### Step 4.1: Final cleanup
- [ ] Verify all commands moved
- [ ] Remove any dead code from mod.rs
- [ ] Run full test suite: `npm run test:backend`

### Step 4.2: Integration test verification
- [ ] Run `npm run test:integration:tier1`
- [ ] Fix any IPC issues

### Step 4.3: Documentation update
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

## Rollback Plan

If issues arise:
1. Git stash current changes
2. Revert to single commands.rs
3. Investigate issue with smaller extraction

## Next Steps (for next session)

1. Extract statistics.rs (largest remaining section, ~1170 lines)
2. Extract export.rs (~280 lines, depends on statistics helpers)
3. Continue with Phase 3 modules
