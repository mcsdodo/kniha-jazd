//! Which routing service computes a route map (Task 85).
//!
//! One place decides, so the three async commands cannot disagree. The avoid
//! list is fixed per provider instance, because the dispatcher builds a new
//! provider for every request anyway.

use crate::constants::env_vars::{MOCK_ROUTER, SYGIC_API_KEY, SYGIC_REFERER};

use super::avoid::toll_options;
use super::osrm::{FetchedRoute, HttpRouteProvider, RouteProvider};
use super::polyline::encode;
use super::sygic::{SygicRouteProvider, PUBLIC_SYGIC_URL};

#[derive(Debug)]
pub enum ProviderConfig {
    Mock,
    Sygic { api_key: String, referer: Option<String> },
    Osrm,
}

impl ProviderConfig {
    /// Pure: `lookup` stands in for `std::env::var`. Blank values count as unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let get = |k: &str| lookup(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        if get(MOCK_ROUTER).is_some() {
            return Self::Mock;
        }
        match get(SYGIC_API_KEY) {
            Some(api_key) => Self::Sygic { api_key, referer: get(SYGIC_REFERER) },
            None => Self::Osrm,
        }
    }

    pub fn from_env() -> Self {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
}

/// `avoid` must already be normalised (`avoid::normalise_avoid`).
pub fn build_provider(config: ProviderConfig, avoid: Vec<String>) -> Result<Box<dyn RouteProvider>, String> {
    match config {
        ProviderConfig::Mock => Ok(Box::new(MockRouteProvider { avoid })),
        ProviderConfig::Sygic { api_key, referer } => {
            Ok(Box::new(SygicRouteProvider::new(PUBLIC_SYGIC_URL, api_key, referer, avoid)))
        }
        ProviderConfig::Osrm if avoid.is_empty() => Ok(Box::new(HttpRouteProvider::public())),
        // No silent fallback (01-task.md, decision 5): OSRM would return a
        // route that ignores the avoid list and looks correct.
        ProviderConfig::Osrm => Err(format!(
            "Avoiding paid roads ({}) needs the Sygic routing service. Set SYGIC_API_KEY on the server, or uncheck the avoid options.",
            avoid.join(", ")
        )),
    }
}

pub fn route_provider(avoid: Vec<String>) -> Result<Box<dyn RouteProvider>, String> {
    build_provider(ProviderConfig::from_env(), avoid)
}

/// Offline stand-in for the integration suite (`KNIHA_JAZD_MOCK_ROUTER`).
/// Fixed numbers, so a spec can see that a checkbox click routed again.
pub struct MockRouteProvider {
    avoid: Vec<String>,
}

#[async_trait::async_trait]
impl RouteProvider for MockRouteProvider {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        if coords.len() < 2 {
            return Err(format!("Route needs at least 2 points, got {}.", coords.len()));
        }
        Ok(FetchedRoute {
            polyline: encode(coords),
            road_km: if self.avoid.is_empty() { 100.0 } else { 120.0 },
            duration_s: 3600.0,
            possible_avoids: toll_options(["cze:tolls"], &self.avoid),
        })
    }
}
