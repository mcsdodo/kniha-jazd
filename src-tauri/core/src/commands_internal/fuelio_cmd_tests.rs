use super::*;
use crate::models::{Trip, Vehicle};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use std::io::Write;
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
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2026).unwrap();
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
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2025).unwrap();
    assert!(report.rows.is_empty());
}

#[test]
fn crosscheck_without_the_folder_reports_it() {
    let (db, _, v, _) = setup();
    let empty = tempfile::tempdir().unwrap();
    let report = get_fuelio_crosscheck_internal(&db, empty.path(), &v.id.to_string(), 2026).unwrap();
    assert!(!report.folder_exists);
    assert_eq!(report.drive_count, 0);
}

#[test]
fn track_returns_the_gps_points_and_the_stored_route() {
    let (db, dir, v, t) = setup();
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2026).unwrap();
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
    let report = get_fuelio_crosscheck_internal(&db, dir.path(), &v.id.to_string(), 2026).unwrap();
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
