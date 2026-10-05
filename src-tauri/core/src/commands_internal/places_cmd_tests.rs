//! Tests for the place commands. Task 88 makes a place an entity; the
//! migration tests own the spelling fold, and Task 4 adds the tests for the
//! place write commands.

use super::*;
use crate::app_state::AppState;
use crate::db_tests::{create_test_vehicle, seed_trip_between};
use crate::models::PlaceSource;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Geocoding a place
// ---------------------------------------------------------------------------

/// A geocoder with no HTTP stack underneath it at all, recording what it was
/// asked.
///
/// This is what `GeocodeProvider` buys. `geocode_tests.rs` already proves the
/// Nominatim client itself, but every one of those tests has to stand up a
/// wiremock server to do it — they exercise the concrete client, not the seam.
/// Answering here proves the command reaches its geocoder through the trait,
/// which is what lets anything other than Nominatim answer it.
struct StubGeocoder {
    answer: Result<Vec<Candidate>, String>,
    asked: Mutex<Vec<String>>,
}

impl StubGeocoder {
    fn answering(candidates: Vec<Candidate>) -> Self {
        Self {
            answer: Ok(candidates),
            asked: Mutex::new(Vec::new()),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            answer: Err(message.to_string()),
            asked: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl GeocodeProvider for StubGeocoder {
    async fn search(&self, query: &str) -> Result<Vec<Candidate>, String> {
        self.asked.lock().unwrap().push(query.to_string());
        self.answer.clone()
    }
}

#[tokio::test]
async fn geocoding_asks_the_provider_and_returns_its_candidates_in_order() {
    let provider = StubGeocoder::answering(vec![
        Candidate {
            lat: 48.1485965,
            lon: 17.1077477,
            label: "Hlavná stanica, Bratislava, Slovensko".into(),
        },
        Candidate {
            lat: 48.7,
            lon: 21.2,
            label: "Hlavná stanica, Košice, Slovensko".into(),
        },
    ]);

    let candidates = geocode_place_internal(&provider, "Hlavná stanica".to_string())
        .await
        .expect("the stub answers");

    assert_eq!(
        provider.asked.lock().unwrap().as_slice(),
        ["Hlavná stanica"],
        "the address must reach the geocoder verbatim, and exactly once — \
         normalising it here would search for a spelling nobody typed"
    );
    // Best match first. The order is the geocoder's own ranking and the
    // command must hand it on untouched: the dialog preselects the first.
    let labels: Vec<&str> = candidates.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Hlavná stanica, Bratislava, Slovensko",
            "Hlavná stanica, Košice, Slovensko"
        ]
    );
    assert_eq!(candidates[0].lat, 48.1485965);
    assert_eq!(candidates[0].lon, 17.1077477);
}

#[tokio::test]
async fn a_provider_failure_is_an_error_not_an_empty_list() {
    // An empty list already means "place it by hand", so swallowing a failure
    // into one would tell the user the geocoder knows of no such place when in
    // fact it was never asked successfully.
    let provider = StubGeocoder::failing("Geocoding service returned HTTP 429 (Too Many Requests)");

    let err = geocode_place_internal(&provider, "Bratislava".to_string())
        .await
        .expect_err("a provider failure must surface as an error");

    assert_eq!(
        err, "Geocoding service returned HTTP 429 (Too Many Requests)",
        "the provider's own message must reach the caller unrewritten"
    );
}

// ---------------------------------------------------------------------------
// Place write commands
// ---------------------------------------------------------------------------

#[test]
fn a_place_needs_a_name() {
    let db = Database::in_memory().unwrap();
    let err = create_place_internal(&db, &AppState::new(), "   ".into(), 48.1, 17.1, PlaceSource::Manual).unwrap_err();
    assert_eq!(err, "Miesto musí mať názov");
}

#[test]
fn create_returns_the_place_with_an_id_and_zero_uses() {
    let db = Database::in_memory().unwrap();
    let p = create_place_internal(&db, &AppState::new(), " Nitra ".into(), 48.3, 18.1, PlaceSource::Geocoder).unwrap();
    assert_eq!(p.name, "Nitra", "the name is trimmed");
    assert_eq!(p.uses, 0);
    assert_eq!(list_places_internal(&db).unwrap().len(), 1);
}

#[test]
fn a_second_place_with_the_same_key_is_refused() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    create_place_internal(&db, &app, "Košice".into(), 48.7, 21.2, PlaceSource::Manual).unwrap();
    let err = create_place_internal(&db, &app, "kosice".into(), 48.7, 21.2, PlaceSource::Manual).unwrap_err();
    assert!(err.contains("Košice"), "the error names the existing place: {err}");
}

#[test]
fn a_rename_that_only_changes_case_or_diacritics_is_allowed() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    let p = create_place_internal(&db, &app, "Kosice".into(), 48.7, 21.2, PlaceSource::Manual).unwrap();
    let renamed = rename_place_internal(&db, &app, p.id.to_string(), "Košice".into()).unwrap();
    assert_eq!(renamed.name, "Košice");
}

#[test]
fn a_rename_onto_another_place_is_refused() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    create_place_internal(&db, &app, "Nitra".into(), 48.3, 18.1, PlaceSource::Manual).unwrap();
    let b = create_place_internal(&db, &app, "Levice".into(), 48.2, 18.6, PlaceSource::Manual).unwrap();
    let err = rename_place_internal(&db, &app, b.id.to_string(), "NITRA".into()).unwrap_err();
    assert!(err.contains("Nitra"), "{err}");
}

