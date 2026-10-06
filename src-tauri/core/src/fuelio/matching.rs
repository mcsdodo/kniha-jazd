//! Drives against trips -> report rows (Task 90). Pure: no DB, no files.
//!
//! One trip can be several drives: Fuelio splits a drive at a stop
//! (2026-09-28: SNV -> BA was three drives). So a trip matches a *run* of
//! consecutive drives that starts at its origin and ends at its destination.
//! Trips are matched first, oldest first; the drives that no trip uses then
//! join into "missing" chains.

use chrono::{Duration, NaiveDateTime};
use serde::Serialize;

use super::geo::{haversine_m, off_route_share};
use super::parse::Drive;

/// A drive end this close to a place counts as "at" the place.
const PLACE_RADIUS_M: f64 = 2_000.0;
/// The longest stop inside one trip's run of drives.
const MAX_STOP_IN_TRIP_MIN: i64 = 90;
/// The longest stop at the turnaround of a round trip without an end time.
const MAX_ROUND_TRIP_STOP_H: i64 = 12;
/// The longest stop inside one missing chain.
const MAX_STOP_IN_CHAIN_MIN: i64 = 60;
/// How far the logbook start may be from the GPS start and still match.
const MAX_START_DIFF_H: i64 = 12;
/// The accepted GPS km, as a share of the logbook km.
const KM_RATIO_MIN: f64 = 0.5;
const KM_RATIO_MAX: f64 = 1.5;
/// A highway row: at least this long...
const HIGHWAY_MIN_KM: f64 = 30.0;
/// ...or at least this many minutes above 100 km/h.
const HIGHWAY_MIN_FAST_MIN: f64 = 5.0;
/// Flag thresholds.
const FLAG_START_DIFF_MIN: i64 = 30;
const FLAG_KM_DIFF_PCT: f64 = 10.0;
const OFF_ROUTE_M: f64 = 500.0;
const FLAG_OFF_ROUTE_PCT: f64 = 10.0;

