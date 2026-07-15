//! Source-agnostic invoice abstraction (Task 64).
//!
//! Both local receipts and Paperless documents are *invoices* from the user's
//! perspective. This module provides the trait, IPC boundary types, and compat
//! check that the unified picker uses. Source-specific dispatch is confined to
//! the Tauri command boundary (see desktop/src/commands/invoices.rs).

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::calculations::to_cents;
use crate::commands_internal::statistics::is_datetime_in_trip_range;
use crate::models::{AssignmentType, AttachmentStatus, Trip, TripInvoiceCoverage};

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
    let trip_end = trip.end_datetime.unwrap_or(trip.start_datetime);
    dt.date() >= trip.start_datetime.date() && dt.date() <= trip_end.date()
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
///
/// Multi-invoice semantics (Task 66, test review C8/I1): `coverage` is the
/// trip's invoice coverage across BOTH sources (local receipts + paperless
/// links). Fuel invoices cannot attach to a trip that already has a Fuel
/// invoice (`can_attach = false` — the picker greys the trip out; the assign
/// pre-check stays authoritative). Other invoices skip the amount comparison
/// entirely once the trip carries >=1 Other invoice (the new amount is summed
/// on assign — there is nothing to match against); with zero Others the
/// comparison is cent-exact via `to_cents`, so the picker verdict always
/// agrees with the assign-time double-count guard.
pub fn check_invoice_trip_compatibility(
    invoice: &dyn Invoice,
    trip: &Trip,
    coverage: &TripInvoiceCoverage,
) -> CompatibilityResult {
    let is_fuel = match invoice.assignment_type() {
        Some(AssignmentType::Fuel) => true,
        Some(AssignmentType::Other) => false,
        None => invoice.liters().is_some(),
    };

    if is_fuel {
        // A trip holds at most ONE Fuel invoice across both sources (I1).
        if coverage.has_fuel {
            return CompatibilityResult {
                can_attach: false,
                status: AttachmentStatus::Differs.as_str().to_string(),
                mismatch_reason: Some("fuel_invoice_exists".to_string()),
            };
        }
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
        // Trip already carries >=1 Other invoice: the total is a running sum,
        // so comparing the new invoice against it is meaningless — skip the
        // amount check entirely (C8; the amount is summed on assign).
        if coverage.has_other {
            return CompatibilityResult {
                can_attach: true,
                status: AttachmentStatus::Matches.as_str().to_string(),
                mismatch_reason: None,
            };
        }
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
            // Cent-exact (Task 66): must agree with the assign-time
            // double-count guard, which compares via to_cents — the old ±0.01
            // epsilon disagreed on borderline values (12.34 vs 12.3345).
            let price_match = trip.other_costs_eur.map(|tc| to_cents(tc) == to_cents(r_price)).unwrap_or(false);
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
