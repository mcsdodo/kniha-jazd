//! Tests for the Invoice trait (Task 64).
use super::*;
use chrono::{NaiveDate, Utc};
use uuid::Uuid;
use crate::models::{
    ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus,
};

fn make_receipt() -> Receipt {
    let now = Utc::now();
    Receipt {
        id: Uuid::nil(),
        vehicle_id: None,
        trip_id: None,
        file_path: "/x/test.jpg".to_string(),
        file_name: "test.jpg".to_string(),
        scanned_at: now,
        liters: Some(40.5),
        total_price_eur: Some(58.20),
        receipt_datetime: NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()
            .and_hms_opt(13, 24, 14),
        station_name: None,
        station_address: None,
        vendor_name: None,
        cost_description: None,
        original_amount: Some(58.20),
        original_currency: Some("EUR".to_string()),
        source_year: None,
        status: ReceiptStatus::Parsed,
        confidence: FieldConfidence {
            liters: ConfidenceLevel::High,
            total_price: ConfidenceLevel::High,
            date: ConfidenceLevel::High,
        },
        raw_ocr_text: None,
        error_message: None,
        assignment_type: None,
        mismatch_override: false,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn receipt_implements_invoice_trait_with_correct_field_mapping() {
    let r = make_receipt();
    let inv: &dyn Invoice = &r;
    assert_eq!(inv.datetime(), r.receipt_datetime);
    assert_eq!(inv.liters(), Some(40.5));
    assert_eq!(inv.total_price_eur(), Some(58.20));
    assert_eq!(inv.display_name(), "test.jpg");
    assert_eq!(inv.invoice_ref(), InvoiceRef::Receipt(Uuid::nil().to_string()));
    assert_eq!(inv.assignment_type(), None);
}

use crate::paperless::PaperlessDoc;

#[test]
fn paperless_doc_implements_invoice_trait_with_uk_us_field_bridge() {
    let doc = PaperlessDoc {
        id: 435,
        title: "Tank Mol Bratislava".into(),
        tag_ids: vec![51], // fuel
        created: chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
        total_amount: Some(58.20),  // UK→US bridge
        litres: Some(40.5),         // UK→US bridge
        receipt_datetime: chrono::NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()
            .and_hms_opt(13, 24, 14),
    };
    let inv: &dyn Invoice = &doc;
    assert_eq!(inv.liters(), Some(40.5));
    assert_eq!(inv.total_price_eur(), Some(58.20));
    assert_eq!(inv.display_name(), "Tank Mol Bratislava");
    assert_eq!(inv.invoice_ref(), InvoiceRef::Paperless(435));
}

#[test]
fn invoice_ref_serde_shape_matches_design() {
    let r = InvoiceRef::Receipt("abc-123".into());
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, r#"{"source":"receipt","id":"abc-123"}"#);

    let p = InvoiceRef::Paperless(435);
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(json, r#"{"source":"paperless","id":435}"#);

    let round: InvoiceRef = serde_json::from_str(&json).unwrap();
    assert_eq!(round, p);
}
