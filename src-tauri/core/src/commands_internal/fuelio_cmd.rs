//! Fuelio commands (Task 90): the cross-check, the map track, the Dropbox
//! sync, and the writes (overwrite a trip, add a trip, ignore a drive).

use std::collections::HashMap;
use std::path::Path;

use chrono::{Datelike, NaiveDateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::check_read_only;
use crate::commands_internal::helpers::trip_order;
use crate::commands_internal::route_maps::build_route_map;
use crate::commands_internal::trips::plan_route_distance;
use crate::db::Database;
use crate::fuelio::{self, parse, CrosscheckRow, Drive, TripRef};
use crate::models::{DistanceWriteback, RouteMode, Waypoint};
use crate::route_map::polyline;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelioReport {
    /// The folder the app reads, for the "no files" hint on the page.
    pub folder: String,
    pub folder_exists: bool,
    /// Drives in the folder for the year.
    pub drive_count: usize,
    /// The three Dropbox secrets are set: the page offers the sync.
    pub dropbox_configured: bool,
    pub rows: Vec<CrosscheckRow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelioTrack {
    /// One `[lat, lon]` line per drive, every GPS fix.
    pub gps: Vec<Vec<[f64; 2]>>,
    /// The trip's stored route, if it has one.
    pub route: Option<Vec<[f64; 2]>>,
}

fn pairs(points: Vec<(f64, f64)>) -> Vec<[f64; 2]> {
    points.into_iter().map(|(a, b)| [a, b]).collect()
}

/// Fuelio is set up only through Dropbox. Without the DROPBOX_* secrets every
/// Fuelio command is refused, even when `<DATA_DIR>/fuelio` holds drives.
pub fn require_fuelio_internal(dropbox_configured: bool) -> Result<(), String> {
    if dropbox_configured {
        Ok(())
    } else {
        Err("Dropbox is not configured: set DROPBOX_APP_KEY, DROPBOX_APP_SECRET and DROPBOX_REFRESH_TOKEN"
            .to_string())
    }
}

/// The cross-check of one vehicle's trips of `year` against the Fuelio
/// drives that start in `year`.
pub fn get_fuelio_crosscheck_internal(
    db: &Database,
    data_dir: &Path,
    vehicle_id: &str,
    year: i32,
) -> Result<FuelioReport, String> {
    let folder = data_dir.join(fuelio::FOLDER_NAME);
    let drives = fuelio::scan_year(&folder, year);

    let trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let points: HashMap<String, (f64, f64)> = db
        .all_places()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|p| Some((p.id, (p.lat?, p.lon?))))
        .collect();
    let ids: Vec<String> = trips.iter().map(|t| t.id.to_string()).collect();
    let routes = db.get_route_maps_for_trips(&ids).map_err(|e| e.to_string())?;

    let refs: Vec<TripRef> = trips
        .into_iter()
        .map(|t| {
            let id = t.id.to_string();
            let saved = routes.get(&id);
            TripRef {
                round_trip: saved.is_some_and(|r| r.round_trip)
                    || t.origin_place_id == t.destination_place_id,
                end: t.end_datetime,
                route: saved
                    .map(|r| polyline::decode(&r.polyline))
                    .filter(|line| line.len() >= 2),
                id,
                start: t.start_datetime,
                origin_point: points.get(&t.origin_place_id.to_string()).copied(),
                destination_point: points.get(&t.destination_place_id.to_string()).copied(),
                origin: t.origin,
                destination: t.destination,
                km: t.distance_km,
            }
        })
        .collect();

    let ignored = db.ignored_fuelio_drives(vehicle_id).map_err(|e| e.to_string())?;
    Ok(FuelioReport {
        folder: folder.display().to_string(),
        folder_exists: folder.is_dir(),
        drive_count: drives.len(),
        dropbox_configured: crate::fuelio::dropbox::DropboxConfig::from_env().is_some(),
        rows: mark_ignored(fuelio::crosscheck(&refs, &drives), &ignored),
    })
}

/// The GPS fixes of `drive_ids` and the stored route of `trip_id`, for the map.
pub fn get_fuelio_track_internal(
    db: &Database,
    data_dir: &Path,
    drive_ids: &[String],
    trip_id: Option<&str>,
) -> Result<FuelioTrack, String> {
    let folder = data_dir.join(fuelio::FOLDER_NAME);
    let mut gps = Vec::new();
    for id in drive_ids {
        check_drive_id(id)?;
        let text = parse::read_data_file(&parse::data_file_path(&folder, id))?;
        gps.push(pairs(
            parse::parse_csv(&text)
                .into_iter()
                .map(|f| (f.lat, f.lon))
                .collect(),
        ));
    }
    let route = match trip_id {
        Some(id) => db
            .get_route_map(id)
            .map_err(|e| e.to_string())?
            .map(|r| pairs(polyline::decode(&r.polyline))),
        None => None,
    };
    Ok(FuelioTrack { gps, route })
}

/// The trip fields that `apply_fuelio_to_trip` overwrites from the GPS.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FuelioFields {
    pub start: bool,
    pub end: bool,
    pub distance: bool,
    pub route: bool,
}

