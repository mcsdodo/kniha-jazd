//! Tests for the Sygic routing provider. Every test runs against `wiremock`;
//! nothing here touches the real Sygic API.

use super::osrm::RouteProvider;
use super::sygic::SygicRouteProvider;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const BA: (f64, f64) = (48.1486, 17.1077);
const BRNO: (f64, f64) = (49.1951, 16.6068);

fn ok_body(routes: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "status": "OK", "routes": routes })
}

fn one_route() -> serde_json::Value {
    ok_body(serde_json::json!([{
        "route": "_p~iF~ps|U_ulLnnqC",
        "distance": { "value": 132900 },
        "duration": { "value": 6660 },
        "possible_avoids": ["svk:highways", "svk:tolls", "svk:country", "cze:highways", "cze:country"]
    }]))
}

fn provider(server: &MockServer, avoid: &[&str]) -> SygicRouteProvider {
    SygicRouteProvider::new(
        server.uri(),
        "test-key".into(),
        None,
        avoid.iter().map(|s| s.to_string()).collect(),
    )
}

fn only_request(requests: Vec<Request>) -> Request {
    assert_eq!(requests.len(), 1, "expected exactly one request");
    requests.into_iter().next().unwrap()
}

fn query(req: &Request, key: &str) -> Option<String> {
    req.url.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned())
}

#[tokio::test]
async fn maps_a_successful_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/v3/api/directions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .mount(&server).await;

    let r = provider(&server, &["cze:tolls"]).fetch(&[BA, BRNO]).await.unwrap();
    assert_eq!(r.polyline, "_p~iF~ps|U_ulLnnqC");
    assert!((r.road_km - 132.9).abs() < 1e-9);
    assert!((r.duration_s - 6660.0).abs() < 1e-9);
    // `*:tolls` only, and the requested `cze:tolls` stays although Sygic no
    // longer offers it (Review Focus 3).
    assert_eq!(r.possible_avoids, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}

/// Sygic takes `lat,lon`. OSRM takes `lon,lat`. A swap puts the route in the
/// wrong country with no error, so pin the exact strings.
#[tokio::test]
async fn sends_lat_lon_with_vias_between_origin_and_destination() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .mount(&server).await;

    let via = (48.7, 17.0);
    provider(&server, &[]).fetch(&[BA, via, BRNO]).await.unwrap();

    let req = only_request(server.received_requests().await.unwrap());
    assert_eq!(query(&req, "origin").as_deref(), Some("48.148600,17.107700"));
    assert_eq!(query(&req, "destination").as_deref(), Some("49.195100,16.606800"));
    assert_eq!(query(&req, "waypoints").as_deref(), Some("48.700000,17.000000"));
    assert_eq!(query(&req, "vehicle_type").as_deref(), Some("car"));
    assert_eq!(query(&req, "return_possible_avoids").as_deref(), Some("true"));
    assert_eq!(query(&req, "key").as_deref(), Some("test-key"));
    assert_eq!(query(&req, "avoid"), None, "no avoid list, no avoid parameter");
    assert_eq!(query(&req, "compute_alternatives"), None, "vias: no alternatives");
}

#[tokio::test]
async fn joins_the_avoid_list_with_a_pipe() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(query_param("avoid", "cze:tolls|svk:tolls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .expect(1)
        .mount(&server).await;

    provider(&server, &["cze:tolls", "svk:tolls"]).fetch(&[BA, BRNO]).await.unwrap();
}

#[tokio::test]
async fn sends_the_referer_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(header("referer", "https://example.test/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(one_route()))
        .expect(1)
        .mount(&server).await;

    SygicRouteProvider::new(server.uri(), "k".into(), Some("https://example.test/".into()), vec![])
        .fetch(&[BA, BRNO]).await.unwrap();
}

#[tokio::test]
async fn asks_for_alternatives_on_a_two_point_route_and_keeps_their_order() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(query_param("compute_alternatives", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(serde_json::json!([
            { "route": "a", "distance": { "value": 132900 }, "duration": { "value": 6660 }, "possible_avoids": [] },
            { "route": "b", "distance": { "value": 137700 }, "duration": { "value": 7080 }, "possible_avoids": [] },
            { "route": "c", "distance": { "value": 150000 }, "duration": { "value": 7380 }, "possible_avoids": [] },
            { "route": "d", "distance": { "value": 160000 }, "duration": { "value": 9000 }, "possible_avoids": [] }
        ]))))
        .expect(1)
        .mount(&server).await;

    let routes = provider(&server, &[]).fetch_alternatives(&[BA, BRNO], 3).await.unwrap();
    let names: Vec<&str> = routes.iter().map(|r| r.polyline.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"], "service order, cut to max, never re-sorted");
}

#[tokio::test]
async fn names_the_status_and_body_of_a_403() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Allowed referers do not match."))
        .mount(&server).await;

    let err = provider(&server, &[]).fetch(&[BA, BRNO]).await.expect_err("403 is not a route");
    assert!(err.contains("Sygic") && err.contains("403") && err.contains("Allowed referers"), "got: {err}");
}

#[tokio::test]
async fn names_http_429_so_the_page_can_offer_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(429)).mount(&server).await;

    let err = provider(&server, &[]).fetch(&[BA, BRNO]).await.expect_err("429 is not a route");
    assert!(err.contains("429"), "got: {err}");
}

#[tokio::test]
async fn a_non_ok_status_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "status": "NO_ROUTE", "routes": [] })))
        .mount(&server).await;

    let err = provider(&server, &[]).fetch(&[BA, BRNO]).await.expect_err("no route");
    assert!(err.contains("NO_ROUTE"), "got: {err}");
}

#[tokio::test]
async fn an_ok_status_with_no_routes_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(ok_body(serde_json::json!([]))))
        .mount(&server).await;

    assert!(provider(&server, &[]).fetch(&[BA, BRNO]).await.is_err());
}

/// Review Focus 6: the key is in the query string, and reqwest errors print
/// the URL. No error may carry it.
#[tokio::test]
async fn a_transport_failure_does_not_leak_the_key() {
    // Port 9 (discard) on localhost: nothing listens, the connect fails.
    let p = SygicRouteProvider::new("http://127.0.0.1:9", "secret-test-key".into(), None, vec![]);
    let err = p.fetch(&[BA, BRNO]).await.expect_err("nothing listens");
    assert!(!err.contains("secret-test-key"), "key leaked: {err}");
}

#[tokio::test]
async fn a_malformed_body_does_not_leak_the_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server).await;

    let p = SygicRouteProvider::new(server.uri(), "secret-test-key".into(), None, vec![]);
    let err = p.fetch(&[BA, BRNO]).await.expect_err("bad body");
    assert!(!err.contains("secret-test-key"), "key leaked: {err}");
}

#[tokio::test]
async fn one_point_is_an_error_not_a_request() {
    let server = MockServer::start().await;
    let err = provider(&server, &[]).fetch(&[BA]).await.expect_err("one point");
    assert!(err.contains("at least 2 points"), "got: {err}");
    assert!(server.received_requests().await.unwrap().is_empty());
}
