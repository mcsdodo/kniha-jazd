**Date:** 2026-01-07
**Subject:** Implementation Plan - Split commands.rs and db.rs
**Status:** Planning

## Target Structure

### commands/ Module
```
src-tauri/src/commands/
├── mod.rs              # Re-exports all commands + shared types
├── vehicle.rs          # 6 commands (~125 lines)
├── trip.rs             # 7 commands (~200 lines)
├── route.rs            # 2 commands: get_routes, get_purposes (~20 lines)
├── calculation.rs      # 1 command (~25 lines)
├── settings.rs         # 2 commands (~35 lines)
├── grid.rs             # 2 commands + 10 helpers (~700 lines)
├── grid_tests.rs       # Tests (existing tests from commands.rs)
├── backup.rs           # 5 commands (~200 lines)
├── export.rs           # 2 commands (~250 lines)
├── receipts.rs         # 11 commands (~300 lines)
├── window.rs           # 1 command (~60 lines)
└── preview.rs          # 1 command (~160 lines)
```

### db/ Module
```
src-tauri/src/db/
├── mod.rs              # Database struct + re-exports (~100 lines)
├── migrations.rs       # Schema migrations (~230 lines)
├── vehicle.rs          # Vehicle CRUD (~170 lines)
├── trip.rs             # Trip CRUD + reorder (~280 lines)
├── route.rs            # Route CRUD + find_or_create (~210 lines)
├── settings.rs         # Settings CRUD (~75 lines)
├── receipt.rs          # Receipt CRUD (~500 lines)
└── db_tests.rs         # All db tests (~650 lines)
```

## Implementation Phases

### Phase 1: Preparation
- [ ] Run `cargo test` - establish baseline (108 tests pass)
- [ ] Run `cargo build` - verify clean compile
- [ ] Create git branch: `git checkout -b refactor/split-large-files`

### Phase 2: Split db.rs (Lower Risk - No Dependents)

**Order matters**: commands.rs depends on db, so split db first.

1. [ ] Create `src-tauri/src/db/` directory
2. [ ] Create `db/mod.rs` - Database struct, connection, re-exports
3. [ ] Create `db/migrations.rs` - move migration logic (~230 lines)
4. [ ] Create `db/vehicle.rs` - move 6 vehicle methods (~170 lines)
5. [ ] Create `db/trip.rs` - move 9 trip methods (~280 lines)
6. [ ] Create `db/route.rs` - move 7 route methods (~210 lines)
7. [ ] Create `db/settings.rs` - move 2 settings methods (~75 lines)
8. [ ] Create `db/receipt.rs` - move 11 receipt methods + helpers (~500 lines)
9. [ ] Create `db/db_tests.rs` - move all tests (~650 lines)
10. [ ] Delete `src-tauri/src/db.rs`
11. [ ] **Verify:** `cargo test` passes, `cargo build` succeeds

### Phase 3: Split commands.rs

**Dependency order**: Create modules in order that satisfies internal dependencies.

1. [ ] Create `src-tauri/src/commands/` directory
2. [ ] Create `commands/mod.rs` with all `pub use` re-exports
3. [ ] Create submodules (in this order for dependencies):
   - [ ] `backup.rs` - standalone, no internal deps
   - [ ] `window.rs` - standalone, no internal deps
   - [ ] `vehicle.rs` - standalone, no internal deps
   - [ ] `route.rs` - standalone, no internal deps
   - [ ] `calculation.rs` - standalone, no internal deps
   - [ ] `settings.rs` - standalone, no internal deps
   - [ ] `trip.rs` - standalone, no internal deps
   - [ ] `grid.rs` - exports `pub(crate)` helpers (MUST come before export/preview)
   - [ ] `grid_tests.rs` - tests using `#[path]` pattern
   - [ ] `export.rs` - uses `super::grid::` helpers
   - [ ] `preview.rs` - uses `super::grid::` helpers
   - [ ] `receipts.rs` - standalone, no internal deps
4. [ ] Delete `src-tauri/src/commands.rs`
5. [ ] **Verify:** `cargo test` passes, `cargo build` succeeds

### Phase 4: Final Verification
- [ ] `cargo test` - all 108 tests pass
- [ ] `cargo build --release` - clean production build
- [ ] `cargo fmt` - format new files
- [ ] `cargo clippy` - no warnings
- [ ] `npm run tauri dev` - manual smoke test

### Phase 5: Commit
- [ ] Commit: `refactor(backend): split commands.rs and db.rs into modules`

---

## Key Implementation Details

### mod.rs Re-export Pattern (API Preservation)

```rust
// commands/mod.rs - maintains same public interface
mod vehicle;
mod trip;
// ... other modules

pub use vehicle::{get_vehicles, create_vehicle, update_vehicle, ...};
pub use trip::{get_trips, create_trip, ...};

// Re-export types used by frontend
pub use backup::BackupInfo;
pub use receipts::{ProcessingProgress, ReceiptSettings, ScanResult, SyncResult};
```

### Struct Definitions to Move

These types are defined in `commands.rs` and must move to their respective modules:

