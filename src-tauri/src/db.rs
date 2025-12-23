use crate::models::{Trip, Vehicle};
use rusqlite::{Connection, OptionalExtension, Result};
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

    // Vehicle CRUD operations

    /// Create a new vehicle in the database
    pub fn create_vehicle(&self, vehicle: &Vehicle) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vehicles (id, name, license_plate, tank_size_liters, tp_consumption, is_active, created_at, updated_at)
             VALUES (:id, :name, :license_plate, :tank_size_liters, :tp_consumption, :is_active, :created_at, :updated_at)",
            rusqlite::named_params! {
                ":id": vehicle.id.to_string(),
                ":name": vehicle.name,
                ":license_plate": vehicle.license_plate,
                ":tank_size_liters": vehicle.tank_size_liters,
                ":tp_consumption": vehicle.tp_consumption,
                ":is_active": vehicle.is_active,
                ":created_at": vehicle.created_at.to_rfc3339(),
                ":updated_at": vehicle.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Get a vehicle by its UUID string
    pub fn get_vehicle(&self, id: &str) -> Result<Option<Vehicle>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, license_plate, tank_size_liters, tp_consumption, is_active, created_at, updated_at
             FROM vehicles WHERE id = ?1",
        )?;

        let vehicle = stmt
            .query_row([id], |row| {
                Ok(Vehicle {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    name: row.get(1)?,
                    license_plate: row.get(2)?,
                    tank_size_liters: row.get(3)?,
                    tp_consumption: row.get(4)?,
                    is_active: row.get(5)?,
                    created_at: row.get::<_, String>(6)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(7)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(vehicle)
    }

    /// Get all vehicles from the database
    pub fn get_all_vehicles(&self) -> Result<Vec<Vehicle>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, license_plate, tank_size_liters, tp_consumption, is_active, created_at, updated_at
             FROM vehicles ORDER BY created_at DESC",
        )?;

        let vehicles = stmt
            .query_map([], |row| {
                Ok(Vehicle {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    name: row.get(1)?,
                    license_plate: row.get(2)?,
                    tank_size_liters: row.get(3)?,
                    tp_consumption: row.get(4)?,
                    is_active: row.get(5)?,
                    created_at: row.get::<_, String>(6)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(7)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vehicles)
    }

    /// Get the currently active vehicle
    pub fn get_active_vehicle(&self) -> Result<Option<Vehicle>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, license_plate, tank_size_liters, tp_consumption, is_active, created_at, updated_at
             FROM vehicles WHERE is_active = 1 LIMIT 1",
        )?;

        let vehicle = stmt
            .query_row([], |row| {
                Ok(Vehicle {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    name: row.get(1)?,
                    license_plate: row.get(2)?,
                    tank_size_liters: row.get(3)?,
                    tp_consumption: row.get(4)?,
                    is_active: row.get(5)?,
                    created_at: row.get::<_, String>(6)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(7)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(vehicle)
    }

    /// Update an existing vehicle
    pub fn update_vehicle(&self, vehicle: &Vehicle) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE vehicles
             SET name = :name,
                 license_plate = :license_plate,
                 tank_size_liters = :tank_size_liters,
                 tp_consumption = :tp_consumption,
                 is_active = :is_active,
                 updated_at = :updated_at
             WHERE id = :id",
            rusqlite::named_params! {
                ":id": vehicle.id.to_string(),
                ":name": vehicle.name,
                ":license_plate": vehicle.license_plate,
                ":tank_size_liters": vehicle.tank_size_liters,
                ":tp_consumption": vehicle.tp_consumption,
                ":is_active": vehicle.is_active,
                ":updated_at": vehicle.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Delete a vehicle by its UUID string
    pub fn delete_vehicle(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM vehicles WHERE id = ?1", [id])?;
        Ok(())
    }

    // Trip CRUD operations

    /// Create a new trip in the database
    pub fn create_trip(&self, trip: &Trip) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO trips (id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, created_at, updated_at)
             VALUES (:id, :vehicle_id, :date, :origin, :destination, :distance_km, :odometer, :purpose, :fuel_liters, :fuel_cost_eur, :other_costs_eur, :other_costs_note, :created_at, :updated_at)",
            rusqlite::named_params! {
                ":id": trip.id.to_string(),
                ":vehicle_id": trip.vehicle_id.to_string(),
                ":date": trip.date.to_string(),
                ":origin": trip.origin,
                ":destination": trip.destination,
                ":distance_km": trip.distance_km,
                ":odometer": trip.odometer,
                ":purpose": trip.purpose,
                ":fuel_liters": trip.fuel_liters,
                ":fuel_cost_eur": trip.fuel_cost_eur,
                ":other_costs_eur": trip.other_costs_eur,
                ":other_costs_note": trip.other_costs_note,
                ":created_at": trip.created_at.to_rfc3339(),
                ":updated_at": trip.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Get a trip by its UUID string
    pub fn get_trip(&self, id: &str) -> Result<Option<Trip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, created_at, updated_at
             FROM trips WHERE id = ?1",
        )?;

        let trip = stmt
            .query_row([id], |row| {
                Ok(Trip {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    date: row.get::<_, String>(2)?.parse().unwrap(),
                    origin: row.get(3)?,
                    destination: row.get(4)?,
                    distance_km: row.get(5)?,
                    odometer: row.get(6)?,
                    purpose: row.get(7)?,
                    fuel_liters: row.get(8)?,
                    fuel_cost_eur: row.get(9)?,
                    other_costs_eur: row.get(10)?,
                    other_costs_note: row.get(11)?,
                    created_at: row.get::<_, String>(12)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(13)?.parse().unwrap(),
                })
            })
            .optional()?;

        Ok(trip)
    }

    /// Get all trips for a vehicle, ordered by date DESC (newest first)
    pub fn get_trips_for_vehicle(&self, vehicle_id: &str) -> Result<Vec<Trip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, created_at, updated_at
             FROM trips WHERE vehicle_id = ?1 ORDER BY date DESC",
        )?;

        let trips = stmt
            .query_map([vehicle_id], |row| {
                Ok(Trip {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    date: row.get::<_, String>(2)?.parse().unwrap(),
                    origin: row.get(3)?,
                    destination: row.get(4)?,
                    distance_km: row.get(5)?,
                    odometer: row.get(6)?,
                    purpose: row.get(7)?,
                    fuel_liters: row.get(8)?,
                    fuel_cost_eur: row.get(9)?,
                    other_costs_eur: row.get(10)?,
                    other_costs_note: row.get(11)?,
                    created_at: row.get::<_, String>(12)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(13)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(trips)
    }

    /// Get all trips for a vehicle in a specific year, ordered by date ASC (chronological)
    pub fn get_trips_for_vehicle_in_year(&self, vehicle_id: &str, year: i32) -> Result<Vec<Trip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, date, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, other_costs_eur, other_costs_note, created_at, updated_at
             FROM trips
             WHERE vehicle_id = ?1 AND strftime('%Y', date) = ?2
             ORDER BY date ASC",
        )?;

        let trips = stmt
            .query_map([vehicle_id, &year.to_string()], |row| {
                Ok(Trip {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    vehicle_id: row.get::<_, String>(1)?.parse().unwrap(),
                    date: row.get::<_, String>(2)?.parse().unwrap(),
                    origin: row.get(3)?,
                    destination: row.get(4)?,
                    distance_km: row.get(5)?,
                    odometer: row.get(6)?,
                    purpose: row.get(7)?,
                    fuel_liters: row.get(8)?,
                    fuel_cost_eur: row.get(9)?,
                    other_costs_eur: row.get(10)?,
                    other_costs_note: row.get(11)?,
                    created_at: row.get::<_, String>(12)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(13)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(trips)
    }

    /// Update an existing trip
    pub fn update_trip(&self, trip: &Trip) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE trips
             SET vehicle_id = :vehicle_id,
                 date = :date,
                 origin = :origin,
                 destination = :destination,
                 distance_km = :distance_km,
                 odometer = :odometer,
                 purpose = :purpose,
                 fuel_liters = :fuel_liters,
                 fuel_cost_eur = :fuel_cost_eur,
                 other_costs_eur = :other_costs_eur,
                 other_costs_note = :other_costs_note,
                 updated_at = :updated_at
             WHERE id = :id",
            rusqlite::named_params! {
                ":id": trip.id.to_string(),
                ":vehicle_id": trip.vehicle_id.to_string(),
                ":date": trip.date.to_string(),
                ":origin": trip.origin,
                ":destination": trip.destination,
                ":distance_km": trip.distance_km,
                ":odometer": trip.odometer,
                ":purpose": trip.purpose,
                ":fuel_liters": trip.fuel_liters,
                ":fuel_cost_eur": trip.fuel_cost_eur,
                ":other_costs_eur": trip.other_costs_eur,
                ":other_costs_note": trip.other_costs_note,
                ":updated_at": trip.updated_at.to_rfc3339(),
            },
        )?;
        Ok(())
    }

    /// Delete a trip by its UUID string
    pub fn delete_trip(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM trips WHERE id = ?1", [id])?;
        Ok(())
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

    #[test]
    fn test_create_and_retrieve_vehicle() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new(
            "Test Car".to_string(),
            "BA123XY".to_string(),
            66.0,
            5.1,
        );

        // Create vehicle
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // Retrieve it
        let retrieved = db
            .get_vehicle(&vehicle.id.to_string())
            .expect("Failed to get vehicle")
            .expect("Vehicle not found");

        assert_eq!(retrieved.id, vehicle.id);
        assert_eq!(retrieved.name, "Test Car");
        assert_eq!(retrieved.license_plate, "BA123XY");
        assert_eq!(retrieved.tank_size_liters, 66.0);
        assert_eq!(retrieved.tp_consumption, 5.1);
        assert!(retrieved.is_active);
    }

    #[test]
    fn test_get_all_vehicles() {
        let db = Database::in_memory().expect("Failed to create database");

        // Create multiple vehicles
        let v1 = Vehicle::new("Car 1".to_string(), "BA111AA".to_string(), 60.0, 5.0);
        let v2 = Vehicle::new("Car 2".to_string(), "BA222BB".to_string(), 70.0, 6.0);

        db.create_vehicle(&v1).expect("Failed to create v1");
        db.create_vehicle(&v2).expect("Failed to create v2");

        // Get all
        let vehicles = db.get_all_vehicles().expect("Failed to get all vehicles");

        assert_eq!(vehicles.len(), 2);
        assert!(vehicles.iter().any(|v| v.id == v1.id));
        assert!(vehicles.iter().any(|v| v.id == v2.id));
    }

    #[test]
    fn test_get_active_vehicle() {
        let db = Database::in_memory().expect("Failed to create database");

        // Create two vehicles
        let mut v1 = Vehicle::new("Car 1".to_string(), "BA111AA".to_string(), 60.0, 5.0);
        v1.is_active = false;
        let v2 = Vehicle::new("Car 2".to_string(), "BA222BB".to_string(), 70.0, 6.0);

        db.create_vehicle(&v1).expect("Failed to create v1");
        db.create_vehicle(&v2).expect("Failed to create v2");

        // Get active vehicle
        let active = db
            .get_active_vehicle()
            .expect("Failed to get active vehicle")
            .expect("No active vehicle found");

        assert_eq!(active.id, v2.id);
        assert!(active.is_active);
    }

    #[test]
    fn test_update_vehicle() {
        let db = Database::in_memory().expect("Failed to create database");
        let mut vehicle = Vehicle::new("Old Name".to_string(), "BA111AA".to_string(), 60.0, 5.0);

        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // Update vehicle
        vehicle.name = "New Name".to_string();
        vehicle.license_plate = "BA999ZZ".to_string();
        vehicle.is_active = false;

        db.update_vehicle(&vehicle).expect("Failed to update vehicle");

        // Retrieve and verify
        let updated = db
            .get_vehicle(&vehicle.id.to_string())
            .expect("Failed to get vehicle")
            .expect("Vehicle not found");

        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.license_plate, "BA999ZZ");
        assert!(!updated.is_active);
    }

    #[test]
    fn test_delete_vehicle() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1);

        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // Delete vehicle
        db.delete_vehicle(&vehicle.id.to_string())
            .expect("Failed to delete vehicle");

        // Verify it's gone
        let result = db
            .get_vehicle(&vehicle.id.to_string())
            .expect("Failed to get vehicle");

        assert!(result.is_none());
    }

    #[test]
    fn test_get_nonexistent_vehicle_returns_none() {
        let db = Database::in_memory().expect("Failed to create database");

        let result = db
            .get_vehicle("00000000-0000-0000-0000-000000000000")
            .expect("Failed to query vehicle");

        assert!(result.is_none());
    }

    // Trip CRUD tests

    fn create_test_trip(vehicle_id: uuid::Uuid, date: &str) -> Trip {
        use chrono::NaiveDate;
        let now = chrono::Utc::now();
        Trip {
            id: uuid::Uuid::new_v4(),
            vehicle_id,
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            origin: "Prague".to_string(),
            destination: "Brno".to_string(),
            distance_km: 200.0,
            odometer: 50000.0,
            purpose: "Business meeting".to_string(),
            fuel_liters: Some(15.0),
            fuel_cost_eur: Some(25.5),
            other_costs_eur: Some(5.0),
            other_costs_note: Some("Parking fee".to_string()),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_create_and_retrieve_trip() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let trip = create_test_trip(vehicle.id, "2024-12-01");

        // Create trip
        db.create_trip(&trip).expect("Failed to create trip");

        // Retrieve it
        let retrieved = db
            .get_trip(&trip.id.to_string())
            .expect("Failed to get trip")
            .expect("Trip not found");

        assert_eq!(retrieved.id, trip.id);
        assert_eq!(retrieved.vehicle_id, vehicle.id);
        assert_eq!(retrieved.date.to_string(), "2024-12-01");
        assert_eq!(retrieved.origin, "Prague");
        assert_eq!(retrieved.destination, "Brno");
        assert_eq!(retrieved.distance_km, 200.0);
        assert_eq!(retrieved.odometer, 50000.0);
        assert_eq!(retrieved.purpose, "Business meeting");
        assert_eq!(retrieved.fuel_liters, Some(15.0));
        assert_eq!(retrieved.fuel_cost_eur, Some(25.5));
        assert_eq!(retrieved.other_costs_eur, Some(5.0));
        assert_eq!(retrieved.other_costs_note, Some("Parking fee".to_string()));
    }

    #[test]
    fn test_create_trip_with_optional_fields_none() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let now = chrono::Utc::now();
        let trip = Trip {
            id: uuid::Uuid::new_v4(),
            vehicle_id: vehicle.id,
            date: chrono::NaiveDate::from_ymd_opt(2024, 12, 1).unwrap(),
            origin: "Vienna".to_string(),
            destination: "Graz".to_string(),
            distance_km: 150.0,
            odometer: 50200.0,
            purpose: "Personal trip".to_string(),
            fuel_liters: None,
            fuel_cost_eur: None,
            other_costs_eur: None,
            other_costs_note: None,
            created_at: now,
            updated_at: now,
        };

        db.create_trip(&trip).expect("Failed to create trip");

        let retrieved = db
            .get_trip(&trip.id.to_string())
            .expect("Failed to get trip")
            .expect("Trip not found");

        assert_eq!(retrieved.fuel_liters, None);
        assert_eq!(retrieved.fuel_cost_eur, None);
        assert_eq!(retrieved.other_costs_eur, None);
        assert_eq!(retrieved.other_costs_note, None);
    }

    #[test]
    fn test_get_trips_for_vehicle() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // Create trips with different dates
        let trip1 = create_test_trip(vehicle.id, "2024-12-01");
        let trip2 = create_test_trip(vehicle.id, "2024-12-15");
        let trip3 = create_test_trip(vehicle.id, "2024-12-10");

        db.create_trip(&trip1).expect("Failed to create trip1");
        db.create_trip(&trip2).expect("Failed to create trip2");
        db.create_trip(&trip3).expect("Failed to create trip3");

        // Get all trips for vehicle
        let trips = db
            .get_trips_for_vehicle(&vehicle.id.to_string())
            .expect("Failed to get trips");

        assert_eq!(trips.len(), 3);

        // Verify ordering: DESC (newest first)
        assert_eq!(trips[0].date.to_string(), "2024-12-15");
        assert_eq!(trips[1].date.to_string(), "2024-12-10");
        assert_eq!(trips[2].date.to_string(), "2024-12-01");
    }

    #[test]
    fn test_get_trips_for_vehicle_in_year() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        // Create trips in different years
        let trip1 = create_test_trip(vehicle.id, "2024-12-01");
        let trip2 = create_test_trip(vehicle.id, "2024-06-15");
        let trip3 = create_test_trip(vehicle.id, "2023-12-10");
        let trip4 = create_test_trip(vehicle.id, "2024-01-05");

        db.create_trip(&trip1).expect("Failed to create trip1");
        db.create_trip(&trip2).expect("Failed to create trip2");
        db.create_trip(&trip3).expect("Failed to create trip3");
        db.create_trip(&trip4).expect("Failed to create trip4");

        // Get trips for 2024
        let trips_2024 = db
            .get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2024)
            .expect("Failed to get trips for 2024");

        assert_eq!(trips_2024.len(), 3);

        // Verify ordering: ASC (chronological for export)
        assert_eq!(trips_2024[0].date.to_string(), "2024-01-05");
        assert_eq!(trips_2024[1].date.to_string(), "2024-06-15");
        assert_eq!(trips_2024[2].date.to_string(), "2024-12-01");

        // Get trips for 2023
        let trips_2023 = db
            .get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2023)
            .expect("Failed to get trips for 2023");

        assert_eq!(trips_2023.len(), 1);
        assert_eq!(trips_2023[0].date.to_string(), "2023-12-10");
    }

    #[test]
    fn test_update_trip() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let mut trip = create_test_trip(vehicle.id, "2024-12-01");

        db.create_trip(&trip).expect("Failed to create trip");

        // Update trip
        trip.origin = "Berlin".to_string();
        trip.destination = "Munich".to_string();
        trip.distance_km = 350.0;
        trip.fuel_liters = Some(25.0);
        trip.other_costs_note = Some("Updated note".to_string());

        db.update_trip(&trip).expect("Failed to update trip");

        // Retrieve and verify
        let updated = db
            .get_trip(&trip.id.to_string())
            .expect("Failed to get trip")
            .expect("Trip not found");

        assert_eq!(updated.origin, "Berlin");
        assert_eq!(updated.destination, "Munich");
        assert_eq!(updated.distance_km, 350.0);
        assert_eq!(updated.fuel_liters, Some(25.0));
        assert_eq!(updated.other_costs_note, Some("Updated note".to_string()));
    }

    #[test]
    fn test_delete_trip() {
        let db = Database::in_memory().expect("Failed to create database");
        let vehicle = Vehicle::new("Test Car".to_string(), "BA123XY".to_string(), 66.0, 5.1);
        db.create_vehicle(&vehicle).expect("Failed to create vehicle");

        let trip = create_test_trip(vehicle.id, "2024-12-01");

        db.create_trip(&trip).expect("Failed to create trip");

        // Delete trip
        db.delete_trip(&trip.id.to_string())
            .expect("Failed to delete trip");

        // Verify it's gone
        let result = db
            .get_trip(&trip.id.to_string())
            .expect("Failed to query trip");

        assert!(result.is_none());
    }

    #[test]
    fn test_get_nonexistent_trip_returns_none() {
        let db = Database::in_memory().expect("Failed to create database");

        let result = db
            .get_trip("00000000-0000-0000-0000-000000000000")
            .expect("Failed to query trip");

        assert!(result.is_none());
    }

    #[test]
    fn test_get_trips_for_nonexistent_vehicle() {
        let db = Database::in_memory().expect("Failed to create database");

        let trips = db
            .get_trips_for_vehicle("00000000-0000-0000-0000-000000000000")
            .expect("Failed to get trips");

        assert_eq!(trips.len(), 0);
    }
}
