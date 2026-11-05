//! The phone pages under /m (Task 93): the places near a GPS point, the
//! route estimate of a manual trip, and its origin default. None needs Fuelio.

use chrono::{Duration, NaiveDateTime};
use serde::Serialize;

use crate::commands_internal::fuelio_cmd::{places_by_distance, preselect_id, PlaceOption};
use crate::commands_internal::helpers::parse_iso_datetime;
use crate::db::Database;
use crate::app_state::AppState;
use crate::check_read_only;
use crate::commands_internal::route_maps::{build_route_map, decode_coordinates, MAX_ALTERNATIVES};
use crate::models::{RouteMode, Waypoint};
use crate::route_map::{RouteProvider, RouteProviderKind};

/// The places near a phone GPS point, nearest first.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NearbyPlaces {
    pub places: Vec<PlaceOption>,
    /// The nearest place within `PLACE_PRESELECT_RADIUS_M`, or `None`.
    pub preselect_id: Option<String>,
}

pub fn get_places_near_internal(db: &Database, lat: f64, lon: f64) -> Result<NearbyPlaces, String> {
    let places = places_by_distance(db, (lat, lon))?;
    Ok(NearbyPlaces { preselect_id: preselect_id(&places), places })
}

/// The prefilled values of a manual trip on the phone.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualTripDefaults {
    /// The destination of the latest trip of the vehicle that starts before
    /// the end: a trip usually starts where the one before it ended. `None`
    /// without such a trip.
    pub origin_place_id: Option<String>,
}

/// The defaults for a manual trip of `vehicle_id` that ends at `end_datetime`
/// ("YYYY-MM-DDTHH:MM", seconds optional). A trip that starts at or after
/// the end does not count. On the same start, the higher odometer is the
/// later trip (the chain order of the book).
pub fn get_manual_trip_defaults_internal(
    db: &Database,
    vehicle_id: &str,
    end_datetime: &str,
) -> Result<ManualTripDefaults, String> {
    let end = parse_iso_datetime(end_datetime)?;
    let trips = db.get_trips_for_vehicle(vehicle_id).map_err(|e| e.to_string())?;
    let latest = trips
        .iter()
        .filter(|t| t.start_datetime < end)
        .max_by(|a, b| {
            a.start_datetime
                .cmp(&b.start_datetime)
                .then(a.odometer.total_cmp(&b.odometer))
        });
    Ok(ManualTripDefaults { origin_place_id: latest.map(|t| t.destination_place_id.to_string()) })
}

/// What the router gives for a manual trip. The numbers are `None` when the
/// route fails; `error` then says why, and the page lets the user type the
/// start time.
///
/// Task 94: `routes` holds every alternative, in the order the router sent
/// them (fastest first). The top-level numbers copy `routes[0]`, so a page
/// that predates the alternatives still works.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualTripEstimate {
    /// Route km, 1 decimal.
    pub route_km: Option<f64>,
    /// The route km in whole km, as a routed trip gets it (`logbook_km`).
    /// The page fills the trip km with it; the user can change it.
    pub distance_km: Option<f64>,
    pub duration_min: Option<i64>,
    /// "YYYY-MM-DDTHH:MM".
    pub start_datetime: Option<String>,
    pub error: Option<String>,
    /// The service that routed. `None` when no route was asked for.
    pub provider: Option<RouteProviderKind>,
    pub routes: Vec<ManualRoute>,
}

/// One route alternative of a manual trip, with its own trip km and start.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualRoute {
    /// Route km, 1 decimal.
    pub route_km: f64,
    /// The route km in whole km (`logbook_km`).
    pub distance_km: f64,
    pub duration_min: i64,
    /// "YYYY-MM-DDTHH:MM": the end minus the drive time.
    pub start_datetime: String,
    /// The exact route km. The page sends it back to save the route.
    pub road_km: f64,
    /// Encoded polyline5. The page sends it back to save the route.
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for L.polyline.
    pub coordinates: Vec<[f64; 2]>,
}

impl ManualTripEstimate {
    fn failed(error: String) -> Self {
        Self {
            route_km: None,
            distance_km: None,
            duration_min: None,
            start_datetime: None,
            error: Some(error),
            provider: None,
            routes: Vec::new(),
        }
    }
}

/// The drive time in whole minutes, and the start of a trip that ends at
/// `end` ("YYYY-MM-DDTHH:MM").
pub fn start_before(end: NaiveDateTime, duration_s: f64) -> (i64, String) {
    let minutes = (duration_s / 60.0).round() as i64;
    let start = end - Duration::minutes(minutes);
    (minutes, start.format("%Y-%m-%dT%H:%M").to_string())
}

/// One end of the estimate: a place, or a GPS point. The phone routes to a
/// point when the destination is a new place that does not exist yet.
#[derive(Debug, Clone, PartialEq)]
pub enum EstimateEnd {
    Place(String),
    Point(f64, f64),
}