/// What an overwrite does (dry run) or did.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelioApply {
    pub trip_id: String,
    pub start_before: NaiveDateTime,
    pub start_after: NaiveDateTime,
    pub end_before: Option<NaiveDateTime>,
    pub end_after: Option<NaiveDateTime>,
    pub distance_before: f64,
    pub distance_after: f64,
    /// The distance plan (odometer chain + margin), when the distance is selected.
    pub writeback: Option<DistanceWriteback>,
    pub route_written: bool,
    pub applied: bool,
}

/// The drives of `drive_ids`, oldest first.
fn load_drives(data_dir: &Path, drive_ids: &[String]) -> Result<Vec<Drive>, String> {
    let folder = data_dir.join(fuelio::FOLDER_NAME);
    let mut drives = Vec::new();
    for id in drive_ids {
        check_drive_id(id)?;
        let text = parse::read_data_file(&parse::data_file_path(&folder, id))?;
        drives.push(
            parse::drive_from_fixes(id, &parse::parse_csv(&text))
                .ok_or_else(|| format!("Drive {id} has fewer than two GPS fixes"))?,
        );
    }
    drives.sort_by_key(|d| d.start);
    Ok(drives)
}

/// Every GPS fix of `drive_ids`, oldest drive first: the geometry of a stored
/// route. Not `Drive::track` (one point per 100 m): that cut corners by up to
/// 93 m on the map (2026-10-07). A fix that repeats the previous one is dropped.
fn full_track(data_dir: &Path, drives: &[Drive]) -> Result<Vec<(f64, f64)>, String> {
    let folder = data_dir.join(fuelio::FOLDER_NAME);
    let mut points: Vec<(f64, f64)> = Vec::new();
    for d in drives {
        let text = parse::read_data_file(&parse::data_file_path(&folder, &d.id))?;
        for f in parse::parse_csv(&text) {
            if points.last() != Some(&(f.lat, f.lon)) {
                points.push((f.lat, f.lon));
            }
        }
    }
    Ok(points)
}

/// Only a missing row can be ignored, and only when the user ignored every one
/// of its drives: a drive that a later sync adds to the chain shows it again.
fn mark_ignored(
    mut rows: Vec<fuelio::CrosscheckRow>,
    ignored: &std::collections::HashSet<String>,
) -> Vec<fuelio::CrosscheckRow> {
    for row in &mut rows {
        row.ignored = row.status == fuelio::RowStatus::Missing
            && !row.drive_ids.is_empty()
            && row.drive_ids.iter().all(|id| ignored.contains(id));
    }
    rows
}

/// Ignore the drives of a missing row (for example a private drive), or show
/// them again. Ignored rows are hidden on the page unless the user asks.
pub fn set_fuelio_drives_ignored_internal(
    db: &Database,
    app_state: &AppState,
    vehicle_id: &str,
    drive_ids: &[String],
    ignored: bool,
) -> Result<(), String> {
    check_read_only!(app_state);
    if drive_ids.is_empty() {
        return Err("No drive selected".into());
    }
    for id in drive_ids {
        check_drive_id(id)?;
    }
    db.set_fuelio_drives_ignored(vehicle_id, drive_ids, ignored)
        .map_err(|e| e.to_string())
}

