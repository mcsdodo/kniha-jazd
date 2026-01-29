# Implementation Plan: Commands Module Split

## Phase 1: Setup and Low-Risk Extraction

### Step 1.1: Create module structure
- [ ] Create `src-tauri/src/commands/` directory
- [ ] Create `mod.rs` with placeholder re-exports
- [ ] Verify project compiles

### Step 1.2: Extract common.rs
- [ ] Move `check_read_only!` macro
- [ ] Move `parse_trip_datetime()`
- [ ] Move `get_app_data_dir()`
- [ ] Move `get_db_paths()`
- [ ] Move shared type definitions
- [ ] Update imports in commands.rs
- [ ] Run tests: `cargo test --lib`

### Step 1.3: Extract vehicles.rs
- [ ] Move 5 vehicle commands
- [ ] Update mod.rs exports
- [ ] Update lib.rs invoke_handler
- [ ] Run tests

### Step 1.4: Extract backup.rs
- [ ] Move 11 backup commands
- [ ] Move helper functions (`parse_backup_filename`, `generate_backup_filename`, `get_cleanup_candidates`)
- [ ] Update mod.rs exports
- [ ] Update lib.rs invoke_handler
- [ ] Run tests

## Phase 2: Complex Module Extraction

### Step 2.1: Extract trips.rs
- [ ] Move 8 trip commands
- [ ] Move route/purpose commands
- [ ] Move year-start helpers (mark as `pub(crate)`)
- [ ] Update mod.rs exports
- [ ] Update lib.rs invoke_handler
- [ ] Run tests

### Step 2.2: Extract statistics.rs (largest module)
- [ ] Move `get_trip_grid_data` and helpers
- [ ] Move `calculate_trip_stats` and helpers
- [ ] Move `calculate_magic_fill_liters`
- [ ] Move `preview_trip_calculation`
- [ ] Mark calculation helpers as `pub(crate)` for export.rs
- [ ] Update imports from trips.rs (year-start helpers)
- [ ] Update mod.rs exports
- [ ] Update lib.rs invoke_handler
- [ ] Run tests

### Step 2.3: Extract export.rs
- [ ] Move `export_to_browser`
- [ ] Move `export_html`
- [ ] Update imports from statistics.rs
- [ ] Update imports from trips.rs
- [ ] Update mod.rs exports
- [ ] Update lib.rs invoke_handler
- [ ] Run tests

## Phase 3: Integration Modules

### Step 3.1: Extract receipts.rs
- [ ] Move 8 receipt commands
- [ ] Move helper functions
- [ ] Update mod.rs exports
- [ ] Update lib.rs invoke_handler
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
- [ ] Update lib.rs invoke_handler
- [ ] Run tests

### Step 3.3: Extract integrations.rs
- [ ] Move Gemini API key command
- [ ] Move receipts folder command
- [ ] Move Home Assistant commands (6)
- [ ] Update mod.rs exports
- [ ] Update lib.rs invoke_handler
- [ ] Run tests

## Phase 4: Cleanup and Verification

### Step 4.1: Remove old commands.rs
- [ ] Verify all commands moved
- [ ] Delete original commands.rs
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

## Rollback Plan

If issues arise:
1. Git stash current changes
2. Revert to single commands.rs
3. Investigate issue with smaller extraction

## Estimated Effort

| Phase | Steps | Est. Time |
|-------|-------|-----------|
| Phase 1 | 4 | 1-2 hours |
| Phase 2 | 3 | 2-3 hours |
| Phase 3 | 3 | 1-2 hours |
| Phase 4 | 3 | 30 min |
| **Total** | **13** | **5-8 hours** |
