//! Fuelio cross-check (Task 90, POC): compare the drives that the Fuelio
//! Android app recorded with the logbook trips.
//!
//! Fuelio backs up each drive to Google Drive as `route-<epoch ms>.data`, a zip
//! with one CSV of GPS fixes. Something outside the app copies that folder to
//! `<DATA_DIR>/fuelio`; this module only reads it. Read-only: nothing here
//! writes to the database.
//!
//! - [`parse`]: the files -> [`Drive`]s
//! - [`geo`]: distances on the earth's surface
//! - [`matching`]: drives against trips -> report rows

pub mod geo;
pub mod matching;
pub mod parse;

pub use matching::{crosscheck, CrosscheckRow, Flag, RowStatus, TripRef};
pub use parse::{scan_dir, Drive};

/// The folder below the data directory that holds the Fuelio files.
pub const FOLDER_NAME: &str = "fuelio";
