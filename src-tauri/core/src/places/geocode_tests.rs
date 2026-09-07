//! Tests for the geocoder.
//!
//! Every test runs against a `wiremock` server — nothing here ever touches
//! Nominatim or any other network host.
//!
//! All of them take `settings::test_env::lock()`. `mock_mode_short_circuits_the_network`
//! sets a process-global env var, and the HTTP tests only behave as written
//! while it is unset, so the two groups must never overlap.

use super::geocode::{GeocodeProvider, HttpGeocodeProvider};
use crate::constants::env_vars::MOCK_GEOCODER_DIR;
use wiremock::matchers::{method, path as wm_path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A real `format=jsonv2` response body, trimmed to the fields we read.
fn nominatim_body() -> serde_json::Value {
    serde_json::json!([
        {
            "place_id": 123456,
            "lat": "48.1485965",
            "lon": "17.1077477",
            "name": "Hlavná stanica",
            "display_name": "Hlavná stanica, Predstaničné námestie, Bratislava, Slovensko",
            "type": "station"
        },
        {
            "place_id": 654321,
            "lat": "48.1516988",
            "lon": "17.1093159",
            "name": "Hlavná stanica",
            "display_name": "Hlavná stanica, Šancová, Bratislava, Slovensko",
            "type": "bus_stop"
        }
    ])
}

async fn mock_search(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(wm_path("/search"))
        .respond_with(response)
        .mount(server)
        .await;
}

#[tokio::test]
async fn parses_candidates_from_a_nominatim_response() {
    let _env = crate::settings::test_env::lock();
    let server = MockServer::start().await;
    mock_search(
        &server,
        ResponseTemplate::new(200).set_body_json(nominatim_body()),
    )
    .await;

    let client = HttpGeocodeProvider::new(server.uri());
    let candidates = client.search("Hlavná stanica, Bratislava").await.unwrap();

    assert_eq!(candidates.len(), 2);
    // lat/lon arrive as JSON strings; the point of these assertions is that
    // they came back as numbers we can store.
    assert!(
        (candidates[0].lat - 48.1485965).abs() < 1e-7,
        "{:?}",
        candidates[0]
    );
    assert!(
        (candidates[0].lon - 17.1077477).abs() < 1e-7,
        "{:?}",
        candidates[0]
    );
    assert_eq!(
        candidates[0].label, "Hlavná stanica, Predstaničné námestie, Bratislava, Slovensko",
        "the label is the geocoder's full display_name — it is what tells two \
         identically named matches apart while choosing"
    );
    // Nominatim returns best match first; the order carries meaning and must survive.
    assert!((candidates[1].lat - 48.1516988).abs() < 1e-7);
}

/// Nine of the book's 47 places are bare names with no city. They geocode to
/// nothing, and that is the normal path to placing a pin by hand — not a
/// failure the UI should present as one.
#[tokio::test]
async fn an_empty_result_is_not_an_error() {
    let _env = crate::settings::test_env::lock();
    let server = MockServer::start().await;
    mock_search(
        &server,
        ResponseTemplate::new(200).set_body_json(serde_json::json!([])),
    )
    .await;

    let client = HttpGeocodeProvider::new(server.uri());
    let candidates = client
        .search("kancelária")
        .await
        .expect("no matches is an answer, not an error");
    assert!(candidates.is_empty());
}

#[tokio::test]
async fn an_http_error_is_an_error() {
    let _env = crate::settings::test_env::lock();
    let server = MockServer::start().await;
    mock_search(&server, ResponseTemplate::new(503)).await;

    let client = HttpGeocodeProvider::new(server.uri());
    let err = client
        .search("Bratislava")
        .await
        .expect_err("a failed request must not be reported as zero matches");
    assert!(
        err.contains("503"),
        "the error should name the HTTP status so the UI can offer Retry, got: {err}"
    );
}

#[tokio::test]
async fn malformed_json_is_an_error_not_a_panic() {
    let _env = crate::settings::test_env::lock();
    let server = MockServer::start().await;
    mock_search(
        &server,
        ResponseTemplate::new(200).set_body_string("<html>Gateway timeout</html>"),
    )
    .await;

    let client = HttpGeocodeProvider::new(server.uri());
    let err = client
        .search("Bratislava")
        .await
        .expect_err("an unparseable body must be an error");
    assert!(!err.is_empty());
}

/// A candidate is one of several independent alternatives, so a single
/// unreadable coordinate costs that one row, not the whole answer. Should every
/// row be unreadable the caller gets an empty vec, which already means "place
/// it by hand" — no response can leave the user with no way forward.
#[tokio::test]
async fn a_candidate_with_an_unparseable_coordinate_is_skipped() {
    let _env = crate::settings::test_env::lock();
    let server = MockServer::start().await;
    mock_search(
        &server,
        ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "lat": "not-a-number", "lon": "17.1", "display_name": "Broken" },
            { "lon": "17.1", "display_name": "Missing lat" },
            { "lat": "48.1485965", "lon": "17.1077477", "display_name": "Good" }
        ])),
    )
    .await;

    let client = HttpGeocodeProvider::new(server.uri());
    let candidates = client.search("Bratislava").await.unwrap();
    assert_eq!(candidates.len(), 1, "got: {candidates:?}");
    assert_eq!(candidates[0].label, "Good");
}

