-- Forward-only in practice (ADR-012); no diesel CLI revert runs in this repo.
ALTER TABLE trip_routes DROP COLUMN mode;
