**Date:** 2025-12-23
**Subject:** Implementation Plan - Backup/Restore & Navigation
**Status:** Ready for Implementation

# Backup/Restore & Navigation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add database backup/restore functionality and improve navigation with header links.

**Architecture:**
- Navigation moves to header bar with tab-style links
- Totals section splits into two rows for better readability
- Backup copies SQLite file to timestamped file in backups folder
- Restore replaces current DB with selected backup after user confirmation

**Tech Stack:** Tauri 2.x, SvelteKit 5, Rust, SQLite

---

## Task 1: Add Navigation to Header

**Files:**
- Modify: `src/routes/+layout.svelte`

**Step 1: Add navigation links to header**

In `src/routes/+layout.svelte`, update the header-content div to include navigation:

```svelte
<div class="header-content">
    <div class="header-left">
        <h1>Kniha Jázd</h1>
        <nav class="main-nav">
            <a href="/" class="nav-link" class:active={$page.url.pathname === '/'}>Kniha jázd</a>
            <a href="/settings" class="nav-link" class:active={$page.url.pathname === '/settings'}>Nastavenia</a>
        </nav>
    </div>
    <div class="vehicle-selector">
        <!-- existing vehicle selector -->
    </div>
</div>
```

**Step 2: Add page store import**

Add at top of script:
```typescript
import { page } from '$app/stores';
```

**Step 3: Add navigation styles**

Add to style block:
```css
.header-left {
    display: flex;
    align-items: center;
    gap: 2rem;
}

.main-nav {
    display: flex;
    gap: 0.5rem;
}

.nav-link {
    color: rgba(255, 255, 255, 0.7);
    text-decoration: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-weight: 500;
    transition: all 0.2s;
}

.nav-link:hover {
    color: white;
    background: rgba(255, 255, 255, 0.1);
}

.nav-link.active {
    color: white;
    background: rgba(255, 255, 255, 0.2);
}
```

**Step 4: Run and verify**

```bash
npm run tauri dev
```

Expected: Navigation links visible in header, active state highlights current page.

**Step 5: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat(ui): add navigation links to header"
```

---

## Task 2: Remove Settings Link from Settings Page

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Remove back link from settings header**

Remove or simplify the header section that contains the back link:

```svelte
<div class="header">
    <h1>Nastavenia</h1>
</div>
```

Remove the `.back-link` style rule as it's no longer needed.

**Step 2: Run and verify**

```bash
npm run tauri dev
```

Navigate to Settings - no duplicate back link.

**Step 3: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "refactor(ui): remove redundant back link from settings"
```

---

## Task 3: Redesign Totals Section - Two Rows

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: Restructure stats display**

Replace the single-row stats with two-row layout:

```svelte
{#if stats}
    <div class="stats-container">
        <div class="stats-row">
            <span class="stat">
                <span class="stat-label">Celkovo najazdené:</span>
                <span class="stat-value">{stats.total_km.toLocaleString('sk-SK')} km</span>
            </span>
            <span class="stat">
                <span class="stat-label">PHM:</span>
                <span class="stat-value">{stats.total_fuel_liters.toFixed(1)} L / {stats.total_fuel_cost_eur.toFixed(2)} €</span>
            </span>
        </div>
        <div class="stats-row">
            <span class="stat">
                <span class="stat-label">Spotreba:</span>
                <span class="stat-value">{stats.avg_consumption_rate.toFixed(2)} L/100km</span>
            </span>
            {#if stats.margin_percent !== null}
                <span class="stat" class:warning={stats.is_over_limit}>
                    <span class="stat-label">Odchýlka:</span>
                    <span class="stat-value">{stats.margin_percent.toFixed(1)}%</span>
                </span>
            {/if}
            <span class="stat">
                <span class="stat-label">Zostatok:</span>
                <span class="stat-value">{stats.zostatok_liters.toFixed(1)} L</span>
            </span>
        </div>
    </div>
{/if}
```

**Step 2: Update styles**

Replace `.stats` with new styles:

```css
.stats-container {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.stats-row {
    display: flex;
    gap: 1.5rem;
    align-items: center;
    font-size: 0.875rem;
}

.stat {
    display: flex;
    gap: 0.25rem;
}

.stat-label {
    color: #7f8c8d;
}

.stat-value {
    font-weight: 600;
    color: #2c3e50;
}

.stat.warning .stat-value {
    color: #d39e00;
}
```

Remove old `.stats`, `.stat`, `.stat-separator` styles.

**Step 3: Update vehicle-header layout**

Change from side-by-side to stacked:

```css
.vehicle-header {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1rem;
}
```

**Step 4: Run and verify**

```bash
npm run tauri dev
```

