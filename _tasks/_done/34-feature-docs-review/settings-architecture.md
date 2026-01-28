# Review: settings-architecture.md

## Convention Compliance

**Overall Assessment:** PARTIALLY COMPLIANT - Good structure but excessive embedded code

The document follows the template structure well:
- Clear one-line description
- Logical flow from overview to implementation
- Key Files table present
- Design Decisions section with rationale

However, it violates the core convention principle of using `file.rs:L###` pointers instead of embedding full implementations.

## Issues Found

### Issue 1: Embedded Rust Implementation Code (CRITICAL)

The document embeds **6 substantial Rust code blocks** that should be references:

| Location | Embedded Code | Should Be |
|----------|---------------|-----------|
| Lines 50-56 | `BackupRetention` struct definition | `settings.rs:L##` reference |
| Lines 74-85 | `Settings::default()` impl | `models.rs:L##` reference |
| Lines 111-124 | `LocalSettings::load()` method | `settings.rs:L##` reference |
| Lines 127-134 | `get_settings()` method | `db.rs:L##` reference |
| Lines 139-148 | `LocalSettings::save()` method | `settings.rs:L##` reference |
| Lines 151-165 | `save_settings()` upsert pattern | `db.rs:L##` reference |

**Risk:** These code blocks will drift from actual implementation. When someone refactors `LocalSettings::load()`, they won't update this doc.

### Issue 2: Embedded TypeScript Code (MODERATE)

Two TypeScript blocks that should be references:

| Location | Embedded Code | Should Be |
|----------|---------------|-----------|
| Lines 173-189 | `onMount()` loading pattern | `+page.svelte:L##` reference |
| Lines 193-195 | Debounce setup | `+page.svelte:L##` reference |

### Issue 3: Interface Definition Embedding (MINOR)

Lines 34-40 embed the `ReceiptSettings` TypeScript interface. This should reference `types.ts:L##`.

### Issue 4: JSON Sample is OK

The sample `local.settings.json` at lines 258-270 is acceptable - it shows configuration format which is stable and user-facing.

## What Should STAY

Per convention, these are appropriate to keep:

1. **Tables with field descriptions** (Lines 24-31, 65-70) - These document the schema, not implementation
2. **Command tables** (Lines 200-213, 216-220) - API surface documentation
3. **Data flow diagrams** - None present, but would be OK if added
4. **Design rationale** - The "Why" sections are valuable

## Recommendations

### 1. Replace Rust Code Blocks with References

**Before:**
```markdown
**LocalSettings** (from file):
\`\`\`rust
impl LocalSettings {
    pub fn load(app_data_dir: &PathBuf) -> Self {
        // ... 10 lines of implementation
    }
}
\`\`\`
```

**After:**
```markdown
**LocalSettings loading:** See `settings.rs:L45-55` for the `load()` method that reads from JSON with fallback to defaults.
```

### 2. Replace TypeScript Blocks with References

**Before:**
```markdown
\`\`\`typescript
onMount(async () => {
    // ... 15 lines of loading code
});
\`\`\`
```

**After:**
```markdown
The Settings page loads both setting types on mount - see `+page.svelte:L##` for the unified loading pattern.
```

### 3. Keep Struct Field Tables, Remove Default Impl

The field tables are great documentation. The `impl Default` code block should become:

```markdown
**Defaults:** `buffer_trip_purpose` defaults to "sluzobna cesta" (see `models.rs:L##`)
```

### 4. Add Line Numbers to Key Files Table

The Key Files table lists files but could benefit from function references:

| File | Purpose | Key Locations |
|------|---------|---------------|
| settings.rs | LocalSettings struct | `load()` L45, `save()` L60 |
| db.rs | Database operations | `get_settings()` L120, `save_settings()` L135 |

## Summary

| Metric | Status |
|--------|--------|
| Template structure | PASS |
| Math formulas | N/A (none present) |
| Code references vs embedding | FAIL - 8 embedded blocks should be references |
| Design rationale | PASS |
| Key Files table | PASS (could add line numbers) |
| Documentation drift risk | HIGH - embedded code will become stale |

**Effort to fix:** LOW - Replace code blocks with references, keep prose explanations
