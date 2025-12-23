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
}
