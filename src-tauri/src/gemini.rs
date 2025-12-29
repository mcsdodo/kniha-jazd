//! Gemini API client for receipt OCR and extraction

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedReceipt {
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_date: Option<String>, // YYYY-MM-DD format
    pub station_name: Option<String>,
    pub station_address: Option<String>,
    pub raw_text: Option<String>,
    pub confidence: ExtractionConfidence,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionConfidence {
    pub liters: String,      // "high", "medium", "low"
    pub total_price: String,
    pub date: String,
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

const EXTRACTION_PROMPT: &str = r#"Analyze this Slovak gas station receipt (bloček).
Extract the following fields. Look for:
- "L" or "litrov" near numbers for liters
- "€" or "EUR" for total price, usually the largest amount
- Date formats: DD.MM.YYYY or DD.MM.YY
- Station name and address if visible

Return null if a field cannot be determined with reasonable confidence.
For confidence: "high" = clearly visible, "medium" = partially visible/guessed, "low" = very uncertain"#;

fn get_response_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "liters": {
                "type": ["number", "null"],
                "description": "Amount of fuel in liters"
            },
            "total_price_eur": {
                "type": ["number", "null"],
                "description": "Total price in EUR"
            },
            "receipt_date": {
                "type": ["string", "null"],
                "description": "Date in YYYY-MM-DD format"
            },
            "station_name": {
                "type": ["string", "null"],
                "description": "Gas station name"
            },
            "station_address": {
                "type": ["string", "null"],
                "description": "Gas station address"
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
                        "enum": ["high", "medium", "low"]
                    },
                    "total_price": {
                        "type": "string",
                        "enum": ["high", "medium", "low"]
                    },
                    "date": {
                        "type": "string",
                        "enum": ["high", "medium", "low"]
                    }
                },
                "required": ["liters", "total_price", "date"]
            }
        },
        "required": ["liters", "total_price_eur", "receipt_date", "confidence"]
    })
}

pub struct GeminiClient {
    api_key: String,
    client: reqwest::blocking::Client,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Extract receipt data from an image file
    pub fn extract_from_image(&self, image_path: &Path) -> Result<ExtractedReceipt, String> {
        // Read and encode image
        let image_data = fs::read(image_path)
            .map_err(|e| format!("Failed to read image: {}", e))?;
        let base64_image = STANDARD.encode(&image_data);

        // Determine mime type from extension
        let mime_type = match image_path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            _ => "image/jpeg", // Default
        };

        // Build request
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text { text: EXTRACTION_PROMPT.to_string() },
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

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_default();
            return Err(format!("API error {}: {}", status, error_text));
        }

        let gemini_response: GeminiResponse = response
            .json()
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

    #[test]
    fn test_extraction_prompt_is_valid() {
        // Just verify the prompt compiles and has expected content
        assert!(EXTRACTION_PROMPT.contains("liters"));
        assert!(EXTRACTION_PROMPT.contains("total price"));
        assert!(EXTRACTION_PROMPT.contains("DD.MM.YYYY")); // Slovak date format
    }

    #[test]
    fn test_extracted_receipt_deserialization() {
        let json = r#"{
            "liters": 45.5,
            "total_price_eur": 72.50,
            "receipt_date": "2024-12-15",
            "station_name": "Slovnaft",
            "station_address": "Bratislava",
            "raw_text": "some text",
            "confidence": {
                "liters": "high",
                "total_price": "high",
                "date": "medium"
            }
        }"#;

        let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
        assert_eq!(extracted.liters, Some(45.5));
        assert_eq!(extracted.total_price_eur, Some(72.50));
        assert_eq!(extracted.receipt_date, Some("2024-12-15".to_string()));
        assert_eq!(extracted.confidence.liters, "high");
    }

    #[test]
    fn test_extracted_receipt_with_nulls() {
        let json = r#"{
            "liters": null,
            "total_price_eur": 50.00,
            "receipt_date": null,
            "station_name": null,
            "station_address": null,
            "raw_text": "blurry text",
            "confidence": {
                "liters": "low",
                "total_price": "medium",
                "date": "low"
            }
        }"#;

        let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
        assert!(extracted.liters.is_none());
        assert_eq!(extracted.total_price_eur, Some(50.00));
        assert!(extracted.receipt_date.is_none());
    }

    #[test]
    fn test_response_schema_is_valid_json() {
        let schema = get_response_schema();
        assert!(schema.is_object());
        assert!(schema.get("type").is_some());
        assert!(schema.get("properties").is_some());
    }
}