| Struct | Source Location | Target Module |
|--------|-----------------|---------------|
| `BackupInfo` | lines 26-34 | `backup.rs` |
| `ReceiptSettings` | lines 1657-1663 | `receipts.rs` |
| `SyncResult` | lines 1695-1699 | `receipts.rs` |
| `SyncError` | lines 1701-1705 | `receipts.rs` |
| `ScanResult` | lines 1708-1712 | `receipts.rs` |
| `ProcessingProgress` | lines 1774-1779 | `receipts.rs` |
| `WindowSize` | lines 1976-1980 | `window.rs` |
| `PhevGridData` | lines 981-997 | `grid.rs` (internal) |

### Internal Helper Visibility

```rust
// commands/grid.rs - helpers used internally and by export.rs/preview.rs
// Public to crate (used by export.rs/preview.rs):
pub(crate) fn calculate_period_rates(...) { ... }
pub(crate) fn calculate_fuel_remaining(...) { ... }
pub(crate) fn get_year_start_fuel_remaining(...) { ... }

// Private to grid.rs (only used by get_trip_grid_data):
fn calculate_energy_grid_data(...) { ... }      // ~90 lines
fn calculate_phev_grid_data(...) { ... }        // ~135 lines
fn calculate_date_warnings(...) { ... }         // ~35 lines
fn calculate_consumption_warnings(...) { ... }  // ~20 lines
fn calculate_missing_receipts(...) { ... }      // ~25 lines

// commands/export.rs - importing helpers from sibling module
use super::grid::{calculate_period_rates, calculate_fuel_remaining, ...};
```

### Test File Pattern (Following Project Convention)

```rust
// commands/grid.rs
#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;

// db/mod.rs - test module for all db tests
#[cfg(test)]
#[path = "db_tests.rs"]
mod tests;
```

### Database impl Split Pattern

```rust
// db/vehicle.rs
use super::Database;

impl Database {
    pub fn get_vehicle(&self, id: &str) -> Result<Option<Vehicle>> { ... }
    pub fn create_vehicle(&self, vehicle: &Vehicle) -> Result<()> { ... }
    // ... other vehicle methods
}
```

---

## Dependency Analysis

### commands/ Internal Dependencies

```
commands/mod.rs
    ├── backup.rs         (standalone)
    ├── window.rs         (standalone)
    ├── vehicle.rs        (standalone)
    ├── route.rs          (standalone)
    ├── calculation.rs    (standalone)
    ├── settings.rs       (standalone)
    ├── trip.rs           (standalone)
    ├── grid.rs           (standalone, exports helpers)
    │   ├── calculate_period_rates()      (pub(crate))
    │   ├── calculate_fuel_remaining()    (pub(crate))
    │   └── get_year_start_fuel_remaining() (pub(crate))
    ├── export.rs         (uses super::grid::*)
    ├── preview.rs        (uses super::grid::*)
    └── receipts.rs       (standalone)
```

### External Dependencies (from crate::)

All command submodules use:
- `crate::db::Database` - database access
- `crate::models::*` - various model structs
- `crate::calculations::*` - calculation functions
- `crate::suggestions::*` - suggestion building

---

## Files Changed

| Before | After |
|--------|-------|
| `src-tauri/src/commands.rs` (3,044 lines) | DELETED |
| `src-tauri/src/db.rs` (1,806 lines) | DELETED |
| — | `src-tauri/src/commands/` (11 files) |
| — | `src-tauri/src/db/` (8 files) |
| `src-tauri/src/lib.rs` | NO CHANGES (Rust finds mod.rs automatically) |

### Re-export Verification

After creating `commands/mod.rs`, verify all 40 commands are re-exported. Check against `lib.rs` invoke_handler:

```rust
// All these must be re-exported from commands/mod.rs:
// Vehicle (6): get_vehicles, create_vehicle, update_vehicle, delete_vehicle, get_vehicle_statistics, get_years_with_trips
// Trip (7): get_trips, create_trip, update_trip, delete_trip, reorder_trips, search_similar_trips, get_trip_years
// Route (2): get_routes, get_purposes
// Calculation (1): calculate_trip
// Settings (2): get_settings, save_settings
// Grid (2): get_trip_grid_data, calculate_trip_stats
// Backup (5): get_backups, create_backup, restore_backup, delete_backup, get_backup_settings
// Export (2): export_to_browser, export_html
// Receipts (11): get_receipt_settings, get_receipts, get_unassigned_receipts, scan_receipts,
//                sync_receipts, process_pending_receipts, update_receipt, delete_receipt,
//                reprocess_receipt, assign_receipt_to_trip, verify_receipts
// Window (1): set_window_size
// Preview (1): get_receipt_preview
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Circular dependencies | `pub(crate)` helpers + `super::module::` imports |
| Missing re-exports | Verify all public items in mod.rs |
| Test breakage | Move tests to `*_tests.rs` using `#[path]` pattern |
| Build errors | Incremental: split db first, verify, then commands |

## Verification Checklist (After Each Phase)

- [ ] `cargo build` compiles without errors
- [ ] `cargo test` passes (no regression)
- [ ] No warnings from `cargo clippy`
- [ ] Files formatted with `cargo fmt`
