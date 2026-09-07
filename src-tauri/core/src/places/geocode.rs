//! Turning a typed address into candidate coordinates.
//!
//! Called once per place the user is placing, from the Miesta section of
//! Settings: the user types (or accepts) an address, picks one of the
//! candidates that come back, and that coordinate is what gets stored.
//!
//! Everything sits behind [`GeocodeProvider`] so tests can stand in a fake and
//! never touch the network.
//!
//! Deliberately out of scope here: retries, caching, and any country filter
//! (ADR-035 — five of the book's 47 places are Czech or Hungarian).

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::constants::env_vars::MOCK_GEOCODER_DIR;
use crate::places::normalise;

/// The public Nominatim instance.
const PUBLIC_NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org";

/// How long to wait for one search. A person is watching a dialog for the
/// answer, so a slow search is a stuck one — no reason to wait as long as for
/// a whole route.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// How many matches to offer. More than a handful is not a choice, it is a
/// list to read; the user who is not served by five drops a pin by hand.
const RESULT_LIMIT: u8 = 5;

/// Identifies this application to Nominatim. Its usage policy requires it, and
/// a generic or absent agent gets the whole application blocked rather than
/// just one request — the same rule OSM's tile policy already imposes in
/// `route_map::tiles`.
const USER_AGENT: &str = concat!(
    "kniha-jazd/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mcsdodo/kniha-jazd)"
);

/// One geocoder match.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub lat: f64,
    pub lon: f64,
    /// What the geocoder calls it. Shown while choosing, never stored — the
    /// book keeps the trip's own spelling (ADR-034).
    pub label: String,
}

#[async_trait::async_trait]
pub trait GeocodeProvider: Send + Sync {
    /// Best matches first. An empty vec is a valid answer, not an error.
    async fn search(&self, query: &str) -> Result<Vec<Candidate>, String>;
}

/// Nominatim over HTTP.
///
/// No throttling here, deliberately, even though Nominatim's usage policy caps
/// requests at one per second. Placing is one address at a time, driven from
/// the browser and persisted before the next begins, so the obligation is
/// already met by the only thing that can pace it. A limiter here would be
/// redundant machinery guarding a queue that never has two items in it.
pub struct HttpGeocodeProvider {
    base_url: String,
    /// Built once in `new`. A build failure (no usable TLS backend) is kept as
    /// an error string instead of panicking, so it can reach the UI like any
    /// other fetch failure.
    client: Result<reqwest::Client, String>,
}

impl HttpGeocodeProvider {
    /// `base_url` without a trailing slash, e.g. "https://nominatim.openstreetmap.org".
    /// A trailing slash is tolerated and stripped.
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| format!("Could not create an HTTP client for the geocoder: {e}"));

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        }
    }

    /// The public Nominatim instance.
    pub fn public() -> Self {
        Self::new(PUBLIC_NOMINATIM_URL)
    }
}

/// One entry of a Nominatim `format=jsonv2` response. Only the fields we use
/// are modelled.
///
/// Every field defaults: a row missing one is a row we skip, not a response we
/// reject (see [`to_candidates`]).
#[derive(Deserialize)]
struct NominatimPlace {
    /// Arrives as a string, e.g. "48.1485965".
    #[serde(default)]
    lat: String,
    #[serde(default)]
    lon: String,
    /// Full human-readable address, e.g. "Hlavná stanica, …, Bratislava, Slovensko".
    #[serde(default)]
    display_name: String,
    /// Short name. Used only when `display_name` is absent.
    #[serde(default)]
    name: String,
}

/// Keep the rows we can read, drop the rows we cannot.
///
/// The candidates are independent alternatives, so one unreadable coordinate
/// costs that row and not the user's other four options. If every row is
/// unreadable the caller gets an empty vec, which already means "place it by
/// hand" — no response leaves the user without a way forward.
fn to_candidates(places: Vec<NominatimPlace>) -> Vec<Candidate> {
    places
        .into_iter()
        .filter_map(|p| {
            let lat = p.lat.parse::<f64>().ok()?;
            let lon = p.lon.parse::<f64>().ok()?;
            let label = if p.display_name.is_empty() {
                p.name
            } else {
                p.display_name
            };
            Some(Candidate { lat, lon, label })
        })
        .collect()
}

/// Read canned candidates from `{mock_dir}/{normalised query}.json`.
///
/// A missing file is an empty answer, which the UI already treats as "place it
/// by hand". A file that is present but unreadable is an error: a broken
/// fixture must shout rather than quietly turn a test green.
fn load_mock_candidates(mock_dir: &str, query: &str) -> Result<Vec<Candidate>, String> {
    let file = std::path::Path::new(mock_dir).join(format!("{}.json", normalise(query)));
    if !file.exists() {
        log::warn!("No geocoder mock file for {query:?} at {file:?}, returning no candidates");
        return Ok(Vec::new());
    }

    let json = std::fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read geocoder mock file {file:?}: {e}"))?;
    let places: Vec<NominatimPlace> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse geocoder mock JSON {file:?}: {e}"))?;
    Ok(to_candidates(places))
}

#[async_trait::async_trait]
impl GeocodeProvider for HttpGeocodeProvider {
    async fn search(&self, query: &str) -> Result<Vec<Candidate>, String> {
        if let Ok(mock_dir) = std::env::var(MOCK_GEOCODER_DIR) {
            log::info!("Mock mode enabled: geocoding {query:?} from {mock_dir:?}");
            return load_mock_candidates(&mock_dir, query);
        }

        let client = self.client.as_ref().map_err(|e| e.clone())?;

        // Built with `query` rather than `format!` so the address — which
        // carries spaces, commas and diacritics — is percent-encoded properly.
        // Note what is NOT here: `countrycodes`. ADR-035.
        let response = client
            .get(format!("{}/search", self.base_url))
            .query(&[
                ("q", query),
                ("format", "jsonv2"),
                ("limit", &RESULT_LIMIT.to_string()),
                ("accept-language", "sk"),
            ])
            .send()
            .await
            .map_err(|e| {
                format!(
                    "Could not reach the geocoding service at {}: {e}. Check your internet connection and try again.",
                    self.base_url
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            // Nominatim rate-limits with 429; the UI turns this into a Retry
            // prompt, so the status code has to survive.
            return Err(format!(
                "Geocoding service returned HTTP {} ({}). Try again in a moment.",
                status.as_u16(),
                status.canonical_reason().unwrap_or("unknown")
            ));
        }

        let places: Vec<NominatimPlace> = response
            .json()
            .await
            .map_err(|e| format!("Could not read the geocoding service response: {e}"))?;

        Ok(to_candidates(places))
    }
}
