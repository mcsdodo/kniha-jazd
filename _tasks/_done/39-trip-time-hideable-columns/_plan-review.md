# Plan Review: Trip Time Field + Hideable Columns

**Reviewer:** Claude Opus 4.5
**Date:** 2026-01-26
**Plan:** 03-plan.md

## Round 1: Initial Assessment

### Completeness

**Critical Issues:**

1. **Missing `down.sql` migration** - Plan only mentions `up.sql`. For database rollback safety, a `down.sql` should be included (even if it's just a comment explaining why it cannot be undone safely in SQLite).

2. **Model update incomplete** - Plan says "Change `Trip.date: NaiveDate` to `Trip.datetime: NaiveDateTime`" but the design doc says to keep both fields with datetime as the source of truth. This inconsistency needs resolution. Looking at current `models.rs`, the `Trip` struct has `date: NaiveDate` which is parsed from the `TripRow.date` column. The plan should clarify:
   - Keep `Trip.date` for backward compatibility OR
   - Replace with `Trip.datetime` and derive `date` where needed

3. **`TripRow` struct update missing** - Plan mentions updating `TripRow` but doesn't specify adding `datetime: String` column to the struct. The Diesel schema will need updating too.

**Important Issues:**

4. **Year filtering logic** - Current `db.rs` filters trips by year using the `date` column with `LIKE '{year}-%'`. If datetime is the new source of truth, this query needs to be updated OR the `date` column must continue to be populated.

5. **Export datetime parsing** - Plan mentions "Extract time from datetime for display" but export.rs currently uses `trip.date.format("%d.%m.%Y")`. Need to:
   - Parse datetime string to extract date + time
   - Add time column to export
   - Update `ExportLabels` struct with `col_time` field

### Feasibility

**Critical Issues:**

6. **Migration SQL syntax** - The proposed SQL:
   ```sql
   ALTER TABLE trips ADD COLUMN datetime TEXT NOT NULL DEFAULT '1970-01-01T00:00:00';
   UPDATE trips SET datetime = date || 'T00:00:00';
   ```
   This is correct SQLite syntax. However, the DEFAULT value with 1970 is misleading - better to use empty string or leave NULL. Also, the UPDATE should run in the same migration file.

**Important Issues:**

7. **Diesel schema regeneration** - Step 1.2 says "Run `diesel print-schema` to regenerate" but this project uses manual schema.rs updates (not auto-generated). The schema.rs file has manual comments and ordering.

8. **`From<TripRow> for Trip` conversion** - Plan mentions updating this but doesn't detail how. The conversion will need to:
   - Parse `datetime` string to `NaiveDateTime`
   - Extract date component if keeping `Trip.date`

### Clarity

**Important Issues:**

9. **File paths incomplete** - Phase 3.1 says "Files: `src/lib/types.ts`" but doesn't specify what changes. The current Trip type in TypeScript uses `date: string`, need to decide on approach.

10. **Frontend API approach unclear** - Plan says "Update `createTrip` to accept time parameter" but doesn't specify if this is:
    - Separate `date` + `time` params (easier for form binding)
    - Combined `datetime` param (matches backend)

11. **Phase 2.2 command registration** - Plan mentions registering in `lib.rs` but should explicitly list the new commands to add to `invoke_handler`.

### YAGNI

**Minor Issues:**

12. **Future "drop column" migration mentioned** - Design says "Migration 2 (future, optional): Drop old `date` column" - this is unnecessary scope creep. SQLite doesn't support DROP COLUMN anyway (requires table rebuild).

---

## Round 2: Deep Dive on Critical Issues

### Issue #1: down.sql resolution

**Finding:** SQLite doesn't support `ALTER TABLE DROP COLUMN` in older versions. The down.sql should document this limitation. For production, the approach should be:
- `down.sql` with a comment: `-- Cannot safely remove datetime column in SQLite. No-op migration.`

### Issue #2 & #3: Model changes clarification

**Finding:** Looking at current code flow:
1. `TripRow.date: String` is parsed to `Trip.date: NaiveDate` in `From<TripRow> for Trip`
2. API uses `Trip.date` for all date operations
3. Export uses `trip.date.format()`

**Recommendation:** Add `datetime` field to both structs while keeping `date`:
- `TripRow`: Add `datetime: String`
- `Trip`: Add `datetime: NaiveDateTime`, keep `date: NaiveDate` (derived from datetime)
- This maintains backward compatibility and minimal code changes

### Issue #5: Export changes detail

**Finding:** `export.rs` needs:
1. Add `col_time: String` to `ExportLabels` struct
2. Add time column header in `col_headers` building
3. Extract time from `trip.datetime` for row rendering
4. Update i18n files with `colTime` export label

**Missing from plan:** The `ExportLabels` struct change and i18n `export.colTime` key.

### Issue #7: Schema.rs update approach

**Finding:** Looking at `schema.rs`, it's annotated with `// @generated automatically by Diesel CLI` but has manual edits (comments). The plan should specify:
- Add `datetime -> Text` to trips table in schema.rs
- Position: After `date` column to maintain logical grouping

---

## Round 3: Test Coverage Check

### Backend Tests

**Missing from plan:**

13. **Migration test** - No test specified for verifying migration runs on existing data. Should test:
    - Fresh DB with migration
    - DB with existing trips gets datetime populated

14. **Datetime parsing edge cases** - Need tests for:
    - Valid datetime parsing ("2026-01-15T08:30:00")
    - Empty time defaults to 00:00
    - Invalid format handling

15. **Hidden columns serialization** - Plan mentions test but should specify:
    - Empty array serializes correctly
    - Unknown column names are preserved (future-proofing)

### Integration Tests

**Finding:** Plan's Phase 6.2 is vague. Current integration test infrastructure uses:
- WebdriverIO + tauri-driver
- Tests in `tests/integration/`

**Specific tests needed:**
- `time-column.spec.ts` - Time input visible, editable, saves correctly
- `column-visibility.spec.ts` - Toggle works, persists on reload

---

## Round 4: i18n Keys Audit

### Missing Keys (Critical)

The plan mentions some i18n keys but misses:

1. **Export label** - `export.colTime` for HTML export column header
2. **Column visibility section** - Needs full key structure:
   ```typescript
   columnVisibility: {
       title: 'Stlpce',
       // Individual column labels should reference existing trips.columns.*
   }
   ```

### Keys Accounted For

- `trips.columns.time` - mentioned in plan
- `trips.columnVisibility.*` - mentioned in plan

---

## Summary of Findings

### Critical (Blocks Implementation)

| # | Issue | Resolution | Status |
|---|-------|------------|--------|
| 1 | Missing down.sql | Add down.sql with no-op comment | ✅ Fixed |
| 2 | Model change approach unclear | Keep both `date` and `datetime` fields | ✅ Fixed |
| 5 | Export changes incomplete | Add ExportLabels.col_time, update row rendering | ✅ Fixed |

### Important (Should Fix Before Implementation)

| # | Issue | Resolution | Status |
|---|-------|------------|--------|
| 3 | TripRow struct update missing | Add `datetime: String` column explicitly | ✅ Fixed |
| 4 | Year filtering query | Clarify if date column continues to be updated | ✅ Fixed |
| 7 | Schema.rs update approach | Manual update, not diesel print-schema | ✅ Fixed |
| 9 | TypeScript types unclear | Specify Trip type change approach | ✅ Fixed |
| 10 | API parameter approach | Specify date+time vs datetime param | ✅ Fixed |
| 13 | Migration test missing | Add migration verification test | ✅ Fixed |
| 14 | Datetime parsing tests | Add edge case tests | ✅ Fixed |

### Minor (Nice to Have) — Skipped

| # | Issue | Resolution | Status |
|---|-------|------------|--------|
| 11 | Command registration detail | List exact commands to add | ⏭️ Skipped |
| 12 | Future DROP COLUMN mention | Remove - unnecessary scope | ⏭️ Skipped |
| 15 | Hidden columns serialization test | Specify edge cases | ⏭️ Skipped |

---

## Checklist Assessment

- [x] Tasks have specific file paths - Mostly, but some phases need more detail
- [ ] Verification steps for each phase - Missing, only final checklist provided
- [x] Correct dependency order - Yes, backend before frontend
- [x] No scope creep beyond task requirements - Minor (future migration mention)
- [ ] Test coverage mentioned for new functionality - Incomplete, needs specific test cases
- [x] Database migration is backward-compatible - Yes, ADD COLUMN with DEFAULT

---

## Recommendations

1. **Update Phase 1.1** to include `down.sql` and migration test
2. **Clarify Phase 1.3** to explicitly show both `Trip` and `TripRow` changes with code snippets
3. **Add Phase 1.6** for year filtering query update if needed
4. **Update Phase 5.1** with complete export.rs changes including ExportLabels
5. **Add verification step after each phase** (e.g., "Run `cargo test` to verify")
6. **Add specific integration test file names** in Phase 6.2

---

## Resolution (2026-01-26)

**Addressed:** All Critical (3) + Important (8) findings
**Skipped:** Minor (4) findings — not blocking implementation

**Key changes to plan:**
- Added `down.sql` with SQLite no-op comment
- Clarified model approach: keep both `date` and `datetime` fields
- Detailed `TripRow` and `NewTripRow` struct changes with code snippets
- Confirmed year filtering continues using `date` column (no change needed)
- Specified manual schema.rs update (not diesel print-schema)
- Clarified API approach: separate `date` + `time` params for easier frontend binding
- Added TypeScript approach: keep `date`, add `datetime`, add `extractTime` helper
- Added ExportLabels.col_time and time column rendering details
- Added verification steps after each phase
- Added specific test cases for datetime parsing and migration
- Added integration test file names: `time-column.spec.ts`, `column-visibility.spec.ts`