impl EstimateEnd {
    /// A place id, or both `lat` and `lon`. A blank id counts as not set.
    pub fn from_args(
        place_id: Option<String>,
        lat: Option<f64>,
        lon: Option<f64>,
    ) -> Result<Self, String> {
        match (place_id.filter(|id| !id.trim().is_empty()), lat, lon) {
            (Some(id), None, None) => Ok(Self::Place(id)),
            (None, Some(lat), Some(lon)) => Ok(Self::Point(lat, lon)),
            (Some(_), _, _) => Err("Give a place id or a point, not both".into()),
            (None, _, _) => Err("Give a place id, or both lat and lon".into()),
        }
    }
}

/// Route from the origin to the destination, and the start time of a trip
/// that ends at `end_datetime`. A bad end or an unknown place is an `Err`; a
/// place without coordinates or a failed route is not.
pub async fn get_manual_trip_estimate_internal(
    db: &Database,
    provider: &dyn RouteProvider,
    origin: &EstimateEnd,
    destination: &EstimateEnd,
    end_datetime: &str,
) -> Result<ManualTripEstimate, String> {
    let end = parse_iso_datetime(end_datetime)?;
    // Load both places first: an unknown id is an Err even when the other
    // end has no coordinates.
    let mut ends = Vec::new();
    for e in [origin, destination] {
        ends.push(match e {
            EstimateEnd::Point(lat, lon) => Ok((*lat, *lon)),
            EstimateEnd::Place(id) => {
                let place = db
                    .get_place(id)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Miesto neexistuje: {id}"))?;
                match (place.lat, place.lon) {
                    (Some(lat), Some(lon)) => Ok((lat, lon)),
                    _ => Err(format!("Miesto nemá polohu: {}", place.name)),
                }
            }
        });
    }
    let points = match ends.into_iter().collect::<Result<Vec<_>, String>>() {
        Ok(points) => points,
        Err(e) => return Ok(ManualTripEstimate::failed(e)),
    };
    let fetched = match provider.fetch_alternatives(&points, MAX_ALTERNATIVES).await {
        Ok(fetched) => fetched,
        Err(e) => return Ok(ManualTripEstimate::failed(e)),
    };
    let routes: Vec<ManualRoute> = fetched
        .into_iter()
        .map(|route| {
            let (minutes, start) = start_before(end, route.duration_s);
            ManualRoute {
                route_km: (route.road_km * 10.0).round() / 10.0,
                distance_km: crate::commands_internal::trips::logbook_km(route.road_km),
                duration_min: minutes,
                start_datetime: start,
                road_km: route.road_km,
                coordinates: decode_coordinates(&route.polyline),
                polyline: route.polyline,
            }
        })
        .collect();
    let Some(first) = routes.first().cloned() else {
        return Ok(ManualTripEstimate::failed("The router returned no route".into()));
    };
    Ok(ManualTripEstimate {
        route_km: Some(first.route_km),
        distance_km: Some(first.distance_km),
        duration_min: Some(first.duration_min),
        start_datetime: Some(first.start_datetime),
        error: None,
        provider: Some(provider.kind()),
        routes,
    })
}

/// Task 94: store the route the user chose on the phone as the route of a
/// trip that was just added by hand.
///
/// The waypoints come from the trip's own places, not from the browser. The
/// trip km does not change: `target_km` is the trip km, so a typed km shows
/// as a deviation on the map, as on `/mapa`. A second write after the trip
/// commit, as with the GPS route of a Fuelio trip: if it fails, the trip
/// stays.
pub fn save_manual_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: &str,
    polyline: String,
    road_km: f64,
    provider: Option<RouteProviderKind>,
) -> Result<(), String> {
    check_read_only!(app_state);
    let trip = db
        .get_trip(trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {trip_id}"))?;
    let point = |id: &uuid::Uuid, name: &str| -> Result<Waypoint, String> {
        let place = db
            .get_place(&id.to_string())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Miesto neexistuje: {id}"))?;
        match (place.lat, place.lon) {
            (Some(lat), Some(lon)) => {
                Ok(Waypoint { lat, lon, name: Some(name.to_string()), node_idx: None })
            }
            _ => Err(format!("Miesto nemá polohu: {}", place.name)),
        }
    };
    let waypoints = vec![
        point(&trip.origin_place_id, &trip.origin)?,
        point(&trip.destination_place_id, &trip.destination)?,
    ];
    let mut map = build_route_map(
        trip_id,
        waypoints,
        polyline,
        road_km,
        RouteMode::Direct,
        false,
        None,
        Vec::new(),
        provider,
    )?;
    map.target_km = trip.distance_km;
    db.save_route_map(&map).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "mobile_tests.rs"]
mod tests;
