# Plan Review: Database Backup Before Updates

**Date:** 2026-01-24
**Reviewed Plan:** `03-plan.md`
**Status:** Review Complete - 4 Iterations

---

## Iteration 1: Initial Review

### Critical Findings

**C1. Task 6: Missing `set_backup_retention` read-only guard**
- Location: Task 6, Step 2
- Issue: `set_backup_retention` command modifies settings but is missing `check_read_only!(app_state)` guard
- Fix: Add `app_state: State<AppState>` parameter and `check_read_only!(app_state);` at start
- Severity: Critical - write command must be blocked in read-only mode

**C2. Task 3/6: BackupRetention not exported from settings.rs**
- Location: Task 3 Step 3 imports `use crate::settings::{LocalSettings, BackupRetention};`
- Issue: `BackupRetention` struct is defined in Task 2 in `settings.rs` but never marked `pub`
- Fix: In Task 2, ensure `pub struct BackupRetention` (the plan shows it but easy to miss)
- Severity: Critical - Rust won't compile without public visibility

**C3. Task 11: Missing internal cleanup function**
- Location: Task 11, Step 2 calls `cleanup_pre_update_backups_internal(&app_clone, keep_count)`
- Issue: This function doesn't exist. The plan only defines `cleanup_pre_update_backups` as a Tauri command with `State<AppState>` parameter
- Fix: Either (a) extract logic into `_internal` helper called by both, or (b) use different approach for startup cleanup
- Severity: Critical - code won't compile

### Important Findings

**I1. Task 2: Test file location mismatch**
- Location: Task 2 says tests go in "settings.rs tests module"
- Issue: Per CLAUDE.md, tests should be in `settings_tests.rs` companion file, not inline
- Actual: Existing `settings.rs` has tests inline. Both patterns exist in codebase.
- Recommendation: Keep inline for consistency with existing file, but note the deviation

**I2. Task 3: Test file location unclear**
- Location: Task 3 Step 1 says "Add test in `commands.rs` tests section (or create `commands_backup_tests.rs`)"
- Issue: `commands.rs` has 36 tests inline. The plan should be explicit about location.
- Recommendation: Add tests inline in `commands.rs` for consistency with existing pattern there

**I3. Task 5: Missing API functions for retention settings**
- Location: Task 5 shows `getBackupRetention()` and `setBackupRetention()` API functions
- Issue: These call commands `get_backup_retention` and `set_backup_retention` which are defined in Task 6
- Recommendation: Task 5 depends on Task 6, but comes before it. Reorder or note dependency clearly.
- Severity: Important - dependency ordering issue

**I4. Task 7: i18n key structure doesn't match existing pattern**
- Location: Task 7 shows keys under `backupRetention` and `backupBadge` as top-level
- Issue: Existing pattern nests backup-related keys under `backup` (see line 254) or `settings` (see line 198)
- Fix: Use `backup.retention.*` or `settings.backupRetention.*` to match conventions
- Severity: Important - inconsistent i18n structure

**I5. Task 7: Missing i18n key `toast.cleanupComplete`**
- Location: Task 8 Step 3 uses `toast.success($LL.toast.cleanupComplete())`
- Issue: This key is not defined in Task 7 translations
- Fix: Add to Task 7: `cleanupComplete: () => 'Zálohy boli vyčistené'`
- Severity: Important - runtime error if key missing

**I6. Task 9: `continueWithoutBackup` references missing download code**
- Location: Task 9 Step 3 shows `continueWithoutBackup` with comment "... same download code ..."
- Issue: The actual download code from existing `install()` function needs to be refactored/shared
- Recommendation: Plan should show explicit refactor to extract download logic into reusable function
- Severity: Important - unclear implementation

**I7. Task 11: `should_run_cleanup` calls `list_backups` without State**
- Location: Task 11 shows `list_backups(app.clone())`
- Issue: Actual `list_backups` signature is `list_backups(app: tauri::AppHandle)` - this is correct, but needs verification that the function can work without State<Database>
- Actual check: Looking at commands.rs, `list_backups` only needs `AppHandle` - OK
- Severity: Minor verification needed - appears OK

### Minor Findings

**M1. Task 1: No test for BackupInfo struct changes**
- Location: Task 1 says "Run backend tests to verify no regressions"
- Issue: No explicit test verifies the new fields serialize correctly
- Recommendation: Add JSON serialization test for BackupInfo with new fields
- Severity: Minor - existing tests may cover indirectly

**M2. Task 12: Integration test selectors too generic**
- Location: Task 12 uses `$('input[type="checkbox"]')` and `$('select')`
- Issue: Settings page has multiple checkboxes and selects. Test may select wrong element.
- Fix: Add `data-testid` attributes or use more specific selectors like `[data-testid="retention-enabled"]`
- Severity: Minor - may cause flaky tests

**M3. Task 8: Missing `formatFileSize` function**
- Location: Task 8 Step 4 uses `formatFileSize(cleanupPreview.totalBytes)`
- Issue: This utility function may not exist
- Check: Need to verify if this exists in codebase
- Severity: Minor - easy to implement if missing

**M4. Task 10: Update modal references non-existent translations**
- Location: Task 10 uses `$LL.update.downloading()`, `$LL.update.installing()`
- Check: These exist at lines 513 and 515 - OK
- Severity: None (verified OK)

---

