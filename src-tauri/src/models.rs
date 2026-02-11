//! Data models for Vehicle, Trip, Route, Settings
//!
//! This module contains both domain models (Vehicle, Trip, etc.) and their
//! database row counterparts (VehicleRow, TripRow, etc.) for Diesel ORM.

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

use crate::schema::{receipts, routes, settings, trips, vehicles};

/// Vehicle powertrain type - determines which fields are required/displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VehicleType {
    #[default]
    Ice, // Internal combustion engine (existing behavior)
    Bev,  // Battery electric vehicle
    Phev, // Plug-in hybrid electric vehicle
}

impl fmt::Display for VehicleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VehicleType::Ice => write!(f, "ICE"),
            VehicleType::Bev => write!(f, "BEV"),
            VehicleType::Phev => write!(f, "PHEV"),
        }
    }
}

impl VehicleType {
    /// Returns true if this vehicle type uses fuel (ICE or PHEV)
    pub fn uses_fuel(&self) -> bool {
        matches!(self, VehicleType::Ice | VehicleType::Phev)
    }

    /// Returns true if this vehicle type uses electricity (BEV or PHEV)
    pub fn uses_electricity(&self) -> bool {
        matches!(self, VehicleType::Bev | VehicleType::Phev)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vehicle {
    pub id: Uuid,
    pub name: String,
    pub license_plate: String,
    pub vehicle_type: VehicleType,
    // Fuel system (ICE + PHEV) - None for BEV
    pub tank_size_liters: Option<f64>,
    pub tp_consumption: Option<f64>, // l/100km from technical passport
    // Energy system (BEV + PHEV) - None for ICE
    pub battery_capacity_kwh: Option<f64>,
    pub baseline_consumption_kwh: Option<f64>, // kWh/100km, user-defined
    pub initial_battery_percent: Option<f64>,  // Initial SoC % for first record (default: 100%)
    // Common fields
    pub initial_odometer: f64, // Starting ODO for "Prvý záznam"
    pub is_active: bool,
    pub vin: Option<String>,
    pub driver_name: Option<String>,
    // Home Assistant integration
    pub ha_odo_sensor: Option<String>, // HA sensor entity ID (e.g., "sensor.car_odometer")
    pub ha_fillup_sensor: Option<String>, // HA sensor entity ID for pushing suggested fillup
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
// Convenience constructors for testing and future use
impl Vehicle {
    /// Create a new ICE vehicle (backward compatible constructor)
    pub fn new(
        name: String,
        license_plate: String,
        tank_size_liters: f64,
        tp_consumption: f64,
        initial_odometer: f64,
    ) -> Self {
        Self::new_ice(
            name,
            license_plate,
            tank_size_liters,
            tp_consumption,
            initial_odometer,
        )
    }

    /// Create a new ICE (Internal Combustion Engine) vehicle
    pub fn new_ice(
        name: String,
        license_plate: String,
        tank_size_liters: f64,
        tp_consumption: f64,
        initial_odometer: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            license_plate,
            vehicle_type: VehicleType::Ice,
            tank_size_liters: Some(tank_size_liters),
            tp_consumption: Some(tp_consumption),
            battery_capacity_kwh: None,
            baseline_consumption_kwh: None,
            initial_battery_percent: None,
            initial_odometer,
            is_active: true,
            vin: None,
            driver_name: None,
            ha_odo_sensor: None,
            ha_fillup_sensor: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new BEV (Battery Electric Vehicle)
    pub fn new_bev(
        name: String,
        license_plate: String,
        battery_capacity_kwh: f64,
        baseline_consumption_kwh: f64,
        initial_odometer: f64,
        initial_battery_percent: Option<f64>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            license_plate,
            vehicle_type: VehicleType::Bev,
            tank_size_liters: None,
            tp_consumption: None,
            battery_capacity_kwh: Some(battery_capacity_kwh),
            baseline_consumption_kwh: Some(baseline_consumption_kwh),
            initial_battery_percent,
            initial_odometer,
            is_active: true,
            vin: None,
            driver_name: None,
            ha_odo_sensor: None,
            ha_fillup_sensor: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new PHEV (Plug-in Hybrid Electric Vehicle)
    pub fn new_phev(
        name: String,
        license_plate: String,
        tank_size_liters: f64,
        tp_consumption: f64,
        battery_capacity_kwh: f64,
        baseline_consumption_kwh: f64,
        initial_odometer: f64,
        initial_battery_percent: Option<f64>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            license_plate,
            vehicle_type: VehicleType::Phev,
            tank_size_liters: Some(tank_size_liters),
            tp_consumption: Some(tp_consumption),
            battery_capacity_kwh: Some(battery_capacity_kwh),
            baseline_consumption_kwh: Some(baseline_consumption_kwh),
            initial_battery_percent,
            initial_odometer,
            is_active: true,
            vin: None,
            driver_name: None,
            ha_odo_sensor: None,
            ha_fillup_sensor: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trip {
    pub id: Uuid,
    pub vehicle_id: Uuid,
    pub start_datetime: NaiveDateTime, // Trip start date + time
    pub end_datetime: Option<NaiveDateTime>, // Trip end date + time (optional)
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub odometer: f64,
    pub purpose: String,
    // Fuel system (ICE + PHEV)
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub full_tank: bool, // true = full tank fillup, false = partial
    // Energy system (BEV + PHEV)
    pub energy_kwh: Option<f64>,           // Energy charged
    pub energy_cost_eur: Option<f64>,      // Cost of charging
    pub full_charge: bool,                 // Charged to 100% (or target SoC)
    pub soc_override_percent: Option<f64>, // Manual SoC override for battery degradation (0-100)
    // Other costs
    pub other_costs_eur: Option<f64>,
    pub other_costs_note: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Trip {
    /// Returns true if this trip includes a fuel fillup
    pub fn is_fillup(&self) -> bool {
        self.fuel_liters.is_some()
    }

    /// Returns true if this trip includes a battery charge
    #[allow(dead_code)]
    pub fn is_charge(&self) -> bool {
        self.energy_kwh.is_some()
    }

    /// Returns true if this trip has a manual SoC override
    #[allow(dead_code)]
    // Helper methods for EV support - may be used in future features
    pub fn has_soc_override(&self) -> bool {
        self.soc_override_percent.is_some()
    }

    /// Create a test ICE trip with default values
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn test_ice_trip(
        date: NaiveDate,
        distance_km: f64,
        fuel_liters: Option<f64>,
        full_tank: bool,
    ) -> Self {
        let now = Utc::now();
        let start_datetime = date.and_hms_opt(0, 0, 0).unwrap();
        Self {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            start_datetime,
            end_datetime: None,
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km,
            odometer: 10000.0,
            purpose: "test".to_string(),
            fuel_liters,
            fuel_cost_eur: fuel_liters.map(|l| l * 1.5), // ~1.50€/L
            full_tank,
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: false,
            soc_override_percent: None,
            other_costs_eur: None,
            other_costs_note: None,
            sort_order: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub id: Uuid,
    pub vehicle_id: Uuid,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub usage_count: i32,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub id: Uuid,
    pub company_name: String,
    pub company_ico: String,
    pub buffer_trip_purpose: String,
    pub updated_at: DateTime<Utc>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            company_name: String::new(),
            company_ico: String::new(),
            buffer_trip_purpose: "služobná cesta".to_string(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TripStats {
    pub fuel_remaining_liters: f64,
    pub avg_consumption_rate: f64, // Average: total_fuel / total_km * 100
    pub last_consumption_rate: f64, // From last fill-up period (for margin calculation)
    pub margin_percent: Option<f64>, // None if no fill-up yet
    pub is_over_limit: bool,
    pub total_km: f64,
    pub total_fuel_liters: f64,
    pub total_fuel_cost_eur: f64,
    pub buffer_km: f64, // Additional km needed to reach 18% margin (0.0 if under target)
}

/// Suggested fuel fillup for a trip in an open period.
/// Pre-calculated to simplify the magic fill button (no backend call needed).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedFillup {
    /// Suggested liters to fill (targets 105-120% of TP rate)
    pub liters: f64,
    /// Resulting consumption rate if this fillup is used (l/100km)
    pub consumption_rate: f64,
}

/// Pre-calculated data for the trip grid display.
/// Eliminates need for frontend to duplicate calculation logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TripGridData {
    pub trips: Vec<Trip>,

    // Fuel data (ICE + PHEV)
    /// Consumption rate (l/100km) for each trip, keyed by trip ID
    pub rates: HashMap<String, f64>,
    /// Trip IDs that use estimated (TP) rate instead of calculated
    pub estimated_rates: HashSet<String>,
    /// Fuel consumed per trip in liters (km × rate / 100), keyed by trip ID
    pub fuel_consumed: HashMap<String, f64>,
    /// Fuel remaining after each trip, keyed by trip ID
    pub fuel_remaining: HashMap<String, f64>,
    /// Trip IDs with consumption over 120% of TP rate
    pub consumption_warnings: HashSet<String>,

    // Energy data (BEV + PHEV)
    /// Energy consumption rate (kWh/100km) for each trip, keyed by trip ID
    pub energy_rates: HashMap<String, f64>,
    /// Trip IDs that use estimated (baseline) energy rate
    pub estimated_energy_rates: HashSet<String>,
    /// Battery remaining (kWh) after each trip, keyed by trip ID
    pub battery_remaining_kwh: HashMap<String, f64>,
    /// Battery remaining (%) after each trip, keyed by trip ID
    pub battery_remaining_percent: HashMap<String, f64>,
    /// Trip IDs with manual SoC override
    pub soc_override_trips: HashSet<String>,

    // Shared warnings
    /// Trip IDs with date ordering issues
    pub date_warnings: HashSet<String>,
    /// Trip IDs that have fuel but are missing a matching receipt
    pub missing_receipts: HashSet<String>,
    /// Trip IDs where assigned receipt datetime is outside trip's [start, end] range
    pub receipt_datetime_warnings: HashSet<String>,
    /// Trip IDs where assigned receipt has mismatch_override = true (user confirmed mismatch)
    pub receipt_mismatch_overrides: HashSet<String>,

    // Year boundary data
    /// Starting odometer for this year (carryover from previous year's ending ODO)
    pub year_start_odometer: f64,
    /// Starting fuel (liters) for this year (carryover from previous year's ending fuel)
    /// Falls back to tank_size if no previous year data
    pub year_start_fuel: f64,

    // Suggested fillup (for trips in open period)
    /// Suggested fillup for each trip in an open period, keyed by trip ID.
    /// None for trips in closed periods or for BEV vehicles.
    pub suggested_fillup: HashMap<String, SuggestedFillup>,
    /// Legend suggestion: the most recent trip's suggestion (for display in legend).
    /// Calculated by backend to avoid frontend logic.
    pub legend_suggested_fillup: Option<SuggestedFillup>,

    // Legal compliance fields (2026)
    /// Trip sequence number (1-based, per year, chronological order)
    pub trip_numbers: HashMap<String, i32>,
    /// Odometer at trip START (derived from previous trip's ending odo)
    pub odometer_start: HashMap<String, f64>,
    /// Synthetic rows for month-end state display
    pub month_end_rows: Vec<MonthEndRow>,
}

/// Synthetic row for month-end state display (legal requirement)
/// Generated for months where no trip falls on the last calendar day.
/// Only contains odometer and fuel state — no trip number, no driver (display-only fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthEndRow {
    /// Last day of the month (e.g., 2026-01-31)
    pub date: NaiveDate,
    /// Odometer reading (same for start/end - no travel)
    pub odometer: f64,
    /// Fuel remaining in liters (carried from last trip state)
    pub fuel_remaining: f64,
    /// Month number 1-12 (for identification/sorting)
    pub month: u32,
    /// Sort key for chronological display (lastTripInMonth + 0.5)
    /// Frontend uses this to interleave month-end rows with trips
    pub sort_key: f64,
}

/// Status of a scanned receipt (OCR state only)
/// Assignment is determined by trip_id, not status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReceiptStatus {
    Pending,     // File detected, not yet parsed
    Parsed,      // Successfully parsed with high confidence
    NeedsReview, // Parsed but has uncertain fields
}

impl Default for ReceiptStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Typed confidence levels - prevents string inconsistencies
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ConfidenceLevel {
    #[default]
    Unknown,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldConfidence {
    pub liters: ConfidenceLevel,
    #[serde(alias = "total_price")] // Accept legacy snake_case from old DB records
    pub total_price: ConfidenceLevel,
    pub date: ConfidenceLevel,
}

/// Assignment type for receipt-to-trip relationship
/// User explicitly selects FUEL or OTHER when assigning receipt to trip
/// Stored in DB as TEXT using serde default serialization ("Fuel" or "Other")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentType {
    Fuel,  // Receipt is for fuel/refueling
    Other, // Receipt is for other costs (parking, toll, car wash, etc.)
}

impl AssignmentType {
    /// Parse from DB string (serde default format)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Fuel" => Some(AssignmentType::Fuel),
            "Other" => Some(AssignmentType::Other),
            _ => None,
        }
    }

    /// Convert to DB string (serde default format)
    pub fn as_str(&self) -> &'static str {
        match self {
            AssignmentType::Fuel => "Fuel",
            AssignmentType::Other => "Other",
        }
    }
}

/// A scanned fuel receipt (blocek)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub id: Uuid,
    pub vehicle_id: Option<Uuid>, // Set when assigned
    pub trip_id: Option<Uuid>,    // Set when assigned (UNIQUE when not null)
    pub file_path: String,        // Full path to image (UNIQUE)
    pub file_name: String,        // Just filename for display
    pub scanned_at: DateTime<Utc>,

    // Parsed fields (None = uncertain/failed)
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_datetime: Option<NaiveDateTime>,
    pub station_name: Option<String>,
    pub station_address: Option<String>,

    // Additional cost fields (for non-fuel receipts: car wash, parking, toll, service)
    pub vendor_name: Option<String>, // Shop/service provider name (e.g., "OMV", "AutoWash Express")
    pub cost_description: Option<String>, // Brief expense description (e.g., "Umytie auta", "Parkovanie 2h")

    // Multi-currency support: original OCR amount + currency (EUR, CZK, HUF, PLN)
    // - EUR receipts: original_amount copied to total_price_eur
    // - Foreign currency: user must manually convert to total_price_eur
    pub original_amount: Option<f64>,
    pub original_currency: Option<String>,

    // Year folder support: which year folder the receipt came from (e.g., 2024 from "2024/" folder)
    // None = flat folder structure, Some(year) = from year subfolder
    pub source_year: Option<i32>,

    // Status tracking
    pub status: ReceiptStatus,
    pub confidence: FieldConfidence,   // Typed struct, not strings
    pub raw_ocr_text: Option<String>,  // For debugging (local only)
    pub error_message: Option<String>, // If parsing failed

    // Assignment fields (Task 51: Receipt-Trip State Model)
    // Data invariant: trip_id = NULL ↔ assignment_type = NULL (unassigned)
    //                 trip_id = SET  ↔ assignment_type = SET  (assigned)
    pub assignment_type: Option<AssignmentType>, // Fuel or Other, set when assigned to trip
    pub mismatch_override: bool, // True = user confirmed data mismatch is intentional

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Receipt {
    #[allow(dead_code)]
    // Convenience constructor for testing
    pub fn new(file_path: String, file_name: String) -> Self {
        Self::new_with_source_year(file_path, file_name, None)
    }

    pub fn new_with_source_year(
        file_path: String,
        file_name: String,
        source_year: Option<i32>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            vehicle_id: None,
            trip_id: None,
            file_path,
            file_name,
            scanned_at: now,
            liters: None,
            total_price_eur: None,
            receipt_datetime: None,
            station_name: None,
            station_address: None,
            vendor_name: None,
            cost_description: None,
            original_amount: None,
            original_currency: None,
            source_year,
            status: ReceiptStatus::Pending,
            confidence: FieldConfidence::default(),
            raw_ocr_text: None,
            error_message: None,
            assignment_type: None,
            mismatch_override: false,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Reason why a receipt could not be matched to a trip
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MismatchReason {
    /// Receipt is verified - no mismatch
    None,
    /// Receipt missing date, liters, or price (OCR incomplete)
    MissingReceiptData,
    /// No trip with fuel data found for this year
    NoFuelTripFound,
    /// Found trip with matching liters+price but different date
    DateMismatch {
        #[serde(rename = "receiptDate")]
        receipt_date: String,
        #[serde(rename = "closestTripDate")]
        closest_trip_date: String,
    },
    /// Receipt datetime is outside trip's [start, end] time range (same date, wrong time)
    DatetimeOutOfRange {
        #[serde(rename = "receiptTime")]
        receipt_time: String,
        #[serde(rename = "tripStart")]
        trip_start: String,
        #[serde(rename = "tripEnd")]
        trip_end: String,
    },
    /// Found trip with matching date+price but different liters
    LitersMismatch {
        #[serde(rename = "receiptLiters")]
        receipt_liters: f64,
        #[serde(rename = "tripLiters")]
        trip_liters: f64,
    },
    /// Found trip with matching date+liters but different price
    PriceMismatch {
        #[serde(rename = "receiptPrice")]
        receipt_price: f64,
        #[serde(rename = "tripPrice")]
        trip_price: f64,
    },
    /// Other-cost receipt - no trip with matching price
    NoOtherCostMatch,
}

impl Default for MismatchReason {
    fn default() -> Self {
        MismatchReason::None
    }
}

// =============================================================================
// Receipt Display State (Task 51: Computed, never stored)
// =============================================================================

// =============================================================================
// Domain Enums - String Constant Replacements
// =============================================================================

/// Backup type - distinguishes manual from automatic pre-update backups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BackupType {
    #[default]
    Manual,
    PreUpdate,
}

impl BackupType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupType::Manual => "manual",
            BackupType::PreUpdate => "pre-update",
        }
    }
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Attachment status for receipt-to-trip matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentStatus {
    #[default]
    Empty, // Trip has no receipt attached
    Matches, // Receipt values match trip
    Differs, // Receipt values differ from trip
}

impl AttachmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AttachmentStatus::Empty => "empty",
            AttachmentStatus::Matches => "matches",
            AttachmentStatus::Differs => "differs",
        }
    }
}

/// Currency codes for multi-currency receipt support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Currency {
    #[default]
    EUR,
    CZK,
    HUF,
    PLN,
}