Expected: Stats now in two rows, "Celkovo najazdené" label visible.

**Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat(ui): redesign totals section with two rows"
```

---

## Task 4: Remove Settings Button from Main Page

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: Remove actions section**

Remove the entire actions div at the bottom:

```svelte
<!-- Remove this entire block -->
<div class="actions">
    <a href="/settings" class="button">Nastavenia</a>
</div>
```

**Step 2: Remove unused styles**

Remove `.actions` and `.button` styles (button styles may still be needed - check usage).

**Step 3: Run and verify**

```bash
npm run tauri dev
```

Expected: No settings button at bottom of main page.

**Step 4: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "refactor(ui): remove redundant settings button from main page"
```

---

## Task 5: Add Backup Types to TypeScript

**Files:**
- Modify: `src/lib/types.ts`

**Step 1: Add BackupInfo type**

```typescript
export interface BackupInfo {
    filename: string;
    created_at: string;
    size_bytes: number;
    vehicle_count: number;
    trip_count: number;
}
```

**Step 2: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(types): add BackupInfo interface"
```

---

## Task 6: Add Backup Rust Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add backup imports at top of commands.rs**

```rust
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
```

**Step 2: Add BackupInfo struct**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub vehicle_count: i32,
    pub trip_count: i32,
}
```

**Step 3: Add create_backup command**

```rust
#[tauri::command]
pub fn create_backup(app: tauri::AppHandle, db: State<Database>) -> Result<BackupInfo, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_dir = app_dir.join("backups");

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    // Generate backup filename with timestamp
    let timestamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S");
    let filename = format!("kniha-jazd-backup-{}.db", timestamp);
    let backup_path = backup_dir.join(&filename);

    // Copy current database to backup
    let db_path = app_dir.join("kniha-jazd.db");
    fs::copy(&db_path, &backup_path).map_err(|e| e.to_string())?;

    // Get file size
    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Get counts from current database
    let vehicles = db.get_all_vehicles().map_err(|e| e.to_string())?;
    let vehicle_count = vehicles.len() as i32;

    // Count trips across all vehicles
    let mut trip_count = 0;
    for vehicle in &vehicles {
        let trips = db.get_trips_for_vehicle(&vehicle.id.to_string()).map_err(|e| e.to_string())?;
        trip_count += trips.len() as i32;
    }

    Ok(BackupInfo {
        filename,
        created_at: chrono::Local::now().to_rfc3339(),
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
    })
}
```

**Step 4: Add list_backups command**

```rust
#[tauri::command]
pub fn list_backups(app: tauri::AppHandle) -> Result<Vec<BackupInfo>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_dir = app_dir.join("backups");

    if !backup_dir.exists() {
        return Ok(vec![]);
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().map(|e| e == "db").unwrap_or(false) {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;

            // Parse timestamp from filename: kniha-jazd-backup-YYYY-MM-DD-HHMMSS.db
            let created_at = if filename.starts_with("kniha-jazd-backup-") {
                let date_part = filename
                    .trim_start_matches("kniha-jazd-backup-")
                    .trim_end_matches(".db");
                // Convert YYYY-MM-DD-HHMMSS to ISO format
                if date_part.len() >= 17 {
                    format!(
                        "{}-{}-{}T{}:{}:{}",
                        &date_part[0..4],
                        &date_part[5..7],
                        &date_part[8..10],
                        &date_part[11..13],
                        &date_part[13..15],
                        &date_part[15..17]
                    )
                } else {
                    chrono::Local::now().to_rfc3339()
                }
            } else {
                chrono::Local::now().to_rfc3339()
            };

            // We can't easily get counts from backup without opening it
            // So we'll return 0 for now - the restore command will show actual counts
            backups.push(BackupInfo {
                filename,
                created_at,
                size_bytes: metadata.len(),
                vehicle_count: 0,
                trip_count: 0,
            });
        }
    }

    // Sort by filename descending (newest first)
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));

    Ok(backups)
}
```

**Step 5: Add get_backup_info command (for restore confirmation)**

