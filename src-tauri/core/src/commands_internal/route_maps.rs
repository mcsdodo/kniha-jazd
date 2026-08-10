//! Generated route map commands (framework-free).
//!
//! Generating and saving are deliberately separate: `generate_route_internal`
//! only proposes a route (it writes nothing), and the caller confirms it with
//! `save_trip_route_internal`. That is what lets the user regenerate until a
//! route looks right without leaving discarded maps behind.
//!
//! Both read paths return `coordinates` — the polyline decoded into `[lat, lon]`
//! pairs — so the frontend can draw a saved map without shipping its own
//! polyline decoder (ADR-008: logic in Rust, display in the frontend).

use std::path::{Path, PathBuf};

use base64::Engine as _;
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::export::RouteMapPage;
use crate::models::{RouteMap, TripGridData, Waypoint};
use crate::route_map::polyline::decode;
use crate::route_map::render::render_route;
use crate::route_map::tiles::TileFetcher;
use crate::route_map::{generate_route_random, Dataset, RouteProvider, TOLERANCE};

/// A freshly generated route. Not persisted — see the module docs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedRoute {
    pub waypoints: Vec<Waypoint>,
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for L.polyline.
    pub coordinates: Vec<[f64; 2]>,
    pub target_km: f64,
    pub road_km: f64,
    /// Signed percentage by which the road distance misses the target.
    pub deviation_percent: f64,
    /// Whether that deviation exceeds [`TOLERANCE`].
    pub off_target: bool,
    pub dataset_version: String,
}

/// How far the finished route's road distance falls from the target, and
/// whether that is far enough to flag.
///
/// Computed here rather than in the frontend so the threshold has one home
/// (ADR-008). The display cannot invent a second, differently-measured notion
/// of "close enough".
fn deviation(target_km: f64, road_km: f64) -> (f64, bool) {
    if target_km <= 0.0 {
        return (0.0, false);
    }
    let fraction = (road_km - target_km) / target_km;
    (fraction * 100.0, fraction.abs() > TOLERANCE)
}

/// A route map loaded back from the database, ready to draw.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRouteMap {
    pub trip_id: String,
    pub waypoints: Vec<Waypoint>,
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for L.polyline.
    pub coordinates: Vec<[f64; 2]>,
    pub target_km: f64,
    pub road_km: f64,
    /// Signed percentage by which the road distance misses the target.
    pub deviation_percent: f64,
    /// Whether that deviation exceeds [`TOLERANCE`].
    pub off_target: bool,
    pub dataset_version: Option<String>,
    pub created_at: String,
}

impl From<RouteMap> for SavedRouteMap {
    fn from(map: RouteMap) -> Self {
        let (deviation_percent, off_target) = deviation(map.target_km, map.road_km);
        Self {
            trip_id: map.trip_id.to_string(),
            waypoints: map.waypoints,
            coordinates: decode_coordinates(&map.polyline),
            polyline: map.polyline,
            target_km: map.target_km,
            road_km: map.road_km,
            deviation_percent,
            off_target,
            dataset_version: map.dataset_version,
            created_at: map.created_at.to_rfc3339(),
        }
    }
}

/// Polyline5 -> `[lat, lon]` pairs. `decode` never panics; malformed input
/// simply yields the prefix that parsed cleanly.
fn decode_coordinates(polyline: &str) -> Vec<[f64; 2]> {
    decode(polyline)
        .into_iter()
        .map(|(lat, lon)| [lat, lon])
        .collect()
}

/// Turn the genetic algorithm's node indices into waypoints carrying the
/// dataset's name and index.
fn waypoints_for(sequence: &[usize], ds: &Dataset) -> Result<Vec<Waypoint>, String> {
    sequence
        .iter()
        .map(|&idx| {
            let node = ds
                .nodes
                .get(idx)
                .ok_or_else(|| format!("Route referenced unknown dataset node {idx}"))?;
            let node_idx = i32::try_from(node.idx)
                .map_err(|_| format!("Dataset node index {} is out of range", node.idx))?;
            Ok(Waypoint {
                lat: node.lat,
                lon: node.lon,
                name: Some(node.name.clone()),
                node_idx: Some(node_idx),
            })
        })
        .collect()
}

/// Propose a round trip of roughly `target_km`, with road-following geometry
/// from `provider`. Persists NOTHING — the caller confirms with
/// `save_trip_route_internal`.
pub async fn generate_route_internal(
    provider: &dyn RouteProvider,
    target_km: f64,
) -> Result<GeneratedRoute, String> {
    let ds = Dataset::bundled();
    let result = generate_route_random(target_km, &ds);
    let waypoints = waypoints_for(&result.sequence, &ds)?;

    let coords: Vec<(f64, f64)> = waypoints.iter().map(|w| (w.lat, w.lon)).collect();
    let fetched = provider.fetch(&coords).await?;

    let (deviation_percent, off_target) = deviation(target_km, fetched.road_km);

    Ok(GeneratedRoute {
        coordinates: decode_coordinates(&fetched.polyline),
        polyline: fetched.polyline,
        waypoints,
        target_km,
        road_km: fetched.road_km,
        deviation_percent,
        off_target,
        dataset_version: ds.version,
    })
}

pub fn get_trip_route_internal(
    db: &Database,
    trip_id: String,
) -> Result<Option<SavedRouteMap>, String> {
    let map = db.get_route_map(&trip_id).map_err(|e| e.to_string())?;
    Ok(map.map(SavedRouteMap::from))
}

