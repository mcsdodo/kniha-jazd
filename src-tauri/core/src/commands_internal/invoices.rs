//! Paperless invoice command implementations (Task 64, collapsed in Task 84).
//!
//! Paperless is the only invoice source. These three boundary functions are
//! what the RPC dispatchers call; everything below them is source-free.

use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::calculations::{from_cents, money_add, money_sub, to_cents};
use crate::check_read_only;
use crate::db::Database;
use crate::invoice::check_paperless_trip_compatibility;
use crate::models::{AssignmentType, Trip, TripInvoiceCoverage};
use crate::paperless::PaperlessDoc;

/// Rule 3 error: a trip can hold at most ONE Fuel invoice. Translated
/// frontend-side (i18n).
pub(crate) const FUEL_INVOICE_EXISTS_ERR: &str = "Trip already has a fuel invoice";

/// A trip annotated with whether a Paperless document can be attached to it.
/// Used by the frontend to show which trips are eligible for assignment.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TripForAssignment {
    pub trip: Trip,
    /// Whether this document can be attached to this trip
    pub can_attach: bool,
    /// Status explaining why: "empty" (no fuel), "matches" (document matches trip fuel), "differs" (data conflicts)
    pub attachment_status: String,
    /// When status is "differs", explains what specifically doesn't match (for UI display)
    /// Values: null, "date", "liters", "price", "liters_and_price", "date_and_liters", "date_and_price", "all"
    pub mismatch_reason: Option<String>,
}

/// Get trips annotated with attachment status for a Paperless document.
/// The document is always backend-fetched; inline caller data is never trusted.
pub fn get_trips_for_paperless_assignment_internal(
    db: &Database,
    doc: &PaperlessDoc,
    vehicle_id: &str,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    let trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Fetched ONCE for the whole picker list (Task 66) -- per-trip entries are
    // passed down so the compat check can enforce multi-invoice rules.
    let coverage = db.get_trip_invoice_coverage().map_err(|e| e.to_string())?;

    Ok(annotate_trips(doc, trips, &coverage))
}

fn annotate_trips(
    doc: &PaperlessDoc,
    trips: Vec<Trip>,
    coverage: &HashMap<String, TripInvoiceCoverage>,
) -> Vec<TripForAssignment> {
    let no_coverage = TripInvoiceCoverage::default();
    trips
        .into_iter()
        .map(|trip| {
            let trip_coverage = coverage.get(&trip.id.to_string()).unwrap_or(&no_coverage);
            let compat = check_paperless_trip_compatibility(doc, &trip, trip_coverage);
            TripForAssignment {
                trip,
                can_attach: compat.can_attach,
                attachment_status: compat.status,
                mismatch_reason: compat.mismatch_reason,
            }
        })
        .collect()
}

