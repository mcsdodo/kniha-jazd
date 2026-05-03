//! Tests for paperless module.
use super::*;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn resolve_tag_id_returns_existing_tag() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/tags/")).and(query_param("name__iexact", "fuel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1, "results": [{"id": 51, "name": "fuel"}]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let id = client.resolve_tag_id("fuel").await.unwrap();
    assert_eq!(id, 51);
}

#[tokio::test]
async fn resolve_tag_id_errors_when_tag_missing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/tags/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 0, "results": []
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let err = client.resolve_tag_id("nonexistent").await.unwrap_err();
    assert!(matches!(err, PaperlessError::TagNotFound(ref n) if n == "nonexistent"));
}

#[tokio::test]
async fn resolve_field_map_finds_all_three_required_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": 1, "name": "total_amount", "data_type": "float"},
                {"id": 5, "name": "litres", "data_type": "float"},
                {"id": 6, "name": "receipt_datetime", "data_type": "string"},
                {"id": 4, "name": "order_id", "data_type": "string"},
            ]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let map = client.resolve_field_map().await.unwrap();
    assert_eq!(map.total_amount_id, 1);
    assert_eq!(map.litres_id, 5);
    assert_eq!(map.receipt_datetime_id, 6);
}

#[tokio::test]
async fn resolve_field_map_errors_when_required_field_missing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"id": 1, "name": "total_amount", "data_type": "float"}]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let err = client.resolve_field_map().await.unwrap_err();
    match err {
        PaperlessError::CustomFieldNotFound(ref n) => {
            assert!(n == "litres" || n == "receipt_datetime");
        }
        _ => panic!("expected CustomFieldNotFound, got {:?}", err),
    }
}
