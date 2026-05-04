//! Source-agnostic invoice abstraction (Task 64).
//!
//! Both local receipts and Paperless documents are *invoices* from the user's
//! perspective. This module provides the trait, IPC boundary types, and compat
//! check that the unified picker uses. Source-specific dispatch is confined to
//! the Tauri command boundary (see desktop/src/commands/invoices.rs).

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::models::AssignmentType;

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

#[cfg(test)]
#[path = "invoice_tests.rs"]
mod tests;
