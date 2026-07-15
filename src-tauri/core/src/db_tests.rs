//! Tests for the Diesel-based Database implementation.

use super::*;
use crate::models::{ReceiptStatus, VehicleType};
use chrono::{NaiveDate, NaiveDateTime};

#[test]
fn test_database_creation() {
    let db = Database::in_memory().expect("Failed to create database");
    let _conn = db.connection();
    // If we got here, tables were created by migration
}

// ============================================================================
// Pre-migration safety backup (Database::new)
// ============================================================================

/// Create a valid, non-empty SQLite file WITHOUT any app migrations applied —
/// simulates a database created by an older app version (all migrations pending).
fn create_legacy_db_file(db_path: &std::path::Path) {
    let mut conn = SqliteConnection::establish(db_path.to_str().unwrap())
        .expect("create legacy sqlite file");
    diesel::sql_query("CREATE TABLE legacy_marker (id INTEGER PRIMARY KEY)")
        .execute(&mut conn)
        .expect("create marker table");
    assert!(std::fs::metadata(db_path).unwrap().len() > 0);
}

#[test]
fn test_pre_migration_backup_created_for_existing_db_with_pending_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("kniha-jazd.db");
    create_legacy_db_file(&db_path);

    let _db = Database::new(db_path).expect("open + migrate");

    let backups_dir = dir.path().join("backups");
    assert!(backups_dir.exists(), "backups dir must be created");
    let names: Vec<String> = std::fs::read_dir(&backups_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(names.len(), 1, "exactly one pre-migration backup expected");
    assert!(names[0].starts_with("kniha-jazd-backup-"));
    assert!(
        names[0].contains("-pre-migration-v"),
        "filename must carry the pre-migration marker: {}",
        names[0]
    );
    assert!(names[0].ends_with(".db"));
}

#[test]
fn test_no_pre_migration_backup_for_fresh_or_up_to_date_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("kniha-jazd.db");

    // Fresh DB: file does not pre-exist -> nothing to back up
    drop(Database::new(db_path.clone()).expect("create fresh DB"));
    assert!(!dir.path().join("backups").exists());

    // Re-open with all migrations already applied -> still no backup
    drop(Database::new(db_path).expect("re-open up-to-date DB"));
    assert!(!dir.path().join("backups").exists());
}

#[test]
fn test_in_memory_db_skips_pre_migration_backup() {
    // In-memory DBs have no file to back up; Database::in_memory must not
    // attempt one and must come up fully migrated.
    let db = Database::in_memory().expect("in-memory DB must skip backup logic");
    assert!(db.get_all_vehicles().is_ok());
}

#[test]
fn test_failed_pre_migration_backup_does_not_block_startup() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("kniha-jazd.db");
    create_legacy_db_file(&db_path);

    // Occupy the backups path with a FILE so the backup directory cannot be
    // created — the backup attempt must fail, but startup must continue.
    std::fs::write(dir.path().join("backups"), b"not a directory").unwrap();

    let db = Database::new(db_path).expect("startup must continue when backup fails");
    // Migrations ran despite the failed backup
    assert!(db.get_all_vehicles().is_ok());
}

// Helper to create test vehicles
pub(crate) fn create_test_vehicle(name: &str) -> Vehicle {
    Vehicle::new(name.to_string(), "BA123XY".to_string(), 66.0, 5.1, 0.0)
}

// Helper to seed a test trip — mirrors create_test_vehicle pattern.
// Trip has a wide field set; only the fields needed for FK + identity are
// meaningful here; the rest are defaulted to plausible values.
pub(crate) fn seed_test_trip(db: &Database, vehicle_id: &str) -> String {
    use crate::models::Trip;
    use chrono::NaiveDateTime;
    use uuid::Uuid;
    let trip = Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::parse_str(vehicle_id).unwrap(),
        origin: "BA".into(),
        destination: "TT".into(),
        distance_km: 50.0,
        odometer: 12345.0,
        purpose: "test".into(),
        fuel_liters: None,
        fuel_cost_eur: None,
        other_costs_eur: None,
        other_costs_note: None,
        full_tank: false,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        start_datetime: NaiveDateTime::parse_from_str(
            "2026-01-01T08:00:00",
            "%Y-%m-%dT%H:%M:%S",
        )
        .unwrap(),
        end_datetime: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let id = trip.id.to_string();
    db.create_trip(&trip).expect("seed trip");
    id
}

