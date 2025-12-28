#!/usr/bin/env python3
"""
Create a showcase database with realistic Slovak vehicle logbook data.
Used for screenshots and demonstrations.

Usage:
    python scripts/create_showcase_db.py

Output:
    scripts/kniha-jazd.db

To use for screenshots:
    1. Run this script to generate kniha-jazd.db
    2. Backup your real database:
       copy %APPDATA%\\com.notavailable.kniha-jazd\\kniha-jazd.db %APPDATA%\\com.notavailable.kniha-jazd\\kniha-jazd.db.bak
    3. Copy showcase DB:
       copy scripts\\kniha-jazd.db %APPDATA%\\com.notavailable.kniha-jazd\\kniha-jazd.db
    4. Launch app and take screenshots
    5. Restore your real database:
       copy %APPDATA%\\com.notavailable.kniha-jazd\\kniha-jazd.db.bak %APPDATA%\\com.notavailable.kniha-jazd\\kniha-jazd.db
"""

import sqlite3
import uuid
from datetime import datetime, timezone
from pathlib import Path

# Output path
SCRIPT_DIR = Path(__file__).parent
OUTPUT_DB = SCRIPT_DIR / "kniha-jazd.db"

# Schema from migrations
SCHEMA = """
-- Vehicles table
CREATE TABLE IF NOT EXISTS vehicles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    license_plate TEXT NOT NULL,
    tank_size_liters REAL NOT NULL,
    tp_consumption REAL NOT NULL,
    initial_odometer REAL NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Trips table
CREATE TABLE IF NOT EXISTS trips (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    date TEXT NOT NULL,
    origin TEXT NOT NULL,
    destination TEXT NOT NULL,
    distance_km REAL NOT NULL,
    odometer REAL NOT NULL,
    purpose TEXT NOT NULL,
    fuel_liters REAL,
    fuel_cost_eur REAL,
    other_costs_eur REAL,
    other_costs_note TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    full_tank INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
);

-- Routes table
CREATE TABLE IF NOT EXISTS routes (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL,
    origin TEXT NOT NULL,
    destination TEXT NOT NULL,
    distance_km REAL NOT NULL,
    usage_count INTEGER NOT NULL DEFAULT 1,
    last_used TEXT NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    UNIQUE(vehicle_id, origin, destination)
);

-- Settings table
CREATE TABLE IF NOT EXISTS settings (
    id TEXT PRIMARY KEY,
    company_name TEXT NOT NULL DEFAULT '',
    company_ico TEXT NOT NULL DEFAULT '',
    buffer_trip_purpose TEXT NOT NULL DEFAULT 'služobná cesta',
    updated_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_trips_vehicle_date ON trips(vehicle_id, date);
CREATE INDEX IF NOT EXISTS idx_routes_vehicle ON routes(vehicle_id);
"""


