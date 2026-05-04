//! Tests for paperless module.
use super::*;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn paperless_field_names_default_matches_app_vocabulary() {
    let n = PaperlessFieldNames::default();
    assert_eq!(n.datetime, "receipt_datetime");
    assert_eq!(n.liters, "liters");
    assert_eq!(n.total, "total_price_eur");
}

#[test]
fn paperless_field_names_from_settings_uses_defaults_when_none() {
    let s = crate::settings::LocalSettings::default();
    let n = PaperlessFieldNames::from_settings(&s);
    assert_eq!(n.datetime, "receipt_datetime");
    assert_eq!(n.liters, "liters");
    assert_eq!(n.total, "total_price_eur");
}

#[test]
fn paperless_field_names_from_settings_uses_defaults_when_empty_strings() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_field_name_datetime = Some("".to_string());
    s.paperless_field_name_liters = Some("   ".to_string());
    s.paperless_field_name_total = Some("\t".to_string());
    let n = PaperlessFieldNames::from_settings(&s);
    assert_eq!(n.datetime, "receipt_datetime");
    assert_eq!(n.liters, "liters");
    assert_eq!(n.total, "total_price_eur");
}

#[test]
fn paperless_field_names_from_settings_uses_custom_when_set() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_field_name_datetime = Some("Dátum dokladu".to_string());
    s.paperless_field_name_liters = Some("Litre".to_string());
    s.paperless_field_name_total = Some("Suma".to_string());
    let n = PaperlessFieldNames::from_settings(&s);
    assert_eq!(n.datetime, "Dátum dokladu");
    assert_eq!(n.liters, "Litre");
    assert_eq!(n.total, "Suma");
}

#[tokio::test]
async fn resolve_field_map_uses_custom_names_when_provided() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": 11, "name": "Suma",          "data_type": "float"},
                {"id": 12, "name": "Litre",         "data_type": "float"},
                {"id": 13, "name": "Dátum dokladu", "data_type": "string"},
            ]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let names = PaperlessFieldNames {
        datetime: "Dátum dokladu".into(),
        liters:   "Litre".into(),
        total:    "Suma".into(),
    };
    let map = client.resolve_field_map(&names).await.unwrap();
    assert_eq!(map.total_amount_id, 11);
    assert_eq!(map.litres_id, 12);
    assert_eq!(map.receipt_datetime_id, 13);
}

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
async fn list_custom_fields_returns_all_with_types() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": 1, "name": "total_amount",     "data_type": "float"},
                {"id": 2, "name": "total_amount_alt", "data_type": "float"},
                {"id": 4, "name": "order_id",         "data_type": "string"},
                {"id": 5, "name": "litres",           "data_type": "float"},
                {"id": 6, "name": "receipt_datetime", "data_type": "string"},
            ]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let fields = client.list_custom_fields().await.unwrap();

    assert_eq!(fields.len(), 5);
    let by_name: std::collections::HashMap<&str, &CustomFieldInfo> =
        fields.iter().map(|f| (f.name.as_str(), f)).collect();
    assert_eq!(by_name["litres"].data_type, "float");
    assert_eq!(by_name["receipt_datetime"].data_type, "string");
    assert_eq!(by_name["total_amount_alt"].id, 2);
}

#[tokio::test]
async fn list_custom_fields_propagates_http_errors() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let err = client.list_custom_fields().await.unwrap_err();
    match err {
        PaperlessError::Http(status) => assert_eq!(status, 401),
        other => panic!("expected Http(401), got {:?}", other),
    }
}

#[tokio::test]
async fn resolve_field_map_finds_all_three_required_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": 1, "name": "total_price_eur",  "data_type": "float"},
                {"id": 5, "name": "liters",           "data_type": "float"},
                {"id": 6, "name": "receipt_datetime", "data_type": "string"},
                {"id": 4, "name": "order_id",         "data_type": "string"},
            ]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let map = client.resolve_field_map(&PaperlessFieldNames::default()).await.unwrap();
    assert_eq!(map.total_amount_id, 1);
    assert_eq!(map.litres_id, 5);
    assert_eq!(map.receipt_datetime_id, 6);
}

