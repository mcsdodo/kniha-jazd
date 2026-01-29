# Problem Statement: Datetime Field Consolidation

## Current State

The trips table has redundant datetime fields:
- `date` (TEXT) - stores "YYYY-MM-DD"
- `datetime` (TEXT) - stores "YYYY-MM-DDTHH:MM:SS" (date + start time combined)
- `end_time` (TEXT) - stores "HH:MM" or empty string

**Issues:**
1. **Redundancy** - date is stored twice (in `date` and inside `datetime`)
2. **Inconsistent types** - start time is full datetime, end time is just time string
3. **No multi-day support** - end_time has no date component (can't represent overnight trips)
4. **String parsing** - end_time requires manual parsing, no type safety

## Desired State

Consolidate into two proper datetime fields:
- `start_datetime` (TEXT) - full ISO 8601 datetime for journey start
- `end_datetime` (TEXT, nullable) - full ISO 8601 datetime for journey end

**Benefits:**
1. No redundancy - date derived from start_datetime when needed
2. Type consistency - both fields are full datetimes
3. Multi-day trips - end_datetime can be on a different day
4. Cleaner sorting - single field for chronological order

## Constraints

Per CLAUDE.md database migration guidelines:
- **Must keep old columns** (date, datetime, end_time) for backward compatibility
- **Add new columns with defaults** - older app versions must be able to read the DB
- **No column removal or renaming**

## Migration Strategy

1. Add `start_datetime` and `end_datetime` columns
2. Populate from existing data:
   - `start_datetime` = existing `datetime` value
   - `end_datetime` = `date` + `end_time` (if end_time exists) or NULL
3. Update Rust code to use new fields as primary, keep old fields in sync
4. Frontend continues to work unchanged (backend handles the mapping)
