# Database Backup Before Updates - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Automatically create database backup before app updates, with configurable retention and cleanup.

**Architecture:** Backend-first (ADR-008). All backup logic lives in Rust. Frontend calls Tauri commands for backup creation and cleanup. Filename encodes backup type and version (no extra storage needed).

**Tech Stack:** Rust (Tauri commands), TypeScript (Svelte stores/components), SQLite (existing backup infrastructure)

---

## Task 1: Extend BackupInfo Struct

**Files:**
- Modify: `src-tauri/src/commands.rs:63-69` (BackupInfo struct)

**Step 1: Add new fields to BackupInfo**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub vehicle_count: i32,
    pub trip_count: i32,
    pub backup_type: String,           // NEW: "manual" | "pre-update"
    pub update_version: Option<String>, // NEW: e.g., "0.20.0" for pre-update
}
```

**Step 2: Run backend tests to verify no regressions**

Run: `cd src-tauri && cargo test`
Expected: All existing tests pass (struct fields are additive)

**Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(backup): extend BackupInfo with backupType and updateVersion fields"
```

---

## Task 2: Add Backup Retention Settings

**Files:**
- Modify: `src-tauri/src/settings.rs:8-15` (LocalSettings struct)
- Test: `src-tauri/src/settings.rs` (tests module)

**Step 1: Write failing test for retention settings**

Add to `settings.rs` tests module:

```rust
#[test]
fn test_load_with_backup_retention() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("local.settings.json");
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(br#"{"backup_retention": {"enabled": true, "keep_count": 5}}"#).unwrap();

    let settings = LocalSettings::load(&dir.path().to_path_buf());
    assert!(settings.backup_retention.is_some());
    let retention = settings.backup_retention.unwrap();
    assert!(retention.enabled);
    assert_eq!(retention.keep_count, 5);
}

#[test]
fn test_backup_retention_defaults() {
    let dir = tempdir().unwrap();
    let settings = LocalSettings::load(&dir.path().to_path_buf());
    // When missing, should be None (not enabled by default)
    assert!(settings.backup_retention.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test test_load_with_backup_retention`
Expected: FAIL - `backup_retention` field does not exist

