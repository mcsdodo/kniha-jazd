//! Receipt Commands
//!
//! Commands for managing fuel receipts, including scanning, OCR processing,
//! assignment to trips, and verification.

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use uuid::Uuid;

use crate::check_read_only;
use crate::constants::date_formats;
use crate::db::Database;
use crate::gemini::is_mock_mode_enabled;
use crate::models::{
    AttachmentStatus, MismatchReason, Receipt, ReceiptStatus, ReceiptVerification, Trip, VerificationResult,
};
use crate::receipts::{
    detect_folder_structure, process_receipt_with_gemini, scan_folder_for_new_receipts,
    FolderStructure,
};
use crate::settings::LocalSettings;

use super::statistics::is_datetime_in_trip_range;
use super::{get_app_data_dir, AppState};

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

#[tauri::command]
pub fn get_receipt_settings(app: tauri::AppHandle) -> Result<ReceiptSettings, String> {
    let app_dir = get_app_data_dir(&app)?;
    let local = LocalSettings::load(&app_dir);

    Ok(ReceiptSettings {
        gemini_api_key: local.gemini_api_key.clone(),
        receipts_folder_path: local.receipts_folder_path.clone(),
        gemini_api_key_from_override: local.gemini_api_key.is_some(),
        receipts_folder_from_override: local.receipts_folder_path.is_some(),
    })
}

#[tauri::command]
pub fn set_gemini_api_key(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
    api_key: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_data_dir = get_app_data_dir(&app_handle)?;
    let mut settings = LocalSettings::load(&app_data_dir);

    // Allow empty string to clear the key
    settings.gemini_api_key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };

    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_receipts_folder_path(
    app_handle: tauri::AppHandle,
    app_state: State<AppState>,
    path: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    let app_data_dir = get_app_data_dir(&app_handle)?;

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

    let mut settings = LocalSettings::load(&app_data_dir);

    // Allow empty string to clear the path
    settings.receipts_folder_path = if path.is_empty() { None } else { Some(path) };

    settings.save(&app_data_dir).map_err(|e| e.to_string())
}

// ============================================================================
// Receipt CRUD Commands
// ============================================================================

/// Get receipts, optionally filtered by year.
/// - If year is provided: returns receipts for that year (by receipt_date, or source_year if date is None)
/// - If year is None: returns all receipts (for backward compatibility)
#[tauri::command]
pub fn get_receipts(db: State<Database>, year: Option<i32>) -> Result<Vec<Receipt>, String> {
    match year {
        Some(y) => db.get_receipts_for_year(y).map_err(|e| e.to_string()),
        None => db.get_all_receipts().map_err(|e| e.to_string()),
    }
}

