//! OSRM route geometry provider.
//!
//! Called once per generated map, after the genetic algorithm has decided which
//! settlements to visit, purely to turn that ordered list of points into a
//! road-following geometry we can draw.
//!
//! Everything sits behind [`RouteProvider`] so tests can stand in a fake and
//! never touch the network.
//!
//! Deliberately out of scope here: retries, caching, and polyline decoding. The
//! encoded polyline5 string is handed back exactly as OSRM returned it.

use serde::Deserialize;
use std::time::Duration;

/// Public OSRM demo server. Rate-limited and best-effort — fine for occasional
/// map generation, not for bulk use.
const PUBLIC_OSRM_URL: &str = "https://router.project-osrm.org";

/// How long to wait for the whole request. Routes over many waypoints can take
/// the public server a few seconds.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// A single road-following route as returned by OSRM.
#[derive(Debug, Clone)]
pub struct FetchedRoute {
    /// Encoded polyline5, as returned by OSRM.
    pub polyline: String,
    /// Total road distance in kilometres (OSRM reports metres).
    pub road_km: f64,
}

#[async_trait::async_trait]
pub trait RouteProvider: Send + Sync {
    /// `coords` are `(lat, lon)` pairs in visit order.
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String>;
}

pub struct HttpRouteProvider {
    base_url: String,
    /// Built once in `new`. A build failure (no usable TLS backend) is kept as
    /// an error string instead of panicking, so it can reach the UI like any
    /// other fetch failure.
    client: Result<reqwest::Client, String>,
}

impl HttpRouteProvider {
    /// `base_url` without a trailing slash, e.g. "https://router.project-osrm.org".
    /// A trailing slash is tolerated and stripped.
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("Could not create an HTTP client for the routing service: {e}"));

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Public OSRM demo server.
    pub fn public() -> Self {
        Self::new(PUBLIC_OSRM_URL)
    }

    /// Build the request URL for `coords`.
    ///
    /// Note the flip: our API takes `(lat, lon)` — the order humans and our
    /// dataset use — but OSRM wants `lon,lat`. Transposing these silently
    /// produces a route in the wrong part of the world rather than an error, so
    /// `sends_coordinates_as_lon_lat_in_order` pins it.
    fn route_url(&self, coords: &[(f64, f64)]) -> String {
        let points = coords
            .iter()
            .map(|(lat, lon)| format!("{lon:.6},{lat:.6}"))
            .collect::<Vec<_>>()
            .join(";");

        format!(
            "{}/route/v1/driving/{}?geometries=polyline&overview=full&steps=false",
            self.base_url, points
        )
    }
}

/// Top level of an OSRM `/route` response. Only the fields we use are modelled.
#[derive(Deserialize)]
struct OsrmResponse {
    code: String,
    /// Present on errors; OSRM omits it on success.
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    routes: Vec<OsrmRoute>,
}

#[derive(Deserialize)]
struct OsrmRoute {
    /// Encoded polyline5 (we request `geometries=polyline`).
    geometry: String,
    /// Metres.
    distance: f64,
}

#[async_trait::async_trait]
impl RouteProvider for HttpRouteProvider {
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String> {
        // Guard before building a request: OSRM needs a start and an end.
        if coords.len() < 2 {
            return Err(format!(
                "Route needs at least 2 points, got {}. Nothing was requested from OSRM.",
                coords.len()
            ));
        }

        let client = self.client.as_ref().map_err(|e| e.clone())?;
        let url = self.route_url(coords);

        let response = client.get(&url).send().await.map_err(|e| {
            format!(
                "Could not reach the routing service at {}: {e}. Check your internet connection and try again.",
                self.base_url
            )
        })?;

        let status = response.status();
        if !status.is_success() {
            // The public OSRM server rate-limits with 429; the UI turns this
            // into a Retry prompt, so the status code has to survive.
            return Err(format!(
                "Routing service returned HTTP {} ({}). Try again in a moment.",
                status.as_u16(),
                status.canonical_reason().unwrap_or("unknown")
            ));
        }

        let body: OsrmResponse = response
            .json()
            .await
            .map_err(|e| format!("Could not read the routing service response: {e}"))?;

        if body.code != "Ok" {
            let detail = body.message.map(|m| format!(" ({m})")).unwrap_or_default();
            return Err(format!(
                "Routing service could not build a route: {}{}",
                body.code, detail
            ));
        }

        const NO_ROUTE: &str = "Routing service reported success but returned no route.";
        let first = body.routes.into_iter().next();
        let route = first.ok_or_else(|| NO_ROUTE.to_string())?;

        Ok(FetchedRoute {
            polyline: route.geometry,
            road_km: route.distance / 1000.0,
        })
    }
}
