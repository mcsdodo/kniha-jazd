# Review: backup-system.md

## Convention Compliance

**Overall:** Partial compliance. The document follows the template structure well but violates the "reference code, don't embed" principle in several places.

### Compliant Areas
- User Flow section is clear and well-structured
- Technical Implementation describes what happens without duplicating logic
- Key Files table uses proper format
- Design Decisions section explains rationale well
- API Functions table is appropriate (stable interface documentation)

### Non-Compliant Areas
- 4 embedded code blocks that should be line references
- Data structures could be referenced rather than duplicated

## Issues Found

### Issue 1: Embedded Rust code for filename generation (lines 31-39)

**Current:**
```rust
// Filename generation
fn generate_backup_filename(backup_type: &str, update_version: Option<&str>) -> String {
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    match (backup_type, update_version) {
        ("pre-update", Some(version)) => format!("kniha-jazd-backup-{}-pre-v{}.db", timestamp, version),
        _ => format!("kniha-jazd-backup-{}.db", timestamp),
    }
}
```

**Should be:**
> Filename generation: `commands.rs:L1597-1605`

### Issue 2: Embedded TypeScript code for pre-update trigger (lines 67-70)

**Current:**
```typescript
// In update store install() method
const { createBackupWithType } = await import('$lib/api');
await createBackupWithType('pre-update', updateObject.version);
```

**Should be:**
> Pre-update backup triggered from update store during installation. See `update.ts` import and `createBackupWithType()` call.

### Issue 3: Embedded TypeScript interface (lines 80-84)

**Current:**
```typescript
interface BackupRetention {
  enabled: boolean;
  keepCount: number; // 3, 5, or 10
}
```

**Should be:**
> Retention settings stored in `local.settings.json`. See `settings.rs:BackupRetention` struct.

### Issue 4: Embedded Rust code for startup cleanup (lines 93-98)

**Current:**
```rust
// Runs in background thread at app startup
if retention.enabled && retention.keep_count > 0 {
    commands::cleanup_pre_update_backups_internal(&cleanup_app_handle, retention.keep_count);
}
```

**Should be:**
> Startup auto-cleanup runs in background thread. See `lib.rs:L140-146`.

### Issue 5: Full Data Structures section (lines 104-128)

The entire Data Structures section embeds TypeScript interfaces. These are prone to drift as the codebase evolves.

**Should be:**
- Reference `types.ts` for TypeScript interfaces
- Reference `commands.rs` for Rust structs (BackupInfo around L1624-1630)

Or keep as-is if these are considered stable API contracts (like math formulas). The convention says "Math formulas are OK (stable)" but data structures may change.

## Recommendations

### High Priority (drift risk)

1. **Replace all 4 embedded code blocks with line references:**
   - `commands.rs:L1597-1605` for filename generation
   - `lib.rs:L140-146` for startup cleanup
   - Remove TypeScript snippets, describe behavior instead

2. **Simplify Data Structures section:**
   - Either remove entirely (reference types.ts)
   - Or keep as stable API documentation with note: "See types.ts for current definitions"

### Low Priority (style improvements)

3. **Add line numbers to Key Files table:**
   - `commands.rs:L1632+` for backup commands
   - `settings.rs:L###` for BackupRetention struct
   - `lib.rs:L140-146` for startup cleanup

4. **Link to Related section:**
   - Link to move-database.md (backup location follows database)
   - Link to any ADR about backup design if exists

## Summary

| Category | Count |
|----------|-------|
| Embedded code blocks to remove | 4 |
| Data structures to reference | 2 |
| Drift risk | Medium |

The document is well-written and informative, but needs refactoring to follow the "reference, don't embed" convention. The main risk is that embedded code will become stale as the implementation evolves.
