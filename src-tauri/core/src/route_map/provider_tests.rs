//! Tests for the routing provider choice. `from_lookup` takes a closure, so
//! nothing here reads or writes the real process environment.

use super::provider::{build_provider, ProviderConfig};
use crate::constants::env_vars::{MOCK_ROUTER, SYGIC_API_KEY, SYGIC_REFERER};

fn lookup(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |k| pairs.iter().find(|(name, _)| *name == k).map(|(_, v)| v.to_string())
}

#[test]
fn no_key_selects_osrm() {
    assert!(matches!(ProviderConfig::from_lookup(lookup(&[])), ProviderConfig::Osrm));
}

#[test]
fn a_blank_key_counts_as_unset() {
    assert!(matches!(ProviderConfig::from_lookup(lookup(&[(SYGIC_API_KEY, "  ")])), ProviderConfig::Osrm));
}

#[test]
fn a_key_selects_sygic_with_the_optional_referer() {
    match ProviderConfig::from_lookup(lookup(&[(SYGIC_API_KEY, "k"), (SYGIC_REFERER, "https://r.test/")])) {
        ProviderConfig::Sygic { api_key, referer } => {
            assert_eq!(api_key, "k");
            assert_eq!(referer.as_deref(), Some("https://r.test/"));
        }
        _ => panic!("expected Sygic"),
    }
}

#[test]
fn the_mock_wins_over_a_real_key() {
    // A developer with SYGIC_API_KEY in the shell must never spend real
    // requests, or get real routes, from the integration suite.
    let cfg = ProviderConfig::from_lookup(lookup(&[(MOCK_ROUTER, "1"), (SYGIC_API_KEY, "k")]));
    assert!(matches!(cfg, ProviderConfig::Mock));
}

#[test]
fn osrm_refuses_a_non_empty_avoid_list() {
    // Review Focus 1: a saved route with an avoid list, recomputed after the
    // key is gone, must not come back from OSRM as if the avoid had worked.
    let err = build_provider(ProviderConfig::Osrm, vec!["cze:tolls".into()])
        .err()
        .expect("OSRM cannot avoid per country");
    assert!(err.contains("SYGIC_API_KEY"), "got: {err}");
}

#[test]
fn osrm_accepts_an_empty_avoid_list() {
    assert!(build_provider(ProviderConfig::Osrm, vec![]).is_ok());
}

#[tokio::test]
async fn the_mock_is_deterministic_and_reflects_the_avoid_list() {
    let plain = build_provider(ProviderConfig::Mock, vec![]).unwrap();
    let r = plain.fetch(&[(48.1486, 17.1077), (49.1951, 16.6068)]).await.unwrap();
    assert_eq!(r.road_km, 100.0);
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string()]);

    let avoiding = build_provider(ProviderConfig::Mock, vec!["cze:tolls".into()]).unwrap();
    let r = avoiding.fetch(&[(48.1486, 17.1077), (49.1951, 16.6068)]).await.unwrap();
    assert_eq!(r.road_km, 120.0);
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string()]);
}
