-- Task 89: one place can be the home of the logbook. The journey grouping
-- (journeys module) matches trip endpoints against it by place ID.
ALTER TABLE places ADD COLUMN is_home BOOLEAN NOT NULL DEFAULT 0;

-- At most one home. A partial index ignores every row with is_home = 0.
CREATE UNIQUE INDEX idx_places_single_home ON places(is_home) WHERE is_home = 1;
