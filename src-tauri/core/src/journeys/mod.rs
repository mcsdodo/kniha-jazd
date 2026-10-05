//! Journey grouping (task 89, BIZ entry "Journeys away from home").
//!
//! A trip row is one leg. A journey is a chain of legs away from home. This
//! module is pure: no DB, no clock. `commands_internal::journeys_cmd` reads the
//! legs and the home mark, and calls `group_journeys` once per vehicle.

use std::collections::HashSet;

use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Leg {
    pub id: Uuid,
    pub start: NaiveDateTime,
    pub odometer: f64,
    pub origin_place_id: Uuid,
    pub destination_place_id: Uuid,
    pub destination_name: String,
    pub distance_km: f64,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Journey {
    pub vehicle_id: Uuid,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub nights: Option<i64>,
    pub total_km: f64,
    pub places: Vec<String>,
    pub purposes: Vec<String>,
    pub complete: bool,
    pub leg_ids: Vec<Uuid>,
}

/// Group the legs of ONE vehicle into journeys away from `home`.
///
/// Rules (BIZ entry "Journeys away from home"):
/// 1. A journey starts at a leg home -> not home.
/// 2. It ends at the next leg not home -> home.
/// 3. A home -> X leg is a day trip if it has a round-trip map, or if the next
///    leg starts at home (a departure or a loop).
/// 4. A home -> home leg is never a journey.
/// 5. A chain with no return leg is incomplete: at the end of the data, or when
///    a chain of two or more legs meets a new leg from home.
/// 6. Legs outside a chain are ignored.
pub fn group_journeys(
    vehicle_id: Uuid,
    legs: &[Leg],
    home: Uuid,
    round_trip_ids: &HashSet<Uuid>,
) -> Vec<Journey> {
    let mut sorted: Vec<&Leg> = legs.iter().collect();
    sorted.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.odometer.total_cmp(&b.odometer))
    });

    let mut journeys = Vec::new();
    let mut open: Option<Vec<&Leg>> = None;
    for leg in sorted {
        let from_home = leg.origin_place_id == home;
        let to_home = leg.destination_place_id == home;
        if from_home {
            // A leg from home closes an open chain without a return leg.
            // One leg: a day trip (rule 3). Two or more: incomplete (rule 5).
            if let Some(chain) = open.take() {
                if chain.len() >= 2 {
                    journeys.push(build(vehicle_id, &chain, home, false));
                }
            }
            if !to_home && !round_trip_ids.contains(&leg.id) {
                open = Some(vec![leg]);
            }
        } else if to_home {
            if let Some(mut chain) = open.take() {
                chain.push(leg);
                journeys.push(build(vehicle_id, &chain, home, true));
            }
        } else if let Some(chain) = open.as_mut() {
            chain.push(leg);
        }
    }
    if let Some(chain) = open {
        journeys.push(build(vehicle_id, &chain, home, false));
    }
    journeys
}

fn build(vehicle_id: Uuid, chain: &[&Leg], home: Uuid, complete: bool) -> Journey {
    let start = chain[0].start;
    let end = complete.then(|| chain[chain.len() - 1].start);
    let nights = end.map(|e| (e.date() - start.date()).num_days());

    let mut places: Vec<String> = Vec::new();
    let mut purposes: Vec<String> = Vec::new();
    for leg in chain {
        if leg.destination_place_id != home && !places.contains(&leg.destination_name) {
            places.push(leg.destination_name.clone());
        }
        let purpose = leg.purpose.trim();
        if !purpose.is_empty() && !purposes.iter().any(|p| p == purpose) {
            purposes.push(purpose.to_string());
        }
    }

    Journey {
        vehicle_id,
        start,
        end,
        nights,
        total_km: chain.iter().map(|l| l.distance_km).sum(),
        places,
        purposes,
        complete,
        leg_ids: chain.iter().map(|l| l.id).collect(),
    }
}

/// True if any day of the journey is inside `from..=to`. An incomplete
/// journey has no end, so it is open towards the future.
pub fn overlaps(journey: &Journey, from: NaiveDate, to: NaiveDate) -> bool {
    journey.start.date() <= to && journey.end.map_or(true, |e| e.date() >= from)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
