//! Source-agnostic invoice abstraction (Task 64).
//!
//! Both local receipts and Paperless documents are *invoices* from the user's
//! perspective. This module provides the trait, IPC boundary types, and compat
//! check that the unified picker uses. Source-specific dispatch is confined to
//! the Tauri command boundary (see desktop/src/commands/invoices.rs).

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::commands_internal::statistics::is_datetime_in_trip_range;
use crate::models::{AssignmentType, AttachmentStatus, Trip};

/// Tagged reference used at the IPC boundary.
/// Serializes to `{ "source": "receipt", "id": "uuid" }`
/// or            `{ "source": "paperless", "id": 12345 }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "source", content = "id", rename_all = "lowercase")]
pub enum InvoiceRef {
    Receipt(String), // UUID string
    Paperless(i64),  // Paperless document ID
}

/// Inline invoice payload sent by the frontend alongside the InvoiceRef.
/// For Receipt: backend ignores this and loads from DB by id.
/// For Paperless: backend uses these fields directly (paperless_trip_links has no doc data).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceData {
    pub datetime: Option<NaiveDateTime>,
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub title: String,
    pub assignment_type: AssignmentType,
}

/// Source-agnostic view of an invoice.
/// All matching, sorting, and display code consumes this — never the concrete types.
pub trait Invoice {
    fn datetime(&self) -> Option<NaiveDateTime>;
    fn liters(&self) -> Option<f64>;
    fn total_price_eur(&self) -> Option<f64>;
    fn display_name(&self) -> &str;
    fn invoice_ref(&self) -> InvoiceRef;
    fn assignment_type(&self) -> Option<AssignmentType>;
}

/// Adapter for Paperless invoices when only the inline `InvoiceData` is available.
/// Used at the IPC boundary to give the compat check an `&dyn Invoice` for paperless docs.
pub struct PaperlessInvoiceView<'a> {
    pub id: i64,
    pub data: &'a InvoiceData,
}

impl<'a> Invoice for PaperlessInvoiceView<'a> {
    fn datetime(&self) -> Option<NaiveDateTime> { self.data.datetime }
    fn liters(&self) -> Option<f64> { self.data.liters }
    fn total_price_eur(&self) -> Option<f64> { self.data.total_price_eur }
    fn display_name(&self) -> &str { &self.data.title }
    fn invoice_ref(&self) -> InvoiceRef { InvoiceRef::Paperless(self.id) }
    fn assignment_type(&self) -> Option<AssignmentType> { Some(self.data.assignment_type) }
}

/// Compat check result.
pub struct CompatibilityResult {
    pub can_attach: bool,
    pub status: String,
    pub mismatch_reason: Option<String>,
}

fn is_same_date(dt: NaiveDateTime, trip: &Trip) -> bool {
    dt.date() == trip.start_datetime.date()
}

fn get_datetime_mismatch_type(dt: Option<NaiveDateTime>, trip: &Trip) -> Option<&'static str> {
    match dt {
        Some(d) if is_datetime_in_trip_range(d, trip) => None,
        Some(d) if is_same_date(d, trip) => Some("time"),
        Some(_) => Some("date"),
        None => Some("date"),
    }
}

/// Check if invoice data matches trip's existing data.
/// Returns compatibility result with detailed mismatch reason.
/// Handles both FUEL invoices (has liters) and OTHER cost invoices (no liters).
pub fn check_invoice_trip_compatibility(
    invoice: &dyn Invoice,
    trip: &Trip,
) -> CompatibilityResult {
    let is_fuel = invoice.liters().is_some();

    if is_fuel {
        let trip_has_fuel = trip.fuel_liters.map(|l| l > 0.0).unwrap_or(false);
        if !trip_has_fuel {
            let status = match invoice.datetime() {
                Some(dt) if is_datetime_in_trip_range(dt, trip) => AttachmentStatus::Matches,
                Some(dt) if is_same_date(dt, trip) => AttachmentStatus::MatchesDate,
                _ => AttachmentStatus::Empty,
            };
            return CompatibilityResult {
                can_attach: true,
                status: status.as_str().to_string(),
                mismatch_reason: None,
            };
        }
        let r_liters = invoice.liters().unwrap();
        let r_price = invoice.total_price_eur().unwrap_or(0.0);
        let datetime_mismatch = get_datetime_mismatch_type(invoice.datetime(), trip);
        let liters_match = trip.fuel_liters.map(|fl| (fl - r_liters).abs() < 0.01).unwrap_or(false);
        let price_match = trip.fuel_cost_eur.map(|fc| (fc - r_price).abs() < 0.01).unwrap_or(false);

        if datetime_mismatch.is_none() && liters_match && price_match {
            return CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Matches.as_str().to_string(),
                mismatch_reason: None,
            };
        }
        let dt_type = datetime_mismatch.unwrap_or("date");
        let mismatch = match (datetime_mismatch.is_some(), liters_match, price_match) {
            (false, false, false) => "liters_and_price",
            (false, false, true) => "liters",
            (false, true, false) => "price",
            (false, true, true) => unreachable!(),
            (true, false, false) => match dt_type { "time" => "time_and_liters_and_price", _ => "all" },
            (true, false, true)  => match dt_type { "time" => "time_and_liters", _ => "date_and_liters" },
            (true, true, false)  => match dt_type { "time" => "time_and_price", _ => "date_and_price" },
            (true, true, true)   => dt_type,
        };
        CompatibilityResult {
            can_attach: true,
            status: AttachmentStatus::Differs.as_str().to_string(),
            mismatch_reason: Some(mismatch.to_string()),
        }
    } else {
        let trip_has_other_costs = trip.other_costs_eur.map(|c| c > 0.0).unwrap_or(false);
        if !trip_has_other_costs {
            let status = match invoice.datetime() {
                Some(dt) if is_datetime_in_trip_range(dt, trip) => AttachmentStatus::Matches,
                Some(dt) if is_same_date(dt, trip) => AttachmentStatus::MatchesDate,
                _ => AttachmentStatus::Empty,
            };
            return CompatibilityResult {
                can_attach: true,
                status: status.as_str().to_string(),
                mismatch_reason: None,
            };
        }
        if let Some(r_price) = invoice.total_price_eur() {
            let datetime_mismatch = get_datetime_mismatch_type(invoice.datetime(), trip);
            let price_match = trip.other_costs_eur.map(|tc| (tc - r_price).abs() < 0.01).unwrap_or(false);
            if datetime_mismatch.is_none() && price_match {
                return CompatibilityResult {
                    can_attach: true,
                    status: AttachmentStatus::Matches.as_str().to_string(),
                    mismatch_reason: None,
                };
            }
            let dt_type = datetime_mismatch.unwrap_or("date");
            let mismatch = match (datetime_mismatch.is_some(), price_match) {
                (false, false) => "price",
                (false, true) => unreachable!(),
                (true, false) => match dt_type { "time" => "time_and_price", _ => "date_and_price" },
                (true, true)  => dt_type,
            };
            CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Differs.as_str().to_string(),
                mismatch_reason: Some(mismatch.to_string()),
            }
        } else {
            CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Empty.as_str().to_string(),
                mismatch_reason: None,
            }
        }
    }
}

#[cfg(test)]
#[path = "invoice_tests.rs"]
mod tests;
