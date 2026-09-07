-- Task 75: the place book. One row per normalised place name, holding the
-- coordinate a human confirmed for it.
--
-- There is no list of places here: the list is derived from trips at read time
-- (ADR-033), so this table stores coordinates and nothing else. A row whose
-- trips are all deleted becomes a harmless orphan rather than a wrong answer.
CREATE TABLE places (
    -- NOT NULL is not redundant: SQLite lets a non-INTEGER PRIMARY KEY hold
    -- NULL, and `schema.rs` types this column as a non-nullable String.
    normalised_name TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    lat REAL,
    lon REAL,
    source TEXT NOT NULL
);
