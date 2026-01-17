**Date:** 2025-01-08
**Subject:** Migrate from rusqlite to Diesel ORM for compile-time SQL safety
**Status:** Complete

## Goal

Replace rusqlite with Diesel ORM to catch database mapping errors at compile-time instead of runtime. Primary pain point: adding struct fields but forgetting to update INSERT/UPDATE SQL, causing data to silently not persist.

## Requirements

1. **Compile-time safety for all CRUD operations:**
   - SELECT: Struct fields must match query columns
   - INSERT: All required columns must be provided
   - UPDATE: Changed fields must exist in schema

2. **Existing database compatibility:**
   - Current `.db` files must work unchanged
   - No schema migrations required for initial switch
   - All existing data preserved

3. **Maintain test coverage:**
   - Migrate existing 17 db.rs tests to Diesel syntax
   - Tests should pass before and after migration

4. **Minimal frontend impact:**
   - Tauri command signatures stay the same
   - No changes to Svelte components

## Technical Notes

### Options Considered

| Option | INSERT/UPDATE Safety | Learning Curve | Chosen |
|--------|---------------------|----------------|--------|
| **SQLx** | Runtime only (tests needed) | Low | No |
| **Diesel** | Compile-time via derives | Medium | **Yes** |
| **SeaORM** | Runtime only | Medium | No |

### Key Diesel Features Used

- `#[derive(Queryable)]` - Maps SELECT results to struct
- `#[derive(Insertable)]` - Ensures INSERT has all columns
- `#[derive(AsChangeset)]` - Ensures UPDATE has all columns
- `diesel print-schema` - Generates schema.rs from existing DB

### Constraints

- Stay synchronous (no async migration needed for SQLite)
- Diesel CLI needed for development workflow
- `.sqlx/` cache NOT needed (that's SQLx pattern)
- `schema.rs` auto-generated, committed to git

## References

- [Diesel Getting Started](https://diesel.rs/guides/getting-started)
- [Diesel + SQLite](https://diesel.rs/guides/configuring-diesel-cli)
