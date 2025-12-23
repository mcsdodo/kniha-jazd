//! Core business logic: consumption, margin, zostatok calculations

/// Calculate fuel consumption rate (l/100km) from a fill-up
/// Formula: (liters / km_since_last_fillup) * 100.0
/// Returns 0.0 if km_since_last_fillup <= 0.0
pub fn calculate_consumption_rate(liters: f64, km_since_last_fillup: f64) -> f64 {
    if km_since_last_fillup <= 0.0 {
        return 0.0;
    }
    (liters / km_since_last_fillup) * 100.0
}

/// Calculate fuel used for a trip (spotreba)
/// Formula: distance_km * consumption_rate / 100.0
pub fn calculate_spotreba(distance_km: f64, consumption_rate: f64) -> f64 {
    distance_km * consumption_rate / 100.0
}

/// Calculate remaining fuel in tank (zostatok)
/// Formula: previous - spotreba + fuel_added (capped at tank_size)
/// Returns remaining fuel, never negative, never exceeding tank_size
pub fn calculate_zostatok(
    previous: f64,
    spotreba: f64,
    fuel_added: Option<f64>,
    tank_size: f64,
) -> f64 {
    let new_zostatok = previous - spotreba + fuel_added.unwrap_or(0.0);
    new_zostatok.min(tank_size).max(0.0)
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
mod tests {
    use super::*;

    #[test]
    fn test_consumption_rate_normal() {
        // 50 liters / 820 km = 6.0975... l/100km
        let rate = calculate_consumption_rate(50.0, 820.0);
        assert!((rate - 6.0975).abs() < 0.001);
    }

    #[test]
    fn test_consumption_rate_zero_km() {
        // Edge case: 0 km should return 0.0
        let rate = calculate_consumption_rate(50.0, 0.0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_consumption_rate_negative_km() {
        // Edge case: negative km should return 0.0
        let rate = calculate_consumption_rate(50.0, -100.0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_spotreba_normal() {
        // 370 km at 6.0 l/100km = 22.2 liters
        let spotreba = calculate_spotreba(370.0, 6.0);
        assert!((spotreba - 22.2).abs() < 0.01);
    }

    #[test]
    fn test_spotreba_zero_distance() {
        // Edge case: 0 km = 0 liters
        let spotreba = calculate_spotreba(0.0, 6.0);
        assert_eq!(spotreba, 0.0);
    }

    #[test]
    fn test_spotreba_zero_consumption() {
        // Edge case: 0 consumption rate = 0 liters
        let spotreba = calculate_spotreba(370.0, 0.0);
        assert_eq!(spotreba, 0.0);
    }

    #[test]
    fn test_zostatok_normal_trip() {
        // Start with 50L, use 5L, no refill = 45L remaining
        let zostatok = calculate_zostatok(50.0, 5.0, None, 66.0);
        assert!((zostatok - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_zostatok_with_fillup() {
        // Start with 30L, use 5L, add 40L = 65L (under 66L tank)
        let zostatok = calculate_zostatok(30.0, 5.0, Some(40.0), 66.0);
        assert!((zostatok - 65.0).abs() < 0.01);
    }

    #[test]
    fn test_zostatok_caps_at_tank_size() {
        // Start with 30L, use 5L, add 50L = would be 75L but capped at 66L
        let zostatok = calculate_zostatok(30.0, 5.0, Some(50.0), 66.0);
        assert!((zostatok - 66.0).abs() < 0.01);
    }

    #[test]
    fn test_zostatok_near_zero() {
        // Start with 5L, use 4.5L = 0.5L remaining
        let zostatok = calculate_zostatok(5.0, 4.5, None, 66.0);
        assert!((zostatok - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_margin_under_limit() {
        // 6.0 / 5.1 = 17.647% over TP rate
        let margin = calculate_margin_percent(6.0, 5.1);
        assert!((margin - 17.647).abs() < 0.1);
        assert!(is_within_legal_limit(margin));
    }

    #[test]
    fn test_margin_at_limit() {
        // 6.12 / 5.1 = exactly 20%
        let margin = calculate_margin_percent(6.12, 5.1);
        assert!((margin - 20.0).abs() < 0.1);
        assert!(is_within_legal_limit(margin));
    }

    #[test]
    fn test_margin_over_limit() {
        // 6.5 / 5.1 = 27.45% over TP rate
        let margin = calculate_margin_percent(6.5, 5.1);
        assert!((margin - 27.45).abs() < 0.1);
        assert!(!is_within_legal_limit(margin));
    }

    #[test]
    fn test_margin_under_tp() {
        // 4.5 / 5.1 = -11.76% (better than TP)
        let margin = calculate_margin_percent(4.5, 5.1);
        assert!(margin < 0.0);
        assert!(is_within_legal_limit(margin));
    }

    #[test]
    fn test_margin_zero_tp_rate() {
        // Edge case: tp_rate = 0 should return 0.0
        let margin = calculate_margin_percent(6.0, 0.0);
        assert_eq!(margin, 0.0);
    }

    #[test]
    fn test_buffer_km_over_margin() {
        // 65.49L filled, 820km driven, TP=5.1, target=18%
        // Target rate = 5.1 * 1.18 = 6.018
        // Required km = 65.49 * 100 / 6.018 = 1088.29
        // Buffer = 1088.29 - 820 = 268.29 km
        let buffer = calculate_buffer_km(65.49, 820.0, 5.1, 0.18);
        assert!((buffer - 268.29).abs() < 1.0);
    }

    #[test]
    fn test_buffer_km_already_under_target() {
        // 50L filled, 1000km driven, TP=5.1, target=18%
        // Target rate = 5.1 * 1.18 = 6.018
        // Required km = 50 * 100 / 6.018 = 830.93
        // Buffer = 830.93 - 1000 = -169.07 km (negative, so return 0.0)
        let buffer = calculate_buffer_km(50.0, 1000.0, 5.1, 0.18);
        assert_eq!(buffer, 0.0);
    }

    #[test]
    fn test_buffer_km_zero_tp_rate() {
        // Edge case: tp_rate = 0 should return 0.0
        let buffer = calculate_buffer_km(50.0, 800.0, 0.0, 0.18);
        assert_eq!(buffer, 0.0);
    }

    #[test]
    fn test_buffer_km_example_case() {
        // Example from task: 50L filled, 800km driven, TP=5.1, target=18%
        // Target rate = 5.1 * 1.18 = 6.018
        // Required km = 50 * 100 / 6.018 = 830.93
        // Buffer = 830.93 - 800 = 30.93 km
        let buffer = calculate_buffer_km(50.0, 800.0, 5.1, 0.18);
        assert!((buffer - 30.93).abs() < 1.0);
    }
}
