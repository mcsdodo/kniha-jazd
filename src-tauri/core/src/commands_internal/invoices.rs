//! Unified invoice command implementations (Task 64).
//!
//! Source dispatch confined to the three boundary functions here.
//! Beyond these, code consumes `&dyn Invoice` and never inspects the source.

use uuid::Uuid;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::invoice::{
    check_invoice_trip_compatibility, Invoice, InvoiceData, InvoiceRef, PaperlessInvoiceView,
};
use crate::models::{AssignmentType, Trip};
use crate::paperless::PaperlessDoc;

use super::receipts_cmd::TripForAssignment;

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

    match invoice_ref {
        InvoiceRef::Receipt(id) => {
            let receipt = db
                .get_receipt_by_id(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Receipt not found".to_string())?;
            Ok(annotate_trips(&receipt, trips))
        }
        InvoiceRef::Paperless(id) => {
            let data = data.ok_or_else(|| {
                "InvoiceData required for Paperless invoices".to_string()
            })?;
            let view = PaperlessInvoiceView { id: *id, data };
            Ok(annotate_trips(&view, trips))
        }
    }
}

fn annotate_trips(invoice: &dyn Invoice, trips: Vec<Trip>) -> Vec<TripForAssignment> {
    trips
        .into_iter()
        .map(|trip| {
            let compat = check_invoice_trip_compatibility(invoice, &trip);
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
            let vehicle_uuid =
                Uuid::parse_str(vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;

            let trip = db
                .get_trip(trip_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Trip not found".to_string())?;

            if trip.vehicle_id != vehicle_uuid {
                return Err("Trip does not belong to the selected vehicle".to_string());
            }

            // Populate trip data from backend-fetched doc when trip side is empty
            match assignment_type {
                AssignmentType::Fuel => {
                    let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);
                    if !trip_has_fuel {
                        let mut updated = trip.clone();
                        updated.fuel_liters = doc.litres;
                        updated.fuel_cost_eur = doc.total_amount;
                        updated.full_tank = true;
                        db.update_trip(&updated).map_err(|e| e.to_string())?;
                    }
                }
                AssignmentType::Other => {
                    let trip_has_other = trip.other_costs_eur.map(|c| c > 0.0).unwrap_or(false);
                    if !trip_has_other {
                        let mut updated = trip.clone();
                        updated.other_costs_eur = doc.total_amount;
                        updated.other_costs_note = Some(doc.title.clone());
                        db.update_trip(&updated).map_err(|e| e.to_string())?;
                    }
                }
            }

            // Snapshot assignment type + doc amount/title at assign time
            // (backend-fetched doc — never caller data). applied_amount_cents
            // stays None here; sum-on-assign semantics land in Task 66 step 5.
            let link = crate::models::PaperlessLink {
                paperless_document_id: *id,
                trip_id: trip_id.to_string(),
                assignment_type,
                amount_eur: doc.total_amount,
                title: Some(doc.title.clone()),
                applied_amount_cents: None,
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
        InvoiceRef::Paperless(id) => db
            .delete_paperless_link_for_doc(*id)
            .map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
#[path = "invoices_tests.rs"]
mod tests;
