use super::*;
use crate::models::{Trip, Vehicle};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use std::io::Write;
use crate::fuelio::MergeRules;
use uuid::Uuid;

/// The drives of the matched row (the report is newest first).
fn matched_ids(report: &FuelioReport) -> Vec<String> {
    report
        .rows
        .iter()
        .find(|r| r.status == crate::fuelio::RowStatus::Matched)
        .unwrap()
        .drive_ids
        .clone()
}

fn place_at(db: &Database, name: &str, lat: f64, lon: f64) -> Uuid {
    let id = db.ensure_place_for_test(name);
    diesel::sql_query("UPDATE places SET lat = ?, lon = ? WHERE id = ?")
        .bind::<diesel::sql_types::Double, _>(lat)
        .bind::<diesel::sql_types::Double, _>(lon)
        .bind::<diesel::sql_types::Text, _>(id.to_string())
        .execute(&mut *db.connection())
        .unwrap();
    id
}

fn write_drive(dir: &std::path::Path, id: &str, rows: &[(i64, f64, f64, f64)]) {
    let csv: String = rows
        .iter()
        .map(|(ts, lat, lon, seg)| format!("{ts},{lat},{lon},{seg},30.0,0,5\n"))
        .collect();
    let file = std::fs::File::create(dir.join(format!("route-{id}.data"))).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(format!("route-{id}.csv"), zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(csv.as_bytes()).unwrap();
    zip.finish().unwrap();
}

/// A data dir with one vehicle, one trip A -> B on 2026-09-28 17:00, and a
/// Fuelio drive A -> B at 17:29 local plus a drive elsewhere on another day.
fn setup() -> (Database, tempfile::TempDir, Vehicle, Trip) {
    let db = Database::in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vehicle::new_ice("Car".into(), "TEST-1".into(), 50.0, 6.5, 0.0);
    v.is_active = true;
    db.create_vehicle(&v).unwrap();
    let a = place_at(&db, "A", 48.0, 20.0);
    let b = place_at(&db, "B", 48.0, 21.0);
    let mut t = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 9, 28).unwrap(), 74.0, None, false);
    t.vehicle_id = v.id;
    t.start_datetime =
        NaiveDateTime::parse_from_str("2026-09-28 17:00", "%Y-%m-%d %H:%M").unwrap();
    t.origin_place_id = a;
    t.destination_place_id = b;
    db.create_trip(&t).unwrap();

    let fuelio = dir.path().join(crate::fuelio::FOLDER_NAME);
    std::fs::create_dir(&fuelio).unwrap();
    // 2026-09-28 15:29 UTC = 17:29 CEST.
    let t0 = NaiveDate::from_ymd_opt(2026, 9, 28)
        .unwrap()
        .and_hms_opt(15, 29, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    write_drive(
        &fuelio,
        &t0.to_string(),
        &[(t0, 48.0, 20.0, 0.0), (t0 + 3_000_000, 48.0, 21.0, 74_000.0)],
    );
    let t1 = t0 + 86_400_000;
    write_drive(
        &fuelio,
        &t1.to_string(),
        &[(t1, 49.0, 19.0, 0.0), (t1 + 600_000, 49.0, 19.2, 15_000.0)],
    );
    (db, dir, v, t)
}

#[test]
fn crosscheck_matches_the_trip_and_lists_the_other_drive_as_missing() {
    let (db, dir, v, t) = setup();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2026, MergeRules::default()).unwrap();
    assert!(report.folder_exists);
    assert_eq!(report.drive_count, 2);
    assert_eq!(report.rows.len(), 2);
    // Newest first: the missing drive of the next day, then the match.
    assert_eq!(report.rows[0].status, crate::fuelio::RowStatus::Missing);
    let matched = &report.rows[1];
    assert_eq!(matched.status, crate::fuelio::RowStatus::Matched);
    assert_eq!(matched.trip_id, Some(t.id.to_string()));
    assert_eq!(matched.origin.as_deref(), Some("A"));
    assert_eq!(matched.start_diff_min, Some(29));
}

