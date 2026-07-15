//! Receipt Commands
//!
//! Commands for managing fuel receipts, including scanning, OCR processing,
//! assignment to trips, and verification.

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::gemini::is_mock_mode_enabled;
use crate::models::{
    AssignmentType, Receipt, ReceiptStatus, ReceiptVerification, Trip, VerificationResult,
};
use crate::receipts::{
    detect_folder_structure, process_receipt_with_gemini, scan_folder_for_new_receipts,
    FolderStructure,
};
use crate::settings::LocalSettings;

use super::statistics::is_datetime_in_trip_range;

use std::path::Path;

// ============================================================================
// Receipt Settings
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub gemini_api_key_from_override: bool,
    pub receipts_folder_from_override: bool,
}

pub fn get_receipt_settings_internal(app_dir: &Path) -> Result<ReceiptSettings, String> {
    let local = LocalSettings::load(app_dir);

    Ok(ReceiptSettings {
        gemini_api_key: local.gemini_api_key.clone(),
        receipts_folder_path: local.receipts_folder_path.clone(),
        gemini_api_key_from_override: local.gemini_api_key.is_some(),
        receipts_folder_from_override: local.receipts_folder_path.is_some(),
    })
}

pub fn set_gemini_api_key_internal(
    app_dir: &Path,
    app_state: &AppState,
    api_key: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    let mut settings = LocalSettings::load(app_dir);

    // Allow empty string to clear the key
    settings.gemini_api_key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };

    settings.save(app_dir).map_err(|e| e.to_string())
}

pub fn set_receipts_folder_path_internal(
    app_dir: &Path,
    app_state: &AppState,
    path: String,
) -> Result<(), String> {
    check_read_only!(app_state);

    // Validate path exists and is a directory (unless clearing)
    if !path.is_empty() {
        let path_buf = std::path::PathBuf::from(&path);
        if !path_buf.exists() {
            return Err(format!("Path does not exist: {}", path));
        }
        if !path_buf.is_dir() {
            return Err(format!("Path is not a directory: {}", path));
        }
    }

    let mut settings = LocalSettings::load(app_dir);

    // Allow empty string to clear the path
    settings.receipts_folder_path = if path.is_empty() { None } else { Some(path) };

    settings.save(app_dir).map_err(|e| e.to_string())
}

// ============================================================================
// Receipt CRUD Commands
// ============================================================================

/// Get receipts, optionally filtered by year.
/// - If year is provided: returns receipts for that year (by receipt_date, or source_year if date is None)
/// - If year is None: returns all receipts (for backward compatibility)
pub fn get_receipts_internal(db: &Database, year: Option<i32>) -> Result<Vec<Receipt>, String> {
    match year {
        Some(y) => db.get_receipts_for_year(y).map_err(|e| e.to_string()),
        None => db.get_all_receipts().map_err(|e| e.to_string()),
    }
}

/// Get receipts filtered by vehicle - returns unassigned receipts + receipts for specified vehicle.
/// Optionally filter by year.
pub fn get_receipts_for_vehicle_internal(
    db: &Database,
    vehicle_id: String,
    year: Option<i32>,
) -> Result<Vec<Receipt>, String> {
    let vehicle_uuid =
        Uuid::parse_str(&vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;
    db.get_receipts_for_vehicle(&vehicle_uuid, year)
        .map_err(|e| e.to_string())
}

pub fn get_unassigned_receipts_internal(db: &Database) -> Result<Vec<Receipt>, String> {
    db.get_unassigned_receipts().map_err(|e| e.to_string())
}

pub fn update_receipt_internal(
    db: &Database,
    app_state: &AppState,
    receipt: Receipt,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.update_receipt(&receipt).map_err(|e| e.to_string())
}

pub fn delete_receipt_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_receipt(&id).map_err(|e| e.to_string())
}

