//! Compensation trip suggestions

use crate::models::Route;
use rand::Rng;

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
}
