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

**BackupRetention**:
```rust
#[serde(rename_all = "camelCase")]
pub struct BackupRetention {
    pub enabled: bool,
    pub keep_count: u32,
}
```

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

**Default values**:
```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            company_name: String::new(),
            company_ico: String::new(),
            buffer_trip_purpose: "služobná cesta".to_string(),
            updated_at: Utc::now(),
        }
    }
}
```

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

**LocalSettings** (from file):
```rust
impl LocalSettings {
    pub fn load(app_data_dir: &PathBuf) -> Self {
        let path = app_data_dir.join("local.settings.json");
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }
}
```

**Settings** (from database):
```rust
pub fn get_settings(&self) -> QueryResult<Option<Settings>> {
    let conn = &mut *self.conn.lock().unwrap();
    let row = settings::table.first::<SettingsRow>(conn).optional()?;
    Ok(row.map(Settings::from))
}
```

### Saving Settings

**LocalSettings** saves to JSON with fsync for durability:
```rust
pub fn save(&self, app_data_dir: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join("local.settings.json");
    let json = serde_json::to_string_pretty(self)?;
    let mut file = fs::File::create(&path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()
}
```

**Settings** uses upsert pattern in SQLite:
```rust
pub fn save_settings(&self, s: &Settings) -> QueryResult<()> {
    let exists: i64 = settings::table.count().get_result(conn)?;
    if exists > 0 {
        diesel::update(settings::table)
            .set((/* fields */))
            .execute(conn)?;
    } else {
        diesel::insert_into(settings::table)
            .values(&new_settings)
            .execute(conn)?;
    }
    Ok(())
}
```

### Frontend Integration

The Settings UI (`+page.svelte`) loads both setting types and presents them in a unified interface:

```typescript
onMount(async () => {
    // Load database settings
    const loadedSettings = await api.getSettings();
    if (loadedSettings) {
        companyName = loadedSettings.companyName;
        companyIco = loadedSettings.companyIco;
        bufferTripPurpose = loadedSettings.bufferTripPurpose;
    }

    // Load local settings (via separate commands)
    autoCheckUpdates = await getAutoCheckUpdates();
    const receiptSettings = await getReceiptSettings();
    geminiApiKey = receiptSettings.geminiApiKey || '';
    receiptsFolderPath = receiptSettings.receiptsFolderPath || '';
    dbLocation = await getDbLocation();
});
```

**Auto-save with debouncing** prevents excessive writes:
```typescript
const debouncedSaveCompanySettings = debounce(saveCompanySettingsNow, 800);
const debouncedSaveReceiptSettings = debounce(saveReceiptSettingsNow, 800);
```

## Tauri Commands

### LocalSettings Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_theme` | — | `String` | Get theme ("system", "light", "dark") |
| `set_theme` | `theme` | `()` | Set theme preference |
| `get_auto_check_updates` | — | `bool` | Get auto-update setting |
| `set_auto_check_updates` | `enabled` | `()` | Set auto-update setting |
| `get_receipt_settings` | — | `ReceiptSettings` | Get API key and folder path |
| `set_gemini_api_key` | `key` | `()` | Set Gemini API key |
| `set_receipts_folder_path` | `path` | `()` | Set receipts folder |
| `get_backup_retention` | — | `BackupRetention?` | Get cleanup settings |
| `set_backup_retention` | `retention` | `()` | Set cleanup settings |
| `get_db_location` | — | `DbLocation` | Get database path info |

### Database Settings Commands

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_settings` | — | `Settings?` | Load company settings |
| `update_settings` | `name`, `ico`, `purpose` | `Settings` | Save company settings |

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
