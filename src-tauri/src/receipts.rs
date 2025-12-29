//! Receipt folder scanning and processing service

use crate::db::Database;
use crate::gemini::{ExtractedReceipt, GeminiClient};
use crate::models::{ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus};
use chrono::NaiveDate;
use std::path::Path;

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "pdf"];

/// Scan folder for new receipt images and return count of new files found
pub fn scan_folder_for_new_receipts(
    folder_path: &str,
    db: &Database,
) -> Result<Vec<Receipt>, String> {
    let path = Path::new(folder_path);
    if !path.exists() {
        return Err(format!("Folder does not exist: {}", folder_path));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path));
    }

    let mut new_receipts = Vec::new();

    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.flatten() {
        let file_path = entry.path();

        // Skip non-files
        if !file_path.is_file() {
            continue;
        }

        // Check extension
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        if !extension.map(|e| SUPPORTED_EXTENSIONS.contains(&e.as_str())).unwrap_or(false) {
            continue;
        }

        let file_path_str = file_path.to_string_lossy().to_string();

        // Check if already processed
        if db.get_receipt_by_file_path(&file_path_str)
            .map_err(|e| e.to_string())?
            .is_some()
        {
            continue;
        }

        // Create new receipt record
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let receipt = Receipt::new(file_path_str, file_name);
        db.create_receipt(&receipt).map_err(|e| e.to_string())?;
        new_receipts.push(receipt);
    }

    Ok(new_receipts)
}

/// Process a pending receipt with Gemini API (async)
pub async fn process_receipt_with_gemini(
    receipt: &mut Receipt,
    api_key: &str,
) -> Result<(), String> {
    let client = GeminiClient::new(api_key.to_string());
    let path = Path::new(&receipt.file_path);

    match client.extract_from_image(path).await {
        Ok(extracted) => {
            apply_extraction_to_receipt(receipt, extracted);
            Ok(())
        }
        Err(e) => {
            receipt.status = ReceiptStatus::NeedsReview;
            receipt.error_message = Some(e.clone());
            Err(e)
        }
    }
}

/// Convert string confidence to typed ConfidenceLevel
fn parse_confidence(s: &str) -> ConfidenceLevel {
    match s.to_lowercase().as_str() {
        "high" => ConfidenceLevel::High,
        "medium" => ConfidenceLevel::Medium,
        "low" => ConfidenceLevel::Low,
        _ => ConfidenceLevel::Unknown,
    }
}

fn apply_extraction_to_receipt(receipt: &mut Receipt, extracted: ExtractedReceipt) {
    receipt.liters = extracted.liters;
    receipt.total_price_eur = extracted.total_price_eur;
    receipt.receipt_date = extracted.receipt_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
    receipt.station_name = extracted.station_name;
    receipt.station_address = extracted.station_address;
    receipt.raw_ocr_text = extracted.raw_text;

    // Map confidence from API response to typed struct
    receipt.confidence = FieldConfidence {
        liters: parse_confidence(&extracted.confidence.liters),
        total_price: parse_confidence(&extracted.confidence.total_price),
        date: parse_confidence(&extracted.confidence.date),
    };

    // Determine status based on confidence and data presence
    let has_uncertainty =
        matches!(receipt.confidence.liters, ConfidenceLevel::Low | ConfidenceLevel::Unknown)
        || matches!(receipt.confidence.total_price, ConfidenceLevel::Low | ConfidenceLevel::Unknown)
        || matches!(receipt.confidence.date, ConfidenceLevel::Low | ConfidenceLevel::Unknown)
        || receipt.liters.is_none()
        || receipt.total_price_eur.is_none()
        || receipt.receipt_date.is_none();

    if has_uncertainty {
        receipt.status = ReceiptStatus::NeedsReview;
    } else {
        receipt.status = ReceiptStatus::Parsed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::ExtractionConfidence;

    #[test]
    fn test_apply_extraction_high_confidence() {
        let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
        let extracted = ExtractedReceipt {
            liters: Some(45.5),
            total_price_eur: Some(72.50),
            receipt_date: Some("2024-12-15".to_string()),
            station_name: Some("Slovnaft".to_string()),
            station_address: Some("Bratislava".to_string()),
            raw_text: Some("OCR text".to_string()),
            confidence: ExtractionConfidence {
                liters: "high".to_string(),
                total_price: "high".to_string(),
                date: "high".to_string(),
            },
        };

        apply_extraction_to_receipt(&mut receipt, extracted);

        assert_eq!(receipt.liters, Some(45.5));
        assert_eq!(receipt.total_price_eur, Some(72.50));
        assert_eq!(receipt.status, ReceiptStatus::Parsed);
        assert_eq!(receipt.confidence.liters, ConfidenceLevel::High);
        assert_eq!(receipt.confidence.total_price, ConfidenceLevel::High);
        assert_eq!(receipt.confidence.date, ConfidenceLevel::High);
    }

    #[test]
    fn test_apply_extraction_low_confidence() {
        let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
        let extracted = ExtractedReceipt {
            liters: None,
            total_price_eur: Some(50.00),
            receipt_date: None,
            station_name: None,
            station_address: None,
            raw_text: Some("blurry".to_string()),
            confidence: ExtractionConfidence {
                liters: "low".to_string(),
                total_price: "medium".to_string(),
                date: "low".to_string(),
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
}