#[test]
fn crosscheck_uses_only_the_drives_of_the_year() {
    let (db, dir, v, _) = setup();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2025, MergeRules::default()).unwrap();
    assert!(report.rows.is_empty());
}

// A known limit: each year is checked alone. A drive that starts on 31
// December cannot match a trip that the logbook starts on 1 January.
#[test]
fn a_drive_and_a_trip_on_both_sides_of_new_year_do_not_match() {
    let (db, dir, v, _) = setup();
    let a = place_at(&db, "A", 48.0, 20.0);
    let b = place_at(&db, "B", 48.0, 21.0);
    let mut t = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(), 74.0, None, false);
    t.vehicle_id = v.id;
    t.start_datetime =
        NaiveDateTime::parse_from_str("2027-01-01 00:05", "%Y-%m-%d %H:%M").unwrap();
    t.origin_place_id = a;
    t.destination_place_id = b;
    db.create_trip(&t).unwrap();
    // 2026-12-31 22:50 UTC = 23:50 CET.
    let t0 = NaiveDate::from_ymd_opt(2026, 12, 31)
        .unwrap()
        .and_hms_opt(22, 50, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    let fuelio = dir.path().join(crate::fuelio::FOLDER_NAME);
    write_drive(
        &fuelio,
        &t0.to_string(),
        &[(t0, 48.0, 20.0, 0.0), (t0 + 3_000_000, 48.0, 21.0, 74_000.0)],
    );

    let vid = v.id.to_string();
    let old = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let row = old.rows.iter().find(|r| r.drive_ids == vec![t0.to_string()]).unwrap();
    assert_eq!(row.status, crate::fuelio::RowStatus::Missing);
    let new = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2027, MergeRules::default()).unwrap();
    assert!(new.rows.is_empty());
}

#[test]
fn crosscheck_without_the_folder_reports_it() {
    let (db, _, v, _) = setup();
    let empty = tempfile::tempdir().unwrap();
    let report = get_fuelio_crosscheck_internal(&db, empty.path(), &v.id.to_string(), 2026, MergeRules::default()).unwrap();
    assert!(!report.folder_exists);
    assert_eq!(report.drive_count, 0);
}

#[test]
fn fuelio_is_refused_without_dropbox_even_with_the_folder() {
    let err = require_fuelio_internal(false).unwrap_err();
    assert!(err.contains("Dropbox is not configured"), "got: {err}");
}

#[test]
fn fuelio_is_allowed_with_dropbox() {
    assert!(require_fuelio_internal(true).is_ok());
}

#[test]
fn track_returns_the_gps_points_and_the_stored_route() {
    let (db, dir, v, t) = setup();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2026, MergeRules::default()).unwrap();
    let ids = matched_ids(&report);
    diesel::sql_query(
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, created_at) \
         VALUES (?, '[]', ?, 74, 74, '2026-01-01T00:00:00')",
    )
    .bind::<diesel::sql_types::Text, _>(t.id.to_string())
    .bind::<diesel::sql_types::Text, _>(crate::route_map::polyline::encode(&[(48.0, 20.0), (48.0, 21.0)]))
    .execute(&mut *db.connection())
    .unwrap();

    let track = get_fuelio_track_internal(&db, dir.path(), &ids, Some(&t.id.to_string())).unwrap();
    assert_eq!(track.gps.len(), 1);
    assert_eq!(track.gps[0].first(), Some(&[48.0, 20.0]));
    assert_eq!(track.gps[0].last(), Some(&[48.0, 21.0]));
    assert_eq!(track.route, Some(vec![[48.0, 20.0], [48.0, 21.0]]));
}

#[test]
fn track_rejects_an_id_that_is_not_a_number() {
    let (db, dir, _, _) = setup();
    assert!(get_fuelio_track_internal(&db, dir.path(), &["../x".into()], None).is_err());
}

// ---------------------------------------------------------------------------
// apply_fuelio_to_trip: overwrite trip fields from the GPS (Task 90)
// ---------------------------------------------------------------------------

