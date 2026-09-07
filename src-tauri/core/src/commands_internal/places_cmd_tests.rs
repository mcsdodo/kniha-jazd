//! Tests for the place book's derived list.
//!
//! Coordinates are seeded through `db.upsert_place` rather than a command:
//! the write commands arrive with Task 4, and this list must not depend on
//! them to be provable.

use super::*;
use crate::db::Database;
use crate::db_tests::{create_test_vehicle, seed_trip_between};
use crate::models::{NewPlaceRow, PlaceSource};
use crate::places::normalise;

/// Store a coordinate for `display_name` the way Task 4's command will.
fn place_at(db: &Database, display_name: &str, lat: f64, lon: f64, source: PlaceSource) {
    db.upsert_place(&NewPlaceRow {
        normalised_name: &normalise(display_name),
        display_name,
        lat: Some(lat),
        lon: Some(lon),
        source: source.as_str(),
    })
    .expect("store a coordinate");
}

#[test]
fn lists_every_place_a_trip_names_with_its_use_count() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip_between(&db, &v.id, "Office, City A", "Depot, City B");
    seed_trip_between(&db, &v.id, "Depot, City B", "Office, City A");
    seed_trip_between(&db, &v.id, "Office, City A", "Depot, City B");

    let places = list_places_internal(&db).unwrap();

    assert_eq!(places.len(), 2, "three trips name two distinct places");
    let office = places
        .iter()
        .find(|p| p.display_name == "Office, City A")
        .expect("Office, City A must be listed");
    assert_eq!(office.uses, 3, "twice an origin, once a destination");
    let depot = places
        .iter()
        .find(|p| p.display_name == "Depot, City B")
        .expect("Depot, City B must be listed");
    assert_eq!(depot.uses, 3, "twice a destination, once an origin");
    assert!(
        office.lat.is_none() && office.lon.is_none(),
        "a place nobody has placed has no coordinates"
    );
    assert!(
        office.source.is_none(),
        "and no source to explain a coordinate it does not have"
    );
}

#[test]
fn a_place_with_coordinates_reports_them() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip_between(&db, &v.id, "Office, City A", "Depot, City B");
    place_at(&db, "Office, City A", 48.1, 17.1, PlaceSource::Geocoder);

    let places = list_places_internal(&db).unwrap();

    let office = places
        .iter()
        .find(|p| p.display_name == "Office, City A")
        .expect("Office, City A must be listed");
    assert_eq!(office.lat, Some(48.1));
    assert_eq!(office.lon, Some(17.1));
    assert_eq!(office.source, Some(PlaceSource::Geocoder));

    let depot = places
        .iter()
        .find(|p| p.display_name == "Depot, City B")
        .expect("Depot, City B must be listed");
    assert!(
        depot.lat.is_none(),
        "one place's coordinate must not leak onto another"
    );
}

#[test]
fn spellings_that_normalise_alike_are_one_place() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip_between(&db, &v.id, "Kosice", "Depot, City B");
    seed_trip_between(&db, &v.id, "KOŠICE", "Depot, City B");

    let places = list_places_internal(&db).unwrap();

    // One entry, both trips counted. The displayed spelling is the most-used
    // one — arbitrary between equals, but never both.
    let kosice: Vec<_> = places
        .iter()
        .filter(|p| p.normalised_name == "kosice")
        .collect();
    assert_eq!(kosice.len(), 1, "two spellings, one place: {places:?}");
    assert_eq!(kosice[0].uses, 2);
    assert!(
        kosice[0].display_name == "Kosice" || kosice[0].display_name == "KOŠICE",
        "the displayed name is one of the spellings trips use, got {:?}",
        kosice[0].display_name
    );
}

#[test]
fn the_most_used_spelling_is_the_one_displayed() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    // Three spellings of one place, each used a different number of times.
    // "Kosice" is rarest, "KOSICE " (whitespace only) is commonest.
    seed_trip_between(&db, &v.id, "Kosice", "Depot, City B");
    seed_trip_between(&db, &v.id, "KOŠICE", "Depot, City B");
    seed_trip_between(&db, &v.id, "KOŠICE", "Depot, City B");
    seed_trip_between(&db, &v.id, "Košice", "Depot, City B");
    seed_trip_between(&db, &v.id, "Košice", "Depot, City B");
    seed_trip_between(&db, &v.id, "Košice", "Depot, City B");

    let places = list_places_internal(&db).unwrap();

    let kosice = places
        .iter()
        .find(|p| p.normalised_name == "kosice")
        .expect("one entry for the three spellings");
    assert_eq!(kosice.uses, 6);
    assert_eq!(
        kosice.display_name, "Košice",
        "the spelling used three times wins over ones used twice and once"
    );
}

#[test]
fn an_orphan_place_row_is_not_listed() {
    // A places row whose trips were all deleted keeps its coordinates on disk
    // but must not appear — the list is what trips say, not what the table holds.
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    let trip = seed_trip_between(&db, &v.id, "Nowhere, City Z", "Depot, City B");
    place_at(&db, "Nowhere, City Z", 1.0, 2.0, PlaceSource::Manual);
    db.delete_trip(&trip.id.to_string()).unwrap();

    let places = list_places_internal(&db).unwrap();

    assert!(
        places.is_empty(),
        "the only trip is gone, so no place is named any more: {places:?}"
    );
    assert_eq!(
        db.all_places().unwrap().len(),
        1,
        "the orphaned coordinate stays on disk — it is simply not listed"
    );
}

#[test]
fn unplaced_places_sort_ahead_of_placed_ones() {
    // The list is a worklist: what still needs a pin comes first, and within
    // each group the most-used place is the most worth placing.
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    seed_trip_between(&db, &v.id, "Placed", "Rare");
    seed_trip_between(&db, &v.id, "Placed", "Common");
    seed_trip_between(&db, &v.id, "Placed", "Common");
    place_at(&db, "Placed", 48.1, 17.1, PlaceSource::Manual);

    let order: Vec<String> = list_places_internal(&db)
        .unwrap()
        .into_iter()
        .map(|p| p.display_name)
        .collect();

    assert_eq!(order, ["Common", "Rare", "Placed"]);
}
