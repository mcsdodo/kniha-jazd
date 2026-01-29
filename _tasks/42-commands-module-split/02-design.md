# Design: Commands Module Split

## Current State
- Single file: `src-tauri/src/commands.rs` (3,908 lines)
- 68 Tauri commands
- 24 helper functions
- Well-organized with section comments

## Target Structure

```
src-tauri/src/commands/
├── mod.rs              # Re-exports all commands
├── common.rs           # Shared helpers, macros, types
├── vehicles.rs         # Vehicle CRUD (5 commands)
├── trips.rs            # Trip CRUD + routes (8 commands)
├── statistics.rs       # Grid data, calculations (3 commands)
├── backup.rs           # Backup/restore (11 commands)
├── export.rs           # HTML export (2 commands)
├── receipts.rs         # Receipt scanning (8 commands)
├── settings.rs         # Theme, columns, DB location (15 commands)
└── integrations.rs     # Home Assistant, Gemini (8 commands)
```

## Module Details

| Module | Lines | Commands | Key Contents |
|--------|-------|----------|--------------|
| `common.rs` | ~180 | 0 | `check_read_only!` macro, `parse_trip_datetime()`, `get_app_data_dir()`, types |
| `vehicles.rs` | ~130 | 5 | Vehicle CRUD operations |
| `trips.rs` | ~220 | 8 | Trip CRUD, routes, `get_year_start_*()` helpers |
| `statistics.rs` | ~1,170 | 3 | `get_trip_grid_data`, calculations, magic fill |
| `backup.rs` | ~400 | 11 | Backup creation, restore, cleanup |
| `export.rs` | ~280 | 2 | `export_to_browser`, `export_html` |
| `receipts.rs` | ~710 | 8 | Scanning, assignment, verification |
| `settings.rs` | ~310 | 15 | All preference get/set pairs |
| `integrations.rs` | ~180 | 8 | HA connection, Gemini API key |

## Dependencies

```
common.rs ← (used by all)

vehicles.rs → common
trips.rs → common, (year-start helpers exported)
statistics.rs → common, trips (year-start), calculations modules
backup.rs → common
export.rs → common, statistics (calculation helpers)
receipts.rs → common, gemini, receipts modules
settings.rs → common, db_location
integrations.rs → common
```

## Shared Utilities (in common.rs)
- `check_read_only!` macro
- `parse_trip_datetime()`
- `get_app_data_dir()`
- `get_db_paths()`
- Type definitions: `BackupInfo`, `CleanupPreview`, `CleanupResult`

## Public Helpers (statistics.rs → export.rs)
- `calculate_period_rates()` - pub(crate)
- `calculate_fuel_remaining()` - pub(crate)
- `calculate_fuel_consumed()` - pub(crate)

## Year-Start Helpers (trips.rs → statistics.rs, export.rs)
- `get_year_start_odometer()` - pub(crate)
- `get_year_start_fuel_remaining()` - pub(crate)
- `get_year_start_battery_remaining()` - pub(crate)
