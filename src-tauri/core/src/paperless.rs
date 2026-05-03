//! Paperless-ngx HTTP client + parsing.

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum PaperlessError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Paperless returned status {0}")]
    Http(u16),
    #[error("Tag '{0}' not found in Paperless")]
    TagNotFound(String),
    #[error("Custom field '{0}' not found in Paperless")]
    CustomFieldNotFound(String),
    #[error("Paperless URL not configured")]
    NotConfigured,
    #[error("Failed to parse Paperless response: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for PaperlessError {
    fn from(e: reqwest::Error) -> Self { PaperlessError::Network(e.to_string()) }
}

#[derive(Debug, Clone, Copy)]
pub struct PaperlessFieldMap {
    pub total_amount_id: i64,
    pub litres_id: i64,
    pub receipt_datetime_id: i64,
}

pub struct PaperlessClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl PaperlessClient {
    pub fn new(base_url: String, token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build().expect("reqwest client");
        Self { base_url: base_url.trim_end_matches('/').to_string(), token, http }
    }

    fn auth(&self) -> String { format!("Token {}", self.token) }

    pub async fn resolve_tag_id(&self, name: &str) -> Result<i64, PaperlessError> {
        #[derive(Deserialize)] struct Tag { id: i64 }
        #[derive(Deserialize)] struct Resp { results: Vec<Tag> }

        let url = format!("{}/api/tags/?name__iexact={}", self.base_url, urlencoding::encode(name));
        let resp = self.http.get(&url).header("Authorization", self.auth()).send().await?;
        if !resp.status().is_success() { return Err(PaperlessError::Http(resp.status().as_u16())); }
        let body: Resp = resp.json().await.map_err(|e| PaperlessError::Parse(e.to_string()))?;
        body.results.first().map(|t| t.id).ok_or_else(|| PaperlessError::TagNotFound(name.to_string()))
    }

    pub async fn resolve_field_map(&self) -> Result<PaperlessFieldMap, PaperlessError> {
        #[derive(Deserialize)] struct Field { id: i64, name: String }
        #[derive(Deserialize)] struct Resp { results: Vec<Field> }

        let url = format!("{}/api/custom_fields/", self.base_url);
        let resp = self.http.get(&url).header("Authorization", self.auth()).send().await?;
        if !resp.status().is_success() { return Err(PaperlessError::Http(resp.status().as_u16())); }
        let body: Resp = resp.json().await.map_err(|e| PaperlessError::Parse(e.to_string()))?;

        let find = |n: &str| body.results.iter().find(|f| f.name == n).map(|f| f.id)
            .ok_or_else(|| PaperlessError::CustomFieldNotFound(n.to_string()));

        Ok(PaperlessFieldMap {
            total_amount_id: find("total_amount")?,
            litres_id: find("litres")?,
            receipt_datetime_id: find("receipt_datetime")?,
        })
    }
}

#[cfg(test)]
#[path = "paperless_tests.rs"]
mod tests;
