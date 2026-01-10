**Date:** 2026-01-10
**Subject:** Portable CSV Backup - Design
**Status:** Planning

# Design: Portable CSV Backup

## Export Format

```
kniha-jazd-export-YYYY-MM-DD.zip
├── metadata.json      # Version info, export timestamp, row counts
├── settings.csv       # Company info (1 row)
├── vehicles.csv       # All vehicles
├── trips.csv          # All trips
├── routes.csv         # Saved routes (autocomplete)
└── receipts.csv       # Receipt metadata (file paths won't be portable)
```

### metadata.json

```json
{
  "version": "1.0",
  "appVersion": "0.12.0",
  "exportedAt": "2026-01-10T14:30:00Z",
  "tables": {
    "vehicles": 2,
    "trips": 156,
    "routes": 23,
    "settings": 1,
    "receipts": 45
  }
}
```

### CSV Format Rules

| Rule | Value | Reason |
|------|-------|--------|
| Encoding | UTF-8 with BOM | Excel handles Slovak characters (ľščťžýáíé) |
| Date format | ISO 8601 (YYYY-MM-DD) | Unambiguous parsing |
| Booleans | 0/1 | Matches SQLite, Excel-friendly |
| NULLs | Empty cell | Standard CSV convention |
| Header row | Yes | Human-readable column names |
| Delimiter | Comma | Standard CSV |
| Quoting | Double-quote when needed | Standard CSV escaping |

### CSV Column Order

Matches database schema for clarity:

**vehicles.csv:**
```
id,name,license_plate,vehicle_type,tank_size_liters,tp_consumption,battery_capacity_kwh,baseline_consumption_kwh,initial_battery_percent,initial_odometer,is_active,vin,driver_name,created_at,updated_at
```

**trips.csv:**
```
id,vehicle_id,date,origin,destination,distance_km,odometer,purpose,fuel_liters,fuel_cost_eur,other_costs_eur,other_costs_note,full_tank,sort_order,energy_kwh,energy_cost_eur,full_charge,soc_override_percent,created_at,updated_at
```

**routes.csv:**
```
id,vehicle_id,origin,destination,distance_km,usage_count,last_used
```

**settings.csv:**
```
id,company_name,company_ico,buffer_trip_purpose,updated_at
```

**receipts.csv:**
```
id,vehicle_id,trip_id,file_path,file_name,scanned_at,liters,total_price_eur,receipt_date,station_name,station_address,source_year,status,confidence,raw_ocr_text,error_message,created_at,updated_at
```

## Export Flow

1. User clicks "Exportovať ZIP" in Settings
2. Backend generates ZIP in memory
3. Frontend opens save dialog
4. User chooses location
5. File saved, success toast

```rust
#[tauri::command]
pub fn export_portable_backup(db: State<Database>) -> Result<Vec<u8>, String> {
    // 1. Fetch all data from all tables
    // 2. Build CSVs in memory (with UTF-8 BOM)
    // 3. Create ZIP archive
    // 4. Return bytes (frontend saves via dialog)
}
```

## Import Flow

1. User clicks "Importovať ZIP" in Settings
2. File picker opens → user selects .zip
3. Backend validates ZIP structure
4. Confirmation dialog: "Nahradí všetky aktuálne dáta. Pokračovať?"
5. Auto-backup current DB
6. Clear tables in FK-safe order
7. Insert in FK-safe order
8. Success/failure toast

```rust
#[tauri::command]
pub fn import_portable_backup(
    app: tauri::AppHandle,
    db: State<Database>,
    zip_bytes: Vec<u8>
) -> Result<ImportResult, String> {
    // 1. Parse ZIP, extract CSVs
    // 2. Validate structure and version
    // 3. Parse all CSVs into memory (fail early)
    // 4. Create auto-backup
    // 5. Clear: receipts → trips → routes → vehicles → settings
    // 6. Insert: settings → vehicles → routes → trips → receipts
    // 7. Return counts
}
```

### Table Order (FK-safe)

**Clear order:** receipts → trips → routes → vehicles → settings
**Insert order:** settings → vehicles → routes → trips → receipts

## Validation & Errors

### Validation Rules

- Required files: `metadata.json`, `vehicles.csv`, `trips.csv`, `settings.csv`
- Optional files: `routes.csv`, `receipts.csv`
- Version check: `metadata.version` must be `"1.0"`
- Row validation: UUIDs valid, dates parseable, FK references exist

### Error Structure

```rust
#[derive(Serialize)]
pub struct ImportError {
    pub file: String,           // "trips.csv"
    pub row: Option<u32>,       // 12 (None for file-level errors)
    pub field: Option<String>,  // "date"
    pub message: String,        // Localized message
}
```

### Error Messages (Slovak)

| Error | Message |
|-------|---------|
| Not a ZIP | "Súbor nie je platný ZIP archív" |
| Missing file | "Chýba {file} — neplatný export" |
| Incompatible version | "Verzia exportu {v} nie je podporovaná. Požadovaná: 1.0" |
| CSV parse error | "Chyba v {file}, riadok {line}: {detail}" |
| Invalid UUID | "{file}, riadok {row}: Neplatné ID '{value}'" |
| Invalid date | "{file}, riadok {row}: Neplatný dátum '{value}'" |
| Missing FK | "{file}, riadok {row}: Vozidlo '{id}' neexistuje" |

## Edge Cases

| Case | Handling |
|------|----------|
| Receipts with file_path | Import metadata only, warn user about missing files |
| Empty database | Valid export with 0 rows |
| Large exports (1000+ trips) | Stream CSV parsing |
| App version mismatch | Warn if older, but allow import |

## UI Placement

Settings page — new section "Prenosná záloha (CSV)":

```
┌─────────────────────────────────────────────────────┐
│ Prenosná záloha (CSV)                               │
│ [Exportovať ZIP]  [Importovať ZIP]                  │
│                                                     │
│ ⓘ Prenosná záloha je čitateľná v Exceli            │
│   a umožňuje prenos dát do iného systému.          │
└─────────────────────────────────────────────────────┘
```

## Dependencies

```toml
# Add to Cargo.toml
csv = "1.3"
zip = { version = "2.2", default-features = false, features = ["deflate"] }
```
