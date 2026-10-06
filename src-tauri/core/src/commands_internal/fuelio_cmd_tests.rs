use super::*;
use crate::models::{Trip, Vehicle};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use std::io::Write;
use uuid::Uuid;

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
    let matched = &report.rows[0];
    assert_eq!(matched.status, crate::fuelio::RowStatus::Matched);
    assert_eq!(matched.trip_id, Some(t.id.to_string()));
    assert_eq!(matched.origin.as_deref(), Some("A"));
    assert_eq!(matched.start_diff_min, Some(29));
    assert_eq!(report.rows[1].status, crate::fuelio::RowStatus::Missing);
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
    let ids = report.rows[0].drive_ids.clone();
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
