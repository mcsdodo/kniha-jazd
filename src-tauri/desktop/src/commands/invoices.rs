//! Tauri command wrappers for unified invoice assignment (Task 64).

use std::sync::Arc;
use tauri::State;

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::invoices as inner;
use kniha_jazd_core::commands_internal::receipts_cmd::TripForAssignment;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::invoice::{InvoiceData, InvoiceRef};
use kniha_jazd_core::models::AssignmentType;

#[tauri::command]
pub fn get_trips_for_invoice_assignment(
    db: State<'_, Arc<Database>>,
    invoice_ref: InvoiceRef,
    invoice_data: Option<InvoiceData>,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    inner::get_trips_for_invoice_assignment_internal(
        &db, &invoice_ref, invoice_data.as_ref(), &vehicle_id, year,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn assign_invoice_to_trip(
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
    invoice_ref: InvoiceRef,
    invoice_data: Option<InvoiceData>,
    trip_id: String,
    vehicle_id: String,
    assignment_type: AssignmentType,
    mismatch_override: bool,
) -> Result<(), String> {
    inner::assign_invoice_to_trip_internal(
        &db, &app_state, &invoice_ref, invoice_data.as_ref(),
        &trip_id, &vehicle_id, assignment_type, mismatch_override,
    )
}

#[tauri::command]
pub fn unassign_invoice(
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
    invoice_ref: InvoiceRef,
) -> Result<(), String> {
    inner::unassign_invoice_internal(&db, &app_state, &invoice_ref)
}
