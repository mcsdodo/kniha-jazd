//! Compensation trip suggestions

use crate::models::Route;
use rand::Rng;

/// Represents a suggested compensation trip to adjust fuel consumption
#[derive(Debug, Clone)]
pub struct CompensationSuggestion {
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub purpose: String,
}

/// Generate random target margin between 16-19%
/// This makes consumption values look natural, not artificially consistent
pub fn generate_target_margin() -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0.16..=0.19)
}

/// Find an existing route whose distance is within ±10% of the target km
/// Returns the route with closest distance to target, or None if no match
pub fn find_matching_route(routes: &[Route], target_km: f64) -> Option<&Route> {
    let tolerance = 0.10; // ±10%
    let min_km = target_km * (1.0 - tolerance);
    let max_km = target_km * (1.0 + tolerance);

    routes
        .iter()
        .filter(|route| route.distance_km >= min_km && route.distance_km <= max_km)
        .min_by(|a, b| {
            let diff_a = (a.distance_km - target_km).abs();
            let diff_b = (b.distance_km - target_km).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        })
}

/// Build a compensation suggestion to reach target margin
///
/// Logic:
/// 1. Try to find a matching route using find_matching_route(routes, buffer_km)
/// 2. If found: Use route's origin/destination/distance, purpose is first word of origin
/// 3. If not found: Create filler trip (current_location → current_location, buffer_km, filler_purpose)
pub fn build_compensation_suggestion(
    routes: &[Route],
    buffer_km: f64,
    current_location: &str,
    filler_purpose: &str,
) -> CompensationSuggestion {
    // Try to find a matching route
    if let Some(route) = find_matching_route(routes, buffer_km) {
        // Use the matched route's data
        // Purpose: first word of origin (simplified)
        let purpose = route
            .origin
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();

        return CompensationSuggestion {
            origin: route.origin.clone(),
            destination: route.destination.clone(),
            distance_km: route.distance_km,
            purpose,
        };
    }

    // Fall back to filler trip
    CompensationSuggestion {
        origin: current_location.to_string(),
        destination: current_location.to_string(),
        distance_km: buffer_km,
        purpose: filler_purpose.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Route;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_random_target_margin_in_range() {
        // Generate 100 values, all should be in 0.16-0.19 range
        for _ in 0..100 {
            let target = generate_target_margin();
            assert!(target >= 0.16, "Target {} is below 0.16", target);
            assert!(target <= 0.19, "Target {} is above 0.19", target);
        }
    }

    #[test]
    fn test_random_target_margin_varies() {
        // Generate multiple values and verify they're not all the same
        let mut values = Vec::new();
        for _ in 0..10 {
            values.push(generate_target_margin());
        }

        // Check that not all values are identical
        let first = values[0];
        let all_same = values.iter().all(|&v| (v - first).abs() < 0.0001);
        assert!(!all_same, "All values are the same: {:?}", values);
    }

    #[test]
    fn test_find_matching_route_exact_match() {
        let vehicle_id = Uuid::new_v4();
        let routes = vec![
            Route {
                id: Uuid::new_v4(),
                vehicle_id,
                origin: "A".to_string(),
                destination: "B".to_string(),
                distance_km: 50.0,
                usage_count: 1,
                last_used: Utc::now(),
            },
            Route {
                id: Uuid::new_v4(),
                vehicle_id,
                origin: "B".to_string(),
                destination: "C".to_string(),
                distance_km: 100.0,
                usage_count: 1,
                last_used: Utc::now(),
            },
            Route {
                id: Uuid::new_v4(),
                vehicle_id,
                origin: "A".to_string(),
                destination: "A".to_string(),
                distance_km: 42.0,
                usage_count: 1,
                last_used: Utc::now(),
            },
        ];

        // Exact match: target 42km should find 42km route
        let result = find_matching_route(&routes, 42.0);
        assert!(result.is_some());
        let route = result.unwrap();
        assert_eq!(route.distance_km, 42.0);
    }

    #[test]
    fn test_find_matching_route_within_10_percent() {
        let vehicle_id = Uuid::new_v4();
        let routes = vec![Route {
            id: Uuid::new_v4(),
            vehicle_id,
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 40.0,
            usage_count: 1,
            last_used: Utc::now(),
        }];

        // 40km is within ±10% of 42km (range 37.8-46.2)
        let result = find_matching_route(&routes, 42.0);
        assert!(result.is_some());
        let route = result.unwrap();
        assert_eq!(route.distance_km, 40.0);
    }

    #[test]
    fn test_find_matching_route_outside_10_percent() {
        let vehicle_id = Uuid::new_v4();
        let routes = vec![Route {
            id: Uuid::new_v4(),
            vehicle_id,
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: 30.0,
            usage_count: 1,
            last_used: Utc::now(),
        }];

        // 30km is NOT within ±10% of 42km (range 37.8-46.2)
        let result = find_matching_route(&routes, 42.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_matching_route_closest_match() {
        let vehicle_id = Uuid::new_v4();
        let routes = vec![
            Route {
                id: Uuid::new_v4(),
                vehicle_id,
                origin: "A".to_string(),
                destination: "B".to_string(),
                distance_km: 40.0, // Within range, diff = 2.0
                usage_count: 1,
                last_used: Utc::now(),
            },
            Route {
                id: Uuid::new_v4(),
                vehicle_id,
                origin: "C".to_string(),
                destination: "D".to_string(),
                distance_km: 41.5, // Within range, diff = 0.5 - CLOSEST
                usage_count: 1,
                last_used: Utc::now(),
            },
            Route {
                id: Uuid::new_v4(),
                vehicle_id,
                origin: "E".to_string(),
                destination: "F".to_string(),
                distance_km: 38.0, // Within range, diff = 4.0
                usage_count: 1,
                last_used: Utc::now(),
            },
        ];

        // Should return the closest match (41.5km)
        let result = find_matching_route(&routes, 42.0);
        assert!(result.is_some());
        let route = result.unwrap();
        assert_eq!(route.distance_km, 41.5);
    }

    #[test]
    fn test_find_matching_route_empty_routes() {
        let routes: Vec<Route> = vec![];

        let result = find_matching_route(&routes, 42.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_compensation_suggestion_uses_matching_route() {
        let vehicle_id = Uuid::new_v4();
        let routes = vec![Route {
            id: Uuid::new_v4(),
            vehicle_id,
            origin: "Bratislava Hlavna Stanica".to_string(),
            destination: "Kosice Centrum".to_string(),
            distance_km: 45.0,
            usage_count: 5,
            last_used: Utc::now(),
        }];

        let suggestion = build_compensation_suggestion(
            &routes,
            42.0, // 45km is within buffer of 42km
            "Bratislava Hlavna Stanica",
            "testovanie",
        );

        // Should use the matching route's data
        assert_eq!(suggestion.origin, "Bratislava Hlavna Stanica");
        assert_eq!(suggestion.destination, "Kosice Centrum");
        assert_eq!(suggestion.distance_km, 45.0);
        // Purpose should be first word of origin
        assert_eq!(suggestion.purpose, "Bratislava");
    }

    #[test]
    fn test_build_compensation_suggestion_falls_back_to_filler() {
        let routes: Vec<Route> = vec![]; // No routes available

        let suggestion = build_compensation_suggestion(
            &routes,
            42.0,
            "Trnava Namestie",
            "testovanie",
        );

        // Should create filler trip
        assert_eq!(suggestion.origin, "Trnava Namestie");
        assert_eq!(suggestion.destination, "Trnava Namestie");
        assert_eq!(suggestion.distance_km, 42.0);
        assert_eq!(suggestion.purpose, "testovanie");
    }

    #[test]
    fn test_build_compensation_suggestion_filler_uses_current_location() {
        let vehicle_id = Uuid::new_v4();
        let routes = vec![Route {
            id: Uuid::new_v4(),
            vehicle_id,
            origin: "Far Away Place".to_string(),
            destination: "Another Place".to_string(),
            distance_km: 200.0, // Too far from 42km
            usage_count: 1,
            last_used: Utc::now(),
        }];

        let suggestion = build_compensation_suggestion(
            &routes,
            42.0,
            "Nitra Centrum",
            "skusobna jazda",
        );

        // Should use current location for both origin and destination
        assert_eq!(suggestion.origin, "Nitra Centrum");
        assert_eq!(suggestion.destination, "Nitra Centrum");
        assert_eq!(suggestion.purpose, "skusobna jazda");
    }
}