/// Save (or replace) the map for a trip.
///
/// `dataset_version` and `created_at` are stamped here rather than accepted
/// from the caller: they describe what the backend actually used and when it
/// stored it, so a client cannot misreport either.
pub fn save_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    waypoints: Vec<Waypoint>,
    polyline: String,
    target_km: f64,
    road_km: f64,
) -> Result<(), String> {
    check_read_only!(app_state);
    let trip_uuid = Uuid::parse_str(&trip_id).map_err(|e| format!("Invalid trip id: {e}"))?;

    let map = RouteMap {
        trip_id: trip_uuid,
        waypoints,
        polyline,
        target_km,
        road_km,
        dataset_version: Some(Dataset::bundled().version),
        created_at: Utc::now(),
    };

    db.save_route_map(&map).map_err(|e| e.to_string())
}

/// Deleting a map a trip never had is a no-op, not an error.
pub fn delete_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_route_map(&trip_id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Export attachments
// ---------------------------------------------------------------------------

/// Attachment canvas, in pixels. Sized for the A4-landscape attachment page the
/// export lays out (`max-height: 170mm`) at roughly 150 dpi — enough that a
/// printed map stays readable, small enough that a year of them still base64s
/// into one HTML file.
const MAP_WIDTH: u32 = 1400;
const MAP_HEIGHT: u32 = 900;

/// The printed table's rows, in printed order, as `(record number, trip id)`.
///
/// This mirrors [`crate::export::generate_html`]'s own row assembly, and is the
/// single place either export path may get record numbers from:
///
/// * The number is `trip_numbers[trip_id]` — literally the value printed in the
///   table's first column — not a position in any list. Positions differ
///   between the two export modes (desktop prepends a synthetic first record,
///   and month-end rows are interleaved); the printed number does not.
/// * The order follows `sort_direction`, so attachments are numbered in the
///   order a reader meets their rows.
///
/// Month-end rows are absent because they are not trips and can hold no map.
/// The synthetic "Prvý záznam" row (`Uuid::nil()`) is skipped because it prints
/// an empty record number — an attachment citing "záznam č. 0" would point at
/// a row that carries no number at all.
pub fn assemble_export_rows(grid_data: &TripGridData, sort_direction: &str) -> Vec<(usize, String)> {
    let mut rows: Vec<(usize, String)> = grid_data
        .trips
        .iter()
        .filter(|trip| trip.id != Uuid::nil())
        .map(|trip| {
            let trip_id = trip.id.to_string();
            let number = grid_data
                .trip_numbers
                .get(&trip_id)
                .copied()
                .unwrap_or_default();
            (usize::try_from(number).unwrap_or_default(), trip_id)
        })
        .collect();

    // Same rule as the export: "desc" is newest first, anything else ascending.
    // `sort_by` is stable, so rows sharing a number keep the grid's order in
    // both directions, exactly as the table does.
    let descending = sort_direction.eq_ignore_ascii_case("desc");
    rows.sort_by(|a, b| {
        let ordering = a.0.cmp(&b.0);
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });

    rows
}

/// Where rendered exports cache OSM tiles, given the application's data dir.
///
/// A subdirectory rather than the data dir itself: the cache is disposable and
/// is deliberately neither backed up nor moved with the database, so it must be
/// separable from everything that is.
pub fn tile_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cache")
}

/// Build attachment pages from ALREADY-ASSEMBLED rows.
///
/// `rows` must be the same ordered `(record number, trip id)` list the printed
/// table is numbered from — see [`assemble_export_rows`], and never recompute
/// it. Desktop injects a synthetic row that server mode does not, so an
/// independently derived record number makes the two modes cite different rows
/// for the same map, and the printed evidence then points at the wrong journey.
///
/// Attachment numbers run 1, 2, 3 … over the pages actually produced, so a row
/// whose map cannot be drawn closes the gap rather than leaving a hole.
///
/// Nothing here can fail the export: a database error yields no attachments,
/// and a route that cannot be decoded or rendered costs only its own page.
pub async fn collect_route_map_pages(
    db: &Database,
    tiles: &dyn TileFetcher,
    rows: &[(usize, String)],
) -> Vec<RouteMapPage> {
    let trip_ids: Vec<String> = rows.iter().map(|(_, trip_id)| trip_id.clone()).collect();

    // One batched query for the whole export — a lookup per row would be a
    // year's worth of queries for a document that is generated in one go.
    let maps = match db.get_route_maps_for_trips(&trip_ids) {
        Ok(maps) => maps,
        Err(e) => {
            log::warn!("Could not load route maps for the export, attaching none: {e}");
            return Vec::new();
        }
    };

    let mut pages: Vec<RouteMapPage> = Vec::new();
    for (row_number, trip_id) in rows {
        let Some(map) = maps.get(trip_id) else {
            continue;
        };

        // `decode` never panics; malformed geometry simply yields no points,
        // which `render_route` reports as an error rather than drawing blank.
        let points = decode(&map.polyline);
        match render_route(tiles, &points, MAP_WIDTH, MAP_HEIGHT).await {
            Ok(png) => pages.push(RouteMapPage {
                attachment_no: pages.len() + 1,
                row_number: *row_number,
                png_base64: base64::engine::general_purpose::STANDARD.encode(png),
            }),
            Err(e) => log::warn!(
                "Skipping the route map attachment for record {row_number} (trip {trip_id}): {e}"
            ),
        }
    }

    pages
}

#[cfg(test)]
#[path = "route_maps_tests.rs"]
mod tests;