#[test]
fn test_vehicle_crud_lifecycle() {
    let db = Database::in_memory().expect("Failed to create database");

    // CREATE + RETRIEVE
    let vehicle = create_test_vehicle("Test Car");
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let retrieved = db
        .get_vehicle(&vehicle.id.to_string())
        .expect("Failed to get vehicle")
        .expect("Vehicle not found");
    assert_eq!(retrieved.name, "Test Car");
    assert!(retrieved.is_active);

    // GET ALL + ACTIVE VEHICLE
    let mut v2 = create_test_vehicle("Inactive Car");
    v2.is_active = false;
    db.create_vehicle(&v2).expect("Failed to create v2");

    let all = db.get_all_vehicles().expect("Failed to get all");
    assert_eq!(all.len(), 2);

    let active = db
        .get_active_vehicle()
        .expect("Failed to get active")
        .unwrap();
    assert_eq!(active.id, vehicle.id);

    // UPDATE
    let mut updated = retrieved;
    updated.name = "Updated Name".to_string();
    updated.tp_consumption = Some(6.5);
    db.update_vehicle(&updated).expect("Failed to update");

    let after_update = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(after_update.name, "Updated Name");
    assert_eq!(after_update.tp_consumption, Some(6.5));

    // DELETE
    db.delete_vehicle(&vehicle.id.to_string())
        .expect("Failed to delete");
    assert!(db.get_vehicle(&vehicle.id.to_string()).unwrap().is_none());
}

#[test]
fn test_delete_vehicle_unassigns_receipts_first() {
    let db = Database::in_memory().unwrap();

    let vehicle = create_test_vehicle("Car A");
    db.create_vehicle(&vehicle).unwrap();

    let mut receipt = Receipt::new("path.jpg".to_string(), "receipt.jpg".to_string());
    receipt.vehicle_id = Some(vehicle.id);
    db.create_receipt(&receipt).unwrap();

    db.delete_vehicle(&vehicle.id.to_string()).unwrap();

    assert!(db.get_vehicle(&vehicle.id.to_string()).unwrap().is_none());

    let receipts = db.get_all_receipts().unwrap();
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0].vehicle_id.is_none());
}

#[test]
fn test_update_vehicle_can_change_type() {
    let db = Database::in_memory().unwrap();

    let mut vehicle = create_test_vehicle("Test Car");
    assert_eq!(vehicle.vehicle_type, VehicleType::Ice);
    db.create_vehicle(&vehicle).unwrap();

    vehicle.vehicle_type = VehicleType::Bev;
    vehicle.battery_capacity_kwh = Some(75.0);
    vehicle.baseline_consumption_kwh = Some(18.0);
    vehicle.initial_battery_percent = Some(100.0);
    vehicle.tank_size_liters = None;
    vehicle.tp_consumption = None;
    db.update_vehicle(&vehicle).unwrap();

    let updated = db.get_vehicle(&vehicle.id.to_string()).unwrap().unwrap();
    assert_eq!(updated.vehicle_type, VehicleType::Bev);
    assert_eq!(updated.battery_capacity_kwh, Some(75.0));
}

