//! Receipt Tauri command wrappers.
//!
//! All `_internal` implementations live in
//! [`kniha_jazd_core::commands_internal::receipts_cmd`]. These wrappers
//! translate Tauri-specific types (`AppHandle`, `State`) into plain types and
//! delegate.
//!
//! `process_pending_receipts` is the only exception — it keeps its body in
//! desktop because it emits Tauri events ("receipt-processing-progress") via
//! `app.emit(...)` while iterating, which is a Tauri-flavored side effect.
//!
//! The `pub use` re-export below keeps `super::receipts_cmd::*` style access
//! working from `commands_tests.rs` until that test file is moved to core
//! (Task 22a).

pub use kniha_jazd_core::commands_internal::receipts_cmd::*;

use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, State};

use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::receipts_cmd as inner;
use kniha_jazd_core::db::Database;
use kniha_jazd_core::gemini::is_mock_mode_enabled;
use kniha_jazd_core::models::{Receipt, VerificationResult};
use kniha_jazd_core::receipts::process_receipt_with_gemini;
use kniha_jazd_core::settings::LocalSettings;

use super::get_app_data_dir;

// ============================================================================
// Receipt Settings
// ============================================================================

#[tauri::command]
pub fn get_receipt_settings(app: tauri::AppHandle) -> Result<ReceiptSettings, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::get_receipt_settings_internal(&app_dir)
}

#[tauri::command]
pub fn set_gemini_api_key(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    api_key: String,
) -> Result<(), String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::set_gemini_api_key_internal(&app_data_dir, &app_state, api_key)
}

#[tauri::command]
pub fn set_receipts_folder_path(
    app_handle: tauri::AppHandle,
    app_state: State<Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let app_data_dir = get_app_data_dir(&app_handle)?;
    inner::set_receipts_folder_path_internal(&app_data_dir, &app_state, path)
}

// ============================================================================
// Receipt CRUD Commands
// ============================================================================

#[tauri::command]
pub fn get_receipts(db: State<Arc<Database>>, year: Option<i32>) -> Result<Vec<Receipt>, String> {
    inner::get_receipts_internal(&db, year)
}

#[tauri::command]
pub fn get_receipts_for_vehicle(
    db: State<Arc<Database>>,
    vehicle_id: String,
    year: Option<i32>,
) -> Result<Vec<Receipt>, String> {
    inner::get_receipts_for_vehicle_internal(&db, vehicle_id, year)
}

#[tauri::command]
pub fn get_unassigned_receipts(db: State<Arc<Database>>) -> Result<Vec<Receipt>, String> {
    inner::get_unassigned_receipts_internal(&db)
}

#[tauri::command]
pub fn update_receipt(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    receipt: Receipt,
) -> Result<(), String> {
    inner::update_receipt_internal(&db, &app_state, receipt)
}

#[tauri::command]
pub fn delete_receipt(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    inner::delete_receipt_internal(&db, &app_state, id)
}

#[tauri::command]
pub fn unassign_receipt(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    inner::unassign_receipt_internal(&db, &app_state, id)
}

#[tauri::command]
pub fn revert_receipt_override(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    inner::revert_receipt_override_internal(&db, &app_state, id)
}

// ============================================================================
// Receipt Scanning & Processing
// ============================================================================

#[tauri::command]
pub fn scan_receipts(
    app: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
) -> Result<ScanResult, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::scan_receipts_internal(&db, &app_state, &app_dir)
}

#[tauri::command]
pub async fn sync_receipts(
    app: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
) -> Result<SyncResult, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::sync_receipts_internal(&db, &app_state, &app_dir).await
}

/// Progress event payload emitted while OCR-processing pending receipts.
#[derive(Clone, Serialize)]
pub struct ProcessingProgress {
    pub current: usize,
    pub total: usize,
    pub file_name: String,
}

/// Process all pending receipts with Gemini, emitting progress events.
///
/// This wrapper keeps its own body (rather than delegating to
/// `process_pending_receipts_internal`) because it emits Tauri events on every
/// iteration, which the framework-free internal version cannot do.
#[tauri::command]
pub async fn process_pending_receipts(
    app: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
) -> Result<SyncResult, String> {
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    // In mock mode, API key is not required (extract_from_image loads from JSON files)
    let api_key = if is_mock_mode_enabled() {
        String::new()
    } else {
        settings
            .gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    // Get all pending receipts
    let mut pending_receipts = db.get_pending_receipts().map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    let total = pending_receipts.len();

    // Process each pending receipt with Gemini
    for (index, receipt) in pending_receipts.iter_mut().enumerate() {
        // Emit progress event
        let _ = app.emit(
            "receipt-processing-progress",
            ProcessingProgress {
                current: index + 1,
                total,
                file_name: receipt.file_name.clone(),
            },
        );

        match process_receipt_with_gemini(receipt, &api_key).await {
            Ok(()) => {
                // Only update DB on success
                db.update_receipt(receipt).map_err(|e| e.to_string())?;
            }
            Err(e) => {
                log::warn!("Failed to process receipt {}: {}", receipt.file_name, e);
                errors.push(SyncError {
                    file_name: receipt.file_name.clone(),
                    error: e,
                });
                // Don't update DB - leave receipt in Pending state for retry
            }
        }
    }

    Ok(SyncResult {
        processed: pending_receipts,
        errors,
    })
}

#[tauri::command]
pub async fn reprocess_receipt(
    app: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Receipt, String> {
    let app_dir = get_app_data_dir(&app)?;
    inner::reprocess_receipt_internal(&db, &app_state, &app_dir, id).await
}

// ============================================================================
// Receipt-Trip Assignment
// ============================================================================

/// Assign a receipt to a trip with explicit type selection.
///
/// Task 51: User explicitly selects assignment type (FUEL or OTHER).
/// - assignment_type: "Fuel" or "Other"
/// - mismatch_override: true = user confirms data mismatch is intentional
#[tauri::command]
pub fn assign_receipt_to_trip(
    db: State<Arc<Database>>,
    app_state: State<Arc<AppState>>,
    receipt_id: String,
    trip_id: String,
    vehicle_id: String,
    assignment_type: String,
    mismatch_override: bool,
) -> Result<Receipt, String> {
    use kniha_jazd_core::check_read_only;
    check_read_only!(app_state);
    inner::assign_receipt_to_trip_internal(
        &db,
        &receipt_id,
        &trip_id,
        &vehicle_id,
        &assignment_type,
        mismatch_override,
    )
}

/// Get trips for a vehicle/year annotated with whether a specific receipt can be attached.
/// This allows the frontend to show which trips are eligible for receipt assignment.
#[tauri::command]
pub fn get_trips_for_receipt_assignment(
    db: State<Arc<Database>>,
    receipt_id: String,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    inner::get_trips_for_receipt_assignment_internal(&db, &receipt_id, &vehicle_id, year)
}

// ============================================================================
// Receipt Verification
// ============================================================================

/// Verify receipts against trips by matching date, liters, and price.
/// Returns verification status for each receipt in the given year.
/// Only considers receipts that are unassigned or assigned to this vehicle.
#[tauri::command]
pub fn verify_receipts(
    db: State<Arc<Database>>,
    vehicle_id: String,
    year: i32,
) -> Result<VerificationResult, String> {
    inner::verify_receipts_internal(&db, &vehicle_id, year)
}
