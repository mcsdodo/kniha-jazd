//! Which routing service computes a route map (Task 85).
//!
//! One place decides, so the three async commands cannot disagree. The avoid
//! list is fixed per provider instance, because the dispatcher builds a new
//! provider for every request anyway.
//!
//! Task 86: the page can ask for a provider per request. The server config
//! still decides what EXISTS (Sygic only with a key); the request only picks
//! among what exists. The default is OSRM, also with a key: Sygic's map lacks
//! the D1 Visnove tunnel, so Sygic is chosen explicitly (for example to avoid
//! paid roads).

use crate::constants::env_vars::{MOCK_ROUTER, SYGIC_API_KEY, SYGIC_REFERER};

use super::avoid::toll_options;
use super::osrm::{FetchedRoute, HttpRouteProvider, RouteProvider};
use super::polyline::encode;
use super::sygic::{SygicRouteProvider, PUBLIC_SYGIC_URL};

/// Which routing service computed a route. Stored on a saved route
/// (`trip_routes.provider`) and sent by the page to pick one per request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RouteProviderKind {
    Osrm,
    Sygic,
}

impl RouteProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Osrm => "osrm",
            Self::Sygic => "sygic",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "osrm" => Some(Self::Osrm),
            "sygic" => Some(Self::Sygic),
            _ => None,
        }
    }
}

/// What the page may offer, and what it starts on.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteProvidersInfo {
    pub available: Vec<RouteProviderKind>,
    pub default: RouteProviderKind,
}

/// Stable marker at the start of the "Sygic was asked for, but there is no
/// key" error. The page matches it (`PROVIDER_NEEDS_SYGIC` in
/// src/routes/mapa/+page.svelte).
pub const PROVIDER_NEEDS_SYGIC: &str = "PROVIDER_NEEDS_SYGIC";

/// Stable marker at the start of the "avoid needs Sygic" error. The page
/// matches it (`AVOID_NEEDS_SYGIC` in src/routes/mapa/+page.svelte).
pub const AVOID_NEEDS_SYGIC: &str = "AVOID_NEEDS_SYGIC";

pub enum ProviderConfig {
    Mock,
    Sygic { api_key: String, referer: Option<String> },
    Osrm,
}

/// Manual `Debug`: the API key must never reach a log line.
impl std::fmt::Debug for ProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mock => write!(f, "Mock"),
            Self::Sygic { referer, .. } => {
                f.debug_struct("Sygic").field("api_key", &"***").field("referer", referer).finish()
            }
            Self::Osrm => write!(f, "Osrm"),
        }
    }
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

    /// The mock stands in for a keyed server, so the integration suite can
    /// see and use the selector. The default is OSRM in every config.
    pub fn info(&self) -> RouteProvidersInfo {
        let available = match self {
            Self::Osrm => vec![RouteProviderKind::Osrm],
            Self::Sygic { .. } | Self::Mock => {
                vec![RouteProviderKind::Osrm, RouteProviderKind::Sygic]
            }
        };
        RouteProvidersInfo { available, default: RouteProviderKind::Osrm }
    }
}

/// `avoid` must already be normalised (`avoid::normalise_avoid`).
/// `requested == None` means the server default ([`ProviderConfig::info`]).
pub fn build_provider(
    config: ProviderConfig,
    requested: Option<RouteProviderKind>,
    avoid: Vec<String>,
) -> Result<Box<dyn RouteProvider>, String> {
    let info = config.info();
    let kind = requested.unwrap_or(info.default);
    if !info.available.contains(&kind) {
        return Err(format!(
            "{PROVIDER_NEEDS_SYGIC}: The Sygic routing service is not configured. Set SYGIC_API_KEY on the server, or select OSRM."
        ));
    }
    // No silent fallback (Task 85, decision 5): OSRM would return a route
    // that ignores the avoid list and looks correct.
    if kind == RouteProviderKind::Osrm && !avoid.is_empty() {
        return Err(format!(
            "{AVOID_NEEDS_SYGIC}: Avoiding paid roads ({}) needs the Sygic routing service. Set SYGIC_API_KEY on the server, or uncheck the avoid options.",
            avoid.join(", ")
        ));
    }
    match (config, kind) {
        (ProviderConfig::Mock, kind) => Ok(Box::new(MockRouteProvider { avoid, kind })),
        (ProviderConfig::Sygic { api_key, referer }, RouteProviderKind::Sygic) => {
            Ok(Box::new(SygicRouteProvider::new(PUBLIC_SYGIC_URL, api_key, referer, avoid)))
        }
        (_, _) => Ok(Box::new(HttpRouteProvider::public())),
    }
}

pub fn route_provider(
    requested: Option<RouteProviderKind>,
    avoid: Vec<String>,
) -> Result<Box<dyn RouteProvider>, String> {
    build_provider(ProviderConfig::from_env(), requested, avoid)
}

/// Offline stand-in for the integration suite (`KNIHA_JAZD_MOCK_ROUTER`).
/// Fixed numbers, so a spec can see that a checkbox click routed again, and
/// that a provider switch did: "OSRM" is 90 km and offers no avoid options.
pub struct MockRouteProvider {
    avoid: Vec<String>,
    kind: RouteProviderKind,
}

#[async_trait::async_trait]
impl RouteProvider for MockRouteProvider {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        if coords.len() < 2 {
            return Err(format!("Route needs at least 2 points, got {}.", coords.len()));
        }
        if self.kind == RouteProviderKind::Osrm {
            return Ok(FetchedRoute {
                polyline: encode(coords),
                road_km: 90.0,
                duration_s: 3600.0,
                possible_avoids: vec![],
            });
        }
        Ok(FetchedRoute {
            polyline: encode(coords),
            road_km: if self.avoid.is_empty() { 100.0 } else { 120.0 },
            duration_s: 3600.0,
            possible_avoids: toll_options(["cze:tolls"], &self.avoid),
        })
    }

    fn kind(&self) -> RouteProviderKind {
        self.kind
    }
}