**Step 3: Add BackupRetention struct and field**

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupRetention {
    pub enabled: bool,
    pub keep_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub theme: Option<String>,
    pub auto_check_updates: Option<bool>,
    pub custom_db_path: Option<String>,
    pub backup_retention: Option<BackupRetention>,  // NEW
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src-tauri/src/settings.rs
git commit -m "feat(backup): add backup retention settings to LocalSettings"
```

---

## Task 3: Create Backup With Type (Backend)

**Files:**
- Modify: `src-tauri/src/commands.rs` (add new command)
- Modify: `src-tauri/src/lib.rs` (register command)

**Step 1: Write failing test for filename parsing**

Add test in `commands.rs` tests section (or create `commands_backup_tests.rs`):

```rust
#[test]
fn test_parse_backup_filename_manual() {
    let filename = "kniha-jazd-backup-2026-01-24-143022.db";
    let (backup_type, update_version) = parse_backup_filename(filename);
    assert_eq!(backup_type, "manual");
    assert_eq!(update_version, None);
}

#[test]
fn test_parse_backup_filename_pre_update() {
    let filename = "kniha-jazd-backup-2026-01-24-143022-pred-v0.20.0.db";
    let (backup_type, update_version) = parse_backup_filename(filename);
    assert_eq!(backup_type, "pre-update");
    assert_eq!(update_version, Some("0.20.0".to_string()));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test test_parse_backup_filename`
Expected: FAIL - function does not exist

**Step 3: Implement parse_backup_filename helper**

```rust
/// Parse backup filename to extract type and version
/// Manual: kniha-jazd-backup-2026-01-24-143022.db
/// Pre-update: kniha-jazd-backup-2026-01-24-143022-pred-v0.20.0.db
fn parse_backup_filename(filename: &str) -> (String, Option<String>) {
    if let Some(caps) = filename.strip_prefix("kniha-jazd-backup-") {
        if let Some(version_start) = caps.find("-pred-v") {
            let version = caps[version_start + 7..].trim_end_matches(".db");
            return ("pre-update".to_string(), Some(version.to_string()));
        }
    }
    ("manual".to_string(), None)
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test test_parse_backup_filename`
Expected: PASS

**Step 5: Write test for create_backup_with_type command**

```rust
#[test]
fn test_generate_backup_filename_manual() {
    let filename = generate_backup_filename("manual", None);
    assert!(filename.starts_with("kniha-jazd-backup-"));
    assert!(filename.ends_with(".db"));
    assert!(!filename.contains("-pred-v"));
}

#[test]
fn test_generate_backup_filename_pre_update() {
    let filename = generate_backup_filename("pre-update", Some("0.20.0"));
    assert!(filename.starts_with("kniha-jazd-backup-"));
    assert!(filename.contains("-pred-v0.20.0.db"));
}
```

**Step 6: Implement generate_backup_filename helper**

```rust
fn generate_backup_filename(backup_type: &str, update_version: Option<&str>) -> String {
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    match (backup_type, update_version) {
        ("pre-update", Some(version)) => format!("kniha-jazd-backup-{}-pred-v{}.db", timestamp, version),
        _ => format!("kniha-jazd-backup-{}.db", timestamp),
    }
}
```

**Step 7: Run tests**

Run: `cd src-tauri && cargo test test_generate_backup_filename`
Expected: PASS

**Step 8: Implement create_backup_with_type command**

```rust
#[tauri::command]
pub fn create_backup_with_type(
    app: tauri::AppHandle,
    db: State<Database>,
    app_state: State<AppState>,
    backup_type: String,
    update_version: Option<String>,
) -> Result<BackupInfo, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let backup_dir = app_dir.join("backups");

    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let filename = generate_backup_filename(&backup_type, update_version.as_deref());
    let backup_path = backup_dir.join(&filename);

    // Copy current database to backup
    let db_path = app_dir.join("kniha-jazd.db");
    fs::copy(&db_path, &backup_path).map_err(|e| e.to_string())?;

    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    let vehicles = db.get_all_vehicles().map_err(|e| e.to_string())?;
    let vehicle_count = vehicles.len() as i32;

    let mut trip_count = 0;
    for vehicle in &vehicles {
        let trips = db.get_trips_for_vehicle(&vehicle.id.to_string()).map_err(|e| e.to_string())?;
        trip_count += trips.len() as i32;
    }

    let (parsed_type, parsed_version) = parse_backup_filename(&filename);

    Ok(BackupInfo {
        filename,
        created_at: Local::now().to_rfc3339(),
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
        backup_type: parsed_type,
        update_version: parsed_version,
    })
}
```

**Step 9: Register command in lib.rs**

Add `create_backup_with_type` to the invoke_handler list.

**Step 10: Update existing create_backup to use new function internally**

Refactor `create_backup` to call the helper with "manual" type.

**Step 11: Update list_backups to populate new fields**

Modify the `list_backups` function to call `parse_backup_filename` and populate `backup_type` and `update_version`.

**Step 12: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 13: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(backup): add create_backup_with_type command with filename-based type detection"
```

---

## Task 4: Cleanup Preview and Execute (Backend)

**Files:**
- Modify: `src-tauri/src/commands.rs` (add cleanup commands)
- Modify: `src-tauri/src/lib.rs` (register commands)

**Step 1: Write failing test for cleanup preview**

```rust
#[test]
fn test_get_cleanup_candidates_keeps_n_most_recent() {
    let backups = vec![
        ("kniha-jazd-backup-2026-01-20-100000-pred-v0.17.0.db", "pre-update"),
        ("kniha-jazd-backup-2026-01-21-100000-pred-v0.18.0.db", "pre-update"),
        ("kniha-jazd-backup-2026-01-22-100000-pred-v0.19.0.db", "pre-update"),
        ("kniha-jazd-backup-2026-01-23-100000-pred-v0.20.0.db", "pre-update"),
        ("kniha-jazd-backup-2026-01-24-100000.db", "manual"),  // Should be ignored
    ];

    let to_delete = get_cleanup_candidates(&backups, 2);

    // Should delete oldest 2 pre-update backups, keep 2 most recent
    assert_eq!(to_delete.len(), 2);
    assert!(to_delete.contains(&"kniha-jazd-backup-2026-01-20-100000-pred-v0.17.0.db"));
    assert!(to_delete.contains(&"kniha-jazd-backup-2026-01-21-100000-pred-v0.18.0.db"));
}

#[test]
fn test_get_cleanup_candidates_ignores_manual() {
    let backups = vec![
        ("kniha-jazd-backup-2026-01-20-100000.db", "manual"),
        ("kniha-jazd-backup-2026-01-21-100000.db", "manual"),
    ];

    let to_delete = get_cleanup_candidates(&backups, 1);

    // Manual backups should never be deleted
    assert_eq!(to_delete.len(), 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test test_get_cleanup_candidates`
Expected: FAIL - function does not exist

**Step 3: Implement get_cleanup_candidates helper**

```rust
fn get_cleanup_candidates(backups: &[(&str, &str)], keep_count: u32) -> Vec<String> {
    let mut pre_update_backups: Vec<&str> = backups
        .iter()
        .filter(|(_, backup_type)| *backup_type == "pre-update")
        .map(|(filename, _)| *filename)
        .collect();

    // Sort by filename (which includes timestamp) - oldest first
    pre_update_backups.sort();

    let total = pre_update_backups.len();
    let keep = keep_count as usize;

    if total <= keep {
        return vec![];
    }

    // Return the oldest ones (to delete)
    pre_update_backups[0..(total - keep)]
        .iter()
        .map(|s| s.to_string())
        .collect()
}
```

**Step 4: Run tests**

Run: `cd src-tauri && cargo test test_get_cleanup_candidates`
Expected: PASS

**Step 5: Add CleanupPreview and CleanupResult types**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
    pub to_delete: Vec<BackupInfo>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub deleted: Vec<String>,
    pub freed_bytes: u64,
}
```

**Step 6: Implement get_cleanup_preview command**

```rust
#[tauri::command]
pub fn get_cleanup_preview(app: tauri::AppHandle, keep_count: u32) -> Result<CleanupPreview, String> {
    let all_backups = list_backups(app.clone())?;

    let pre_update_backups: Vec<&BackupInfo> = all_backups
        .iter()
        .filter(|b| b.backup_type == "pre-update")
        .collect();

    let mut sorted: Vec<&BackupInfo> = pre_update_backups;
    sorted.sort_by(|a, b| a.filename.cmp(&b.filename));

    let total = sorted.len();
    let keep = keep_count as usize;

    if total <= keep {
        return Ok(CleanupPreview {
            to_delete: vec![],
            total_bytes: 0,
        });
    }

    let to_delete: Vec<BackupInfo> = sorted[0..(total - keep)]
        .iter()
        .map(|b| (*b).clone())
        .collect();

    let total_bytes: u64 = to_delete.iter().map(|b| b.size_bytes).sum();

    Ok(CleanupPreview { to_delete, total_bytes })
}
```

**Step 7: Implement cleanup_pre_update_backups command**

```rust
#[tauri::command]
pub fn cleanup_pre_update_backups(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    check_read_only!(app_state);

    let preview = get_cleanup_preview(app.clone(), keep_count)?;
    let app_dir = get_app_data_dir(&app)?;
    let backup_dir = app_dir.join("backups");

    let mut deleted = Vec::new();
    let mut freed_bytes = 0u64;

    for backup in &preview.to_delete {
        let path = backup_dir.join(&backup.filename);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            deleted.push(backup.filename.clone());
            freed_bytes += backup.size_bytes;
        }
    }

    Ok(CleanupResult { deleted, freed_bytes })
}
```

**Step 8: Register commands in lib.rs**

Add `get_cleanup_preview` and `cleanup_pre_update_backups` to invoke_handler.

**Step 9: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 10: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(backup): add cleanup preview and execute commands"
```

