//! Tests for the OSRM route geometry provider.
//!
//! Every test runs against a `wiremock` server — nothing here ever touches the
//! public OSRM instance or any other network host.

use super::osrm::{HttpRouteProvider, RouteProvider};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn parses_a_successful_route_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{ "geometry": "_p~iF~ps|U_ulLnnqC", "distance": 118432.0 }]
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    let r = client
        .fetch(&[(48.935, 20.553), (48.997, 20.591)])
        .await
        .unwrap();
    assert_eq!(r.polyline, "_p~iF~ps|U_ulLnnqC");
    assert!((r.road_km - 118.432).abs() < 1e-3);
}

#[tokio::test]
async fn surfaces_a_non_ok_code_as_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "NoRoute",
            "routes": []
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    let err = client
        .fetch(&[(48.935, 20.553), (48.997, 20.591)])
        .await
        .expect_err("a non-Ok OSRM code must not be reported as a route");
    assert!(
        err.contains("NoRoute"),
        "error should name the OSRM code, got: {err}"
    );
}

#[tokio::test]
async fn surfaces_http_429_as_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    let err = client
        .fetch(&[(48.935, 20.553), (48.997, 20.591)])
        .await
        .expect_err("a rate-limit response must not be reported as a route");
    assert!(
        err.contains("429"),
        "error should name the HTTP status so the UI can offer Retry, got: {err}"
    );
}

/// Latitude/longitude transposition is the classic OSRM bug: the Rust API takes
/// `(lat, lon)` but the URL wants `lon,lat`. Transposed, a Slovak route would
/// silently be plotted somewhere off the coast of Somalia. Pin the exact path.
#[tokio::test]
async fn sends_coordinates_as_lon_lat_in_order() {
    let server = MockServer::start().await;
    // Deliberately permissive: this test asserts on the request we RECORDED,
    // not on whether a strict path matcher happened to fire. Inferring the
    // coordinate order from a 404-vs-200 makes every unrelated hiccup report
    // itself as "wrong coordinate order", which is exactly the wrong message.
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{ "geometry": "abc", "distance": 1000.0 }]
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    // Deliberately asymmetric: three points, all with lat > lon, so neither a
    // transposition nor a reordering could still produce the expected path.
    client
        .fetch(&[(48.935, 20.553), (48.997, 20.591), (48.716, 21.250)])
        .await
        .expect("request should succeed");

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests by default");
    assert_eq!(requests.len(), 1, "exactly one OSRM call per fetch");
    assert_eq!(
        requests[0].url.path(),
        "/route/v1/driving/20.553000,48.935000;20.591000,48.997000;21.250000,48.716000",
        "coordinates must be serialised lon,lat in visit order"
    );

    // Every mock here matches on path alone, so without this the query could
    // drift to geometries=geojson and all tests would still pass while the
    // `polyline` field silently filled with unparseable garbage — a failure
    // that would only surface as a blank map at export time.
    let query = requests[0].url.query().unwrap_or_default();
    assert!(query.contains("geometries=polyline"), "query was: {query}");
    assert!(query.contains("overview=full"), "query was: {query}");
    assert!(query.contains("steps=false"), "query was: {query}");
}

#[tokio::test]
async fn an_empty_coordinate_list_is_an_error_not_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/route/v1/driving/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "Ok",
            "routes": [{ "geometry": "abc", "distance": 1000.0 }]
        })))
        .mount(&server)
        .await;

    let client = HttpRouteProvider::new(server.uri());
    let err = client
        .fetch(&[])
        .await
        .expect_err("an empty coordinate list must be an error");
    assert!(!err.is_empty());

    let requests = server.received_requests().await.unwrap_or_default();
    assert!(
        requests.is_empty(),
        "no HTTP call may be issued for an empty coordinate list"
    );

    // OSRM cannot route a single point, so the guard rejects one coordinate
    // too. That half of the guard is novel behaviour and needs its own pin.
    client
        .fetch(&[(48.935, 20.553)])
        .await
        .expect_err("a single coordinate must be an error");
    let requests = server.received_requests().await.unwrap_or_default();
    assert!(requests.is_empty(), "still no HTTP call for one coordinate");
}

#[tokio::test]
async fn a_transport_failure_is_reported_as_such() {
    // Port 1 is never bound: this exercises the transport branch, which was
    // the one error kind of the three with no coverage.
    let client = HttpRouteProvider::new("http://127.0.0.1:1");
    let err = client
        .fetch(&[(48.935, 20.553), (48.997, 20.591)])
        .await
        .expect_err("an unreachable host must be an error");
    assert!(
        err.contains("127.0.0.1:1"),
        "the transport error should name the host it could not reach: {err}"
    );
}
