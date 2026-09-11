//! Boundary tests for the Paperless invoice commands (Task 64, collapsed 84).
use super::*;
use crate::app_state::AppState;
use crate::db::Database;
use crate::db_tests;
use crate::models::AssignmentType;
use crate::paperless::PaperlessDoc;
use chrono::NaiveDate;

fn paperless_doc_fuel() -> PaperlessDoc {
    PaperlessDoc {
        id: 435,
        title: "Doc 435".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        total_amount: Some(58.20),
        litres: Some(40.5),
        receipt_datetime: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(12, 0, 0),
    }
}

fn paperless_doc_other() -> PaperlessDoc {
    PaperlessDoc {
        id: 435,
        title: "Iná cena".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        total_amount: Some(15.00),
        litres: None,
        receipt_datetime: None,
    }
}

#[test]
fn get_trips_for_paperless_assignment_returns_trips() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    db_tests::seed_test_trip(&db, &v.id.to_string());

    let result = get_trips_for_paperless_assignment_internal(
        &db,
        &paperless_doc_fuel(),
        &v.id.to_string(),
        2026,
    )
    .unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn assign_paperless_populates_trip_fuel_when_empty() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let doc = paperless_doc_fuel();
    assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Fuel,
        false,
    )
    .unwrap();
    let trip = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(trip.fuel_liters, Some(40.5));
    assert_eq!(trip.fuel_cost_eur, Some(58.20));
    let link = db.get_paperless_link(435).unwrap().expect("link created");
    assert_eq!(link.trip_id, trip_id);
    assert_eq!(link.assignment_type, AssignmentType::Fuel);
    // Snapshots taken from the backend-fetched doc at assign time
    assert_eq!(link.amount_eur, Some(58.20));
    assert_eq!(link.title, Some("Doc 435".to_string()));
}

#[test]
fn assign_paperless_invoice_blocked_when_read_only() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    app_state.enable_read_only("test");
    let doc = paperless_doc_other();
    let err = assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Other,
        false,
    )
    .unwrap_err();
    let lower = err.to_lowercase();
    assert!(
        lower.contains("read") || lower.contains("čítanie"),
        "expected read-only error, got: {}",
        err
    );
}

// Fix 2: vehicle ownership check
#[test]
fn assign_paperless_rejects_trip_from_different_vehicle() {
    let db = Database::in_memory().unwrap();
    let v1 = db_tests::create_test_vehicle("Vehicle1");
    db.create_vehicle(&v1).unwrap();
    let v2 = db_tests::create_test_vehicle("Vehicle2");
    db.create_vehicle(&v2).unwrap();
    // Trip belongs to v1, but we pass v2 as the vehicle_id
    let trip_id = db_tests::seed_test_trip(&db, &v1.id.to_string());
    let app_state = AppState::new();
    let doc = paperless_doc_fuel();
    let err = assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v2.id.to_string(),
        AssignmentType::Fuel,
        false,
    )
    .unwrap_err();
    assert!(
        err.to_lowercase().contains("vehicle"),
        "expected vehicle mismatch error, got: {}",
        err
    );
}

#[test]
fn assign_paperless_persists_datetime_and_override() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let dt = chrono::NaiveDateTime::parse_from_str("2026-05-04T13:24:14", "%Y-%m-%dT%H:%M:%S")
        .unwrap();
    let doc = PaperlessDoc {
        id: 435,
        title: "Fuel".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(63.34),
        litres: Some(40.0),
        receipt_datetime: Some(dt),
    };
    assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Fuel,
        true,
    )
    .unwrap();
    let link = db.get_paperless_link(435).unwrap().unwrap();
    assert_eq!(link.receipt_datetime, Some(dt));
    assert!(link.mismatch_override);
}