fn create_test_trip(vehicle_id: Uuid, date: &str) -> Trip {
    let now = Utc::now();
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
    let start_datetime = parsed_date.and_hms_opt(8, 0, 0).unwrap();
    Trip {
        id: Uuid::new_v4(),
        vehicle_id,
        start_datetime,
        end_datetime: None,
        origin: "Prague".to_string(),
        destination: "Brno".to_string(),
        distance_km: 200.0,
        odometer: 50000.0,
        purpose: "Business meeting".to_string(),
        fuel_liters: Some(15.0),
        fuel_cost_eur: Some(25.5),
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: Some(5.0),
        other_costs_note: Some("Parking fee".to_string()),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn test_trip_crud_lifecycle() {
    let db = Database::in_memory().expect("Failed to create database");
    let vehicle = create_test_vehicle("Test Car");
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let trip = create_test_trip(vehicle.id, "2024-12-01");
    db.create_trip(&trip).expect("Failed to create trip");

    let retrieved = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(retrieved.origin, "Prague");
    assert_eq!(retrieved.fuel_liters, Some(15.0));

    let trips = db.get_trips_for_vehicle(&vehicle.id.to_string()).unwrap();
    assert_eq!(trips.len(), 1);

    let mut updated = retrieved;
    updated.origin = "Berlin".to_string();
    db.update_trip(&updated).expect("Failed to update");

    let after_update = db.get_trip(&trip.id.to_string()).unwrap().unwrap();
    assert_eq!(after_update.origin, "Berlin");

    db.delete_trip(&trip.id.to_string())
        .expect("Failed to delete");
    assert!(db.get_trip(&trip.id.to_string()).unwrap().is_none());
}

#[test]
fn test_get_trips_for_vehicle_in_year() {
    let db = Database::in_memory().expect("Failed to create database");
    let vehicle = create_test_vehicle("Test Car");
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let trip1 = create_test_trip(vehicle.id, "2024-12-01");
    let trip2 = create_test_trip(vehicle.id, "2024-06-15");
    let trip3 = create_test_trip(vehicle.id, "2023-12-10");

    db.create_trip(&trip1).unwrap();
    db.create_trip(&trip2).unwrap();
    db.create_trip(&trip3).unwrap();

    let trips_2024 = db
        .get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2024)
        .unwrap();
    assert_eq!(trips_2024.len(), 2);

    let trips_2023 = db
        .get_trips_for_vehicle_in_year(&vehicle.id.to_string(), 2023)
        .unwrap();
    assert_eq!(trips_2023.len(), 1);
}

#[test]
fn test_find_or_create_route_upsert() {
    let db = Database::in_memory().expect("Failed to create database");
    let vehicle = create_test_vehicle("Test Car");
    db.create_vehicle(&vehicle)
        .expect("Failed to create vehicle");

    let route1 = db
        .find_or_create_route(&vehicle.id.to_string(), "Budapest", "Prague", 500.0)
        .expect("Failed to create route");
    assert_eq!(route1.usage_count, 1);

    let route2 = db
        .find_or_create_route(&vehicle.id.to_string(), "Budapest", "Prague", 500.0)
        .expect("Failed to find route");
    assert_eq!(route2.id, route1.id);
    assert_eq!(route2.usage_count, 2);
}

#[test]
fn test_receipt_crud() {
    let db = Database::in_memory().unwrap();

    let receipt = Receipt::new(
        "C:\\test\\receipt.jpg".to_string(),
        "receipt.jpg".to_string(),
    );
    db.create_receipt(&receipt).unwrap();

    let receipts = db.get_all_receipts().unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].file_name, "receipt.jpg");
    assert_eq!(receipts[0].status, ReceiptStatus::Pending);

    let found = db
        .get_receipt_by_file_path("C:\\test\\receipt.jpg")
        .unwrap();
    assert!(found.is_some());

    let mut updated = receipt.clone();
    updated.liters = Some(45.5);
    updated.status = ReceiptStatus::Parsed;
    db.update_receipt(&updated).unwrap();

    let receipts = db.get_all_receipts().unwrap();
    assert_eq!(receipts[0].liters, Some(45.5));
    assert_eq!(receipts[0].status, ReceiptStatus::Parsed);

    db.delete_receipt(&receipt.id.to_string()).unwrap();
    assert_eq!(db.get_all_receipts().unwrap().len(), 0);
}

#[test]
fn test_get_unassigned_receipts() {
    let db = Database::in_memory().unwrap();

    let vehicle = create_test_vehicle("Test Car");
    db.create_vehicle(&vehicle).unwrap();

    let trip = create_test_trip(vehicle.id, "2024-12-01");
    db.create_trip(&trip).unwrap();

    let receipt1 = Receipt::new("path1.jpg".to_string(), "1.jpg".to_string());
    let mut receipt2 = Receipt::new("path2.jpg".to_string(), "2.jpg".to_string());
    receipt2.trip_id = Some(trip.id);
    receipt2.vehicle_id = Some(vehicle.id);

    db.create_receipt(&receipt1).unwrap();
    db.create_receipt(&receipt2).unwrap();

    let unassigned = db.get_unassigned_receipts().unwrap();
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].file_name, "1.jpg");
}

#[test]
fn test_get_pending_receipts() {
    let db = Database::in_memory().unwrap();

    let receipt1 = Receipt::new("path1.jpg".to_string(), "pending.jpg".to_string());
    let mut receipt2 = Receipt::new("path2.jpg".to_string(), "parsed.jpg".to_string());
    receipt2.status = ReceiptStatus::Parsed;

    db.create_receipt(&receipt1).unwrap();
    db.create_receipt(&receipt2).unwrap();

    let pending = db.get_pending_receipts().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].file_name, "pending.jpg");
}

#[test]
fn test_settings_crud() {
    let db = Database::in_memory().unwrap();

    // Initially no settings
    assert!(db.get_settings().unwrap().is_none());

    // Create settings
    let settings = Settings::default();
    db.save_settings(&settings).unwrap();

    let retrieved = db.get_settings().unwrap().unwrap();
    assert_eq!(retrieved.buffer_trip_purpose, "služobná cesta");

    // Update settings
    let mut updated = retrieved;
    updated.company_name = "Test Company".to_string();
    db.save_settings(&updated).unwrap();

    let after_update = db.get_settings().unwrap().unwrap();
    assert_eq!(after_update.company_name, "Test Company");
}

