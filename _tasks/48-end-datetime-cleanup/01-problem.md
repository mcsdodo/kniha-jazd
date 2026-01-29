# Problem Statement: end_time to end_datetime Cleanup

**Date:** 2026-01-29
**Status:** Planning
**Related:** Task 47 (datetime consolidation), ADR-012 (forward-only migrations)

## Current State

The Trip model still uses legacy field structure internally:
- `end_time: Option<String>` - stores "HH:MM" format
- `datetime: NaiveDateTime` - stores start datetime
- `date: NaiveDate` - redundant (derived from datetime)

The API was updated in Task 47 to accept `start_datetime` and `end_datetime`, but internally the code still:
1. Extracts `end_time` string from `end_datetime` in commands
2. Stores `end_time` in Trip struct
3. Writes to legacy `end_time` DB column

## Desired State

Clean Trip model with proper datetime fields:
- `start_datetime: NaiveDateTime` (rename from `datetime`)
- `end_datetime: NaiveDateTime` (replace `end_time: Option<String>`)
- Remove redundant `date: NaiveDate` field

## Why Now?

Per ADR-012, we don't maintain backward compatibility for older app versions reading newer databases. This means we can:
- Drop deprecated columns
- Rename fields freely
- Simplify the data model

## Scope

### In Scope
- Update Trip struct in `models.rs`
- Update TripRow and NewTripRow in `models.rs`
- Update `db.rs` (create_trip, update_trip, From<TripRow>)
- Update all Trip struct initializations in tests
- Update commands to use new field names
- Consider DB migration to rename columns (optional)

### Out of Scope
- Frontend changes (already uses startDatetime/endDatetime API)
- API parameter changes (already done in Task 47)

## Files to Modify

| File | Changes |
|------|---------|
| `models.rs` | Trip struct, TripRow, NewTripRow, test helpers |
| `db.rs` | create_trip, update_trip, From<TripRow> impl |
| `commands/trips.rs` | Remove end_time extraction, use end_datetime directly |
| `commands_tests.rs` | Update all Trip struct initializations (~40+) |
| `calculations_tests.rs` | Update Trip struct initializations |
| `db_tests.rs` | Update Trip struct initializations |
| `export.rs` | Update mock Trip structs |
| `schema.rs` | Optional: rename columns |

## Estimated Effort

Medium - mostly mechanical changes but many files/locations affected.
