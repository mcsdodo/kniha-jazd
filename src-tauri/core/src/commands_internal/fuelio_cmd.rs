//! Fuelio cross-check commands (Task 90, POC). Read-only.

use std::collections::HashMap;
use std::path::Path;

use chrono::Datelike;
use serde::Serialize;

use crate::db::Database;
use crate::fuelio::{self, parse, CrosscheckRow, TripRef};
use crate::route_map::polyline;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelioReport {
    /// The folder the app reads, for the "no files" hint on the page.
    pub folder: String,
    pub folder_exists: bool,
    /// Drives in the folder for the year.
    pub drive_count: usize,
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

/// The cross-check of one vehicle's trips of `year` against the Fuelio
/// drives that start in `year`.
pub fn get_fuelio_crosscheck_internal(
    db: &Database,
    data_dir: &Path,
    vehicle_id: &str,
    year: i32,
) -> Result<FuelioReport, String> {
    let folder = data_dir.join(fuelio::FOLDER_NAME);
    let drives: Vec<_> = fuelio::scan_dir(&folder)
        .into_iter()
        .filter(|d| d.start.year() == year)
        .collect();

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

    Ok(FuelioReport {
        folder: folder.display().to_string(),
        folder_exists: folder.is_dir(),
        drive_count: drives.len(),
        rows: fuelio::crosscheck(&refs, &drives),
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
        // The ID becomes a file name: digits only, so no path can escape the folder.
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("Invalid drive id: {id}"));
        }
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

#[cfg(test)]
#[path = "fuelio_cmd_tests.rs"]
mod tests;