impl Currency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::EUR => "EUR",
            Currency::CZK => "CZK",
            Currency::HUF => "HUF",
            Currency::PLN => "PLN",
        }
    }

    /// Parse currency from string (case insensitive) - available for gradual adoption
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "EUR" => Some(Currency::EUR),
            "CZK" => Some(Currency::CZK),
            "HUF" => Some(Currency::HUF),
            "PLN" => Some(Currency::PLN),
            _ => None,
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Theme mode for UI appearance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Parse theme from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "system" => Some(Theme::System),
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Verification status of a single receipt against trips
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptVerification {
    pub receipt_id: String,
    pub matched: bool,
    pub matched_trip_id: Option<String>,
    /// Formatted as "D.M. HH:MM–HH:MM" (e.g., "22.1. 15:00–17:00")
    pub matched_trip_datetime: Option<String>,
    pub matched_trip_route: Option<String>,
    pub mismatch_reason: MismatchReason,
    /// True if receipt datetime is outside the matched trip's [start, end] range
    pub datetime_warning: bool,
    /// Trip time range for warning message (e.g., "09:00–11:34")
    pub matched_trip_time_range: Option<String>,
}

/// Result of verifying all receipts for a vehicle/year
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub total: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub receipts: Vec<ReceiptVerification>,
}

