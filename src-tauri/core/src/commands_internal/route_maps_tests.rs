//! Tests for generated route map commands.
//!
//! Nothing here touches the network: [`StubProvider`] stands in for OSRM.

use super::*;
use crate::app_state::AppState;
use crate::db::Database;
use crate::export::RouteMapPage;
use crate::models::{Trip, Vehicle, Waypoint};
use crate::route_map::polyline::encode;
use crate::route_map::tiles::TileFetcher;
use crate::commands_internal::build_trip_grid_data;
use crate::route_map::{Dataset, FetchedRoute, RouteProvider};
use chrono::NaiveDate;
use uuid::Uuid;

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
    let trip = seed_trip(&db);
    let app_state = AppState::new();
    let (_, polyline) = sample_geometry();

    // 100 km target, 108 km of road: 8% out, beyond the 5% tolerance.
    save_trip_route_internal(
        &db,
        &app_state,
        trip.id.to_string(),
        sample_waypoints(),
        polyline.clone(),
        100.0,
        108.0,
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
    )
    .unwrap();

    let loaded = get_trip_route_internal(&db, trip.id.to_string())
        .unwrap()
        .expect("must exist");
    assert!((loaded.deviation_percent - 2.0).abs() < 1e-9);
    assert!(!loaded.off_target, "2% must be within tolerance");
}
