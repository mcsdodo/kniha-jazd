//! Paperless-ngx integration command implementations.

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::models::{AssignmentType, PaperlessInvoiceRow};
use crate::paperless::{CustomFieldInfo, PaperlessClient, PaperlessDoc, PaperlessError, PaperlessFieldNames};
use crate::settings::LocalSettings;
use std::path::Path;

pub fn map_assignment(tag_ids: &[i64], fuel_id: i64, car_id: i64) -> AssignmentType {
    if tag_ids.contains(&fuel_id) {
        AssignmentType::Fuel
    } else if tag_ids.contains(&car_id) {
        AssignmentType::Other
    } else {
        log::warn!(
            "paperless: doc has neither fuel ({}) nor car ({}) tag — got {:?}; \
             check server-side filter",
            fuel_id, car_id, tag_ids
        );
        AssignmentType::Other
    }
}

pub fn doc_year(dt: &Option<chrono::NaiveDateTime>, created: &chrono::NaiveDate) -> i32 {
    use chrono::Datelike;
    dt.as_ref().map(|d| d.year()).unwrap_or(created.year())
}

/// List all custom fields from the configured Paperless server.
/// Used by the Settings UI to populate field-name dropdowns.
/// Returns NotConfigured if URL or token is missing — the caller (UI) treats that
/// as "hide the dropdown section" rather than an error to surface.
pub async fn list_paperless_custom_fields_internal(
    app_dir: &Path,
) -> Result<Vec<CustomFieldInfo>, PaperlessError> {
    let settings = LocalSettings::load(app_dir);
    let (url, token) = match (settings.paperless_url, settings.paperless_api_token) {
        (Some(u), Some(t)) if !u.is_empty() && !t.is_empty() => (u, t),
        _ => return Err(PaperlessError::NotConfigured),
    };
    let base = url.trim_end_matches('/').to_string();
    let client = PaperlessClient::new(base, token);
    client.list_custom_fields().await
}

/// Paperless v1 is single-vehicle scoped — vehicle_id is intentionally unused.
/// See DECISIONS.md "BIZ — Paperless v1 is single-vehicle scoped" (added in Task 16).
pub async fn get_paperless_invoices_internal(
    app_dir: &Path,
    db: &Database,
    vehicle_id: &str,
    year: i32,
) -> Result<Vec<PaperlessInvoiceRow>, PaperlessError> {
    let _ = vehicle_id;

    let settings = LocalSettings::load(app_dir);
    let names = PaperlessFieldNames::from_settings(&settings);
    let (url, token) = match (settings.paperless_url, settings.paperless_api_token) {
        (Some(u), Some(t)) if !u.is_empty() && !t.is_empty() => (u, t),
        _ => return Err(PaperlessError::NotConfigured),
    };
    let base = url.trim_end_matches('/').to_string();

    let client = PaperlessClient::new(base.clone(), token);
    let fuel_id = client.resolve_tag_id("fuel").await?;
    let car_id  = client.resolve_tag_id("car").await?;
    let fmap    = client.resolve_field_map(&names).await?;

    let docs: Vec<PaperlessDoc> = client.fetch_invoice_documents(fuel_id, car_id, &fmap).await?;
    let docs: Vec<PaperlessDoc> = docs.into_iter()
        .filter(|d| doc_year(&d.receipt_datetime, &d.created) == year)
        .collect();

    let doc_ids: Vec<i64> = docs.iter().map(|d| d.id).collect();
    let links = db.list_paperless_links_for_docs(&doc_ids)
        .map_err(|e| PaperlessError::Parse(e.to_string()))?;
    let link_map: std::collections::HashMap<i64, String> = links.into_iter().collect();

    Ok(docs.into_iter().map(|d| PaperlessInvoiceRow {
        paperless_url: format!("{}/documents/{}/", base, d.id),
        trip_id: link_map.get(&d.id).cloned(),
        assignment_type: map_assignment(&d.tag_ids, fuel_id, car_id),
        paperless_document_id: d.id, title: d.title,
        total_price_eur: d.total_amount, liters: d.litres,
        receipt_datetime: d.receipt_datetime, created_date: d.created,
    }).collect())
}

pub fn assign_paperless_doc_to_trip_internal(
    app_state: &AppState, db: &Database,
    doc_id: i64, trip_id: &str,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.upsert_paperless_link(trip_id, doc_id).map_err(|e| e.to_string())
}

pub fn unassign_paperless_doc_internal(
    app_state: &AppState, db: &Database, doc_id: i64,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_paperless_link_for_doc(doc_id).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "paperless_cmd_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) mod test_helpers {
    pub fn make_doc(tag_ids: &[i64]) -> crate::paperless::PaperlessDoc {
        crate::paperless::PaperlessDoc {
            id: 0, title: "t".into(), tag_ids: tag_ids.to_vec(),
            created: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            total_amount: None, litres: None, receipt_datetime: None,
        }
    }
}