fn create_receipt_for_year_test(
    file_path: &str,
    receipt_datetime: Option<NaiveDateTime>,
    source_year: Option<i32>,
) -> Receipt {
    let mut receipt =
        Receipt::new_with_source_year(file_path.to_string(), file_path.to_string(), source_year);
    receipt.receipt_datetime = receipt_datetime;
    receipt
}

#[test]
fn test_get_receipts_for_year_filters_by_receipt_datetime() {
    let db = Database::in_memory().unwrap();

    let receipt = create_receipt_for_year_test(
        "r1.jpg",
        Some(
            NaiveDateTime::parse_from_str("2024-05-01T10:30:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        ),
        Some(2024),
    );
    db.create_receipt(&receipt).unwrap();

    let receipt2 = create_receipt_for_year_test(
        "r2.jpg",
        Some(
            NaiveDateTime::parse_from_str("2023-12-31T23:59:59", "%Y-%m-%dT%H:%M:%S").unwrap(),
        ),
        Some(2024),
    );
    db.create_receipt(&receipt2).unwrap();

    let receipts_2024 = db.get_receipts_for_year(2024).unwrap();
    assert_eq!(receipts_2024.len(), 1);
    assert_eq!(receipts_2024[0].file_name, "r1.jpg");
}

#[test]
fn test_get_receipts_for_vehicle_returns_unassigned_and_own() {
    let db = Database::in_memory().unwrap();

    let vehicle_a = create_test_vehicle("Car A");
    let vehicle_b = create_test_vehicle("Car B");
    db.create_vehicle(&vehicle_a).unwrap();
    db.create_vehicle(&vehicle_b).unwrap();

    let unassigned = Receipt::new("path1.jpg".to_string(), "receipt1.jpg".to_string());
    let mut receipt_a = Receipt::new("path2.jpg".to_string(), "receipt2.jpg".to_string());
    receipt_a.vehicle_id = Some(vehicle_a.id);
    let mut receipt_b = Receipt::new("path3.jpg".to_string(), "receipt3.jpg".to_string());
    receipt_b.vehicle_id = Some(vehicle_b.id);

    db.create_receipt(&unassigned).unwrap();
    db.create_receipt(&receipt_a).unwrap();
    db.create_receipt(&receipt_b).unwrap();

    let results = db.get_receipts_for_vehicle(&vehicle_a.id, None).unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.id == unassigned.id));
    assert!(results.iter().any(|r| r.id == receipt_a.id));
    assert!(!results.iter().any(|r| r.id == receipt_b.id));
}

#[test]
fn test_get_embedded_migration_versions() {
    let versions = Database::get_embedded_migration_versions();

    // Should have at least the baseline migration
    assert!(!versions.is_empty());
    // Check that we have known migrations (folder names without timestamp)
    assert!(versions.iter().any(|v: &String| v.starts_with("2026")));
}

#[test]
fn test_check_migration_compatibility_passes_for_current_app() {
    let db = Database::in_memory().unwrap();

    // A fresh in-memory DB with current migrations should be compatible
    let result = db.check_migration_compatibility();
    assert!(result.is_ok());
}

// ============================================================================
// Paperless trip link tests
// ============================================================================

#[test]
fn paperless_link_upsert_creates_then_replaces() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let v_id = v.id.to_string();
    let trip_a = seed_test_trip(&db, &v_id);
    let trip_b = seed_test_trip(&db, &v_id);

    db.upsert_paperless_link(&trip_a, 435).unwrap();
    assert_eq!(
        db.get_paperless_link_for_doc(435).unwrap(),
        Some(trip_a.clone())
    );

    db.upsert_paperless_link(&trip_b, 435).unwrap();
    assert_eq!(
        db.get_paperless_link_for_doc(435).unwrap(),
        Some(trip_b.clone())
    );
    assert_eq!(db.get_paperless_link_for_trip(&trip_a).unwrap(), None);
}

#[test]
fn paperless_link_delete_is_idempotent() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip = seed_test_trip(&db, &v.id.to_string());

    db.upsert_paperless_link(&trip, 435).unwrap();
    db.delete_paperless_link_for_doc(435).unwrap();
    db.delete_paperless_link_for_doc(435).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), None);
}

