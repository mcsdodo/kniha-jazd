//! Gemini API client for receipt OCR and extraction

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Environment variable to enable mock mode for testing.
/// When set to a directory path, `extract_from_image` will load mock JSON
/// files instead of calling the Gemini API.
/// Mock file naming: {receipt_filename_stem}.json (e.g., invoice.pdf → invoice.json)
pub const MOCK_GEMINI_DIR_ENV: &str = "KNIHA_JAZD_MOCK_GEMINI_DIR";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedReceipt {
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_date: Option<String>, // YYYY-MM-DD format
    pub station_name: Option<String>,
    pub station_address: Option<String>,
    pub vendor_name: Option<String>, // For non-fuel receipts: company/shop name
    pub cost_description: Option<String>, // For non-fuel receipts: brief description
    pub original_amount: Option<f64>, // Raw amount from OCR (in original currency)
    pub original_currency: Option<String>, // "EUR", "CZK", "HUF", "PLN"
    pub raw_text: Option<String>,
    pub confidence: ExtractionConfidence,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionConfidence {
    pub liters: String, // "high", "medium", "low"
    pub total_price: String,
    pub date: String,
    pub currency: String, // confidence in currency detection
}

impl Default for ExtractedReceipt {
    fn default() -> Self {
        Self {
            liters: None,
            total_price_eur: None,
            receipt_date: None,
            station_name: None,
            station_address: None,
            vendor_name: None,
            cost_description: None,
            original_amount: None,
            original_currency: None,
            raw_text: None,
            confidence: ExtractionConfidence {
                liters: "low".to_string(),
                total_price: "low".to_string(),
                date: "low".to_string(),
                currency: "low".to_string(),
            },
        }
    }
}

/// Check if mock mode is enabled via environment variable.
/// Used by commands to skip API key validation in tests.
pub fn is_mock_mode_enabled() -> bool {
    std::env::var(MOCK_GEMINI_DIR_ENV).is_ok()
}

/// Load mock extraction data from a JSON file.
/// Used when `KNIHA_JAZD_MOCK_GEMINI_DIR` is set for testing.
pub fn load_mock_extraction(mock_dir: &str, image_path: &Path) -> Result<ExtractedReceipt, String> {
    let stem = image_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Could not get file stem from image path".to_string())?;

    let mock_file = Path::new(mock_dir).join(format!("{}.json", stem));

    if mock_file.exists() {
        let json = std::fs::read_to_string(&mock_file)
            .map_err(|e| format!("Failed to read mock file {:?}: {}", mock_file, e))?;

        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse mock JSON {:?}: {}", mock_file, e))
    } else {
        // No mock found - return default (pending-like state)
        log::warn!(
            "No mock file found for {:?} at {:?}, returning default",
            image_path,
            mock_file
        );
        Ok(ExtractedReceipt::default())
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
    #[serde(rename = "responseJsonSchema")]
    response_json_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

const EXTRACTION_PROMPT: &str = r#"Analyze this receipt/invoice image.

This could be either a FUEL receipt or OTHER expense (car wash, parking, toll, service, etc.).
Receipts may be in Slovak, Czech, Hungarian, or Polish.

Extract fields as JSON:
{
  "receipt_date": "YYYY-MM-DD" or null,
  "original_amount": number or null,      // Raw total amount found
  "original_currency": "EUR" | "CZK" | "HUF" | "PLN" or null,

  // FUEL-SPECIFIC (only if this is a gas station receipt):
  "liters": number or null,  // null if NOT a fuel receipt
  "station_name": string or null,
  "station_address": string or null,

  // OTHER COSTS (for non-fuel receipts):
  "vendor_name": string or null,      // Company/shop name
  "cost_description": string or null, // Brief description (e.g., "Umytie auta", "Parkovanie 2h")

  "confidence": {
    "liters": "high" | "medium" | "low" | "not_applicable",
    "total_price": "high" | "medium" | "low",
    "date": "high" | "medium" | "low",
    "currency": "high" | "medium" | "low"
  },
  "raw_text": "full OCR text"
}

Rules:
- If you see "L", "litrov", fuel types (Natural 95, Diesel, benzín, nafta) → it's FUEL, extract liters
- If NO liters/fuel indicators → it's OTHER COST, set liters=null, confidence.liters="not_applicable"
- For amounts: Look for total/sum keywords in the receipt's language
- Currency detection:
  - € or EUR → "EUR"
  - Kč or CZK → "CZK" (Czech koruna)
  - Ft or HUF → "HUF" (Hungarian forint)
  - zł or PLN → "PLN" (Polish złoty)
  - If no symbol found, guess from language/country context
- Date formats: DD.MM.YYYY or DD.MM.YY (European format)
- Return null if field cannot be determined"#;

fn get_response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "liters": {
                "type": ["number", "null"],
                "description": "Amount of fuel in liters (null if not a fuel receipt)"
            },
            "total_price_eur": {
                "type": ["number", "null"],
                "description": "Total price in EUR (legacy field, may be null)"
            },
            "original_amount": {
                "type": ["number", "null"],
                "description": "Raw total amount found on receipt (in original currency)"
            },
            "original_currency": {
                "type": ["string", "null"],
                "enum": ["EUR", "CZK", "HUF", "PLN", null],
                "description": "Currency code: EUR, CZK (Czech), HUF (Hungarian), PLN (Polish)"
            },
            "receipt_date": {
                "type": ["string", "null"],
                "description": "Date in YYYY-MM-DD format"
            },
            "station_name": {
                "type": ["string", "null"],
                "description": "Gas station name (for fuel receipts)"
            },
            "station_address": {
                "type": ["string", "null"],
                "description": "Gas station address (for fuel receipts)"
            },
            "vendor_name": {
                "type": ["string", "null"],
                "description": "Company/shop name (for non-fuel receipts)"
            },
            "cost_description": {
                "type": ["string", "null"],
                "description": "Brief description of the expense (for non-fuel receipts)"
            },
            "raw_text": {
                "type": ["string", "null"],
                "description": "Full OCR text from receipt"
            },
            "confidence": {
                "type": "object",
                "properties": {
                    "liters": {
                        "type": "string",
                        "enum": ["high", "medium", "low", "not_applicable"]
                    },
                    "total_price": {
                        "type": "string",
                        "enum": ["high", "medium", "low"]
                    },
                    "date": {
                        "type": "string",
                        "enum": ["high", "medium", "low"]
                    },
                    "currency": {
                        "type": "string",
                        "enum": ["high", "medium", "low"]
                    }
                },
                "required": ["liters", "total_price", "date", "currency"]
            }
        },
        "required": ["liters", "original_amount", "original_currency", "receipt_date", "confidence"]
    })
}

