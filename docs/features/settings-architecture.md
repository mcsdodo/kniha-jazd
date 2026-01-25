# Feature: Settings Architecture

> Dual storage system separating machine-specific configuration (LocalSettings) from business data (Settings) for safe database sharing across devices.

## Overview

The application uses a **two-tier settings architecture** that separates:

1. **LocalSettings** — Machine-specific configuration stored in a local JSON file
2. **Settings** — Business/company data stored in the SQLite database

This architecture enables users to share their database across multiple computers (via Google Drive, NAS, etc.) while keeping sensitive credentials and machine-specific paths local to each device.

## The Two Storage Systems

### LocalSettings (File-based)

Stored in `local.settings.json` in the app data directory:
- **Windows**: `%APPDATA%/com.notavailable.kniha-jazd/local.settings.json`
- **Dev mode**: `%APPDATA%/com.notavailable.kniha-jazd.dev/local.settings.json`

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `gemini_api_key` | `Option<String>` | API key for receipt OCR scanning |
| `receipts_folder_path` | `Option<String>` | Local folder path for receipt images |
| `theme` | `Option<String>` | UI theme: `"system"`, `"light"`, or `"dark"` |
| `auto_check_updates` | `Option<bool>` | Enable automatic update checks (default: `true`) |
| `custom_db_path` | `Option<String>` | Custom database location (Google Drive, NAS) |
| `backup_retention` | `Option<BackupRetention>` | Auto-cleanup settings for pre-update backups |

**ReceiptSettings return shape:** See `types.ts:L177-182` for the TypeScript interface.

**Notes**:
- `KNIHA_JAZD_DATA_DIR` can override the app data directory for local settings and database paths.
- Theme and auto-update preferences currently use the default app data dir (do not honor `KNIHA_JAZD_DATA_DIR`).
- Setting `gemini_api_key` or `receipts_folder_path` to an empty string clears the value.
- `receipts_folder_path` must exist and be a directory.

**BackupRetention:** See `settings.rs:L11-14` for the struct definition. Contains `enabled` (bool) and `keep_count` (u32) fields, serialized with camelCase for JSON.

### Settings (Database)

Stored in the `settings` table of `kniha-jazd.db`:

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `company_name` | `String` | Company name for PDF exports |
| `company_ico` | `String` | Company identification number (IČO) |
| `buffer_trip_purpose` | `String` | Default purpose text for buffer trips |
| `updated_at` | `DateTime<Utc>` | Last modification timestamp |

**Defaults:** See `models.rs:L275-285` for the `Default` implementation. The `buffer_trip_purpose` defaults to "sluzobna cesta" (service trip); other string fields default to empty.

## Why the Split?

The separation exists for **three key reasons**:

### 1. API Keys Don't Travel

API keys (like Gemini) are personal credentials that shouldn't be shared when syncing the database across computers. Each user/machine needs their own key.

### 2. Paths Are Machine-Specific

File paths (like receipts folder) differ between computers:
- Home PC: `C:\Users\John\Documents\Receipts`
- Work PC: `D:\Data\Receipts`
- Mac: `/Users/john/Documents/Receipts`

### 3. Preferences Stay Local

Theme preferences and update settings are personal choices that may differ between a user's devices.

## Technical Implementation

### Loading Settings

**LocalSettings loading:** See `settings.rs:L29-39` for the `load()` method. Reads from `local.settings.json` in the app data directory, falling back to defaults if the file is missing or malformed.

**Settings loading:** See `db.rs:L593-598` for the `get_settings()` method. Queries the `settings` table and converts the row to a domain model using Diesel ORM.

### Saving Settings

**LocalSettings saving:** See `settings.rs:L42-52` for the `save()` method. Writes pretty-printed JSON to disk with `sync_all()` to ensure durability (data flushed to disk before returning).

**Settings saving:** See `db.rs:L601-636` for the `save_settings()` method. Uses an upsert pattern - checks if settings exist, then updates or inserts accordingly.

Write commands fail in read-only mode with a user-facing error.

### Frontend Integration

The Settings UI (`+page.svelte`) loads both setting types and presents them in a unified interface. See `+page.svelte:L273-308` for the `onMount()` loading pattern that fetches database settings, local settings (API key, receipts folder), and database location info in parallel.

**Auto-save with debouncing:** See `+page.svelte:L117-118` for the debounce setup. Both company settings and receipt settings use 800ms debounce to prevent excessive writes while typing.

## Tauri Commands

### LocalSettings Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_theme_preference` | — | `String` | Get theme ("system", "light", "dark") |
| `set_theme_preference` | `theme` | `()` | Set theme preference |
| `get_auto_check_updates` | — | `bool` | Get auto-update setting |
| `set_auto_check_updates` | `enabled` | `()` | Set auto-update setting |
| `get_receipt_settings` | — | `ReceiptSettings` | Get API key, folder path, and override flags |
| `set_gemini_api_key` | `key` | `()` | Set Gemini API key |
| `set_receipts_folder_path` | `path` | `()` | Set receipts folder |
| `get_backup_retention` | — | `BackupRetention?` | Get cleanup settings |
| `set_backup_retention` | `retention` | `()` | Set cleanup settings |
| `get_db_location` | — | `DbLocationInfo` | Get database path, custom-path flag, and backups path |

### Database Settings Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_settings` | — | `Settings?` | Load company settings |
| `save_settings` | `name`, `ico`, `purpose` | `Settings` | Save company settings |

## Key Files

| File | Purpose |
|------|---------|
| [settings.rs](src-tauri/src/settings.rs) | `LocalSettings` struct, load/save methods, `BackupRetention` |
| [models.rs](src-tauri/src/models.rs) | `Settings` struct definition with defaults |
| [commands.rs](src-tauri/src/commands.rs) | All Tauri commands for both setting types |
| [db.rs](src-tauri/src/db.rs) | Database operations for `Settings` |
| [+page.svelte](src/routes/settings/+page.svelte) | Unified settings UI |
| [api.ts](src/lib/api.ts) | TypeScript API wrappers |
| [types.ts](src/lib/types.ts) | TypeScript interfaces (`Settings`, `ReceiptSettings`) |

## Design Decisions

### Why JSON for LocalSettings?

1. **Survives reinstalls** — App data directory persists when updating/reinstalling
2. **Human-readable** — Users can manually edit if needed
3. **No migration needed** — New fields with `Option<T>` are backward compatible
4. **Platform standard** — Uses OS-appropriate config locations

### Why Database for Settings?

1. **Travels with data** — Company info is tied to the vehicle/trip data
2. **Consistent** — Same ACID guarantees as other business data
3. **Single source** — No sync conflicts between files

### Unified UI Despite Split Storage

The user sees one Settings page, unaware of the underlying split. This provides:
- Simple mental model for users
- All settings in one place
- Transparent save/load behavior

### Sample `local.settings.json`

```json
{
    "gemini_api_key": "YOUR_API_KEY_HERE",
    "receipts_folder_path": "C:\\Users\\YourUsername\\Documents\\Receipts",
    "theme": "dark",
    "auto_check_updates": true,
    "custom_db_path": "D:\\GoogleDrive\\kniha-jazd",
    "backup_retention": {
        "enabled": true,
        "keepCount": 3
    }
}
```
