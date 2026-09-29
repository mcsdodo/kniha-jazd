//! Sygic route geometry provider (Task 85).
//!
//! Used in place of OSRM when `SYGIC_API_KEY` is set, because Sygic can avoid
//! the paid roads of ONE country (`avoid=cze:tolls`). The public OSRM servers
//! reject every `exclude` value and know no countries -- see
//! _tasks/85-route-avoid-tolls-per-country/01-task.md.
//!
//! The avoid list is fixed when the provider is built, so the `RouteProvider`
//! trait and its callers do not change.

use serde::Deserialize;
use std::time::Duration;

use super::avoid::toll_options;
use super::osrm::{FetchedRoute, RouteProvider};

pub const PUBLIC_SYGIC_URL: &str = "https://routing.api.sygic.com";

const USER_AGENT: &str = concat!(
    "kniha-jazd/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mcsdodo/kniha-jazd)"
);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Enough of an error body to name the cause ("Allowed referers do not match.").
const MAX_ERROR_BODY: usize = 200;

pub struct SygicRouteProvider {
    base_url: String,
    api_key: String,
    referer: Option<String>,
    /// Already normalised by `avoid::normalise_avoid`.
    avoid: Vec<String>,
    client: Result<reqwest::Client, String>,
}

impl SygicRouteProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: String,
        referer: Option<String>,
        avoid: Vec<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| format!("Could not create an HTTP client for the routing service: {e}"));
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            referer,
            avoid,
            client,
        }
    }

    /// Query parameters for `coords` (`(lat, lon)` pairs in visit order).
    /// Sygic wants `lat,lon` -- the OPPOSITE of OSRM.
    fn params(&self, coords: &[(f64, f64)], alternatives: bool) -> Vec<(&'static str, String)> {
        let point = |(lat, lon): (f64, f64)| format!("{lat:.6},{lon:.6}");
        let mut p = vec![
            ("origin", point(coords[0])),
            ("destination", point(coords[coords.len() - 1])),
            ("vehicle_type", "car".to_string()),
            ("return_possible_avoids", "true".to_string()),
            ("key", self.api_key.clone()),
        ];
        if coords.len() > 2 {
            let vias = coords[1..coords.len() - 1].iter().map(|&c| point(c)).collect::<Vec<_>>().join("|");
            p.push(("waypoints", vias));
        }
        if !self.avoid.is_empty() {
            p.push(("avoid", self.avoid.join("|")));
        }
        // Same rule as OSRM (DECISIONS.md, alternatives ADR): only a plain
        // two-point request offers a choice.
        if alternatives && coords.len() == 2 {
            p.push(("compute_alternatives", "true".to_string()));
        }
        p
    }

    /// Replace the key with `***`. The key is in the query string, and a
    /// Sygic error body could echo it (Review Focus 6).
    fn redact(&self, msg: String) -> String {
        if self.api_key.is_empty() {
            msg
        } else {
            msg.replace(&self.api_key, "***")
        }
    }

    async fn request(&self, coords: &[(f64, f64)], alternatives: bool) -> Result<Vec<FetchedRoute>, String> {
        self.request_unredacted(coords, alternatives).await.map_err(|e| self.redact(e))
    }

    async fn request_unredacted(&self, coords: &[(f64, f64)], alternatives: bool) -> Result<Vec<FetchedRoute>, String> {
        if coords.len() < 2 {
            return Err(format!(
                "Route needs at least 2 points, got {}. Nothing was requested from Sygic.",
                coords.len()
            ));
        }
        let client = self.client.as_ref().map_err(|e| e.clone())?;
        let mut req = client
            .get(format!("{}/v3/api/directions", self.base_url))
            .query(&self.params(coords, alternatives));
        if let Some(referer) = &self.referer {
            req = req.header(reqwest::header::REFERER, referer);
        }

        // `without_url`: reqwest prints the URL in its errors, and the URL
        // carries the key (Review Focus 6).
        let response = req.send().await.map_err(|e| {
            format!(
                "Could not reach the Sygic routing service: {}. Check your internet connection and try again.",
                e.without_url()
            )
        })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let body: String = body.trim().chars().take(MAX_ERROR_BODY).collect();
            return Err(format!(
                "Sygic routing service returned HTTP {} ({}){}. Try again in a moment.",
                status.as_u16(),
                status.canonical_reason().unwrap_or("unknown"),
                if body.is_empty() { String::new() } else { format!(": {body}") }
            ));
        }

        // `without_url`: a decode error prints the URL, and the URL carries the key.
        let body: SygicResponse = response
            .json()
            .await
            .map_err(|e| format!("Could not read the Sygic routing service response: {}", e.without_url()))?;
        if body.status != "OK" {
            return Err(format!("Sygic routing service could not build a route: {}", body.status));
        }
        if body.routes.is_empty() {
            return Err("Sygic routing service reported success but returned no route.".to_string());
        }

        Ok(body
            .routes
            .into_iter()
            .map(|r| FetchedRoute {
                possible_avoids: toll_options(r.possible_avoids.iter().map(String::as_str), &self.avoid),
                polyline: r.route,
                road_km: r.distance.value / 1000.0,
                duration_s: r.duration.value,
            })
            .collect())
    }
}

#[derive(Deserialize)]
struct SygicResponse {
    status: String,
    #[serde(default)]
    routes: Vec<SygicRoute>,
}

#[derive(Deserialize)]
struct SygicRoute {
    /// Google encoded polyline, precision 1e5 -- the same polyline5 we store.
    route: String,
    distance: SygicValue,
    duration: SygicValue,
    #[serde(default)]
    possible_avoids: Vec<String>,
}

#[derive(Deserialize)]
struct SygicValue {
    value: f64,
}

#[async_trait::async_trait]
impl RouteProvider for SygicRouteProvider {
    fn kind(&self) -> crate::route_map::provider::RouteProviderKind {
        crate::route_map::provider::RouteProviderKind::Sygic
    }

    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        let mut routes = self.request(coords, false).await?;
        Ok(routes.remove(0))
    }

    async fn fetch_alternatives(&self, coords: &[(f64, f64)], max: usize) -> Result<Vec<FetchedRoute>, String> {
        let mut routes = self.request(coords, max > 1).await?;
        routes.truncate(max.max(1));
        Ok(routes)
    }
}
