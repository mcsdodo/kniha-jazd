//! Compensation trip suggestions

use crate::models::Route;
use rand::Rng;

/// Represents a suggested compensation trip to adjust fuel consumption
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompensationSuggestion {
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub purpose: String,
    pub is_buffer: bool,
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
/// 3. If not found: Create buffer trip (current_location → current_location, buffer_km, buffer_purpose)
pub fn build_compensation_suggestion(
    routes: &[Route],
    buffer_km: f64,
    current_location: &str,
    buffer_purpose: &str,
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
            is_buffer: false,
        };
    }

    // Fall back to buffer trip
    CompensationSuggestion {
        origin: current_location.to_string(),
        destination: current_location.to_string(),
        distance_km: buffer_km,
        purpose: buffer_purpose.to_string(),
        is_buffer: true,
    }
}

#[cfg(test)]
#[path = "suggestions_tests.rs"]
mod tests;
