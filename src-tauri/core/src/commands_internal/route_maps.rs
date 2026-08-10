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

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::models::{RouteMap, Waypoint};
use crate::route_map::polyline::decode;
use crate::route_map::{generate_route_random, Dataset, RouteProvider};

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
    pub dataset_version: String,
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
    pub dataset_version: Option<String>,
    pub created_at: String,
}

impl From<RouteMap> for SavedRouteMap {
    fn from(map: RouteMap) -> Self {
        Self {
            trip_id: map.trip_id.to_string(),
            waypoints: map.waypoints,
            coordinates: decode_coordinates(&map.polyline),
            polyline: map.polyline,
            target_km: map.target_km,
            road_km: map.road_km,
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

    Ok(GeneratedRoute {
        coordinates: decode_coordinates(&fetched.polyline),
        polyline: fetched.polyline,
        waypoints,
        target_km,
        road_km: fetched.road_km,
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

#[cfg(test)]
#[path = "route_maps_tests.rs"]
mod tests;
