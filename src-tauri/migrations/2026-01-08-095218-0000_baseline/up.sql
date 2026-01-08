-- Baseline migration: Full schema for kniha-jazd
-- This creates all tables for fresh databases (tests, new installs)
-- Existing production DBs already have these tables and won't run this

-- Vehicles table (with EV support fields)
CREATE TABLE IF NOT EXISTS vehicles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    license_plate TEXT NOT NULL,
    vehicle_type TEXT NOT NULL DEFAULT 'Ice',
    tank_size_liters REAL,
    tp_consumption REAL,
    battery_capacity_kwh REAL,
    baseline_consumption_kwh REAL,
    initial_battery_percent REAL,
    initial_odometer REAL NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Trips table (with EV and sorting fields)
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
    full_tank INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    energy_kwh REAL,
    energy_cost_eur REAL,
    full_charge INTEGER DEFAULT 0,
    soc_override_percent REAL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)
);

-- Routes table (for autocomplete)
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

-- Receipts table (for fuel receipt OCR)
CREATE TABLE IF NOT EXISTS receipts (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT,
    trip_id TEXT UNIQUE,
    file_path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    scanned_at TEXT NOT NULL,
    liters REAL,
    total_price_eur REAL,
    receipt_date TEXT,
    station_name TEXT,
    station_address TEXT,
    source_year INTEGER,
    status TEXT NOT NULL DEFAULT 'Pending',
    confidence TEXT NOT NULL DEFAULT '{"liters":"Unknown","total_price":"Unknown","date":"Unknown"}',
    raw_ocr_text TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
    FOREIGN KEY (trip_id) REFERENCES trips(id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_trips_vehicle_date ON trips(vehicle_id, date);
CREATE INDEX IF NOT EXISTS idx_routes_vehicle ON routes(vehicle_id);
CREATE INDEX IF NOT EXISTS idx_vehicles_type ON vehicles(vehicle_type);
CREATE INDEX IF NOT EXISTS idx_receipts_status ON receipts(status);
CREATE INDEX IF NOT EXISTS idx_receipts_trip ON receipts(trip_id);
CREATE INDEX IF NOT EXISTS idx_receipts_date ON receipts(receipt_date);
CREATE INDEX IF NOT EXISTS idx_receipts_vehicle ON receipts(vehicle_id);
