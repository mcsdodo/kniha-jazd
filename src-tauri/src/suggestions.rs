//! Compensation trip suggestions

use rand::Rng;

/// Generate random target margin between 16-19%
/// This makes consumption values look natural, not artificially consistent
pub fn generate_target_margin() -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0.16..=0.19)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
