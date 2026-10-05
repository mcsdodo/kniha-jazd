use super::*;
use crate::models::{Trip, Vehicle};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn setup() -> (Arc<Database>, LogbookReader) {
    let db = Arc::new(Database::in_memory().unwrap());
    let reader = LogbookReader::new(db.clone());
    (db, reader)
}

fn vehicle(db: &Database, name: &str, active: bool) -> Vehicle {
    let mut v = Vehicle::new_ice(name.into(), "TEST-1".into(), 50.0, 6.5, 0.0);
    v.is_active = active;
    db.create_vehicle(&v).unwrap();
    v
}

fn trip(db: &Database, v: &Vehicle, at: &str, odo: f64, from: Uuid, to: Uuid, km: f64) -> Trip {
    let mut t = Trip::test_ice_trip(d("2026-01-01"), km, None, false);
    t.vehicle_id = v.id;
    t.start_datetime = NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M").unwrap();
    t.odometer = odo;
    t.origin_place_id = from;
    t.destination_place_id = to;
    t.purpose = "Project".into();
    db.create_trip(&t).unwrap();
    t
}

fn mark_round_trip(db: &Database, trip_id: &str) {
    diesel::sql_query(
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, created_at, round_trip) \
         VALUES (?, '[]', '', 0, 0, '2026-01-01T00:00:00', 1)",
    )
    .bind::<diesel::sql_types::Text, _>(trip_id)
    .execute(&mut *db.connection())
    .unwrap();
}

#[test]
fn trips_in_range_are_inclusive_and_ascending() {
    let (db, _) = setup();
    let v = vehicle(&db, "Car", true);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    trip(&db, &v, "2026-01-31 23:59", 100.0, home, a, 10.0);
    trip(&db, &v, "2026-02-01 00:00", 110.0, a, home, 10.0);
    trip(&db, &v, "2026-02-28 12:00", 120.0, home, a, 10.0);
    trip(&db, &v, "2026-03-01 00:00", 130.0, a, home, 10.0);

    let trips = db
        .get_trips_for_vehicle_in_range(&v.id.to_string(), d("2026-02-01"), d("2026-02-28"))
        .unwrap();

    let odos: Vec<f64> = trips.iter().map(|t| t.odometer).collect();
    assert_eq!(odos, vec![110.0, 120.0]);
    assert_eq!(trips[0].origin, "City A", "the range read fills the place names");
}

#[test]
fn trips_without_vehicle_cover_all_vehicles_sorted_by_time() {
    let (db, reader) = setup();
    let selected = vehicle(&db, "Selected", true);
    let other = vehicle(&db, "Other", false);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    trip(&db, &selected, "2026-02-02 08:00", 100.0, home, a, 10.0);
    trip(&db, &other, "2026-02-03 08:00", 500.0, home, a, 10.0);
    trip(&db, &selected, "2026-02-04 08:00", 110.0, a, home, 10.0);

    let trips = reader.trips_in_range(d("2026-02-01"), d("2026-02-28"), None).unwrap();

    let owners: Vec<Uuid> = trips.iter().map(|t| t.vehicle_id).collect();
    assert_eq!(owners, vec![selected.id, other.id, selected.id]);
}

#[test]
fn unknown_vehicle_is_an_error() {
    let (_, reader) = setup();
    let id = Uuid::new_v4().to_string();
    let err = reader.trips_in_range(d("2026-02-01"), d("2026-02-28"), Some(&id));
    assert_eq!(err.unwrap_err(), ReadError::Invalid("Vehicle not found".into()));
}

#[test]
fn date_from_after_date_to_is_an_error() {
    let (_, reader) = setup();
    let err = reader.trips_in_range(d("2026-03-01"), d("2026-02-01"), None);
    assert_eq!(err.unwrap_err(), ReadError::Invalid("date_from is after date_to".into()));
}

#[test]
fn journeys_without_home_is_an_error() {
    let (db, reader) = setup();
    vehicle(&db, "Car", true);
    let err = reader.journeys(d("2026-01-01"), d("2026-12-31"), None);
    assert_eq!(err.unwrap_err(), ReadError::Invalid(HOME_NOT_SET.into()));
}

#[test]
fn journey_that_starts_before_range_is_found() {
    let (db, reader) = setup();
    let v = vehicle(&db, "Car", true);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    db.set_home_place(Some(&home.to_string())).unwrap();
    trip(&db, &v, "2026-01-27 07:00", 100.0, home, a, 357.0);
    trip(&db, &v, "2026-02-10 16:00", 457.0, a, home, 357.0);

    let list = reader.journeys(d("2026-02-01"), d("2026-02-28"), None).unwrap();

    assert_eq!(list.home_place, "Home St 1, Hometown");
    assert_eq!(list.journeys.len(), 1);
    assert_eq!(list.journeys[0].nights, Some(14));
}

#[test]
fn round_trip_map_from_db_makes_a_day_trip() {
    let (db, reader) = setup();
    let v = vehicle(&db, "Car", true);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let village = db.ensure_place_for_test("Village");
    db.set_home_place(Some(&home.to_string())).unwrap();
    let t = trip(&db, &v, "2026-05-04 08:00", 100.0, home, village, 52.0);
    mark_round_trip(&db, &t.id.to_string());

    let list = reader.journeys(d("2026-05-01"), d("2026-05-31"), None).unwrap();

    assert!(list.journeys.is_empty());
}

#[test]
fn journeys_of_one_vehicle_only_when_vehicle_given() {
    let (db, reader) = setup();
    let car = vehicle(&db, "Car", true);
    let van = vehicle(&db, "Van", false);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    db.set_home_place(Some(&home.to_string())).unwrap();
    for v in [&car, &van] {
        trip(&db, v, "2026-06-01 07:00", 100.0, home, a, 50.0);
        trip(&db, v, "2026-06-02 07:00", 150.0, a, home, 50.0);
    }

    let all = reader.journeys(d("2026-06-01"), d("2026-06-30"), None).unwrap();
    assert_eq!(all.journeys.len(), 2, "None means all vehicles, also not selected ones");

    let id = van.id.to_string();
    let one = reader.journeys(d("2026-06-01"), d("2026-06-30"), Some(&id)).unwrap();
    assert_eq!(one.journeys.len(), 1);
    assert_eq!(one.journeys[0].vehicle_id, van.id);
}

#[test]
fn journeys_with_the_same_start_sort_by_vehicle_id() {
    let (db, reader) = setup();
    // Name order is the opposite of id order, so only an id sort passes.
    let mut ids = [Uuid::new_v4(), Uuid::new_v4()];
    ids.sort();
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    db.set_home_place(Some(&home.to_string())).unwrap();
    for (id, name) in [(ids[0], "Vehicle B"), (ids[1], "Vehicle A")] {
        let mut v = Vehicle::new_ice(name.into(), "TEST-1".into(), 50.0, 6.5, 0.0);
        v.id = id;
        db.create_vehicle(&v).unwrap();
        trip(&db, &v, "2026-06-01 07:00", 100.0, home, a, 50.0);
        trip(&db, &v, "2026-06-02 07:00", 150.0, a, home, 50.0);
    }

    let list = reader.journeys(d("2026-06-01"), d("2026-06-30"), None).unwrap();

    let got: Vec<Uuid> = list.journeys.iter().map(|j| j.vehicle_id).collect();
    assert_eq!(got, ids.to_vec());
}
