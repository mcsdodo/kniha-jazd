//! Read-only logbook queries for the MCP endpoint (task 89).
//!
//! `LogbookReader` is the only thing the `mcp` module holds. It keeps its
//! `Database` private and exposes reads only, so the MCP module has no path
//! to a write. The source guard in `mcp/tests.rs` checks this file too.

use std::sync::Arc;

use crate::commands_internal::get_vehicles_internal;
use crate::db::Database;
use crate::models::Vehicle;

/// `Invalid`: the caller can fix it (bad range, unknown vehicle, no home).
/// `Internal`: a DB error.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    Invalid(String),
    Internal(String),
}

#[derive(Clone)]
pub struct LogbookReader {
    db: Arc<Database>,
}

impl LogbookReader {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn vehicles(&self) -> Result<Vec<Vehicle>, ReadError> {
        get_vehicles_internal(&self.db).map_err(ReadError::Internal)
    }
}
