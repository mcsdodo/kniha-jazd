//! Receipt folder scanning and processing service

use crate::db::Database;
use crate::gemini::{ExtractedReceipt, GeminiClient};
use crate::models::{ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus};
use chrono::NaiveDate;
use std::path::Path;

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "pdf"];

/// Result of detecting the folder structure for receipt scanning
#[derive(Debug, Clone, PartialEq)]
pub enum FolderStructure {
    /// Flat structure: only image files at root level
    Flat,
    /// Year-based structure: only folders named with 4-digit years (e.g., "2024", "2025")
    YearBased(Vec<i32>),
    /// Invalid/mixed structure that cannot be processed
    Invalid(String),
}

/// Detect the folder structure for receipt scanning
pub fn detect_folder_structure(path: &str) -> FolderStructure {
    let dir_path = Path::new(path);

    if !dir_path.exists() || !dir_path.is_dir() {
        return FolderStructure::Invalid(format!("Path is not a valid directory: {}", path));
    }

    let entries = match std::fs::read_dir(dir_path) {
        Ok(entries) => entries,
        Err(e) => return FolderStructure::Invalid(format!("Failed to read directory: {}", e)),
    };

    let mut has_files = false;
    let mut year_folders: Vec<i32> = Vec::new();
    let mut non_year_folders: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let file_name = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        if entry_path.is_file() {
            // Check if it's a supported image file
            let extension = entry_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());

            if extension
                .map(|e| SUPPORTED_EXTENSIONS.contains(&e.as_str()))
                .unwrap_or(false)
            {
                has_files = true;
            }
            // Ignore non-supported files (they don't affect folder structure detection)
        } else if entry_path.is_dir() {
            // Check if folder name is a 4-digit year
            if file_name.len() == 4 && file_name.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(year) = file_name.parse::<i32>() {
                    year_folders.push(year);
                }
            } else {
                non_year_folders.push(file_name);
            }
        }
    }

    // Determine structure based on what we found
    match (has_files, year_folders.is_empty(), non_year_folders.is_empty()) {
        // Only files (no folders) -> Flat
        (true, true, true) => FolderStructure::Flat,

        // Only year folders (no files, no other folders) -> YearBased
        (false, false, true) => {
            year_folders.sort();
            FolderStructure::YearBased(year_folders)
        }

        // Empty folder (no files, no folders) -> Flat (nothing to scan)
        (false, true, true) => FolderStructure::Flat,

        // Files + year folders -> Invalid (mixed)
        (true, false, _) => FolderStructure::Invalid(
            "Mixed structure: contains both image files and year folders".to_string()
        ),

        // Files + non-year folders -> Invalid (mixed)
        (true, _, false) => FolderStructure::Invalid(
            format!("Mixed structure: contains both image files and non-year folders: {}",
                    non_year_folders.join(", "))
        ),

        // Only non-year folders -> Invalid
        (false, true, false) => FolderStructure::Invalid(
            format!("Invalid folder names (expected 4-digit years): {}",
                    non_year_folders.join(", "))
        ),

        // Year folders + non-year folders -> Invalid
        (false, false, false) => FolderStructure::Invalid(
            format!("Mixed folder types: year folders and non-year folders: {}",
                    non_year_folders.join(", "))
        ),
    }
}

/// Scan folder for new receipt images and return count of new files found
/// Supports both flat folder structure and year-based folder structure.
pub fn scan_folder_for_new_receipts(
    folder_path: &str,
    db: &Database,
) -> Result<Vec<Receipt>, String> {
    // Detect folder structure first
    let structure = detect_folder_structure(folder_path);

    match structure {
        FolderStructure::Flat => {
            // Scan files directly in folder, no source_year
            scan_files_in_folder(folder_path, None, db)
        }
        FolderStructure::YearBased(years) => {
            // Scan each year folder with source_year set
            let mut all_receipts = Vec::new();
            for year in years {
                let year_folder = Path::new(folder_path).join(year.to_string());
                let year_path = year_folder.to_string_lossy().to_string();
                let receipts = scan_files_in_folder(&year_path, Some(year), db)?;
                all_receipts.extend(receipts);
            }
            Ok(all_receipts)
        }
        FolderStructure::Invalid(reason) => {
            Err(format!("Invalid folder structure: {}", reason))
        }
    }
}

/// Scan files in a single folder and create receipts
fn scan_files_in_folder(
    folder_path: &str,
    source_year: Option<i32>,
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

        // Create new receipt record with source_year
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let receipt = Receipt::new_with_source_year(file_path_str, file_name, source_year);
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
#[path = "receipts_tests.rs"]
mod tests;
