**Date:** 2024-12-23
**Subject:** Implementation Plan
**Status:** Ready for execution

---

# Kniha Jázd Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a desktop vehicle logbook app that tracks trips, fuel consumption, and ensures Slovak legal compliance (20% margin rule).

**Architecture:** Tauri desktop app with Rust backend handling SQLite storage and business logic calculations. SvelteKit frontend provides an Excel-like grid interface for trip management. All calculation logic lives in Rust with comprehensive TDD coverage.

**Tech Stack:** Tauri 2.x, Rust, SQLite (rusqlite), SvelteKit, TypeScript, svelte-i18n, Playwright (E2E)

---

## Phase 1: Project Scaffolding

### Task 1.1: Initialize Tauri + SvelteKit Project

**Files:**
- Create: Project root with Tauri + SvelteKit structure

**Step 1: Create SvelteKit project**

Run:
```bash
npm create svelte@latest kniha-jazd-app
# Select: Skeleton project, TypeScript, ESLint, Prettier
```

**Step 2: Add Tauri**

Run:
```bash
cd kniha-jazd-app
npm install
npm install -D @tauri-apps/cli@latest
npm run tauri init
# App name: Kniha Jázd
# Window title: Kniha Jázd
# Frontend dev URL: http://localhost:5173
# Frontend build command: npm run build
# Frontend dist: build
```

**Step 3: Add Rust dependencies**

Modify: `src-tauri/Cargo.toml`
```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
rand = "0.8"
thiserror = "1"
```

**Step 4: Verify setup**

Run:
```bash
npm run tauri dev
```
Expected: Empty SvelteKit window opens in Tauri

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: initialize Tauri + SvelteKit project"
```

---

### Task 1.2: Create Rust Module Structure

**Files:**
- Create: `src-tauri/src/db.rs`
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/calculations.rs`
- Create: `src-tauri/src/suggestions.rs`
- Create: `src-tauri/src/export.rs`
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Create error module**

Create: `src-tauri/src/error.rs`
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
```

**Step 2: Create empty module files**

Create: `src-tauri/src/models.rs`
```rust
//! Data models for Vehicle, Trip, Route, Settings
```

Create: `src-tauri/src/db.rs`
```rust
//! SQLite database operations
```

Create: `src-tauri/src/calculations.rs`
```rust
//! Core business logic: consumption, margin, zostatok calculations
```

Create: `src-tauri/src/suggestions.rs`
```rust
//! Compensation trip suggestions
```

Create: `src-tauri/src/export.rs`
```rust
//! PDF export functionality
```

**Step 3: Update main.rs with module declarations**

Modify: `src-tauri/src/main.rs`
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod calculations;
mod db;
mod error;
mod export;
mod models;
mod suggestions;

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 4: Verify compilation**

Run:
```bash
cd src-tauri && cargo build
```
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: create Rust module structure"
```

---

## Phase 2: Data Models & Database

### Task 2.1: Define Data Models

**Files:**
- Modify: `src-tauri/src/models.rs`

**Step 1: Write Vehicle model**

```rust
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: Uuid,
    pub name: String,
    pub license_plate: String,
    pub tank_size_liters: f64,
    pub tp_consumption: f64, // l/100km from technical passport
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Vehicle {
    pub fn new(
        name: String,
        license_plate: String,
        tank_size_liters: f64,
        tp_consumption: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            license_plate,
            tank_size_liters,
            tp_consumption,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trip {
    pub id: Uuid,
    pub vehicle_id: Uuid,
    pub date: NaiveDate,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub odometer: f64,
    pub purpose: String,
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub other_costs_eur: Option<f64>,
    pub other_costs_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Trip {
    pub fn is_fillup(&self) -> bool {
        self.fuel_liters.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: Uuid,
    pub vehicle_id: Uuid,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub usage_count: i32,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub id: Uuid,
    pub company_name: String,
    pub company_ico: String,
    pub buffer_trip_purpose: String,
    pub updated_at: DateTime<Utc>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            company_name: String::new(),
            company_ico: String::new(),
            buffer_trip_purpose: "testovanie".to_string(),
            updated_at: Utc::now(),
        }
    }
}
```

**Step 2: Verify compilation**

