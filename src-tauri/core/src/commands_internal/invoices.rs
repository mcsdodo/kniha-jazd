//! Unified invoice command implementations (Task 64).
//!
//! Source dispatch confined to the three boundary functions here.
//! Beyond these, code consumes `&dyn Invoice` and never inspects the source.

use std::collections::HashMap;

use uuid::Uuid;

use crate::app_state::AppState;
use crate::calculations::{from_cents, money_add, money_sub, to_cents};
use crate::check_read_only;
use crate::db::Database;
use crate::invoice::{
    check_invoice_trip_compatibility, Invoice, InvoiceData, InvoiceRef, PaperlessInvoiceView,
};
use crate::models::{AssignmentType, Trip, TripInvoiceCoverage};
use crate::paperless::PaperlessDoc;

use super::receipts_cmd::TripForAssignment;

/// Rule 3 error: a trip can hold at most ONE Fuel invoice across both sources
/// (local receipts + paperless links). Translated frontend-side (i18n).
pub(crate) const FUEL_INVOICE_EXISTS_ERR: &str = "Trip already has a fuel invoice";

/// Get trips annotated with attachment status for a given invoice.
/// For Receipt: backend loads from DB by id (ignores `data`).
/// For Paperless: backend uses `data` directly (the inline doc payload from the frontend).
pub fn get_trips_for_invoice_assignment_internal(
    db: &Database,
    invoice_ref: &InvoiceRef,
    data: Option<&InvoiceData>,
    vehicle_id: &str,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    let trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Fetched ONCE for the whole picker list (Task 66) — per-trip entries are
    // passed down so the compat check can enforce multi-invoice rules.
    let coverage = db.get_trip_invoice_coverage().map_err(|e| e.to_string())?;

    match invoice_ref {
        InvoiceRef::Receipt(id) => {
            let receipt = db
                .get_receipt_by_id(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Receipt not found".to_string())?;
            Ok(annotate_trips(&receipt, trips, &coverage))
        }
        InvoiceRef::Paperless(id) => {
            let data = data.ok_or_else(|| {
                "InvoiceData required for Paperless invoices".to_string()
            })?;
            let view = PaperlessInvoiceView { id: *id, data };
            Ok(annotate_trips(&view, trips, &coverage))
        }
    }
}

fn annotate_trips(
    invoice: &dyn Invoice,
    trips: Vec<Trip>,
    coverage: &HashMap<String, TripInvoiceCoverage>,
) -> Vec<TripForAssignment> {
    let no_coverage = TripInvoiceCoverage::default();
    trips
        .into_iter()
        .map(|trip| {
            let trip_coverage = coverage.get(&trip.id.to_string()).unwrap_or(&no_coverage);
            let compat = check_invoice_trip_compatibility(invoice, &trip, trip_coverage);
            TripForAssignment {
                trip,
                can_attach: compat.can_attach,
                attachment_status: compat.status,
                mismatch_reason: compat.mismatch_reason,
            }
        })
        .collect()
}

/// Assign an invoice to a trip.
/// For Receipt: delegates to existing receipt-assignment logic.
/// For Paperless: `doc` must be backend-fetched (never trust caller-supplied data for writes).
#[allow(clippy::too_many_arguments)]
pub fn assign_invoice_to_trip_internal(
    db: &Database,
    app_state: &AppState,
    invoice_ref: &InvoiceRef,
    doc: Option<&PaperlessDoc>,
    trip_id: &str,
    vehicle_id: &str,
    assignment_type: AssignmentType,
    mismatch_override: bool,
) -> Result<(), String> {
    check_read_only!(app_state);

    match invoice_ref {
        InvoiceRef::Receipt(id) => {
            super::receipts_cmd::assign_receipt_to_trip_internal(
                db,
                id,
                trip_id,
                vehicle_id,
                assignment_type.as_str(),
                mismatch_override,
            )
            .map(|_| ())
        }
        InvoiceRef::Paperless(id) => {
            let doc = doc.ok_or_else(|| {
                "PaperlessDoc required for Paperless invoices".to_string()
            })?;

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

            // Rule 2: idempotency — same trip + same type is a no-op (I12).
            // Assigned elsewhere (or same trip, different type): reverse the
            // old contribution first (C4), then proceed as a fresh assign.
            if let Some(old_link) = db.get_paperless_link(*id).map_err(|e| e.to_string())? {
                if old_link.trip_id == trip_id && old_link.assignment_type == assignment_type {
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
            // time — always from the backend-fetched doc, never caller data.
            let link = crate::models::PaperlessLink {
                paperless_document_id: *id,
                trip_id: trip_id.to_string(),
                assignment_type,
                amount_eur: doc.total_amount,
                title: Some(doc.title.clone()),
                applied_amount_cents,
            };
            db.upsert_paperless_link(&link).map_err(|e| e.to_string())?;
            // mismatch_override is currently ignored for Paperless — `paperless_trip_links`
            // has no override column. If users need this for Paperless, extend the schema
            // in a follow-up task.
            let _ = mismatch_override;
            Ok(())
        }
    }
}

// ============================================================================
// Shared assignment rules (Task 66) — used by BOTH invoice sources.
// receipts_cmd::assign_receipt_to_trip_internal and the Paperless arm above
// route through these helpers so the semantics can never drift apart.
// ============================================================================

/// Rule 1: an invoice amount, when present, must be finite and non-negative.
/// Validated at the boundary before any mutation — `to_cents(f64::NAN)` would
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

/// Per-trip invoice coverage across BOTH stores (receipts + paperless links).
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
/// segment when trivially identifiable, and tolerates orphaned links — a
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
///                                                  (double-count guard — the user
///                                                  pre-entered the cost manually)
/// otherwise                                     -> total = money_add(total, amount);
///                                                  append note; applied Some(cents)
/// ```
///
/// Populate-if-empty is the money_add branch with an empty total — identical
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

/// Strip the note segment appended at assign time — only when trivially
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

/// Unassign an invoice from its trip.
pub fn unassign_invoice_internal(
    db: &Database,
    app_state: &AppState,
    invoice_ref: &InvoiceRef,
) -> Result<(), String> {
    check_read_only!(app_state);
    match invoice_ref {
        InvoiceRef::Receipt(id) => {
            super::receipts_cmd::unassign_receipt_internal(db, app_state, id.clone())
        }
        InvoiceRef::Paperless(id) => {
            // Rule 5: reverse an applied Other contribution before deleting
            // the link. Fuel unassignments never touch other_costs.
            if let Some(link) = db.get_paperless_link(*id).map_err(|e| e.to_string())? {
                if link.assignment_type == AssignmentType::Other {
                    if let Some(cents) = link.applied_amount_cents {
                        remove_other_contribution(db, &link.trip_id, cents, link.title.as_deref())?;
                    }
                }
            }
            db.delete_paperless_link_for_doc(*id).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
#[path = "invoices_tests.rs"]
mod tests;
