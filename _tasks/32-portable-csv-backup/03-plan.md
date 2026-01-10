**Date:** 2026-01-10
**Subject:** Portable CSV Backup - Implementation Plan
**Status:** Planning

# Implementation Plan: Portable CSV Backup

## Phase 1: Backend - Export

### 1.1 Add dependencies
- Add `csv = "1.3"` to Cargo.toml
- Add `zip = { version = "2.2", default-features = false, features = ["deflate"] }` to Cargo.toml
- Run `cargo build` to verify

### 1.2 Create portable_backup.rs module
- New file: `src-tauri/src/portable_backup.rs`
- Add module to `lib.rs`
- Define structs:
  - `ExportMetadata` (version, appVersion, exportedAt, tables)
  - `ImportResult` (success counts)
  - `ImportError` (file, row, field, message)

### 1.3 Implement CSV export helpers
- `write_csv_with_bom<T: Serialize>(records: &[T]) -> Vec<u8>`
- UTF-8 BOM prefix: `[0xEF, 0xBB, 0xBF]`
- Use `csv::Writer` with serde serialization

### 1.4 Implement export_portable_backup command
- Fetch all data: vehicles, trips, routes, settings, receipts
- Generate metadata.json
- Write each table to CSV
- Create ZIP archive in memory
- Return `Vec<u8>`

### 1.5 Add missing db.rs functions
- `get_all_trips()` — unfiltered (currently only filtered by vehicle/year)
- `get_all_routes()` — if not exists
- `get_all_receipts()` — if not exists

### 1.6 Register command in main.rs
- Add `export_portable_backup` to `invoke_handler`

### 1.7 Write tests for export
- Test: export empty database
- Test: export with sample data
- Test: verify CSV format (BOM, headers, encoding)
- Test: verify ZIP structure

## Phase 2: Backend - Import

### 2.1 Implement ZIP parsing
- `parse_zip(bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, ImportError>`
- Extract all files to memory

### 2.2 Implement validation
- Validate required files present
- Parse and validate metadata.json
- Validate version compatibility

### 2.3 Implement CSV parsing
- `parse_csv<T: DeserializeOwned>(bytes: &[u8]) -> Result<Vec<T>, ImportError>`
- Strip BOM if present
- Detailed row-level error reporting

### 2.4 Implement FK validation
- After parsing all CSVs, before inserting:
- Check all trip.vehicle_id references exist in vehicles
- Check all receipt.vehicle_id and receipt.trip_id references exist

### 2.5 Implement import_portable_backup command
- Parse ZIP
- Validate all CSVs
- Create auto-backup (reuse existing backup logic)
- Clear tables in FK order
- Insert in FK order
- Return ImportResult

### 2.6 Add db.rs clear functions
- `clear_all_receipts()`
- `clear_all_trips()`
- `clear_all_routes()`
- `clear_all_vehicles()`
- `clear_settings()`

### 2.7 Add db.rs bulk insert functions
- `insert_vehicles_bulk(vehicles: Vec<Vehicle>)`
- `insert_trips_bulk(trips: Vec<Trip>)`
- `insert_routes_bulk(routes: Vec<Route>)`
- `insert_receipts_bulk(receipts: Vec<Receipt>)`

### 2.8 Register command in main.rs
- Add `import_portable_backup` to `invoke_handler`

### 2.9 Write tests for import
- Test: import valid ZIP
- Test: import with missing required file
- Test: import with invalid version
- Test: import with invalid CSV data
- Test: import with FK violation
- Test: round-trip (export → import → verify data matches)

## Phase 3: Frontend - UI

### 3.1 Add i18n keys
- Slovak and English translations for:
  - Section title, button labels
  - Confirmation dialog
  - Success/error toasts
  - Info text

### 3.2 Create PortableBackup component
- Export button → calls `export_portable_backup` → save dialog
- Import button → file picker → calls `import_portable_backup`
- Confirmation dialog before import
- Toast notifications for success/error

### 3.3 Add to Settings page
- Import PortableBackup component
- Place in new section below existing backup UI

### 3.4 Handle import errors
- Display list of validation errors
- Show file, row, field context

## Phase 4: Finalization

### 4.1 Manual testing
- Export from app with real data
- Open CSVs in Excel — verify Slovak characters
- Import into fresh app
- Verify all data restored correctly

### 4.2 Update CHANGELOG.md
- Add entry for new feature

### 4.3 Update README
- Document new backup feature

## Estimated Effort

| Phase | Effort |
|-------|--------|
| Phase 1: Export | ~2-3 hours |
| Phase 2: Import | ~3-4 hours |
| Phase 3: Frontend | ~1-2 hours |
| Phase 4: Finalization | ~1 hour |
| **Total** | ~7-10 hours |

## Dependencies Graph

```
Phase 1.1 (deps) ─┬─→ Phase 1.2-1.7 (export)
                  └─→ Phase 2.1-2.9 (import)
                           │
                           ▼
                    Phase 3.1-3.4 (frontend)
                           │
                           ▼
                    Phase 4 (finalize)
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Large file memory issues | Stream CSV parsing, test with 5000+ rows |
| Excel encoding issues | UTF-8 BOM, test with Slovak chars |
| FK constraint failures on import | Validate all references before any inserts |
| Import corrupts data | Auto-backup before import |