#[test]
fn paperless_link_unique_doc_invariant() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_a = seed_test_trip(&db, &v.id.to_string());
    let trip_b = seed_test_trip(&db, &v.id.to_string());

    db.upsert_paperless_link(&trip_a, 435).unwrap();
    db.upsert_paperless_link(&trip_b, 435).unwrap();
    assert_eq!(db.count_paperless_links().unwrap(), 1);
}

#[test]
fn paperless_link_unique_trip_invariant() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip = seed_test_trip(&db, &v.id.to_string());

    // Link trip to doc 435, then re-link the same trip to doc 999.
    // Doc 435 link must be removed — one trip can hold at most one doc.
    db.upsert_paperless_link(&trip, 435).unwrap();
    db.upsert_paperless_link(&trip, 999).unwrap();

    assert_eq!(db.count_paperless_links().unwrap(), 1);
    assert_eq!(db.get_paperless_link_for_trip(&trip).unwrap(), Some(999));
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), None);
}

#[test]
fn delete_trip_removes_paperless_link() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip = seed_test_trip(&db, &v.id.to_string());

    db.upsert_paperless_link(&trip, 435).unwrap();
    assert_eq!(db.count_paperless_links().unwrap(), 1);

    db.delete_trip(&trip).unwrap();

    assert_eq!(db.count_paperless_links().unwrap(), 0);
}

// ============================================================================
// Source-agnostic invoice attachment query
// ============================================================================

#[test]
fn get_trip_ids_with_invoice_unions_receipts_and_paperless() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_with_receipt = seed_test_trip(&db, &v.id.to_string());
    let trip_with_paperless = seed_test_trip(&db, &v.id.to_string());
    let trip_uncovered = seed_test_trip(&db, &v.id.to_string());

    // Receipt-side coverage: assign a receipt to trip_with_receipt
    let mut receipt = Receipt::new("p.jpg".to_string(), "r.jpg".to_string());
    receipt.trip_id = Some(uuid::Uuid::parse_str(&trip_with_receipt).unwrap());
    receipt.vehicle_id = Some(v.id);
    db.create_receipt(&receipt).unwrap();

    // An unassigned receipt must NOT contribute to the covered set
    let unassigned = Receipt::new("u.jpg".to_string(), "u.jpg".to_string());
    db.create_receipt(&unassigned).unwrap();

    // Paperless-side coverage: link a paperless doc to trip_with_paperless
    db.upsert_paperless_link(&trip_with_paperless, 435).unwrap();

    let covered = db.get_trip_ids_with_invoice().unwrap();

    assert!(covered.contains(&trip_with_receipt));
    assert!(covered.contains(&trip_with_paperless));
    assert!(!covered.contains(&trip_uncovered));
    assert_eq!(covered.len(), 2);
}

// ============================================================================
// Chronological ordering contract (datetime-is-order, Task 65)
// ============================================================================

#[test]
fn test_get_trips_for_vehicle_returns_chronological_order() {
    // get_trips_for_vehicle must return trips ordered by start_datetime DESC
    // (newest first), with insertion order intentionally unrelated to date order.
    let db = Database::in_memory().expect("Failed to create database");
    let vehicle = create_test_vehicle("Test Car");
    db.create_vehicle(&vehicle).expect("Failed to create vehicle");
    let v_id = vehicle.id.to_string();

    let mut trip_old = Trip::test_ice_trip(
        NaiveDate::from_ymd_opt(2026, 5, 5).unwrap(),
        10.0,
        None,
        true,
    );
    trip_old.vehicle_id = vehicle.id;

    let mut trip_new = Trip::test_ice_trip(
        NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
        10.0,
        None,
        true,
    );
    trip_new.vehicle_id = vehicle.id;

    let mut trip_mid = Trip::test_ice_trip(
        NaiveDate::from_ymd_opt(2026, 5, 12).unwrap(),
        10.0,
        None,
        true,
    );
    trip_mid.vehicle_id = vehicle.id;

    db.create_trip(&trip_old).unwrap();
    db.create_trip(&trip_new).unwrap();
    db.create_trip(&trip_mid).unwrap();

    let trips = db.get_trips_for_vehicle(&v_id).unwrap();
    assert_eq!(trips.len(), 3);
    assert_eq!(
        trips[0].start_datetime.date(),
        NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
        "newest first"
    );
    assert_eq!(
        trips[1].start_datetime.date(),
        NaiveDate::from_ymd_opt(2026, 5, 12).unwrap(),
        "middle second"
    );
    assert_eq!(
        trips[2].start_datetime.date(),
        NaiveDate::from_ymd_opt(2026, 5, 5).unwrap(),
        "oldest last"
    );
}
