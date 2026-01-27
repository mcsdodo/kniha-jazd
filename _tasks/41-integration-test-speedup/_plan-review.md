# Plan Review: Integration Test Speedup

## Summary

The plan is solid in concept but has several technical issues that would cause implementation failures, particularly around foreign key constraint ordering and Database access patterns. Recommend fixing critical issues before implementation.

## Findings

### Critical

- [ ] **Foreign key deletion order is wrong (Step 1.1):** The plan shows `DELETE FROM trips` before `DELETE FROM receipts`, but `receipts.trip_id` references `trips.id`. Correct order must be: receipts -> trips -> routes -> vehicles. SQLite will fail if foreign keys are enforced.

- [ ] **Database access pattern incorrect (Step 1.1):** Plan shows `db::get_connection(&app_state)` but should use `State<Database>` and call `db.connection()` method. The `Database` struct exposes `connection(&self) -> MutexGuard<SqliteConnection>` for raw SQL. Correct signature:
  ```rust
  pub fn reset_test_database(db: State<Database>) -> Result<(), String> {
      let conn = &mut *db.connection();
      diesel::sql_query("DELETE FROM receipts").execute(conn)?;
      // ...
  }
  ```

- [ ] **Raw SQL syntax wrong for Diesel (Step 1.1):** `conn.execute("DELETE FROM trips", [])` is rusqlite syntax. With Diesel, use `diesel::sql_query("DELETE FROM trips").execute(conn)`.

### Important

- [ ] **seedTrip() has no refresh to remove (Step 3.2):** The plan says to remove refresh from seedTrip() but looking at `tests/integration/utils/db.ts`, only `seedVehicle()` (line 157-158), `seedScenario()` (line 376-377), and `setActiveVehicle()` (line 422-423) have browser.refresh() calls. seedTrip() never had one.

- [ ] **Settings table not cleared (Step 1.1):** Plan says "Settings can stay or be reset to defaults" but tests that seed settings (e.g., company name) could leak into other tests. Recommend adding `DELETE FROM settings` for complete isolation.

- [ ] **Missing store: `locale` store may need reset (Step 2.1):** The `localeStore` in `src/lib/stores/locale.ts` persists to localStorage. While beforeTest sets locale, if tests modify locale during execution, other stores may cache the old value.

- [ ] **Test helpers loaded in production (Step 2.2):** Importing `$lib/stores/test-helpers` unconditionally in `+layout.svelte` exposes `__TEST_RESET_STORES__` global in production builds. Should guard with `import.meta.env.DEV` or equivalent check.

- [ ] **Rollback strategy is weak (Rollback Plan):** "Keep old code commented" creates technical debt. Better approach: create a feature flag or separate branch, not commented code.

### Minor

- [ ] **Unit test skeleton incomplete (Step 1.2):** Test code shows `std::env::remove_var()` but doesn't show how to properly set up the test fixture with Database state. Consider using existing `test_setup!` macro pattern from other test files.

- [ ] **No mention of which stores DON'T need reset:** Plan lists stores to reset but doesn't explain why `toast`, `confirm`, `theme`, `update`, `appMode`, `homeAssistant` stores are excluded. Document rationale for future maintainers.

- [ ] **Isolation test file path correct:** `tests/integration/specs/tier1/isolation.spec.ts` is a valid location within Tier 1.

- [ ] **Verification timing (Step 4.2):** "Compare before/after" baseline measurement should happen BEFORE any changes, not during Phase 4. Consider measuring current performance first.

## File Path Verification

| Plan Path | Exists | Correct |
|-----------|--------|---------|
| `src-tauri/src/commands.rs` | Yes | Yes |
| `src-tauri/src/lib.rs` | Yes | Yes |
| `src-tauri/src/commands_tests.rs` | Yes | Yes |
| `src/lib/stores/test-helpers.ts` | No (new) | Yes |
| `src/routes/+layout.svelte` | Yes | Yes |
| `tests/integration/wdio.conf.ts` | Yes | Yes |
| `tests/integration/utils/db.ts` | Yes | Yes |
| `tests/integration/specs/tier1/isolation.spec.ts` | No (new) | Yes |

## Task Dependency Order

Current order is correct:
1. Backend command (no dependencies)
2. Frontend helpers (no dependencies, can parallel with 1)
3. Test infrastructure (depends on 1 and 2)
4. Testing & validation (depends on 3)
5. Cleanup & documentation (depends on 4)

## Scope Assessment

- No scope creep detected
- Plan stays focused on test speedup goal
- No unnecessary abstractions

## Recommendation

**APPROVE WITH CHANGES**

Fix the Critical issues before implementation:

1. Correct foreign key deletion order: `receipts -> trips -> routes -> vehicles`
2. Fix Database access: use `State<Database>` and `db.connection()`
3. Fix SQL syntax: use `diesel::sql_query().execute()`

Also address the Important items:
- Add `DELETE FROM settings`
- Guard test-helpers import with dev mode check
- Remove seedTrip from the "refresh to remove" list

Once these changes are made to the plan, implementation can proceed.
