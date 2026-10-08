//! Tests for the routing provider choice. `from_lookup` takes a closure, so
//! nothing here reads or writes the real process environment.

use super::provider::{
    build_provider, ProviderConfig, RouteProviderKind, AVOID_NEEDS_SYGIC, PROVIDER_NEEDS_SYGIC,
};
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
    let err = build_provider(ProviderConfig::Osrm, None, vec!["cze:tolls".into()])
        .err()
        .expect("OSRM cannot avoid per country");
    assert!(err.contains("SYGIC_API_KEY"), "got: {err}");
    assert!(err.starts_with(AVOID_NEEDS_SYGIC), "got: {err}");
}

#[test]
fn debug_output_hides_the_api_key() {
    let cfg = ProviderConfig::Sygic { api_key: "k-secret".into(), referer: None };
    let shown = format!("{cfg:?}");
    assert!(!shown.contains("k-secret"), "key leaked: {shown}");
}

#[test]
fn osrm_accepts_an_empty_avoid_list() {
    assert!(build_provider(ProviderConfig::Osrm, None, vec![]).is_ok());
}

#[tokio::test]
async fn the_mock_is_deterministic_and_reflects_the_avoid_list() {
    let plain = build_provider(ProviderConfig::Mock, Some(RouteProviderKind::Sygic), vec![]).unwrap();
    let r = plain.fetch(&[(48.1486, 17.1077), (49.1951, 16.6068)]).await.unwrap();
    assert_eq!(r.road_km, 100.0);
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string()]);

    let avoiding = build_provider(ProviderConfig::Mock, Some(RouteProviderKind::Sygic), vec!["cze:tolls".into()]).unwrap();
    let r = avoiding.fetch(&[(48.1486, 17.1077), (49.1951, 16.6068)]).await.unwrap();
    assert_eq!(r.road_km, 120.0);
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string()]);
}

// ============================================================================
// Task 86 -- the page picks the provider per request
// ============================================================================

fn sygic() -> ProviderConfig {
    ProviderConfig::Sygic { api_key: "k".into(), referer: None }
}

#[test]
fn no_request_means_osrm_even_with_a_sygic_key() {
    // OSRM is the default: the OpenStreetMap data has the D1 Visnove tunnel,
    // the Sygic map does not. Sygic is chosen explicitly.
    let p = build_provider(sygic(), None, vec![]).unwrap();
    assert_eq!(p.kind(), RouteProviderKind::Osrm);
    let p = build_provider(ProviderConfig::Osrm, None, vec![]).unwrap();
    assert_eq!(p.kind(), RouteProviderKind::Osrm);
}

#[test]
fn osrm_can_be_requested_while_a_sygic_key_is_set() {
    // The point of the task: Sygic's map lacks the Visnove tunnel, OSM has it.
    let p = build_provider(sygic(), Some(RouteProviderKind::Osrm), vec![]).unwrap();
    assert_eq!(p.kind(), RouteProviderKind::Osrm);
}

#[test]
fn sygic_requested_without_a_key_is_refused_not_downgraded() {
    let err = build_provider(ProviderConfig::Osrm, Some(RouteProviderKind::Sygic), vec![])
        .err()
        .expect("no key, no Sygic");
    assert!(err.starts_with(PROVIDER_NEEDS_SYGIC), "got: {err}");
}

#[test]
fn osrm_requested_with_an_avoid_list_is_refused_even_with_a_key() {
    let err = build_provider(sygic(), Some(RouteProviderKind::Osrm), vec!["cze:tolls".into()])
        .err()
        .expect("OSRM cannot avoid per country");
    assert!(err.starts_with(AVOID_NEEDS_SYGIC), "got: {err}");
}

#[test]
fn available_providers_follow_the_key() {
    let info = ProviderConfig::Osrm.info();
    assert_eq!(info.available, vec![RouteProviderKind::Osrm]);
    assert_eq!(info.default, RouteProviderKind::Osrm);

    let info = sygic().info();
    assert_eq!(info.available, vec![RouteProviderKind::Osrm, RouteProviderKind::Sygic]);
    assert_eq!(info.default, RouteProviderKind::Osrm);
}

#[test]
fn the_mock_offers_both_providers_like_a_keyed_server() {
    // The integration suite must be able to see and use the selector.
    let info = ProviderConfig::Mock.info();
    assert_eq!(info.available, vec![RouteProviderKind::Osrm, RouteProviderKind::Sygic]);
    assert_eq!(info.default, RouteProviderKind::Osrm);
}

#[tokio::test]
async fn the_mock_as_osrm_reports_osrm_its_own_km_and_no_avoid_options() {
    let p = build_provider(ProviderConfig::Mock, Some(RouteProviderKind::Osrm), vec![]).unwrap();
    assert_eq!(p.kind(), RouteProviderKind::Osrm);
    let r = p.fetch(&[(48.1486, 17.1077), (49.1951, 16.6068)]).await.unwrap();
    assert_eq!(r.road_km, 90.0);
    assert!(r.possible_avoids.is_empty());
}

#[test]
fn provider_kind_round_trips_through_text_and_json() {
    for k in [RouteProviderKind::Osrm, RouteProviderKind::Sygic] {
        assert_eq!(RouteProviderKind::parse(k.as_str()), Some(k));
        let json = serde_json::to_string(&k).unwrap();
        assert_eq!(json, format!("\"{}\"", k.as_str()));
    }
    assert_eq!(RouteProviderKind::parse("google"), None);
    assert!(serde_json::from_str::<RouteProviderKind>("\"google\"").is_err());
}

#[test]
fn an_avoid_list_without_a_provider_is_refused_by_the_osrm_default() {
    // No silent switch to Sygic either: the caller asks for it.
    let err = build_provider(sygic(), None, vec!["cze:tolls".into()])
        .err()
        .expect("the default is OSRM, and OSRM cannot avoid per country");
    assert!(err.starts_with(AVOID_NEEDS_SYGIC), "got: {err}");
}

#[tokio::test]
async fn mock_table_is_straight_line_times_1_3() {
    let p = build_provider(ProviderConfig::Mock, None, vec![]).unwrap();
    let a = (48.1486, 17.1077);
    let b = (48.3774, 17.5872);
    let m = p.table(&[a, b]).await.unwrap();
    assert_eq!(m[0][0], 0.0);
    let want = crate::route_map::areas::haversine_km(a, b) * 1.3;
    assert!((m[0][1] - want).abs() < 1e-9);
    assert!((m[1][0] - want).abs() < 1e-9);
}