/// A drive ID becomes a file name: digits only, so no path can escape the folder.
fn check_drive_id(id: &str) -> Result<(), String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Invalid drive id: {id}"));
    }
    Ok(())
}

/// The logbook keeps whole minutes.
fn to_minute(t: NaiveDateTime) -> NaiveDateTime {
    t.with_second(0).and_then(|t| t.with_nanosecond(0)).unwrap_or(t)
}

/// Overwrite the selected fields of a trip with the values of its Fuelio
/// drives (Task 90). A dry run plans and writes nothing.
///
/// - start / end: the GPS start and end, in whole minutes. A new start must not
///   move the trip past another trip: the order is the odometer chain.
/// - distance: the GPS km in whole km, through the same plan as the route
///   distance write-back (task 87): the later odometers move, and the margin
///   impact comes with the plan.
/// - route: the GPS track becomes the trip's stored route (direct, no provider).
///
/// Everything selected is written in one transaction.
pub fn apply_fuelio_to_trip_internal(
    db: &Database,
    app_state: &AppState,
    data_dir: &Path,
    trip_id: &str,
    drive_ids: &[String],
    fields: FuelioFields,
    dry_run: bool,
) -> Result<FuelioApply, String> {
    if !(fields.start || fields.end || fields.distance || fields.route) {
        return Err("Select at least one field to overwrite".into());
    }
    if drive_ids.is_empty() {
        return Err("No Fuelio drive given".into());
    }
    let drives = load_drives(data_dir, drive_ids)?;
    let (first, last) = (&drives[0], &drives[drives.len() - 1]);
    let gps_km: f64 = drives.iter().map(|d| d.km).sum();

    let existing = db
        .get_trip(trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {trip_id}"))?;
    let start = if fields.start { to_minute(first.start) } else { existing.start_datetime };
    let end = if fields.end { Some(to_minute(last.end)) } else { existing.end_datetime };
    if end.is_some_and(|e| e < start) {
        return Err("The end would be before the start".into());
    }

    // The order guard: the distance plan and the odometer chain follow the
    // stored order, so a new start must keep the trip in its place.
    if start != existing.start_datetime {
        if start.year() != existing.start_datetime.year() {
            return Err("The new start is in another year; the trip order would change".into());
        }
        let mut trips = db
            .get_trips_for_vehicle_in_year(&existing.vehicle_id.to_string(), start.year())
            .map_err(|e| e.to_string())?;
        trips.sort_by(trip_order);
        let before: Vec<_> = trips.iter().map(|t| t.id).collect();
        for t in trips.iter_mut().filter(|t| t.id == existing.id) {
            t.start_datetime = start;
        }
        trips.sort_by(trip_order);
        if trips.iter().map(|t| t.id).ne(before) {
            return Err(
                "The GPS start would move the trip past another trip; the trip order (the odometer chain) would change"
                    .into(),
            );
        }
    }

    let plan = if fields.distance {
        Some(plan_route_distance(db, trip_id, gps_km)?)
    } else {
        None
    };
    let mut trip = plan
        .as_ref()
        .and_then(|p| p.updated_trip())
        .unwrap_or_else(|| existing.clone());
    let shifts = plan.as_ref().map(|p| p.shifts()).unwrap_or_default();
    trip.start_datetime = start;
    trip.end_datetime = end;
    let trip_changed = trip.start_datetime != existing.start_datetime
        || trip.end_datetime != existing.end_datetime
        || plan.as_ref().is_some_and(|p| p.changes_trip());
    if trip_changed {
        trip.updated_at = Utc::now();
    }

    let map = if fields.route {
        let track = full_track(data_dir, &drives)?;
        let point = |p: (f64, f64), name: &str| Waypoint {
            lat: p.0,
            lon: p.1,
            name: Some(name.to_string()),
            node_idx: None,
        };
        let mut map = build_route_map(
            trip_id,
            vec![
                point(first.start_point, &existing.origin),
                point(last.end_point, &existing.destination),
            ],
            polyline::encode(&track),
            gps_km,
            RouteMode::Direct,
            false,
            None,
            Vec::new(),
            None,
        )?;
        map.target_km = trip.distance_km;
        Some(map)
    } else {
        None
    };

    let result = |applied: bool, saved: Option<crate::models::Trip>| FuelioApply {
        trip_id: trip_id.to_string(),
        start_before: existing.start_datetime,
        start_after: trip.start_datetime,
        end_before: existing.end_datetime,
        end_after: trip.end_datetime,
        distance_before: existing.distance_km,
        distance_after: trip.distance_km,
        writeback: None,
        route_written: applied && fields.route,
        applied,
    }
    .with_writeback(plan, saved);

    if dry_run {
        return Ok(result(false, None));
    }
    check_read_only!(app_state);
    match &map {
        Some(map) => db
            .save_route_map_with_trip_distance(map, trip_changed.then_some(&trip), &shifts)
            .map_err(|e| e.to_string())?,
        None if trip_changed => db
            .update_trip_with_odometer_shift(&trip, &shifts)
            .map_err(|e| e.to_string())?,
        None => {}
    }
    let saved = trip_changed.then(|| trip.clone());
    Ok(result(true, saved))
}

impl FuelioApply {
    fn with_writeback(
        mut self,
        plan: Option<crate::commands_internal::trips::RouteDistancePlan>,
        saved: Option<crate::models::Trip>,
    ) -> Self {
        self.writeback = plan.map(|p| p.into_writeback(saved));
        self
    }
}

/// One place to offer for a new trip's end, with its distance from the GPS point.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOption {
    pub id: String,
    pub name: String,
    pub distance_m: f64,
}