pub fn unassign_receipt_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    // Task 66 rule 5: reverse an applied Other contribution before clearing
    // the link. Keyed on the assignment TYPE — a Fuel unassignment never
    // touches other_costs_eur, even with a stale snapshot. Orphaned receipts
    // (trip deleted) skip the subtract inside remove_other_contribution.
    if let Some(receipt) = db.get_receipt_by_id(&id).map_err(|e| e.to_string())? {
        if let (Some(trip_id), Some(AssignmentType::Other), Some(cents)) = (
            receipt.trip_id,
            receipt.assignment_type,
            receipt.applied_amount_cents,
        ) {
            let segment = receipt_note_segment(&receipt);
            super::invoices::remove_other_contribution(
                db,
                &trip_id.to_string(),
                cents,
                Some(&segment),
            )?;
        }
    }
    db.unassign_receipt(&id).map_err(|e| e.to_string())
}

pub fn revert_receipt_override_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.revert_receipt_override(&id).map_err(|e| e.to_string())
}

// ============================================================================
// Receipt Scanning & Processing
// ============================================================================

/// Result of sync operation - includes both successes and errors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub processed: Vec<Receipt>,
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncError {
    pub file_name: String,
    pub error: String,
}

/// Result of scanning folder for new receipts (no OCR)
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub new_count: usize,
    pub warning: Option<String>,
}

/// Scan folder for new receipts without OCR processing
/// Returns count of new files found and any folder structure warnings
pub fn scan_receipts_internal(
    db: &Database,
    app_state: &AppState,
    app_dir: &Path,
) -> Result<ScanResult, String> {
    check_read_only!(app_state);
    let settings = LocalSettings::load(app_dir);

    let folder_path = settings
        .receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    // Scan for new files (this also inserts them into DB as Pending)
    let new_receipts = scan_folder_for_new_receipts(&folder_path, db)?;

    // Check folder structure for warnings
    let structure = detect_folder_structure(&folder_path);
    let warning = match structure {
        FolderStructure::Invalid(msg) => Some(msg),
        _ => None,
    };

    Ok(ScanResult {
        new_count: new_receipts.len(),
        warning,
    })
}

