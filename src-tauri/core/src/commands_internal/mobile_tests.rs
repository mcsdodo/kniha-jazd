//! Tests for the phone page commands (Task 93).
//!
//! Nothing here touches the network: [`Stub`] stands in for the router.

use super::*;
use crate::app_state::AppState;
use crate::models::PlaceSource;
use crate::route_map::{FetchedRoute, RouteProvider};
use chrono::NaiveDateTime;
use std::sync::Mutex;

/// A placed place through the normal create path; returns its id.
fn place(db: &Database, name: &str, at: (f64, f64)) -> String {
    crate::commands_internal::create_place_internal(
        db,
        &AppState::new(),
        name.into(),
        at.0,
        at.1,
        PlaceSource::Manual,
    )
    .unwrap()
    .id
    .to_string()
}

/// A place without lat/lon; returns its id. (`ensure_place_for_test` gives
/// its place a fixed position.)
fn unplaced(db: &Database, name: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    db.insert_place(&crate::models::NewPlaceRow {
        id: &id,
        name,
        normalised_name: &crate::places::normalise(name),
        lat: None,
        lon: None,
        source: None,
        created_at: "2026-11-05T00:00:00Z",
    })
    .unwrap();
    id
}

/// `(lat, lon)` about `m` metres north of `from`, checked with the same
/// distance fn the preselect uses.
fn north_of(from: (f64, f64), m: f64) -> (f64, f64) {
    let to = (from.0 + m / 111_194.93, from.1);
    let d = crate::fuelio::geo::haversine_m(from, to);
    assert!((d - m).abs() < 0.5, "fixture is {d} m, not {m} m");
    to
}

const HERE: (f64, f64) = (48.1486, 17.1077);

// ---------------------------------------------------------------------------
// get_places_near
// ---------------------------------------------------------------------------

#[test]
fn places_near_lists_nearest_first_and_preselects_within_300_m() {
    let db = Database::in_memory().unwrap();
    let far = place(&db, "Far", north_of(HERE, 301.0));
    let near = place(&db, "Near", north_of(HERE, 299.0));
    let r = get_places_near_internal(&db, HERE.0, HERE.1).unwrap();
    let ids: Vec<&str> = r.places.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec![near.as_str(), far.as_str()]);
    assert_eq!(r.preselect_id, Some(near));
}

#[test]
fn places_near_preselects_nothing_beyond_300_m() {
    let db = Database::in_memory().unwrap();
    place(&db, "Far", north_of(HERE, 301.0));
    let r = get_places_near_internal(&db, HERE.0, HERE.1).unwrap();
    assert_eq!(r.places.len(), 1);
    assert_eq!(r.preselect_id, None);
}

#[test]
fn places_near_without_places_is_empty() {
    let db = Database::in_memory().unwrap();
    unplaced(&db, "No position"); // not offered: it has no lat/lon
    let r = get_places_near_internal(&db, HERE.0, HERE.1).unwrap();
    assert!(r.places.is_empty());
    assert_eq!(r.preselect_id, None);
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json, serde_json::json!({ "places": [], "preselectId": null }));
}

#[test]
fn places_near_json_is_camel_case() {
    let db = Database::in_memory().unwrap();
    let id = place(&db, "Here", HERE);
    let json = serde_json::to_value(get_places_near_internal(&db, HERE.0, HERE.1).unwrap()).unwrap();
    assert_eq!(json["preselectId"], serde_json::json!(id));
    assert_eq!(json["places"][0]["name"], "Here");
    assert!(json["places"][0]["distanceM"].as_f64().unwrap() < 1.0);
}

// ---------------------------------------------------------------------------
// Start time of a manual trip (pure)
// ---------------------------------------------------------------------------

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
}

#[test]
fn start_is_the_end_minus_the_drive_in_whole_minutes() {
    assert_eq!(start_before(dt("2026-11-05T08:30"), 3600.0), (60, "2026-11-05T07:30".into()));
    // 25.5 min rounds to 26.
    assert_eq!(start_before(dt("2026-11-05T08:30"), 1530.0), (26, "2026-11-05T08:04".into()));
    // 29 s rounds to 0 min: start = end.
    assert_eq!(start_before(dt("2026-11-05T08:30"), 29.0), (0, "2026-11-05T08:30".into()));
}

#[test]
fn start_crosses_midnight_and_new_year() {
    assert_eq!(start_before(dt("2026-11-06T00:20"), 2700.0), (45, "2026-11-05T23:35".into()));
    assert_eq!(start_before(dt("2027-01-01T00:10"), 1200.0), (20, "2026-12-31T23:50".into()));
}

