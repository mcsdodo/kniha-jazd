//! Tests for generated route map commands.
//!
//! Nothing here touches the network: [`StubProvider`] stands in for OSRM.

use super::*;
use crate::app_state::AppState;
use crate::db::Database;
use crate::export::RouteMapPage;
use crate::models::{PlaceSource, RouteMode, Trip, Vehicle, Waypoint};
use crate::route_map::polyline::encode;
use crate::route_map::tiles::TileFetcher;
use crate::commands_internal::{build_trip_grid_data, save_place_internal};
use crate::route_map::{Dataset, FetchedRoute, RouteProvider};
use chrono::NaiveDate;
use uuid::Uuid;

/// Geometry provider that never leaves the process.
struct StubProvider {
    polyline: String,
    road_km: f64,
    duration_s: f64,
}

impl StubProvider {
    /// A stub whose polyline really is a valid encoding of `points`, so tests
    /// that assert on decoded coordinates are asserting on real codec output.
    fn encoding(points: &[(f64, f64)], road_km: f64) -> Self {
        Self {
            polyline: encode(points),
            road_km,
            duration_s: 3600.0,
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
            duration_s: self.duration_s,
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
        RouteMode::Loop,
        false,
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
        RouteMode::Loop,
        false,
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
        RouteMode::Loop,
        false,
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
    assert_eq!(loaded.mode, RouteMode::Loop);
    assert!(!loaded.round_trip, "a loop must never be saved as a round trip");
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
    assert_eq!(
        route.dataset_version,
        Some(ds.version.clone()),
        "the dataset version must be reported"
    );
    assert_eq!(route.mode, RouteMode::Loop);
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

#[test]
fn grid_data_marks_only_the_trips_that_have_a_saved_map() {
    // The grid needs to know which rows already carry a map. It rides along on
    // the grid data rather than a command of its own, because a per-trip lookup
    // would be one request per row on every reload.
    let db = Database::in_memory().unwrap();
    let mapped = seed_trip(&db);

    let mut unmapped = Trip::test_ice_trip(
        NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
        80.0,
        None,
        true,
    );
    unmapped.vehicle_id = mapped.vehicle_id;
    unmapped.odometer = mapped.odometer + 80.0;
    db.create_trip(&unmapped).unwrap();

    let app_state = AppState::new();
    let (_, polyline) = sample_geometry();
    save_trip_route_internal(
        &db,
        &app_state,
        mapped.id.to_string(),
        sample_waypoints(),
        polyline,
        120.0,
        118.4,
        RouteMode::Loop,
        false,
    )
    .unwrap();

    let grid = crate::commands_internal::build_trip_grid_data(
        &db,
        &mapped.vehicle_id.to_string(),
        2026,
    )
    .unwrap();

    assert!(
        grid.route_map_trip_ids.contains(&mapped.id.to_string()),
        "the mapped trip must be marked"
    );
    assert!(
        !grid.route_map_trip_ids.contains(&unmapped.id.to_string()),
        "an unmapped trip must not be marked"
    );
    assert_eq!(grid.route_map_trip_ids.len(), 1);
}

// ---------------------------------------------------------------------------
// Export attachments
// ---------------------------------------------------------------------------

/// The offline case: no tile ever arrives. Used everywhere below so that no
/// test in this file can reach the network — `render_route` skips unreachable
/// tiles and still returns a PNG, which is exactly the behaviour an export
/// needs.
struct OfflineTiles;

#[async_trait::async_trait]
impl TileFetcher for OfflineTiles {
    async fn tile(&self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, String> {
        Err(format!("offline: no tile service for {z}/{x}/{y}"))
    }
}

/// A vehicle plus `count` consecutive trips, one per day, 100 km each — so the
/// grid numbers them 1..=count chronologically.
fn seed_vehicle_with_trips(db: &Database, count: usize) -> (Uuid, Vec<Trip>) {
    let vehicle = Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
    db.create_vehicle(&vehicle).unwrap();

    let mut trips = Vec::with_capacity(count);
    for i in 0..count {
        let mut trip = Trip::test_ice_trip(
            NaiveDate::from_ymd_opt(2026, 3, 1 + i as u32).unwrap(),
            100.0,
            None,
            true,
        );
        trip.vehicle_id = vehicle.id;
        trip.odometer = 10_000.0 + 100.0 * (i as f64 + 1.0);
        db.create_trip(&trip).unwrap();
        trips.push(trip);
    }

    (vehicle.id, trips)
}

fn save_map_for(db: &Database, trip_id: &Uuid, polyline: &str) {
    save_trip_route_internal(
        db,
        &AppState::new(),
        trip_id.to_string(),
        sample_waypoints(),
        polyline.to_string(),
        120.0,
        118.4,
        RouteMode::Loop,
        false,
    )
    .unwrap();
}

/// `(attachment_no, row_number)` for every page, in order — the only two
/// numbers a reviewer can cross-reference by.
fn numbering(pages: &[RouteMapPage]) -> Vec<(usize, usize)> {
    pages
        .iter()
        .map(|p| (p.attachment_no, p.row_number))
        .collect()
}

/// Every page must carry a decodable PNG, not just a non-empty string.
fn assert_pages_carry_pngs(pages: &[RouteMapPage]) {
    for page in pages {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&page.png_base64)
            .expect("the page image must be valid base64");
        assert_eq!(
            &bytes[..8],
            b"\x89PNG\r\n\x1a\n",
            "the page image must be a PNG"
        );
    }
}

/// The row number is the one cross-reference between an attachment and the
/// journey it documents, so it must come out of the row list the printed table
/// is numbered from — never from a fresh index or a re-derived order.
#[tokio::test]
async fn attachment_row_numbers_come_from_the_assembled_rows() {
    let db = Database::in_memory().unwrap();
    let (_, trips) = seed_vehicle_with_trips(&db, 4);
    let (_, polyline) = sample_geometry();

    save_map_for(&db, &trips[0].id, &polyline);
    save_map_for(&db, &trips[2].id, &polyline);
    // A saved map on a trip the printed table does not list (another year's
    // export, say). It has no row to point at, so it must not be attached.
    save_map_for(&db, &trips[3].id, &polyline);

    let rows = vec![
        (1usize, trips[0].id.to_string()),
        (2, trips[1].id.to_string()),
        (3, trips[2].id.to_string()),
    ];

    let pages = collect_route_map_pages(&db, &OfflineTiles, &rows).await;

    assert_eq!(numbering(&pages), vec![(1, 1), (2, 3)]);
    assert_pages_carry_pngs(&pages);
}

/// Attachments are numbered by how many there are, not by which rows they
/// document: "Príloha č. 2" is the second attachment even when it points at
/// record 9.
#[tokio::test]
async fn attachment_numbers_are_sequential_regardless_of_row_gaps() {
    let db = Database::in_memory().unwrap();
    let (_, trips) = seed_vehicle_with_trips(&db, 2);
    let (_, polyline) = sample_geometry();
    save_map_for(&db, &trips[0].id, &polyline);
    save_map_for(&db, &trips[1].id, &polyline);

    let rows = vec![(2usize, trips[0].id.to_string()), (9, trips[1].id.to_string())];

    let pages = collect_route_map_pages(&db, &OfflineTiles, &rows).await;

    assert_eq!(numbering(&pages), vec![(1, 2), (2, 9)]);
}

#[tokio::test]
async fn trips_without_maps_produce_no_pages() {
    let db = Database::in_memory().unwrap();
    let (_, trips) = seed_vehicle_with_trips(&db, 3);

    let rows: Vec<(usize, String)> = trips
        .iter()
        .enumerate()
        .map(|(i, t)| (i + 1, t.id.to_string()))
        .collect();

    let pages = collect_route_map_pages(&db, &OfflineTiles, &rows).await;

    assert!(
        pages.is_empty(),
        "no saved maps means no attachment pages at all, got {}",
        pages.len()
    );
}

/// A stored polyline that cannot be decoded costs its own page and nothing
/// else. Failing the whole export over one bad row would lose the printed
/// logbook, which is the thing that actually has to be produced.
#[tokio::test]
async fn an_unrenderable_route_skips_only_its_own_page() {
    let db = Database::in_memory().unwrap();
    let (_, trips) = seed_vehicle_with_trips(&db, 3);
    let (_, polyline) = sample_geometry();

    save_map_for(&db, &trips[0].id, &polyline);
    // Bytes below the polyline5 ASCII offset: `decode` yields no points at all,
    // so there is nothing to render.
    save_map_for(&db, &trips[1].id, "!!! not a polyline !!!");
    save_map_for(&db, &trips[2].id, &polyline);

    let rows: Vec<(usize, String)> = trips
        .iter()
        .enumerate()
        .map(|(i, t)| (i + 1, t.id.to_string()))
        .collect();

    let pages = collect_route_map_pages(&db, &OfflineTiles, &rows).await;

    assert_eq!(
        numbering(&pages),
        vec![(1, 1), (2, 3)],
        "the broken row drops out; the good rows keep their record numbers and \
         the attachment numbering closes the gap"
    );
    assert_pages_carry_pngs(&pages);
}

/// Both export paths now agree: each prepends a synthetic "Prvý záznam" row and
/// each honours the user's sort direction (Task 73 widened
/// `export_html_internal` so server mode does both too). This test still pins
/// the property that made the difference safe in the first place — whatever the
/// extra rows and whatever the order, both must cite the SAME record number for
/// the same trip, because the attachment heading is the only link back to it.
/// The two grids below are therefore deliberately built differently.
#[tokio::test]
async fn both_export_modes_cite_the_same_record_for_the_same_trip() {
    let db = Database::in_memory().unwrap();
    let (vehicle_id, trips) = seed_vehicle_with_trips(&db, 3);
    let (_, polyline) = sample_geometry();
    let mapped = &trips[1];
    save_map_for(&db, &mapped.id, &polyline);

    // Configuration A: the grid exactly as built, oldest first.
    let server_grid = build_trip_grid_data(&db, &vehicle_id.to_string(), 2026).unwrap();
    let server_rows = assemble_export_rows(&server_grid, "asc");

    // Configuration B: the same grid plus the synthetic first record, newest first.
    let mut desktop_grid = build_trip_grid_data(&db, &vehicle_id.to_string(), 2026).unwrap();
    let mut first_record = Trip::test_ice_trip(
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        0.0,
        None,
        true,
    );
    first_record.id = Uuid::nil();
    first_record.vehicle_id = vehicle_id;
    desktop_grid.trips.push(first_record);
    desktop_grid.trip_numbers.insert(Uuid::nil().to_string(), 0);
    let desktop_rows = assemble_export_rows(&desktop_grid, "desc");

    assert!(
        !desktop_rows
            .iter()
            .any(|(_, id)| id == &Uuid::nil().to_string()),
        "the synthetic first record prints no number, so it can never be cited"
    );
    assert_eq!(
        desktop_rows.first().map(|(n, _)| *n),
        Some(3),
        "descending export puts the highest record on top"
    );
    assert_eq!(server_rows.first().map(|(n, _)| *n), Some(1));

    let server_pages = collect_route_map_pages(&db, &OfflineTiles, &server_rows).await;
    let desktop_pages = collect_route_map_pages(&db, &OfflineTiles, &desktop_rows).await;

    assert_eq!(numbering(&server_pages), vec![(1, 2)]);
    assert_eq!(
        numbering(&desktop_pages),
        numbering(&server_pages),
        "the same trip must be cited as the same record in both export modes"
    );
}

#[test]
fn deviation_is_measured_against_the_road_distance_and_flagged_from_one_constant() {
    // The threshold lives in Rust so the display cannot invent a second,
    // differently-measured notion of "close enough" (ADR-008).
    let db = Database::in_memory().unwrap();
    let mut trip = seed_trip(&db);
    let app_state = AppState::new();
    let (_, polyline) = sample_geometry();

    // Task 78: the target is read from the trip's own distance, not from the
    // value passed to save_trip_route_internal, so the trip must carry it.
    trip.distance_km = 100.0;
    db.update_trip(&trip).unwrap();

    // 100 km target, 108 km of road: 8% out, beyond the 5% tolerance.
    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline.clone(),
        100.0,
        108.0,
        RouteMode::Loop,
        false,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .expect("must exist");
    assert!((loaded.deviation_percent - 8.0).abs() < 1e-9);
    assert!(loaded.off_target, "8% must exceed the {TOLERANCE} tolerance");

    // 100 km target, 102 km of road: 2% out, within tolerance.
    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline,
        100.0,
        102.0,
        RouteMode::Loop,
        false,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .expect("must exist");
    assert!((loaded.deviation_percent - 2.0).abs() < 1e-9);
    assert!(!loaded.off_target, "2% must be within tolerance");
}

// ---------------------------------------------------------------------------
// mode_for / start_route_for_trip: the place-book route mode decision
// ---------------------------------------------------------------------------

fn app_state() -> AppState {
    AppState::new()
}

#[test]
fn the_same_place_twice_is_a_loop() {
    assert_eq!(mode_for("Domov", "Domov").unwrap(), RouteMode::Loop);
}

/// Mode selection and the place book MUST share one notion of sameness, or a
/// row could route A->B while its endpoints resolve to one book entry.
/// Both call `places::normalise` -- there is no second implementation.
#[test]
fn sameness_is_judged_after_normalisation() {
    assert_eq!(mode_for("Spišská", "spisska ").unwrap(), RouteMode::Loop);
}

#[test]
fn different_places_are_a_direct_route() {
    assert_eq!(
        mode_for("Bratislava", "Spišská Nová Ves").unwrap(),
        RouteMode::Direct
    );
}

/// A blank endpoint must NOT quietly become a home loop: that hands the user
/// a map of somewhere they never were, labelled as evidence.
#[test]
fn a_blank_endpoint_is_an_error_not_a_loop() {
    assert!(mode_for("", "Košice").is_err());
    assert!(mode_for("Košice", "   ").is_err());
    assert!(mode_for("", "").is_err());
}

// --- start_route_for_trip: the one caller of mode_for ---

fn seed_trip_between(db: &Database, origin: &str, destination: &str) -> Trip {
    let trip = seed_trip(db);
    let mut updated = trip.clone();
    updated.origin = origin.into();
    updated.destination = destination.into();
    db.update_trip(&updated).unwrap();
    updated
}

#[test]
fn a_same_place_row_starts_in_loop_mode() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip_between(&db, "Domov", "domov ");

    let start = start_route_for_trip_internal(&db, trip.id.to_string()).unwrap();

    assert_eq!(start.mode, RouteMode::Loop);
}

#[test]
fn a_direct_row_carries_both_endpoints_from_the_book() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip_between(&db, "Office, City A", "Depot, City B");
    save_place_internal(&db, &app_state(), "Office, City A".into(), 48.1, 17.1, PlaceSource::Manual).unwrap();
    save_place_internal(&db, &app_state(), "Depot, City B".into(), 48.7, 21.2, PlaceSource::Geocoder).unwrap();

    let start = start_route_for_trip_internal(&db, trip.id.to_string()).unwrap();

    assert_eq!(start.mode, RouteMode::Direct);
    assert_eq!(start.origin.unwrap().lat, Some(48.1));
    assert_eq!(start.destination.unwrap().lon, Some(21.2));
}

/// The endpoint's spelling in the trip need not match the book's byte for byte --
/// the book is keyed on the normalised form, which is the whole point.
#[test]
fn an_endpoint_is_found_regardless_of_spelling() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip_between(&db, "OFFICE, CITY A", "Depot, City B");
    save_place_internal(&db, &app_state(), "Office, City A".into(), 48.1, 17.1, PlaceSource::Manual).unwrap();