Run:
```bash
cd src-tauri && cargo build
```
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add -A
git commit -m "feat(models): add Vehicle, Trip, Route, Settings data models"
```

---

### Task 2.2: Setup SQLite Database

**Files:**
- Create: `src-tauri/migrations/001_initial.sql`
- Modify: `src-tauri/src/db.rs`

**Step 1: Create migration file**

Create: `src-tauri/migrations/001_initial.sql`
```sql
-- Vehicles table
CREATE TABLE IF NOT EXISTS vehicles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    license_plate TEXT NOT NULL,
    tank_size_liters REAL NOT NULL,
    tp_consumption REAL NOT NULL,
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
    buffer_trip_purpose TEXT NOT NULL DEFAULT 'testovanie',
    updated_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_trips_vehicle_date ON trips(vehicle_id, date);
CREATE INDEX IF NOT EXISTS idx_routes_vehicle ON routes(vehicle_id);
```

**Step 2: Implement database module**

Modify: `src-tauri/src/db.rs`
```rust
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        Ok(())
    }

    pub fn connection(&self) -> std::sync::MutexGuard<Connection> {
        self.conn.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::in_memory().expect("Failed to create database");
        let conn = db.connection();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"vehicles".to_string()));
        assert!(tables.contains(&"trips".to_string()));
        assert!(tables.contains(&"routes".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }
}
```

**Step 3: Run test**

Run:
```bash
cd src-tauri && cargo test test_database_creation -- --nocapture
```
Expected: PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(db): add SQLite database with migrations"
```

---

## Phase 3: Core Calculation Logic (TDD)

### Task 3.1: Consumption Rate Calculation

**Files:**
- Modify: `src-tauri/src/calculations.rs`

**Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consumption_rate_normal() {
        // 50 liters / 820 km = 6.097... l/100km
        let rate = calculate_consumption_rate(50.0, 820.0);
        assert!((rate - 6.0975).abs() < 0.001);
    }

    #[test]
    fn test_consumption_rate_exact_match() {
        // 50.36 liters / 828 km = 6.0821... l/100km (from Excel)
        let rate = calculate_consumption_rate(50.36, 828.0);
        assert!((rate - 6.0821).abs() < 0.001);
    }
}
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_consumption_rate -- --nocapture
```
Expected: FAIL with "cannot find function `calculate_consumption_rate`"

**Step 3: Write minimal implementation**

Add to `src-tauri/src/calculations.rs`:
```rust
/// Calculate fuel consumption rate (l/100km) from a fill-up
/// Formula: liters / km * 100
pub fn calculate_consumption_rate(liters: f64, km_since_last_fillup: f64) -> f64 {
    if km_since_last_fillup <= 0.0 {
        return 0.0;
    }
    (liters / km_since_last_fillup) * 100.0
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_consumption_rate -- --nocapture
```
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(calc): add consumption rate calculation"
```

---

### Task 3.2: Spotreba (Fuel Used) Calculation

**Files:**
- Modify: `src-tauri/src/calculations.rs`

**Step 1: Write failing test**

Add to tests:
```rust
    #[test]
    fn test_spotreba_calculation() {
        // 370 km at 6.08 l/100km = 22.496 liters
        let spotreba = calculate_spotreba(370.0, 6.08);
        assert!((spotreba - 22.496).abs() < 0.01);
    }

    #[test]
    fn test_spotreba_short_distance() {
        // 55 km at 6.08 l/100km = 3.344 liters
        let spotreba = calculate_spotreba(55.0, 6.08);
        assert!((spotreba - 3.344).abs() < 0.01);
    }
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_spotreba -- --nocapture
```
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
/// Calculate fuel used for a trip (spotreba)
/// Formula: km * consumption_rate / 100
pub fn calculate_spotreba(distance_km: f64, consumption_rate: f64) -> f64 {
    distance_km * consumption_rate / 100.0
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_spotreba -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(calc): add spotreba calculation"
```

---

### Task 3.3: Zostatok (Remaining Fuel) Calculation

**Files:**
- Modify: `src-tauri/src/calculations.rs`

**Step 1: Write failing tests**

```rust
    #[test]
    fn test_zostatok_after_trip() {
        // Start with 66L, use 22.5L = 43.5L remaining
        let zostatok = calculate_zostatok(66.0, 22.5, None, 66.0);
        assert!((zostatok - 43.5).abs() < 0.01);
    }

    #[test]
    fn test_zostatok_after_fillup() {
        // Start with 5L, use 2L, add 63L = 66L (capped at tank size)
        let zostatok = calculate_zostatok(5.0, 2.0, Some(63.0), 66.0);
        assert!((zostatok - 66.0).abs() < 0.01);
    }

    #[test]
    fn test_zostatok_caps_at_tank_size() {
        // Overfill scenario: 50L - 10L + 30L = 70L, but capped at 66L
        let zostatok = calculate_zostatok(50.0, 10.0, Some(30.0), 66.0);
        assert!((zostatok - 66.0).abs() < 0.01);
    }
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_zostatok -- --nocapture
```
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
/// Calculate remaining fuel in tank (zostatok)
/// Formula: previous - spotreba + fuel_added (capped at tank_size)
pub fn calculate_zostatok(
    previous_zostatok: f64,
    spotreba: f64,
    fuel_added: Option<f64>,
    tank_size: f64,
) -> f64 {
    let new_zostatok = previous_zostatok - spotreba + fuel_added.unwrap_or(0.0);
    new_zostatok.min(tank_size).max(0.0)
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_zostatok -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(calc): add zostatok calculation"
```

---

### Task 3.4: Margin Calculation

**Files:**
- Modify: `src-tauri/src/calculations.rs`

**Step 1: Write failing tests**

```rust
    #[test]
    fn test_margin_under_limit() {
        // 6.0 / 5.1 = 17.6% over
        let margin = calculate_margin_percent(6.0, 5.1);
        assert!((margin - 17.647).abs() < 0.1);
        assert!(is_within_legal_limit(margin));
    }

    #[test]
    fn test_margin_at_limit() {
        // 6.12 / 5.1 = exactly 20%
        let margin = calculate_margin_percent(6.12, 5.1);
        assert!((margin - 20.0).abs() < 0.1);
        assert!(is_within_legal_limit(margin));
    }

    #[test]
    fn test_margin_over_limit() {
        // 6.5 / 5.1 = 27.45% over
        let margin = calculate_margin_percent(6.5, 5.1);
        assert!((margin - 27.45).abs() < 0.1);
        assert!(!is_within_legal_limit(margin));
    }

    #[test]
    fn test_margin_under_tp() {
        // 4.5 / 5.1 = -11.76% (better than TP)
        let margin = calculate_margin_percent(4.5, 5.1);
        assert!(margin < 0.0);
        assert!(is_within_legal_limit(margin));
    }
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_margin -- --nocapture
```
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
/// Calculate margin percentage vs TP consumption
/// Formula: (actual / tp - 1) * 100
pub fn calculate_margin_percent(consumption_rate: f64, tp_rate: f64) -> f64 {
    if tp_rate <= 0.0 {
        return 0.0;
    }
    (consumption_rate / tp_rate - 1.0) * 100.0
}

/// Check if consumption is within legal limit (max 20% over TP)
pub fn is_within_legal_limit(margin_percent: f64) -> bool {
    margin_percent <= 20.0
}

/// Legal limit constant
pub const LEGAL_MARGIN_LIMIT: f64 = 20.0;
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_margin -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(calc): add margin calculation and legal limit check"
```

---

### Task 3.5: Buffer KM Calculation (for compensation)

**Files:**
- Modify: `src-tauri/src/calculations.rs`

**Step 1: Write failing tests**

```rust
    #[test]
    fn test_buffer_km_calculation() {
        // 50L filled, 800km driven, TP=5.1, target=18%
        // Target rate = 5.1 * 1.18 = 6.018
        // Required km = 50 * 100 / 6.018 = 830.93
        // Buffer = 830.93 - 800 = 30.93 km
        let buffer = calculate_buffer_km(50.0, 800.0, 5.1, 0.18);
        assert!((buffer - 30.93).abs() < 1.0);
    }

    #[test]
    fn test_buffer_km_already_compliant() {
        // Already under target, buffer should be 0 or negative
        let buffer = calculate_buffer_km(50.0, 1000.0, 5.1, 0.18);
        assert!(buffer <= 0.0);
    }
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_buffer_km -- --nocapture
```
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
/// Calculate buffer km needed to reach target margin
/// Returns km to add (negative if already compliant)
pub fn calculate_buffer_km(
    liters_filled: f64,
    km_driven: f64,
    tp_rate: f64,
    target_margin: f64, // e.g., 0.18 for 18%
) -> f64 {
    let target_rate = tp_rate * (1.0 + target_margin);
    let required_km = (liters_filled * 100.0) / target_rate;
    required_km - km_driven
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_buffer_km -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(calc): add buffer km calculation for compensation"
```

---

## Phase 4: Suggestion Logic (TDD)

### Task 4.1: Target Margin Generation

**Files:**
- Modify: `src-tauri/src/suggestions.rs`

**Step 1: Write failing test**

```rust
use rand::Rng;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_target_margin_in_range() {
        // Generate 100 values, all should be in 0.16-0.19 range
        for _ in 0..100 {
            let target = generate_target_margin();
            assert!(target >= 0.16, "Target {} is below 0.16", target);
            assert!(target <= 0.19, "Target {} is above 0.19", target);
        }
    }
}
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_random_target -- --nocapture
```
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
use rand::Rng;

/// Generate random target margin between 16-19%
/// This makes consumption values look natural, not artificially consistent
pub fn generate_target_margin() -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0.16..=0.19)
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_random_target -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(suggestions): add random target margin generation"
```

---

### Task 4.2: Route Matching for Compensation

**Files:**
- Modify: `src-tauri/src/suggestions.rs`

**Step 1: Write failing tests**

```rust
    #[test]
    fn test_find_matching_route_exact() {
        let routes = vec![
            ("A", "B", 50.0),
            ("B", "C", 100.0),
            ("A", "A", 42.0),
        ];

        let result = find_matching_route(&routes, "A", 42.0, 0.1);
        assert!(result.is_some());
        let (origin, dest, km) = result.unwrap();
        assert_eq!(origin, "A");
        assert_eq!(dest, "A");
        assert_eq!(km, 42.0);
    }

    #[test]
    fn test_find_matching_route_within_tolerance() {
        let routes = vec![
            ("A", "B", 50.0),
            ("B", "C", 100.0),
        ];

        // Looking for ~48km, should match 50km (within 10%)
        let result = find_matching_route(&routes, "A", 48.0, 0.1);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_matching_route_no_match() {
        let routes = vec![
            ("A", "B", 50.0),
            ("B", "C", 100.0),
        ];

        // Looking for 200km from A, no match
        let result = find_matching_route(&routes, "A", 200.0, 0.1);
        assert!(result.is_none());
    }
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_find_matching_route -- --nocapture
```
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
/// Find a route from current location that matches needed km (within tolerance)
pub fn find_matching_route<'a>(
    routes: &'a [(&'a str, &'a str, f64)],
    current_location: &str,
    needed_km: f64,
    tolerance: f64, // e.g., 0.1 for ±10%
) -> Option<(&'a str, &'a str, f64)> {
    let min_km = needed_km * (1.0 - tolerance);
    let max_km = needed_km * (1.0 + tolerance);

    routes
        .iter()
        .filter(|(origin, _, km)| {
            *origin == current_location && *km >= min_km && *km <= max_km
        })
        .min_by(|a, b| {
            let diff_a = (a.2 - needed_km).abs();
            let diff_b = (b.2 - needed_km).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        })
        .map(|&(o, d, k)| (o, d, k))
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_find_matching_route -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(suggestions): add route matching for compensation trips"
```

---

### Task 4.3: Compensation Suggestion Builder

**Files:**
- Modify: `src-tauri/src/suggestions.rs`

**Step 1: Write failing test**

```rust
    use crate::models::Route;

    #[test]
    fn test_build_compensation_suggestion_with_route() {
        let routes = vec![
            Route {
                id: uuid::Uuid::new_v4(),
                vehicle_id: uuid::Uuid::new_v4(),
                origin: "Nejaka Ulica 123".to_string(),
                destination: "Ina Ulica 456".to_string(),
                distance_km: 45.0,
                usage_count: 5,
                last_used: chrono::Utc::now(),
            },
        ];

        let suggestion = build_compensation_suggestion(
            &routes,
            "Nejaka Ulica 123",
            42.0,
            "testovanie",
        );

        // Should find the 45km route (within 10% of 42km)
        assert_eq!(suggestion.origin, "Nejaka Ulica 123");
        assert_eq!(suggestion.destination, "Ina Ulica 456");
        assert!(!suggestion.is_buffer);
    }

    #[test]
    fn test_build_compensation_suggestion_filler() {
        let routes: Vec<Route> = vec![]; // No routes

        let suggestion = build_compensation_suggestion(
            &routes,
            "Nejaka Ulica 123",
            42.0,
            "testovanie",
        );

        // Should create filler trip
        assert_eq!(suggestion.origin, "Nejaka Ulica 123");
        assert_eq!(suggestion.destination, "Nejaka Ulica 123");
        assert_eq!(suggestion.distance_km, 42.0);
        assert_eq!(suggestion.purpose, "testovanie");
        assert!(suggestion.is_buffer);
    }
```

**Step 2: Run test to verify it fails**

Run:
```bash
cd src-tauri && cargo test test_build_compensation -- --nocapture
```
Expected: FAIL

**Step 3: Write minimal implementation**

Add to `src-tauri/src/suggestions.rs`:
```rust
use crate::models::Route;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompensationSuggestion {
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub purpose: String,
    pub is_buffer: bool,
}

/// Build a compensation suggestion - try existing routes first, fall back to filler
pub fn build_compensation_suggestion(
    routes: &[Route],
    current_location: &str,
    needed_km: f64,
    filler_purpose: &str,
) -> CompensationSuggestion {
    // Convert routes to tuple format for matching
    let route_tuples: Vec<(&str, &str, f64)> = routes
        .iter()
        .map(|r| (r.origin.as_str(), r.destination.as_str(), r.distance_km))
        .collect();

    // Try to find matching route
    if let Some((origin, dest, km)) = find_matching_route(&route_tuples, current_location, needed_km, 0.1) {
        return CompensationSuggestion {
            origin: origin.to_string(),
            destination: dest.to_string(),
            distance_km: km,
            purpose: filler_purpose.to_string(),
            is_buffer: false,
        };
    }

    // Fall back to filler trip
    CompensationSuggestion {
        origin: current_location.to_string(),
        destination: current_location.to_string(),
        distance_km: needed_km,
        purpose: filler_purpose.to_string(),
        is_buffer: true,
    }
}
```

**Step 4: Run test to verify it passes**

Run:
```bash
cd src-tauri && cargo test test_build_compensation -- --nocapture
```
Expected: PASS

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(suggestions): add compensation suggestion builder"
```

---

## Phase 5: Database CRUD Operations

### Task 5.1: Vehicle CRUD

**Files:**
- Modify: `src-tauri/src/db.rs`

*[See 02-design.md for implementation details]*

**Commit after implementation:**
```bash
git add -A
git commit -m "feat(db): add vehicle CRUD operations"
```

---

### Task 5.2: Trip CRUD

**Files:**
- Modify: `src-tauri/src/db.rs`

*[See 02-design.md for implementation details]*

**Commit after implementation:**
```bash
git add -A
git commit -m "feat(db): add trip CRUD operations"
```

---

### Task 5.3: Route CRUD

**Files:**
- Modify: `src-tauri/src/db.rs`

*[See 02-design.md for implementation details]*

**Commit after implementation:**
```bash
git add -A
git commit -m "feat(db): add route CRUD for autocomplete"
```

---

## Phase 6: Tauri Commands

### Task 6.1: Vehicle Commands
### Task 6.2: Trip Commands
### Task 6.3: Route Commands
### Task 6.4: Calculation Commands

*[Implementation follows same pattern - see 02-design.md]*

---

## Phase 7: Frontend Shell

### Task 7.1: Setup SvelteKit with i18n
### Task 7.2: Create Layout with Vehicle Selector
### Task 7.3: Create API bindings to Tauri

---

## Phase 8: Trip Grid Component

### Task 8.1: TripGrid Component
### Task 8.2: Inline Editing
### Task 8.3: Autocomplete Component
### Task 8.4: Compensation Suggestion Component

---

## Phase 9: Settings & Export

### Task 9.1: Settings Page
### Task 9.2: PDF Export

---

## Phase 10: Polish & Testing

### Task 10.1: E2E Tests with Playwright
### Task 10.2: Final i18n Pass

---

## Execution Notes

- Each phase builds on the previous
- Run `cargo test` after each Rust change
- Commit after each passing test
- Frontend phases can start after Phase 6 is complete
- Phases 7-10 are outlined - will be detailed when reached