```rust
#[tauri::command]
pub fn get_backup_info(app: tauri::AppHandle, filename: String) -> Result<BackupInfo, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_path = app_dir.join("backups").join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    let metadata = fs::metadata(&backup_path).map_err(|e| e.to_string())?;

    // Open backup database to get counts
    let conn = rusqlite::Connection::open(&backup_path).map_err(|e| e.to_string())?;

    let vehicle_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM vehicles", [], |row| row.get(0))
        .unwrap_or(0);

    let trip_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM trips", [], |row| row.get(0))
        .unwrap_or(0);

    // Parse timestamp from filename
    let created_at = if filename.starts_with("kniha-jazd-backup-") {
        let date_part = filename
            .trim_start_matches("kniha-jazd-backup-")
            .trim_end_matches(".db");
        if date_part.len() >= 17 {
            format!(
                "{}-{}-{}T{}:{}:{}",
                &date_part[0..4],
                &date_part[5..7],
                &date_part[8..10],
                &date_part[11..13],
                &date_part[13..15],
                &date_part[15..17]
            )
        } else {
            chrono::Local::now().to_rfc3339()
        }
    } else {
        chrono::Local::now().to_rfc3339()
    };

    Ok(BackupInfo {
        filename,
        created_at,
        size_bytes: metadata.len(),
        vehicle_count,
        trip_count,
    })
}
```

**Step 6: Add restore_backup command**

```rust
#[tauri::command]
pub fn restore_backup(app: tauri::AppHandle, filename: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_path = app_dir.join("backups").join(&filename);
    let db_path = app_dir.join("kniha-jazd.db");

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    // Copy backup over current database
    fs::copy(&backup_path, &db_path).map_err(|e| e.to_string())?;

    Ok(())
}
```

**Step 7: Register commands in lib.rs**

Add to invoke_handler:
```rust
commands::create_backup,
commands::list_backups,
commands::get_backup_info,
commands::restore_backup,
```

**Step 8: Build to verify**

```bash
cd src-tauri && cargo build
```

**Step 9: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(backend): add backup/restore commands"
```

---

## Task 7: Add Backup API Functions

**Files:**
- Modify: `src/lib/api.ts`

**Step 1: Add backup API functions**

```typescript
import type { Vehicle, Trip, Route, CompensationSuggestion, Settings, TripStats, BackupInfo } from './types';

// ... existing code ...

// Backup commands
export async function createBackup(): Promise<BackupInfo> {
    return await invoke('create_backup');
}

export async function listBackups(): Promise<BackupInfo[]> {
    return await invoke('list_backups');
}

export async function getBackupInfo(filename: string): Promise<BackupInfo> {
    return await invoke('get_backup_info', { filename });
}

export async function restoreBackup(filename: string): Promise<void> {
    return await invoke('restore_backup', { filename });
}
```

**Step 2: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add backup/restore API functions"
```

---

## Task 8: Add Backup Section to Settings Page

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Add backup state variables**

In script section:
```typescript
import type { Vehicle, Settings, BackupInfo } from '$lib/types';

// Add backup state
let backups: BackupInfo[] = [];
let loadingBackups = false;
let backupInProgress = false;
let restoreConfirmation: BackupInfo | null = null;
```

**Step 2: Load backups on mount**

Add to onMount:
```typescript
// Load backups
await loadBackups();
```

**Step 3: Add backup functions**

```typescript
async function loadBackups() {
    loadingBackups = true;
    try {
        backups = await api.listBackups();
    } catch (error) {
        console.error('Failed to load backups:', error);
    } finally {
        loadingBackups = false;
    }
}

async function handleCreateBackup() {
    backupInProgress = true;
    try {
        const backup = await api.createBackup();
        await loadBackups();
        alert(`Záloha vytvorená: ${backup.filename}`);
    } catch (error) {
        console.error('Failed to create backup:', error);
        alert('Nepodarilo sa vytvoriť zálohu: ' + error);
    } finally {
        backupInProgress = false;
    }
}

async function handleRestoreClick(backup: BackupInfo) {
    try {
        // Get full backup info with counts
        restoreConfirmation = await api.getBackupInfo(backup.filename);
    } catch (error) {
        console.error('Failed to get backup info:', error);
        alert('Nepodarilo sa načítať informácie o zálohe: ' + error);
    }
}

async function handleConfirmRestore() {
    if (!restoreConfirmation) return;

    try {
        await api.restoreBackup(restoreConfirmation.filename);
        restoreConfirmation = null;
        alert('Záloha bola úspešne obnovená. Aplikácia sa reštartuje.');
        // Reload the app to pick up restored data
        window.location.reload();
    } catch (error) {
        console.error('Failed to restore backup:', error);
        alert('Nepodarilo sa obnoviť zálohu: ' + error);
    }
}

function cancelRestore() {
    restoreConfirmation = null;
}

function formatBackupDate(isoDate: string): string {
    try {
        const date = new Date(isoDate);
        return date.toLocaleString('sk-SK', {
            day: '2-digit',
            month: '2-digit',
            year: 'numeric',
            hour: '2-digit',
            minute: '2-digit'
        });
    } catch {
        return isoDate;
    }
}

function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
```

**Step 4: Add Backup section HTML after Export section**

