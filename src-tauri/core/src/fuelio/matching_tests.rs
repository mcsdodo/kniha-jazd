use super::*;
use chrono::NaiveDate;

// Three places on one east-west line, about 74 km apart.
const A: (f64, f64) = (48.0, 20.0);
const B: (f64, f64) = (48.0, 21.0);
const C: (f64, f64) = (48.0, 22.0);

fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, d)
        .unwrap()
        .and_hms_opt(h, m, 0)
        .unwrap()
}

fn mid(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

fn drive(id: &str, start: NaiveDateTime, mins: i64, from: (f64, f64), to: (f64, f64), km: f64) -> Drive {
    Drive {
        id: id.into(),
        start,
        end: start + chrono::Duration::minutes(mins),
        km,
        fast_minutes: 0.0,
        max_kmh: 0.0,
        start_point: from,
        end_point: to,
        track: vec![from, mid(from, to), to],
    }
}

fn trip(id: &str, start: NaiveDateTime, from: (f64, f64), to: (f64, f64), km: f64) -> TripRef {
    TripRef {
        id: id.into(),
        start,
        end: None,
        round_trip: false,
        origin: format!("{from:?}"),
        destination: format!("{to:?}"),
        origin_point: Some(from),
        destination_point: Some(to),
        km,
        route: None,
    }
}

fn find<'a>(rows: &'a [CrosscheckRow], trip_id: &str) -> &'a CrosscheckRow {
    rows.iter()
        .find(|r| r.trip_id.as_deref() == Some(trip_id))
        .unwrap()
}

#[test]
fn one_drive_matches_its_trip() {
    let trips = [trip("t1", at(28, 17, 0), A, B, 74.0)];
    let drives = [drive("d1", at(28, 17, 10), 50, A, B, 75.0)];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.status, RowStatus::Matched);
    assert_eq!(r.drive_ids, vec!["d1".to_string()]);
    assert_eq!(r.start_diff_min, Some(10));
    assert!(r.flags.is_empty(), "{:?}", r.flags);
}

#[test]
fn a_trip_split_by_stops_matches_all_its_drives() {
    // 2026-09-28: SNV -> BA was three Fuelio drives.
    let trips = [trip("t1", at(28, 4, 25), A, C, 148.0)];
    let drives = [
        drive("d1", at(28, 4, 30), 50, A, (48.0, 20.5), 37.0),
        drive("d2", at(28, 5, 35), 60, (48.0, 20.5), B, 37.0),
        drive("d3", at(28, 6, 51), 60, B, C, 74.0),
    ];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].drive_ids, vec!["d1", "d2", "d3"]);
    assert!((rows[0].gps_km.unwrap() - 148.0).abs() < 1e-9);
    assert_eq!(rows[0].gps_start, Some(at(28, 4, 30)));
    assert_eq!(rows[0].gps_end, Some(at(28, 7, 51)));
}

#[test]
fn a_short_stop_between_two_trips_does_not_join_them() {
    // BA -> OMV, then OMV -> SNV 7 minutes later: two trips, two matches.
    let trips = [
        trip("t1", at(28, 17, 0), A, B, 74.0),
        trip("t2", at(28, 18, 55), B, C, 74.0),
    ];
    let drives = [
        drive("d1", at(28, 17, 29), 70, A, B, 74.0),
        drive("d2", at(28, 18, 46), 70, B, C, 74.0),
    ];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(find(&rows, "t1").drive_ids, vec!["d1"]);
    assert_eq!(find(&rows, "t2").drive_ids, vec!["d2"]);
}

#[test]
fn a_start_time_far_from_the_logbook_is_flagged() {
    let trips = [trip("t1", at(28, 17, 0), A, B, 74.0)];
    let drives = [drive("d1", at(28, 17, 45), 50, A, B, 74.0)];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(rows[0].start_diff_min, Some(45));
    assert_eq!(rows[0].flags, vec![Flag::TimeDiffers]);
}

#[test]
fn a_km_difference_above_ten_percent_is_flagged() {
    let trips = [trip("t1", at(28, 17, 0), A, B, 74.0)];
    let drives = [drive("d1", at(28, 17, 0), 50, A, B, 90.0)];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(rows[0].flags, vec![Flag::KmDiffers]);
    assert!((rows[0].km_diff_pct.unwrap() - (90.0 - 74.0) / 74.0 * 100.0).abs() < 1e-9);
}

#[test]
fn a_drive_that_starts_elsewhere_does_not_match() {
    let trips = [trip("t1", at(28, 17, 0), A, B, 74.0)];
    let drives = [drive("d1", at(28, 17, 0), 50, C, B, 74.0)];
    let rows = crosscheck(&trips, &drives);
    let statuses: Vec<RowStatus> = rows.iter().map(|r| r.status).collect();
    assert_eq!(statuses, vec![RowStatus::NoDrive, RowStatus::Missing]);
}