    let start = start_route_for_trip_internal(&db, trip.id.to_string()).unwrap();

    assert_eq!(start.origin.unwrap().lat, Some(48.1));
}

/// An unplaced endpoint is NOT an error: the map view opens the book's dialog in
/// place so the user can fix it without leaving the page. Failing here would
/// turn a two-click fix into a redirect.
#[test]
fn an_unplaced_endpoint_is_reported_not_refused() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip_between(&db, "Office, City A", "Depot, City B");
    save_place_internal(&db, &app_state(), "Office, City A".into(), 48.1, 17.1, PlaceSource::Manual).unwrap();

    let start = start_route_for_trip_internal(&db, trip.id.to_string()).unwrap();

    assert!(start.origin.is_some());
    assert!(start.destination.is_none(), "the unplaced endpoint reports as None");
}

#[test]
fn a_blank_endpoint_still_fails_the_whole_call() {
    let db = Database::in_memory().unwrap();
    let trip = seed_trip_between(&db, "", "Depot, City B");
    assert!(start_route_for_trip_internal(&db, trip.id.to_string()).is_err());
}

// ---------------------------------------------------------------------------
// insert_waypoint: where a dragged-in point lands in the waypoint sequence
// ---------------------------------------------------------------------------

/// Points along a straight west->east line, so "between" is unambiguous.
fn line_points() -> Vec<(f64, f64)> {
    (0..=10).map(|i| (48.9, 20.0 + i as f64 * 0.1)).collect()
}

