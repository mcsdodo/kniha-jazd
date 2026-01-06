//! Core business logic: consumption, margin, fuel calculations

/// Calculate fuel consumption rate (l/100km) from a fill-up
/// Formula: (liters / km_since_last_fillup) * 100.0
/// Returns 0.0 if km_since_last_fillup <= 0.0
pub fn calculate_consumption_rate(liters: f64, km_since_last_fillup: f64) -> f64 {
    if km_since_last_fillup <= 0.0 {
        return 0.0;
    }
    (liters / km_since_last_fillup) * 100.0
}

/// Calculate fuel used for a trip
/// Formula: distance_km * consumption_rate / 100.0
pub fn calculate_fuel_used(distance_km: f64, consumption_rate: f64) -> f64 {
    distance_km * consumption_rate / 100.0
}

/// Calculate remaining fuel in tank after a trip
/// Formula: previous - fuel_used + fuel_added (capped at tank_size)
/// Returns remaining fuel, never negative, never exceeding tank_size
pub fn calculate_fuel_level(
    previous: f64,
    fuel_used: f64,
    fuel_added: Option<f64>,
    tank_size: f64,
) -> f64 {
    let new_level = previous - fuel_used + fuel_added.unwrap_or(0.0);
    new_level.min(tank_size).max(0.0)
}

/// Calculate margin percentage vs TP consumption
/// Formula: (consumption_rate / tp_rate - 1.0) * 100.0
/// Returns percentage over the TP (technical passport) rate
/// Returns 0.0 if tp_rate <= 0.0 to handle edge case
pub fn calculate_margin_percent(consumption_rate: f64, tp_rate: f64) -> f64 {
    if tp_rate <= 0.0 {
        return 0.0;
    }
    (consumption_rate / tp_rate - 1.0) * 100.0
}

/// Check if consumption is within legal limit (max 20% over TP)
/// Returns true if margin_percent <= 20.0
/// Uses small epsilon (0.001) to handle floating point precision issues
pub fn is_within_legal_limit(margin_percent: f64) -> bool {
    const LEGAL_LIMIT: f64 = 20.0;
    const EPSILON: f64 = 0.001;
    margin_percent <= LEGAL_LIMIT + EPSILON
}

/// Calculate buffer km needed to reach target margin
/// When consumption is over the target margin, this calculates how many additional
/// kilometers are needed to bring the consumption rate down to the target.
///
/// # Arguments
/// * `liters_filled` - Liters filled at the fill-up
/// * `km_driven` - Kilometers driven since last fill-up
/// * `tp_rate` - Technical passport consumption rate (l/100km)
/// * `target_margin` - Target margin as decimal (e.g., 0.18 for 18%)
///
/// # Returns
/// * Positive number: additional km needed to reach target margin
/// * 0.0: if already under target or tp_rate is 0
///
/// # Formula
/// 1. target_rate = tp_rate * (1.0 + target_margin)
/// 2. required_km = (liters_filled * 100.0) / target_rate
/// 3. buffer_km = required_km - km_driven
/// 4. Return 0.0 if result is negative (already under target)
pub fn calculate_buffer_km(
    liters_filled: f64,
    km_driven: f64,
    tp_rate: f64,
    target_margin: f64,
) -> f64 {
    // Handle edge case: tp_rate is 0
    if tp_rate <= 0.0 {
        return 0.0;
    }

    // Calculate target consumption rate at the desired margin
    let target_rate = tp_rate * (1.0 + target_margin);

    // Calculate required km to achieve target rate
    let required_km = (liters_filled * 100.0) / target_rate;

    // Calculate buffer (additional km needed)
    let buffer_km = required_km - km_driven;

    // Return 0.0 if already under target (negative buffer)
    if buffer_km < 0.0 {
        0.0
    } else {
        buffer_km
    }
}

#[cfg(test)]
#[path = "calculations_tests.rs"]
mod tests;