/// What a new trip from Fuelio drives would get (Task 90).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelioAddPreview {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    /// The GPS km in whole km: what the trip gets.
    pub distance_km: f64,
    pub gps_km: f64,
    /// Every place with coordinates, nearest to the GPS start first.
    pub origin: Vec<PlaceOption>,
    /// Every place with coordinates, nearest to the GPS end first.
    pub destination: Vec<PlaceOption>,
}

/// Every place with coordinates, nearest to `point` first.
fn places_by_distance(db: &Database, point: (f64, f64)) -> Result<Vec<PlaceOption>, String> {
    let mut options: Vec<PlaceOption> = db
        .all_places()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|p| {
            let at = (p.lat?, p.lon?);
            Some(PlaceOption {
                distance_m: fuelio::geo::haversine_m(point, at),
                id: p.id,
                name: p.name,
            })
        })
        .collect();
    options.sort_by(|a, b| a.distance_m.total_cmp(&b.distance_m));
    Ok(options)
}

/// The values a new trip from `drive_ids` would get, and the places to choose
/// its ends from.
pub fn get_fuelio_add_preview_internal(
    db: &Database,
    data_dir: &Path,
    drive_ids: &[String],
) -> Result<FuelioAddPreview, String> {
    if drive_ids.is_empty() {
        return Err("No Fuelio drive given".into());
    }
    let drives = load_drives(data_dir, drive_ids)?;
    let (first, last) = (&drives[0], &drives[drives.len() - 1]);
    let gps_km: f64 = drives.iter().map(|d| d.km).sum();
    Ok(FuelioAddPreview {
        start: to_minute(first.start),
        end: to_minute(last.end),
        distance_km: crate::commands_internal::trips::logbook_km(gps_km),
        gps_km,
        origin: places_by_distance(db, first.start_point)?,
        destination: places_by_distance(db, last.end_point)?,
    })
}

/// What adding a trip did (or would do, on a dry run).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelioAdd {
    /// The new trip; `None` on a dry run.
    pub trip: Option<crate::models::Trip>,
    /// The odometer plan of the insert (ADR-046).
    pub plan: crate::models::CascadePlan,
    pub route_written: bool,
    /// The trip was added, but its route was not saved: why. Not an `Err`,
    /// because the trip exists and a retry would add it again.
    pub route_error: Option<String>,
}

