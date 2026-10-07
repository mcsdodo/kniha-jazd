//! Drives against trips -> report rows (Task 90). Pure: no DB, no files.
//!
//! Fuelio is the reference: every row is a run of drives, and the question is
//! "does the logbook have this drive?". A trip with no drive is no problem
//! (Fuelio does not record every drive), so it has no row.
//!
//! One trip can be several drives: Fuelio splits a drive at a stop
//! (2026-09-28: SNV -> BA was three drives). Three passes, strict first:
//! 1. complete: a run from the trip's origin to its destination;
//! 2. partial: a run on the trip's stored route that covers part of it;
//! 3. loose: the drives left over join into chains, and a chain matches the
//!    nearest trip in time that it touches (a place or the stored route).
//!
//! Only a chain that fits no trip, even loosely, is "missing".

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
/// How far the logbook start may be from the GPS start and still match
/// completely. Wide, because a wrong logbook time is what the check finds.
const MAX_START_DIFF_H: i64 = 12;
/// The partial and loose passes take a drive only if it starts this close to
/// the trip: from this long before its start to this long after its end.
/// 12 hours gave a 2.2 km city drive at 19:03 to a trip that arrived at 07:52
/// (2026-10-06).
const SIDE_WINDOW_H: i64 = 3;
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
/// A partial match: at least this share of the track on the stored route.
const PARTIAL_MIN_ON_ROUTE: f64 = 0.8;
/// A loose match: a chain end this close to a trip place...
const LOOSE_PLACE_RADIUS_M: f64 = 5_000.0;
/// ...or at least this share of the chain this close to the stored route.
const LOOSE_MIN_ON_ROUTE: f64 = 0.5;
const LOOSE_ROUTE_M: f64 = 1_000.0;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Flag {
    TimeDiffers,
    KmDiffers,
    DifferentRoute,
    /// The drives cover only part of the trip (Fuelio did not record the
    /// rest). The km difference then says how much is missing.
    PartialGps,
    /// Only the loose pass found the trip: same time, and a place or the
    /// stored route in common. Times, km and route are not compared.
    LooseMatch,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrosscheckRow {
    pub status: RowStatus,
    pub trip_id: Option<String>,
    pub trip_start: Option<NaiveDateTime>,
    pub trip_end: Option<NaiveDateTime>,
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

/// Where a partial or loose run may start: [`SIDE_WINDOW_H`] around the trip.
/// A trip without an end time counts as ending [`SIDE_WINDOW_H`] after its start.
fn side_window(trip: &TripRef) -> (NaiveDateTime, NaiveDateTime) {
    let pad = Duration::hours(SIDE_WINDOW_H);
    (trip.start - pad, trip.end.unwrap_or(trip.start + pad) + pad)
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

/// The best partial run for `trip`: unused drives that lie on its stored
/// route, inside its time, with an end at one of its places. The run with the
/// most km wins. `None` without a stored route.
fn best_partial_run(trip: &TripRef, drives: &[Drive], used: &[bool]) -> Option<(usize, usize)> {
    let line = trip.route.as_ref()?;
    let (from, to) = side_window(trip);
    let (_, max_stop) = run_rules(trip);
    let at_place = |p: (f64, f64)| {
        [trip.origin_point, trip.destination_point]
            .iter()
            .any(|place| place.is_some_and(|q| haversine_m(p, q) <= PLACE_RADIUS_M))
    };
    let mut best: Option<(f64, (usize, usize))> = None;
    for i in 0..drives.len() {
        if used[i] || drives[i].start < from || drives[i].start > to {
            continue;
        }
        let mut km = 0.0;
        for j in i..drives.len() {
            if used[j]
                || drives[j].start > to
                || (j > i && minutes(drives[j - 1].end, drives[j].start) > max_stop)
            {
                break;
            }
            km += drives[j].km;
            if trip.km > 0.0 && km > trip.km * KM_RATIO_MAX {
                break;
            }
            let run = &drives[i..=j];
            let track: Vec<(f64, f64)> = run.iter().flat_map(|d| d.track.iter().copied()).collect();
            let on_route = 1.0 - off_route_share(&track, line, OFF_ROUTE_M);
            if on_route >= PARTIAL_MIN_ON_ROUTE
                && (at_place(run[0].start_point) || at_place(run[run.len() - 1].end_point))
                && best.as_ref().is_none_or(|(k, _)| km > *k)
            {
                best = Some((km, (i, j)));
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
        trip_end: None,
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

fn matched_row(trip: &TripRef, run: &[Drive], partial: bool) -> CrosscheckRow {
    let mut row = gps_row(RowStatus::Matched, run);
    let gps_km = row.gps_km.unwrap_or(0.0);
    // A partial run that starts mid-trip says nothing about the start time.
    row.start_diff_min = (!partial || near(run[0].start_point, trip.origin_point))
        .then(|| minutes(trip.start, run[0].start));
    row.km_diff_pct = (trip.km > 0.0).then(|| (gps_km - trip.km) / trip.km * 100.0);
    row.off_route_pct = trip.route.as_ref().map(|line| {
        let track: Vec<(f64, f64)> = run.iter().flat_map(|d| d.track.iter().copied()).collect();
        off_route_share(&track, line, OFF_ROUTE_M) * 100.0
    });
    if row.start_diff_min.is_some_and(|m| m.abs() > FLAG_START_DIFF_MIN) {
        row.flags.push(Flag::TimeDiffers);
    }
    if !partial && row.km_diff_pct.is_some_and(|p| p.abs() > FLAG_KM_DIFF_PCT) {
        row.flags.push(Flag::KmDiffers);
    }
    if row.off_route_pct.is_some_and(|p| p > FLAG_OFF_ROUTE_PCT) {
        row.flags.push(Flag::DifferentRoute);
    }
    if partial {
        row.flags.push(Flag::PartialGps);
    }
    with_trip(row, trip)
}

fn with_trip(mut row: CrosscheckRow, trip: &TripRef) -> CrosscheckRow {
    row.trip_id = Some(trip.id.clone());
    row.trip_start = Some(trip.start);
    row.trip_end = trip.end;
    row.origin = Some(trip.origin.clone());
    row.destination = Some(trip.destination.clone());
    row.trip_km = Some(trip.km);
    row
}

/// Does `chain` loosely fit `trip`? `taken_km`: the GPS km the trip has already.
fn loose_fit(trip: &TripRef, chain: &[Drive], taken_km: f64) -> bool {
    let (from, to) = side_window(trip);
    let start = chain[0].start;
    if start < from || start > to {
        return false;
    }
    let km: f64 = chain.iter().map(|d| d.km).sum();
    if trip.km > 0.0 && taken_km + km > trip.km * KM_RATIO_MAX {
        return false;
    }
    let ends = [chain[0].start_point, chain[chain.len() - 1].end_point];
    let at_place = ends.iter().any(|p| {
        [trip.origin_point, trip.destination_point]
            .iter()
            .any(|q| q.is_some_and(|q| haversine_m(*p, q) <= LOOSE_PLACE_RADIUS_M))
    });
    at_place
        || trip.route.as_ref().is_some_and(|line| {
            let track: Vec<(f64, f64)> =
                chain.iter().flat_map(|d| d.track.iter().copied()).collect();
            1.0 - off_route_share(&track, line, LOOSE_ROUTE_M) >= LOOSE_MIN_ON_ROUTE
        })
}

fn loose_row(trip: &TripRef, chain: &[Drive]) -> CrosscheckRow {
    let mut row = gps_row(RowStatus::Matched, chain);
    let gps_km = row.gps_km.unwrap_or(0.0);
    row.start_diff_min = Some(minutes(trip.start, chain[0].start));
    row.km_diff_pct = (trip.km > 0.0).then(|| (gps_km - trip.km) / trip.km * 100.0);
    row.flags.push(Flag::LooseMatch);
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
    // Pass 1: complete runs. Pass 2: partial runs for the trips left over, so
    // a partial match never takes a drive that a complete match needs.
    let mut runs: Vec<Option<((usize, usize), bool)>> = trips
        .iter()
        .map(|trip| {
            let run = best_run(trip, drives, &used)?;
            used[run.0..=run.1].iter_mut().for_each(|u| *u = true);
            Some((run, false))
        })
        .collect();
    for (trip, run) in trips.iter().zip(runs.iter_mut()) {
        if run.is_none() {
            if let Some((i, j)) = best_partial_run(trip, drives, &used) {
                used[i..=j].iter_mut().for_each(|u| *u = true);
                *run = Some(((i, j), true));
            }
        }
    }
    let mut taken_km: Vec<f64> = vec![0.0; trips.len()];
    let mut rows: Vec<CrosscheckRow> = Vec::new();
    for (k, (trip, run)) in trips.iter().zip(&runs).enumerate() {
        if let Some(((i, j), partial)) = run {
            let row = matched_row(trip, &drives[*i..=*j], *partial);
            taken_km[k] = row.gps_km.unwrap_or(0.0);
            rows.push(row);
        }
    }
    // Pass 3: loose. Each chain goes to the nearest trip in time that fits.
    for chain in missing_chains(drives, &used) {
        let best = trips
            .iter()
            .enumerate()
            .filter(|(k, trip)| loose_fit(trip, &chain, taken_km[*k]))
            .min_by_key(|(_, trip)| minutes(trip.start, chain[0].start).abs());
        match best {
            Some((k, trip)) => {
                taken_km[k] += chain.iter().map(|d| d.km).sum::<f64>();
                rows.push(loose_row(trip, &chain));
            }
            None => rows.push(gps_row(RowStatus::Missing, &chain)),
        }
    }
    // Newest drive first: the page opens on the latest drives.
    rows.sort_by_key(|r| std::cmp::Reverse(r.gps_start));
    rows
}

#[cfg(test)]
#[path = "matching_tests.rs"]
mod tests;
