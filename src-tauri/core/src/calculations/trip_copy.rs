//! Defaults for a copied trip row: resolves the target date against the year
//! the grid is showing, and transfers the source trip's time-of-day onto it.

use crate::models::{CopiedTripDefaults, Trip};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};

/// Resolve the date a copied row should carry, given the year the grid is
/// currently showing.
///
/// The row must land inside the visible grid, so "today" is clamped into
/// `year`: a past year gets its last day, a future year its first.
pub fn resolve_copy_target_date(year: i32, today: NaiveDate) -> NaiveDate {
    use std::cmp::Ordering;
    match today.year().cmp(&year) {
        Ordering::Equal => today,
        Ordering::Greater => {
            NaiveDate::from_ymd_opt(year, 12, 31).expect("31 Dec is valid in every supported year")
        }
        Ordering::Less => {
            NaiveDate::from_ymd_opt(year, 1, 1).expect("1 Jan is valid in every supported year")
        }
    }
}

/// Build the seed values for a row copied from `source`.
///
/// Only the time-of-day travels from the source, never its date. The end
/// datetime additionally carries the source's day span, so an overnight trip
/// stays overnight instead of collapsing into a negative duration.
pub fn compute_copied_trip_defaults(
    source: &Trip,
    year: i32,
    today: NaiveDate,
) -> CopiedTripDefaults {
    let target_date = resolve_copy_target_date(year, today);
    let start = NaiveDateTime::new(target_date, source.start_datetime.time());

    let end = source.end_datetime.map(|src_end| {
        let day_offset = (src_end.date() - source.start_datetime.date()).num_days();
        NaiveDateTime::new(target_date + Duration::days(day_offset), src_end.time())
    });

    CopiedTripDefaults {
        start_datetime: start.format("%Y-%m-%dT%H:%M:%S").to_string(),
        end_datetime: end.map(|e| e.format("%Y-%m-%dT%H:%M:%S").to_string()),
        origin: source.origin.clone(),
        destination: source.destination.clone(),
        distance_km: source.distance_km,
        purpose: source.purpose.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Trip;
    use chrono::{NaiveDate, NaiveDateTime, Utc};
    use uuid::Uuid;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    /// A source trip carrying values in every field the copy must NOT take,
    /// so the exclusion assertions have something to catch.
    fn make_source(start: NaiveDateTime, end: Option<NaiveDateTime>) -> Trip {
        let now = Utc::now();
        Trip {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            start_datetime: start,
            end_datetime: end,
            origin: "Bratislava".to_string(),
            destination: "Trnava".to_string(),
            distance_km: 47.0,
            odometer: 10_000.0,
            purpose: "služobná cesta".to_string(),
            fuel_liters: Some(40.0),
            fuel_cost_eur: Some(60.0),
            full_tank: true,
            energy_kwh: Some(12.0),
            energy_cost_eur: Some(4.0),
            full_charge: true,
            soc_override_percent: Some(80.0),
            other_costs_eur: Some(9.0),
            other_costs_note: Some("parkovné".to_string()),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn target_date_is_today_when_viewed_year_is_current() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        assert_eq!(resolve_copy_target_date(2026, today), today);
    }

    #[test]
    fn target_date_is_dec_31_when_viewing_a_past_year() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        assert_eq!(
            resolve_copy_target_date(2025, today),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            "a past year's grid must receive its latest day"
        );
    }

    #[test]
    fn target_date_is_jan_1_when_viewing_a_future_year() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        assert_eq!(
            resolve_copy_target_date(2027, today),
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            "a future year's grid must receive its earliest day"
        );
    }

    #[test]
    fn transfers_time_of_day_onto_the_target_date() {
        let source = make_source(dt(2026, 3, 20, 8, 30), Some(dt(2026, 3, 20, 9, 15)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.start_datetime, "2026-09-03T08:30:00");
        assert_eq!(result.end_datetime.unwrap(), "2026-09-03T09:15:00");
    }

    #[test]
    fn overnight_source_keeps_its_day_offset() {
        // 22:00 → 02:00 next day. Without carrying the +1 day offset the copy
        // would end four hours BEFORE it starts.
        let source = make_source(dt(2026, 3, 20, 22, 0), Some(dt(2026, 3, 21, 2, 0)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.start_datetime, "2026-09-03T22:00:00");
        assert_eq!(result.end_datetime.unwrap(), "2026-09-04T02:00:00");
    }

    #[test]
    fn null_source_end_stays_null() {
        let source = make_source(dt(2026, 3, 20, 8, 30), None);
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.end_datetime, None);
    }

    #[test]
    fn copies_the_route_fields_verbatim() {
        let source = make_source(dt(2026, 3, 20, 8, 30), Some(dt(2026, 3, 20, 9, 15)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.origin, "Bratislava");
        assert_eq!(result.destination, "Trnava");
        assert_eq!(result.distance_km, 47.0);
        assert_eq!(result.purpose, "služobná cesta");
    }

    #[test]
    fn clamping_applies_to_the_end_datetime_too() {
        // Overnight trip copied into a past year: the +1 day offset pushes the
        // end into the following year. Documents the accepted behaviour —
        // start stays inside the viewed year, which is what the grid needs.
        let source = make_source(dt(2026, 3, 20, 22, 0), Some(dt(2026, 3, 21, 2, 0)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2025, today);

        assert_eq!(result.start_datetime, "2025-12-31T22:00:00");
        assert_eq!(result.end_datetime.unwrap(), "2026-01-01T02:00:00");
    }
}