fn wp(lat: f64, lon: f64) -> Waypoint {
    Waypoint { lat, lon, name: None, node_idx: None }
}

#[test]
fn a_point_dragged_mid_route_lands_between_the_endpoints() {
    let points = line_points();
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 21.0)];
    // Dragged off the middle of the line.
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.5);

    assert_eq!(inserted.len(), 3);
    assert!((inserted[1].lat - 48.95).abs() < 1e-9);
    assert!((inserted[1].lon - 20.5).abs() < 1e-9);
}

#[test]
fn a_point_dragged_from_the_first_leg_lands_in_the_first_slot() {
    let points = line_points();
    // Three waypoints: start, middle of the line, end.
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 20.5), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.2);

    assert_eq!(inserted.len(), 4);
    assert!(
        (inserted[1].lon - 20.2).abs() < 1e-9,
        "a point on the first leg belongs before the middle waypoint, got {:?}",
        inserted.iter().map(|w| w.lon).collect::<Vec<_>>()
    );
}

#[test]
fn a_point_dragged_from_the_last_leg_lands_in_the_last_slot() {
    let points = line_points();
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 20.5), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.8);

    assert_eq!(inserted.len(), 4);
    assert!((inserted[2].lon - 20.8).abs() < 1e-9);
}

/// A new waypoint is never an endpoint: dragging must not silently change
/// where the journey started or finished.
#[test]
fn insertion_never_displaces_an_endpoint() {
    let points = line_points();
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, &encode(&points), 48.95, 20.01);

    assert!((inserted[0].lon - 20.0).abs() < 1e-9, "origin moved");
    assert!(
        (inserted.last().unwrap().lon - 21.0).abs() < 1e-9,
        "destination moved"
    );
}

/// Undecodable geometry must not lose the point or panic -- append before the
/// destination, which is the only slot that is always valid.
#[test]
fn a_broken_polyline_still_places_the_point() {
    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 21.0)];
    let inserted = insert_waypoint(&waypoints, "", 48.95, 20.5);
    assert_eq!(inserted.len(), 3);
    assert!((inserted[1].lon - 20.5).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// route_direct_internal: A-to-B routing with alternatives
// ---------------------------------------------------------------------------

/// Returns as many alternatives as asked for, each with its own distance --
/// enough to prove per-alternative deviation is computed, not copied.
struct MultiRouteProvider {
    routes: Vec<FetchedRoute>,
}

#[async_trait::async_trait]
impl RouteProvider for MultiRouteProvider {
    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        Ok(self.routes[0].clone())
    }
    async fn fetch_alternatives(
        &self,
        _coords: &[(f64, f64)],
        _max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        Ok(self.routes.clone())
    }
}

fn fetched(polyline: &str, road_km: f64, duration_s: f64) -> FetchedRoute {
    FetchedRoute { polyline: polyline.into(), road_km, duration_s }
}

fn direct_waypoints() -> Vec<Waypoint> {
    vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
    ]
}

#[tokio::test]
async fn direct_routes_are_returned_in_provider_order() {
    let provider = MultiRouteProvider {
        routes: vec![
            fetched(&encode(&[(48.1, 17.1), (48.9, 20.5)]), 400.0, 14000.0),
            fetched(&encode(&[(48.1, 17.1), (49.0, 20.6)]), 380.0, 16000.0),
        ],
    };

    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None, false)
        .await
        .unwrap();

    assert_eq!(routes.len(), 2);
    assert!((routes[0].road_km - 400.0).abs() < 1e-9, "fastest must stay first");
    assert!((routes[1].road_km - 380.0).abs() < 1e-9);
}

/// Fastest-first even when the fastest route is the WORSE distance match.
/// `direct_routes_are_returned_in_provider_order` cannot see a deviation-based
/// sort: there the fastest route (400 km) is also the closest to 420 km, so
/// such a sort would reproduce provider order by accident. Here the two
/// orders disagree, so only a genuine "never re-sort" implementation passes.
#[tokio::test]
async fn alternatives_are_never_reordered_by_distance_match() {
    let provider = MultiRouteProvider {
        routes: vec![
            fetched(&encode(&[(48.1, 17.1), (48.9, 20.5)]), 300.0, 14000.0),
            fetched(&encode(&[(48.1, 17.1), (49.0, 20.6)]), 420.0, 16000.0),
        ],
    };

    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None, false)
        .await
        .unwrap();

    assert!(
        (routes[0].road_km - 300.0).abs() < 1e-9,
        "the fastest route stays first even though it misses the recorded distance"
    );
    assert!((routes[0].duration_s - 14000.0).abs() < 1e-9);
    assert!(routes[0].off_target, "deviation labels it, it does not move it");
    assert!((routes[1].road_km - 420.0).abs() < 1e-9);
    assert!(!routes[1].off_target);
}

