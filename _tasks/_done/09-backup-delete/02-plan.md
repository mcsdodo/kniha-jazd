**Date:** 2025-12-26
**Subject:** Delete backup implementation plan
**Status:** Complete

## Implementation Steps

### 1. Backend: Add delete_backup command

**File:** `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub fn delete_backup(app: tauri::AppHandle, filename: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_path = app_dir.join("backups").join(&filename);

    if !backup_path.exists() {
        return Err(format!("Backup not found: {}", filename));
    }

    fs::remove_file(&backup_path).map_err(|e| e.to_string())?;
    Ok(())
}
```

**File:** `src-tauri/src/lib.rs`
- Register `delete_backup` in the invoke_handler

### 2. Frontend API

**File:** `src/lib/api.ts`

```typescript
export async function deleteBackup(filename: string): Promise<void> {
    return await invoke('delete_backup', { filename });
}
```

### 3. UI Changes

**File:** `src/routes/settings/+page.svelte`

1. Add state: `let deleteConfirmation: BackupInfo | null = null;`

2. Add handler functions:
   - `handleDeleteClick(backup)` - sets deleteConfirmation
   - `handleConfirmDelete()` - calls API, reloads list
   - `cancelDelete()` - clears deleteConfirmation

3. Add delete button in backup list (next to "Obnoviť")

4. Add confirmation modal (reuse pattern from restore modal)

## Testing

- Manual test: create backup, delete it, verify file removed
- Error case: verify error shown if deletion fails

## Files Changed

1. `src-tauri/src/commands.rs` - add delete_backup command
2. `src-tauri/src/lib.rs` - register command
3. `src/lib/api.ts` - add deleteBackup function
4. `src/routes/settings/+page.svelte` - add UI
