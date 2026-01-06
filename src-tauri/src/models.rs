//! Data models for Vehicle, Trip, Route, Settings

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

/// Vehicle powertrain type - determines which fields are required/displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VehicleType {
    #[default]
    Ice,  // Internal combustion engine (existing behavior)
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Vehicle {
    /// Create a new ICE vehicle (backward compatible constructor)
    pub fn new(
        name: String,
        license_plate: String,
        tank_size_liters: f64,
        tp_consumption: f64,
        initial_odometer: f64,
    ) -> Self {
        Self::new_ice(name, license_plate, tank_size_liters, tp_consumption, initial_odometer)
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
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trip {
    pub id: Uuid,
    pub vehicle_id: Uuid,
    pub date: NaiveDate,
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
    pub energy_kwh: Option<f64>,       // Energy charged
    pub energy_cost_eur: Option<f64>,  // Cost of charging
    pub full_charge: bool,             // Charged to 100% (or target SoC)
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
    pub fn is_charge(&self) -> bool {
        self.energy_kwh.is_some()
    }

    /// Returns true if this trip has a manual SoC override
    pub fn has_soc_override(&self) -> bool {
        self.soc_override_percent.is_some()
    }

    /// Create a test ICE trip with default values
    #[cfg(test)]
    pub fn test_ice_trip(
        date: NaiveDate,
        distance_km: f64,
        fuel_liters: Option<f64>,
        full_tank: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            date,
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
pub struct TripStats {
    pub fuel_remaining_liters: f64,
    pub avg_consumption_rate: f64,  // Average: total_fuel / total_km * 100
    pub last_consumption_rate: f64, // From last fill-up period (for margin calculation)
    pub margin_percent: Option<f64>, // None if no fill-up yet
    pub is_over_limit: bool,
    pub total_km: f64,
    pub total_fuel_liters: f64,
    pub total_fuel_cost_eur: f64,
}

/// Pre-calculated data for the trip grid display.
/// Eliminates need for frontend to duplicate calculation logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripGridData {
    pub trips: Vec<Trip>,
    /// Consumption rate (l/100km) for each trip, keyed by trip ID
    pub rates: HashMap<String, f64>,
    /// Trip IDs that use estimated (TP) rate instead of calculated
    pub estimated_rates: HashSet<String>,
    /// Fuel remaining after each trip, keyed by trip ID
    pub fuel_remaining: HashMap<String, f64>,
    /// Trip IDs with date ordering issues
    pub date_warnings: HashSet<String>,
    /// Trip IDs with consumption over 120% of TP rate
    pub consumption_warnings: HashSet<String>,
    /// Trip IDs that have fuel but are missing a matching receipt
    pub missing_receipts: HashSet<String>,
}

/// Status of a scanned receipt
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReceiptStatus {
    Pending,     // File detected, not yet parsed
    Parsed,      // Successfully parsed with high confidence
    NeedsReview, // Parsed but has uncertain fields
    Assigned,    // Linked to a trip
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
pub struct FieldConfidence {
    pub liters: ConfidenceLevel,
    pub total_price: ConfidenceLevel,
    pub date: ConfidenceLevel,
}

/// A scanned fuel receipt (blocek)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub vehicle_id: Option<Uuid>,  // Set when assigned
    pub trip_id: Option<Uuid>,     // Set when assigned (UNIQUE when not null)
    pub file_path: String,         // Full path to image (UNIQUE)
    pub file_name: String,         // Just filename for display
    pub scanned_at: DateTime<Utc>,

    // Parsed fields (None = uncertain/failed)
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_date: Option<NaiveDate>,
    pub station_name: Option<String>,
    pub station_address: Option<String>,

    // Year folder support: which year folder the receipt came from (e.g., 2024 from "2024/" folder)
    // None = flat folder structure, Some(year) = from year subfolder
    pub source_year: Option<i32>,

    // Status tracking
    pub status: ReceiptStatus,
    pub confidence: FieldConfidence, // Typed struct, not strings
    pub raw_ocr_text: Option<String>, // For debugging (local only)
    pub error_message: Option<String>, // If parsing failed

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Receipt {
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
            receipt_date: None,
            station_name: None,
            station_address: None,
            source_year,
            status: ReceiptStatus::Pending,
            confidence: FieldConfidence::default(),
            raw_ocr_text: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_assigned(&self) -> bool {
        self.trip_id.is_some()
    }
}

/// Verification status of a single receipt against trips
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptVerification {
    pub receipt_id: String,
    pub matched: bool,
    pub matched_trip_id: Option<String>,
    pub matched_trip_date: Option<String>,
    pub matched_trip_route: Option<String>,
}

/// Result of verifying all receipts for a vehicle/year
#[derive(Debug, Clone, Serialize, Deserialize)]
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