/// The trip data the matcher needs.
#[derive(Debug, Clone)]
pub struct TripRef {
    pub id: String,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    /// Out and back to the origin (the stored route is a round trip, or the
    /// origin is the destination). The drives must end at the origin, and the
    /// stop at the turnaround can last the whole trip (2026-08-27: SNV ->
    /// Poprad -> SNV with a 3-hour stop). The turnaround is not checked.
    pub round_trip: bool,
    pub origin: String,
    pub destination: String,
    /// `None` when the place has no coordinates: that end is not checked.
    pub origin_point: Option<(f64, f64)>,
    pub destination_point: Option<(f64, f64)>,
    pub km: f64,
    /// The stored route (`trip_routes`), decoded.
    pub route: Option<Vec<(f64, f64)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RowStatus {
    /// A logbook trip with its drives.
    Matched,
    /// Drives with no logbook trip.
    Missing,
    /// A logbook trip with no drive (Fuelio did not record, or the drive
    /// does not fit).
    NoDrive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Flag {
    TimeDiffers,
    KmDiffers,
    DifferentRoute,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrosscheckRow {
    pub status: RowStatus,
    pub trip_id: Option<String>,
    pub trip_start: Option<NaiveDateTime>,
    pub origin: Option<String>,
    pub destination: Option<String>,
    pub trip_km: Option<f64>,
    pub drive_ids: Vec<String>,
    pub gps_start: Option<NaiveDateTime>,
    pub gps_end: Option<NaiveDateTime>,
    pub gps_km: Option<f64>,
    pub fast_minutes: Option<f64>,
    pub max_kmh: Option<f64>,
    pub is_highway: bool,
    /// GPS start minus logbook start, in minutes.
    pub start_diff_min: Option<i64>,
    /// (GPS km - logbook km) / logbook km, in percent.
    pub km_diff_pct: Option<f64>,
    /// The share of track points off the stored route, in percent.
    pub off_route_pct: Option<f64>,
    pub flags: Vec<Flag>,
}

fn near(point: (f64, f64), place: Option<(f64, f64)>) -> bool {
    place.is_none_or(|p| haversine_m(point, p) <= PLACE_RADIUS_M)
}

fn minutes(a: NaiveDateTime, b: NaiveDateTime) -> i64 {
    (b - a).num_minutes()
}

/// Where the run of drives must end, and the longest stop inside it.
fn run_rules(trip: &TripRef) -> (Option<(f64, f64)>, i64) {
    if !trip.round_trip {
        return (trip.destination_point, MAX_STOP_IN_TRIP_MIN);
    }
    let span = trip
        .end
        .map_or(MAX_ROUND_TRIP_STOP_H * 60, |end| minutes(trip.start, end));
    (trip.origin_point, span.max(MAX_STOP_IN_TRIP_MIN))
}

/// The best run of unused drives for `trip`, as an index range.
fn best_run(trip: &TripRef, drives: &[Drive], used: &[bool]) -> Option<(usize, usize)> {
    let window = Duration::hours(MAX_START_DIFF_H);
    let km_ok = |km: f64| {
        trip.km <= 0.0 || (km >= trip.km * KM_RATIO_MIN && km <= trip.km * KM_RATIO_MAX)
    };
    let (end_point, max_stop) = run_rules(trip);
    let mut best: Option<((i64, f64), (usize, usize))> = None;
    for i in 0..drives.len() {
        let first = &drives[i];
        if used[i]
            || (first.start - trip.start).abs() > window
            || !near(first.start_point, trip.origin_point)
        {
            continue;
        }
        let mut km = 0.0;
        for j in i..drives.len() {
            if used[j] || (j > i && minutes(drives[j - 1].end, drives[j].start) > max_stop) {
                break;
            }
            km += drives[j].km;
            if trip.km > 0.0 && km > trip.km * KM_RATIO_MAX {
                break;
            }
            if near(drives[j].end_point, end_point) && km_ok(km) {
                let score = (minutes(trip.start, first.start).abs(), (km - trip.km).abs());
                if best.as_ref().is_none_or(|(s, _)| score < *s) {
                    best = Some((score, (i, j)));
                }
            }
        }
    }
    best.map(|(_, run)| run)
}

/// The GPS fields of a row, from a run of drives.
fn gps_row(status: RowStatus, run: &[Drive]) -> CrosscheckRow {
    let km: f64 = run.iter().map(|d| d.km).sum();
    let fast: f64 = run.iter().map(|d| d.fast_minutes).sum();
    CrosscheckRow {
        status,
        trip_id: None,
        trip_start: None,
        origin: None,
        destination: None,
        trip_km: None,
        drive_ids: run.iter().map(|d| d.id.clone()).collect(),
        gps_start: run.first().map(|d| d.start),
        gps_end: run.last().map(|d| d.end),
        gps_km: Some(km),
        fast_minutes: Some(fast),
        max_kmh: Some(run.iter().map(|d| d.max_kmh).fold(0.0, f64::max)),
        is_highway: km >= HIGHWAY_MIN_KM || fast >= HIGHWAY_MIN_FAST_MIN,
        start_diff_min: None,
        km_diff_pct: None,
        off_route_pct: None,
        flags: Vec::new(),
    }
}

fn matched_row(trip: &TripRef, run: &[Drive]) -> CrosscheckRow {
    let mut row = gps_row(RowStatus::Matched, run);
    let gps_km = row.gps_km.unwrap_or(0.0);
    row.start_diff_min = Some(minutes(trip.start, run[0].start));
    row.km_diff_pct = (trip.km > 0.0).then(|| (gps_km - trip.km) / trip.km * 100.0);
    row.off_route_pct = trip.route.as_ref().map(|line| {
        let track: Vec<(f64, f64)> = run.iter().flat_map(|d| d.track.iter().copied()).collect();
        off_route_share(&track, line, OFF_ROUTE_M) * 100.0
    });
    if row.start_diff_min.is_some_and(|m| m.abs() > FLAG_START_DIFF_MIN) {
        row.flags.push(Flag::TimeDiffers);
    }
    if row.km_diff_pct.is_some_and(|p| p.abs() > FLAG_KM_DIFF_PCT) {
        row.flags.push(Flag::KmDiffers);
    }
    if row.off_route_pct.is_some_and(|p| p > FLAG_OFF_ROUTE_PCT) {
        row.flags.push(Flag::DifferentRoute);
    }
    with_trip(row, trip)
}

fn with_trip(mut row: CrosscheckRow, trip: &TripRef) -> CrosscheckRow {
    row.trip_id = Some(trip.id.clone());
    row.trip_start = Some(trip.start);
    row.origin = Some(trip.origin.clone());
    row.destination = Some(trip.destination.clone());
    row.trip_km = Some(trip.km);
    row
}

fn no_drive_row(trip: &TripRef) -> CrosscheckRow {
    let mut row = gps_row(RowStatus::NoDrive, &[]);
    row.gps_km = None;
    row.fast_minutes = None;
    row.max_kmh = None;
    row.is_highway = trip.km >= HIGHWAY_MIN_KM;
    with_trip(row, trip)
}

/// Unused drives -> runs of drives with short stops at the same place.
fn missing_chains(drives: &[Drive], used: &[bool]) -> Vec<Vec<Drive>> {
    let mut chains: Vec<Vec<Drive>> = Vec::new();
    for (d, _) in drives.iter().zip(used).filter(|(_, u)| !**u) {
        let joins = chains.last().and_then(|c| c.last()).is_some_and(|prev| {
            minutes(prev.end, d.start) <= MAX_STOP_IN_CHAIN_MIN
                && haversine_m(prev.end_point, d.start_point) <= PLACE_RADIUS_M
        });
        if joins {
            chains.last_mut().unwrap().push(d.clone());
        } else {
            chains.push(vec![d.clone()]);
        }
    }
    chains
}

/// The cross-check report: one row per trip and one per missing chain, in
/// time order. `drives` must be sorted by start (as [`super::scan_dir`]
/// returns them).
pub fn crosscheck(trips: &[TripRef], drives: &[Drive]) -> Vec<CrosscheckRow> {
    let mut trips: Vec<&TripRef> = trips.iter().collect();
    trips.sort_by_key(|t| t.start);
    let mut used = vec![false; drives.len()];
    let mut rows = Vec::new();
    for trip in trips {
        match best_run(trip, drives, &used) {
            Some((i, j)) => {
                used[i..=j].iter_mut().for_each(|u| *u = true);
                rows.push(matched_row(trip, &drives[i..=j]));
            }
            None => rows.push(no_drive_row(trip)),
        }
    }
    rows.extend(
        missing_chains(drives, &used)
            .iter()
            .map(|c| gps_row(RowStatus::Missing, c)),
    );
    // Trips first on a tie: a missing drive at the same minute reads as "next to" its trip.
    rows.sort_by_key(|r| {
        (
            r.trip_start.or(r.gps_start),
            r.status == RowStatus::Missing,
        )
    });
    rows
}

#[cfg(test)]
#[path = "matching_tests.rs"]
mod tests;