/// Preview result for live calculation feedback during trip editing.
/// Provides instant feedback on fuel consumption and remaining fuel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    /// Fuel remaining after this trip (liters)
    pub fuel_remaining: f64,
    /// Consumption rate for the fill-up period this trip belongs to (l/100km)
    pub consumption_rate: f64,
    /// Percentage over TP rate (e.g., 18.5 means 18.5% over TP)
    pub margin_percent: f64,
    /// True if margin exceeds 20% legal limit
    pub is_over_limit: bool,
    /// True if rate is estimated (no full-tank fill-up yet in this period)
    pub is_estimated_rate: bool,
}

// =============================================================================
// Diesel ORM Row Structs
// =============================================================================
// These structs map directly to database tables with String/i32 types.
// Use the From implementations to convert to/from domain models.

/// Database row for vehicles table
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = vehicles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct VehicleRow {
    pub id: Option<String>,
    pub name: String,
    pub license_plate: String,
    pub vehicle_type: String,
    pub tank_size_liters: Option<f64>,
    pub tp_consumption: Option<f64>,
    pub battery_capacity_kwh: Option<f64>,
    pub baseline_consumption_kwh: Option<f64>,
    pub initial_battery_percent: Option<f64>,
    pub initial_odometer: f64,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
    // Added via migration 2026-01-09-100000-add_vehicle_metadata (at end of table)
    pub vin: Option<String>,
    pub driver_name: Option<String>,
    // Added via migration 2026-01-27-100000_add_vehicle_ha_sensor
    pub ha_odo_sensor: Option<String>,
    // Added via migration 2026-02-11-100000_add_vehicle_ha_fillup_sensor
    pub ha_fillup_sensor: Option<String>,
}