/// Each alternative is measured against the row's own distance_km, by the
/// SAME deviation helper loop mode uses -- one tolerance, one home.
#[tokio::test]
async fn every_alternative_carries_its_own_deviation() {
    let provider = MultiRouteProvider {
        routes: vec![
            fetched(&encode(&[(48.1, 17.1), (48.9, 20.5)]), 420.0, 14000.0),
            fetched(&encode(&[(48.1, 17.1), (49.0, 20.6)]), 300.0, 16000.0),
        ],
    };

    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None, false)
        .await
        .unwrap();

    assert!(routes[0].deviation_percent.abs() < 1e-6);
    assert!(!routes[0].off_target);
    assert!(routes[1].deviation_percent < -20.0);
    assert!(routes[1].off_target, "a 300 km route for a 420 km row must be flagged");
}

#[tokio::test]
async fn a_direct_route_is_marked_direct_and_claims_no_dataset() {
    let provider = MultiRouteProvider {
        routes: vec![fetched(&encode(&[(48.1, 17.1), (48.9, 20.5)]), 400.0, 14000.0)],
    };
    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None, false)
        .await
        .unwrap();

    assert_eq!(routes[0].mode, RouteMode::Direct);
    assert!(
        routes[0].dataset_version.is_none(),
        "no dataset node was used, so claiming a dataset version would be a lie"
    );
}

/// The insert point is applied BEFORE routing, and the returned waypoints are
/// authoritative -- that is what lets the frontend adopt them wholesale.
#[tokio::test]
async fn an_insert_point_is_applied_before_routing() {
    let geometry = encode(&[(48.9, 20.0), (48.9, 20.5), (48.9, 21.0)]);
    let provider = MultiRouteProvider {
        routes: vec![fetched(&geometry, 400.0, 14000.0)],
    };

    let routes = route_direct_internal(
        &provider,
        direct_waypoints(),
        420.0,
        Some(InsertPoint { lat: 48.95, lon: 20.5, polyline: geometry.clone() }),
        false,
    )
    .await
    .unwrap();

    assert_eq!(routes[0].waypoints.len(), 3, "the dragged point must be in the result");
    assert!((routes[0].waypoints[1].lat - 48.95).abs() < 1e-9);
}

/// The point must reach the routing service already inserted -- checking only
/// the returned waypoints (as the mandated test above does) cannot tell
/// "inserted before routing" apart from "inserted into the response after".
/// This provider inspects what it was actually asked to route.
struct CoordAssertingProvider {
    route: FetchedRoute,
}

#[async_trait::async_trait]
impl RouteProvider for CoordAssertingProvider {
    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        Ok(self.route.clone())
    }
    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        _max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        assert_eq!(
            coords.len(),
            3,
            "the provider must be asked to route the dragged point, not just the endpoints"
        );
        assert!(
            (coords[1].0 - 48.95).abs() < 1e-9 && (coords[1].1 - 20.5).abs() < 1e-9,
            "the dragged point must sit in the routed coordinate list, got {:?}",
            coords
        );
        Ok(vec![self.route.clone()])
    }
}

#[tokio::test]
async fn the_routing_service_is_asked_to_route_through_the_inserted_point() {
    let geometry = encode(&[(48.9, 20.0), (48.9, 20.5), (48.9, 21.0)]);
    let provider = CoordAssertingProvider {
        route: fetched(&geometry, 400.0, 14000.0),
    };

    route_direct_internal(
        &provider,
        direct_waypoints(),
        420.0,
        Some(InsertPoint { lat: 48.95, lon: 20.5, polyline: geometry.clone() }),
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn a_route_needs_at_least_two_waypoints() {
    let provider = MultiRouteProvider {
        routes: vec![fetched("aaa", 1.0, 1.0)],
    };
    assert!(
        route_direct_internal(&provider, vec![direct_waypoints()[0].clone()], 10.0, None, false)
            .await
            .is_err()
    );
}

// ---------------------------------------------------------------------------
// route_direct_internal: round_trip appends a return leg (Task 19)
// ---------------------------------------------------------------------------

/// Inspects the coordinate list actually sent to routing, the same way
/// `CoordAssertingProvider` does for `insert` -- checking only the RETURNED
/// waypoints cannot tell "closed before routing" apart from "closed after the
/// response came back".
struct RoundTripAssertingProvider {
    /// Whether the routed coordinate list is expected to end where it began.
    expect_closed: bool,
    route: FetchedRoute,
}

#[async_trait::async_trait]
impl RouteProvider for RoundTripAssertingProvider {
    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        Ok(self.route.clone())
    }
    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        _max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        // Comparing the FIRST and LAST coordinate the provider actually
        // received -- not the length of the list -- so a mutation that
        // lengthens the list without truly closing the loop (e.g. appending
        // the last waypoint again instead of the first) is still caught.
        let closed = coords.first() == coords.last();
        assert_eq!(
            closed, self.expect_closed,
            "round_trip={} must{} route a coordinate list whose last point equals \
             its first, got {:?}",
            self.expect_closed,
            if self.expect_closed { "" } else { " not" },
            coords
        );
        Ok(vec![self.route.clone()])
    }
}

#[tokio::test]
async fn round_trip_true_closes_the_loop_back_to_the_first_waypoint() {
    let provider = RoundTripAssertingProvider {
        expect_closed: true,
        route: fetched(
            &encode(&[(48.1486, 17.1077), (48.9444, 20.5675), (48.1486, 17.1077)]),
            800.0,
            28000.0,
        ),
    };

    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None, true)
        .await
        .unwrap();

    assert_eq!(
        routes[0].waypoints.len(),
        3,
        "the return leg must be appended to the returned waypoints too"
    );
    assert_eq!(routes[0].waypoints.first().unwrap().lat, routes[0].waypoints.last().unwrap().lat);
    assert_eq!(routes[0].waypoints.first().unwrap().lon, routes[0].waypoints.last().unwrap().lon);
}

#[tokio::test]
async fn round_trip_false_leaves_the_route_one_way() {
    let provider = RoundTripAssertingProvider {
        expect_closed: false,
        route: fetched(&encode(&[(48.1486, 17.1077), (48.9444, 20.5675)]), 400.0, 14000.0),
    };

    let routes = route_direct_internal(&provider, direct_waypoints(), 420.0, None, false)
        .await
        .unwrap();

    assert_eq!(routes[0].waypoints.len(), 2, "no return leg means no third waypoint");
}