#[tokio::test]
async fn resolve_field_map_errors_when_required_field_missing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"id": 1, "name": "total_price_eur", "data_type": "float"}]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let err = client.resolve_field_map(&PaperlessFieldNames::default()).await.unwrap_err();
    match err {
        PaperlessError::CustomFieldNotFound(ref n) => {
            assert!(n == "liters" || n == "receipt_datetime");
        }
        _ => panic!("expected CustomFieldNotFound, got {:?}", err),
    }
}

#[tokio::test]
async fn fetch_documents_parses_real_fuel_doc_with_litres() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/documents/"))
        .and(query_param("tags__id__in", "51,59"))
        .and(query_param("page_size", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1, "next": null,
            "results": [{
                "id": 435, "title": "OMV Slovensko, s.r.o. - Scanned_20260427-1325",
                "tags": [54, 55, 51, 48], "created": "2026-04-27",
                "custom_fields": [
                    {"value": 113.95, "field": 1},
                    {"value": 63.34, "field": 5},
                    {"value": "2026-04-27T13:24:14", "field": 6}
                ]
            }]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let map = PaperlessFieldMap { total_amount_id: 1, litres_id: 5, receipt_datetime_id: 6 };
    let docs = client.fetch_invoice_documents(51, 59, &map).await.unwrap();

    assert_eq!(docs.len(), 1);
    let d = &docs[0];
    assert_eq!(d.id, 435);
    assert_eq!(d.title, "OMV Slovensko, s.r.o. - Scanned_20260427-1325");
    assert_eq!(d.tag_ids, vec![54, 55, 51, 48]);
    assert_eq!(d.created, chrono::NaiveDate::from_ymd_opt(2026, 4, 27).unwrap());
    assert_eq!(d.total_amount, Some(113.95));
    assert_eq!(d.litres, Some(63.34));
    assert_eq!(d.receipt_datetime,
               chrono::NaiveDateTime::parse_from_str("2026-04-27T13:24:14", "%Y-%m-%dT%H:%M:%S").ok());
}

#[tokio::test]
async fn fetch_documents_parses_car_doc_with_no_litres() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/documents/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1, "next": null,
            "results": [{
                "id": 423, "title": "Hlavné mesto SR Bratislava - 1776180674432",
                "tags": [54, 55, 59, 48], "created": "2026-04-14",
                "custom_fields": [
                    {"value": 1.95, "field": 1},
                    {"value": "1776180674432", "field": 4},
                    {"value": "2026-04-14T15:31:00", "field": 6}
                ]
            }]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock.uri(), "tok".into());
    let map = PaperlessFieldMap { total_amount_id: 1, litres_id: 5, receipt_datetime_id: 6 };
    let docs = client.fetch_invoice_documents(51, 59, &map).await.unwrap();
    assert_eq!(docs[0].litres, None);
    assert_eq!(docs[0].total_amount, Some(1.95));
}

#[tokio::test]
async fn fetch_documents_follows_pagination_next_link() {
    let mock = MockServer::start().await;
    let mock_uri = mock.uri();

    Mock::given(method("GET")).and(path("/api/documents/"))
        .and(query_param("page_size", "100"))
        .and(query_param_is_missing("page"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 2,
            "next": format!("{}/api/documents/?page=2&page_size=100&tags__id__in=51%2C59", mock_uri),
            "results": [{
                "id": 1, "title": "p1", "tags": [51], "created": "2026-01-01",
                "custom_fields": [{"value": 10.0, "field": 1}, {"value": "2026-01-01T00:00:00", "field": 6}]
            }]
        })))
        .mount(&mock).await;

    Mock::given(method("GET")).and(path("/api/documents/")).and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 2, "next": null,
            "results": [{
                "id": 2, "title": "p2", "tags": [59], "created": "2026-01-02",
                "custom_fields": [{"value": 20.0, "field": 1}, {"value": "2026-01-02T00:00:00", "field": 6}]
            }]
        })))
        .mount(&mock).await;

    let client = PaperlessClient::new(mock_uri, "tok".into());
    let map = PaperlessFieldMap { total_amount_id: 1, litres_id: 5, receipt_datetime_id: 6 };
    let docs = client.fetch_invoice_documents(51, 59, &map).await.unwrap();
    assert_eq!(docs.iter().map(|d| d.id).collect::<Vec<_>>(), vec![1, 2]);
}
