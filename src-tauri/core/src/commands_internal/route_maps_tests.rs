//! Tests for generated route map commands.
//!
//! Nothing here touches the network: [`StubProvider`] stands in for OSRM.

use super::*;
use crate::app_state::AppState;
use crate::db::Database;
use crate::models::{Trip, Vehicle, Waypoint};
use crate::route_map::polyline::encode;
use crate::route_map::{Dataset, FetchedRoute, RouteProvider};
use chrono::NaiveDate;

/// Geometry provider that never leaves the process.
struct StubProvider {
    polyline: String,
    road_km: f64,
}

impl StubProvider {
    /// A stub whose polyline really is a valid encoding of `points`, so tests
    /// that assert on decoded coordinates are asserting on real codec output.
    fn encoding(points: &[(f64, f64)], road_km: f64) -> Self {
        Self {
            polyline: encode(points),
            road_km,
        }
    }
}

#[async_trait::async_trait]
impl RouteProvider for StubProvider {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        assert!(
            coords.len() >= 2,
            "a route needs at least a start and an end, got {}",
            coords.len()
        );
        Ok(FetchedRoute {
            polyline: self.polyline.clone(),
            road_km: self.road_km,
        })
    }
}

fn seed_trip(db: &Database) -> Trip {
    let vehicle = Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
    db.create_vehicle(&vehicle).unwrap();
    let mut trip = Trip::test_ice_trip(
        NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        120.0,
        None,
        true,
    );
    trip.vehicle_id = vehicle.id;
    db.create_trip(&trip).unwrap();
    trip
}

fn sample_waypoints() -> Vec<Waypoint> {
    vec![
        Waypoint {
            lat: 48.935,
            lon: 20.553,
            name: Some("Domov".into()),
            node_idx: Some(0),
        },
        Waypoint {
            lat: 48.997,
            lon: 20.591,
            name: Some("Niekde".into()),
            node_idx: Some(14),
        },
    ]
}

/// The polyline of a two-point line, plus the points it encodes.
fn sample_geometry() -> (Vec<(f64, f64)>, String) {
    let points = vec![(48.935_00, 20.553_00), (48.997_00, 20.591_00)];
    let encoded = encode(&points);
    (points, encoded)
}

#[test]
fn save_rejects_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    let app_state = AppState::new();
    app_state.enable_read_only("Test read-only");

    let (_, encoded) = sample_geometry();
    let err = save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        encoded,
        120.0,
        118.4,
    )
    .unwrap_err();
    assert!(err.contains("len na čítanie"), "got: {err}");

    // A guard that errors *after* writing would still produce that message.
    assert!(
        get_trip_route_internal(&db, trip.id.to_string())
            .unwrap()
            .is_none(),
        "read-only save must not have written anything"
    );
}

#[test]
fn delete_rejects_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    let writable = AppState::new();
    let (_, encoded) = sample_geometry();
    save_trip_route_internal(
        &db,
        &writable,
        trip.id.to_string(),
        sample_waypoints(),
        encoded,
        120.0,
        118.4,
    )
    .unwrap();

    let read_only = AppState::new();
    read_only.enable_read_only("Test read-only");
    let err = delete_trip_route_internal(&db, &read_only, trip.id.to_string()).unwrap_err();
    assert!(err.contains("len na čítanie"), "got: {err}");

    // The refusal has to be a refusal, not a delete plus an error.
    assert!(
        get_trip_route_internal(&db, trip.id.to_string())
            .unwrap()
            .is_some(),
        "read-only delete must have left the saved map in place"
    );
}

#[test]
fn get_returns_none_for_a_trip_without_a_map() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    assert!(get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .is_none());
}

#[test]
fn delete_is_idempotent() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    let app_state = AppState::new();

    delete_trip_route_internal(&db, &app_state, trip.id.to_string())
        .expect("deleting a map that was never generated must be Ok");
    delete_trip_route_internal(&db, &app_state, trip.id.to_string())
        .expect("a second delete must also be Ok");
}

/// ADR-008: the frontend draws what the backend decoded. Returning only the
/// encoded polyline would force a JavaScript decoder into the UI.
#[test]
fn saved_route_is_returned_with_decoded_coordinates() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    let app_state = AppState::new();
    let (points, encoded) = sample_geometry();

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        encoded.clone(),
        120.0,
        118.4,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .expect("the saved map must load back");

    assert_eq!(loaded.trip_id, trip.id.to_string());
    assert_eq!(loaded.polyline, encoded);
    assert_eq!(loaded.target_km, 120.0);
    assert_eq!(loaded.road_km, 118.4);
    assert_eq!(loaded.waypoints, sample_waypoints());
    assert!(
        !loaded.coordinates.is_empty(),
        "a saved map must come back ready to draw"
    );
    assert_eq!(loaded.coordinates.len(), points.len());
    let (lat, lon) = points[0];
    assert!(
        (loaded.coordinates[0][0] - lat).abs() < 1e-5
            && (loaded.coordinates[0][1] - lon).abs() < 1e-5,
        "first coordinate must be the polyline's first decoded point, got {:?}",
        loaded.coordinates[0]
    );

    // Stamped by the backend, never by the caller.
    assert_eq!(
        loaded.dataset_version,
        Some(Dataset::bundled().version),
        "the bundled dataset version must be stamped on save"
    );
    assert!(!loaded.created_at.is_empty());
}

#[tokio::test]
async fn generate_route_produces_a_home_loop_with_geometry() {
    let (points, _) = sample_geometry();
    let provider = StubProvider::encoding(&points, 117.2);

    let route = generate_route_internal(&provider, 120.0).await.unwrap();

    let ds = Dataset::bundled();
    let home = &ds.nodes[0];
    assert!(
        route.waypoints.len() >= 3,
        "a loop is home -> at least one stop -> home, got {}",
        route.waypoints.len()
    );
    let first = route.waypoints.first().unwrap();
    let last = route.waypoints.last().unwrap();
    assert_eq!(first.node_idx, Some(0), "a route must start at home");
    assert_eq!(last.node_idx, Some(0), "a route must end at home");
    assert_eq!(first.lat, home.lat);
    assert_eq!(first.lon, home.lon);
    assert_eq!(first.name, Some(home.name.clone()));

    assert!(!route.coordinates.is_empty(), "geometry must be decoded");
    assert_eq!(route.polyline, encode(&points));
    assert_eq!(route.target_km, 120.0);
    assert_eq!(route.road_km, 117.2);
    assert!(
        !route.dataset_version.is_empty(),
        "the dataset version must be reported"
    );
}

/// Generating is a preview; only an explicit save writes anything.
#[tokio::test]
async fn generate_route_persists_nothing() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip(&db);
    let (points, _) = sample_geometry();
    let provider = StubProvider::encoding(&points, 117.2);

    generate_route_internal(&provider, 120.0).await.unwrap();

    assert!(
        get_trip_route_internal(&db, trip.id.to_string())
            .unwrap()
            .is_none(),
        "generate must persist nothing — the caller confirms with save_trip_route"
    );
}
