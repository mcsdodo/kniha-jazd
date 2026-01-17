# Plan Review

**Target:** `_tasks/28-diesel-migration/02-plan.md`
**Started:** 2026-01-08
**Status:** Complete
**Focus:** Completeness, feasibility, clarity

---

## Review Summary

**Iterations:** 1
**Total Findings:** 3 Critical, 7 Important, 4 Minor
**Addressed:** 14 findings (all)
**Skipped:** 0

### Recommendation

**Ready for implementation** - All findings have been addressed. The plan now includes proper Diesel 2.x syntax, explicit type mapping examples, raw SQL documentation, and comprehensive error handling scope.

---

## All Findings (Consolidated)

### Critical

1. [x] **Direct rusqlite usage in commands.rs for backup inspection**
   - Location: `commands.rs:1339` in `get_backup_info`
   - **Resolution:** Task 11 updated to migrate backup inspection to Diesel with `Database::from_path()` method

2. [x] **error.rs has rusqlite dependency not mentioned in plan**
   - Location: `src-tauri/src/error.rs`
   - **Resolution:** Task 11 now includes error.rs in scope with explicit code example

3. [x] **Diesel 2.x migration API syntax is wrong in Task 4**
   - **Resolution:** Task 4 updated with correct `embed_migrations!` and `MigrationHarness` syntax

### Important

4. [x] **r2d2 connection pooling is YAGNI**
   - **Resolution:** Task 1 updated - removed `r2d2` feature from Cargo.toml

5. [x] **Type mapping complexity underestimated in Task 3**
   - **Resolution:** Task 3 expanded with detailed code examples for VehicleType, ReceiptStatus, FieldConfidence

6. [x] **Baseline migration approach is problematic (Task 2)**
   - **Resolution:** Task 2 updated - baseline now contains actual CREATE TABLE statements, not empty files

7. [x] **Complex SQL queries need special handling**
   - **Resolution:** Tasks 6, 7, 10 updated with explicit raw SQL examples and `sql_query` usage

8. [x] **Transaction support not addressed for reorder_trip**
   - **Resolution:** Task 6 updated with `conn.transaction()` wrapper example

9. [x] **NewVehicle/NewTrip dual-struct pattern needs evaluation**
   - **Resolution:** Noted in Task 3 - dual-struct is standard Diesel pattern, implementation will evaluate `treat_none_as_null`

10. [x] **Test count discrepancy**
    - **Resolution:** Task 12 updated to reflect actual test counts (108 backend + 61 integration)

### Minor

11. [x] **Verification steps are weak**
    - **Resolution:** All verification steps updated to use `cargo test --lib`

12. [x] **Documentation update should include CLI command reference**
    - **Resolution:** Task 14 scope confirmed to include Diesel CLI commands

13. [x] **Pre-migration step: preserve Cargo.lock**
    - **Resolution:** Rollback plan updated with Cargo.lock backup step

14. [x] **get_purposes_for_vehicle listed in Task 7 but complexity not noted**
    - **Resolution:** Task 7 updated with explicit raw SQL example for DISTINCT TRIM()

---

## Resolution Summary

**User Decision:** Address all findings, Option B for backup inspection (migrate to Diesel)

### Applied Changes to Plan

| Task | Change Applied |
|------|----------------|
| Task 1 | Removed `r2d2` feature, added comment explaining why |
| Task 2 | Added specific Windows path for DB copy, baseline migration now contains actual schema |
| Task 3 | Added 40+ lines of type mapping code examples (VehicleType, ReceiptStatus, FieldConfidence) |
| Task 4 | Fixed to Diesel 2.x API with `embed_migrations!` and `MigrationHarness` |
| Task 5 | Updated verification step |
| Task 6 | Added raw SQL examples for year filtering, transaction wrapper for reorder_trip |
| Task 7 | Added raw SQL example with `QueryableByName` for get_purposes_for_vehicle |
| Task 8 | Updated verification step |
| Task 9 | Updated verification step |
| Task 10 | Added complete raw SQL example for populate_routes_from_trips |
| Task 11 | Expanded to include error.rs and backup migration to Diesel |
| Task 12 | Updated test counts to actual values |
| Rollback | Added Cargo.lock backup step |

---

## Risk Assessment

**Original estimate:** Medium risk
**Post-review estimate:** Medium risk (improved from Medium-High)

**Mitigations applied:**
- Correct Diesel 2.x syntax prevents compilation issues
- Explicit raw SQL patterns prevent DSL confusion
- Backup migration strategy decided upfront
- Type mapping examples prevent trial-and-error

---

## Coverage Assessment

**Areas reviewed:**
- [x] All db.rs methods checked against plan tasks
- [x] models.rs struct definitions and type mappings
- [x] commands.rs integration points
- [x] error.rs dependency
- [x] Cargo.toml dependencies
- [x] Test coverage scope
- [x] Diesel API version correctness

**Confidence:** High - all findings addressed with explicit code examples