/// For inserting new vehicles
#[derive(Debug, Insertable)]
#[diesel(table_name = vehicles)]
pub struct NewVehicleRow<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub license_plate: &'a str,
    pub vehicle_type: &'a str,
    pub tank_size_liters: Option<f64>,
    pub tp_consumption: Option<f64>,
    pub battery_capacity_kwh: Option<f64>,
    pub baseline_consumption_kwh: Option<f64>,
    pub initial_battery_percent: Option<f64>,
    pub initial_odometer: f64,
    pub is_active: i32,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    // Added via migration 2026-01-09-100000-add_vehicle_metadata (at end of table)
    pub vin: Option<&'a str>,
    pub driver_name: Option<&'a str>,
    // Added via migration 2026-01-27-100000_add_vehicle_ha_sensor
    pub ha_odo_sensor: Option<&'a str>,
    // Added via migration 2026-02-11-100000_add_vehicle_ha_fillup_sensor
    pub ha_fillup_sensor: Option<&'a str>,
}

/// Database row for trips table
/// IMPORTANT: Field order MUST match schema.rs column order for Diesel Queryable to work
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset, QueryableByName)]
#[diesel(table_name = trips)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TripRow {
    pub id: Option<String>,
    pub vehicle_id: String,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub odometer: f64,
    pub purpose: String,
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub other_costs_eur: Option<f64>,
    pub other_costs_note: Option<String>,
    // Note: column order matches actual database (migrations added columns at end)
    pub created_at: String,
    pub updated_at: String,
    pub sort_order: i32,
    pub full_tank: i32,
    pub energy_kwh: Option<f64>,
    pub energy_cost_eur: Option<f64>,
    pub full_charge: Option<i32>,
    pub soc_override_percent: Option<f64>,
    pub start_datetime: String,
    pub end_datetime: Option<String>,
}

