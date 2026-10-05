//! The unit-test table of task 89. Invented addresses only.

use super::*;
use std::collections::HashMap;

const HOME: &str = "Home St 1, Hometown";

/// Gives each invented place name a stable ID and builds legs in order.
struct Fixture {
    vehicle_id: Uuid,
    ids: HashMap<String, Uuid>,
    legs: Vec<Leg>,
    round_trips: HashSet<Uuid>,
    odometer: f64,
}

impl Fixture {
    fn new() -> Self {
        Self {
            vehicle_id: Uuid::new_v4(),
            ids: HashMap::new(),
            legs: Vec::new(),
            round_trips: HashSet::new(),
            odometer: 10_000.0,
        }
    }

    fn place(&mut self, name: &str) -> Uuid {
        *self.ids.entry(name.to_string()).or_insert_with(Uuid::new_v4)
    }

    /// `at` is "YYYY-MM-DD HH:MM". The odometer grows by `km` per leg.
    fn leg(&mut self, at: &str, from: &str, to: &str, km: f64, purpose: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.odometer += km;
        let leg = Leg {
            id,
            start: NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M").unwrap(),
            odometer: self.odometer,
            origin_place_id: self.place(from),
            destination_place_id: self.place(to),
            destination_name: to.to_string(),
            distance_km: km,
            purpose: purpose.to_string(),
        };
        self.legs.push(leg);
        id
    }

    fn round_trip(&mut self, leg_id: Uuid) {
        self.round_trips.insert(leg_id);
    }

    fn run(&mut self) -> Vec<Journey> {
        let home = self.place(HOME);
        group_journeys(self.vehicle_id, &self.legs, home, &self.round_trips)
    }
}

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

#[test]
fn trip_with_local_legs_and_return_two_days_later() {
    let mut fx = Fixture::new();
    let out_id = fx.leg("2026-03-02 07:00", HOME, "City A", 357.0, "Customer visit");
    fx.leg("2026-03-02 13:00", "City A", "City A Plant", 8.0, "Customer visit");
    fx.leg("2026-03-03 09:00", "City A Plant", "City A Office", 5.0, "Meeting");
    fx.leg("2026-03-03 16:00", "City A Office", "City A", 6.0, "Meeting");
    let back_id = fx.leg("2026-03-04 08:00", "City A", HOME, 357.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    let j = &journeys[0];
    assert_eq!(j.nights, Some(2));
    assert!(j.complete);
    assert_eq!(j.start, dt("2026-03-02 07:00"));
    assert_eq!(j.end, Some(dt("2026-03-04 08:00")));
    assert_eq!(j.total_km, 733.0);
    assert_eq!(j.leg_ids.len(), 5);
    assert_eq!(j.leg_ids[0], out_id);
    assert_eq!(j.leg_ids[4], back_id);
    assert_eq!(j.purposes, vec!["Customer visit", "Meeting", "Return"]);
    assert_eq!(j.vehicle_id, fx.vehicle_id);
}

#[test]
fn places_are_distinct_and_in_order() {
    let mut fx = Fixture::new();
    fx.leg("2026-04-06 06:00", HOME, "City A", 300.0, "Trip");
    fx.leg("2026-04-07 08:00", "City A", "City B", 120.0, "Trip");
    fx.leg("2026-04-08 08:00", "City B", "City A", 120.0, "Trip");
    fx.leg("2026-04-09 15:00", "City A", HOME, 300.0, "Trip");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].places, vec!["City A", "City B"]);
    assert_eq!(journeys[0].nights, Some(3));
}

#[test]
fn single_leg_day_trip_followed_by_departure_is_not_a_journey() {
    // Spec row 3 ("home -> Village, one leg, the next leg starts at home").
    // The second departure has a return leg, so it is the only journey; the
    // first leg must not show up, also not as an incomplete journey.
    let mut fx = Fixture::new();
    fx.leg("2026-05-04 08:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-05-05 08:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-05-05 12:00", "Village", HOME, 26.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].start, dt("2026-05-05 08:00"));
    assert!(journeys[0].complete);
}

#[test]
fn round_trip_map_on_newest_row_is_a_day_trip() {
    let mut fx = Fixture::new();
    let id = fx.leg("2026-05-04 08:00", HOME, "Village", 52.0, "Delivery");
    fx.round_trip(id);

    assert!(fx.run().is_empty());
}

#[test]
fn home_to_home_loop_is_not_a_journey() {
    let mut fx = Fixture::new();
    fx.leg("2026-05-04 08:00", HOME, HOME, 204.0, "Errands");

    assert!(fx.run().is_empty());
}

#[test]
fn same_day_chain_has_zero_nights() {
    let mut fx = Fixture::new();
    fx.leg("2026-06-01 07:00", HOME, "Workshop", 30.0, "Service");
    fx.leg("2026-06-01 10:00", "Workshop", "Other Town", 40.0, "Service");
    fx.leg("2026-06-01 15:00", "Other Town", HOME, 22.0, "Service");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].nights, Some(0));
    assert_eq!(journeys[0].total_km, 92.0);
}

#[test]
fn long_same_day_trip_counts_all_km() {
    let mut fx = Fixture::new();
    fx.leg("2026-06-02 04:00", HOME, "City A", 505.0, "Customer visit");
    fx.leg("2026-06-02 15:00", "City A", HOME, 506.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].nights, Some(0));
    assert_eq!(journeys[0].total_km, 1011.0);
}