/// Get receipts filtered by vehicle - returns unassigned receipts + receipts for specified vehicle.
/// Optionally filter by year.
#[tauri::command]
pub fn get_receipts_for_vehicle(
    db: State<Database>,
    vehicle_id: String,
    year: Option<i32>,
) -> Result<Vec<Receipt>, String> {
    let vehicle_uuid =
        Uuid::parse_str(&vehicle_id).map_err(|e| format!("Invalid vehicle ID: {}", e))?;
    db.get_receipts_for_vehicle(&vehicle_uuid, year)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_unassigned_receipts(db: State<Database>) -> Result<Vec<Receipt>, String> {
    db.get_unassigned_receipts().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_receipt(
    db: State<Database>,
    app_state: State<AppState>,
    receipt: Receipt,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.update_receipt(&receipt).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_receipt(
    db: State<Database>,
    app_state: State<AppState>,
    id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_receipt(&id).map_err(|e| e.to_string())
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
#[tauri::command]
pub fn scan_receipts(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    app_state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

    let folder_path = settings
        .receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    // Scan for new files (this also inserts them into DB as Pending)
    let new_receipts = scan_folder_for_new_receipts(&folder_path, &db)?;

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

#[tauri::command]
pub async fn sync_receipts(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    app_state: State<'_, AppState>,
) -> Result<SyncResult, String> {
    check_read_only!(app_state);
    let app_dir = get_app_data_dir(&app)?;
    let settings = LocalSettings::load(&app_dir);

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
    let mut new_receipts = scan_folder_for_new_receipts(&folder_path, &db)?;
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

#[derive(Clone, Serialize)]
pub struct ProcessingProgress {
    pub current: usize,
    pub total: usize,
    pub file_name: String,
}

#[tauri::command]
pub async fn process_pending_receipts(
    app: tauri::AppHandle,
    db: State<'_, Database>,
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
    db: State<'_, Database>,
    app_state: State<'_, AppState>,
    id: String,
) -> Result<Receipt, String> {
    check_read_only!(app_state);
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
pub fn assign_receipt_to_trip_internal(
    db: &Database,
    receipt_id: &str,
    trip_id: &str,
    vehicle_id: &str,
) -> Result<Receipt, String> {
    let mut receipts = db.get_all_receipts().map_err(|e| e.to_string())?;
    let receipt = receipts
        .iter_mut()
        .find(|r| r.id.to_string() == receipt_id)
        .ok_or("Receipt not found")?;

    let trip_uuid = Uuid::parse_str(trip_id).map_err(|e| e.to_string())?;
    let vehicle_uuid = Uuid::parse_str(vehicle_id).map_err(|e| e.to_string())?;

    let trip = db
        .get_trip(trip_id)
        .map_err(|e| e.to_string())?
        .ok_or("Trip not found")?;

    // Multi-stage matching: determine if this is FUEL or OTHER COST
    // Receipt is FUEL if:
    //   1. Receipt has liters + price, AND
    //   2. Trip has NO fuel data (empty trip) OR trip fuel data matches receipt
    // Otherwise it's OTHER COST

    let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);

    let is_fuel_receipt = match (receipt.liters, receipt.total_price_eur) {
        (Some(liters), Some(price)) if liters > 0.0 => {
            if !trip_has_fuel {
                // Trip has no fuel → receipt will populate fuel fields
                true
            } else {
                // Trip has fuel → check if receipt matches (verification)
                // Receipt datetime must be within trip's [start, end] range
                let datetime_match = receipt.receipt_datetime
                    .map(|dt| is_datetime_in_trip_range(dt, &trip))
                    .unwrap_or(false);
                let liters_match = trip
                    .fuel_liters
                    .map(|fl| (fl - liters).abs() < 0.01)
                    .unwrap_or(false);
                let price_match = trip
                    .fuel_cost_eur
                    .map(|fc| (fc - price).abs() < 0.01)
                    .unwrap_or(false);
                datetime_match && liters_match && price_match
            }
        }
        _ => false, // No liters or no price → cannot be fuel
    };

    if is_fuel_receipt {
        // FUEL: populate or verify fuel fields
        if !trip_has_fuel {
            // Trip has no fuel → populate from receipt
            let mut updated_trip = trip.clone();
            updated_trip.fuel_liters = receipt.liters;
            updated_trip.fuel_cost_eur = receipt.total_price_eur;
            updated_trip.full_tank = true; // Assume full tank when populating from receipt
            db.update_trip(&updated_trip).map_err(|e| e.to_string())?;
        }
        // If trip already has matching fuel data, nothing to update (just link receipt)
    } else {
        // OTHER COST: populate trip.other_costs_* fields
        // (receipt without liters, or liters that don't match existing trip fuel)

        // Check for collision
        if trip.other_costs_eur.is_some() {
            return Err("Jazda už má iné náklady".to_string());
        }

        // Build note from receipt data
        let note = match (&receipt.vendor_name, &receipt.cost_description) {
            (Some(v), Some(d)) => format!("{}: {}", v, d),
            (Some(v), None) => v.clone(),
            (None, Some(d)) => d.clone(),
            (None, None) => "Iné náklady".to_string(),
        };

        // Update trip with other costs
        let mut updated_trip = trip.clone();
        updated_trip.other_costs_eur = receipt.total_price_eur;
        updated_trip.other_costs_note = Some(note);
        db.update_trip(&updated_trip).map_err(|e| e.to_string())?;
    }

    // Mark receipt as assigned (same for both types)
    receipt.trip_id = Some(trip_uuid);
    receipt.vehicle_id = Some(vehicle_uuid);
    receipt.status = ReceiptStatus::Assigned;
    db.update_receipt(receipt).map_err(|e| e.to_string())?;

    Ok(receipt.clone())
}

#[tauri::command]
pub fn assign_receipt_to_trip(
    db: State<Database>,
    app_state: State<AppState>,
    receipt_id: String,
    trip_id: String,
    vehicle_id: String,
) -> Result<Receipt, String> {
    check_read_only!(app_state);
    assign_receipt_to_trip_internal(&db, &receipt_id, &trip_id, &vehicle_id)
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

/// Result of checking receipt-trip compatibility
struct CompatibilityResult {
    can_attach: bool,
    status: String,
    mismatch_reason: Option<String>,
}

/// Check if receipt data matches trip's existing fuel data.
/// Returns compatibility result with detailed mismatch reason.
fn check_receipt_trip_compatibility(receipt: &Receipt, trip: &Trip) -> CompatibilityResult {
    // No fuel data on trip → can attach (receipt will populate fuel fields)
    let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);
    if !trip_has_fuel {
        return CompatibilityResult {
            can_attach: true,
            status: AttachmentStatus::Empty.as_str().to_string(),
            mismatch_reason: None,
        };
    }

    // Trip has fuel data - check if receipt matches
    match (receipt.liters, receipt.total_price_eur) {
        (Some(r_liters), Some(r_price)) => {
            // Receipt has fuel data - compare with trip
            // Receipt datetime must be within trip's [start, end] range
            let datetime_match = receipt.receipt_datetime
                .map(|dt| is_datetime_in_trip_range(dt, trip))
                .unwrap_or(false);
            let liters_match = trip
                .fuel_liters
                .map(|fl| (fl - r_liters).abs() < 0.01)
                .unwrap_or(false);
            let price_match = trip
                .fuel_cost_eur
                .map(|fc| (fc - r_price).abs() < 0.01)
                .unwrap_or(false);

            if datetime_match && liters_match && price_match {
                CompatibilityResult {
                    can_attach: true,
                    status: AttachmentStatus::Matches.as_str().to_string(),
                    mismatch_reason: None,
                }
            } else {
                // Determine what specifically doesn't match
                let mismatch = match (datetime_match, liters_match, price_match) {
                    (false, false, false) => "all",
                    (false, false, true) => "date_and_liters",
                    (false, true, false) => "date_and_price",
                    (false, true, true) => "date",
                    (true, false, false) => "liters_and_price",
                    (true, false, true) => "liters",
                    (true, true, false) => "price",
                    (true, true, true) => unreachable!(), // Would have matched above
                };
                CompatibilityResult {
                    can_attach: false,
                    status: AttachmentStatus::Differs.as_str().to_string(),
                    mismatch_reason: Some(mismatch.to_string()),
                }
            }
        }
        _ => {
            // Receipt has no fuel data (other cost receipt) - can still attach as other cost
            // But wait - trip already has fuel, so this would be "other cost" on a fuel trip
            // Allow it since trips can have both fuel AND other costs
            CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Empty.as_str().to_string(),
                mismatch_reason: None,
            }
        }
    }
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

    // Annotate each trip with attachment eligibility
    let result = trips
        .into_iter()
        .map(|trip| {
            let compat = check_receipt_trip_compatibility(&receipt, &trip);
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

/// Get trips for a vehicle/year annotated with whether a specific receipt can be attached.
/// This allows the frontend to show which trips are eligible for receipt assignment.
#[tauri::command]
pub fn get_trips_for_receipt_assignment(
    db: State<Database>,
    receipt_id: String,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<TripForAssignment>, String> {
    get_trips_for_receipt_assignment_internal(&db, &receipt_id, &vehicle_id, year)
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
    let all_receipts = db
        .get_receipts_for_vehicle(&vehicle_uuid, Some(year))
        .map_err(|e| e.to_string())?;
    let receipts_for_year: Vec<_> = all_receipts
        .into_iter()
        .filter(|r| r.receipt_datetime.map(|dt| dt.year() == year).unwrap_or(false))
        .collect();

    verify_receipts_with_data(db, vehicle_id, year, receipts_for_year)
}

/// Helper to perform verification with pre-fetched receipts
fn verify_receipts_with_data(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    receipts_for_year: Vec<Receipt>,
) -> Result<VerificationResult, String> {
    use crate::models::MismatchReason;

    // Get all trips for this vehicle/year
    let all_trips = db
        .get_trips_for_vehicle_in_year(vehicle_id, year)
        .map_err(|e| e.to_string())?;

    // Separate trips with fuel and trips with other costs
    let trips_with_fuel: Vec<_> = all_trips
        .iter()
        .filter(|t| t.fuel_liters.is_some())
        .collect();
    let trips_with_other_costs: Vec<_> = all_trips
        .iter()
        .filter(|t| t.other_costs_eur.is_some())
        .collect();

    let mut verifications = Vec::new();
    let mut matched_count = 0;

    for receipt in &receipts_for_year {
        let mut matched = false;
        let mut matched_trip_id = None;
        let mut matched_trip_date = None;
        let mut matched_trip_route = None;
        let mut mismatch_reason = MismatchReason::None;

        // Check if receipt has the necessary data for fuel matching
        let has_fuel_data = receipt.receipt_datetime.is_some()
            && receipt.liters.is_some()
            && receipt.total_price_eur.is_some();

        // Track closest match for determining specific mismatch reason
        // (date_match, liters_match, price_match, trip_date_str)
        let mut closest_match: Option<(bool, bool, bool, String)> = None;

        // 1. Try to match FUEL receipts (has liters) to fuel trips
        if let (Some(receipt_datetime), Some(receipt_liters), Some(receipt_price)) = (
            receipt.receipt_datetime,
            receipt.liters,
            receipt.total_price_eur,
        ) {
            for trip in &trips_with_fuel {
                if let (Some(trip_liters), Some(trip_price)) =
                    (trip.fuel_liters, trip.fuel_cost_eur)
                {
                    // Match by datetime within trip range, liters (within small tolerance), and price (within small tolerance)
                    let datetime_match = is_datetime_in_trip_range(receipt_datetime, trip);
                    let liters_match = (trip_liters - receipt_liters).abs() < 0.01;
                    let price_match = (trip_price - receipt_price).abs() < 0.01;

                    if datetime_match && liters_match && price_match {
                        matched = true;
                        matched_trip_id = Some(trip.id.to_string());
                        matched_trip_date = Some(
                            trip.start_datetime
                                .date()
                                .format(date_formats::ISO_DATE)
                                .to_string(),
                        );
                        matched_trip_route =
                            Some(format!("{} - {}", trip.origin, trip.destination));
                        break;
                    }

                    // Track closest match (most fields matching)
                    let match_count = datetime_match as u8 + liters_match as u8 + price_match as u8;
                    if match_count >= 2 {
                        // At least 2 fields match - this is a close match
                        let trip_date_str =
                            trip.start_datetime.date().format("%-d.%-m.").to_string();
                        closest_match =
                            Some((datetime_match, liters_match, price_match, trip_date_str));
                    }
                }
            }

            // Determine mismatch reason for fuel receipts
            if !matched {
                if trips_with_fuel.is_empty() {
                    mismatch_reason = MismatchReason::NoFuelTripFound;
                } else if let Some((datetime_in_range, liters_match, price_match, ref trip_date)) =
                    closest_match
                {
                    // Prioritize: datetime > liters > price (most common user error is datetime)
                    if !datetime_in_range && liters_match && price_match {
                        // Find the trip with matching liters+price to check if it's a time vs date issue
                        let matching_trip = trips_with_fuel.iter().find(|t| {
                            t.fuel_liters.map(|l| (l - receipt_liters).abs() < 0.01).unwrap_or(false)
                                && t.fuel_cost_eur.map(|p| (p - receipt_price).abs() < 0.01).unwrap_or(false)
                        });

                        if let Some(trip) = matching_trip {
                            // Check if date is the same but time is outside range
                            let same_date = receipt_datetime.date() == trip.start_datetime.date();
                            if same_date {
                                // Time is the issue, not date
                                let trip_end = trip.end_datetime.unwrap_or(trip.start_datetime);
                                mismatch_reason = MismatchReason::DatetimeOutOfRange {
                                    receipt_time: receipt_datetime.format("%H:%M").to_string(),
                                    trip_start: trip.start_datetime.format("%H:%M").to_string(),
                                    trip_end: trip_end.format("%H:%M").to_string(),
                                };
                            } else {
                                // Different date
                                mismatch_reason = MismatchReason::DateMismatch {
                                    receipt_date: receipt_datetime.date().format("%-d.%-m.").to_string(),
                                    closest_trip_date: trip_date.clone(),
                                };
                            }
                        } else {
                            mismatch_reason = MismatchReason::DateMismatch {
                                receipt_date: receipt_datetime.date().format("%-d.%-m.").to_string(),
                                closest_trip_date: trip_date.clone(),
                            };
                        }
                    } else if datetime_in_range && !liters_match && price_match {
                        // Find trip where receipt datetime falls within [start, end] range
                        let trip_liters = trips_with_fuel
                            .iter()
                            .find(|t| is_datetime_in_trip_range(receipt_datetime, t))
                            .and_then(|t| t.fuel_liters)
                            .unwrap_or(0.0);
                        mismatch_reason = MismatchReason::LitersMismatch {
                            receipt_liters,
                            trip_liters,
                        };
                    } else if datetime_in_range && liters_match && !price_match {
                        // Find trip where receipt datetime falls within [start, end] range
                        let trip_price = trips_with_fuel
                            .iter()
                            .find(|t| is_datetime_in_trip_range(receipt_datetime, t))
                            .and_then(|t| t.fuel_cost_eur)
                            .unwrap_or(0.0);
                        mismatch_reason = MismatchReason::PriceMismatch {
                            receipt_price,
                            trip_price,
                        };
                    } else {
                        // Multiple fields don't match - show as no matching trip
                        mismatch_reason = MismatchReason::NoFuelTripFound;
                    }
                } else {
                    mismatch_reason = MismatchReason::NoFuelTripFound;
                }
            }
        } else if !has_fuel_data && receipt.liters.is_some() {
            // Has liters but missing date or price
            mismatch_reason = MismatchReason::MissingReceiptData;
        }

        // 2. If not matched as fuel, try to match "other cost" receipts by price
        if !matched && mismatch_reason == MismatchReason::None {
            if let Some(receipt_price) = receipt.total_price_eur {
                for trip in &trips_with_other_costs {
                    if let Some(trip_other_costs) = trip.other_costs_eur {
                        // Match by price (within small tolerance)
                        let price_match = (trip_other_costs - receipt_price).abs() < 0.01;

                        if price_match {
                            matched = true;
                            matched_trip_id = Some(trip.id.to_string());
                            matched_trip_date = Some(
                                trip.start_datetime
                                    .date()
                                    .format(date_formats::ISO_DATE)
                                    .to_string(),
                            );
                            matched_trip_route =
                                Some(format!("{} - {}", trip.origin, trip.destination));
                            break;
                        }
                    }
                }

                // Set mismatch reason for other-cost receipts (non-fuel)
                if !matched && receipt.liters.is_none() {
                    mismatch_reason = MismatchReason::NoOtherCostMatch;
                }
            } else if receipt.liters.is_none() {
                // No price and no liters - missing data
                mismatch_reason = MismatchReason::MissingReceiptData;
            }
        }

        if matched {
            matched_count += 1;
            mismatch_reason = MismatchReason::None;
        }

        verifications.push(ReceiptVerification {
            receipt_id: receipt.id.to_string(),
            matched,
            matched_trip_id,
            matched_trip_date,
            matched_trip_route,
            mismatch_reason,
        });
    }

    let total = verifications.len();
    Ok(VerificationResult {
        total,
        matched: matched_count,
        unmatched: total - matched_count,
        receipts: verifications,
    })
}

/// Verify receipts against trips by matching date, liters, and price.
/// Returns verification status for each receipt in the given year.
/// Only considers receipts that are unassigned or assigned to this vehicle.
#[tauri::command]
pub fn verify_receipts(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<VerificationResult, String> {
    verify_receipts_internal(&db, &vehicle_id, year)
}