```svelte
<!-- Backup Section -->
<section class="settings-section">
    <h2>Záloha databázy</h2>
    <div class="section-content">
        <button class="button" on:click={handleCreateBackup} disabled={backupInProgress}>
            {backupInProgress ? 'Vytváram zálohu...' : 'Zálohovať'}
        </button>

        <div class="backup-list">
            <h3>Dostupné zálohy</h3>
            {#if loadingBackups}
                <p class="placeholder">Načítavam...</p>
            {:else if backups.length === 0}
                <p class="placeholder">Žiadne zálohy. Vytvorte prvú zálohu.</p>
            {:else}
                {#each backups as backup}
                    <div class="backup-item">
                        <div class="backup-info">
                            <span class="backup-date">{formatBackupDate(backup.created_at)}</span>
                            <span class="backup-size">{formatFileSize(backup.size_bytes)}</span>
                        </div>
                        <button class="button-small" on:click={() => handleRestoreClick(backup)}>
                            Obnoviť
                        </button>
                    </div>
                {/each}
            {/if}
        </div>
    </div>
</section>

<!-- Restore Confirmation Modal -->
{#if restoreConfirmation}
    <div class="modal-overlay" on:click={cancelRestore}>
        <div class="modal" on:click|stopPropagation>
            <h2>Potvrdiť obnovenie</h2>
            <div class="modal-content">
                <p><strong>Dátum zálohy:</strong> {formatBackupDate(restoreConfirmation.created_at)}</p>
                <p><strong>Veľkosť:</strong> {formatFileSize(restoreConfirmation.size_bytes)}</p>
                <p><strong>Obsahuje:</strong> {restoreConfirmation.vehicle_count} vozidiel, {restoreConfirmation.trip_count} jázd</p>
                <p class="warning-text">
                    Aktuálne dáta budú prepísané! Táto akcia sa nedá vrátiť späť.
                </p>
            </div>
            <div class="modal-actions">
                <button class="button-small" on:click={cancelRestore}>Zrušiť</button>
                <button class="button-small danger" on:click={handleConfirmRestore}>Obnoviť zálohu</button>
            </div>
        </div>
    </div>
{/if}
```

**Step 5: Add backup styles**

```css
.backup-list {
    margin-top: 1rem;
}

.backup-list h3 {
    font-size: 1rem;
    color: #2c3e50;
    margin: 0 0 0.75rem 0;
}

.backup-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    background: #fafafa;
    margin-bottom: 0.5rem;
}

.backup-info {
    display: flex;
    gap: 1rem;
}

.backup-date {
    font-weight: 500;
    color: #2c3e50;
}

.backup-size {
    color: #7f8c8d;
    font-size: 0.875rem;
}

.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.modal {
    background: white;
    padding: 1.5rem;
    border-radius: 8px;
    max-width: 400px;
    width: 90%;
}

.modal h2 {
    margin: 0 0 1rem 0;
    font-size: 1.25rem;
    color: #2c3e50;
}

.modal-content {
    margin-bottom: 1.5rem;
}

.modal-content p {
    margin: 0.5rem 0;
}

.warning-text {
    color: #c0392b;
    font-weight: 500;
    margin-top: 1rem !important;
    padding: 0.75rem;
    background: #fee;
    border-radius: 4px;
}

.modal-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
}
```

**Step 6: Run and verify**

```bash
npm run tauri dev
```

Expected:
- Backup section visible in Settings
- Can create backup
- Backups listed with date and size
- Restore shows confirmation with counts and warning

**Step 7: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(ui): add backup/restore section to settings"
```

---

## Task 9: Final Testing & Commit

**Step 1: Full test of all features**

```bash
npm run tauri dev
```

Test checklist:
- [ ] Navigation links work (Kniha jázd / Nastavenia)
- [ ] Active state shows on current page
- [ ] Totals in two rows with "Celkovo najazdené" label
- [ ] No duplicate Settings buttons
- [ ] Create backup works
- [ ] Backups listed correctly
- [ ] Restore confirmation shows counts
- [ ] Restore actually replaces data
- [ ] App reloads after restore

**Step 2: Run Rust tests**

```bash
cd src-tauri && cargo test
```

**Step 3: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: address any issues from testing"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add navigation to header | +layout.svelte |
| 2 | Remove back link from settings | settings/+page.svelte |
| 3 | Redesign totals to two rows | +page.svelte |
| 4 | Remove settings button from main | +page.svelte |
| 5 | Add BackupInfo type | types.ts |
| 6 | Add backup Rust commands | commands.rs, lib.rs |
| 7 | Add backup API functions | api.ts |
| 8 | Add backup UI to settings | settings/+page.svelte |
| 9 | Final testing | - |

Total: ~9 tasks, each with clear steps and commits.