/// The return leg is appended AFTER `insert` is applied (design decision 2):
/// the dragged-in via must sit between the endpoints, not get caught up in
/// the ambiguity of a route that already doubles back on itself.
#[tokio::test]
async fn round_trip_closes_the_loop_after_the_insert_is_placed() {
    let geometry = encode(&[(48.9, 20.0), (48.9, 20.5), (48.9, 21.0)]);
    let provider = RoundTripAssertingProvider {
        expect_closed: true,
        route: fetched(&geometry, 900.0, 30000.0),
    };

    let waypoints = vec![wp(48.9, 20.0), wp(48.9, 21.0)];
    let routes = route_direct_internal(
        &provider,
        waypoints,
        420.0,
        Some(InsertPoint { lat: 48.9, lon: 20.5, polyline: geometry.clone() }),
        true,
    )
    .await
    .unwrap();

    assert_eq!(
        routes[0].waypoints.len(),
        4,
        "one inserted via plus one appended return leg, on top of the two endpoints"
    );
    assert!(
        (routes[0].waypoints[1].lon - 20.5).abs() < 1e-9,
        "the via must sit between the endpoints, not be swallowed by the return leg"
    );
    assert_eq!(routes[0].waypoints.first().unwrap().lat, routes[0].waypoints.last().unwrap().lat);
    assert_eq!(routes[0].waypoints.first().unwrap().lon, routes[0].waypoints.last().unwrap().lon);
}

/// Reopening a saved round trip yields an ALREADY-CLOSED waypoint list
/// (`[A, B, A]`) -- that is what got persisted the first time round. Ticking
/// the checkbox again must not append a SECOND closing point on top of an
/// already-closed route: that would silently invent a zero-length final leg
/// and a wrong stop count on every subsequent regenerate (fix round 1,
/// review finding "Important 1"). The guard belongs here, in Rust: it is the
/// layer that owns this logic (ADR-008), and it defends every caller,
/// including a persisted round-trip flag that does not exist yet.
#[tokio::test]
async fn round_trip_does_not_double_close_an_already_closed_route() {
    let already_closed = vec![
        wp(48.1486, 17.1077),
        wp(48.9444, 20.5675),
        wp(48.1486, 17.1077),
    ];
    let provider = RoundTripAssertingProvider {
        expect_closed: true,
        route: fetched(
            &encode(&[(48.1486, 17.1077), (48.9444, 20.5675), (48.1486, 17.1077)]),
            800.0,
            28000.0,
        ),
    };

    let routes = route_direct_internal(&provider, already_closed, 420.0, None, true)
        .await
        .unwrap();

    assert_eq!(
        routes[0].waypoints.len(),
        3,
        "an already-closed route must not gain a second, redundant closing point"
    );
}

/// The mirror of the test above (Task 20, fix round 2): `round_trip: false`
/// against an ALREADY-CLOSED list must strip the trailing closing point back
/// open, not leave it in place. Before this fix the guard only normalised in
/// one direction (close when true); a frontend that handed this function a
/// stale closed list with `round_trip: false` -- e.g. unticking the checkbox
/// after a cold load, before any successful recalculate re-derived
/// `baseWaypoints` -- got a 3-stop route back while reporting itself
/// unticked. Fixing that in the frontend closes one door; a caller can always
/// find another. This function is the one place ADR-008 says must be
/// authoritative regardless of what state the frontend leaks: a closed list
/// reaching `route_direct_internal` can only be one this function itself
/// closed (a same-place row is routed as Loop by `mode_for`, never Direct),
/// so `round_trip: false` on a closed list is unambiguously "this used to be
/// a round trip and no longer is."
///
/// Uses `RoundTripAssertingProvider` with `expect_closed: false`, the same
/// helper the test above uses with `true` -- it inspects the coordinates
/// actually sent to routing, so this proves the strip happens BEFORE the
/// request goes out, not that the response is trimmed afterward.
#[tokio::test]
async fn round_trip_false_reopens_an_already_closed_list() {
    let already_closed = vec![
        wp(48.1486, 17.1077),
        wp(48.9444, 20.5675),
        wp(48.1486, 17.1077),
    ];
    let provider = RoundTripAssertingProvider {
        expect_closed: false,
        route: fetched(&encode(&[(48.1486, 17.1077), (48.9444, 20.5675)]), 400.0, 14000.0),
    };

    let routes = route_direct_internal(&provider, already_closed, 420.0, None, false)
        .await
        .unwrap();

    assert_eq!(
        routes[0].waypoints.len(),
        2,
        "round_trip: false must strip the trailing closing point from an \
         already-closed list, not leave it in place"
    );
}

/// Asserts on the NUMBER of coordinates actually sent to routing, not just
/// the length of the returned waypoints -- the same reasoning
/// `RoundTripAssertingProvider` documents: the returned list and the routed
/// list happen to be the same variable today, but a test that only reads the
/// response could not tell a future refactor apart from a genuine fix.
struct WaypointCountAssertingProvider {
    expected_count: usize,
    route: FetchedRoute,
}

#[async_trait::async_trait]
impl RouteProvider for WaypointCountAssertingProvider {
    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        Ok(self.route.clone())
    }
    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        _max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        assert_eq!(
            coords.len(),
            self.expected_count,
            "expected {} coordinate(s) sent to routing, got {:?}",
            self.expected_count,
            coords
        );
        Ok(vec![self.route.clone()])
    }
}

/// Task 20, fix round 3: two DIFFERENT places can carry bit-identical
/// coordinates. `mode_for` (this module, below) decides Direct vs Loop by
/// comparing NAMES after `places::normalise`, never by coordinate, and
/// `save_place_internal` (`places_cmd.rs`) keys the book on
/// `normalised_name` alone -- it enforces no coordinate uniqueness. So a
/// book with two entries for one real address under different spellings
/// (e.g. "Mlynske Nivy 14" and "Mlynske Nivy 14, Bratislava") can hand this
/// function a `[A, via, B]` list where A and B share a coordinate but are
/// not the same waypoint. Comparing only lat/lon for "already closed" would
/// mistake B for a stale closing point this function itself appended, and
/// pop it -- the user silently loses their destination. Comparing the name
/// too tells the cases apart: see the doc comment on `already_closed`.
#[tokio::test]
async fn round_trip_false_leaves_a_coincidentally_co_located_destination_alone() {
    let origin = Waypoint {
        lat: 48.1486,
        lon: 17.1077,
        name: Some("Mlynske Nivy 14".into()),
        node_idx: None,
    };
    let via = wp(48.9444, 20.5675);
    let destination = Waypoint {
        lat: 48.1486,
        lon: 17.1077,
        name: Some("Mlynske Nivy 14, Bratislava".into()),
        node_idx: None,
    };
    let waypoints = vec![origin, via, destination];

    let provider = WaypointCountAssertingProvider {
        expected_count: 3,
        route: fetched(
            &encode(&[(48.1486, 17.1077), (48.9444, 20.5675), (48.1486, 17.1077)]),
            800.0,
            28000.0,
        ),
    };

    let routes = route_direct_internal(&provider, waypoints, 420.0, None, false)
        .await
        .unwrap();

    assert_eq!(
        routes[0].waypoints.len(),
        3,
        "two DIFFERENT places that merely share a coordinate must not be \
         mistaken for a stale closing point and popped"
    );
    assert_eq!(
        routes[0].waypoints.last().and_then(|w| w.name.as_deref()),
        Some("Mlynske Nivy 14, Bratislava"),
        "the real destination must survive -- the returned last waypoint \
         must still be B, not the via or a truncated list"
    );
}