use crate::app_state::AppState;

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
}

/// The setup trip with a consistent chain: 70 km at odometer 70, and a later
/// trip of 10 km at odometer 80. The GPS drive says 17:29 to 18:19, 74 km.
fn chain() -> (Database, tempfile::TempDir, Trip, Trip, Vec<String>) {
    let (db, dir, v, mut t) = setup();
    t.distance_km = 70.0;
    t.odometer = 70.0;
    db.update_trip(&t).unwrap();
    let mut later = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(), 10.0, None, false);
    later.vehicle_id = v.id;
    later.start_datetime = dt("2026-09-29 08:00");
    later.odometer = 80.0;
    later.origin_place_id = t.destination_place_id;
    later.destination_place_id = t.origin_place_id;
    db.create_trip(&later).unwrap();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2026, MergeRules::default()).unwrap();
    let ids = matched_ids(&report);
    (db, dir, t, later, ids)
}

fn all_fields() -> FuelioFields {
    FuelioFields { start: true, end: true, distance: true, route: true }
}

#[test]
fn apply_dry_run_previews_and_writes_nothing() {
    let (db, dir, t, later, ids) = chain();
    let state = AppState::new();
    let r = apply_fuelio_to_trip_internal(&db, &state, dir.path(), &t.id.to_string(), &ids, all_fields(), true)
        .unwrap();
    assert!(!r.applied);
    assert_eq!(r.start_after, dt("2026-09-28 17:29"));
    assert_eq!(r.end_after, Some(dt("2026-09-28 18:19")));
    assert_eq!(r.distance_after, 74.0);
    let wb = r.writeback.unwrap();
    assert_eq!(wb.plan.changes.len(), 1, "the later trip moves");
    assert_eq!(db.get_trip(&t.id.to_string()).unwrap().unwrap().distance_km, 70.0);
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 80.0);
    assert!(db.get_route_map(&t.id.to_string()).unwrap().is_none());
}

#[test]
fn apply_times_only_leaves_distance_and_odometers() {
    let (db, dir, t, later, ids) = chain();
    let fields = FuelioFields { start: true, end: true, distance: false, route: false };
    let r = apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &ids, fields, false)
        .unwrap();
    assert!(r.applied);
    let saved = db.get_trip(&t.id.to_string()).unwrap().unwrap();
    assert_eq!(saved.start_datetime, dt("2026-09-28 17:29"));
    assert_eq!(saved.end_datetime, Some(dt("2026-09-28 18:19")));
    assert_eq!(saved.distance_km, 70.0);
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 80.0);
    assert!(db.get_route_map(&t.id.to_string()).unwrap().is_none());
}

#[test]
fn apply_distance_moves_the_later_odometers() {
    let (db, dir, t, later, ids) = chain();
    let fields = FuelioFields { start: false, end: false, distance: true, route: false };
    apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &ids, fields, false)
        .unwrap();
    let saved = db.get_trip(&t.id.to_string()).unwrap().unwrap();
    assert_eq!(saved.distance_km, 74.0);
    assert_eq!(saved.odometer, 74.0);
    assert_eq!(saved.start_datetime, dt("2026-09-28 17:00"), "start not selected");
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 84.0);
}

#[test]
fn apply_route_only_saves_the_gps_track_and_keeps_the_distance() {
    let (db, dir, t, _, ids) = chain();
    let fields = FuelioFields { start: false, end: false, distance: false, route: true };
    apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &ids, fields, false)
        .unwrap();
    let map = db.get_route_map(&t.id.to_string()).unwrap().unwrap();
    let line = crate::route_map::polyline::decode(&map.polyline);
    assert_eq!(line.first(), Some(&(48.0, 20.0)));
    assert_eq!(line.last(), Some(&(48.0, 21.0)));
    assert_eq!(map.road_km, 74.0);
    assert_eq!(map.mode, crate::models::RouteMode::Direct);
    assert_eq!(map.provider, None);
    assert_eq!(db.get_trip(&t.id.to_string()).unwrap().unwrap().distance_km, 70.0);
}

