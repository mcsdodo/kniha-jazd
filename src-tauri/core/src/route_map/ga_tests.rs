// `mod ga_tests` is declared in `route_map/mod.rs`, so `super` is `route_map`.
use super::ga::*;
use crate::route_map::Dataset;

fn seeded(seed: u64) -> SeededRouteRng {
    SeededRouteRng::new(seed)
}

#[test]
fn route_starts_and_ends_at_home() {
    let ds = Dataset::bundled();
    let r = generate_route(120.0, &ds, None, &mut seeded(42));
    assert_eq!(*r.sequence.first().unwrap(), 0);
    assert_eq!(*r.sequence.last().unwrap(), 0);
}

#[test]
fn route_uses_only_valid_node_indices() {
    let ds = Dataset::bundled();
    let r = generate_route(120.0, &ds, None, &mut seeded(7));
    assert!(r.sequence.iter().all(|&i| i < ds.len()));
}

#[test]
fn route_visits_no_node_twice() {
    let ds = Dataset::bundled();
    let r = generate_route(150.0, &ds, None, &mut seeded(11));
    let mut mids = r.sequence[1..r.sequence.len() - 1].to_vec();
    let before = mids.len();
    mids.sort_unstable();
    mids.dedup();
    assert_eq!(mids.len(), before, "intermediate stops must be unique");
}

#[test]
fn route_lands_within_tolerance_across_targets() {
    let ds = Dataset::bundled();
    for (i, target) in [50.0, 100.0, 150.0, 200.0, 300.0].iter().enumerate() {
        let r = generate_route(*target, &ds, None, &mut seeded(100 + i as u64));
        let err = ((r.total_km - target) / target).abs();
        assert!(
            err <= TOLERANCE,
            "target {target}: got {} ({:.1}% off)",
            r.total_km,
            err * 100.0
        );
    }
}

#[test]
fn different_seeds_produce_different_routes() {
    let ds = Dataset::bundled();
    let a = generate_route(120.0, &ds, None, &mut seeded(1));
    let b = generate_route(120.0, &ds, None, &mut seeded(2));
    assert_ne!(a.sequence, b.sequence, "variety is the feature");
}

#[test]
fn same_seed_reproduces_the_same_route() {
    let ds = Dataset::bundled();
    let a = generate_route(120.0, &ds, None, &mut seeded(9));
    let b = generate_route(120.0, &ds, None, &mut seeded(9));
    assert_eq!(a.sequence, b.sequence);
}

#[test]
fn total_km_matches_the_matrix_sum() {
    let ds = Dataset::bundled();
    let r = generate_route(90.0, &ds, None, &mut seeded(3));
    let sum: f64 = r.sequence.windows(2).map(|w| ds.distance(w[0], w[1])).sum();
    assert!((r.total_km - sum).abs() < 1e-9);
}

// A fixed stop count from the map page (2026-10-09).

fn stop_count(r: &RouteResult) -> usize {
    r.sequence.len() - 2
}

#[test]
fn a_fixed_stop_count_gives_exactly_that_many_unique_stops() {
    let ds = Dataset::bundled();
    for n in 1..=MAX_FIXED_STOPS {
        let r = generate_route(120.0, &ds, Some(n), &mut seeded(n as u64));
        assert_eq!(stop_count(&r), n, "stops {n}: {:?}", r.sequence);
        let mut mids = r.sequence[1..r.sequence.len() - 1].to_vec();
        mids.sort_unstable();
        mids.dedup();
        assert_eq!(mids.len(), n, "stops {n} must be unique");
    }
}

#[test]
fn auto_stop_count_stays_between_one_and_five() {
    let ds = Dataset::bundled();
    for seed in 0..20 {
        let r = generate_route(120.0, &ds, None, &mut seeded(seed));
        assert!((1..=5).contains(&stop_count(&r)), "{:?}", r.sequence);
    }
}

#[test]
fn a_stop_count_above_the_candidates_uses_every_candidate() {
    let ds = Dataset::bundled();
    let r = generate_route(5_000.0, &ds, Some(ds.len() + 3), &mut seeded(5));
    assert_eq!(stop_count(&r), ds.len() - 1);
}