#[test]
fn revert_paperless_override_clears_the_flag() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    let dt = chrono::NaiveDateTime::parse_from_str("2026-05-04T13:24:14", "%Y-%m-%dT%H:%M:%S")
        .unwrap();
    let doc = PaperlessDoc {
        id: 435,
        title: "Fuel".into(),
        tag_ids: vec![],
        created: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(63.34),
        litres: Some(40.0),
        receipt_datetime: Some(dt),
    };
    assign_paperless_invoice_internal(
        &db,
        &app_state,
        &doc,
        &trip_id,
        &v.id.to_string(),
        AssignmentType::Fuel,
        true,
    )
    .unwrap();
    revert_paperless_override_internal(&db, &app_state, 435).unwrap();
    let link = db.get_paperless_link(435).unwrap().unwrap();
    assert!(!link.mismatch_override);
}

#[test]
fn unassign_dispatches_paperless_source() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = db_tests::seed_test_trip(&db, &v.id.to_string());
    let app_state = AppState::new();
    db.upsert_paperless_link(&crate::models::PaperlessLink {
        paperless_document_id: 435,
        trip_id: trip_id.clone(),
        assignment_type: AssignmentType::Other,
        amount_eur: Some(15.00),
        title: Some("Iná cena".to_string()),
        applied_amount_cents: None,
        receipt_datetime: None,
        mismatch_override: false,
    })
    .unwrap();
    unassign_paperless_invoice_internal(&db, &app_state, 435).unwrap();
    assert!(db.get_paperless_link(435).unwrap().is_none());
}

// ============================================================================
// Other-cost note append/strip and unassign edge cases (ported from the
// deleted local-receipt coverage in task 84).
// ============================================================================

/// Seed a trip with a known `other_costs_eur` / `other_costs_note` and return
/// its id.
fn seed_other_trip(
    db: &Database,
    vehicle_id: &str,
    eur: Option<f64>,
    note: Option<&str>,
) -> String {
    let trip_id = db_tests::seed_test_trip(db, vehicle_id);
    let mut trip = db.get_trip(&trip_id).unwrap().unwrap();
    trip.other_costs_eur = eur;
    trip.other_costs_note = note.map(|s| s.to_string());
    db.update_trip(&trip).unwrap();
    trip_id
}

#[test]
fn other_assign_appends_title_to_note_and_unassign_strips_it() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = seed_other_trip(&db, &v.id.to_string(), Some(10.0), Some("Manual note"));
    let trip = db.get_trip(&trip_id).unwrap().unwrap();

    let applied = apply_other_amount(&db, &trip, Some(5.0), "AutoWash", false).unwrap();
    assert_eq!(applied, Some(500), "5.00 EUR applied in cents");

    let updated = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(updated.other_costs_eur, Some(15.0));
    assert_eq!(
        updated.other_costs_note.as_deref(),
        Some("Manual note; AutoWash"),
        "the doc title is appended as a note segment"
    );

    remove_other_contribution(&db, &trip_id, 500, Some("AutoWash")).unwrap();
    let restored = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(restored.other_costs_eur, Some(10.0), "restored bit-exact");
    assert_eq!(restored.other_costs_note.as_deref(), Some("Manual note"));
}

#[test]
fn other_assign_populates_empty_note_with_title_then_unassign_clears_both() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = seed_other_trip(&db, &v.id.to_string(), None, None);
    let trip = db.get_trip(&trip_id).unwrap().unwrap();

    apply_other_amount(&db, &trip, Some(15.0), "Toll", false).unwrap();
    let updated = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(updated.other_costs_eur, Some(15.0));
    assert_eq!(updated.other_costs_note.as_deref(), Some("Toll"));

    remove_other_contribution(&db, &trip_id, 1500, Some("Toll")).unwrap();
    let restored = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(
        restored.other_costs_eur, None,
        "zero result stored as None, not Some(0.0)"
    );
    assert_eq!(restored.other_costs_note, None, "whole-note segment removed");
}

#[test]
fn strip_note_segment_removes_whole_suffix_and_prefix() {
    assert_eq!(strip_note_segment(Some("Toll".into()), "Toll"), None);
    assert_eq!(
        strip_note_segment(Some("Manual; Toll".into()), "Toll").as_deref(),
        Some("Manual"),
        "suffix segment removed"
    );
    assert_eq!(
        strip_note_segment(Some("Toll; Manual".into()), "Toll").as_deref(),
        Some("Manual"),
        "prefix segment removed"
    );
    assert_eq!(
        strip_note_segment(Some("Something else entirely".into()), "Toll").as_deref(),
        Some("Something else entirely"),
        "edited note left untouched"
    );
}