/// The mirror case, and the regression guard for the test above: a GENUINE
/// closing point -- cloned from the first waypoint by this very function, so
/// it carries the SAME name as well as the same coordinate -- must still be
/// stripped when `round_trip` goes back to `false`. Without this test, a fix
/// that starts comparing names could over-correct (e.g. by requiring a
/// `Some` name and refusing to close a list of unnamed waypoints) and this
/// would not be caught.
#[tokio::test]
async fn round_trip_false_still_strips_a_genuinely_closed_named_list() {
    let start = Waypoint {
        lat: 48.1486,
        lon: 17.1077,
        name: Some("Bratislava".into()),
        node_idx: None,
    };
    let via = wp(48.9444, 20.5675);
    let already_closed = vec![start.clone(), via, start.clone()];

    let provider = WaypointCountAssertingProvider {
        expected_count: 2,
        route: fetched(&encode(&[(48.1486, 17.1077), (48.9444, 20.5675)]), 400.0, 14000.0),
    };

    let routes = route_direct_internal(&provider, already_closed, 420.0, None, false)
        .await
        .unwrap();

    assert_eq!(
        routes[0].waypoints.len(),
        2,
        "a genuine closing point -- cloned from the first waypoint, same \
         name and coordinates -- must still be stripped when round_trip is \
         false"
    );
}

/// All five tests above drag their point onto a straight line held at a
/// constant latitude, so the longitude term alone could be driving every
/// placement decision and the latitude term would never be exercised. This
/// route is a dogleg (east, then north) built so neither single-axis metric
/// agrees with the true two-axis nearest-vertex answer:
///
/// - `decoy_lat` sits on the first (east) leg but carries the SAME latitude
///   as the dragged point, so a latitude-only search picks it over the true
///   answer on the second leg.
/// - `decoy_lon` sits on the first leg but carries the SAME longitude as the
///   dragged point, so a longitude-only search picks it over the true answer
///   too.
/// - Only comparing both axes together correctly prefers the vertex that is
///   actually nearest, which sits on the second (north) leg.
///
/// If `nearest()` ever degenerated to a single coordinate, this test fails
/// while the five above would keep passing silently.
#[test]
fn a_point_dragged_off_a_dogleg_lands_by_the_full_route_not_one_axis() {
    let points = vec![
        (48.80, 20.00), // origin
        (48.85, 20.10),
        (49.30, 20.02), // decoy_lat: shares the dragged point's latitude
        (48.88, 20.50), // decoy_lon: shares the dragged point's longitude
        (48.90, 20.55), // corner (middle waypoint)
        (49.10, 20.52),
        (49.29, 20.53), // the true nearest vertex, on the north leg
        (49.50, 20.60), // destination
    ];
    let waypoints = vec![wp(48.80, 20.00), wp(48.90, 20.55), wp(49.50, 20.60)];

    let inserted = insert_waypoint(&waypoints, &encode(&points), 49.30, 20.50);

    assert_eq!(inserted.len(), 4);
    assert!(
        (inserted[2].lat - 49.30).abs() < 1e-9 && (inserted[2].lon - 20.50).abs() < 1e-9,
        "a point nearest the north leg must land there even though one decoy \
         vertex on the first leg shares its latitude and another shares its \
         longitude, got lat/lon pairs {:?}",
        inserted.iter().map(|w| (w.lat, w.lon)).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Persisting the mode (Task 72, Phase 2)
// ---------------------------------------------------------------------------

#[test]
fn a_saved_direct_route_round_trips_with_its_mode_and_vias() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let waypoints = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.7, lon: 19.1, name: None, node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská".into()), node_idx: None },
    ];

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        waypoints.clone(),
        encode(&[(48.1, 17.1), (48.9, 20.5)]),
        420.0,
        400.0,
        RouteMode::Direct,
        false,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.mode, RouteMode::Direct);
    assert_eq!(loaded.waypoints.len(), 3);
    assert!(
        loaded.dataset_version.is_none(),
        "a direct route must not claim a dataset version"
    );
}

#[test]
fn a_saved_loop_route_still_stamps_the_dataset_version() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        encode(&[(48.9, 20.5), (49.0, 20.6)]),
        120.0,
        118.0,
        RouteMode::Loop,
        false,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.mode, RouteMode::Loop);
    assert!(loaded.dataset_version.is_some());
}

// ---------------------------------------------------------------------------
// Persisting the round-trip flag (Task 20)
// ---------------------------------------------------------------------------

/// The mirror pair below deliberately saves a DIRECT route, not a loop:
/// design decision 3 forces `round_trip` to `false` for `RouteMode::Loop`
/// regardless of what is passed in, so a Loop-mode test could not tell
/// "the flag round-trips" apart from "the flag is always false by policy".
/// Only Direct exercises the actual plumbing.
#[test]
fn a_saved_direct_round_trip_round_trips_true() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        direct_waypoints(),
        encode(&[(48.1, 17.1), (48.9, 20.5), (48.1, 17.1)]),
        420.0,
        400.0,
        RouteMode::Direct,
        true,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .unwrap();
    assert!(loaded.round_trip, "a saved round trip must read back as true");
}

#[test]
fn a_saved_direct_one_way_round_trips_false() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        direct_waypoints(),
        encode(&[(48.1, 17.1), (48.9, 20.5)]),
        420.0,
        400.0,
        RouteMode::Direct,
        false,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .unwrap();
    assert!(!loaded.round_trip, "a saved one-way route must read back as false");
}

/// The load-bearing test for design decision 3: a caller passing
/// `round_trip: true` alongside `RouteMode::Loop` must still be stored as
/// `false`, because a loop is already closed and the flag describes only the
/// direct router's behaviour. Without this test, forcing the value for Loop
/// is only a comment.
#[test]
fn a_saved_loop_route_never_stores_round_trip_even_if_asked() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        encode(&[(48.9, 20.5), (49.0, 20.6)]),
        120.0,
        118.0,
        RouteMode::Loop,
        true,
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .unwrap();
    assert!(
        !loaded.round_trip,
        "a loop must never be stored as a round trip, even if the caller asked for one"
    );
}

// ---------------------------------------------------------------------------
// Round trip as two legs (Task 78)
// ---------------------------------------------------------------------------

/// Answers each call with its own routes and records the coordinate lists it
/// was asked for, so a test can prove there were TWO requests and see the
/// points of each.
struct TwoLegProvider {
    outbound: Vec<FetchedRoute>,
    inbound: Vec<FetchedRoute>,
    calls: std::sync::Mutex<Vec<Vec<(f64, f64)>>>,
}

impl TwoLegProvider {
    fn new(outbound: Vec<FetchedRoute>, inbound: Vec<FetchedRoute>) -> Self {
        Self { outbound, inbound, calls: std::sync::Mutex::new(Vec::new()) }
    }
}

#[async_trait::async_trait]
impl RouteProvider for TwoLegProvider {
    async fn fetch(&self, _coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        Ok(self.outbound[0].clone())
    }
    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        _max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        let mut calls = self.calls.lock().unwrap();
        calls.push(coords.to_vec());
        if calls.len() == 1 {
            Ok(self.outbound.clone())
        } else {
            Ok(self.inbound.clone())
        }
    }
}

#[tokio::test]
async fn a_round_trip_is_two_requests_one_per_leg() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0), fetched("out2", 27.0, 1400.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let result = route_round_trip_internal(
        &provider,
        direct_waypoints(),
        Vec::new(),
        50.0,
        None,
    )
    .await
    .unwrap();

    let calls = provider.calls.lock().unwrap();
    assert_eq!(calls.len(), 2, "each leg must be its own routing request");
    assert_eq!(calls[0].len(), 2, "the outbound leg is a two-point request");
    assert_eq!(calls[1].len(), 2, "the return leg is a two-point request");
    // Reversed: the return leg starts where the outbound one ended.
    assert_eq!(calls[1][0], calls[0][1]);
    assert_eq!(calls[1][1], calls[0][0]);

    assert_eq!(result.outbound.len(), 2, "the outbound leg keeps its alternatives");
    assert_eq!(result.inbound.len(), 1);
}