#[test]
fn apply_all_fields_writes_them_together() {
    let (db, dir, t, later, ids) = chain();
    apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &ids, all_fields(), false)
        .unwrap();
    let saved = db.get_trip(&t.id.to_string()).unwrap().unwrap();
    assert_eq!((saved.start_datetime, saved.distance_km), (dt("2026-09-28 17:29"), 74.0));
    assert!(db.get_route_map(&t.id.to_string()).unwrap().is_some());
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 84.0);
}

#[test]
fn apply_refuses_a_start_that_moves_the_trip_past_another() {
    let (db, dir, t, _, ids) = chain();
    // A trip at 17:15: the GPS start 17:29 would put our trip after it.
    let mut between = Trip::test_ice_trip(NaiveDate::from_ymd_opt(2026, 9, 28).unwrap(), 1.0, None, false);
    between.vehicle_id = t.vehicle_id;
    between.start_datetime = dt("2026-09-28 17:15");
    between.odometer = 71.0;
    between.origin_place_id = t.destination_place_id;
    between.destination_place_id = t.destination_place_id;
    db.create_trip(&between).unwrap();
    let err = apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &ids, all_fields(), true)
        .unwrap_err();
    assert!(err.contains("order"), "{err}");
}

#[test]
fn apply_is_blocked_in_read_only_mode_but_a_dry_run_is_not() {
    let (db, dir, t, _, ids) = chain();
    let state = AppState::new();
    state.enable_read_only("test");
    assert!(apply_fuelio_to_trip_internal(&db, &state, dir.path(), &t.id.to_string(), &ids, all_fields(), true).is_ok());
    assert!(apply_fuelio_to_trip_internal(&db, &state, dir.path(), &t.id.to_string(), &ids, all_fields(), false).is_err());
}

#[test]
fn apply_needs_a_field_and_a_drive() {
    let (db, dir, t, _, ids) = chain();
    let none = FuelioFields { start: false, end: false, distance: false, route: false };
    assert!(apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &ids, none, true).is_err());
    assert!(apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &[], all_fields(), true).is_err());
}

#[tokio::test]
async fn sync_without_dropbox_secrets_says_what_to_set() {
    let dir = tempfile::tempdir().unwrap();
    let err = sync_fuelio_dropbox_internal(None, dir.path(), 2026).await.unwrap_err();
    assert!(err.contains("DROPBOX_REFRESH_TOKEN"), "{err}");
}

// ---------------------------------------------------------------------------
// add_fuelio_trip: a new trip from a "missing" drive (Task 90)
// ---------------------------------------------------------------------------

/// A drive B -> A on 2026-09-28 at 21:00 local (19:00 UTC), 74 km: between
/// the setup trip (28th 17:00) and the later trip of `chain` (29th 08:00).
fn evening_drive(dir: &std::path::Path) -> String {
    let t0 = NaiveDate::from_ymd_opt(2026, 9, 28)
        .unwrap()
        .and_hms_opt(19, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    write_drive(
        &dir.join(crate::fuelio::FOLDER_NAME),
        &t0.to_string(),
        &[(t0, 48.0, 21.0, 0.0), (t0 + 3_090_000, 48.0, 20.0, 74_400.0)],
    );
    t0.to_string()
}

#[test]
fn add_preview_offers_every_placed_place_nearest_first() {
    let (db, dir, _, _, _) = chain();
    let id = evening_drive(dir.path());
    let p = get_fuelio_add_preview_internal(&db, dir.path(), &[id]).unwrap();
    assert_eq!(p.start, dt("2026-09-28 21:00"));
    assert_eq!(p.end, dt("2026-09-28 21:51"));
    assert_eq!(p.distance_km, 74.0, "whole km of 74.4");
    let names = |opts: &[PlaceOption]| opts.iter().map(|o| o.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&p.origin), vec!["B", "A"]);
    assert_eq!(names(&p.destination), vec!["A", "B"]);
    assert!(p.origin[0].distance_m < 1.0);
}