// ---------------------------------------------------------------------------
// get_manual_trip_estimate
// ---------------------------------------------------------------------------

/// A router that answers with `result` and records the points it got.
struct Stub {
    result: Result<FetchedRoute, String>,
    asked: Mutex<Vec<Vec<(f64, f64)>>>,
}

impl Stub {
    fn new(result: Result<FetchedRoute, String>) -> Self {
        Self { result, asked: Mutex::new(Vec::new()) }
    }

    fn route(road_km: f64, duration_s: f64) -> Self {
        Self::new(Ok(FetchedRoute { road_km, duration_s, ..Default::default() }))
    }
}

#[async_trait::async_trait]
impl RouteProvider for Stub {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        self.asked.lock().unwrap().push(coords.to_vec());
        self.result.clone()
    }

    async fn table(&self, _coords: &[(f64, f64)]) -> Result<Vec<Vec<f64>>, String> {
        unreachable!("the estimate needs no matrix")
    }
}

const THERE: (f64, f64) = (48.7164, 21.2611);

fn pl(id: &str) -> EstimateEnd {
    EstimateEnd::Place(id.to_string())
}

#[test]
fn an_estimate_end_is_a_place_id_or_a_whole_point() {
    let s = |v: &str| Some(v.to_string());
    assert_eq!(EstimateEnd::from_args(s("id-1"), None, None), Ok(pl("id-1")));
    assert_eq!(EstimateEnd::from_args(None, Some(48.1), Some(17.1)), Ok(EstimateEnd::Point(48.1, 17.1)));
    // A blank id counts as not set.
    assert_eq!(EstimateEnd::from_args(s(" "), Some(48.1), Some(17.1)), Ok(EstimateEnd::Point(48.1, 17.1)));
    assert!(EstimateEnd::from_args(None, None, None).is_err());
    assert!(EstimateEnd::from_args(s(""), None, None).is_err());
    assert!(EstimateEnd::from_args(None, Some(48.1), None).is_err());
    assert!(EstimateEnd::from_args(None, None, Some(17.1)).is_err());
    assert!(EstimateEnd::from_args(s("id-1"), Some(48.1), Some(17.1)).is_err());
    assert!(EstimateEnd::from_args(s("id-1"), Some(48.1), None).is_err());
}

// The main phone case: the destination is a new place at the GPS point, so
// it has no id yet.
#[tokio::test]
async fn estimate_routes_from_a_place_to_a_point() {
    let db = Database::in_memory().unwrap();
    let a = place(&db, "A", HERE);
    let stub = Stub::route(12.0, 900.0);
    let r = get_manual_trip_estimate_internal(
        &db,
        &stub,
        &pl(&a),
        &EstimateEnd::Point(THERE.0, THERE.1),
        "2026-11-05T12:00",
    )
    .await
    .unwrap();
    assert_eq!((r.route_km, r.duration_min), (Some(12.0), Some(15)));
    assert_eq!(r.start_datetime.as_deref(), Some("2026-11-05T11:45"));
    assert_eq!(*stub.asked.lock().unwrap(), vec![vec![HERE, THERE]]);
}

#[tokio::test]
async fn estimate_with_an_unknown_place_is_an_err_even_if_the_other_has_no_position() {
    let db = Database::in_memory().unwrap();
    let no_position = unplaced(&db, "Bez polohy");
    let stub = Stub::route(1.0, 60.0);
    let err = get_manual_trip_estimate_internal(&db, &stub, &pl(&no_position), &pl("no-such-id"), "2026-11-05T12:00")
        .await
        .unwrap_err();
    assert!(err.contains("no-such-id"), "{err}");
}

#[tokio::test]
async fn estimate_routes_from_origin_to_destination() {
    let db = Database::in_memory().unwrap();
    let a = place(&db, "A", HERE);
    let b = place(&db, "B", THERE);
    let stub = Stub::route(398.36, 14_130.0); // 235.5 min
    let r = get_manual_trip_estimate_internal(&db, &stub, &pl(&a), &pl(&b), "2026-11-05T12:00")
        .await
        .unwrap();
    assert_eq!(r.route_km, Some(398.4));
    // The trip km: the route km rounded like a routed or GPS trip distance.
    assert_eq!(r.distance_km, Some(398.0));
    assert_eq!(r.duration_min, Some(236));
    assert_eq!(r.start_datetime.as_deref(), Some("2026-11-05T08:04"));
    assert_eq!(r.error, None);
    assert_eq!(*stub.asked.lock().unwrap(), vec![vec![HERE, THERE]]);
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "routeKm": 398.4, "distanceKm": 398.0, "durationMin": 236,
            "startDatetime": "2026-11-05T08:04", "error": null,
            "provider": "osrm",
            "routes": [{
                "routeKm": 398.4, "distanceKm": 398.0, "durationMin": 236,
                "startDatetime": "2026-11-05T08:04",
                "roadKm": 398.36, "polyline": "", "coordinates": []
            }]
        })
    );
}

