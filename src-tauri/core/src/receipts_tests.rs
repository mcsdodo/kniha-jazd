//! Tests for receipts module

use super::*;
use crate::gemini::ExtractionConfidence;
use std::fs::{self, File};
use tempfile::TempDir;

// Helper to create a temp directory with specific structure
fn create_test_folder_structure(files: &[&str], folders: &[&str]) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    for file in files {
        let file_path = temp_dir.path().join(file);
        if let Some(parent) = file_path.parent() {
            if parent != temp_dir.path() {
                fs::create_dir_all(parent).unwrap();
            }
        }
        File::create(file_path).unwrap();
    }

    for folder in folders {
        fs::create_dir_all(temp_dir.path().join(folder)).unwrap();
    }

    temp_dir
}

// ===========================================
// Folder Structure Detection Tests
// ===========================================

#[test]
fn test_detect_flat_structure_with_images() {
    let temp = create_test_folder_structure(&["a.jpg", "b.png", "c.jpeg"], &[]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    assert_eq!(result, FolderStructure::Flat);
}

#[test]
fn test_detect_flat_structure_empty_folder() {
    let temp = create_test_folder_structure(&[], &[]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    // Empty folder is treated as Flat (nothing to scan)
    assert_eq!(result, FolderStructure::Flat);
}

#[test]
fn test_detect_flat_structure_ignores_non_image_files() {
    let temp = create_test_folder_structure(&["receipt.jpg", "notes.txt", "data.json"], &[]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    // Only considers supported image files
    assert_eq!(result, FolderStructure::Flat);
}

#[test]
fn test_detect_year_based_structure() {
    let temp = create_test_folder_structure(&[], &["2024", "2025"]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    assert_eq!(result, FolderStructure::YearBased(vec![2024, 2025]));
}

#[test]
fn test_detect_year_based_structure_single_year() {
    let temp = create_test_folder_structure(&[], &["2024"]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    assert_eq!(result, FolderStructure::YearBased(vec![2024]));
}

#[test]
fn test_detect_year_based_structure_sorted() {
    let temp = create_test_folder_structure(&[], &["2025", "2023", "2024"]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    // Years should be sorted
    assert_eq!(result, FolderStructure::YearBased(vec![2023, 2024, 2025]));
}

#[test]
fn test_detect_invalid_mixed_files_and_year_folders() {
    let temp = create_test_folder_structure(&["receipt.jpg"], &["2024"]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    match result {
        FolderStructure::Invalid(reason) => {
            assert!(
                reason.contains("Mixed"),
                "Expected 'Mixed' in reason: {}",
                reason
            );
        }
        _ => panic!("Expected Invalid, got {:?}", result),
    }
}

#[test]
fn test_detect_invalid_non_year_folders() {
    let temp = create_test_folder_structure(&[], &["January", "misc"]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    match result {
        FolderStructure::Invalid(reason) => {
            assert!(
                reason.contains("January"),
                "Expected 'January' in reason: {}",
                reason
            );
            assert!(
                reason.contains("misc"),
                "Expected 'misc' in reason: {}",
                reason
            );
        }
        _ => panic!("Expected Invalid, got {:?}", result),
    }
}

#[test]
fn test_detect_invalid_mixed_year_and_non_year_folders() {
    let temp = create_test_folder_structure(&[], &["2024", "misc"]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    match result {
        FolderStructure::Invalid(reason) => {
            assert!(
                reason.contains("misc"),
                "Expected 'misc' in reason: {}",
                reason
            );
        }
        _ => panic!("Expected Invalid, got {:?}", result),
    }
}

#[test]
fn test_detect_invalid_path_not_exists() {
    let result = detect_folder_structure("/nonexistent/path/to/folder");
    match result {
        FolderStructure::Invalid(reason) => {
            assert!(
                reason.contains("not a valid directory"),
                "Expected 'not a valid directory' in reason: {}",
                reason
            );
        }
        _ => panic!("Expected Invalid, got {:?}", result),
    }
}

#[test]
fn test_detect_invalid_files_with_non_year_folders() {
    let temp = create_test_folder_structure(&["receipt.jpg"], &["backup"]);

    let result = detect_folder_structure(temp.path().to_str().unwrap());
    match result {
        FolderStructure::Invalid(reason) => {
            assert!(
                reason.contains("Mixed") || reason.contains("non-year"),
                "Expected mixed/non-year in reason: {}",
                reason
            );
        }
        _ => panic!("Expected Invalid, got {:?}", result),
    }
}

// ===========================================
// Scanning Tests with Folder Structures
// ===========================================

#[test]
fn test_scan_year_folders_populates_source_year() {
    let temp = create_test_folder_structure(&["2024/receipt1.jpg", "2025/receipt2.jpg"], &[]);

    let db = crate::db::Database::in_memory().unwrap();
    let receipts = scan_folder_for_new_receipts(temp.path().to_str().unwrap(), &db).unwrap();

    assert_eq!(receipts.len(), 2);

    // Find the receipt from 2024 folder
    let receipt_2024 = receipts
        .iter()
        .find(|r| r.file_path.contains("2024"))
        .expect("Should find receipt from 2024 folder");
    assert_eq!(receipt_2024.source_year, Some(2024));

    // Find the receipt from 2025 folder
    let receipt_2025 = receipts
        .iter()
        .find(|r| r.file_path.contains("2025"))
        .expect("Should find receipt from 2025 folder");
    assert_eq!(receipt_2025.source_year, Some(2025));
}

#[test]
fn test_scan_flat_folder_has_no_source_year() {
    let temp = create_test_folder_structure(&["receipt1.jpg", "receipt2.png"], &[]);

    let db = crate::db::Database::in_memory().unwrap();
    let receipts = scan_folder_for_new_receipts(temp.path().to_str().unwrap(), &db).unwrap();

    assert_eq!(receipts.len(), 2);
    for receipt in &receipts {
        assert_eq!(
            receipt.source_year, None,
            "Flat folder should not set source_year"
        );
    }
}

#[test]
fn test_scan_invalid_structure_returns_error() {
    let temp = create_test_folder_structure(&["receipt.jpg"], &["2024"]);

    let db = crate::db::Database::in_memory().unwrap();
    let result = scan_folder_for_new_receipts(temp.path().to_str().unwrap(), &db);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Invalid folder structure"),
        "Expected 'Invalid folder structure' in error: {}",
        err
    );
}

// ===========================================
// Existing Extraction Tests
// ===========================================

#[test]
fn test_apply_extraction_high_confidence_eur_full_datetime() {
    let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
    let extracted = ExtractedReceipt {
        liters: Some(45.5),
        total_price_eur: None, // Not used directly anymore
        original_amount: Some(72.50),
        original_currency: Some("EUR".to_string()),
        receipt_datetime: Some("2024-12-15T14:32:00".to_string()),
        station_name: Some("Slovnaft".to_string()),
        station_address: Some("Bratislava".to_string()),
        vendor_name: None,
        cost_description: None,
        raw_text: Some("OCR text".to_string()),
        confidence: ExtractionConfidence {
            liters: "high".to_string(),
            total_price: "high".to_string(),
            date: "high".to_string(),
            currency: "high".to_string(),
        },
    };

    apply_extraction_to_receipt(&mut receipt, extracted);

    assert_eq!(receipt.liters, Some(45.5));
    assert_eq!(receipt.original_amount, Some(72.50));
    assert_eq!(receipt.original_currency, Some("EUR".to_string()));
    assert_eq!(receipt.total_price_eur, Some(72.50)); // Auto-populated from EUR
    assert_eq!(receipt.status, ReceiptStatus::Parsed);
    assert_eq!(receipt.confidence.liters, ConfidenceLevel::High);
    assert_eq!(receipt.confidence.total_price, ConfidenceLevel::High);
    assert_eq!(receipt.confidence.date, ConfidenceLevel::High);
    // Verify datetime was parsed correctly
    let datetime = receipt.receipt_datetime.unwrap();
    assert_eq!(datetime.format("%Y-%m-%dT%H:%M:%S").to_string(), "2024-12-15T14:32:00");
}

#[test]
fn test_apply_extraction_foreign_currency_needs_review() {
    // CZK receipt should set status to NeedsReview (foreign currency)
    let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
    let extracted = ExtractedReceipt {
        liters: None,
        total_price_eur: None,
        original_amount: Some(100.0),
        original_currency: Some("CZK".to_string()),
        receipt_datetime: Some("2024-12-15T10:30:00".to_string()),
        station_name: None,
        station_address: None,
        vendor_name: Some("Parking Praha".to_string()),
        cost_description: Some("Parkovne".to_string()),
        raw_text: Some("100 Kč".to_string()),
        confidence: ExtractionConfidence {
            liters: "not_applicable".to_string(),
            total_price: "high".to_string(),
            date: "high".to_string(),
            currency: "high".to_string(),
        },
    };

    apply_extraction_to_receipt(&mut receipt, extracted);

    assert_eq!(receipt.original_amount, Some(100.0));
    assert_eq!(receipt.original_currency, Some("CZK".to_string()));
    assert!(receipt.total_price_eur.is_none()); // Not set for foreign currency
    assert_eq!(receipt.status, ReceiptStatus::NeedsReview); // Needs user conversion
}

#[test]
fn test_apply_extraction_low_confidence() {
    let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
    let extracted = ExtractedReceipt {
        liters: None,
        total_price_eur: Some(50.00), // Legacy fallback
        original_amount: Some(50.00),
        original_currency: None, // Unknown currency
        receipt_datetime: None,
        station_name: None,
        station_address: None,
        vendor_name: None,
        cost_description: None,
        raw_text: Some("blurry".to_string()),
        confidence: ExtractionConfidence {
            liters: "low".to_string(),
            total_price: "medium".to_string(),
            date: "low".to_string(),
            currency: "low".to_string(),
        },
    };

    apply_extraction_to_receipt(&mut receipt, extracted);

    assert_eq!(receipt.status, ReceiptStatus::NeedsReview);
    assert_eq!(receipt.confidence.liters, ConfidenceLevel::Low);
    assert_eq!(receipt.confidence.date, ConfidenceLevel::Low);
}

#[test]
fn test_parse_confidence() {
    assert_eq!(parse_confidence("high"), ConfidenceLevel::High);
    assert_eq!(parse_confidence("HIGH"), ConfidenceLevel::High);
    assert_eq!(parse_confidence("medium"), ConfidenceLevel::Medium);
    assert_eq!(parse_confidence("low"), ConfidenceLevel::Low);
    assert_eq!(parse_confidence("unknown"), ConfidenceLevel::Unknown);
    assert_eq!(parse_confidence("invalid"), ConfidenceLevel::Unknown);
}

// ===========================================
// Datetime Parsing Tests
// ===========================================

#[test]
fn test_parse_receipt_datetime_full_datetime() {
    // Full datetime (YYYY-MM-DDTHH:MM:SS) should parse correctly and not trigger NeedsReview
    let (datetime, needs_review) = parse_receipt_datetime(Some("2026-01-15T14:32:00".to_string()));

    assert!(datetime.is_some());
    let dt = datetime.unwrap();
    assert_eq!(dt.format("%Y-%m-%dT%H:%M:%S").to_string(), "2026-01-15T14:32:00");
    assert!(!needs_review, "Full datetime should not trigger NeedsReview");
}

#[test]
fn test_parse_receipt_datetime_date_only_triggers_needs_review() {
    // Date-only (YYYY-MM-DD) should parse with midnight time and trigger NeedsReview
    let (datetime, needs_review) = parse_receipt_datetime(Some("2026-01-15".to_string()));

    assert!(datetime.is_some());
    let dt = datetime.unwrap();
    assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-01-15");
    assert_eq!(dt.format("%H:%M:%S").to_string(), "00:00:00"); // Midnight
    assert!(needs_review, "Date-only should trigger NeedsReview");
}

#[test]
fn test_parse_receipt_datetime_none() {
    // None should return None and not trigger NeedsReview
    let (datetime, needs_review) = parse_receipt_datetime(None);

    assert!(datetime.is_none());
    assert!(!needs_review, "None datetime should not trigger NeedsReview flag");
}

#[test]
fn test_parse_receipt_datetime_invalid_format() {
    // Invalid format should return None
    let (datetime, needs_review) = parse_receipt_datetime(Some("invalid-date".to_string()));

    assert!(datetime.is_none());
    assert!(!needs_review);
}

#[test]
fn test_apply_extraction_date_only_triggers_needs_review() {
    // Date-only extraction should mark receipt as NeedsReview even with high confidence
    let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
    let extracted = ExtractedReceipt {
        liters: Some(45.5),
        total_price_eur: None,
        original_amount: Some(72.50),
        original_currency: Some("EUR".to_string()),
        receipt_datetime: Some("2026-01-15".to_string()), // Date only, no time
        station_name: Some("Slovnaft".to_string()),
        station_address: Some("Bratislava".to_string()),
        vendor_name: None,
        cost_description: None,
        raw_text: Some("OCR text".to_string()),
        confidence: ExtractionConfidence {
            liters: "high".to_string(),
            total_price: "high".to_string(),
            date: "high".to_string(),
            currency: "high".to_string(),
        },
    };

    apply_extraction_to_receipt(&mut receipt, extracted);

    // Should be NeedsReview because time is missing
    assert_eq!(receipt.status, ReceiptStatus::NeedsReview);
    // But datetime should still be parsed (with midnight)
    let datetime = receipt.receipt_datetime.unwrap();
    assert_eq!(datetime.format("%Y-%m-%d").to_string(), "2026-01-15");
    assert_eq!(datetime.format("%H:%M:%S").to_string(), "00:00:00");
}