#[test]
fn add_dry_run_plans_the_insert_and_writes_nothing() {
    let (db, dir, t, later, _) = chain();
    let id = evening_drive(dir.path());
    let r = add_fuelio_trip_internal(
        &db, &AppState::new(), dir.path(), &t.vehicle_id.to_string(), &[id],
        &t.destination_place_id.to_string(), &t.origin_place_id.to_string(), "Back", true, true,
    )
    .unwrap();
    assert!(r.trip.is_none());
    assert_eq!(r.plan.changes.len(), 1, "the later trip moves");
    let trips = db.get_trips_for_vehicle_in_year(&t.vehicle_id.to_string(), 2026).unwrap();
    assert_eq!(trips.len(), 2);
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 80.0);
}

#[test]
fn add_creates_the_trip_with_its_route_and_moves_the_later_odometers() {
    let (db, dir, t, later, _) = chain();
    let id = evening_drive(dir.path());
    let r = add_fuelio_trip_internal(
        &db, &AppState::new(), dir.path(), &t.vehicle_id.to_string(), &[id],
        &t.destination_place_id.to_string(), &t.origin_place_id.to_string(), "Back", true, false,
    )
    .unwrap();
    let new = r.trip.unwrap();
    let saved = db.get_trip(&new.id.to_string()).unwrap().unwrap();
    assert_eq!((saved.start_datetime, saved.end_datetime), (dt("2026-09-28 21:00"), Some(dt("2026-09-28 21:51"))));
    assert_eq!((saved.distance_km, saved.odometer), (74.0, 144.0));
    assert_eq!((saved.origin.as_str(), saved.destination.as_str(), saved.purpose.as_str()), ("B", "A", "Back"));
    assert!(r.route_written);
    let map = db.get_route_map(&new.id.to_string()).unwrap().unwrap();
    assert_eq!(map.road_km, 74.4);
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 154.0);
}

// The trip exists once the insert is done: a failed route save must not look
// like a failed add, or a second click adds the trip again.
#[test]
fn add_with_a_failed_route_save_returns_the_trip_and_the_route_error() {
    let (db, dir, t, _, _) = chain();
    let id = evening_drive(dir.path());
    diesel::sql_query(
        "CREATE TRIGGER no_routes BEFORE INSERT ON trip_routes BEGIN SELECT RAISE(ABORT, 'disk full'); END",
    )
    .execute(&mut *db.connection())
    .unwrap();
    let r = add_fuelio_trip_internal(
        &db, &AppState::new(), dir.path(), &t.vehicle_id.to_string(), &[id],
        &t.destination_place_id.to_string(), &t.origin_place_id.to_string(), "Back", true, false,
    )
    .unwrap();
    assert!(r.trip.is_some());
    assert!(!r.route_written);
    assert!(r.route_error.as_deref().is_some_and(|e| e.contains("disk full")), "{:?}", r.route_error);
    let trips = db.get_trips_for_vehicle_in_year(&t.vehicle_id.to_string(), 2026).unwrap();
    assert_eq!(trips.len(), 3);
}

#[test]
fn add_without_the_route_saves_no_route() {
    let (db, dir, t, _, _) = chain();
    let id = evening_drive(dir.path());
    let r = add_fuelio_trip_internal(
        &db, &AppState::new(), dir.path(), &t.vehicle_id.to_string(), &[id],
        &t.destination_place_id.to_string(), &t.origin_place_id.to_string(), "Back", false, false,
    )
    .unwrap();
    assert!(!r.route_written);
    assert!(r.route_error.is_none());
    assert!(db.get_route_map(&r.trip.unwrap().id.to_string()).unwrap().is_none());
}

