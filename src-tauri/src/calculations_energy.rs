//! Energy calculations for BEV and PHEV electricity tracking
//!
//! Parallel to calculations.rs but for kWh/battery instead of liters/fuel.
//! No margin calculation - BEV has no legal consumption limit.

/// Calculate energy consumption rate (kWh/100km) from a charge
/// Formula: (kwh / km_since_last_charge) * 100.0
pub fn calculate_consumption_rate_kwh(kwh: f64, km_since_last_charge: f64) -> f64 {
    if km_since_last_charge <= 0.0 {
        return 0.0;
    }
    (kwh / km_since_last_charge) * 100.0
}

/// Calculate energy used for a trip
/// Formula: distance_km * consumption_rate_kwh / 100.0
pub fn calculate_energy_used(distance_km: f64, consumption_rate_kwh: f64) -> f64 {
    distance_km * consumption_rate_kwh / 100.0
}

/// Calculate remaining battery (kWh)
/// Formula: previous - energy_used + energy_charged (clamped to [0, capacity])
pub fn calculate_battery_remaining(
    previous_kwh: f64,
    energy_used: f64,
    energy_charged: Option<f64>,
    battery_capacity: f64,
) -> f64 {
    let new_level = previous_kwh - energy_used + energy_charged.unwrap_or(0.0);
    new_level.min(battery_capacity).max(0.0)
}

/// Convert kWh to percentage of battery capacity
pub fn kwh_to_percent(kwh: f64, battery_capacity: f64) -> f64 {
    if battery_capacity <= 0.0 {
        return 0.0;
    }
    (kwh / battery_capacity) * 100.0
}

/// Convert percentage to kWh based on battery capacity
pub fn percent_to_kwh(percent: f64, battery_capacity: f64) -> f64 {
    if battery_capacity <= 0.0 {
        return 0.0;
    }
    percent * battery_capacity / 100.0
}

#[cfg(test)]
#[path = "calculations_energy_tests.rs"]
mod tests;