#[tokio::test]
async fn the_return_leg_is_derived_when_the_caller_sends_none() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let result =
        route_round_trip_internal(&provider, direct_waypoints(), Vec::new(), 50.0, None)
            .await
            .unwrap();

    let out = &result.outbound_waypoints;
    let back = &result.inbound_waypoints;
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].lat, out[out.len() - 1].lat);
    assert_eq!(back[0].lon, out[out.len() - 1].lon);
    assert_eq!(back[1].lat, out[0].lat);
    assert_eq!(back[1].lon, out[0].lon);
}

#[tokio::test]
async fn the_two_legs_are_always_joined_even_when_the_caller_sends_them_apart() {
    // The user dragged the outbound leg's destination handle. The return leg
    // the browser still holds starts at the OLD point. The backend must not
    // route a pair that does not join.
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let stale_inbound = vec![
        Waypoint { lat: 1.0, lon: 1.0, name: None, node_idx: None },
        Waypoint { lat: 2.0, lon: 2.0, name: None, node_idx: None },
    ];

    let result = route_round_trip_internal(
        &provider,
        direct_waypoints(),
        stale_inbound,
        50.0,
        None,
    )
    .await
    .unwrap();

    let out = &result.outbound_waypoints;
    let back = &result.inbound_waypoints;
    assert_eq!(back[0].lat, out[out.len() - 1].lat);
    assert_eq!(back[back.len() - 1].lat, out[0].lat);
}

#[tokio::test]
async fn a_via_dropped_on_the_return_leg_stays_on_the_return_leg() {
    // The bug this task exists to remove: with one three-point request, a via
    // dragged onto the way home landed on the way out, because the search ran
    // against the open outbound list while the polyline was the closed line.
    let leg_points = vec![(48.9444, 20.5675), (48.55, 18.85), (48.1486, 17.1077)];
    let leg_polyline = encode(&leg_points);

    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched(&leg_polyline, 26.0, 1600.0)],
    );

    let inbound = vec![
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    let result = route_round_trip_internal(
        &provider,
        direct_waypoints(),
        inbound,
        50.0,
        Some(LegInsertPoint {
            lat: 48.55,
            lon: 18.85,
            polyline: leg_polyline.clone(),
            leg: Leg::Inbound,
        }),
    )
    .await
    .unwrap();

    assert_eq!(
        result.outbound_waypoints.len(),
        2,
        "the outbound leg must be untouched by a drag on the return leg"
    );
    assert_eq!(result.inbound_waypoints.len(), 3, "the via belongs to the return leg");
    assert!(result.inbound_waypoints[1].name.is_none(), "a dragged point is unnamed");
}

#[tokio::test]
async fn the_deviation_of_a_pair_is_measured_against_the_two_legs_together() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0), fetched("out2", 30.0, 1400.0)],
        vec![fetched("back1", 25.0, 1600.0)],
    );

    let result =
        route_round_trip_internal(&provider, direct_waypoints(), Vec::new(), 50.0, None)
            .await
            .unwrap();

    assert_eq!(result.combined.len(), 2);
    assert_eq!(result.combined[0].len(), 1);
    // 25 + 25 against a 50 km target is exact.
    assert!((result.combined[0][0].road_km - 50.0).abs() < 1e-9);
    assert!((result.combined[0][0].deviation_percent).abs() < 1e-9);
    assert!(!result.combined[0][0].off_target);
    // 30 + 25 is 55 km, ten percent long.
    assert!((result.combined[1][0].road_km - 55.0).abs() < 1e-9);
    assert!((result.combined[1][0].deviation_percent - 10.0).abs() < 1e-6);
    // Durations add too -- the panel shows the time of the whole journey.
    assert!((result.combined[0][0].duration_s - 3100.0).abs() < 1e-9);
}

#[tokio::test]
async fn a_leg_keeps_the_routing_services_own_order() {
    // ADR-038: fastest first, never re-sorted, even when a slower alternative
    // is closer to the target distance.
    let provider = TwoLegProvider::new(
        vec![fetched("fast", 40.0, 1000.0), fetched("slow", 25.0, 2000.0)],
        vec![fetched("back1", 25.0, 1600.0)],
    );

    let result =
        route_round_trip_internal(&provider, direct_waypoints(), Vec::new(), 50.0, None)
            .await
            .unwrap();

    assert_eq!(result.outbound[0].polyline, "fast");
    assert_eq!(result.outbound[1].polyline, "slow");
}

#[tokio::test]
async fn a_loop_row_cannot_be_routed_as_a_round_trip() {
    // A row whose two endpoints are one place is a Loop (`mode_for`), and a
    // loop has no second leg. The same first/last comparison ADR-041 uses:
    // coordinate AND name.
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let same_place = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    let err = route_round_trip_internal(&provider, same_place, Vec::new(), 50.0, None)
        .await
        .unwrap_err();
    assert!(err.contains("two different endpoints"), "got: {err}");
}

#[tokio::test]
async fn a_round_trip_needs_a_start_and_an_end() {
    let provider = TwoLegProvider::new(
        vec![fetched("out1", 25.0, 1500.0)],
        vec![fetched("back1", 26.0, 1600.0)],
    );

    let one = vec![Waypoint { lat: 48.1, lon: 17.1, name: None, node_idx: None }];
    let err = route_round_trip_internal(&provider, one, Vec::new(), 50.0, None)
        .await
        .unwrap_err();
    assert!(err.contains("start and an end"), "got: {err}");
}

#[test]
fn saving_a_round_trip_joins_the_two_legs_into_one_row() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let out_points = vec![(48.1486, 17.1077), (48.5, 18.0), (48.9444, 20.5675)];
    let back_points = vec![(48.9444, 20.5675), (48.6, 18.4), (48.1486, 17.1077)];

    let outbound_waypoints = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
    ];
    let inbound_waypoints = vec![
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
        Waypoint { lat: 48.6, lon: 18.4, name: None, node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    save_trip_round_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        outbound_waypoints,
        inbound_waypoints,
        encode(&out_points),
        encode(&back_points),
        25.0,
        27.0,
        50.0,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();

    // The shared turnaround point is stored once, not twice.
    assert_eq!(saved.waypoints.len(), 4);
    assert_eq!(saved.turnaround_index, Some(1));
    assert_eq!(saved.waypoints[1].name.as_deref(), Some("Spišská Nová Ves"));
    assert!(saved.round_trip);
    assert_eq!(saved.mode, RouteMode::Direct);

    // The distance is the sum of the two legs -- the backend adds it, not the
    // browser.
    assert!((saved.road_km - 52.0).abs() < 1e-9);

    // The geometry is the two legs, in order, as one line.
    assert_eq!(saved.coordinates.len(), out_points.len() + back_points.len());
    assert!((saved.coordinates[0][0] - 48.1486).abs() < 1e-4);
    assert!((saved.coordinates[3][0] - 48.9444).abs() < 1e-4);
}

#[test]
fn a_round_trip_save_refuses_a_leg_that_is_not_a_leg() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let one = vec![Waypoint { lat: 48.1, lon: 17.1, name: None, node_idx: None }];
    let two = direct_waypoints();

    let err = save_trip_round_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        one,
        two,
        "a".into(),
        "b".into(),
        1.0,
        1.0,
        2.0,
    )
    .unwrap_err();
    assert!(err.contains("two legs"), "got: {err}");
}