#[test]
fn a_drive_more_than_12_hours_away_does_not_match() {
    let trips = [trip("t1", at(28, 6, 0), A, B, 74.0)];
    let drives = [drive("d1", at(28, 19, 0), 50, A, B, 74.0)];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(find(&rows, "t1").status, RowStatus::NoDrive);
}

#[test]
fn a_drive_with_half_the_km_does_not_match() {
    let trips = [trip("t1", at(28, 6, 0), A, A, 200.0)];
    let drives = [drive("d1", at(28, 6, 0), 20, A, A, 5.0)];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(find(&rows, "t1").status, RowStatus::NoDrive);
}

#[test]
fn of_two_fitting_drives_the_nearest_in_time_wins() {
    let trips = [trip("t1", at(28, 17, 0), A, B, 74.0)];
    let drives = [
        drive("early", at(28, 8, 0), 50, A, B, 74.0),
        drive("near", at(28, 17, 20), 50, A, B, 74.0),
    ];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(find(&rows, "t1").drive_ids, vec!["near"]);
    assert!(rows.iter().any(|r| r.status == RowStatus::Missing && r.drive_ids == vec!["early"]));
}

#[test]
fn a_drive_is_used_by_one_trip_only() {
    let trips = [
        trip("t1", at(28, 17, 0), A, B, 74.0),
        trip("t2", at(28, 17, 5), A, B, 74.0),
    ];
    let drives = [drive("d1", at(28, 17, 0), 50, A, B, 74.0)];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(find(&rows, "t1").status, RowStatus::Matched);
    assert_eq!(find(&rows, "t2").status, RowStatus::NoDrive);
}

#[test]
fn unused_drives_join_into_one_missing_row_over_a_short_stop() {
    let drives = [
        drive("d1", at(28, 10, 0), 50, A, B, 74.0),
        drive("d2", at(28, 11, 20), 50, B, C, 74.0),
        // 3 hours later: a new row.
        drive("d3", at(28, 15, 0), 50, C, A, 148.0),
    ];
    let rows = crosscheck(&[], &drives);
    let ids: Vec<Vec<String>> = rows.iter().map(|r| r.drive_ids.clone()).collect();
    assert_eq!(ids, vec![vec!["d1".to_string(), "d2".into()], vec!["d3".into()]]);
    assert!(rows.iter().all(|r| r.status == RowStatus::Missing));
    assert!((rows[0].gps_km.unwrap() - 148.0).abs() < 1e-9);
}

#[test]
fn missing_drives_do_not_join_when_the_car_moved_between_them() {
    let drives = [
        drive("d1", at(28, 10, 0), 50, A, B, 74.0),
        drive("d2", at(28, 11, 0), 50, C, A, 148.0),
    ];
    assert_eq!(crosscheck(&[], &drives).len(), 2);
}

#[test]
fn highway_is_long_or_fast() {
    let mut fast = drive("fast", at(28, 10, 0), 10, A, A, 12.0);
    fast.fast_minutes = 6.0;
    let rows = crosscheck(
        &[],
        &[
            drive("long", at(27, 10, 0), 30, A, B, 74.0),
            fast,
            drive("short", at(29, 10, 0), 10, A, A, 12.0),
        ],
    );
    let hw: Vec<bool> = rows.iter().map(|r| r.is_highway).collect();
    assert_eq!(hw, vec![true, true, false]);
}

#[test]
fn a_trip_without_a_drive_is_highway_by_its_logbook_km() {
    let rows = crosscheck(
        &[
            trip("long", at(28, 10, 0), A, B, 74.0),
            trip("short", at(28, 12, 0), A, A, 5.0),
        ],
        &[],
    );
    assert!(find(&rows, "long").is_highway);
    assert!(!find(&rows, "short").is_highway);
}

#[test]
fn a_track_off_the_stored_route_is_flagged_as_a_different_route() {
    let mut t = trip("t1", at(28, 17, 0), A, B, 74.0);
    t.route = Some(vec![A, B]);
    let mut d = drive("d1", at(28, 17, 0), 50, A, B, 74.0);
    // Half of the track runs 0.2 deg (about 22 km) south of the stored route.
    d.track = vec![A, (47.8, 20.3), (47.8, 20.6), B];
    let rows = crosscheck(&[t], &[d]);
    assert_eq!(rows[0].flags, vec![Flag::DifferentRoute]);
    assert!((rows[0].off_route_pct.unwrap() - 50.0).abs() < 1e-9);
}

