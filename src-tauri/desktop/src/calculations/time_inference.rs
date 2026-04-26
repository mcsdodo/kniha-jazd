//! Time inference for new trip rows from the most recent matching route.
//!
//! Given a base start time and base duration (extracted from the most recent
//! historical trip with the same origin/destination), produces a jittered
//! start/end datetime pair so consecutive auto-filled trips do not look
//! machine-identical. Jitter bounds: ±15 minutes on start, ±15% on duration.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

/// Source of randomness for time inference. Implementations supply the
/// per-call jitter values; production uses [`ThreadRngJitter`], tests use
/// a stub returning fixed values.
pub trait Jitter {
    /// Returns a value in the closed interval [-15, 15] (minutes).
    fn minutes(&mut self) -> i64;
    /// Returns a value in the closed interval [0.85, 1.15].
    fn duration_factor(&mut self) -> f64;
}

/// Production [`Jitter`] backed by `rand::thread_rng`.
pub struct ThreadRngJitter;

impl Jitter for ThreadRngJitter {
    fn minutes(&mut self) -> i64 {
        use rand::Rng;
        rand::thread_rng().gen_range(-15..=15)
    }
    fn duration_factor(&mut self) -> f64 {
        use rand::Rng;
        rand::thread_rng().gen_range(0.85..=1.15)
    }
}

/// Compute jittered start/end datetimes for a new trip row.
///
/// `row_date` is the date selected on the new row; the historical trip's
/// date is intentionally ignored — only its HH:MM start and its duration
/// inform the inference. Duration jitter is rounded to the nearest minute.
pub fn compute_inferred_times(
    row_date: NaiveDate,
    base_start: NaiveTime,
    base_duration_mins: i64,
    jitter: &mut dyn Jitter,
) -> (NaiveDateTime, NaiveDateTime) {
    let base_start_dt = NaiveDateTime::new(row_date, base_start);
    let start = base_start_dt + Duration::minutes(jitter.minutes());
    let jittered_duration_mins =
        (base_duration_mins as f64 * jitter.duration_factor()).round() as i64;
    let end = start + Duration::minutes(jittered_duration_mins);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubJitter {
        minutes: i64,
        factor: f64,
    }
    impl Jitter for StubJitter {
        fn minutes(&mut self) -> i64 {
            self.minutes
        }
        fn duration_factor(&mut self) -> f64 {
            self.factor
        }
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }
    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn applies_max_positive_jitter() {
        let row = date(2026, 4, 15);
        let base_start = time(8, 0);
        let base_duration = 60i64; // 60 min
        let mut j = StubJitter {
            minutes: 15,
            factor: 1.15,
        };
        let (start, end) = compute_inferred_times(row, base_start, base_duration, &mut j);
        assert_eq!(start, NaiveDateTime::new(row, time(8, 15)));
        // 60 * 1.15 = 69 → end = 08:15 + 69 min = 09:24
        assert_eq!(end, NaiveDateTime::new(row, time(9, 24)));
    }

    #[test]
    fn applies_zero_jitter() {
        let row = date(2026, 4, 15);
        let base_start = time(7, 30);
        let base_duration = 45i64;
        let mut j = StubJitter {
            minutes: 0,
            factor: 1.0,
        };
        let (start, end) = compute_inferred_times(row, base_start, base_duration, &mut j);
        assert_eq!(start, NaiveDateTime::new(row, time(7, 30)));
        assert_eq!(end, NaiveDateTime::new(row, time(8, 15)));
    }

    #[test]
    fn crosses_midnight_to_next_day() {
        let row = date(2026, 4, 15);
        let base_start = time(23, 50);
        let base_duration = 30i64;
        let mut j = StubJitter {
            minutes: 15,
            factor: 1.15,
        };
        // start: 23:50 + 15 = 00:05 next day (Apr 16)
        // duration: 30 * 1.15 = 34.5 → 35 (banker's rounding via round())
        // Actually f64::round rounds half away from zero: 34.5 → 35
        // end: 00:05 + 35 = 00:40 next day
        let (start, end) = compute_inferred_times(row, base_start, base_duration, &mut j);
        assert_eq!(start, NaiveDateTime::new(date(2026, 4, 16), time(0, 5)));
        assert_eq!(end, NaiveDateTime::new(date(2026, 4, 16), time(0, 40)));
    }

    #[test]
    fn rounds_duration_to_nearest_minute() {
        // 50 minutes * 0.91 = 45.5 minutes → 46 (round-half-away-from-zero)
        let row = date(2026, 4, 15);
        let mut j = StubJitter {
            minutes: 0,
            factor: 0.91,
        };
        let (start, end) = compute_inferred_times(row, time(10, 0), 50, &mut j);
        assert_eq!(start, NaiveDateTime::new(row, time(10, 0)));
        assert_eq!(end, NaiveDateTime::new(row, time(10, 46)));
    }
}
