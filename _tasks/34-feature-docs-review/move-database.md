# Review: move-database.md

## Convention Compliance

**Overall:** Partial compliance. The document follows the template structure well but violates the "no embedded code" convention in several places.

**What it does well:**
- Clear user flow section
- Good use of `file.rs:L###` line number references in Key Files table
- Design decisions are well-documented with rationale
- Related section links appropriately

**What violates conventions:**
- Multiple embedded code/pseudo-code blocks that duplicate implementation details
- Risk of documentation drift when these implementations change

## Issues Found

### Issue 1: Embedded TypeScript API signatures (Lines 34-37)

**Current:**
```markdown
**API Wrapper:** `src/lib/api.ts`
\`\`\`typescript
export async function moveDatabase(targetFolder: string): Promise<MoveDbResult>
export async function resetDatabaseLocation(): Promise<MoveDbResult>
\`\`\`
```

**Problem:** Function signatures can change (parameter names, return types). This duplicates what's already in the source file.

**Recommendation:** Replace with:
```markdown
**API Wrapper:** `src/lib/api.ts`
- `moveDatabase()` - Invokes backend move command
- `resetDatabaseLocation()` - Resets to default path
```

---

### Issue 2: Embedded 10-step process diagram (Lines 43-54)

**Current:** ASCII box with 9 numbered implementation steps

**Problem:** This is a pseudo-code representation of the `move_database` command implementation. If the order changes, steps are added/removed, or error handling is modified, this diagram becomes stale.

**Recommendation:** Either:
1. Remove entirely - the Data Flow section already covers this conceptually
2. Reduce to high-level phases only:
```markdown
**Main Command:** `move_database` in `commands.rs:L3168-3248`
- Validates permissions, copies files, updates settings, manages locks
```

---

### Issue 3: Detailed Data Flow with function names (Lines 67-95)

**Current:** Embedded ASCII diagram listing specific Rust functions like:
- `check_read_only!()`
- `fs::create_dir_all(target)`
- `fs::copy(db_file)`
- `copy_dir_all(backups/)`
- `LocalSettings::save()`
- `acquire_lock(new_path)`
- `release_lock(old_path)`
- `fs::remove_file(old_db)`
- `app_state.set_db_path()`

**Problem:** This duplicates the implementation in `commands.rs:L3168-3248`. If function names change, method signatures change, or steps are reordered, this becomes outdated.

**Recommendation:** Simplify to conceptual flow:
```markdown
### Data Flow

\`\`\`
User clicks "Change..." → Directory Picker → Frontend validation
        ↓
Confirmation Modal → invoke("move_database")
        ↓
Rust Backend: validate → copy files → update settings → manage locks → cleanup
        ↓
MoveDbResult → Frontend toast → App reload
\`\`\`

For implementation details, see `commands.rs:L3168-3248`.
```

---

### Issue 4: Embedded JSON lock file structure (Lines 112-120)

**Current:**
```json
{
  "pc_name": "DESKTOP-ABC123",
  "opened_at": "2024-01-15T10:30:00Z",
  "last_heartbeat": "2024-01-15T10:35:00Z",
  "app_version": "1.2.0",
  "pid": 12345
}
```

**Assessment:** This is **borderline acceptable**. Unlike code, data structures tend to be more stable and this helps users understand what the lock file contains without reading Rust code.

**Recommendation:** Keep, but add a reference:
```markdown
## Lock File Structure

Located at `<db_folder>/kniha-jazd.lock`. See `db_location.rs:L##` for `LockInfo` struct.

Example format:
```json
{...}
```
```

---

## Summary of Recommendations

| Section | Issue | Action |
|---------|-------|--------|
| Lines 34-37 | TypeScript signatures embedded | Replace with brief descriptions |
| Lines 43-54 | 10-step process diagram | Remove or reduce to one-liner |
| Lines 67-95 | Data flow with Rust function names | Simplify to conceptual diagram |
| Lines 112-120 | JSON lock structure | Keep, add code reference |

## Estimated Drift Risk

- **High:** Lines 43-54, 67-95 - These mirror implementation closely
- **Medium:** Lines 34-37 - API signatures rarely change but can
- **Low:** Lines 112-120 - Data format is stable

## Template Adherence Score

- Structure: 9/10 (follows template well)
- Code references vs embedding: 5/10 (too much embedding)
- Maintainability: 6/10 (several drift-prone sections)

**Overall: 7/10** - Good foundation, needs de-duplication of implementation details.