#[test]
fn a_place_a_trip_uses_cannot_be_deleted() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let trip = seed_trip_between(&db, &vehicle.id, "Nitra", "Levice");
    let err = delete_place_internal(&db, &AppState::new(), trip.origin_place_id.to_string()).unwrap_err();
    assert!(err.contains('1'), "the error gives the trip count: {err}");
}

#[test]
fn a_place_only_an_orphan_route_uses_can_be_deleted() {
    let db = Database::in_memory().unwrap();
    let vehicle = create_test_vehicle("Car");
    db.create_vehicle(&vehicle).unwrap();
    let v = vehicle.id.to_string();
    let a = db.ensure_place_for_test("Nitra").to_string();
    let b = db.ensure_place_for_test("Levice").to_string();
    db.find_or_create_route(&v, &a, &b, 40.0).unwrap();
    delete_place_internal(&db, &AppState::new(), a.clone()).unwrap();
    assert!(db.get_place(&a).unwrap().is_none());
    assert!(db.all_route_rows_for_test(&v).unwrap().is_empty(), "the orphan route goes with it");
}

#[test]
fn find_place_folds_case_and_diacritics() {
    let db = Database::in_memory().unwrap();
    create_place_internal(&db, &AppState::new(), "Bratislava".into(), 48.1, 17.1, PlaceSource::Manual).unwrap();
    let found = find_place_internal(&db, "  bratislava ".into()).unwrap().unwrap();
    assert_eq!(found.name, "Bratislava");
    assert!(find_place_internal(&db, "Senec".into()).unwrap().is_none());
}

#[test]
fn set_position_moves_the_place_and_keeps_its_id() {
    let db = Database::in_memory().unwrap();
    let app = AppState::new();
    let p = create_place_internal(&db, &app, "Nitra".into(), 48.3, 18.1, PlaceSource::Geocoder).unwrap();
    let moved = set_place_position_internal(&db, &app, p.id.to_string(), 48.31, 18.09, PlaceSource::Manual).unwrap();
    assert_eq!((moved.id, moved.lat, moved.source), (p.id, Some(48.31), Some(PlaceSource::Manual)));
}

/// The only test of the "needs a position" marker data. No RPC can make an
/// unplaced place (only the migration does), and a test-only RPC must not
/// exist on a live server, so the marker itself has no integration test.
#[test]
fn an_unplaced_place_is_listed_first_with_null_coordinates() {
    let db = Database::in_memory().unwrap();
    create_place_internal(&db, &AppState::new(), "Nitra".into(), 48.3, 18.1, PlaceSource::Manual).unwrap();
    db.ensure_unplaced_place_for_test("Stará adresa");
    let list = list_places_internal(&db).unwrap();
    assert_eq!(list[0].name, "Stará adresa");
    assert_eq!((list[0].lat, list[0].source), (None, None));
}

#[test]
fn place_writes_are_refused_in_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let ro = AppState::new();
    ro.enable_read_only("Test read-only");
    let err = create_place_internal(&db, &ro, "Nitra".into(), 48.3, 18.1, PlaceSource::Manual).unwrap_err();
    assert!(err.contains("len na čítanie"), "got: {err}");
    assert!(db.all_places().unwrap().is_empty(), "a read-only create must not write");
}

// ---------------------------------------------------------------------------
// Home mark (task 89)
// ---------------------------------------------------------------------------

#[test]
fn set_home_place_internal_marks_and_list_places_reports_it() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let id = db.ensure_place_for_test("Home St 1, Hometown");

    set_home_place_internal(&db, &app_state, Some(id.to_string())).unwrap();

    let places = list_places_internal(&db).unwrap();
    let home: Vec<_> = places.iter().filter(|p| p.is_home).collect();
    assert_eq!(home.len(), 1);
    assert_eq!(home[0].id, id);
}

#[test]
fn set_home_place_internal_unknown_id_is_an_error() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let err = set_home_place_internal(&db, &app_state, Some(uuid::Uuid::new_v4().to_string()));
    assert_eq!(err.unwrap_err(), "Place not found");
}

#[test]
fn set_home_place_internal_refuses_in_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    app_state.enable_read_only("test");
    let id = db.ensure_place_for_test("Home St 1, Hometown");
    assert!(set_home_place_internal(&db, &app_state, Some(id.to_string())).is_err());
    assert!(db.get_home_place().unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Two writes that race for one name
// ---------------------------------------------------------------------------

#[test]
fn a_unique_violation_on_the_name_gives_the_slovak_message() {
    // The free-name check and the write are two DB calls. If another write
    // takes the name between them, the UNIQUE index refuses the second write.
    // The user must see the same message as for a normal collision, not the
    // raw "UNIQUE constraint failed" text (code review, 2026-10-05).
    let db = Database::in_memory().unwrap();
    db.ensure_place_for_test("Košice");
    let key = normalise("Košice");
    let raced = db
        .insert_place(&NewPlaceRow {
            id: "raced",
            name: "Kosice",
            normalised_name: &key,
            lat: None,
            lon: None,
            source: None,
            created_at: "2026-10-05T00:00:00Z",
        })
        .unwrap_err();

    let msg = write_error(&db, &key, None, raced);

    assert_eq!(msg, "Miesto s týmto názvom už existuje: Košice");
}