/// ADR-035: no country filter. Five of the book's 47 places are Czech or
/// Hungarian, and one of those alone carries 39 trips — `countrycodes=sk` would
/// return nothing for every one of them.
#[tokio::test]
async fn the_request_carries_no_country_filter() {
    let _env = crate::settings::test_env::lock();
    let server = MockServer::start().await;
    mock_search(
        &server,
        ResponseTemplate::new(200).set_body_json(serde_json::json!([])),
    )
    .await;

    let client = HttpGeocodeProvider::new(server.uri());
    client
        .search("Brno, Česko")
        .await
        .expect("request should succeed");

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests by default");
    assert_eq!(requests.len(), 1, "exactly one call per search");
    // Assert on what the server RECEIVED, not on a URL built here — otherwise
    // this only tests our own string formatting.
    assert_eq!(requests[0].url.path(), "/search");

    let query = requests[0].url.query().unwrap_or_default();
    assert!(
        !query.contains("countrycodes"),
        "ADR-035: the geocoder must not be pinned to one country, query was: {query}"
    );
    assert!(query.contains("format=jsonv2"), "query was: {query}");
    assert!(query.contains("limit=5"), "query was: {query}");
    assert!(query.contains("accept-language=sk"), "query was: {query}");

    // Nominatim's usage policy blocks the whole application over a generic or
    // absent agent, so the header is as load-bearing as the query.
    let agent = requests[0]
        .headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(agent.starts_with("kniha-jazd/"), "user-agent was: {agent}");
}

/// Holds the mock directory env var for the life of the value, serialised
/// against every other test that touches the real process environment.
///
/// `test_env::with_env_vars` takes a synchronous closure and so cannot wrap an
/// `.await`; this keeps the same lock and the same remove-on-unwind guarantee.
struct MockDir {
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl MockDir {
    fn set(dir: &std::path::Path) -> Self {
        let guard = crate::settings::test_env::lock();
        std::env::set_var(MOCK_GEOCODER_DIR, dir);
        Self { _guard: guard }
    }
}

impl Drop for MockDir {
    fn drop(&mut self) {
        std::env::remove_var(MOCK_GEOCODER_DIR);
    }
}

#[tokio::test]
async fn mock_mode_short_circuits_the_network() {
    let server = MockServer::start().await;
    mock_search(
        &server,
        ResponseTemplate::new(200).set_body_json(nominatim_body()),
    )
    .await;

    let dir = tempfile::tempdir().unwrap();
    // The filename is the normalised query — spaces and commas included, which
    // is what task 12's fixtures will be named.
    std::fs::write(
        dir.path().join("hlavna stanica, bratislava.json"),
        serde_json::to_string(&nominatim_body()).unwrap(),
    )
    .unwrap();

    let _mock = MockDir::set(dir.path());
    let client = HttpGeocodeProvider::new(server.uri());

    // Deliberately spelled differently from the filename: mock lookup goes
    // through `normalise`, the same key the rest of the place book uses.
    let candidates = client.search("Hlavná Stanica,  Bratislava").await.unwrap();
    assert_eq!(candidates.len(), 2);
    assert!((candidates[0].lat - 48.1485965).abs() < 1e-7);

    // A query with no fixture is not an error: an empty list already means
    // "place it by hand", which is the right answer for an unmocked place.
    let none = client
        .search("nowhere at all")
        .await
        .expect("a missing fixture is an empty answer, not an error");
    assert!(none.is_empty());

    let requests = server.received_requests().await.unwrap_or_default();
    assert!(
        requests.is_empty(),
        "mock mode must issue no HTTP call at all, got {} request(s)",
        requests.len()
    );
}