---

## Task 5: Frontend Types and API

> **Note:** This task defines API functions that call commands from Task 6. Implement Task 6 first, or implement both tasks together.

**Files:**
- Modify: `src/lib/types.ts` (extend BackupInfo)
- Modify: `src/lib/api.ts` (add new functions)

**Step 1: Extend BackupInfo type**

```typescript
export interface BackupInfo {
	filename: string;
	createdAt: string;
	sizeBytes: number;
	vehicleCount: number;
	tripCount: number;
	backupType: 'manual' | 'pre-update';  // NEW
	updateVersion: string | null;          // NEW
}

export interface CleanupPreview {
	toDelete: BackupInfo[];
	totalBytes: number;
}

export interface CleanupResult {
	deleted: string[];
	freedBytes: number;
}

export interface BackupRetention {
	enabled: boolean;
	keepCount: number;
}
```

**Step 2: Add API functions**

```typescript
export async function createBackupWithType(
	backupType: 'manual' | 'pre-update',
	updateVersion: string | null
): Promise<BackupInfo> {
	return invoke('create_backup_with_type', { backupType, updateVersion });
}

export async function getCleanupPreview(keepCount: number): Promise<CleanupPreview> {
	return invoke('get_cleanup_preview', { keepCount });
}

export async function cleanupPreUpdateBackups(keepCount: number): Promise<CleanupResult> {
	return invoke('cleanup_pre_update_backups', { keepCount });
}

export async function getBackupRetention(): Promise<BackupRetention | null> {
	return invoke('get_backup_retention');
}

export async function setBackupRetention(retention: BackupRetention): Promise<void> {
	return invoke('set_backup_retention', { retention });
}
```

