//! Tests for Gemini API client

use super::*;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_extracted_receipt_deserialization_fuel_eur() {
    // EUR fuel receipt: has liters, station info, EUR currency
    let json = r#"{
        "liters": 45.5,
        "total_price_eur": 72.50,
        "original_amount": 72.50,
        "original_currency": "EUR",
        "receipt_date": "2024-12-15",
        "station_name": "Slovnaft",
        "station_address": "Bratislava",
        "vendor_name": null,
        "cost_description": null,
        "raw_text": "some text",
        "confidence": {
            "liters": "high",
            "total_price": "high",
            "date": "medium",
            "currency": "high"
        }
    }"#;

    let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
    assert_eq!(extracted.liters, Some(45.5));
    assert_eq!(extracted.total_price_eur, Some(72.50));
    assert_eq!(extracted.original_amount, Some(72.50));
    assert_eq!(extracted.original_currency, Some("EUR".to_string()));
    assert_eq!(extracted.receipt_date, Some("2024-12-15".to_string()));
    assert_eq!(extracted.station_name, Some("Slovnaft".to_string()));
    assert_eq!(extracted.confidence.liters, "high");
    assert_eq!(extracted.confidence.currency, "high");
    // Non-fuel fields should be null for fuel receipts
    assert!(extracted.vendor_name.is_none());
    assert!(extracted.cost_description.is_none());
}

#[test]
fn test_extracted_receipt_deserialization_fuel_czk() {
    // CZK fuel receipt: foreign currency parking receipt
    let json = r#"{
        "liters": null,
        "total_price_eur": null,
        "original_amount": 100.0,
        "original_currency": "CZK",
        "receipt_date": "2024-12-15",
        "station_name": null,
        "station_address": null,
        "vendor_name": "Parkoviště Praha",
        "cost_description": "Parkovné 2h",
        "raw_text": "100 Kč",
        "confidence": {
            "liters": "not_applicable",
            "total_price": "high",
            "date": "high",
            "currency": "high"
        }
    }"#;

    let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
    assert!(extracted.liters.is_none());
    assert!(extracted.total_price_eur.is_none()); // No EUR value yet
    assert_eq!(extracted.original_amount, Some(100.0));
    assert_eq!(extracted.original_currency, Some("CZK".to_string()));
    assert_eq!(extracted.vendor_name, Some("Parkoviště Praha".to_string()));
    assert_eq!(extracted.confidence.currency, "high");
}

#[test]
fn test_extracted_receipt_deserialization_other_cost() {
    // Non-fuel receipt: no liters, has vendor info (EUR)
    let json = r#"{
        "liters": null,
        "total_price_eur": 15.00,
        "original_amount": 15.00,
        "original_currency": "EUR",
        "receipt_date": "2024-12-16",
        "station_name": null,
        "station_address": null,
        "vendor_name": "AutoUmyváreň SK",
        "cost_description": "Umytie auta - komplet",
        "raw_text": "AutoUmyváreň SK\nUmytie komplet 15.00 EUR",
        "confidence": {
            "liters": "not_applicable",
            "total_price": "high",
            "date": "high",
            "currency": "high"
        }
    }"#;

    let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
    assert!(extracted.liters.is_none());
    assert_eq!(extracted.total_price_eur, Some(15.00));
    assert_eq!(extracted.original_amount, Some(15.00));
    assert_eq!(extracted.original_currency, Some("EUR".to_string()));
    assert_eq!(extracted.receipt_date, Some("2024-12-16".to_string()));
    assert_eq!(extracted.vendor_name, Some("AutoUmyváreň SK".to_string()));
    assert_eq!(
        extracted.cost_description,
        Some("Umytie auta - komplet".to_string())
    );
    assert_eq!(extracted.confidence.liters, "not_applicable");
    // Fuel-specific fields should be null for non-fuel receipts
    assert!(extracted.station_name.is_none());
    assert!(extracted.station_address.is_none());
}

