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

    // One entry, both trips counted, and never two rows.
    let kosice: Vec<_> = places
        .iter()
        .filter(|p| p.normalised_name == "kosice")
        .collect();
    assert_eq!(kosice.len(), 1, "two spellings, one place: {places:?}");
    assert_eq!(kosice[0].uses, 2);

    // The two spellings are used once each, so this is the tie case, settled on
    // the spelling itself: byte-wise "KOŠICE" < "Kosice", because 'O' (0x4F)
    // precedes 'o' (0x6F). Pinning the exact winner keeps the label from
    // flipping between two reads of the same book.
    //
    // Measured, not assumed: reversing the tie rule fails this assertion, but
    // DELETING it does not, because `distinct_trip_places` emits groups
    // ascending by `raw`, so the byte-smaller spelling arrives first and
    // first-in-wins agrees with the rule. Nothing at this level can separate
    // the two while that query has no ORDER BY.
    assert_eq!(
        kosice[0].display_name, "KOŠICE",
        "equally used spellings are settled on the spelling, not on arrival order"
    );
}

#[test]
fn the_most_used_spelling_is_the_one_displayed() {
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    // Three spellings of one place, used once, twice and three times.
    //
    // The fixture's power depends on the order `distinct_trip_places` returns:
    // its `GROUP BY raw` has no ORDER BY, and SQLite happens to emit groups
    // ascending by `raw` under BINARY collation, which puts "Košice" — the
    // three-use spelling — last. The bug this pins (comparing a spelling's
    // count against the running total rather than the leading spelling's own
    // count) only shows when the most-used spelling arrives late; it survives
    // two of the six arrival orders. So if that query ever grows an ORDER BY,
    // recheck this fixture rather than trusting a green run.
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
fn the_list_is_ordered_unplaced_first_then_most_used_then_by_name() {
    // The list is a worklist, and all three sort keys have to be here: what
    // still needs a pin comes first, then the places most worth pinning, then
    // the name.
    //
    // That last key is not cosmetic. The entries come out of a `HashMap`, whose
    // iteration order Rust randomises per process, and `sort_by` is stable — so
    // without the name key, places tied on placement and use count would come
    // back in a different order on every run. Three are tied here, so dropping
    // it leaves only a one-in-six chance of the assertion passing anyway.
    let db = Database::in_memory().unwrap();
    let v = create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    // Placed: 3 uses, the most-used place of all — and still last, because it
    // is the only one that already has a pin.
    seed_trip_between(&db, &v.id, "Placed", "Bratislava");
    seed_trip_between(&db, &v.id, "Placed", "Bratislava");
    seed_trip_between(&db, &v.id, "Placed", "Trnava");
    // Bratislava, Trnava and Zvolen end on 2 uses each; Rare on 1.
    seed_trip_between(&db, &v.id, "Trnava", "Zvolen");
    seed_trip_between(&db, &v.id, "Zvolen", "Rare");
    place_at(&db, "Placed", 48.1, 17.1, PlaceSource::Manual);

    let order: Vec<String> = list_places_internal(&db)
        .unwrap()
        .into_iter()
        .map(|p| p.display_name)
        .collect();

    assert_eq!(
        order,
        ["Bratislava", "Trnava", "Zvolen", "Rare", "Placed"],
        "unplaced before placed, then uses descending, then name ascending"
    );
}