/// Add a logbook trip from Fuelio drives (Task 90): the GPS start and end in
/// whole minutes, the GPS km in whole km, the ends the user confirmed. Goes
/// through the normal insert (`create_trip_cascade_internal`), so the later
/// odometers move. With `with_route`, the GPS track becomes the trip's stored
/// route afterwards.
#[allow(clippy::too_many_arguments)]
pub fn add_fuelio_trip_internal(
    db: &Database,
    app_state: &AppState,
    data_dir: &Path,
    vehicle_id: &str,
    drive_ids: &[String],
    origin_place_id: &str,
    destination_place_id: &str,
    purpose: &str,
    with_route: bool,
    dry_run: bool,
) -> Result<FuelioAdd, String> {
    if drive_ids.is_empty() {
        return Err("No Fuelio drive given".into());
    }
    let drives = load_drives(data_dir, drive_ids)?;
    let (first, last) = (&drives[0], &drives[drives.len() - 1]);
    let gps_km: f64 = drives.iter().map(|d| d.km).sum();
    let km = crate::commands_internal::trips::logbook_km(gps_km);
    let fmt = |t: NaiveDateTime| to_minute(t).format("%Y-%m-%dT%H:%M").to_string();

    let created = crate::commands_internal::trips::create_trip_cascade_internal(
        db,
        app_state,
        vehicle_id.to_string(),
        fmt(first.start),
        fmt(last.end),
        origin_place_id.to_string(),
        destination_place_id.to_string(),
        km,
        purpose.to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        dry_run,
    )?;

    let mut route_written = false;
    let mut route_error = None;
    if let (Some(trip), true) = (&created.trip, with_route) {
        // A second write: the trip exists even if this one fails.
        match save_gps_route(db, data_dir, &drives, trip, gps_km) {
            Ok(()) => route_written = true,
            Err(e) => route_error = Some(e),
        }
    }
    Ok(FuelioAdd {
        trip: created.trip,
        plan: created.plan,
        route_written,
        route_error,
    })
}

/// Save the full GPS track of `drives` as the stored route of a new `trip`.
fn save_gps_route(
    db: &Database,
    data_dir: &Path,
    drives: &[Drive],
    trip: &crate::models::Trip,
    gps_km: f64,
) -> Result<(), String> {
    let (first, last) = (&drives[0], &drives[drives.len() - 1]);
    let track = full_track(data_dir, drives)?;
    let point = |p: (f64, f64), name: &str| Waypoint {
        lat: p.0,
        lon: p.1,
        name: Some(name.to_string()),
        node_idx: None,
    };
    let mut map = build_route_map(
        &trip.id.to_string(),
        vec![point(first.start_point, &trip.origin), point(last.end_point, &trip.destination)],
        polyline::encode(&track),
        gps_km,
        RouteMode::Direct,
        false,
        None,
        Vec::new(),
        None,
    )?;
    map.target_km = trip.distance_km;
    db.save_route_map(&map).map_err(|e| e.to_string())
}

/// Copy the Fuelio drives of `year` from Dropbox into `<DATA_DIR>/fuelio`.
/// `config` is `None` when the Dropbox secrets are not set.
pub async fn sync_fuelio_dropbox_internal(
    config: Option<crate::fuelio::dropbox::DropboxConfig>,
    data_dir: &Path,
    year: i32,
) -> Result<crate::fuelio::dropbox::SyncReport, String> {
    use crate::fuelio::dropbox::{sync_year, DropboxStore};
    require_fuelio_internal(config.is_some())?;
    let config = config.expect("checked above");
    let store = DropboxStore::connect(config).await?;
    sync_year(std::sync::Arc::new(store), &data_dir.join(fuelio::FOLDER_NAME), year).await
}

#[cfg(test)]
#[path = "fuelio_cmd_tests.rs"]
mod tests;