#[tokio::test]
async fn estimate_with_a_failed_route_is_not_an_err() {
    let db = Database::in_memory().unwrap();
    let a = place(&db, "A", HERE);
    let b = place(&db, "B", THERE);
    let stub = Stub::new(Err("OSRM timeout".into()));
    let r = get_manual_trip_estimate_internal(&db, &stub, &pl(&a), &pl(&b), "2026-11-05T12:00")
        .await
        .unwrap();
    assert_eq!((r.route_km, r.duration_min, r.start_datetime), (None, None, None));
    assert_eq!(r.distance_km, None);
    assert!(r.routes.is_empty());
    assert!(r.error.as_deref().is_some_and(|e| e.contains("OSRM timeout")), "{:?}", r.error);
}

#[tokio::test]
async fn estimate_names_the_place_without_coordinates_and_does_not_route() {
    let db = Database::in_memory().unwrap();
    let a = place(&db, "A", HERE);
    let b = unplaced(&db, "Chata bez polohy");
    let stub = Stub::route(1.0, 60.0);
    for (o, d) in [(&a, &b), (&b, &a)] {
        let r = get_manual_trip_estimate_internal(&db, &stub, &pl(o), &pl(d), "2026-11-05T12:00")
            .await
            .unwrap();
        assert_eq!((r.route_km, r.duration_min, r.start_datetime), (None, None, None));
        assert!(r.error.as_deref().is_some_and(|e| e.contains("Chata bez polohy")), "{:?}", r.error);
    }
    assert!(stub.asked.lock().unwrap().is_empty());
}