def create_db():
    """Create showcase database with realistic data."""
    # Remove existing
    if OUTPUT_DB.exists():
        OUTPUT_DB.unlink()

    conn = sqlite3.connect(OUTPUT_DB)
    cursor = conn.cursor()

    # Create schema
    cursor.executescript(SCHEMA)

    now = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")

    # --- Settings ---
    settings_id = str(uuid.uuid4())
    cursor.execute("""
        INSERT INTO settings (id, company_name, company_ico, buffer_trip_purpose, updated_at)
        VALUES (?, ?, ?, ?, ?)
    """, (settings_id, "DEMO s.r.o.", "12345678", "služobná cesta", now))

    # --- Vehicle ---
    # Škoda Octavia - common Slovak business car
    vehicle_id = str(uuid.uuid4())
    cursor.execute("""
        INSERT INTO vehicles (id, name, license_plate, tank_size_liters, tp_consumption, initial_odometer, is_active, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, (vehicle_id, "Škoda Octavia", "BA-123AB", 50.0, 5.1, 45000.0, 1, now, now))

    # --- Trips ---
    # Realistic 2024 data showing:
    # - Regular commutes
    # - Business trips
    # - Fill-ups with consumption calculation
    # - Margin around 15-18% (healthy, under 20% limit)

    trips_data = [
        # (date, origin, destination, km, fuel_liters, fuel_cost, purpose)
        # January - starting fresh
        ("2024-01-03", "Bratislava", "Trnava", 55, None, None, "stretnutie s klientom"),
        ("2024-01-03", "Trnava", "Bratislava", 55, None, None, "návrat"),
        ("2024-01-08", "Bratislava", "Nitra", 92, None, None, "obchodné rokovanie"),
        ("2024-01-08", "Nitra", "Bratislava", 92, None, None, "návrat"),
        ("2024-01-15", "Bratislava", "Senec", 28, None, None, "dodávka tovaru"),
        ("2024-01-15", "Senec", "Bratislava", 28, 19.6, 31.36, "návrat + tankovanie"),  # Fill-up: 5.6 l/100km (110%)

        # February
        ("2024-02-05", "Bratislava", "Trenčín", 128, None, None, "školenie"),
        ("2024-02-05", "Trenčín", "Bratislava", 128, None, None, "návrat"),
        ("2024-02-12", "Bratislava", "Malacky", 42, None, None, "audit"),
        ("2024-02-12", "Malacky", "Bratislava", 42, None, None, "návrat"),
        ("2024-02-20", "Bratislava", "Dunajská Streda", 48, None, None, "stretnutie"),
        ("2024-02-20", "Dunajská Streda", "Bratislava", 48, 28.9, 46.24, "návrat + tankovanie"),  # Fill-up: 6.63 l/100km (130%) - OVER LIMIT

        # March
        ("2024-03-04", "Bratislava", "Žilina", 198, None, None, "konferencia"),
        ("2024-03-04", "Žilina", "Bratislava", 198, None, None, "návrat"),
        ("2024-03-11", "Bratislava", "Pezinok", 22, None, None, "obhliadka"),
        ("2024-03-11", "Pezinok", "Bratislava", 22, None, None, "návrat"),
        ("2024-03-18", "Bratislava", "Trnava", 55, None, None, "stretnutie s klientom"),
        ("2024-03-18", "Trnava", "Bratislava", 55, 32.0, 51.20, "návrat + tankovanie"),  # Fill-up: 5.82 l/100km (114%)
    ]

    odometer = 45000.0  # Starting from initial_odometer
    sort_order = 0

    for date_str, origin, dest, km, fuel, cost, purpose in trips_data:
        trip_id = str(uuid.uuid4())
        odometer += km

        cursor.execute("""
            INSERT INTO trips (id, vehicle_id, date, origin, destination, distance_km, odometer,
                             purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note,
                             sort_order, full_tank, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (trip_id, vehicle_id, date_str, origin, dest, km, odometer,
              purpose, fuel, cost, None, None,
              sort_order, 1, now, now))
        sort_order += 1

    # --- Routes (for autocomplete) ---
    routes = [
        ("Bratislava", "Trnava", 55),
        ("Trnava", "Bratislava", 55),
        ("Bratislava", "Nitra", 92),
        ("Nitra", "Bratislava", 92),
        ("Bratislava", "Senec", 28),
        ("Senec", "Bratislava", 28),
        ("Bratislava", "Trenčín", 128),
        ("Trenčín", "Bratislava", 128),
        ("Bratislava", "Žilina", 198),
        ("Žilina", "Bratislava", 198),
    ]

    for origin, dest, km in routes:
        route_id = str(uuid.uuid4())
        cursor.execute("""
            INSERT INTO routes (id, vehicle_id, origin, destination, distance_km, usage_count, last_used)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        """, (route_id, vehicle_id, origin, dest, km, 2, now))

    conn.commit()
    conn.close()

    print(f"Created showcase database: {OUTPUT_DB}")
    print(f"  - 1 vehicle: Škoda Octavia (BA-123AB)")
    print(f"  - {len(trips_data)} trips (Jan-Mar 2024)")
    print(f"  - 3 fill-ups with consumption data")
    print(f"  - Final odometer: {odometer:.0f} km")


if __name__ == "__main__":
    create_db()