#[test]
fn a_track_on_the_stored_route_is_not_flagged() {
    let mut t = trip("t1", at(28, 17, 0), A, B, 74.0);
    t.route = Some(vec![A, B]);
    let rows = crosscheck(&[t], &[drive("d1", at(28, 17, 0), 50, A, B, 74.0)]);
    assert!(rows[0].flags.is_empty());
    assert_eq!(rows[0].off_route_pct, Some(0.0));
}

#[test]
fn a_trip_without_a_stored_route_has_no_route_check() {
    let rows = crosscheck(
        &[trip("t1", at(28, 17, 0), A, B, 74.0)],
        &[drive("d1", at(28, 17, 0), 50, A, B, 74.0)],
    );
    assert_eq!(rows[0].off_route_pct, None);
}

#[test]
fn a_place_without_coordinates_matches_on_time_and_km() {
    let mut t = trip("t1", at(28, 17, 0), A, B, 74.0);
    t.origin_point = None;
    let rows = crosscheck(&[t], &[drive("d1", at(28, 17, 0), 50, C, B, 74.0)]);
    assert_eq!(rows[0].status, RowStatus::Matched);
}

#[test]
fn rows_come_out_in_time_order() {
    let rows = crosscheck(
        &[trip("t1", at(28, 12, 0), A, B, 74.0)],
        &[
            drive("late", at(28, 20, 0), 50, C, B, 74.0),
            drive("early", at(27, 9, 0), 50, C, B, 74.0),
        ],
    );
    let ids: Vec<String> = rows
        .iter()
        .map(|r| r.trip_id.clone().unwrap_or_else(|| r.drive_ids[0].clone()))
        .collect();
    assert_eq!(ids, vec!["early", "t1", "late"]);
}

fn round_trip(id: &str, start: NaiveDateTime, end: Option<NaiveDateTime>, from: (f64, f64), to: (f64, f64), km: f64) -> TripRef {
    TripRef { end, round_trip: true, ..trip(id, start, from, to, km) }
}

// 2026-08-27: SNV -> Ganovce -> SNV is one logbook trip (15:00 to 19:16), and
// Fuelio has the way out and the way back with a 3-hour stop in Poprad,
// 3.5 km from the Ganovce place.
#[test]
fn a_round_trip_matches_the_way_out_and_back_over_a_long_stop() {
    let turnaround = (48.03, 21.0); // about 3.3 km from B
    let trips = [round_trip("t1", at(27, 15, 0), Some(at(27, 19, 16)), A, B, 88.0)];
    let drives = [
        drive("out", at(27, 14, 57), 44, A, turnaround, 45.0),
        drive("back", at(27, 18, 36), 30, turnaround, A, 34.0),
    ];
    let rows = crosscheck(&trips, &drives);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, RowStatus::Matched);
    assert_eq!(rows[0].drive_ids, vec!["out", "back"]);
    assert_eq!(rows[0].flags, vec![Flag::KmDiffers]);
}

#[test]
fn a_round_trip_without_an_end_time_allows_a_stop_up_to_12_hours() {
    let trips = [round_trip("t1", at(27, 8, 0), None, A, B, 148.0)];
    let drives = [
        drive("out", at(27, 8, 0), 50, A, B, 74.0),
        drive("back", at(27, 19, 0), 50, B, A, 74.0),
    ];
    assert_eq!(crosscheck(&trips, &drives)[0].drive_ids, vec!["out", "back"]);
}

#[test]
fn a_round_trip_stop_longer_than_the_trip_does_not_join() {
    let trips = [round_trip("t1", at(27, 8, 0), Some(at(27, 12, 0)), A, B, 148.0)];
    let drives = [
        drive("out", at(27, 8, 0), 50, A, B, 74.0),
        drive("back", at(27, 18, 0), 50, B, A, 74.0),
    ];
    assert_eq!(crosscheck(&trips, &drives)[0].status, RowStatus::NoDrive);
}

#[test]
fn a_round_trip_does_not_match_a_one_way_drive_to_its_destination() {
    let trips = [round_trip("t1", at(27, 8, 0), None, A, B, 74.0)];
    let rows = crosscheck(&trips, &[drive("d1", at(27, 8, 0), 50, A, B, 74.0)]);
    assert_eq!(find(&rows, "t1").status, RowStatus::NoDrive);
}

#[test]
fn a_one_way_trip_does_not_join_drives_over_a_long_stop() {
    let trips = [trip("t1", at(27, 8, 0), A, C, 148.0)];
    let drives = [
        drive("d1", at(27, 8, 0), 50, A, B, 74.0),
        drive("d2", at(27, 11, 0), 50, B, C, 74.0),
    ];
    assert_eq!(find(&crosscheck(&trips, &drives), "t1").status, RowStatus::NoDrive);
}
