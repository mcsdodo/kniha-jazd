**Date:** 2026-01-24
**Subject:** Automatic Database Backup Before Updates
**Status:** Planning

## Overview

Create automatic database backup before app updates, with configurable retention for cleanup.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Trigger | Before download starts | User sees backup as part of update flow, simplest to implement |
| Naming | Filename suffix `-pred-v{version}` + UI badge | Recognizable in file explorer + clear in app UI |
| Backup failure | Warn user, let them choose | User stays in control, doesn't block updates for minor issues |
| Retention | Configurable (keep N) + manual execute | User controls disk space, cleanup on-demand |
| Auto-cleanup detection | Backup filename contains target version | No extra storage needed, elegant solution |
| Update flow UX | Sequential steps with visible progress | Builds trust, clear indication |
| Skip option | None — always create backup | Safety is mandatory, retention handles disk space |

## Data Model

### Extended BackupInfo

```typescript
interface BackupInfo {
  filename: string;      // "kniha-jazd-backup-2026-01-23-143022-pred-v0.20.0.db"
  createdAt: string;
  sizeBytes: number;
  vehicleCount: number;
  tripCount: number;
  backupType: 'manual' | 'pre-update';  // NEW
  updateVersion: string | null;          // NEW - e.g., "0.20.0" for pre-update backups
}
```

### Retention Settings

Stored in `local.settings.json`:
```json
{
  "theme": "system",
  "customDbPath": null,
  "backupRetention": {
    "enabled": false,
    "keepCount": 3
  }
}
```

### Filename Format

```
Manual:     kniha-jazd-backup-2026-01-23-143022.db
Pre-update: kniha-jazd-backup-2026-01-23-143022-pred-v0.20.0.db
```

Parsed with regex:
```rust
let re = Regex::new(r"kniha-jazd-backup-(\d{4}-\d{2}-\d{2}-\d{6})(?:-pred-v([\d.]+))?\.db")?;
```

## Update Flow

```
User clicks "Aktualizovať"
         │
         ▼
┌─────────────────────────┐
│ 1. Create backup        │ ← createBackupWithType("pre-update", "0.20.0")
└───────────┬─────────────┘
            │
      ┌─────┴─────┐
      │  Success? │
      └─────┬─────┘
           │
    ┌──────┴──────┐
    │ Yes         │ No
    ▼             ▼
┌─────────┐  ┌─────────────────────────────┐
│ Step ✓  │  │ Show warning dialog:        │
│ Continue│  │ "Záloha zlyhala. Pokračovať │
└────┬────┘  │  bez zálohy?"               │
     │       │ [Zrušiť] [Pokračovať]       │
     │       └──────────────┬──────────────┘
     │                      │
     │         ┌────────────┴────────────┐
     │         │ User chooses            │
     │         ▼                         ▼
     │    [Pokračovať]              [Zrušiť]
     │         │                         │
     ├─────────┘                    (abort)
     │
     ▼
┌─────────────────────────┐
│ 2. Download update      │
└───────────┬─────────────┘
            ▼
┌─────────────────────────┐
│ 3. Install & relaunch   │
└─────────────────────────┘
```

### Update Modal States

```
┌─ Aktualizácia na v0.20.0 ──────────────────────────────────┐
│                                                            │
│  ✓ Záloha vytvorená                                       │
│  ● Sťahujem aktualizáciu...  [████████░░░░] 67%           │
│  ○ Inštalácia                                              │
│                                                            │
│                                    [Zrušiť]               │
└────────────────────────────────────────────────────────────┘
```

## Auto-Cleanup Detection

On startup, check if we just updated:

```rust
let latest_pre_update = find_most_recent_pre_update_backup();
if let Some(backup) = latest_pre_update {
    if backup.update_version == current_app_version && retention_enabled {
        cleanup_pre_update_backups(keep_count);
    }
}
```

No extra version storage needed — the backup filename contains the target version.

## Settings UI

### Backup Section Layout