/// Assign a Paperless document to a trip.
/// `doc` must be backend-fetched (never trust caller-supplied data for writes).
#[allow(clippy::too_many_arguments)]
pub fn assign_paperless_invoice_internal(
    db: &Database,
    app_state: &AppState,
    doc: &PaperlessDoc,
    trip_id: &str,
    vehicle_id: &str,
    assignment_type: AssignmentType,
    mismatch_override: bool,
) -> Result<(), String> {
    check_read_only!(app_state);

    let id = doc.id;

    // Rule 1: validate the backend-fetched amount BEFORE any mutation.
    validate_invoice_amount(doc.total_amount)?;

    let vehicle_uuid =
        Uuid::parse_str(vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;

    let trip = db
        .get_trip(trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Trip not found".to_string())?;

    if trip.vehicle_id != vehicle_uuid {
        return Err("Trip does not belong to the selected vehicle".to_string());
    }

    // Rule 2: idempotency -- same trip + same type is a no-op (I12).
    // Assigned elsewhere (or same trip, different type): reverse the
    // old contribution first (C4), then proceed as a fresh assign.
    if let Some(old_link) = db.get_paperless_link(id).map_err(|e| e.to_string())? {
        if old_link.trip_id == trip_id && old_link.assignment_type == assignment_type {
            // The assignment itself is unchanged, but the override flag is a
            // separate user decision and may have flipped -- persist it rather
            // than silently discarding it (Task 84 review, Minor 4).
            if old_link.mismatch_override != mismatch_override {
                db.set_paperless_override(id, mismatch_override)
                    .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        if old_link.assignment_type == AssignmentType::Other {
            if let Some(cents) = old_link.applied_amount_cents {
                remove_other_contribution(
                    db,
                    &old_link.trip_id,
                    cents,
                    old_link.title.as_deref(),
                )?;
            }
        }
    }
    // Re-load: the reversal may have mutated THIS trip (same-trip type change).
    let trip = db
        .get_trip(trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Trip not found".to_string())?;

    let coverage = trip_coverage(db, trip_id)?;
    let applied_amount_cents = match assignment_type {
        AssignmentType::Fuel => {
            // Rule 3: max one Fuel invoice per trip ACROSS both stores
            // (the partial unique indexes only guard within each table).
            if coverage.has_fuel {
                return Err(FUEL_INVOICE_EXISTS_ERR.to_string());
            }
            // Populate-if-empty unchanged.
            let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);
            if !trip_has_fuel {
                let mut updated = trip.clone();
                updated.fuel_liters = doc.litres;
                updated.fuel_cost_eur = doc.total_amount;
                updated.full_tank = true;
                db.update_trip(&updated).map_err(|e| e.to_string())?;
            }
            None
        }
        // Rule 4: sum-on-assign decision table.
        AssignmentType::Other => apply_other_amount(
            db,
            &trip,
            doc.total_amount,
            &doc.title,
            coverage.has_other,
        )?,
    };

    // Rule 6: snapshot assignment type + doc amount/title at assign
    // time -- always from the backend-fetched doc, never caller data.
    let link = crate::models::PaperlessLink {
        paperless_document_id: id,
        trip_id: trip_id.to_string(),
        assignment_type,
        amount_eur: doc.total_amount,
        title: Some(doc.title.clone()),
        applied_amount_cents,
        receipt_datetime: doc.receipt_datetime,
        mismatch_override,
    };
    db.upsert_paperless_link(&link).map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// Shared assignment rules (Task 66).
// ============================================================================

/// Rule 1: an invoice amount, when present, must be finite and non-negative.
/// Validated at the boundary before any mutation -- `to_cents(f64::NAN)` would
/// silently be 0, corrupting sums downstream.
pub(crate) fn validate_invoice_amount(amount: Option<f64>) -> Result<(), String> {
    match amount {
        Some(a) if !a.is_finite() || a < 0.0 => Err(format!(
            "Invalid invoice amount: {} (must be a non-negative number)",
            a
        )),
        _ => Ok(()),
    }
}

/// Per-trip invoice coverage across the stored links.
pub(crate) fn trip_coverage(db: &Database, trip_id: &str) -> Result<TripInvoiceCoverage, String> {
    Ok(db
        .get_trip_invoice_coverage()
        .map_err(|e| e.to_string())?
        .remove(trip_id)
        .unwrap_or_default())
}

/// Rule 5 (and the reversal half of rule 2): remove a previously applied
/// Other contribution from a trip. Subtracts exactly the applied snapshot in
/// cents (never the live invoice amount, which the user may have edited),
/// stores a zero result as `None` (not `Some(0.0)`), strips the appended note
/// segment when trivially identifiable, and tolerates orphaned links -- a
/// deleted trip means there is nothing to mutate (I10).
pub(crate) fn remove_other_contribution(
    db: &Database,
    trip_id: &str,
    applied_cents: i64,
    note_segment: Option<&str>,
) -> Result<(), String> {
    let Some(trip) = db.get_trip(trip_id).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let mut updated = trip.clone();
    let new_total = money_sub(
        trip.other_costs_eur.unwrap_or(0.0),
        from_cents(applied_cents),
    );
    updated.other_costs_eur = if to_cents(new_total) == 0 {
        None
    } else {
        Some(new_total)
    };
    if let Some(segment) = note_segment {
        updated.other_costs_note = strip_note_segment(updated.other_costs_note.take(), segment);
    }
    db.update_trip(&updated).map_err(|e| e.to_string())
}

/// Rule 4: the Other-assignment decision table. Mutates the trip when the
/// amount is applied and returns the `applied_amount_cents` snapshot value:
///
/// ```text
/// amount None                                   -> link-only; applied None (I3)
/// no existing Other && total == amount (cents)  -> link-only; applied None
///                                                  (double-count guard -- the user
///                                                  pre-entered the cost manually)
/// otherwise                                     -> total = money_add(total, amount);
///                                                  append note; applied Some(cents)
/// ```
///
/// Populate-if-empty is the money_add branch with an empty total -- identical
/// arithmetic, and appending a segment to an empty note sets it.
pub(crate) fn apply_other_amount(
    db: &Database,
    trip: &Trip,
    amount: Option<f64>,
    note_segment: &str,
    has_existing_other: bool,
) -> Result<Option<i64>, String> {
    let Some(amount) = amount else {
        return Ok(None);
    };
    let total = trip.other_costs_eur.unwrap_or(0.0);
    if !has_existing_other && to_cents(total) == to_cents(amount) {
        return Ok(None);
    }
    let mut updated = trip.clone();
    updated.other_costs_eur = Some(money_add(total, amount));
    updated.other_costs_note = Some(append_note_segment(
        updated.other_costs_note.take(),
        note_segment,
    ));
    db.update_trip(&updated).map_err(|e| e.to_string())?;
    Ok(Some(to_cents(amount)))
}

fn append_note_segment(existing: Option<String>, segment: &str) -> String {
    match existing {
        Some(note) if !note.trim().is_empty() => format!("{}; {}", note, segment),
        _ => segment.to_string(),
    }
}

/// Strip the note segment appended at assign time -- only when trivially
/// identifiable (the whole note, or a "; "-joined suffix/prefix). Anything
/// else means the user edited the note: leave it untouched.
fn strip_note_segment(note: Option<String>, segment: &str) -> Option<String> {
    let note = note?;
    if note == segment {
        return None;
    }
    if let Some(rest) = note.strip_suffix(&format!("; {}", segment)) {
        return Some(rest.to_string());
    }
    if let Some(rest) = note.strip_prefix(&format!("{}; ", segment)) {
        return Some(rest.to_string());
    }
    Some(note)
}

/// Clear the user-confirmed mismatch flag on a Paperless link.
pub fn revert_paperless_override_internal(
    db: &Database,
    app_state: &AppState,
    doc_id: i64,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.set_paperless_override(doc_id, false).map_err(|e| e.to_string())
}

/// Unassign a Paperless document from its trip.
pub fn unassign_paperless_invoice_internal(
    db: &Database,
    app_state: &AppState,
    doc_id: i64,
) -> Result<(), String> {
    check_read_only!(app_state);
    // Rule 5: reverse an applied Other contribution before deleting
    // the link. Fuel unassignments never touch other_costs.
    if let Some(link) = db.get_paperless_link(doc_id).map_err(|e| e.to_string())? {
        if link.assignment_type == AssignmentType::Other {
            if let Some(cents) = link.applied_amount_cents {
                remove_other_contribution(db, &link.trip_id, cents, link.title.as_deref())?;
            }
        }
    }
    db.delete_paperless_link_for_doc(doc_id).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "invoices_tests.rs"]
mod tests;
