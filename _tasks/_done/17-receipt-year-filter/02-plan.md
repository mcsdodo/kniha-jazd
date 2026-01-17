# Implementation Plan: Receipt Year Filtering

## Phase 1: Backend - Data Model & Migration

### 1.1 Add source_year field to Receipt model
- [ ] Update `src-tauri/src/models.rs`: Add `source_year: Option<i32>` to `Receipt` struct
- [ ] Update `Receipt::new()` to accept optional source_year parameter

### 1.2 Database migration
- [ ] Create migration: `ALTER TABLE receipts ADD COLUMN source_year INTEGER`
- [ ] Update `src-tauri/src/db.rs`:
  - [ ] `create_receipt()` - include source_year
  - [ ] `get_receipt_*()` functions - read source_year
  - [ ] `update_receipt()` - include source_year

### 1.3 Tests for model changes
- [ ] Test Receipt creation with/without source_year
- [ ] Test DB round-trip with source_year

---

## Phase 2: Backend - Folder Structure Detection

### 2.1 Create folder detection module
- [ ] Add `FolderStructure` enum: `Flat`, `YearBased(Vec<i32>)`, `Invalid(String)`
- [ ] Add `detect_folder_structure(path: &str) -> FolderStructure` function
- [ ] Implement detection logic:
  - List entries, categorize as files/year-folders/other-folders
  - Return appropriate variant

### 2.2 Update scan_folder_for_new_receipts
- [ ] Call `detect_folder_structure()` first
- [ ] Match on result:
  - `Flat` → current behavior, source_year = None
  - `YearBased(years)` → scan inside each year folder, source_year = folder year
  - `Invalid(reason)` → return error with explanation

### 2.3 Tests for folder detection
- [ ] Test flat structure detection
- [ ] Test year-based structure detection
- [ ] Test mixed structure (should return Invalid)
- [ ] Test non-year folder names (should return Invalid)
- [ ] Test empty folder
- [ ] Test scanning with year folders populates source_year

---

## Phase 3: Backend - Year Filtering API

### 3.1 Update get_receipts command
- [ ] Change signature: `get_receipts(year: Option<i32>)`
- [ ] Implement filtering logic:
  ```rust
  // If year filter provided:
  // - Include if receipt_date.year() == year
  // - Include if receipt_date is None AND source_year == year
  // - Include if both are None (flat mode, unprocessed)
  ```

### 3.2 Add DB query for filtered receipts
- [ ] `get_receipts_for_year(year: i32)` in db.rs
- [ ] SQL with proper NULL handling

### 3.3 Tests for year filtering
- [ ] Test filtering by receipt_date year
- [ ] Test fallback to source_year when receipt_date is None
- [ ] Test receipts with neither show in all years

---

## Phase 4: Frontend - API & Display

### 4.1 Update API calls
- [ ] Update `src/lib/api.ts`: `getReceipts(year?: number)`
- [ ] Update Doklady page to pass `selectedYearStore` to API

### 4.2 Handle folder structure errors
- [ ] Display warning message when sync returns Invalid structure
- [ ] Add i18n translations for warning messages (SK + EN)

### 4.3 Date mismatch indicator
- [ ] Add logic to detect `source_year ≠ receipt_date.year()`
- [ ] Add warning icon/badge to receipt card
- [ ] Add tooltip with explanation
- [ ] Add i18n translations

---

## Phase 5: Documentation

### 5.1 DECISIONS.md
- [ ] Add ADR entry explaining:
  - Year filtering logic (OCR primary, folder fallback)
  - Mismatch warning rationale
  - Folder structure requirements

### 5.2 README updates
- [ ] Update README.md (Slovak): Folder structure options for receipts
- [ ] Update README.en.md (English): Same content

---

## Phase 6: Final

### 6.1 Integration testing
- [ ] E2E test with flat folder structure
- [ ] E2E test with year-based folder structure
- [ ] Verify year filter works correctly

### 6.2 Cleanup
- [ ] Run `cargo test` - all pass
- [ ] Run `npm run check` - no errors
- [ ] Update CHANGELOG.md

---

## File Changes Summary

| File | Changes |
|------|---------|
| `src-tauri/src/models.rs` | Add `source_year` field |
| `src-tauri/src/db.rs` | Migration, CRUD updates, year filter query |
| `src-tauri/src/receipts.rs` | Folder detection, scan updates |
| `src-tauri/src/commands.rs` | Update `get_receipts` signature |
| `src/lib/api.ts` | Update `getReceipts()` |
| `src/routes/doklady/+page.svelte` | Year filtering, warnings, mismatch indicator |
| `src/lib/i18n/sk/index.ts` | New translations |
| `src/lib/i18n/en/index.ts` | New translations |
| `DECISIONS.md` | New ADR entry |
| `README.md` | Folder structure docs |
| `README.en.md` | Folder structure docs |
| `CHANGELOG.md` | Feature entry |