**Step 3: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(backup): add frontend types and API for backup with type and cleanup"
```

---

## Task 6: Retention Settings Commands (Backend)

**Files:**
- Modify: `src-tauri/src/commands.rs` (add settings commands)
- Modify: `src-tauri/src/lib.rs` (register commands)

**Step 1: Implement get_backup_retention command**

```rust
#[tauri::command]
pub fn get_backup_retention(app: tauri::AppHandle) -> Result<Option<BackupRetention>, String> {
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);
    Ok(settings.backup_retention)
}
```

**Step 2: Implement set_backup_retention command**

```rust
#[tauri::command]
pub fn set_backup_retention(
    app: tauri::AppHandle,
    app_state: State<AppState>,  // Required for read-only check
    retention: BackupRetention,
) -> Result<(), String> {
    check_read_only!(app_state);  // Write command must check read-only mode
    let app_dir = get_app_data_dir(&app)?;
    let mut settings = LocalSettings::load(&app_dir);
    settings.backup_retention = Some(retention);
    settings.save(&app_dir).map_err(|e| e.to_string())
}
```

**Step 3: Add import for BackupRetention in commands.rs**

```rust
use crate::settings::{LocalSettings, BackupRetention};
```

**Step 4: Register commands in lib.rs**

**Step 5: Run backend tests**

Run: `cd src-tauri && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/src/settings.rs
git commit -m "feat(backup): add retention settings get/set commands"
```

---

## Task 7: i18n Translations

**Files:**
- Modify: `src/lib/i18n/sk/index.ts`
- Modify: `src/lib/i18n/en/index.ts`

**Step 1: Add Slovak translations**

```typescript
// In backup section (nest under existing `backup` object)
backup: {
    // ... existing keys ...
    retention: {
        title: () => 'Automatické čistenie',
        enabled: () => 'Ponechať iba posledných',
        backups: () => 'automatických záloh',
        toDelete: (params: { count: number; size: string }) =>
            `Na vymazanie: ${params.count} ${params.count === 1 ? 'záloha' : params.count < 5 ? 'zálohy' : 'záloh'} (${params.size})`,
        cleanNow: () => 'Vyčistiť teraz',
        nothingToClean: () => 'Nič na vyčistenie',
    },
    badge: {
        preUpdate: (params: { version: string }) => `pred ${params.version}`,
    },
},
// In update section (nest under existing `update` object)
update: {
    // ... existing keys ...
    backupStep: () => 'Záloha vytvorená',
    backupInProgress: () => 'Vytváranie zálohy...',
    backupFailed: () => 'Záloha zlyhala',
    backupFailedMessage: () => 'Nepodarilo sa vytvoriť zálohu databázy. Chcete pokračovať v aktualizácii bez zálohy?',
    continueWithoutBackup: () => 'Pokračovať bez zálohy',
},
// In toast section
toast: {
    // ... existing keys ...
    cleanupComplete: () => 'Zálohy boli vyčistené',
},
```

**Step 2: Add English translations**

```typescript
// In backup section (nest under existing `backup` object)
backup: {
    // ... existing keys ...
    retention: {
        title: () => 'Automatic cleanup',
        enabled: () => 'Keep only last',
        backups: () => 'automatic backups',
        toDelete: (params: { count: number; size: string }) =>
            `To delete: ${params.count} backup${params.count === 1 ? '' : 's'} (${params.size})`,
        cleanNow: () => 'Clean now',
        nothingToClean: () => 'Nothing to clean',
    },
    badge: {
        preUpdate: (params: { version: string }) => `before ${params.version}`,
    },
},
// In update section
update: {
    // ... existing keys ...
    backupStep: () => 'Backup created',
    backupInProgress: () => 'Creating backup...',
    backupFailed: () => 'Backup failed',
    backupFailedMessage: () => 'Failed to create database backup. Do you want to continue updating without a backup?',
    continueWithoutBackup: () => 'Continue without backup',
},
// In toast section
toast: {
    // ... existing keys ...
    cleanupComplete: () => 'Backups cleaned up',
},
```

**Step 3: Commit**

```bash
git add src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts
git commit -m "feat(backup): add i18n translations for backup retention and update flow"
```

---

## Task 8: Settings UI - Retention Controls

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Add state for retention settings**

```typescript
// Backup retention state
let retentionEnabled = false;
let retentionKeepCount = 3;
let cleanupPreview: CleanupPreview | null = null;
let cleaningUp = false;
```

**Step 2: Load retention settings on mount**

```typescript
// In onMount or the async IIFE
const retention = await api.getBackupRetention();
if (retention) {
    retentionEnabled = retention.enabled;
    retentionKeepCount = retention.keepCount;
}
if (retentionEnabled) {
    cleanupPreview = await api.getCleanupPreview(retentionKeepCount);
}
```

**Step 3: Add handlers**

```typescript
async function handleRetentionChange() {
    await api.setBackupRetention({
        enabled: retentionEnabled,
        keepCount: retentionKeepCount
    });
    if (retentionEnabled) {
        cleanupPreview = await api.getCleanupPreview(retentionKeepCount);
    } else {
        cleanupPreview = null;
    }
}