```
┌─ Zálohy ─────────────────────────────────────────────────────┐
│  [Vytvoriť zálohu]                                           │
│                                                              │
│  ┌─ Automatické čistenie ─────────────────────────────────┐ │
│  │  [✓] Ponechať iba posledných [3 ▼] automatických záloh │ │
│  │                                                         │ │
│  │  Na vymazanie: 2 zálohy (2.1 MB)    [Vyčistiť teraz]   │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  Dostupné zálohy:                                           │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ 23.1.2026, 14:30  [pred v0.20.0]  1.2MB [Otv][Obn][X]  │ │
│  │ 21.1.2026, 10:15  [pred v0.19.0]  1.1MB [Otv][Obn][X]  │ │
│  │ 18.1.2026, 09:30                  1.0MB [Otv][Obn][X]  │ │
│  └─────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```

- Badge `[pred v0.20.0]` only for pre-update backups
- Manual backups show no badge
- Cleanup only affects pre-update backups

## New Tauri Commands

```rust
// Create backup with type and version metadata
#[tauri::command]
fn create_backup_with_type(
    backup_type: String,            // "manual" | "pre-update"
    update_version: Option<String>  // e.g., "0.20.0"
) -> Result<BackupInfo, String>

// Get cleanup preview (what would be deleted)
#[tauri::command]
fn get_cleanup_preview(keep_count: u32) -> Result<CleanupPreview, String>
// Returns: { toDelete: BackupInfo[], totalBytes: u64 }

// Execute cleanup
#[tauri::command]
fn cleanup_pre_update_backups(keep_count: u32) -> Result<CleanupResult, String>
// Returns: { deleted: Vec<String>, freedBytes: u64 }
```

Existing `list_backups` enhanced to parse `backupType` and `updateVersion` from filename.

## Testing Strategy

### Backend Unit Tests

| Test Case | File |
|-----------|------|
| `create_backup_with_type` creates correct filename for manual | `db_location_tests.rs` |
| `create_backup_with_type` creates correct filename for pre-update | `db_location_tests.rs` |
| `list_backups` parses manual backup filename | `db_location_tests.rs` |
| `list_backups` parses pre-update backup with version | `db_location_tests.rs` |
| `get_cleanup_preview` returns correct count for keep=3 | `db_location_tests.rs` |
| `get_cleanup_preview` ignores manual backups | `db_location_tests.rs` |
| `cleanup_pre_update_backups` deletes only pre-update backups | `db_location_tests.rs` |
| `cleanup_pre_update_backups` keeps N most recent | `db_location_tests.rs` |
| Retention settings save/load correctly | `settings_tests.rs` |

### Integration Tests

| Test Case | File | Tier |
|-----------|------|------|
| Backup list shows badge for pre-update backups | `backup-cleanup.spec.ts` | 2 |
| Cleanup preview shows correct count | `backup-cleanup.spec.ts` | 2 |
| "Vyčistiť teraz" deletes old pre-update backups | `backup-cleanup.spec.ts` | 2 |
| Manual backups not affected by cleanup | `backup-cleanup.spec.ts` | 2 |

### Not Tested (by design)

- Update flow with actual download (requires mock server, manual testing via `test-release.ps1`)
- Tauri updater plugin behavior (third-party library)

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/db_location.rs` | New backup functions with type/version |
| `src-tauri/src/commands.rs` | New commands: `create_backup_with_type`, `get_cleanup_preview`, `cleanup_pre_update_backups` |
| `src-tauri/src/lib.rs` | Register new commands, add startup cleanup check |
| `src-tauri/src/settings.rs` | Add `backupRetention` to local settings |
| `src/lib/types.ts` | Extend `BackupInfo` with `backupType`, `updateVersion` |
| `src/lib/api.ts` | Add new API functions |
| `src/lib/stores/update.ts` | Hook backup creation before download |
| `src/routes/settings/+page.svelte` | Retention UI + badge in backup list |
| `src/lib/i18n/sk/index.ts` | Slovak translations |
| `src/lib/i18n/en/index.ts` | English translations |

## New Files

| File | Purpose |
|------|---------|
| `tests/integration/specs/tier2/backup-cleanup.spec.ts` | Integration tests for cleanup UI |
