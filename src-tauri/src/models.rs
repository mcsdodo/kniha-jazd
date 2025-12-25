//! Data models for Vehicle, Trip, Route, Settings

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: Uuid,
    pub name: String,
    pub license_plate: String,
    pub tank_size_liters: f64,
    pub tp_consumption: f64, // l/100km from technical passport
    pub initial_odometer: f64, // Starting ODO for "Prvý záznam"
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Vehicle {
    pub fn new(
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
            tank_size_liters,
            tp_consumption,
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
    pub fuel_liters: Option<f64>,
    pub fuel_cost_eur: Option<f64>,
    pub other_costs_eur: Option<f64>,
    pub other_costs_note: Option<String>,
    pub full_tank: bool, // true = full tank fillup, false = partial
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Trip {
    pub fn is_fillup(&self) -> bool {
        self.fuel_liters.is_some()
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
    pub zostatok_liters: f64,
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
    /// Fuel remaining (zostatok) after each trip, keyed by trip ID
    pub fuel_remaining: HashMap<String, f64>,
    /// Trip IDs with date ordering issues
    pub date_warnings: HashSet<String>,
    /// Trip IDs with consumption over 120% of TP rate
    pub consumption_warnings: HashSet<String>,
}