/// For inserting new trips
#[derive(Debug, Insertable)]
#[diesel(table_name = trips)]
pub struct NewTripRow<'a> {
    pub id: &'a str,
    pub vehicle_id: &'a str,
    pub origin: &'a str,
    pub destination: &'a str,
    pub distance_km: f64,
    pub odometer: f64,
    pub purpose: &'a str,
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub other_costs_eur: Option<f64>,
    pub other_costs_note: Option<&'a str>,
    pub full_tank: i32,
    pub sort_order: i32,
    pub energy_kwh: Option<f64>,
    pub energy_cost_eur: Option<f64>,
    pub full_charge: Option<i32>,
    pub soc_override_percent: Option<f64>,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub start_datetime: &'a str,
    pub end_datetime: Option<&'a str>,
}

/// Database row for routes table
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = routes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RouteRow {
    pub id: Option<String>,
    pub vehicle_id: String,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub usage_count: i32,
    pub last_used: String,
}

/// For inserting new routes
#[derive(Debug, Insertable)]
#[diesel(table_name = routes)]
pub struct NewRouteRow<'a> {
    pub id: &'a str,
    pub vehicle_id: &'a str,
    pub origin: &'a str,
    pub destination: &'a str,
    pub distance_km: f64,
    pub usage_count: i32,
    pub last_used: &'a str,
}