pub struct GeminiClient {
    api_key: String,
    client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Extract receipt data from an image file (async)
    ///
    /// If `KNIHA_JAZD_MOCK_GEMINI_DIR` is set, loads mock data from JSON file
    /// instead of calling the Gemini API. This enables deterministic testing.
    pub async fn extract_from_image(&self, image_path: &Path) -> Result<ExtractedReceipt, String> {
        // Check for mock mode (used in integration tests)
        if let Ok(mock_dir) = std::env::var(MOCK_GEMINI_DIR_ENV) {
            log::info!(
                "Mock mode enabled: loading from {:?} for {:?}",
                mock_dir,
                image_path
            );
            return load_mock_extraction(&mock_dir, image_path);
        }

        // Read and encode image
        let image_data = tokio::fs::read(image_path)
            .await
            .map_err(|e| format!("Failed to read image: {}", e))?;
        let base64_image = STANDARD.encode(&image_data);

        // Determine mime type from extension
        let mime_type = match image_path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("pdf") => "application/pdf",
            _ => "image/jpeg", // Default
        };

        // Build request
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text {
                        text: EXTRACTION_PROMPT.to_string(),
                    },
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type: mime_type.to_string(),
                            data: base64_image,
                        },
                    },
                ],
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".to_string(),
                response_json_schema: get_response_schema(),
            },
        };

        // Call Gemini API (using gemini-2.5-flash as per latest docs)
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, error_text));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        // Extract JSON from response
        let text = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or("No response text from API")?;

        // Parse extracted data
        let extracted: ExtractedReceipt = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse extraction result: {} - Raw: {}", e, text))?;

        Ok(extracted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Removed: test_extraction_prompt_is_valid
    // Asserting string.contains() doesn't test behavior

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
        use std::io::Write;
        use tempfile::tempdir;

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
        use tempfile::tempdir;

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
        use std::io::Write;
        use tempfile::tempdir;

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
}
