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

/// Identifies this application to the routing service. The public OSRM demo
/// server answers **403 Forbidden** when a request carries no User-Agent, and
/// reqwest sends none by default. Mirrors `tiles.rs`, which learned the same
/// lesson for the tile server.
const USER_AGENT: &str = concat!(
    "kniha-jazd/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mcsdodo/kniha-jazd)"
);

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
    /// Estimated driving time in seconds (OSRM reports seconds directly).
    /// Displayed while choosing between alternatives; never persisted -- the
    /// printed export renders no text, so a stored duration has no reader.
    pub duration_s: f64,
}

#[async_trait::async_trait]
pub trait RouteProvider: Send + Sync {
    /// `coords` are `(lat, lon)` pairs in visit order.
    async fn fetch(&self, coords: &[(f64, f64)]) -> Result<FetchedRoute, String>;

    /// Up to `max` routes for the same points, **in the order the service
    /// returned them** -- OSRM lists them fastest first, which is the order the
    /// UI shows and must never re-sort.
    ///
    /// Defaulted to a single `fetch` so stubs need no extra impl. Only the
    /// HTTP provider overrides it.
    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        let _ = max;
        Ok(vec![self.fetch(coords).await?])
    }
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
            .user_agent(USER_AGENT)
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
    fn route_url(&self, coords: &[(f64, f64)], alternatives: Option<usize>) -> String {
        let points = coords
            .iter()
            .map(|(lat, lon)| format!("{lon:.6},{lat:.6}"))
            .collect::<Vec<_>>()
            .join(";");

        let mut url = format!(
            "{}/route/v1/driving/{}?geometries=polyline&overview=full&steps=false",
            self.base_url, points
        );
        // Only meaningful for exactly two points -- with vias OSRM returns the
        // single through-route regardless.
        if let Some(n) = alternatives {
            if coords.len() == 2 && n > 1 {
                url.push_str(&format!("&alternatives={}", n - 1));
            }
        }
        url
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
    /// Seconds.
    #[serde(default)]
    duration: f64,
}

impl HttpRouteProvider {
    /// Issues the request and maps every `OsrmRoute` in the response to a
    /// [`FetchedRoute`], preserving OSRM's order. Shared by `fetch` and
    /// `fetch_alternatives` so every error branch (connection, non-2xx,
    /// non-`Ok` code, empty `routes`) is handled exactly once.
    async fn request(&self, url: &str) -> Result<Vec<FetchedRoute>, String> {
        let client = self.client.as_ref().map_err(|e| e.clone())?;

        let response = client.get(url).send().await.map_err(|e| {
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

        if body.routes.is_empty() {
            return Err("Routing service reported success but returned no route.".to_string());
        }

        Ok(body
            .routes
            .into_iter()
            .map(|route| FetchedRoute {
                polyline: route.geometry,
                road_km: route.distance / 1000.0,
                duration_s: route.duration,
            })
            .collect())
    }
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

        let mut routes = self.request(&self.route_url(coords, None)).await?;
        Ok(routes.remove(0))
    }

    async fn fetch_alternatives(
        &self,
        coords: &[(f64, f64)],
        max: usize,
    ) -> Result<Vec<FetchedRoute>, String> {
        // Same guard as `fetch`: OSRM needs a start and an end.
        if coords.len() < 2 {
            return Err(format!(
                "Route needs at least 2 points, got {}. Nothing was requested from OSRM.",
                coords.len()
            ));
        }

        self.request(&self.route_url(coords, Some(max))).await
    }
}