/// Database row for settings table
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = settings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SettingsRow {
    pub id: Option<String>,
    pub company_name: String,
    pub company_ico: String,
    pub buffer_trip_purpose: String,
    pub updated_at: String,
}

/// For inserting new settings
#[derive(Debug, Insertable)]
#[diesel(table_name = settings)]
pub struct NewSettingsRow<'a> {
    pub id: &'a str,
    pub company_name: &'a str,
    pub company_ico: &'a str,
    pub buffer_trip_purpose: &'a str,
    pub updated_at: &'a str,
}

/// Database row for receipts table
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset, QueryableByName)]
#[diesel(table_name = receipts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ReceiptRow {
    pub id: Option<String>,
    pub vehicle_id: Option<String>,
    pub trip_id: Option<String>,
    pub file_path: String,
    pub file_name: String,
    pub scanned_at: String,
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_datetime: Option<String>,
    pub station_name: Option<String>,
    pub station_address: Option<String>,
    pub source_year: Option<i32>,
    pub status: String,
    pub confidence: String,
    pub raw_ocr_text: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub vendor_name: Option<String>,
    pub cost_description: Option<String>,
    // Multi-currency support (migration 2026-01-21-100000)
    pub original_amount: Option<f64>,
    pub original_currency: Option<String>,
    // Assignment fields (migration 2026-02-03-100000_receipt_assignment_type)
    pub assignment_type: Option<String>, // "Fuel" or "Other"
    pub mismatch_override: i32,          // 0 = no override, 1 = user confirmed
}

/// For inserting new receipts
#[derive(Debug, Insertable)]
#[diesel(table_name = receipts)]
pub struct NewReceiptRow<'a> {
    pub id: &'a str,
    pub vehicle_id: Option<&'a str>,
    pub trip_id: Option<&'a str>,
    pub file_path: &'a str,
    pub file_name: &'a str,
    pub scanned_at: &'a str,
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_datetime: Option<&'a str>,
    pub station_name: Option<&'a str>,
    pub station_address: Option<&'a str>,
    pub source_year: Option<i32>,
    pub status: &'a str,
    pub confidence: &'a str,
    pub raw_ocr_text: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub vendor_name: Option<&'a str>,
    pub cost_description: Option<&'a str>,
    // Multi-currency support (migration 2026-01-21-100000)
    pub original_amount: Option<f64>,
    pub original_currency: Option<&'a str>,
    // Assignment fields (migration 2026-02-03-100000_receipt_assignment_type)
    pub assignment_type: Option<&'a str>, // "Fuel" or "Other"
    pub mismatch_override: i32,           // 0 = no override, 1 = user confirmed
}

// =============================================================================
// Conversion implementations: Row <-> Domain
// =============================================================================