#[test]
fn add_is_blocked_in_read_only_mode() {
    let (db, dir, t, _, _) = chain();
    let id = evening_drive(dir.path());
    let state = AppState::new();
    state.enable_read_only("test");
    let args = |dry| {
        add_fuelio_trip_internal(
            &db, &state, dir.path(), &t.vehicle_id.to_string(), &[id.clone()],
            &t.destination_place_id.to_string(), &t.origin_place_id.to_string(), "Back", true, dry,
        )
    };
    assert!(args(true).is_ok());
    assert!(args(false).is_err());
}

// The stored route is every GPS fix, not the 100 m thinned track: thinning
// cut corners by up to 93 m (2026-10-07).

/// A drive of 20 fixes about 11 m apart, on 2026-09-28 at 21:00 local.
fn dense_drive(dir: &std::path::Path) -> String {
    let t0 = NaiveDate::from_ymd_opt(2026, 9, 28)
        .unwrap()
        .and_hms_opt(19, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    let rows: Vec<(i64, f64, f64, f64)> = (0..20)
        .map(|i| (t0 + i * 1000, 48.0 + i as f64 * 0.0001, 21.0, if i == 0 { 0.0 } else { 11.1 }))
        .collect();
    write_drive(&dir.join(crate::fuelio::FOLDER_NAME), &t0.to_string(), &rows);
    t0.to_string()
}

#[test]
fn overwrite_stores_every_gps_fix_as_the_route() {
    let (db, dir, t, _, _) = chain();
    let id = dense_drive(dir.path());
    let fields = FuelioFields { start: false, end: false, distance: false, route: true };
    apply_fuelio_to_trip_internal(&db, &AppState::new(), dir.path(), &t.id.to_string(), &[id], fields, false)
        .unwrap();
    let map = db.get_route_map(&t.id.to_string()).unwrap().unwrap();
    assert_eq!(crate::route_map::polyline::decode(&map.polyline).len(), 20);
}

#[test]
fn add_stores_every_gps_fix_as_the_route() {
    let (db, dir, t, _, _) = chain();
    let id = dense_drive(dir.path());
    let r = add_fuelio_trip_internal(
        &db, &AppState::new(), dir.path(), &t.vehicle_id.to_string(), &[id],
        &t.destination_place_id.to_string(), &t.destination_place_id.to_string(), "x", true, false,
    )
    .unwrap();
    let map = db.get_route_map(&r.trip.unwrap().id.to_string()).unwrap().unwrap();
    assert_eq!(crate::route_map::polyline::decode(&map.polyline).len(), 20);
}

// ---------------------------------------------------------------------------
// Ignore a missing drive
// ---------------------------------------------------------------------------

fn missing_ids(report: &FuelioReport) -> Vec<String> {
    report
        .rows
        .iter()
        .find(|r| r.status == crate::fuelio::RowStatus::Missing)
        .unwrap()
        .drive_ids
        .clone()
}

#[test]
fn an_ignored_missing_drive_is_marked_ignored() {
    let (db, dir, v, _) = setup();
    let vid = v.id.to_string();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    assert!(report.rows.iter().all(|r| !r.ignored), "nothing is ignored at first");
    let ids = missing_ids(&report);

    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &ids, true).unwrap();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let missing = report.rows.iter().find(|r| r.drive_ids == ids).unwrap();
    assert!(missing.ignored);
    let matched = report.rows.iter().find(|r| r.status == crate::fuelio::RowStatus::Matched).unwrap();
    assert!(!matched.ignored);

    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &ids, false).unwrap();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    assert!(report.rows.iter().all(|r| !r.ignored), "un-ignore brings the row back");
}

#[test]
fn an_ignored_drive_of_a_matched_trip_does_not_hide_the_match() {
    let (db, dir, v, _) = setup();
    let vid = v.id.to_string();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let ids = matched_ids(&report);
    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &ids, true).unwrap();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let matched = report.rows.iter().find(|r| r.drive_ids == ids).unwrap();
    assert_eq!(matched.status, crate::fuelio::RowStatus::Matched);
    assert!(!matched.ignored, "only a missing row can be ignored");
}

