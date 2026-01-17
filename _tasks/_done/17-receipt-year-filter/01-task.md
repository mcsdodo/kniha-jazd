# Receipt Year Filtering

## Problem

Currently, `scan_folder_for_new_receipts` only reads top-level files and skips subdirectories. The year filter in the header only affects trips, not receipts. Users want to organize receipts in year-based folders and have them filter accordingly.

## Requirements

1. Support two folder structures:
   - **Flat**: Only files at root (current behavior)
   - **Year-based**: Only `YYYY/` folders at root (e.g., `2024/`, `2025/`)

2. Invalid structures show warning, don't load:
   - Mixed (files + folders at root)
   - Non-year folder names (e.g., `January/`, `misc/`)

3. Year filtering logic:
   - **Primary**: Use `receipt_date.year()` from OCR
   - **Fallback**: Use `source_year` from folder (for unprocessed receipts)
   - Receipts with no date and no source_year show in all years

4. Mismatch handling:
   - If `source_year ≠ receipt_date.year()`, show warning icon
   - Trust OCR date for filtering, but alert user of discrepancy

## Design

### Folder Structure Detection

```
1. List all entries in receipts_folder_path
2. Categorize each entry:
   - File → mark as "has_files"
   - Directory matching /^\d{4}$/ → mark as "has_year_folders", collect years
   - Directory not matching → mark as "has_other_folders"

3. Determine mode:
   - Only files → FLAT mode
   - Only year folders → YEAR mode
   - Mixed or other folders → INVALID
```

| Folder contents | Mode | Result |
|-----------------|------|--------|
| `a.jpg, b.jpg` | FLAT | Scan files, source_year = None |
| `2024/, 2025/` | YEAR | Scan inside each, source_year = folder |
| `a.jpg, 2024/` | INVALID | Warning, don't scan |
| `January/, misc/` | INVALID | Warning, don't scan |

### Data Model

```rust
pub struct Receipt {
    // existing fields...
    pub source_year: Option<i32>,  // NEW: Year from folder structure
}
```

- `None` = Flat mode receipt (no year filtering from folder)
- `Some(2024)` = From `2024/` folder

### Year Filtering Logic

```
display_year = receipt_date.year() ?? source_year ?? None

If None → show in all years
If Some(year) → filter by selected year
```

### Mismatch Warning

If both `source_year` and `receipt_date` exist and years differ:
- Show warning icon on receipt card
- Tooltip: "Dátum dokladu (2024) nezodpovedá priečinku (2025)"

### Frontend Changes

1. Pass `selectedYearStore` to `get_receipts` API
2. Show folder structure warning when invalid:
   ```
   Neplatná štruktúra priečinka

   Priečinok musí obsahovať buď:
   • Len súbory (bez podpriečinkov)
   • Len priečinky s názvami rokov (2024, 2025, ...)
   ```
3. Date mismatch indicator on receipt cards
4. Unprocessed receipts filter by `source_year`

## Documentation Required

- `DECISIONS.md`: ADR for year filtering logic
- `README.md` / `README.en.md`: User guide for folder structure options
