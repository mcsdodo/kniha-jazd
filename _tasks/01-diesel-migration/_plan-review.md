# Plan Review

**Target:** `_tasks/01-diesel-migration/02-plan.md`
**Started:** 2026-01-08
**Status:** Ready for User Review
**Focus:** Completeness, feasibility, clarity

---

## Review Summary

**Iterations:** 1
**Total Findings:** 3 Critical, 7 Important, 4 Minor

### Recommendation

**Needs revisions** - The plan is a solid foundation but underestimates complexity in several areas. With amendments, the migration is feasible and will deliver the promised compile-time safety benefits.

---

## All Findings (Consolidated)

### Critical

1. [ ] **Direct rusqlite usage in commands.rs for backup inspection**
   - Location: `commands.rs:1339` in `get_backup_info`
   - Opens backup DB files directly with rusqlite to query metadata
   - **Decision needed:** Keep rusqlite just for backup inspection OR migrate that code too
   - Keeping rusqlite partially defeats the migration goal

2. [ ] **error.rs has rusqlite dependency not mentioned in plan**
   - Location: `src-tauri/src/error.rs`
   - Contains `Database(#[from] rusqlite::Error)` enum variant
   - Must change to `diesel::result::Error`
   - **Task 11 scope must include error.rs**

3. [ ] **Diesel 2.x migration API syntax is wrong in Task 4**
   - Plan shows Diesel 1.x API: `diesel_migrations::run_pending_migrations(&mut conn)`
   - Diesel 2.x requires:
     ```rust
     use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
     pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
     conn.run_pending_migrations(MIGRATIONS).unwrap();
     ```
   - **Code won't compile as written**

### Important

4. [ ] **r2d2 connection pooling is YAGNI**
   - Plan includes `r2d2` feature for connection pooling
   - This is a single-user desktop app with `Mutex<Connection>` serialization
   - No concurrent connections needed; SQLite has single-writer limitation
   - **Recommendation:** Remove `r2d2` feature to reduce complexity

5. [ ] **Type mapping complexity underestimated in Task 3**
   - `VehicleType` and `ReceiptStatus` enums need `ToSql/FromSql` implementations
   - `FieldConfidence` is stored as JSON text - Diesel doesn't natively support this
   - Current code has manual string conversion functions
   - **Need explicit type mapping examples** for enums and JSON fields

6. [ ] **Baseline migration approach is problematic (Task 2)**
   - Empty `up.sql`/`down.sql` means `diesel migration run` on fresh DB creates nothing
   - Better: Use `embed_migrations!` for tests, skip creating migrations folder
   - Existing `001_initial.sql` + inline migrations already handle schema creation

7. [ ] **Complex SQL queries need special handling**
   - Several queries use SQLite-specific SQL that Diesel DSL doesn't support:
     | Query | Method | Approach |
     |-------|--------|----------|
     | `strftime('%Y', date) = ?` | `get_trips_for_vehicle_in_year` | `diesel::dsl::sql` |
     | `CAST(strftime('%Y'...)` | `get_years_with_trips` | Custom SQL expression |
     | `DISTINCT TRIM(purpose)` | `get_purposes_for_vehicle` | `diesel::sql_query` |
     | `INSERT...SELECT...AVG...COUNT` | `populate_routes_from_trips` | `diesel::sql_query` |
   - **Tasks 6, 7, 10 should note these need raw SQL**

8. [ ] **Transaction support not addressed for reorder_trip**
   - `reorder_trip` performs multiple operations that should be atomic
   - Current code relies on implicit SQLite autocommit
   - **Task 6 should add `conn.transaction()` requirement**

9. [ ] **NewVehicle/NewTrip dual-struct pattern needs evaluation**
   - Every entity gets two structs (Queryable vs Insertable)
   - Consider `#[diesel(treat_none_as_null = false)]` for single-struct approach
   - Affects `commands.rs` and test helpers

10. [ ] **Test count discrepancy**
    - Plan says "migrate existing 17 db.rs tests"
    - Actual count is ~25+ tests including recent receipt year filtering tests

### Minor

11. [ ] **Verification steps are weak**
    - "cargo test vehicle" won't work - tests aren't organized by entity name
    - Better: List specific test function names or "All db.rs tests pass"

12. [ ] **Documentation update should include CLI command reference**
    - Task 14 should document: schema regeneration, diesel CLI commands, r2d2 decision

13. [ ] **Pre-migration step: preserve Cargo.lock**
    - Rollback plan should note to save current Cargo.lock
    - Adding Diesel changes many transitive dependencies

14. [ ] **get_purposes_for_vehicle listed in Task 7 but complexity not noted**
    - Uses `DISTINCT TRIM(purpose)` - needs raw SQL approach

---

## Required Plan Amendments

| Task | Amendment |
|------|-----------|
| Task 1 | Remove `r2d2` feature from Cargo.toml |
| Task 2 | Reconsider baseline migration approach; use embedded migrations |
| Task 3 | Add detailed enum/JSON mapping examples with code |
| Task 4 | Fix Diesel 2.x migration syntax; show correct `embed_migrations!` usage |
| Task 6 | Add transaction requirement for `reorder_trip`; note year queries need raw SQL |
| Task 7 | Note that `get_purposes_for_vehicle` uses raw SQL |
| Task 10 | Note that `populate_routes_from_trips` uses raw SQL |
| Task 11 | Add `error.rs` to scope; **decide backup inspection strategy** |
| Task 12 | Update test count to actual number (~25+) |

---

## Risk Assessment

**Original estimate:** Medium risk
**Revised estimate:** Medium-High risk

**Reasons:**
- Complex SQL queries requiring raw SQL escape hatches
- Enum/JSON type mapping complexity
- Backup inspection code requiring separate handling
- Diesel 2.x syntax differences from plan examples

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

**Confidence:** High - comprehensive codebase review completed
