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

#[derive(Debug, Clone)]
pub struct PaperlessDoc {
    pub id: i64,
    pub title: String,
    pub tag_ids: Vec<i64>,
    pub created: chrono::NaiveDate,
    pub total_amount: Option<f64>,
    pub litres: Option<f64>,
    pub receipt_datetime: Option<chrono::NaiveDateTime>,
}

impl PaperlessClient {
    pub async fn fetch_invoice_documents(
        &self, fuel_id: i64, car_id: i64, fields: &PaperlessFieldMap,
    ) -> Result<Vec<PaperlessDoc>, PaperlessError> {
        #[derive(Deserialize)] struct CustomField { field: i64, value: serde_json::Value }
        #[derive(Deserialize)] struct Raw {
            id: i64, title: String, tags: Vec<i64>, created: String,
            #[serde(default)] custom_fields: Vec<CustomField>,
        }
        #[derive(Deserialize)] struct Page { next: Option<String>, results: Vec<Raw> }

        let mut url = format!(
            "{}/api/documents/?tags__id__in={},{}&page_size=100",
            self.base_url, fuel_id, car_id
        );

        let mut out = Vec::new();
        loop {
            let resp = self.http.get(&url).header("Authorization", self.auth()).send().await?;
            if !resp.status().is_success() { return Err(PaperlessError::Http(resp.status().as_u16())); }
            let page: Page = resp.json().await.map_err(|e| PaperlessError::Parse(e.to_string()))?;

            for r in page.results {
                let created = chrono::NaiveDate::parse_from_str(&r.created, "%Y-%m-%d")
                    .map_err(|e| PaperlessError::Parse(format!("created '{}': {}", r.created, e)))?;

                let mut total = None;
                let mut litres = None;
                let mut dt = None;
                for cf in r.custom_fields {
                    if cf.field == fields.total_amount_id {
                        total = cf.value.as_f64();
                    } else if cf.field == fields.litres_id {
                        litres = cf.value.as_f64();
                    } else if cf.field == fields.receipt_datetime_id {
                        if let Some(s) = cf.value.as_str() {
                            dt = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok();
                        }
                    }
                }

                out.push(PaperlessDoc {
                    id: r.id, title: r.title, tag_ids: r.tags, created,
                    total_amount: total, litres, receipt_datetime: dt,
                });
            }

            match page.next { Some(n) => url = n, None => break }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "paperless_tests.rs"]
mod tests;
