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
}