pub async fn sync_receipts_internal(
    db: &Database,
    app_state: &AppState,
    app_dir: &Path,
) -> Result<SyncResult, String> {
    check_read_only!(app_state);
    let settings = LocalSettings::load(app_dir);

    let folder_path = settings
        .receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    // In mock mode, API key is not required (extract_from_image loads from JSON files)
    let api_key = if is_mock_mode_enabled() {
        String::new()
    } else {
        settings
            .gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    // Scan for new files
    let mut new_receipts = scan_folder_for_new_receipts(&folder_path, db)?;
    let mut errors = Vec::new();

    // Process each new receipt with Gemini (async)
    for receipt in &mut new_receipts {
        if let Err(e) = process_receipt_with_gemini(receipt, &api_key).await {
            log::warn!("Failed to process receipt {}: {}", receipt.file_name, e);
            errors.push(SyncError {
                file_name: receipt.file_name.clone(),
                error: e,
            });
        }
        // Update in DB regardless of success/failure
        db.update_receipt(receipt).map_err(|e| e.to_string())?;
    }

    Ok(SyncResult {
        processed: new_receipts,
        errors,
    })
}

pub async fn process_pending_receipts_internal(
    db: &Database,
    app_dir: &Path,
) -> Result<SyncResult, String> {
    let settings = LocalSettings::load(app_dir);

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

    // Process each pending receipt with Gemini (no progress events in internal version)
    for receipt in pending_receipts.iter_mut() {
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

pub async fn reprocess_receipt_internal(
    db: &Database,
    app_state: &AppState,
    app_dir: &Path,
    id: String,
) -> Result<Receipt, String> {
    check_read_only!(app_state);
    let settings = LocalSettings::load(app_dir);

    // In mock mode, API key is not required (extract_from_image loads from JSON files)
    let api_key = if is_mock_mode_enabled() {
        String::new()
    } else {
        settings
            .gemini_api_key
            .ok_or("Gemini API key not configured")?
    };

    let mut receipt = db
        .get_receipt_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or("Receipt not found")?;

    // Clear previous error and reprocess
    receipt.error_message = None;

    // Process with async Gemini API
    if let Err(e) = process_receipt_with_gemini(&mut receipt, &api_key).await {
        receipt.error_message = Some(e.clone());
        receipt.status = ReceiptStatus::NeedsReview;
    }

    db.update_receipt(&receipt).map_err(|e| e.to_string())?;
    Ok(receipt)
}

// ============================================================================
// Receipt-Trip Assignment
// ============================================================================

/// Internal assign_receipt_to_trip logic (testable without State wrapper)
///
/// Task 51: User explicitly selects assignment type (FUEL or OTHER).
/// - assignment_type: "Fuel" or "Other"
/// - mismatch_override: true = user confirms data mismatch is intentional
///
/// Task 66: 1 Fuel + N Other invoices per trip. Shared rules (validation,
/// idempotency + reversal, cross-source Fuel uniqueness, sum-on-assign
/// decision table) live in [`super::invoices`] so the receipt and paperless
/// paths can never drift apart.
///
/// Data invariant: trip_id SET ↔ assignment_type SET
pub fn assign_receipt_to_trip_internal(
    db: &Database,
    receipt_id: &str,
    trip_id: &str,
    vehicle_id: &str,
    assignment_type: &str,
    mismatch_override: bool,
) -> Result<Receipt, String> {
    use super::invoices::{
        apply_other_amount, remove_other_contribution, trip_coverage, validate_invoice_amount,
        FUEL_INVOICE_EXISTS_ERR,
    };

    // Parse assignment type
    let assignment_type_enum = AssignmentType::from_str(assignment_type)
        .ok_or_else(|| format!("Invalid assignment type: {}", assignment_type))?;

    let mut receipt = db
        .get_receipt_by_id(receipt_id)
        .map_err(|e| e.to_string())?
        .ok_or("Receipt not found")?;

    // Rule 1: validate the amount BEFORE any mutation.
    validate_invoice_amount(receipt.total_price_eur)?;

    let trip_uuid = Uuid::parse_str(trip_id).map_err(|e| e.to_string())?;
    let vehicle_uuid = Uuid::parse_str(vehicle_id).map_err(|e| e.to_string())?;

    // Target trip must exist before anything is reversed.
    if db.get_trip(trip_id).map_err(|e| e.to_string())?.is_none() {
        return Err("Trip not found".to_string());
    }

    // Rule 2: idempotency — same trip + same type is a no-op (I12).
    if receipt.trip_id == Some(trip_uuid) && receipt.assignment_type == Some(assignment_type_enum)
    {
        return Ok(receipt);
    }
    // Assigned elsewhere (or same trip, different type): reverse the old
    // contribution first (C4), then proceed as a fresh assign.
    if let Some(old_trip_id) = receipt.trip_id {
        if receipt.assignment_type == Some(AssignmentType::Other) {
            if let Some(cents) = receipt.applied_amount_cents {
                let segment = receipt_note_segment(&receipt);
                remove_other_contribution(db, &old_trip_id.to_string(), cents, Some(&segment))?;
            }
        }
        receipt.applied_amount_cents = None;
    }

    // (Re-)load the target trip AFTER the reversal — it may be the same trip.
    let trip = db
        .get_trip(trip_id)
        .map_err(|e| e.to_string())?
        .ok_or("Trip not found")?;

    let coverage = trip_coverage(db, trip_id)?;

    match assignment_type_enum {
        AssignmentType::Fuel => {
            // Rule 3: max one Fuel invoice per trip ACROSS both stores
            // (the partial unique indexes only guard within each table).
            if coverage.has_fuel {
                return Err(FUEL_INVOICE_EXISTS_ERR.to_string());
            }

            // FUEL assignment: populate or link fuel fields
            let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);

            if !trip_has_fuel {
                // Trip has no fuel → populate from receipt (scenario C1)
                let mut updated_trip = trip.clone();
                updated_trip.fuel_liters = receipt.liters;
                updated_trip.fuel_cost_eur = receipt.total_price_eur;
                updated_trip.full_tank = true; // Assume full tank when populating from receipt
                db.update_trip(&updated_trip).map_err(|e| e.to_string())?;
            }
            // If trip already has fuel data, just link receipt (scenarios C3, C4, C5)
            // Mismatch detection is handled by mismatch_override flag - UI decides whether to warn
            receipt.applied_amount_cents = None;
        }
        AssignmentType::Other => {
            // Rule 4: sum-on-assign decision table (Task 66) — populate when
            // empty, guard against double-counting a manually pre-entered
            // amount, otherwise add cent-exactly and append the note segment.
            let segment = receipt_note_segment(&receipt);
            receipt.applied_amount_cents = apply_other_amount(
                db,
                &trip,
                receipt.total_price_eur,
                &segment,
                coverage.has_other,
            )?;
        }
    }

    // Mark receipt as assigned with explicit type (data invariant: trip_id + assignment_type set together)
    receipt.trip_id = Some(trip_uuid);
    receipt.vehicle_id = Some(vehicle_uuid);
    receipt.assignment_type = Some(assignment_type_enum);
    receipt.mismatch_override = mismatch_override;
    // Status unchanged - OCR status is orthogonal to assignment
    db.update_receipt(&receipt).map_err(|e| e.to_string())?;

    Ok(receipt)
}

/// Note segment for an Other receipt — appended to `trip.other_costs_note`
/// when the amount is applied, and used to strip that exact segment again on
/// unassign/reversal.
fn receipt_note_segment(receipt: &Receipt) -> String {
    match (&receipt.vendor_name, &receipt.cost_description) {
        (Some(v), Some(d)) => format!("{}: {}", v, d),
        (Some(v), None) => v.clone(),
        (None, Some(d)) => d.clone(),
        (None, None) => "Iné náklady".to_string(),
    }
}

// ============================================================================
// Trip Selection for Receipt Assignment
// ============================================================================

/// A trip annotated with whether a receipt can be attached to it.
/// Used by the frontend to show which trips are eligible for receipt assignment.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TripForAssignment {
    pub trip: Trip,
    /// Whether this receipt can be attached to this trip
    pub can_attach: bool,
    /// Status explaining why: "empty" (no fuel), "matches" (receipt matches trip fuel), "differs" (data conflicts)
    pub attachment_status: String,
    /// When status is "differs", explains what specifically doesn't match (for UI display)
    /// Values: null, "date", "liters", "price", "liters_and_price", "date_and_liters", "date_and_price", "all"
    pub mismatch_reason: Option<String>,
}

/// Internal get_trips_for_receipt_assignment logic (testable without State wrapper)
pub fn get_trips_for_receipt_assignment_internal(
    db: &Database,
    receipt_id: &str,
    vehicle_id: &str,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    // Get the receipt
    let receipt = db
        .get_receipt_by_id(receipt_id)
        .map_err(|e| e.to_string())?
        .ok_or("Receipt not found")?;

    // Get trips for this vehicle and year
    let trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Fetched ONCE for the whole picker list (Task 66) — per-trip entries are
    // passed down so the compat check can enforce multi-invoice rules.
    let coverage = db.get_trip_invoice_coverage().map_err(|e| e.to_string())?;
    let no_coverage = crate::models::TripInvoiceCoverage::default();

    // Annotate each trip with attachment eligibility
    let result = trips
        .into_iter()
        .map(|trip| {
            let trip_coverage = coverage.get(&trip.id.to_string()).unwrap_or(&no_coverage);
            let compat = crate::invoice::check_invoice_trip_compatibility(&receipt as &dyn crate::invoice::Invoice, &trip, trip_coverage);
            TripForAssignment {
                trip,
                can_attach: compat.can_attach,
                attachment_status: compat.status,
                mismatch_reason: compat.mismatch_reason,
            }
        })
        .collect();

    Ok(result)
}

// ============================================================================
// Receipt Verification
// ============================================================================

/// Internal verify_receipts logic (testable without State wrapper)
pub fn verify_receipts_internal(
    db: &Database,
    vehicle_id: &str,
    year: i32,
) -> Result<VerificationResult, String> {
    let vehicle_uuid =
        Uuid::parse_str(vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;

    // Get receipts filtered by vehicle (unassigned + this vehicle's receipts)
    // Filter by year: use receipt_datetime if available, fallback to source_year (folder structure)
    let all_receipts = db
        .get_receipts_for_vehicle(&vehicle_uuid, Some(year))
        .map_err(|e| e.to_string())?;
    let receipts_for_year: Vec<_> = all_receipts
        .into_iter()
        .filter(|r| {
            r.receipt_datetime
                .map(|dt| dt.year() == year)
                .unwrap_or(false)
                || r.source_year == Some(year)
        })
        .collect();

    verify_receipts_with_data(db, vehicle_id, year, receipts_for_year)
}

/// Helper to perform verification with pre-fetched receipts.
///
/// Design spec v7: Simple model - receipt is "assigned" if trip_id is set, "unassigned" otherwise.
/// No computed data matching - user explicitly assigns receipts to trips.
fn verify_receipts_with_data(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    receipts_for_year: Vec<Receipt>,
) -> Result<VerificationResult, String> {
    use crate::models::MismatchReason;

    // Get all trips for this vehicle/year (needed for trip info display)
    let all_trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;

    let mut verifications = Vec::new();
    let mut assigned_count = 0;

    for receipt in &receipts_for_year {
        // Design spec v7: Simple check - is trip_id set?
        let is_assigned = receipt.trip_id.is_some();

        let mut matched_trip_id = None;
        let mut matched_trip_datetime = None;
        let mut matched_trip_time_range = None;
        let mut matched_trip_route = None;
        let mut datetime_warning = false;

        // If assigned, get trip info for display
        if let Some(trip_uuid) = receipt.trip_id {
            matched_trip_id = Some(trip_uuid.to_string());

            if let Some(trip) = all_trips.iter().find(|t| t.id == trip_uuid) {
                let trip_end = trip.end_datetime.unwrap_or(trip.start_datetime);

                // Format: "D.M. HH:MM–HH:MM" (e.g., "22.1. 15:00–17:00")
                matched_trip_datetime = Some(format!(
                    "{} {}–{}",
                    trip.start_datetime.date().format("%-d.%-m."),
                    trip.start_datetime.format("%H:%M"),
                    trip_end.format("%H:%M")
                ));

                // Format: "HH:MM–HH:MM" for warning message
                matched_trip_time_range = Some(format!(
                    "{}–{}",
                    trip.start_datetime.format("%H:%M"),
                    trip_end.format("%H:%M")
                ));

                matched_trip_route = Some(format!("{} - {}", trip.origin, trip.destination));

                // Check datetime warning for assigned FUEL receipts
                if let Some(receipt_dt) = receipt.receipt_datetime {
                    datetime_warning = !is_datetime_in_trip_range(receipt_dt, trip);
                }
            }

            assigned_count += 1;
        }

        verifications.push(ReceiptVerification {
            receipt_id: receipt.id.to_string(),
            matched: is_assigned,
            matched_trip_id,
            matched_trip_datetime,
            matched_trip_time_range,
            matched_trip_route,
            mismatch_reason: MismatchReason::None, // No computed mismatch in new model
            datetime_warning,
        });
    }

    let total = verifications.len();
    Ok(VerificationResult {
        total,
        matched: assigned_count,
        unmatched: total - assigned_count,
        receipts: verifications,
    })
}
