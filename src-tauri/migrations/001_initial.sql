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

-- Indexes
CREATE INDEX IF NOT EXISTS idx_trips_vehicle_date ON trips(vehicle_id, date);
CREATE INDEX IF NOT EXISTS idx_routes_vehicle ON routes(vehicle_id);
