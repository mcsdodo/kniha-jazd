# Review: read-only-mode.md

## Convention Compliance

**Overall Assessment:** PARTIAL COMPLIANCE - Document structure is good but contains embedded code blocks that should be converted to file:line references.

| Convention | Status | Notes |
|------------|--------|-------|
| User Flow section | OK | Clear step-by-step flow |
| Technical Implementation | NEEDS FIX | Contains 2 embedded code blocks |
| Key Files table | OK | Has proper file paths |
| Design Decisions | OK | Good rationale with no code |
| Math formulas | N/A | None in this doc |
| Code references vs embedding | NEEDS FIX | 2 code blocks should be references |

## Issues Found

### Issue 1: Embedded Heartbeat Thread Code (Lines 63-72)

**Current:** Full Rust code block showing heartbeat loop
```rust
std::thread::spawn(move || {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        if let Err(e) = db_location::refresh_lock(&heartbeat_lock_path) {
            log::warn!("Failed to refresh lock: {}", e);
            break;
        }
    }
});
```

**Problem:** This implementation lives in `lib.rs` and could drift from documentation. The doc should reference the source.

**Fix:** Replace with reference:
```markdown
**Implementation**: See heartbeat thread spawn in `lib.rs:L###` (search for `std::thread::spawn`)

**Interval**: 30 seconds
**Purpose**: Updates `last_heartbeat` timestamp to prove the app is still running
```

### Issue 2: Embedded Migration Compatibility Code (Lines 84-94)

**Current:** Pseudo-code showing migration compatibility check
```rust
pub fn check_migration_compatibility(&self) -> Result<(), Vec<String>> {
    let embedded = Self::get_embedded_migration_versions();
    let applied = /* query __diesel_schema_migrations table */;

    let unknown: Vec<String> = applied
        .filter(|m| !embedded.contains(&m.version))
        .collect();

    if unknown.is_empty() { Ok(()) } else { Err(unknown) }
}
```

**Problem:** This is pseudo-code (note the comment placeholder). While it explains the algorithm, embedding code (even pseudo-code) risks drift and contradicts the "reference, don't embed" principle.

**Fix:** Replace with reference:
```markdown
**Implementation**: See `check_migration_compatibility()` in `db.rs:L###`

**Algorithm**:
1. Get list of migrations compiled into this app version
2. Query `__diesel_schema_migrations` table for applied migrations
3. Find any applied migrations not in the embedded list
4. If unknown migrations exist, return error (triggers read-only mode)
```

### Issue 3: Lock File JSON Format (Lines 39-45)

**Current:** JSON example of lock file format
```json
{
  "pc_name": "DESKTOP-ABC123",
  "opened_at": "2025-01-25T10:00:00Z",
  "last_heartbeat": "2025-01-25T10:05:30Z",
  "app_version": "0.21.0",
  "pid": 12345
}
```

**Assessment:** ACCEPTABLE - This is data format documentation, not implementation code. Lock file format is essentially a "protocol" that should be documented. Similar to how API docs show request/response formats.

### Issue 4: Missing Line Numbers in Key Files

**Current:** Key Files table uses relative links but no line numbers
```markdown
| [src-tauri/src/app_state.rs](../../src-tauri/src/app_state.rs) | `AppMode` enum... |
```

**Problem:** No `file.rs:L###` format as specified in conventions.

**Fix:** Add line references for key locations:
```markdown
| File | Purpose | Key Lines |
|------|---------|-----------|
| `app_state.rs` | `AppMode` enum, `AppState` struct | `AppMode:L##`, `is_read_only():L##` |
| `db_location.rs` | Lock operations | `check_lock:L##`, `acquire_lock:L##` |
| `commands.rs` | `check_read_only!` macro | `L##` |
| `db.rs` | Migration compatibility | `check_migration_compatibility:L##` |
```

## Recommendations

### Priority 1: Remove Embedded Code Blocks

1. **Heartbeat thread** (L63-72): Replace with `lib.rs:L###` reference + prose description
2. **Migration check** (L84-94): Replace with `db.rs:L###` reference + algorithm bullets

### Priority 2: Add Line Numbers

Update Key Files table to include specific line numbers for:
- `AppMode` enum definition
- `check_read_only!` macro definition
- `check_migration_compatibility()` function
- `check_lock()`, `acquire_lock()` functions

### Priority 3: Keep What's Good

The following should be preserved as-is:
- Lock file JSON format (protocol documentation)
- Lock Status Resolution table (decision logic, not code)
- Design Decisions section (excellent rationale)
- Error message format example (user-facing, stable)

## Summary

The document is well-structured and provides excellent design rationale. The main issue is two embedded code blocks that should be converted to file:line references with prose descriptions. The lock file JSON format is acceptable as protocol documentation.

**Estimated effort to fix:** ~15 minutes (locate actual line numbers, replace code blocks with references)
