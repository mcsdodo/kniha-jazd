-- Task 75: the place book. One row per normalised place name, holding the
-- coordinate a human confirmed for it.
--
-- There is no list of places here: the list is derived from trips at read time
-- (ADR-033), so nothing in this table decides which places exist. A row whose
-- trips are all deleted becomes a harmless orphan rather than a wrong answer.
--
-- `display_name` is a forensic record of the spelling the human confirmed the
-- coordinate under, never a source the list reads back — the list takes its
-- label from the trips (see `models.rs`).
CREATE TABLE places (
    -- NOT NULL is not redundant: SQLite lets a non-INTEGER PRIMARY KEY hold
    -- NULL, and `schema.rs` types this column as a non-nullable String.
    normalised_name TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    lat REAL,
    lon REAL,
    source TEXT NOT NULL
);