impl From<VehicleRow> for Vehicle {
    fn from(row: VehicleRow) -> Self {
        Vehicle {
            id: Uuid::parse_str(row.id.as_deref().unwrap_or_default())
                .unwrap_or_else(|_| Uuid::new_v4()),
            name: row.name,
            license_plate: row.license_plate,
            vehicle_type: match row.vehicle_type.as_str() {
                "Bev" => VehicleType::Bev,
                "Phev" => VehicleType::Phev,
                _ => VehicleType::Ice,
            },
            tank_size_liters: row.tank_size_liters,
            tp_consumption: row.tp_consumption,
            battery_capacity_kwh: row.battery_capacity_kwh,
            baseline_consumption_kwh: row.baseline_consumption_kwh,
            initial_battery_percent: row.initial_battery_percent,
            initial_odometer: row.initial_odometer,
            is_active: row.is_active != 0,
            vin: row.vin,
            driver_name: row.driver_name,
            ha_odo_sensor: row.ha_odo_sensor,
            ha_fillup_sensor: row.ha_fillup_sensor,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

impl From<TripRow> for Trip {
    fn from(row: TripRow) -> Self {
        // Parse start_datetime
        let start_datetime =
            NaiveDateTime::parse_from_str(&row.start_datetime, "%Y-%m-%dT%H:%M:%S")
                .unwrap_or_else(|_| Utc::now().naive_utc());

        // Parse end_datetime (optional)
        let end_datetime = row
            .end_datetime
            .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").ok());

        Trip {
            id: Uuid::parse_str(row.id.as_deref().unwrap_or_default())
                .unwrap_or_else(|_| Uuid::new_v4()),
            vehicle_id: Uuid::parse_str(&row.vehicle_id).unwrap_or_else(|_| Uuid::new_v4()),
            start_datetime,
            end_datetime,
            origin: row.origin,
            destination: row.destination,
            distance_km: row.distance_km,
            odometer: row.odometer,
            purpose: row.purpose,
            fuel_liters: row.fuel_liters,
            fuel_cost_eur: row.fuel_cost_eur,
            full_tank: row.full_tank != 0,
            energy_kwh: row.energy_kwh,
            energy_cost_eur: row.energy_cost_eur,
            full_charge: row.full_charge.map(|v| v != 0).unwrap_or(false),
            soc_override_percent: row.soc_override_percent,
            other_costs_eur: row.other_costs_eur,
            other_costs_note: row.other_costs_note,
            sort_order: row.sort_order,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

impl From<RouteRow> for Route {
    fn from(row: RouteRow) -> Self {
        Route {
            id: Uuid::parse_str(row.id.as_deref().unwrap_or_default())
                .unwrap_or_else(|_| Uuid::new_v4()),
            vehicle_id: Uuid::parse_str(&row.vehicle_id).unwrap_or_else(|_| Uuid::new_v4()),
            origin: row.origin,
            destination: row.destination,
            distance_km: row.distance_km,
            usage_count: row.usage_count,
            last_used: DateTime::parse_from_rfc3339(&row.last_used)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

impl From<SettingsRow> for Settings {
    fn from(row: SettingsRow) -> Self {
        Settings {
            id: Uuid::parse_str(row.id.as_deref().unwrap_or_default())
                .unwrap_or_else(|_| Uuid::new_v4()),
            company_name: row.company_name,
            company_ico: row.company_ico,
            buffer_trip_purpose: row.buffer_trip_purpose,
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

impl From<ReceiptRow> for Receipt {
    fn from(row: ReceiptRow) -> Self {
        let status = match row.status.as_str() {
            "Pending" => ReceiptStatus::Pending,
            "Parsed" | "Assigned" => ReceiptStatus::Parsed, // Legacy "Assigned" → Parsed
            "NeedsReview" => ReceiptStatus::NeedsReview,
            _ => ReceiptStatus::Pending,
        };

        let confidence: FieldConfidence = serde_json::from_str(&row.confidence).unwrap_or_default();

        Receipt {
            id: Uuid::parse_str(row.id.as_deref().unwrap_or_default())
                .unwrap_or_else(|_| Uuid::new_v4()),
            vehicle_id: row.vehicle_id.and_then(|s| Uuid::parse_str(&s).ok()),
            trip_id: row.trip_id.and_then(|s| Uuid::parse_str(&s).ok()),
            file_path: row.file_path,
            file_name: row.file_name,
            scanned_at: DateTime::parse_from_rfc3339(&row.scanned_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            liters: row.liters,
            total_price_eur: row.total_price_eur,
            receipt_datetime: row
                .receipt_datetime
                .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").ok()),
            station_name: row.station_name,
            station_address: row.station_address,
            vendor_name: row.vendor_name,
            cost_description: row.cost_description,
            original_amount: row.original_amount,
            original_currency: row.original_currency,
            source_year: row.source_year,
            status,
            confidence,
            raw_ocr_text: row.raw_ocr_text,
            error_message: row.error_message,
            assignment_type: row
                .assignment_type
                .and_then(|s| AssignmentType::from_str(&s)),
            mismatch_override: row.mismatch_override != 0,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

// =============================================================================
// Helper functions for domain -> row conversion (used in db.rs)
// =============================================================================

impl Vehicle {
    /// Convert to row struct for Diesel operations
    pub fn to_vehicle_type_str(&self) -> &'static str {
        match self.vehicle_type {
            VehicleType::Ice => "Ice",
            VehicleType::Bev => "Bev",
            VehicleType::Phev => "Phev",
        }
    }
}

impl Receipt {
    /// Convert status to database string
    pub fn status_to_str(&self) -> &'static str {
        match self.status {
            ReceiptStatus::Pending => "Pending",
            ReceiptStatus::Parsed => "Parsed",
            ReceiptStatus::NeedsReview => "NeedsReview",
        }
    }

    /// Convert confidence to JSON string
    pub fn confidence_to_json(&self) -> String {
        serde_json::to_string(&self.confidence).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // TripRow datetime parsing tests (From<TripRow> for Trip)
    // ========================================================================

    /// Helper to create a TripRow with specified start_datetime
    fn make_trip_row(start_datetime: &str, end_datetime: Option<&str>) -> TripRow {
        TripRow {
            id: Some("00000000-0000-0000-0000-000000000001".to_string()),
            vehicle_id: "00000000-0000-0000-0000-000000000002".to_string(),
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 100.0,
            odometer: 10000.0,
            purpose: "test".to_string(),
            fuel_liters: None,
            fuel_cost_eur: None,
            other_costs_eur: None,
            other_costs_note: None,
            full_tank: 0,
            sort_order: 0,
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: None,
            soc_override_percent: None,
            created_at: "2026-01-15T00:00:00+00:00".to_string(),
            updated_at: "2026-01-15T00:00:00+00:00".to_string(),
            start_datetime: start_datetime.to_string(),
            end_datetime: end_datetime.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_trip_row_datetime_parsing_valid() {
        // Test valid start_datetime parsing
        let row = make_trip_row("2026-01-15T08:30:00", None);
        let trip: Trip = row.into();

        assert_eq!(
            trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2026-01-15T08:30:00"
        );
        assert_eq!(
            trip.start_datetime.date().format("%Y-%m-%d").to_string(),
            "2026-01-15"
        );
    }

    #[test]
    fn test_trip_row_end_datetime_parsing() {
        // Test end_datetime parsing when provided
        let row = make_trip_row("2026-01-15T08:30:00", Some("2026-01-15T17:00:00"));
        let trip: Trip = row.into();

        assert_eq!(
            trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2026-01-15T08:30:00"
        );
        assert!(trip.end_datetime.is_some());
        assert_eq!(
            trip.end_datetime
                .unwrap()
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            "2026-01-15T17:00:00"
        );
    }

    #[test]
    fn test_trip_row_datetime_midnight() {
        // Test edge case: midnight "2026-01-15T00:00:00" parses correctly
        let row = make_trip_row("2026-01-15T00:00:00", None);
        let trip: Trip = row.into();

        assert_eq!(
            trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2026-01-15T00:00:00"
        );
        assert_eq!(trip.start_datetime.format("%H:%M").to_string(), "00:00");
    }

    // ========================================================================
    // FieldConfidence serialization tests
    // ========================================================================

    #[test]
    fn test_confidence_parses_camelcase() {
        // New format with camelCase (post-migration and new receipts)
        let json = r#"{"liters":"High","totalPrice":"Medium","date":"Low"}"#;
        let confidence: FieldConfidence = serde_json::from_str(json).unwrap();

        assert_eq!(confidence.liters, ConfidenceLevel::High);
        assert_eq!(confidence.total_price, ConfidenceLevel::Medium);
        assert_eq!(confidence.date, ConfidenceLevel::Low);
    }

    #[test]
    fn test_confidence_parses_legacy_snake_case() {
        // Legacy format with snake_case (pre-migration records)
        let json = r#"{"liters":"High","total_price":"Medium","date":"Low"}"#;
        let confidence: FieldConfidence = serde_json::from_str(json).unwrap();

        assert_eq!(confidence.liters, ConfidenceLevel::High);
        assert_eq!(confidence.total_price, ConfidenceLevel::Medium);
        assert_eq!(confidence.date, ConfidenceLevel::Low);
    }

    #[test]
    fn test_confidence_serializes_to_camelcase() {
        // Ensure new records are saved with camelCase
        let confidence = FieldConfidence {
            liters: ConfidenceLevel::High,
            total_price: ConfidenceLevel::Medium,
            date: ConfidenceLevel::Low,
        };
        let json = serde_json::to_string(&confidence).unwrap();

        assert!(json.contains("totalPrice")); // camelCase
        assert!(!json.contains("total_price")); // NOT snake_case
    }
}