#[test]
fn test_extracted_receipt_with_nulls() {
    // Blurry receipt with minimal data - unknown currency
    let json = r#"{
        "liters": null,
        "total_price_eur": null,
        "original_amount": 50.00,
        "original_currency": null,
        "receipt_date": null,
        "station_name": null,
        "station_address": null,
        "vendor_name": null,
        "cost_description": null,
        "raw_text": "blurry text",
        "confidence": {
            "liters": "low",
            "total_price": "medium",
            "date": "low",
            "currency": "low"
        }
    }"#;

    let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
    assert!(extracted.liters.is_none());
    assert!(extracted.total_price_eur.is_none());
    assert_eq!(extracted.original_amount, Some(50.00));
    assert!(extracted.original_currency.is_none());
    assert!(extracted.receipt_date.is_none());
    assert!(extracted.vendor_name.is_none());
    assert!(extracted.cost_description.is_none());
}

#[test]
fn test_response_schema_is_valid_json() {
    let schema = get_response_schema();
    assert!(schema.is_object());
    assert!(schema.get("type").is_some());
    assert!(schema.get("properties").is_some());
}

// =============================================================================
// Mock Loading Tests
// =============================================================================

#[test]
fn test_load_mock_extraction_valid_file() {
    // Create a temp directory with a mock JSON file
    let mock_dir = tempdir().unwrap();
    let mock_file = mock_dir.path().join("invoice.json");

    let mock_json = r#"{
        "liters": 63.68,
        "total_price_eur": 91.32,
        "original_amount": 91.32,
        "original_currency": "EUR",
        "receipt_date": "2026-01-20",
        "station_name": "Slovnaft, a.s.",
        "station_address": "Prístavna ulica, Bratislava",
        "vendor_name": null,
        "cost_description": null,
        "raw_text": null,
        "confidence": {
            "liters": "high",
            "total_price": "high",
            "date": "high",
            "currency": "high"
        }
    }"#;

    std::fs::File::create(&mock_file)
        .unwrap()
        .write_all(mock_json.as_bytes())
        .unwrap();

    // Load mock for "invoice.pdf"
    let image_path = Path::new("/some/path/invoice.pdf");
    let result = load_mock_extraction(mock_dir.path().to_str().unwrap(), image_path);

    assert!(result.is_ok());
    let extracted = result.unwrap();
    assert_eq!(extracted.liters, Some(63.68));
    assert_eq!(extracted.total_price_eur, Some(91.32));
    assert_eq!(extracted.original_amount, Some(91.32));
    assert_eq!(extracted.original_currency, Some("EUR".to_string()));
    assert_eq!(extracted.receipt_date, Some("2026-01-20".to_string()));
    assert_eq!(extracted.station_name, Some("Slovnaft, a.s.".to_string()));
    assert_eq!(extracted.confidence.liters, "high");
    assert_eq!(extracted.confidence.currency, "high");
}

#[test]
fn test_load_mock_extraction_missing_file_returns_default() {
    let mock_dir = tempdir().unwrap();
    // No mock file created

    let image_path = Path::new("/some/path/missing.pdf");
    let result = load_mock_extraction(mock_dir.path().to_str().unwrap(), image_path);

    assert!(result.is_ok());
    let extracted = result.unwrap();
    // Default values
    assert!(extracted.liters.is_none());
    assert!(extracted.total_price_eur.is_none());
    assert!(extracted.receipt_date.is_none());
    assert_eq!(extracted.confidence.liters, "low");
}

#[test]
fn test_load_mock_extraction_invalid_json() {
    let mock_dir = tempdir().unwrap();
    let mock_file = mock_dir.path().join("bad.json");

    // Write invalid JSON
    std::fs::File::create(&mock_file)
        .unwrap()
        .write_all(b"{ invalid json }")
        .unwrap();

    let image_path = Path::new("/some/path/bad.pdf");
    let result = load_mock_extraction(mock_dir.path().to_str().unwrap(), image_path);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to parse mock JSON"));
}

#[test]
fn test_extracted_receipt_default() {
    let default = ExtractedReceipt::default();

    assert!(default.liters.is_none());
    assert!(default.total_price_eur.is_none());
    assert!(default.original_amount.is_none());
    assert!(default.original_currency.is_none());
    assert!(default.receipt_date.is_none());
    assert!(default.station_name.is_none());
    assert!(default.station_address.is_none());
    assert!(default.vendor_name.is_none());
    assert!(default.cost_description.is_none());
    assert!(default.raw_text.is_none());
    assert_eq!(default.confidence.liters, "low");
    assert_eq!(default.confidence.total_price, "low");
    assert_eq!(default.confidence.date, "low");
    assert_eq!(default.confidence.currency, "low");
}
