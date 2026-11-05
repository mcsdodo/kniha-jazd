//! The phone pages under /m (Task 93): the places near a GPS point, and the
//! route estimate of a manual trip. Neither needs Fuelio.

use chrono::{Duration, NaiveDateTime};
use serde::Serialize;

use crate::commands_internal::fuelio_cmd::{places_by_distance, preselect_id, PlaceOption};
use crate::commands_internal::helpers::parse_iso_datetime;
use crate::db::Database;
use crate::route_map::RouteProvider;

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

/// What the router gives for a manual trip. The numbers are `None` when the
/// route fails; `error` then says why, and the page lets the user type the
/// start time.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualTripEstimate {
    /// Route km, 1 decimal. A hint only: the user types the trip km.
    pub route_km: Option<f64>,
    pub duration_min: Option<i64>,
    /// "YYYY-MM-DDTHH:MM".
    pub start_datetime: Option<String>,
    pub error: Option<String>,
}

impl ManualTripEstimate {
    fn failed(error: String) -> Self {
        Self { route_km: None, duration_min: None, start_datetime: None, error: Some(error) }
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
    let route = match provider.fetch(&points).await {
        Ok(route) => route,
        Err(e) => return Ok(ManualTripEstimate::failed(e)),
    };
    let (minutes, start) = start_before(end, route.duration_s);
    Ok(ManualTripEstimate {
        route_km: Some((route.road_km * 10.0).round() / 10.0),
        duration_min: Some(minutes),
        start_datetime: Some(start),
        error: None,
    })
}

#[cfg(test)]
#[path = "mobile_tests.rs"]
mod tests;