#[test]
fn journey_across_month_boundary_overlaps_both_months() {
    let mut fx = Fixture::new();
    fx.leg("2026-01-27 07:00", HOME, "City A", 357.0, "Project");
    fx.leg("2026-02-10 16:00", "City A", HOME, 357.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    let j = &journeys[0];
    assert!(overlaps(j, d("2026-01-01"), d("2026-01-31")));
    assert!(overlaps(j, d("2026-02-01"), d("2026-02-28")));
    assert!(!overlaps(j, d("2026-03-01"), d("2026-03-31")));
    assert!(!overlaps(j, d("2025-12-01"), d("2025-12-31")));
}

#[test]
fn last_leg_leaves_home_without_return_is_incomplete() {
    let mut fx = Fixture::new();
    let id = fx.leg("2026-07-01 07:00", HOME, "City A", 357.0, "Project");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    let j = &journeys[0];
    assert!(!j.complete);
    assert_eq!(j.end, None);
    assert_eq!(j.nights, None);
    assert_eq!(j.leg_ids, vec![id]);
}

#[test]
fn incomplete_journey_is_open_at_the_end() {
    let mut fx = Fixture::new();
    fx.leg("2026-07-01 07:00", HOME, "City A", 357.0, "Project");
    let j = fx.run().remove(0);

    assert!(overlaps(&j, d("2026-12-01"), d("2026-12-31")));
    assert!(!overlaps(&j, d("2026-06-01"), d("2026-06-30")));
}

#[test]
fn broken_chain_is_incomplete_and_next_departure_starts_a_new_one() {
    // User decision: home -> A, A -> B, then home -> C. The first chain has no
    // return leg, so it is an incomplete journey, not dropped.
    let mut fx = Fixture::new();
    let first = fx.leg("2026-09-01 07:00", HOME, "City A", 357.0, "Project");
    let second = fx.leg("2026-09-02 07:00", "City A", "City B", 100.0, "Project");
    let next = fx.leg("2026-09-05 07:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-09-05 12:00", "Village", HOME, 52.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 2);
    assert_eq!(journeys[0].leg_ids, vec![first, second]);
    assert!(!journeys[0].complete);
    assert_eq!(journeys[0].end, None);
    assert_eq!(journeys[0].nights, None);
    assert_eq!(journeys[0].total_km, 457.0);
    assert_eq!(journeys[1].leg_ids[0], next);
    assert!(journeys[1].complete);
}

#[test]
fn broken_chain_is_not_open_after_the_next_departure() {
    // The broken chain has no end, but the car left home again on 2025-03-10.
    // So the chain cannot reach any range after that departure.
    let mut fx = Fixture::new();
    fx.leg("2025-03-01 07:00", HOME, "City A", 357.0, "Project");
    fx.leg("2025-03-02 07:00", "City A", "City B", 100.0, "Project");
    fx.leg("2025-03-10 07:00", HOME, "Village", 52.0, "Delivery");
    let broken = fx.run().remove(0);

    assert!(!broken.complete);
    assert_eq!(broken.end, None);
    assert!(!overlaps(&broken, d("2026-10-01"), d("2026-10-31")));
    assert!(!overlaps(&broken, d("2025-03-11"), d("2025-03-31")));
    assert!(overlaps(&broken, d("2025-03-05"), d("2025-03-05")));
    assert!(overlaps(&broken, d("2025-03-10"), d("2025-03-10")));
}

#[test]
fn chain_broken_by_a_loop_is_not_open_after_the_loop() {
    let mut fx = Fixture::new();
    fx.leg("2025-03-01 07:00", HOME, "City A", 357.0, "Project");
    fx.leg("2025-03-02 07:00", "City A", "City B", 100.0, "Project");
    fx.leg("2025-03-10 07:00", HOME, HOME, 40.0, "Errands");
    let broken = fx.run().remove(0);

    assert!(!overlaps(&broken, d("2025-04-01"), d("2025-04-30")));
}

#[test]
fn loop_after_open_chain_closes_it_as_incomplete() {
    let mut fx = Fixture::new();
    fx.leg("2026-09-01 07:00", HOME, "City A", 357.0, "Project");
    fx.leg("2026-09-02 07:00", "City A", "City B", 100.0, "Project");
    fx.leg("2026-09-05 07:00", HOME, HOME, 40.0, "Errands");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert!(!journeys[0].complete);
    assert_eq!(journeys[0].leg_ids.len(), 2);
}

#[test]
fn single_leg_then_loop_is_a_day_trip() {
    // A one-leg chain is a day trip when the next leg starts at home, also
    // when that next leg is a loop.
    let mut fx = Fixture::new();
    fx.leg("2026-09-01 07:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-09-02 07:00", HOME, HOME, 40.0, "Errands");

    assert!(fx.run().is_empty());
}

#[test]
fn same_start_datetime_is_ordered_by_odometer() {
    let mut fx = Fixture::new();
    // Inserted in the wrong order on purpose: the return leg first.
    let back = fx.leg("2026-08-03 09:00", "City A", HOME, 50.0, "Return");
    let out = fx.leg("2026-08-03 09:00", HOME, "City A", 50.0, "Errand");
    // Give the outbound leg the lower odometer.
    fx.legs[1].odometer = 1.0;

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].leg_ids, vec![out, back]);
}

#[test]
fn return_leg_without_departure_is_ignored() {
    let mut fx = Fixture::new();
    fx.leg("2026-01-02 08:00", "City A", HOME, 357.0, "Return");

    assert!(fx.run().is_empty());
}

#[test]
fn empty_purpose_is_not_listed() {
    let mut fx = Fixture::new();
    fx.leg("2026-10-01 07:00", HOME, "City A", 100.0, "");
    fx.leg("2026-10-01 17:00", "City A", HOME, 100.0, "Return");

    assert_eq!(fx.run()[0].purposes, vec!["Return"]);
}