#[test]
fn a_round_trip_saved_before_the_index_existed_resolves_its_own_split_point() {
    // The legacy shape, written the way the old code wrote it: one clone of
    // the first waypoint appended to close the route, and no stored index.
    // The rule that recovers the split -- the outbound leg ended at `len - 2`
    // -- is a rule about how this application wrote its own data, so it is
    // resolved in Rust and never in the browser (ADR-008).
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    let mut closed = direct_waypoints();
    let via = Waypoint { lat: 48.5, lon: 18.0, name: None, node_idx: None };
    closed.insert(1, via);
    closed.push(closed[0].clone());

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        closed,
        polyline,
        trip.distance_km,
        120.0,
        RouteMode::Direct,
        true,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert_eq!(saved.waypoints.len(), 4, "[A, via, B, A]");
    assert_eq!(
        saved.turnaround_index,
        Some(2),
        "the outbound leg of a legacy round trip ended at len - 2"
    );
}

#[test]
fn a_one_way_saved_map_resolves_no_split_point() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline,
        trip.distance_km,
        120.0,
        RouteMode::Direct,
        false,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert_eq!(saved.turnaround_index, None);
}

#[test]
fn a_saved_map_reports_the_trips_distance_as_its_target() {
    // trip_routes.target_km records what the trip measured when the map was
    // saved. After a write-back (or any ordinary edit of the row) the trip's
    // distance moves, and a map still reporting the old target would show a
    // deviation against a distance the book no longer holds.
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let mut trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline,
        trip.distance_km,
        118.0,
        RouteMode::Direct,
        false,
    )
    .unwrap();

    trip.distance_km = 118.0;
    db.update_trip(&trip).unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert!((saved.target_km - 118.0).abs() < 1e-9);
    assert!(
        saved.deviation_percent.abs() < 1e-9,
        "a route whose distance now matches the trip has no deviation left"
    );
    assert!(!saved.off_target);
}

#[test]
fn a_saved_round_trip_returns_its_geometry_split_into_two_legs() {
    // The row stores one line -- the two legs concatenated. The map draws the
    // outbound leg and the return leg in different colours, and a waypoint
    // dropped on one of them is placed against that leg's own geometry, so
    // the split has to come back with the row. Deriving it in the browser
    // would need a polyline codec there (ADR-008), so Rust splits it.
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let out_points = vec![(48.1486, 17.1077), (48.5, 18.0), (48.9444, 20.5675)];
    let back_points = vec![(48.9444, 20.5675), (48.6, 18.4), (48.1486, 17.1077)];

    let outbound_waypoints = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
    ];
    let inbound_waypoints = vec![
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
        Waypoint { lat: 48.6, lon: 18.4, name: None, node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    save_trip_round_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        outbound_waypoints,
        inbound_waypoints,
        encode(&out_points),
        encode(&back_points),
        25.0,
        27.0,
        50.0,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    let legs = saved.legs.expect("a saved round trip carries its two legs");

    // Each leg is exactly the geometry that was saved for it. The turnaround
    // point belongs to both, so the two halves overlap by one point.
    assert_eq!(legs.outbound.coordinates.len(), out_points.len());
    assert_eq!(legs.inbound.coordinates.len(), back_points.len());
    assert!((legs.outbound.coordinates[0][0] - 48.1486).abs() < 1e-4);
    assert!((legs.outbound.coordinates[2][0] - 48.9444).abs() < 1e-4);
    assert!((legs.inbound.coordinates[0][0] - 48.9444).abs() < 1e-4);
    assert!((legs.inbound.coordinates[2][0] - 48.1486).abs() < 1e-4);

    // The polyline comes back too: a waypoint dropped on a leg is placed
    // against that leg's line, not against the whole round trip.
    assert_eq!(legs.outbound.polyline, encode(&out_points));
    assert_eq!(legs.inbound.polyline, encode(&back_points));
}

#[test]
fn a_saved_round_trip_splits_at_the_turnaround_even_when_the_legs_differ() {
    // The seam is found by the turnaround waypoint, not by halving the line:
    // the return leg here is longer than the way out.
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let out_points = vec![(48.1486, 17.1077), (48.9444, 20.5675)];
    let back_points = vec![
        (48.9444, 20.5675),
        (48.8, 20.0),
        (48.6, 19.0),
        (48.4, 18.0),
        (48.1486, 17.1077),
    ];

    save_trip_round_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        direct_waypoints(),
        vec![
            Waypoint { lat: 48.9444, lon: 20.5675, name: None, node_idx: None },
            Waypoint { lat: 48.1486, lon: 17.1077, name: None, node_idx: None },
        ],
        encode(&out_points),
        encode(&back_points),
        25.0,
        30.0,
        50.0,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    let legs = saved.legs.expect("a saved round trip carries its two legs");
    assert_eq!(legs.outbound.coordinates.len(), 2);
    assert_eq!(legs.inbound.coordinates.len(), 5);
}

#[test]
fn a_round_trip_saved_before_the_index_existed_still_splits_its_geometry() {
    // A legacy row's line has no seam -- it was one routing result. The split
    // still has to land on the turnaround the resolved index names, so the
    // return leg is drawn as a return leg.
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);

    let points = vec![
        (48.1486, 17.1077),
        (48.5, 18.0),
        (48.9444, 20.5675),
        (48.6, 18.4),
        (48.1486, 17.1077),
    ];

    // The legacy shape: one clone of the first waypoint appended, no index.
    let closed = vec![
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
        Waypoint { lat: 48.9444, lon: 20.5675, name: Some("Spišská Nová Ves".into()), node_idx: None },
        Waypoint { lat: 48.6, lon: 18.4, name: None, node_idx: None },
        Waypoint { lat: 48.1486, lon: 17.1077, name: Some("Bratislava".into()), node_idx: None },
    ];

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        closed,
        encode(&points),
        trip.distance_km,
        120.0,
        RouteMode::Direct,
        true,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    // len - 2 == 2, the via -- that is the legacy rule, already pinned above.
    assert_eq!(saved.turnaround_index, Some(2));
    let legs = saved.legs.expect("a legacy round trip carries its two legs too");
    // Split at the point nearest waypoint[2] == (48.6, 18.4), index 3.
    assert_eq!(legs.outbound.coordinates.len(), 4);
    assert_eq!(legs.inbound.coordinates.len(), 2);
}

#[test]
fn a_one_way_saved_map_carries_no_legs() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let trip = seed_trip(&db);
    let (_, polyline) = sample_geometry();

    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline,
        trip.distance_km,
        120.0,
        RouteMode::Direct,
        false,
    )
    .unwrap();

    let saved = get_trip_route_internal(&db, trip.id.to_string()).unwrap().unwrap();
    assert!(saved.legs.is_none(), "a one-way route has no return leg to colour");
}