#[test]
fn a_missing_chain_with_a_new_drive_is_not_ignored() {
    let (db, dir, v, _) = setup();
    let vid = v.id.to_string();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let ids = missing_ids(&report);
    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &ids, true).unwrap();

    // A later sync brings a drive that continues the ignored one (5 min later,
    // from its end point), so both join one missing chain.
    let first: i64 = ids[0].parse().unwrap();
    let t2 = first + 600_000 + 300_000;
    write_drive(
        &dir.path().join(crate::fuelio::FOLDER_NAME),
        &t2.to_string(),
        &[(t2, 49.0, 19.2, 0.0), (t2 + 600_000, 49.0, 19.4, 15_000.0)],
    );
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let chain = report.rows.iter().find(|r| r.drive_ids.contains(&ids[0])).unwrap();
    assert_eq!(chain.drive_ids.len(), 2, "the new drive joins the chain");
    assert!(!chain.ignored, "a row is ignored only when all its drives are");
}

#[test]
fn ignoring_is_per_vehicle() {
    let (db, dir, v, _) = setup();
    let vid = v.id.to_string();
    let other = Vehicle::new_ice("Other".into(), "TEST-2".into(), 50.0, 6.5, 0.0);
    db.create_vehicle(&other).unwrap();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let ids = missing_ids(&report);
    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &other.id.to_string(), &ids, true).unwrap();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    assert!(report.rows.iter().all(|r| !r.ignored));
}

#[test]
fn ignoring_twice_and_un_ignoring_an_unknown_drive_are_harmless() {
    let (db, dir, v, _) = setup();
    let vid = v.id.to_string();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    let ids = missing_ids(&report);
    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &ids, true).unwrap();
    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &ids, true).unwrap();
    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &["1".to_string()], false).unwrap();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, MergeRules::default()).unwrap();
    assert!(report.rows.iter().find(|r| r.drive_ids == ids).unwrap().ignored);
}

#[test]
fn ignore_is_blocked_in_read_only_mode() {
    let (db, _, v, _) = setup();
    let state = AppState::new();
    state.enable_read_only("test");
    let err = set_fuelio_drives_ignored_internal(&db, &state, &v.id.to_string(), &["1".to_string()], true)
        .unwrap_err();
    assert!(err.contains("len na čítanie"), "got: {err}");
}

#[test]
fn ignore_rejects_a_bad_drive_id() {
    let (db, _, v, _) = setup();
    let err = set_fuelio_drives_ignored_internal(&db, &AppState::new(), &v.id.to_string(), &["../x".to_string()], true)
        .unwrap_err();
    assert!(!err.is_empty());
    let none: Vec<String> = vec![];
    assert!(set_fuelio_drives_ignored_internal(&db, &AppState::new(), &v.id.to_string(), &none, true).is_err());
}

#[test]
fn deleting_the_vehicle_removes_its_ignored_drives() {
    let (db, _, _, _) = setup();
    // A vehicle with trips cannot be deleted (FK), so use one without
    let other = Vehicle::new_ice("Other".into(), "TEST-2".into(), 50.0, 6.5, 0.0);
    db.create_vehicle(&other).unwrap();
    let vid = other.id.to_string();
    let ids = vec!["1790562640107".to_string()];
    set_fuelio_drives_ignored_internal(&db, &AppState::new(), &vid, &ids, true).unwrap();
    db.delete_vehicle(&vid).unwrap();
    assert!(db.ignored_fuelio_drives(&vid).unwrap().is_empty());
}

#[test]
fn crosscheck_refuses_negative_merge_rules() {
    let (db, dir, v, _) = setup();
    let vid = v.id.to_string();
    let gap = MergeRules { max_gap_min: -1, ..MergeRules::default() };
    assert!(get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, gap).is_err());
    let jump = MergeRules { max_jump_m: -1.0, ..MergeRules::default() };
    assert!(get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, jump).is_err());
    let nan = MergeRules { max_jump_m: f64::NAN, ..MergeRules::default() };
    assert!(get_fuelio_crosscheck_internal(&db, dir.path(), &vid, 2026, nan).is_err());
}
