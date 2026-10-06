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

/// True if a complete or partial match (not a loose one) took `trip_id`.
fn strict(rows: &[CrosscheckRow], trip_id: &str) -> bool {
    rows.iter()
        .any(|r| r.trip_id.as_deref() == Some(trip_id) && !r.flags.contains(&Flag::LooseMatch))
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
    assert_eq!(r.trip_end, None);
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
    assert!(!strict(&crosscheck(&trips, &drives), "t1"));
}

#[test]
fn a_drive_more_than_12_hours_away_does_not_match() {
    let trips = [trip("t1", at(28, 6, 0), A, B, 74.0)];
    let drives = [drive("d1", at(28, 19, 0), 50, A, B, 74.0)];
    let rows = crosscheck(&trips, &drives);
    assert!(!strict(&rows, "t1"));
}

#[test]
fn a_drive_with_half_the_km_does_not_match() {
    let trips = [trip("t1", at(28, 6, 0), A, A, 200.0)];
    let drives = [drive("d1", at(28, 6, 0), 20, A, A, 5.0)];
    let rows = crosscheck(&trips, &drives);
    assert!(!strict(&rows, "t1"));
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
    assert!(!strict(&rows, "t2"));
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
fn rows_come_out_in_gps_time_order() {
    let rows = crosscheck(
        &[trip("t1", at(28, 12, 0), A, B, 74.0)],
        &[
            drive("early", at(27, 9, 0), 50, C, C, 74.0),
            drive("mid", at(28, 12, 30), 50, A, B, 74.0),
            drive("late", at(29, 20, 0), 50, C, C, 74.0),
        ],
    );
    // The missing rows and the matched row are merged by GPS time.
    let ids: Vec<&str> = rows.iter().map(|r| r.drive_ids[0].as_str()).collect();
    assert_eq!(ids, vec!["early", "mid", "late"]);
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
    assert!(!strict(&crosscheck(&trips, &drives), "t1"));
}

#[test]
fn a_round_trip_does_not_match_a_one_way_drive_to_its_destination() {
    let trips = [round_trip("t1", at(27, 8, 0), None, A, B, 74.0)];
    let rows = crosscheck(&trips, &[drive("d1", at(27, 8, 0), 50, A, B, 74.0)]);
    assert!(!strict(&rows, "t1"));
}

#[test]
fn a_one_way_trip_does_not_join_drives_over_a_long_stop() {
    let trips = [trip("t1", at(27, 8, 0), A, C, 148.0)];
    let drives = [
        drive("d1", at(27, 8, 0), 50, A, B, 74.0),
        drive("d2", at(27, 11, 0), 50, B, C, 74.0),
    ];
    assert!(!strict(&crosscheck(&trips, &drives), "t1"));
}

// 2026-08-18: BA -> Brno -> BA is one logbook trip; Fuelio recorded only the
// way back, starting 14 km past the Brno place. A partial match needs the
// stored route.
fn routed(t: TripRef, line: Vec<(f64, f64)>) -> TripRef {
    TripRef { route: Some(line), ..t }
}

#[test]
fn a_drive_on_the_stored_route_that_covers_part_of_the_trip_is_a_partial_match() {
    let t = routed(
        round_trip("t1", at(18, 8, 0), Some(at(18, 13, 0)), A, B, 148.0),
        vec![A, B, A],
    );
    let back = drive("back", at(18, 8, 5), 112, B, A, 74.0);
    let rows = crosscheck(&[t], &[back]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, RowStatus::Matched);
    assert_eq!(rows[0].drive_ids, vec!["back"]);
    assert_eq!(rows[0].flags, vec![Flag::PartialGps]);
    // The run does not start at the origin: no start time to compare.
    assert_eq!(rows[0].start_diff_min, None);
    assert!((rows[0].km_diff_pct.unwrap() + 50.0).abs() < 1e-9);
}

#[test]
fn a_partial_match_that_starts_at_the_origin_keeps_the_time_difference() {
    let t = routed(trip("t1", at(18, 8, 0), A, C, 148.0), vec![A, B, C]);
    // Fuelio stopped recording halfway, 40 km before the destination.
    let rows = crosscheck(&[t], &[drive("out", at(18, 8, 45), 50, A, (48.0, 21.45), 107.0)]);
    assert_eq!(rows[0].flags, vec![Flag::TimeDiffers, Flag::PartialGps]);
    assert_eq!(rows[0].start_diff_min, Some(45));
}

#[test]
fn a_partial_match_needs_a_stored_route() {
    let t = round_trip("t1", at(18, 8, 0), Some(at(18, 13, 0)), A, B, 148.0);
    let rows = crosscheck(&[t], &[drive("back", at(18, 8, 5), 112, B, A, 74.0)]);
    assert!(!strict(&rows, "t1"));
}

#[test]
fn a_partial_match_needs_the_track_on_the_stored_route() {
    let t = routed(
        round_trip("t1", at(18, 8, 0), Some(at(18, 13, 0)), A, B, 148.0),
        vec![A, B, A],
    );
    let mut back = drive("back", at(18, 8, 5), 112, B, A, 74.0);
    back.track = vec![B, (47.7, 20.8), (47.7, 20.5), (47.7, 20.2), A];
    let rows = crosscheck(&[t], &[back]);
    assert!(!strict(&rows, "t1"));
}

#[test]
fn a_partial_match_needs_an_end_at_a_trip_place() {
    let t = routed(trip("t1", at(18, 8, 0), A, C, 148.0), vec![A, C]);
    // On the route, but from 20.3 to 21.7: neither end is at A or C.
    let mid = drive("mid", at(18, 8, 30), 60, (48.0, 20.3), (48.0, 21.7), 104.0);
    let rows = crosscheck(&[t], &[mid]);
    assert!(!strict(&rows, "t1"));
}

#[test]
fn a_partial_match_outside_the_trip_time_does_not_match() {
    let t = routed(
        round_trip("t1", at(18, 8, 0), Some(at(18, 13, 0)), A, B, 148.0),
        vec![A, B, A],
    );
    let rows = crosscheck(&[t], &[drive("back", at(19, 9, 0), 112, B, A, 74.0)]);
    assert!(!strict(&rows, "t1"));
}

#[test]
fn a_complete_match_of_a_later_trip_wins_over_a_partial_one() {
    let early = routed(trip("early", at(18, 8, 0), A, C, 148.0), vec![A, B, C]);
    let late = trip("late", at(18, 9, 0), A, B, 74.0);
    let rows = crosscheck(&[early, late], &[drive("d1", at(18, 9, 0), 50, A, B, 74.0)]);
    assert_eq!(find(&rows, "late").drive_ids, vec!["d1"]);
    assert!(!strict(&rows, "early"));
}

// The loose pass: Fuelio is the reference. A drive is "missing" only if no
// trip fits it even loosely.

#[test]
fn a_trip_without_a_drive_has_no_row() {
    assert!(crosscheck(&[trip("t1", at(28, 10, 0), A, B, 74.0)], &[]).is_empty());
}

#[test]
fn a_drive_that_ends_near_a_trip_place_matches_loosely() {
    let trips = [trip("t1", at(28, 8, 0), A, B, 74.0)];
    // Starts elsewhere, ends 4 km from B, two hours after the logbook time.
    let rows = crosscheck(&trips, &[drive("d1", at(28, 10, 0), 50, C, (48.036, 21.0), 74.0)]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, RowStatus::Matched);
    assert_eq!(rows[0].trip_id.as_deref(), Some("t1"));
    assert_eq!(rows[0].flags, vec![Flag::LooseMatch]);
    assert_eq!(rows[0].start_diff_min, Some(120));
}

#[test]
fn a_drive_on_the_stored_route_matches_loosely() {
    let t = routed(trip("t1", at(28, 8, 0), A, C, 148.0), vec![A, B, C]);
    let mid = drive("mid", at(28, 9, 0), 60, (48.0, 20.3), (48.0, 21.7), 104.0);
    let rows = crosscheck(&[t], &[mid]);
    assert_eq!(rows[0].flags, vec![Flag::LooseMatch]);
}

#[test]
fn a_loose_match_needs_the_trip_time() {
    let trips = [trip("t1", at(28, 8, 0), A, B, 74.0)];
    let rows = crosscheck(&trips, &[drive("d1", at(28, 20, 30), 50, C, B, 74.0)]);
    assert_eq!(rows[0].status, RowStatus::Missing);
}

#[test]
fn a_loose_match_can_use_the_trip_end_time() {
    let mut t = trip("t1", at(28, 8, 0), A, B, 74.0);
    t.end = Some(at(28, 18, 0));
    let rows = crosscheck(&[t], &[drive("d1", at(29, 5, 0), 50, C, B, 74.0)]);
    assert_eq!(rows[0].status, RowStatus::Matched);
    assert_eq!(rows[0].trip_end, Some(at(28, 18, 0)));
}

#[test]
fn a_loose_match_needs_a_trip_place_or_the_stored_route() {
    let trips = [trip("t1", at(28, 8, 0), A, B, 74.0)];
    let rows = crosscheck(&trips, &[drive("d1", at(28, 8, 0), 50, C, (48.5, 22.0), 74.0)]);
    assert_eq!(rows[0].status, RowStatus::Missing);
}

#[test]
fn a_second_full_drive_on_a_matched_trip_is_missing() {
    // The trip already has its 74 km; another 74 km would be 200%.
    let trips = [trip("t1", at(28, 8, 0), A, B, 74.0)];
    let drives = [
        drive("d1", at(28, 8, 0), 50, A, B, 74.0),
        drive("d2", at(28, 13, 0), 50, A, B, 74.0),
    ];
    let rows = crosscheck(&trips, &drives);
    let d2 = rows.iter().find(|r| r.drive_ids == vec!["d2"]).unwrap();
    assert_eq!(d2.status, RowStatus::Missing);
}

#[test]
fn a_short_fragment_next_to_a_matched_trip_matches_loosely() {
    let trips = [trip("t1", at(28, 8, 0), A, B, 74.0)];
    let drives = [
        drive("d1", at(28, 8, 0), 50, A, B, 70.0),
        drive("frag", at(28, 12, 0), 10, B, (48.01, 21.0), 3.0),
    ];
    let rows = crosscheck(&trips, &drives);
    let frag = rows.iter().find(|r| r.drive_ids == vec!["frag"]).unwrap();
    assert_eq!(frag.trip_id.as_deref(), Some("t1"));
    assert_eq!(frag.flags, vec![Flag::LooseMatch]);
}

#[test]
fn a_loose_match_takes_the_trip_nearest_in_time() {
    let trips = [
        trip("far", at(28, 6, 0), A, B, 74.0),
        trip("near", at(28, 11, 0), A, B, 74.0),
    ];
    let rows = crosscheck(&trips, &[drive("d1", at(28, 12, 0), 50, C, B, 74.0)]);
    assert_eq!(rows[0].trip_id.as_deref(), Some("near"));
}