## Iteration 2: Dependency and Order Analysis

### Critical Findings (New)

**C4. Task 3/11: `get_app_data_dir` called with reference mismatch**
- Location: Task 11 calls `get_app_data_dir(app)` where `app` is `&tauri::AppHandle`
- Issue: Existing `get_app_data_dir` takes `&tauri::AppHandle` - this matches. OK.
- Verification: Checked commands.rs line 50 - signature is correct
- Severity: Withdrawn - verified OK

### Important Findings (New)

**I8. Task 4: Test function signature mismatch**
- Location: Task 4 Step 1 shows test with `get_cleanup_candidates(&backups, 2)` where backups is `Vec<(&str, &str)>`
- Issue: The helper takes tuples, but actual implementation in Step 3 shows different signature
- Reality: Step 3 implementation matches step 1 test - OK but confusing
- Severity: Minor clarification needed

**I9. Task 11: Startup cleanup runs unconditionally**
- Location: Task 11 checks if `latest_pre_update.update_version == current_app_version`
- Issue: This logic has a subtle bug - after cleanup, the most recent backup might not match anymore, causing repeated cleanup attempts on subsequent startups
- Fix: The logic should work because cleanup keeps N recent backups, so the most recent should still match
- Verification: Logic appears sound - if you update to v0.20.0, create backup "pred-v0.20.0", then cleanup keeps latest N including this one
- Severity: Minor - logic is actually correct on review

---

## Iteration 3: Rust Compilation Verification

### Critical Findings (New)

**C5. Task 3: `Local::now()` import missing**
- Location: Task 3 Step 6 uses `Local::now().format(...)`
- Issue: `Local` is from `chrono` crate. Need to verify import exists.
- Check: commands.rs line 16 has `use chrono::{Datelike, NaiveDate, Utc, Local};` - OK
- Severity: Withdrawn - import exists

**C6. Task 6: `LocalSettings::save` error handling**
- Location: Task 6 Step 2 shows `.map_err(|e| e.to_string())`
- Issue: `save()` returns `std::io::Result<()>`, so `e.to_string()` works. OK.
- Severity: Withdrawn - verified OK

### Important Findings (New)

**I10. Task 5: Type naming inconsistency**
- Location: Task 5 types use `backupType: 'manual' | 'pre-update'`
- Issue: Rust struct uses `String`, so frontend should accept any string or define union type
- Recommendation: Use TypeScript string literal union as shown (acceptable pattern)
- Severity: Minor

---

## Iteration 4: Final Edge Case Review

### Important Findings (New)

**I11. Edge case: First update ever (no pre-update backups)**
- Location: Task 11 `should_run_cleanup`
- Issue: If no pre-update backups exist, `max_by` returns None, function returns None - correct behavior
- Status: Handled correctly via `?` operator
- Severity: None - already handled

**I12. Edge case: Backup failure at startup**
- Location: Not covered in plan
- Issue: What if backup directory doesn't exist or disk is full during update backup?
- Status: Task 9 handles failure gracefully with user prompt - adequate
- Severity: None - already handled

**I13. Edge case: Empty backup list display**
- Location: Task 8 UI
- Issue: Need to handle case where no backups exist yet
- Status: Existing backup list UI handles empty state. Task 8 cleanup preview shows "nothingToClean"
- Severity: None - adequately covered

---

## Summary of Findings

### Critical (3 issues - blocks implementation)
1. **C1**: `set_backup_retention` missing read-only guard
2. **C2**: `BackupRetention` struct visibility (likely `pub` but worth verifying)
3. **C3**: Missing `cleanup_pre_update_backups_internal` function for startup cleanup

### Important (6 issues - should fix before implementation)
1. **I3**: Task 5 depends on Task 6 (reorder or document dependency)
2. **I4**: i18n key structure should nest under `backup.*` or `settings.*`
3. **I5**: Missing `toast.cleanupComplete` i18n key
4. **I6**: `continueWithoutBackup` needs explicit download code refactor
5. **I8**: Test/implementation signature in Task 4 slightly confusing
6. **I10**: TypeScript types should be consistent with Rust serialization

### Minor (3 issues - nice to have)
1. **M1**: Add BackupInfo serialization test for new fields
2. **M2**: Integration test selectors too generic - add data-testid
3. **M3**: Verify `formatFileSize` utility exists

---

## Checklist Verification

- [x] Each task has exact file paths - **YES**, paths are specific
- [x] Each task has verification steps - **YES**, each has "Run tests" step
- [x] Tasks are in correct dependency order - **PARTIAL**, Task 5/6 dependency issue (I3)
- [x] No scope creep beyond design - **YES**, matches design doc
- [ ] i18n keys match existing patterns - **NO**, needs nesting fix (I4)
- [x] New commands registered in lib.rs - **YES**, Tasks 3, 4, 6 mention registration
- [x] Integration tests follow tier2 patterns - **PARTIAL**, selectors too generic (M2)

---

## Recommendation

**READY WITH FIXES**: The plan is comprehensive and well-structured. Address the 3 critical issues before implementation:

1. Add read-only guard to `set_backup_retention` (Task 6)
2. Ensure `BackupRetention` is `pub` (Task 2)
3. Create `cleanup_pre_update_backups_internal` helper for startup use (Task 11)

The 6 important issues are minor clarifications that can be fixed during implementation but should be noted.