async function handleCleanupNow() {
    if (!cleanupPreview || cleanupPreview.toDelete.length === 0) return;
    cleaningUp = true;
    try {
        await api.cleanupPreUpdateBackups(retentionKeepCount);
        await loadBackups();
        cleanupPreview = await api.getCleanupPreview(retentionKeepCount);
        toast.success($LL.toast.cleanupComplete());
    } catch (error) {
        toast.error(String(error));
    } finally {
        cleaningUp = false;
    }
}
```

**Step 4: Add UI in backup section**

```svelte
<!-- After the create backup button -->
<div class="retention-settings" data-testid="retention-settings">
    <h4>{$LL.backup.retention.title()}</h4>
    <label class="checkbox-label">
        <input
            type="checkbox"
            bind:checked={retentionEnabled}
            on:change={handleRetentionChange}
            data-testid="retention-enabled"
        />
        {$LL.backup.retention.enabled()}
        <select
            bind:value={retentionKeepCount}
            data-testid="retention-keep-count"
            on:change={handleRetentionChange}
            disabled={!retentionEnabled}
        >
            <option value={3}>3</option>
            <option value={5}>5</option>
            <option value={10}>10</option>
        </select>
        {$LL.backup.retention.backups()}
    </label>

    {#if retentionEnabled && cleanupPreview}
        <div class="cleanup-preview">
            {#if cleanupPreview.toDelete.length > 0}
                <span>{$LL.backup.retention.toDelete({
                    count: cleanupPreview.toDelete.length,
                    size: formatFileSize(cleanupPreview.totalBytes)
                })}</span>
                <button
                    class="button-small"
                    on:click={handleCleanupNow}
                    disabled={cleaningUp}
                >
                    {cleaningUp ? '...' : $LL.backup.retention.cleanNow()}
                </button>
            {:else}
                <span class="placeholder">{$LL.backup.retention.nothingToClean()}</span>
            {/if}
        </div>
    {/if}
</div>
```

**Step 5: Add badge to backup list**

```svelte
{#each backups as backup}
    <div class="backup-item">
        <div class="backup-info">
            <span class="backup-date">{formatBackupDate(backup.createdAt)}</span>
            {#if backup.backupType === 'pre-update' && backup.updateVersion}
                <span class="backup-badge">{$LL.backup.badge.preUpdate({ version: backup.updateVersion })}</span>
            {/if}
            <span class="backup-size">{formatFileSize(backup.sizeBytes)}</span>
        </div>
        <!-- ... actions ... -->
    </div>
{/each}
```

**Step 6: Add CSS for new elements**

```css
.retention-settings {
    margin-top: 1rem;
    padding: 1rem;
    background: var(--bg-secondary);
    border-radius: 8px;
}

.retention-settings h4 {
    margin: 0 0 0.75rem 0;
    font-size: 0.9rem;
    color: var(--text-secondary);
}

.retention-settings select {
    margin: 0 0.5rem;
    padding: 0.25rem 0.5rem;
}

.cleanup-preview {
    margin-top: 0.75rem;
    display: flex;
    align-items: center;
    gap: 1rem;
}

.backup-badge {
    font-size: 0.75rem;
    padding: 0.125rem 0.5rem;
    background: var(--accent-light);
    color: var(--accent);
    border-radius: 4px;
}
```

**Step 7: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(backup): add retention settings UI with cleanup preview"
```

---

## Task 9: Update Store - Backup Before Download

**Files:**
- Modify: `src/lib/stores/update.ts`

**Step 1: Add backup states**

```typescript
interface UpdateState {
    checking: boolean;
    available: boolean;
    version: string | null;
    releaseNotes: string | null;
    dismissed: boolean;
    downloading: boolean;
    progress: number;
    error: string | null;
    // NEW states for backup
    backupStep: 'pending' | 'in-progress' | 'done' | 'failed' | 'skipped';
    backupError: string | null;
}
```

**Step 2: Update initial state**

```typescript
const initialState: UpdateState = {
    // ... existing ...
    backupStep: 'pending',
    backupError: null
};
```

**Step 3: Modify install function**

```typescript
install: async () => {
    if (!updateObject) {
        throw new Error('No update available to install');
    }

    // Step 1: Create backup
    updateState((state) => ({
        ...state,
        backupStep: 'in-progress',
        backupError: null,
        error: null
    }));

    try {
        const { createBackupWithType } = await import('$lib/api');
        await createBackupWithType('pre-update', updateObject.version);
        updateState((state) => ({ ...state, backupStep: 'done' }));
    } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        updateState((state) => ({
            ...state,
            backupStep: 'failed',
            backupError: errorMsg
        }));
        // Don't proceed - let UI handle the failed state
        return;
    }

    // Step 2: Download and install
    await performDownloadAndInstall();
},

// NEW: Continue after backup failure (user chose to proceed)
continueWithoutBackup: async () => {
    updateState((state) => ({ ...state, backupStep: 'skipped' }));
    await performDownloadAndInstall();
},
```

**Step 4: Extract download logic into reusable function**

Add this helper inside `createUpdateStore()` before the return statement:

```typescript
// Extracted download logic (used by install and continueWithoutBackup)
async function performDownloadAndInstall() {
    if (!updateObject) {
        throw new Error('No update available to install');
    }

    updateState((state) => ({ ...state, downloading: true, error: null }));
    try {
        let contentLength = 0;
        let downloaded = 0;
        await updateObject.downloadAndInstall((event) => {
            if (event.event === 'Started') {
                contentLength = event.data.contentLength || 0;
                updateState((state) => ({ ...state, downloading: true }));
            } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                const progress = contentLength > 0 ? Math.round((downloaded / contentLength) * 100) : 0;
                updateState((state) => ({ ...state, progress }));
            } else if (event.event === 'Finished') {
                updateState((state) => ({ ...state, downloading: false, progress: 100 }));
            }
        });

        await relaunch();
    } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        updateState((state) => ({
            ...state,
            downloading: false,
            error: errorMsg
        }));
    }
}
```

**Step 4: Commit**

```bash
git add src/lib/stores/update.ts
git commit -m "feat(backup): add backup step to update store before download"
```

---

## Task 10: Update Modal UI

**Files:**
- Modify: `src/routes/settings/+page.svelte` (or dedicated UpdateModal component)

**Step 1: Update the update modal to show backup step**

Find the update modal section and update to show three steps:

```svelte
{#if $updateStore.available && !$updateStore.dismissed}
    <!-- Update Modal -->
    <div class="modal-overlay">
        <div class="modal update-modal">
            <h2>{$LL.update.title({ version: $updateStore.version })}</h2>

            <div class="update-steps">
                <!-- Step 1: Backup -->
                <div class="step" class:done={$updateStore.backupStep === 'done' || $updateStore.backupStep === 'skipped'}
                     class:active={$updateStore.backupStep === 'in-progress'}
                     class:failed={$updateStore.backupStep === 'failed'}>
                    {#if $updateStore.backupStep === 'done'}
                        <span class="step-icon">✓</span>
                    {:else if $updateStore.backupStep === 'skipped'}
                        <span class="step-icon">–</span>
                    {:else if $updateStore.backupStep === 'in-progress'}
                        <span class="step-icon spinner">●</span>
                    {:else if $updateStore.backupStep === 'failed'}
                        <span class="step-icon">✗</span>
                    {:else}
                        <span class="step-icon">○</span>
                    {/if}
                    <span>{$updateStore.backupStep === 'in-progress'
                        ? $LL.update.backupInProgress()
                        : $LL.update.backupStep()}</span>
                </div>

                <!-- Step 2: Download -->
                <div class="step" class:done={$updateStore.progress === 100}
                     class:active={$updateStore.downloading}>
                    {#if $updateStore.progress === 100}
                        <span class="step-icon">✓</span>
                    {:else if $updateStore.downloading}
                        <span class="step-icon spinner">●</span>
                    {:else}
                        <span class="step-icon">○</span>
                    {/if}
                    <span>{$LL.update.downloading()}</span>
                    {#if $updateStore.downloading}
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: {$updateStore.progress}%"></div>
                        </div>
                        <span class="progress-text">{$updateStore.progress}%</span>
                    {/if}
                </div>

                <!-- Step 3: Install -->
                <div class="step">
                    <span class="step-icon">○</span>
                    <span>{$LL.update.installing()}</span>
                </div>
            </div>

            {#if $updateStore.backupStep === 'failed'}
                <div class="warning-box">
                    <p>{$LL.update.backupFailedMessage()}</p>
                    <div class="modal-actions">
                        <button class="button-small" on:click={() => updateStore.dismiss()}>
                            {$LL.common.cancel()}
                        </button>
                        <button class="button-small danger" on:click={() => updateStore.continueWithoutBackup()}>
                            {$LL.update.continueWithoutBackup()}
                        </button>
                    </div>
                </div>
            {:else if !$updateStore.downloading && $updateStore.backupStep === 'pending'}
                <div class="modal-actions">
                    <button class="button-small" on:click={() => updateStore.dismiss()}>
                        {$LL.common.cancel()}
                    </button>
                    <button class="button" on:click={() => updateStore.install()}>
                        {$LL.update.buttonUpdate()}
                    </button>
                </div>
            {/if}
        </div>
    </div>
{/if}
```

**Step 2: Add CSS for steps**

```css
.update-steps {
    margin: 1.5rem 0;
}

.step {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0;
    color: var(--text-secondary);
}

.step.active {
    color: var(--text-primary);
}

.step.done {
    color: var(--success);
}

.step.failed {
    color: var(--danger);
}

.step-icon {
    width: 1.5rem;
    text-align: center;
}

.step-icon.spinner {
    animation: pulse 1s infinite;
}

@keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
}

.warning-box {
    background: var(--warning-bg);
    border: 1px solid var(--warning);
    border-radius: 8px;
    padding: 1rem;
    margin-top: 1rem;
}
```

**Step 3: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(backup): update modal UI with backup step visualization"
```

---

## Task 11: Auto-Cleanup on Startup

**Files:**
- Modify: `src-tauri/src/lib.rs` (startup logic)
- Modify: `src-tauri/src/commands.rs` (add internal helper)

**Step 1: Add internal cleanup helper (no State parameters)**

This is needed because the Tauri command version requires `State<AppState>` which isn't available at startup:

```rust
/// Internal cleanup function for use at startup (no State parameters)
pub fn cleanup_pre_update_backups_internal(
    app: &tauri::AppHandle,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    let app_dir = get_app_data_dir(app)?;
    let backup_dir = app_dir.join("backups");

    // Get all backups
    let all_backups = list_backups(app.clone())?;

    // Filter to pre-update only, sort by filename (oldest first)
    let mut pre_update: Vec<&BackupInfo> = all_backups
        .iter()
        .filter(|b| b.backup_type == "pre-update")
        .collect();
    pre_update.sort_by(|a, b| a.filename.cmp(&b.filename));

    let total = pre_update.len();
    let keep = keep_count as usize;

    if total <= keep {
        return Ok(CleanupResult { deleted: vec![], freed_bytes: 0 });
    }

    // Delete oldest backups (keep N most recent)
    let to_delete = &pre_update[0..(total - keep)];
    let mut deleted = Vec::new();
    let mut freed_bytes = 0u64;

    for backup in to_delete {
        let path = backup_dir.join(&backup.filename);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            deleted.push(backup.filename.clone());
            freed_bytes += backup.size_bytes;
        }
    }

    Ok(CleanupResult { deleted, freed_bytes })
}
```

**Step 2: Update cleanup_pre_update_backups command to use internal helper**

Refactor the Tauri command to delegate to the internal function:

```rust
#[tauri::command]
pub fn cleanup_pre_update_backups(
    app: tauri::AppHandle,
    app_state: State<AppState>,
    keep_count: u32,
) -> Result<CleanupResult, String> {
    check_read_only!(app_state);
    cleanup_pre_update_backups_internal(&app, keep_count)
}
```

**Step 3: Add function to check if just updated**

```rust
/// Check if app just updated by comparing current version with most recent pre-update backup
fn should_run_cleanup(app: &tauri::AppHandle, current_version: &str) -> Option<u32> {
    let app_dir = match get_app_data_dir(app) {
        Ok(dir) => dir,
        Err(_) => return None,
    };

    let settings = LocalSettings::load(&app_dir);
    let retention = settings.backup_retention?;

    if !retention.enabled {
        return None;
    }

    // Get most recent pre-update backup
    let backups = match list_backups(app.clone()) {
        Ok(b) => b,
        Err(_) => return None,
    };

    let latest_pre_update = backups
        .iter()
        .filter(|b| b.backup_type == "pre-update")
        .max_by(|a, b| a.filename.cmp(&b.filename))?;

    // If backup version matches current, we just updated to this version
    if latest_pre_update.update_version.as_deref() == Some(current_version) {
        Some(retention.keep_count)
    } else {
        None
    }
}
```

**Step 4: Call cleanup on startup in lib.rs setup**

In the setup function, after database initialization:

```rust
// Auto-cleanup after update
let current_version = env!("CARGO_PKG_VERSION");
if let Some(keep_count) = should_run_cleanup(&app, current_version) {
    // Run cleanup in background - don't block startup
    let app_clone = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = cleanup_pre_update_backups_internal(&app_clone, keep_count) {
            eprintln!("Auto-cleanup failed: {}", e);
        }
    });
}
```

**Step 5: Run backend tests**

Run: `cd src-tauri && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat(backup): add auto-cleanup on startup after update"
```

---

## Task 12: Integration Tests

**Files:**
- Create: `tests/integration/specs/tier2/backup-cleanup.spec.ts`

**Step 1: Create test file with setup**

```typescript
import { browser, expect } from '@wdio/globals';
import { seedDatabase, clearDatabase, goToSettings } from '../../helpers/test-utils';

describe('Backup Cleanup', () => {
    beforeEach(async () => {
        await clearDatabase();
        await seedDatabase({ vehicles: 1, trips: 5 });
    });

    it('should show badge for pre-update backups', async () => {
        // Create a pre-update backup via IPC
        await browser.execute(async () => {
            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('create_backup_with_type', {
                backupType: 'pre-update',
                updateVersion: '0.20.0'
            });
        });

        await goToSettings();

        // Check for badge in backup list
        const badge = await $('span.backup-badge');
        await expect(badge).toBeDisplayed();
        await expect(badge).toHaveText('pred 0.20.0');
    });

    it('should show cleanup preview when retention enabled', async () => {
        // Create multiple pre-update backups
        await browser.execute(async () => {
            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('create_backup_with_type', { backupType: 'pre-update', updateVersion: '0.18.0' });
            await invoke('create_backup_with_type', { backupType: 'pre-update', updateVersion: '0.19.0' });
            await invoke('create_backup_with_type', { backupType: 'pre-update', updateVersion: '0.20.0' });
        });

        await goToSettings();

        // Enable retention with keep=1 (using data-testid selectors)
        const checkbox = await $('[data-testid="retention-enabled"]');
        await checkbox.click();

        const select = await $('[data-testid="retention-keep-count"]');
        await select.selectByAttribute('value', '1');

        // Should show 2 to delete
        const preview = await $('.cleanup-preview');
        await expect(preview).toHaveTextContaining('2');
    });

    it('should not delete manual backups during cleanup', async () => {
        // Create manual and pre-update backups
        await browser.execute(async () => {
            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('create_backup'); // manual
            await invoke('create_backup_with_type', { backupType: 'pre-update', updateVersion: '0.19.0' });
            await invoke('create_backup_with_type', { backupType: 'pre-update', updateVersion: '0.20.0' });
        });

        await goToSettings();

        // Enable retention with keep=1 (using data-testid selectors)
        const checkbox = await $('[data-testid="retention-enabled"]');
        await checkbox.click();
        const select = await $('[data-testid="retention-keep-count"]');
        await select.selectByAttribute('value', '1');

        // Click cleanup
        const cleanupBtn = await $('button*=Vyčistiť');
        await cleanupBtn.click();

        // Manual backup should still exist
        const backupItems = await $$('.backup-item');
        expect(backupItems.length).toBe(2); // 1 manual + 1 pre-update
    });
});
```

**Step 2: Run integration tests**

Run: `npm run test:integration:build && npm run test:integration`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add tests/integration/specs/tier2/backup-cleanup.spec.ts
git commit -m "test(backup): add integration tests for backup cleanup feature"
```

---

## Task 13: Documentation and Changelog

**Files:**
- Modify: `CHANGELOG.md`

**Step 1: Add changelog entry**

```markdown
## [Unreleased]

### Pridané
- **Automatická záloha pred aktualizáciou** - databáza sa automaticky zálohuje pred každou aktualizáciou
  - Zálohy pred aktualizáciou sú označené štítkom (napr. "pred v0.20.0")
  - Ak záloha zlyhá, používateľ môže zvoliť pokračovanie bez zálohy
  - Nastaviteľné uchovávanie: ponechať posledných N automatických záloh
  - Tlačidlo "Vyčistiť teraz" pre manuálne vyčistenie starých záloh
  - Automatické čistenie po úspešnej aktualizácii (ak je zapnuté)
```

**Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add changelog entry for backup before update feature"
```

---

## Final: Run All Tests

**Step 1: Run backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 2: Run integration tests**

Run: `npm run test:integration`
Expected: All tests PASS

**Step 3: Manual test with test-release**

```powershell
.\scripts\test-release.ps1
node _test-releases/serve.js
# In another terminal:
set TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json && npm run tauri dev
```

Verify:
- Clicking update shows backup step
- Backup is created with correct filename
- Settings shows retention controls
- Cleanup works correctly

---

## Summary

| Task | Description | Tests |
|------|-------------|-------|
| 1 | Extend BackupInfo struct | - |
| 2 | Add retention settings | 2 unit tests |
| 3 | Create backup with type | 4 unit tests |
| 4 | Cleanup preview/execute | 2 unit tests |
| 5 | Frontend types and API | - |
| 6 | Retention settings commands | - |
| 7 | i18n translations | - |
| 8 | Settings UI retention | - |
| 9 | Update store backup step | - |
| 10 | Update modal UI | - |
| 11 | Auto-cleanup on startup | - |
| 12 | Integration tests | 3 integration tests |
| 13 | Documentation | - |

Total: ~8 unit tests + 3 integration tests