#[tokio::test]
async fn estimate_rejects_a_bad_end_and_an_unknown_place() {
    let db = Database::in_memory().unwrap();
    let a = place(&db, "A", HERE);
    let b = place(&db, "B", THERE);
    let stub = Stub::route(1.0, 60.0);
    let err = get_manual_trip_estimate_internal(&db, &stub, &pl(&a), &pl(&b), "5.11.2026 12:00")
        .await
        .unwrap_err();
    assert!(err.contains("Invalid datetime"), "{err}");
    let err = get_manual_trip_estimate_internal(&db, &stub, &pl(&a), &pl("no-such-id"), "2026-11-05T12:00")
        .await
        .unwrap_err();
    assert!(err.contains("no-such-id"), "{err}");
    assert!(stub.asked.lock().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// get_manual_trip_defaults
// ---------------------------------------------------------------------------

fn vehicle(db: &Database) -> crate::models::Vehicle {
    let v = crate::models::Vehicle::new_ice("Car".into(), "TEST-1".into(), 50.0, 6.5, 0.0);
    db.create_vehicle(&v).unwrap();
    v
}

/// A trip of `v` that starts at `at` ("YYYY-MM-DD HH:MM") and ends at `to`.
fn trip_to(db: &Database, v: &crate::models::Vehicle, at: &str, odo: f64, to: &str) {
    let mut t = crate::models::Trip::test_ice_trip(
        chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        10.0,
        None,
        false,
    );
    t.vehicle_id = v.id;
    t.start_datetime = NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M").unwrap();
    t.odometer = odo;
    t.origin_place_id = uuid::Uuid::parse_str(&place(db, &format!("From {at} {odo}"), HERE)).unwrap();
    t.destination_place_id = uuid::Uuid::parse_str(to).unwrap();
    db.create_trip(&t).unwrap();
}

#[test]
fn defaults_origin_is_the_destination_of_the_latest_earlier_trip() {
    let db = Database::in_memory().unwrap();
    let v = vehicle(&db);
    let office = place(&db, "Office", north_of(HERE, 1000.0));
    let shop = place(&db, "Shop", north_of(HERE, 2000.0));
    trip_to(&db, &v, "2026-11-05 08:00", 100.0, &office);
    trip_to(&db, &v, "2026-11-05 12:00", 110.0, &shop);
    let r = get_manual_trip_defaults_internal(&db, &v.id.to_string(), "2026-11-05T15:30").unwrap();
    assert_eq!(r.origin_place_id, Some(shop));
}

#[test]
fn defaults_ignore_trips_that_start_at_or_after_the_end() {
    let db = Database::in_memory().unwrap();
    let v = vehicle(&db);
    let office = place(&db, "Office", north_of(HERE, 1000.0));
    let shop = place(&db, "Shop", north_of(HERE, 2000.0));
    let later = place(&db, "Later", north_of(HERE, 3000.0));
    trip_to(&db, &v, "2026-11-05 08:00", 100.0, &office);
    trip_to(&db, &v, "2026-11-05 15:30", 110.0, &shop);
    trip_to(&db, &v, "2026-11-06 09:00", 120.0, &later);
    // Seconds are accepted too.
    let r =
        get_manual_trip_defaults_internal(&db, &v.id.to_string(), "2026-11-05T15:30:00").unwrap();
    assert_eq!(r.origin_place_id, Some(office));
}

#[test]
fn defaults_on_the_same_start_take_the_higher_odometer() {
    let db = Database::in_memory().unwrap();
    let v = vehicle(&db);
    let first = place(&db, "First", north_of(HERE, 1000.0));
    let second = place(&db, "Second", north_of(HERE, 2000.0));
    trip_to(&db, &v, "2026-11-05 08:00", 120.0, &second);
    trip_to(&db, &v, "2026-11-05 08:00", 110.0, &first);
    let r = get_manual_trip_defaults_internal(&db, &v.id.to_string(), "2026-11-05T15:30").unwrap();
    assert_eq!(r.origin_place_id, Some(second));
}

#[test]
fn defaults_ignore_other_vehicles_and_give_none_without_an_earlier_trip() {
    let db = Database::in_memory().unwrap();
    let v = vehicle(&db);
    let other = vehicle(&db);
    let office = place(&db, "Office", north_of(HERE, 1000.0));
    trip_to(&db, &other, "2026-11-05 08:00", 100.0, &office);
    let r = get_manual_trip_defaults_internal(&db, &v.id.to_string(), "2026-11-05T15:30").unwrap();
    assert_eq!(r.origin_place_id, None);
    let unknown =
        get_manual_trip_defaults_internal(&db, "no-such-vehicle", "2026-11-05T15:30").unwrap();
    assert_eq!(unknown.origin_place_id, None);
}

#[test]
fn defaults_reject_a_bad_end() {
    let db = Database::in_memory().unwrap();
    let v = vehicle(&db);
    assert!(get_manual_trip_defaults_internal(&db, &v.id.to_string(), "5.11.2026").is_err());
}

// ---------------------------------------------------------------------------
// Route alternatives on the phone (Task 94)
// ---------------------------------------------------------------------------

/// A router that answers `fetch_alternatives` with fixed routes, in the
/// order given, and says it is `kind`.
struct Alternatives {
    routes: Vec<FetchedRoute>,
    kind: crate::route_map::RouteProviderKind,
    asked_max: Mutex<Vec<usize>>,
}

#[async_trait::async_trait]
impl RouteProvider for Alternatives {
    fn kind(&self) -> crate::route_map::RouteProviderKind {
        self.kind
    }

    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        unreachable!("the estimate asks for alternatives")
    }

    async fn fetch_alternatives(&self, _coords: &[(f64, f64)], max: usize) -> Result<Vec<FetchedRoute>, String> {
        self.asked_max.lock().unwrap().push(max);
        Ok(self.routes.clone())
    }
}

fn fetched(road_km: f64, duration_s: f64, polyline: &str) -> FetchedRoute {
    FetchedRoute { road_km, duration_s, polyline: polyline.into(), ..Default::default() }
}

#[tokio::test]
async fn estimate_lists_every_alternative_in_the_router_order() {
    let db = Database::in_memory().unwrap();
    let a = place(&db, "A", HERE);
    let b = place(&db, "B", THERE);
    let one = crate::route_map::polyline::encode(&[HERE, THERE]);
    let router = Alternatives {
        // The router lists the fastest first; the shorter one is second.
        routes: vec![fetched(23.84, 1860.0, &one), fetched(21.2, 2280.0, "")],
        kind: crate::route_map::RouteProviderKind::Sygic,
        asked_max: Mutex::new(Vec::new()),
    };
    let r = get_manual_trip_estimate_internal(&db, &router, &pl(&a), &pl(&b), "2026-11-05T15:30")
        .await
        .unwrap();
    assert_eq!(r.provider, Some(crate::route_map::RouteProviderKind::Sygic));
    assert_eq!(r.routes.len(), 2);
    let (first, second) = (&r.routes[0], &r.routes[1]);
    assert_eq!((first.route_km, first.distance_km, first.duration_min), (23.8, 24.0, 31));
    assert_eq!(first.start_datetime, "2026-11-05T14:59");
    assert_eq!(first.road_km, 23.84);
    assert_eq!(first.polyline, one);
    assert_eq!(first.coordinates.len(), 2);
    assert_eq!((second.route_km, second.distance_km, second.duration_min), (21.2, 21.0, 38));
    assert_eq!(second.start_datetime, "2026-11-05T14:52");
    // The old fields copy the first route, so an old page still works.
    assert_eq!(r.route_km, Some(23.8));
    assert_eq!(r.distance_km, Some(24.0));
    assert_eq!(r.duration_min, Some(31));
    assert_eq!(r.start_datetime.as_deref(), Some("2026-11-05T14:59"));
    assert_eq!(*router.asked_max.lock().unwrap(), vec![3]);
}

#[tokio::test]
async fn estimate_with_no_route_from_the_router_is_a_failed_route() {
    let db = Database::in_memory().unwrap();
    let a = place(&db, "A", HERE);
    let b = place(&db, "B", THERE);
    let router = Alternatives {
        routes: vec![],
        kind: crate::route_map::RouteProviderKind::Osrm,
        asked_max: Mutex::new(Vec::new()),
    };
    let r = get_manual_trip_estimate_internal(&db, &router, &pl(&a), &pl(&b), "2026-11-05T15:30")
        .await
        .unwrap();
    assert!(r.routes.is_empty());
    assert_eq!(r.route_km, None);
    assert!(r.error.is_some());
}

/// A trip from `from` to `to` with `km`, through the normal create path.
fn manual_trip(db: &Database, v: &crate::models::Vehicle, from: &str, to: &str, km: f64) -> String {
    crate::commands_internal::create_trip_cascade_internal(
        db,
        &AppState::new(),
        v.id.to_string(),
        "2026-11-05T14:59".into(),
        "2026-11-05T15:30".into(),
        from.into(),
        to.into(),
        km,
        "služobná cesta".into(),
        None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap()
    .trip
    .unwrap()
    .id
    .to_string()
}

#[test]
fn saved_route_uses_the_trip_places_and_keeps_the_trip_km() {
    let db = Database::in_memory().unwrap();
    let v = vehicle(&db);
    let a = place(&db, "Domov", HERE);
    let b = place(&db, "Kancelária", THERE);
    // The user typed 25 km over a 23.84 km route.
    let trip_id = manual_trip(&db, &v, &a, &b, 25.0);
    save_manual_trip_route_internal(
        &db,
        &AppState::new(),
        &trip_id,
        "poly".into(),
        23.84,
        Some(crate::route_map::RouteProviderKind::Sygic),
    )
    .unwrap();

    let map = db.get_route_map(&trip_id).unwrap().expect("a route row");
    assert_eq!(map.polyline, "poly");
    assert_eq!(map.road_km, 23.84);
    // The target is what the trip measures, so the map shows the deviation.
    assert_eq!(map.target_km, 25.0);
    assert_eq!(map.mode, crate::models::RouteMode::Direct);
    assert!(!map.round_trip);
    assert_eq!(map.provider, Some(crate::route_map::RouteProviderKind::Sygic));
    let points: Vec<(f64, f64, Option<String>)> =
        map.waypoints.iter().map(|w| (w.lat, w.lon, w.name.clone())).collect();
    assert_eq!(
        points,
        vec![(HERE.0, HERE.1, Some("Domov".into())), (THERE.0, THERE.1, Some("Kancelária".into()))]
    );
    // The trip itself does not change.
    assert_eq!(db.get_trip(&trip_id).unwrap().unwrap().distance_km, 25.0);
}

#[test]
fn saved_route_needs_a_known_trip_and_a_writable_book() {
    let db = Database::in_memory().unwrap();
    let err = save_manual_trip_route_internal(
        &db,
        &AppState::new(),
        &uuid::Uuid::new_v4().to_string(),
        "poly".into(),
        1.0,
        None,
    )
    .unwrap_err();
    assert!(err.contains("Trip not found"), "{err}");

    let v = vehicle(&db);
    let a = place(&db, "A", HERE);
    let b = place(&db, "B", THERE);
    let trip_id = manual_trip(&db, &v, &a, &b, 10.0);
    let read_only = AppState::new();
    read_only.enable_read_only("newer database");
    assert!(save_manual_trip_route_internal(&db, &read_only, &trip_id, "poly".into(), 10.0, None).is_err());
    assert!(db.get_route_map(&trip_id).unwrap().is_none());
}