#[test]
fn unassign_leaves_manually_edited_note_untouched() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = seed_other_trip(&db, &v.id.to_string(), Some(10.0), Some("Manual note"));
    let trip = db.get_trip(&trip_id).unwrap().unwrap();

    apply_other_amount(&db, &trip, Some(5.0), "AutoWash", false).unwrap();

    // User edits the note after assigning, dropping the appended segment.
    let mut edited = db.get_trip(&trip_id).unwrap().unwrap();
    edited.other_costs_note = Some("Something else entirely".to_string());
    db.update_trip(&edited).unwrap();

    remove_other_contribution(&db, &trip_id, 500, Some("AutoWash")).unwrap();
    let restored = db.get_trip(&trip_id).unwrap().unwrap();
    assert_eq!(
        restored.other_costs_note.as_deref(),
        Some("Something else entirely"),
        "user-edited note left untouched"
    );
    assert_eq!(restored.other_costs_eur, Some(10.0));
}

#[test]
fn unassign_after_manual_overwrite_below_applied_clamps_to_none() {
    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = seed_other_trip(&db, &v.id.to_string(), Some(10.0), None);
    let trip = db.get_trip(&trip_id).unwrap().unwrap();

    apply_other_amount(&db, &trip, Some(5.01), "Parking", false).unwrap();
    assert_eq!(db.get_trip(&trip_id).unwrap().unwrap().other_costs_eur, Some(15.01));

    // User hand-edits the total below the applied snapshot.
    let mut edited = db.get_trip(&trip_id).unwrap().unwrap();
    edited.other_costs_eur = Some(3.0);
    db.update_trip(&edited).unwrap();

    remove_other_contribution(&db, &trip_id, 501, Some("Parking")).unwrap();
    assert_eq!(
        db.get_trip(&trip_id).unwrap().unwrap().other_costs_eur,
        None,
        "clamped-to-zero result stored as None"
    );
}

#[test]
fn remove_other_contribution_tolerates_orphaned_link() {
    let db = Database::in_memory().unwrap();
    // No trip exists with this id: an orphaned link must be a no-op, not an error.
    remove_other_contribution(&db, "00000000-0000-0000-0000-0000000000ff", 500, Some("Toll"))
        .expect("orphaned link must not error");
}

#[test]
fn unassign_orphaned_paperless_link_succeeds_without_trip() {
    use diesel::RunQueryDsl;

    let db = Database::in_memory().unwrap();
    let v = db_tests::create_test_vehicle("Test");
    db.create_vehicle(&v).unwrap();
    let trip_id = seed_other_trip(&db, &v.id.to_string(), Some(10.0), None);
    let app_state = AppState::new();
    db.upsert_paperless_link(&crate::models::PaperlessLink {
        paperless_document_id: 435,
        trip_id: trip_id.clone(),
        assignment_type: AssignmentType::Other,
        amount_eur: Some(5.0),
        title: Some("Parking".to_string()),
        applied_amount_cents: Some(500),
        receipt_datetime: None,
        mismatch_override: false,
    })
    .unwrap();

    // Orphan the link the way production data got orphaned (trips deleted by
    // app versions predating FK enforcement): delete with FKs suspended.
    {
        let conn = &mut *db.connection();
        diesel::sql_query("PRAGMA foreign_keys = OFF").execute(conn).unwrap();
        diesel::sql_query(format!("DELETE FROM trips WHERE id = '{}'", trip_id))
            .execute(conn)
            .unwrap();
        diesel::sql_query("PRAGMA foreign_keys = ON").execute(conn).unwrap();
    }
    assert!(db.get_trip(&trip_id).unwrap().is_none(), "trip gone");

    unassign_paperless_invoice_internal(&db, &app_state, 435)
        .expect("orphaned link unassign must not error");
    assert!(db.get_paperless_link(435).unwrap().is_none(), "link deleted");
}
