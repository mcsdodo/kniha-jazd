//! Tests for suggestions module

use super::*;
use crate::models::Route;
use chrono::Utc;
use uuid::Uuid;

// Removed: test_random_target_margin_in_range and test_random_target_margin_varies
// These tested Rust's stdlib rng.gen_range(), not our code

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

    // 40km is within +-10% of 42km (range 37.8-46.2)
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

    // 30km is NOT within +-10% of 42km (range 37.8-46.2)
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
fn test_build_compensation_suggestion_falls_back_to_buffer() {
    let routes: Vec<Route> = vec![]; // No routes available

    let suggestion = build_compensation_suggestion(
        &routes,
        42.0,
        "Trnava Namestie",
        "sluzobna cesta",
    );

    // Should create buffer trip
    assert_eq!(suggestion.origin, "Trnava Namestie");
    assert_eq!(suggestion.destination, "Trnava Namestie");
    assert_eq!(suggestion.distance_km, 42.0);
    assert_eq!(suggestion.purpose, "sluzobna cesta");
}

#[test]
fn test_build_compensation_suggestion_buffer_uses_current_location() {
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
