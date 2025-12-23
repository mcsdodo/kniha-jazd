use crate::models::Vehicle;
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
}
