-- Task 90: Fuelio drives the user marked "ignore" on the Fuelio page (for
-- example a private drive). Per vehicle: the drives folder is shared, and the
-- cross-check runs for one vehicle. drive_id is the Fuelio route ID (epoch ms).
CREATE TABLE fuelio_ignored_drives (
    vehicle_id TEXT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    drive_id TEXT NOT NULL,
    ignored_at TEXT NOT NULL,
    PRIMARY KEY (vehicle_id, drive_id)
);
